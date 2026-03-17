const INVITE_CODE_LENGTH: usize = 12;
const INVITE_CODE_RETRY_LIMIT: usize = 24;

impl AppInfra {
    pub async fn register_user_with_invite(
        &self,
        username: &str,
        password: &str,
        invite_code: Option<&str>,
    ) -> anyhow::Result<UserDto> {
        let username = username.trim();
        if username.is_empty() {
            anyhow::bail!("username is required");
        }

        let existing_user_id: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM users WHERE username = $1 LIMIT 1")
                .bind(username)
                .fetch_optional(&self.pool)
                .await?;
        if existing_user_id.is_some() {
            return Err(anyhow::Error::new(InfraError::UserAlreadyExists));
        }

        let normalized_code = invite_code
            .map(normalize_invite_code)
            .filter(|value| !value.is_empty());

        if self.config_snapshot().auth.invite.force_on_register && normalized_code.is_none() {
            return Err(anyhow::Error::new(InfraError::InviteCodeRequired));
        }

        let mut tx = self.pool.begin().await?;

        let inviter = if let Some(code) = normalized_code.as_deref() {
            let row = sqlx::query_as::<_, InviteCodeOwnerRow>(
                r#"
SELECT user_id
FROM user_invite_codes
WHERE code = $1 AND enabled = true
LIMIT 1
FOR UPDATE
                "#,
            )
            .bind(code)
            .fetch_optional(&mut *tx)
            .await?;

            match row {
                Some(row) => Some(row.user_id),
                None => return Err(anyhow::Error::new(InfraError::InviteCodeInvalid)),
            }
        } else {
            None
        };

        let user_id = Uuid::now_v7();
        let hash = auth::hash_password(password);
        let user_row = sqlx::query_as::<_, UserRow>(
            r#"
INSERT INTO users (id, username, password_hash, role, is_admin, is_disabled)
VALUES ($1, $2, $3, 'Viewer', false, false)
RETURNING id, username, password_hash, role, is_admin, is_disabled
            "#,
        )
        .bind(user_id)
        .bind(username)
        .bind(hash)
        .fetch_one(&mut *tx)
        .await;
        let user_row = match user_row {
            Ok(row) => row,
            Err(sqlx::Error::Database(db_err)) if db_err.code().as_deref() == Some("23505") => {
                return Err(anyhow::Error::new(InfraError::UserAlreadyExists));
            }
            Err(err) => return Err(anyhow::Error::new(err)),
        };

        let max_concurrent_streams =
            normalize_default_optional_i32(self.config_snapshot().security.default_user_max_concurrent_streams);
        let traffic_quota_bytes =
            normalize_default_optional_i64(self.config_snapshot().security.default_user_traffic_quota_bytes);
        let traffic_window_days =
            normalize_traffic_window_days(self.config_snapshot().security.default_user_traffic_window_days);

        sqlx::query(
            r#"
INSERT INTO user_stream_policies (
    user_id,
    expires_at,
    max_concurrent_streams,
    traffic_quota_bytes,
    traffic_window_days,
    updated_at
) VALUES (
    $1, NULL, $2, $3, $4, now()
)
ON CONFLICT (user_id) DO NOTHING
            "#,
        )
        .bind(user_id)
        .bind(max_concurrent_streams)
        .bind(traffic_quota_bytes)
        .bind(traffic_window_days)
        .execute(&mut *tx)
        .await?;

        let now = Utc::now();
        sqlx::query(
            r#"
INSERT INTO playlists (id, owner_user_id, name, description, is_public, is_default, created_at, updated_at)
VALUES ($1, $2, $3, $4, FALSE, TRUE, $5, $6)
ON CONFLICT (owner_user_id, name) DO NOTHING
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(user_id)
        .bind(DEFAULT_FAVORITES_PLAYLIST_NAME)
        .bind(DEFAULT_FAVORITES_PLAYLIST_DESCRIPTION)
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        let _ = Self::ensure_user_invite_code_tx(&mut tx, user_id).await?;

        let relation_id = if let Some(inviter_user_id) = inviter {
            if inviter_user_id == user_id {
                return Err(anyhow::Error::new(InfraError::InviteCodeInvalid));
            }

            let relation_id = Uuid::now_v7();
            let invite_code_value = normalized_code.clone().unwrap_or_default();
            sqlx::query(
                r#"
INSERT INTO user_invite_relations (id, inviter_user_id, invitee_user_id, invite_code, created_at)
VALUES ($1, $2, $3, $4, now())
                "#,
            )
            .bind(relation_id)
            .bind(inviter_user_id)
            .bind(user_id)
            .bind(invite_code_value)
            .execute(&mut *tx)
            .await?;

            Some((relation_id, inviter_user_id))
        } else {
            None
        };

        let bonus_amount = normalize_invite_money(self.config_snapshot().auth.invite.invitee_bonus_amount);
        if let Some((relation_id, inviter_user_id)) = relation_id {
            if self.config_snapshot().auth.invite.invitee_bonus_enabled && bonus_amount > Decimal::ZERO {
                let mut wallet = Self::ensure_wallet_account_locked_tx(&mut tx, user_id).await?;
                wallet.balance = normalize_invite_money(wallet.balance + bonus_amount);

                sqlx::query(
                    r#"
UPDATE wallet_accounts
SET balance = $2, total_recharged = $3, total_spent = $4, updated_at = now()
WHERE user_id = $1
                    "#,
                )
                .bind(wallet.user_id)
                .bind(wallet.balance)
                .bind(wallet.total_recharged)
                .bind(wallet.total_spent)
                .execute(&mut *tx)
                .await?;

                let relation_ref = relation_id.to_string();
                Self::append_wallet_ledger_tx(
                    &mut tx,
                    user_id,
                    "invite_bonus",
                    bonus_amount,
                    wallet.balance,
                    Some("invite_relation"),
                    Some(relation_ref.as_str()),
                    Some("invite signup bonus"),
                    json!({
                        "inviter_user_id": inviter_user_id,
                    }),
                )
                .await?;
            }
        }

        tx.commit().await?;

        Ok(to_user_dto(&user_row, &self.server_id))
    }

    pub async fn get_invite_summary(&self, user_id: Uuid) -> anyhow::Result<Option<InviteSummary>> {
        let exists: Option<Uuid> = sqlx::query_scalar("SELECT id FROM users WHERE id = $1 LIMIT 1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?;
        if exists.is_none() {
            return Ok(None);
        }

        let code_row = self.ensure_user_invite_code(user_id).await?;
        let invited_count = sqlx::query_scalar::<_, i64>(
            r#"
SELECT COUNT(*)::BIGINT
FROM user_invite_relations
WHERE inviter_user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        let rebate_total = sqlx::query_scalar::<_, Option<Decimal>>(
            r#"
SELECT COALESCE(SUM(rebate_amount), 0)::NUMERIC(18,2)
FROM invite_rebate_records
WHERE inviter_user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(Decimal::ZERO);

        Ok(Some(InviteSummary {
            code: code_row.code,
            enabled: code_row.enabled,
            invited_count,
            rebate_total: normalize_invite_money(rebate_total),
            invitee_bonus_enabled: self.config_snapshot().auth.invite.invitee_bonus_enabled,
        }))
    }

    pub async fn reset_invite_code(&self, user_id: Uuid) -> anyhow::Result<Option<InviteSummary>> {
        let exists: Option<Uuid> = sqlx::query_scalar("SELECT id FROM users WHERE id = $1 LIMIT 1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?;
        if exists.is_none() {
            return Ok(None);
        }

        let mut tx = self.pool.begin().await?;
        let _ = Self::ensure_user_invite_code_tx(&mut tx, user_id).await?;

        let mut updated_row = None;
        for _ in 0..INVITE_CODE_RETRY_LIMIT {
            let candidate = generate_invite_code();
            let row = sqlx::query_as::<_, UserInviteCodeRow>(
                r#"
UPDATE user_invite_codes
SET code = $2, enabled = true, updated_at = now(), reset_at = now()
WHERE user_id = $1
RETURNING user_id, code, enabled, created_at, updated_at, reset_at
                "#,
            )
            .bind(user_id)
            .bind(candidate)
            .fetch_optional(&mut *tx)
            .await;

            match row {
                Ok(row) => {
                    updated_row = row;
                    break;
                }
                Err(sqlx::Error::Database(db_err)) if db_err.code().as_deref() == Some("23505") => {
                    continue;
                }
                Err(err) => return Err(anyhow::Error::new(err)),
            }
        }

        let Some(row) = updated_row else {
            anyhow::bail!("failed to generate unique invite code after retries");
        };

        tx.commit().await?;

        let invited_count = sqlx::query_scalar::<_, i64>(
            r#"
SELECT COUNT(*)::BIGINT
FROM user_invite_relations
WHERE inviter_user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        let rebate_total = sqlx::query_scalar::<_, Option<Decimal>>(
            r#"
SELECT COALESCE(SUM(rebate_amount), 0)::NUMERIC(18,2)
FROM invite_rebate_records
WHERE inviter_user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(Decimal::ZERO);

        Ok(Some(InviteSummary {
            code: row.code,
            enabled: row.enabled,
            invited_count,
            rebate_total: normalize_invite_money(rebate_total),
            invitee_bonus_enabled: self.config_snapshot().auth.invite.invitee_bonus_enabled,
        }))
    }

    pub async fn list_invite_relations(&self, limit: i64) -> anyhow::Result<Vec<InviteRelationView>> {
        let safe_limit = limit.clamp(1, 500);
        let rows = sqlx::query_as::<_, InviteRelationRow>(
            r#"
SELECT
    r.id,
    r.inviter_user_id,
    inviter.username AS inviter_username,
    r.invitee_user_id,
    invitee.username AS invitee_username,
    r.invite_code,
    r.created_at
FROM user_invite_relations r
JOIN users inviter ON inviter.id = r.inviter_user_id
JOIN users invitee ON invitee.id = r.invitee_user_id
ORDER BY r.created_at DESC
LIMIT $1
            "#,
        )
        .bind(safe_limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn list_invite_rebates(&self, limit: i64) -> anyhow::Result<Vec<InviteRebateView>> {
        let safe_limit = limit.clamp(1, 500);
        let rows = sqlx::query_as::<_, InviteRebateRow>(
            r#"
SELECT
    r.id,
    r.invitee_user_id,
    invitee.username AS invitee_username,
    r.inviter_user_id,
    inviter.username AS inviter_username,
    r.recharge_order_id,
    r.recharge_amount,
    r.rebate_rate,
    r.rebate_amount,
    r.created_at
FROM invite_rebate_records r
JOIN users inviter ON inviter.id = r.inviter_user_id
JOIN users invitee ON invitee.id = r.invitee_user_id
ORDER BY r.created_at DESC
LIMIT $1
            "#,
        )
        .bind(safe_limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn apply_inviter_first_recharge_rebate_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        invitee_user_id: Uuid,
        recharge_order_id: Uuid,
        recharge_amount: Decimal,
        provider_trade_no: Option<&str>,
    ) -> anyhow::Result<Option<Decimal>> {
        if !self.config_snapshot().auth.invite.inviter_rebate_enabled {
            return Ok(None);
        }

        let rebate_rate = normalize_ratio(self.config_snapshot().auth.invite.inviter_rebate_rate);
        if rebate_rate <= Decimal::ZERO {
            return Ok(None);
        }

        let relation = sqlx::query_as::<_, InviteRelationBindingRow>(
            r#"
SELECT inviter_user_id, invite_code
FROM user_invite_relations
WHERE invitee_user_id = $1
LIMIT 1
FOR UPDATE
            "#,
        )
        .bind(invitee_user_id)
        .fetch_optional(&mut **tx)
        .await?;

        let Some(relation) = relation else {
            return Ok(None);
        };

        let already_rebated: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM invite_rebate_records WHERE invitee_user_id = $1 LIMIT 1")
                .bind(invitee_user_id)
                .fetch_optional(&mut **tx)
                .await?;
        if already_rebated.is_some() {
            return Ok(None);
        }

        let rebate_amount = normalize_invite_money(recharge_amount * rebate_rate);
        if rebate_amount <= Decimal::ZERO {
            return Ok(None);
        }

        let mut inviter_wallet =
            Self::ensure_wallet_account_locked_tx(tx, relation.inviter_user_id).await?;
        inviter_wallet.balance = normalize_invite_money(inviter_wallet.balance + rebate_amount);

        sqlx::query(
            r#"
UPDATE wallet_accounts
SET balance = $2, total_recharged = $3, total_spent = $4, updated_at = now()
WHERE user_id = $1
            "#,
        )
        .bind(inviter_wallet.user_id)
        .bind(inviter_wallet.balance)
        .bind(inviter_wallet.total_recharged)
        .bind(inviter_wallet.total_spent)
        .execute(&mut **tx)
        .await?;

        let recharge_ref = recharge_order_id.to_string();
        Self::append_wallet_ledger_tx(
            tx,
            inviter_wallet.user_id,
            "invite_rebate",
            rebate_amount,
            inviter_wallet.balance,
            Some("recharge_order"),
            Some(recharge_ref.as_str()),
            Some("invite first recharge rebate"),
            json!({
                "invitee_user_id": invitee_user_id,
                "invite_code": relation.invite_code,
                "recharge_amount": normalize_invite_money(recharge_amount),
                "rebate_rate": rebate_rate,
                "provider_trade_no": provider_trade_no,
            }),
        )
        .await?;

        sqlx::query(
            r#"
INSERT INTO invite_rebate_records (
    id,
    invitee_user_id,
    inviter_user_id,
    recharge_order_id,
    recharge_amount,
    rebate_rate,
    rebate_amount,
    created_at
) VALUES (
    $1, $2, $3, $4, $5, $6, $7, now()
)
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(invitee_user_id)
        .bind(inviter_wallet.user_id)
        .bind(recharge_order_id)
        .bind(normalize_invite_money(recharge_amount))
        .bind(rebate_rate)
        .bind(rebate_amount)
        .execute(&mut **tx)
        .await?;

        Ok(Some(rebate_amount))
    }

    async fn ensure_user_invite_code(&self, user_id: Uuid) -> anyhow::Result<UserInviteCodeRow> {
        if let Some(row) = sqlx::query_as::<_, UserInviteCodeRow>(
            r#"
SELECT user_id, code, enabled, created_at, updated_at, reset_at
FROM user_invite_codes
WHERE user_id = $1
LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        {
            return Ok(row);
        }

        let mut tx = self.pool.begin().await?;
        let row = Self::ensure_user_invite_code_tx(&mut tx, user_id).await?;
        tx.commit().await?;
        Ok(row)
    }

    async fn ensure_user_invite_code_tx(
        tx: &mut sqlx::Transaction<'_, Postgres>,
        user_id: Uuid,
    ) -> anyhow::Result<UserInviteCodeRow> {
        for _ in 0..INVITE_CODE_RETRY_LIMIT {
            let candidate = generate_invite_code();
            let inserted = sqlx::query_as::<_, UserInviteCodeRow>(
                r#"
INSERT INTO user_invite_codes (user_id, code, enabled, created_at, updated_at)
VALUES ($1, $2, true, now(), now())
ON CONFLICT DO NOTHING
RETURNING user_id, code, enabled, created_at, updated_at, reset_at
                "#,
            )
            .bind(user_id)
            .bind(candidate)
            .fetch_optional(&mut **tx)
            .await?;

            if let Some(row) = inserted {
                return Ok(row);
            }

            if let Some(existing) = sqlx::query_as::<_, UserInviteCodeRow>(
                r#"
SELECT user_id, code, enabled, created_at, updated_at, reset_at
FROM user_invite_codes
WHERE user_id = $1
LIMIT 1
                "#,
            )
            .bind(user_id)
            .fetch_optional(&mut **tx)
            .await?
            {
                return Ok(existing);
            }
        }

        anyhow::bail!("failed to generate unique invite code after retries")
    }

    async fn ensure_wallet_account_locked_tx(
        tx: &mut sqlx::Transaction<'_, Postgres>,
        user_id: Uuid,
    ) -> anyhow::Result<InviteWalletAccountRow> {
        sqlx::query(
            r#"
INSERT INTO wallet_accounts (user_id, balance, total_recharged, total_spent, updated_at)
VALUES ($1, 0, 0, 0, now())
ON CONFLICT (user_id) DO NOTHING
            "#,
        )
        .bind(user_id)
        .execute(&mut **tx)
        .await?;

        let row = sqlx::query_as::<_, InviteWalletAccountRow>(
            r#"
SELECT user_id, balance, total_recharged, total_spent, updated_at
FROM wallet_accounts
WHERE user_id = $1
LIMIT 1
FOR UPDATE
            "#,
        )
        .bind(user_id)
        .fetch_one(&mut **tx)
        .await?;

        Ok(row)
    }
}

fn normalize_invite_code(raw: &str) -> String {
    raw.trim().to_ascii_uppercase()
}

fn generate_invite_code() -> String {
    let raw = Uuid::new_v4().simple().to_string().to_ascii_uppercase();
    raw.chars().take(INVITE_CODE_LENGTH).collect()
}

fn normalize_invite_money(raw: Decimal) -> Decimal {
    raw.round_dp_with_strategy(MONEY_SCALE, rust_decimal::RoundingStrategy::ToZero)
}

fn normalize_ratio(raw: Decimal) -> Decimal {
    raw.clamp(Decimal::ZERO, Decimal::ONE).round_dp(4)
}

#[derive(Debug, Clone, FromRow)]
struct UserInviteCodeRow {
    code: String,
    enabled: bool,
    #[allow(dead_code)]
    created_at: DateTime<Utc>,
    #[allow(dead_code)]
    updated_at: DateTime<Utc>,
    #[allow(dead_code)]
    reset_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow)]
struct InviteWalletAccountRow {
    user_id: Uuid,
    balance: Decimal,
    total_recharged: Decimal,
    total_spent: Decimal,
    #[allow(dead_code)]
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
struct InviteCodeOwnerRow {
    user_id: Uuid,
}

#[derive(Debug, Clone, FromRow)]
struct InviteRelationBindingRow {
    inviter_user_id: Uuid,
    invite_code: String,
}

#[derive(Debug, Clone, FromRow)]
struct InviteRelationRow {
    id: Uuid,
    inviter_user_id: Uuid,
    inviter_username: String,
    invitee_user_id: Uuid,
    invitee_username: String,
    invite_code: String,
    created_at: DateTime<Utc>,
}

impl From<InviteRelationRow> for InviteRelationView {
    fn from(value: InviteRelationRow) -> Self {
        Self {
            id: value.id,
            inviter_user_id: value.inviter_user_id,
            inviter_username: value.inviter_username,
            invitee_user_id: value.invitee_user_id,
            invitee_username: value.invitee_username,
            invite_code: value.invite_code,
            created_at: value.created_at,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
struct InviteRebateRow {
    id: Uuid,
    invitee_user_id: Uuid,
    invitee_username: String,
    inviter_user_id: Uuid,
    inviter_username: String,
    recharge_order_id: Uuid,
    recharge_amount: Decimal,
    rebate_rate: Decimal,
    rebate_amount: Decimal,
    created_at: DateTime<Utc>,
}

impl From<InviteRebateRow> for InviteRebateView {
    fn from(value: InviteRebateRow) -> Self {
        Self {
            id: value.id,
            invitee_user_id: value.invitee_user_id,
            invitee_username: value.invitee_username,
            inviter_user_id: value.inviter_user_id,
            inviter_username: value.inviter_username,
            recharge_order_id: value.recharge_order_id,
            recharge_amount: value.recharge_amount,
            rebate_rate: value.rebate_rate,
            rebate_amount: value.rebate_amount,
            created_at: value.created_at,
        }
    }
}
