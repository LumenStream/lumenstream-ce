impl AppInfra {
    pub async fn list_admin_user_summaries(
        &self,
        query: AdminUserSummaryQuery,
    ) -> anyhow::Result<AdminUserSummaryPage> {
        let safe_page = query.page.max(1);
        let safe_page_size = query.page_size.clamp(1, 200);

        let mut rows = sqlx::query_as::<_, AdminUserSummaryRow>(
            r#"
SELECT
    u.id,
    u.username,
    u.role,
    u.is_admin,
    u.is_disabled,
    u.created_at,
    up.email,
    up.display_name,
    up.remark,
    COALESCE(auth.active_auth_sessions, 0)::BIGINT AS active_auth_sessions,
    COALESCE(playback.active_playback_sessions, 0)::BIGINT AS active_playback_sessions,
    sub.plan_name AS subscription_name,
    COALESCE(usage.used_bytes, 0)::BIGINT AS used_bytes
FROM users u
LEFT JOIN user_profiles up
    ON up.user_id = u.id
LEFT JOIN (
    SELECT user_id, COUNT(*)::BIGINT AS active_auth_sessions
    FROM user_sessions
    WHERE is_active = true
    GROUP BY user_id
) auth
    ON auth.user_id = u.id
LEFT JOIN (
    SELECT user_id, COUNT(*)::BIGINT AS active_playback_sessions
    FROM playback_sessions
    WHERE is_active = true
    GROUP BY user_id
) playback
    ON playback.user_id = u.id
LEFT JOIN LATERAL (
    SELECT plan_name
    FROM billing_plan_subscriptions
    WHERE user_id = u.id
      AND status = 'active'
    ORDER BY started_at DESC
    LIMIT 1
) sub
    ON true
LEFT JOIN (
    SELECT user_id, COALESCE(SUM(bytes_served), 0)::BIGINT AS used_bytes
    FROM user_stream_usage_daily
    GROUP BY user_id
) usage
    ON usage.user_id = u.id
ORDER BY u.created_at DESC, u.username ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let keyword = query
            .q
            .as_ref()
            .map(|v| v.trim().to_ascii_lowercase())
            .filter(|v| !v.is_empty());
        if let Some(value) = keyword.as_deref() {
            rows.retain(|row| {
                row.id.to_string().to_ascii_lowercase().contains(value)
                    || row.username.to_ascii_lowercase().contains(value)
                    || row
                        .email
                        .as_deref()
                        .unwrap_or_default()
                        .to_ascii_lowercase()
                        .contains(value)
                    || row
                        .display_name
                        .as_deref()
                        .unwrap_or_default()
                        .to_ascii_lowercase()
                        .contains(value)
                    || row
                        .remark
                        .as_deref()
                        .unwrap_or_default()
                        .to_ascii_lowercase()
                        .contains(value)
            });
        }

        let status = query
            .status
            .as_ref()
            .map(|v| v.trim().to_ascii_lowercase())
            .filter(|v| !v.is_empty() && v != "all");
        if let Some(status) = status.as_deref() {
            rows.retain(|row| match status {
                "enabled" => !row.is_disabled,
                "disabled" => row.is_disabled,
                _ => true,
            });
        }

        let role_filter = query
            .role
            .as_ref()
            .map(|v| v.trim().to_ascii_lowercase())
            .filter(|v| !v.is_empty() && v != "all");
        if let Some(role) = role_filter.as_deref() {
            rows.retain(|row| row.role.to_ascii_lowercase() == role);
        }

        let mut items: Vec<AdminUserSummaryItem> = rows.into_iter().map(Into::into).collect();
        let sort_by = query
            .sort_by
            .as_deref()
            .map(|v| v.trim().to_ascii_lowercase())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "created_at".to_string());
        let sort_desc = !matches!(
            query
                .sort_dir
                .as_deref()
                .map(|v| v.trim().to_ascii_lowercase())
                .as_deref(),
            Some("asc") | Some("ascending")
        );

        items.sort_by(|left, right| {
            let primary = match sort_by.as_str() {
                "id" => left.id.cmp(&right.id),
                "email" => compare_option_string(left.email.as_deref(), right.email.as_deref()),
                "online_devices" => left
                    .active_playback_sessions
                    .cmp(&right.active_playback_sessions),
                "status" => left.is_disabled.cmp(&right.is_disabled),
                "subscription" => compare_option_string(
                    left.subscription_name.as_deref(),
                    right.subscription_name.as_deref(),
                ),
                "role" => left.role.cmp(&right.role),
                "used_bytes" => left.used_bytes.cmp(&right.used_bytes),
                "username" => left.username.cmp(&right.username),
                _ => left.created_at.cmp(&right.created_at),
            };
            let ordered = if primary == CmpOrdering::Equal {
                left.username
                    .cmp(&right.username)
                    .then(left.id.cmp(&right.id))
            } else {
                primary
            };
            if sort_desc {
                ordered.reverse()
            } else {
                ordered
            }
        });

        let total = i64::try_from(items.len()).unwrap_or(i64::MAX);
        let offset = usize::try_from((safe_page - 1) * safe_page_size).unwrap_or(usize::MAX);
        let limit = usize::try_from(safe_page_size).unwrap_or(usize::MAX);
        let paged_items = if offset >= items.len() {
            Vec::new()
        } else {
            items
                .into_iter()
                .skip(offset)
                .take(limit)
                .collect::<Vec<_>>()
        };

        Ok(AdminUserSummaryPage {
            page: safe_page,
            page_size: safe_page_size,
            total,
            items: paged_items,
        })
    }

    pub async fn get_user_profile(&self, user_id: Uuid) -> anyhow::Result<Option<UserProfile>> {
        let row = sqlx::query_as::<_, UserProfileRow>(
            r#"
SELECT user_id, email, display_name, remark, created_at, updated_at
FROM user_profiles
WHERE user_id = $1
LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn upsert_user_profile(
        &self,
        user_id: Uuid,
        patch: UserProfileUpdate,
    ) -> anyhow::Result<Option<UserProfile>> {
        let user_exists: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM users WHERE id = $1 LIMIT 1")
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await?;
        if user_exists.is_none() {
            return Ok(None);
        }

        let current = self
            .get_user_profile(user_id)
            .await?
            .unwrap_or_else(|| empty_user_profile(user_id));
        let next_email = normalize_optional_text(patch.email.unwrap_or(current.email));
        let next_display_name =
            normalize_optional_text(patch.display_name.unwrap_or(current.display_name));
        let next_remark = normalize_optional_text(patch.remark.unwrap_or(current.remark));

        let row = sqlx::query_as::<_, UserProfileRow>(
            r#"
INSERT INTO user_profiles (user_id, email, display_name, remark)
VALUES ($1, $2, $3, $4)
ON CONFLICT (user_id) DO UPDATE SET
    email = EXCLUDED.email,
    display_name = EXCLUDED.display_name,
    remark = EXCLUDED.remark,
    updated_at = now()
RETURNING user_id, email, display_name, remark, created_at, updated_at
            "#,
        )
        .bind(user_id)
        .bind(next_email)
        .bind(next_display_name)
        .bind(next_remark)
        .fetch_one(&self.pool)
        .await?;

        Ok(Some(row.into()))
    }

    pub async fn update_user_role_and_status(
        &self,
        user_id: Uuid,
        role: Option<UserRole>,
        is_disabled: Option<bool>,
    ) -> anyhow::Result<Option<UserDto>> {
        let role_text = role.as_ref().map(UserRole::as_str);
        let is_admin = role.as_ref().map(|value| matches!(value, UserRole::Admin));

        let row = sqlx::query_as::<_, UserRow>(
            r#"
UPDATE users
SET role = COALESCE($2, role),
    is_admin = COALESCE($3, is_admin),
    is_disabled = COALESCE($4, is_disabled),
    updated_at = now()
WHERE id = $1
RETURNING id, username, password_hash, role, is_admin, is_disabled
            "#,
        )
        .bind(user_id)
        .bind(role_text)
        .bind(is_admin)
        .bind(is_disabled)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|value| to_user_dto(&value, &self.server_id)))
    }

    pub async fn get_user_sessions_summary(
        &self,
        user_id: Uuid,
    ) -> anyhow::Result<UserSessionsSummary> {
        let row = sqlx::query_as::<_, UserSessionsSummaryRow>(
            r#"
SELECT
    COALESCE(
        (
            SELECT COUNT(*)::BIGINT
            FROM user_sessions
            WHERE user_id = $1 AND is_active = true
        ),
        0
    ) AS active_auth_sessions,
    COALESCE(
        (
            SELECT COUNT(*)::BIGINT
            FROM playback_sessions
            WHERE user_id = $1 AND is_active = true
        ),
        0
    ) AS active_playback_sessions,
    (
        SELECT MAX(last_seen_at)
        FROM user_sessions
        WHERE user_id = $1
    ) AS last_auth_seen_at,
    (
        SELECT MAX(updated_at)
        FROM playback_sessions
        WHERE user_id = $1
    ) AS last_playback_seen_at
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }

    pub async fn get_admin_user_manage_profile(
        &self,
        user_id: Uuid,
    ) -> anyhow::Result<Option<AdminUserManageProfile>> {
        let Some(user) = self.get_user_by_id(user_id).await? else {
            return Ok(None);
        };

        let profile = self
            .get_user_profile(user_id)
            .await?
            .unwrap_or_else(|| empty_user_profile(user_id));
        let stream_policy = self
            .get_user_stream_policy(user_id)
            .await?
            .unwrap_or_else(|| self.default_user_stream_policy(user_id));
        let traffic_usage = self.get_user_traffic_usage_summary(user_id, None).await?;
        let wallet = self.ensure_wallet_account(user_id).await?;
        let subscriptions = self.list_user_subscriptions(user_id, 20).await?;
        let sessions_summary = self.get_user_sessions_summary(user_id).await?;

        Ok(Some(AdminUserManageProfile {
            user,
            profile,
            stream_policy,
            traffic_usage,
            wallet,
            subscriptions,
            sessions_summary,
        }))
    }

    pub async fn create_user(
        &self,
        username: &str,
        password: &str,
        role: UserRole,
    ) -> anyhow::Result<UserDto> {
        let user_id = Uuid::now_v7();
        let hash = auth::hash_password(password);
        let row = sqlx::query_as::<_, UserRow>(
            r#"
INSERT INTO users (id, username, password_hash, role, is_admin, is_disabled)
VALUES ($1, $2, $3, $4, $5, false)
RETURNING id, username, password_hash, role, is_admin, is_disabled
            "#,
        )
        .bind(user_id)
        .bind(username)
        .bind(hash)
        .bind(role.as_str())
        .bind(matches!(role, UserRole::Admin))
        .fetch_one(&self.pool)
        .await?;

        self.ensure_default_user_stream_policy(user_id).await?;
        self.ensure_default_playlist(user_id).await?;

        Ok(to_user_dto(&row, &self.server_id))
    }

    pub async fn set_user_disabled(
        &self,
        user_id: Uuid,
        disabled: bool,
    ) -> anyhow::Result<Option<UserDto>> {
        let row = sqlx::query_as::<_, UserRow>(
            r#"
UPDATE users
SET is_disabled = $2, updated_at = now()
WHERE id = $1
RETURNING id, username, password_hash, role, is_admin, is_disabled
            "#,
        )
        .bind(user_id)
        .bind(disabled)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|v| to_user_dto(&v, &self.server_id)))
    }

    pub async fn set_users_disabled_bulk(
        &self,
        user_ids: &[Uuid],
        disabled: bool,
    ) -> anyhow::Result<Vec<UserDto>> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }

        let rows = sqlx::query_as::<_, UserRow>(
            r#"
UPDATE users
SET is_disabled = $2, updated_at = now()
WHERE id = ANY($1)
RETURNING id, username, password_hash, role, is_admin, is_disabled
            "#,
        )
        .bind(user_ids)
        .bind(disabled)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .iter()
            .map(|row| to_user_dto(row, &self.server_id))
            .collect())
    }

    pub async fn delete_user(&self, user_id: Uuid) -> anyhow::Result<bool> {
        let deleted = sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await?
            .rows_affected();
        Ok(deleted > 0)
    }

    pub async fn update_user_password(
        &self,
        user_id: Uuid,
        new_password: &str,
    ) -> anyhow::Result<Option<UserDto>> {
        let hash = auth::hash_password(new_password);
        let row = sqlx::query_as::<_, UserRow>(
            r#"
UPDATE users
SET password_hash = $2, updated_at = now()
WHERE id = $1
RETURNING id, username, password_hash, role, is_admin, is_disabled
            "#,
        )
        .bind(user_id)
        .bind(hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|v| to_user_dto(&v, &self.server_id)))
    }

    pub async fn verify_user_password(
        &self,
        user_id: Uuid,
        password: &str,
    ) -> anyhow::Result<PasswordCheckResult> {
        let row: Option<String> =
            sqlx::query_scalar("SELECT password_hash FROM users WHERE id = $1 LIMIT 1")
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await?;

        match row {
            Some(hash) => match auth::verify_password(password, &hash) {
                auth::PasswordVerifyOutcome::Verified => Ok(PasswordCheckResult::Valid),
                auth::PasswordVerifyOutcome::Invalid => Ok(PasswordCheckResult::Invalid),
                auth::PasswordVerifyOutcome::ResetRequired => {
                    Ok(PasswordCheckResult::PasswordResetRequired)
                }
            },
            None => Ok(PasswordCheckResult::Invalid),
        }
    }

    pub async fn update_user_policy(
        &self,
        user_id: Uuid,
        is_admin: Option<bool>,
        is_disabled: Option<bool>,
    ) -> anyhow::Result<Option<UserDto>> {
        let current = self.get_user_by_id(user_id).await?;
        let Some(_) = current else {
            return Ok(None);
        };

        let row = sqlx::query_as::<_, UserRow>(
            r#"
UPDATE users
SET is_admin = COALESCE($2, is_admin),
    is_disabled = COALESCE($3, is_disabled),
    role = CASE WHEN $2 = true THEN 'Admin' ELSE role END,
    updated_at = now()
WHERE id = $1
RETURNING id, username, password_hash, role, is_admin, is_disabled
            "#,
        )
        .bind(user_id)
        .bind(is_admin)
        .bind(is_disabled)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|v| to_user_dto(&v, &self.server_id)))
    }

    /// Get user avatar path if it exists
    pub async fn get_user_avatar_path(&self, user_id: Uuid) -> anyhow::Result<Option<String>> {
        let avatar_dir = Path::new(&self.config_snapshot().storage.s3_cache_dir).join("avatars");
        for ext in &["png", "jpg", "jpeg", "webp", "gif"] {
            let path = avatar_dir.join(format!("{}.{}", user_id, ext));
            if path.exists() {
                return Ok(Some(path.to_string_lossy().to_string()));
            }
        }
        Ok(None)
    }

    /// Save user avatar image
    pub async fn save_user_avatar(
        &self,
        user_id: Uuid,
        data: &[u8],
        extension: &str,
    ) -> anyhow::Result<()> {
        let avatar_dir = Path::new(&self.config_snapshot().storage.s3_cache_dir).join("avatars");
        tokio::fs::create_dir_all(&avatar_dir).await?;

        // Remove any existing avatar files for this user
        self.delete_user_avatar(user_id).await?;

        let path = avatar_dir.join(format!("{}.{}", user_id, extension));
        tokio::fs::write(&path, data).await?;
        Ok(())
    }

    /// Delete user avatar image
    pub async fn delete_user_avatar(&self, user_id: Uuid) -> anyhow::Result<()> {
        let avatar_dir = Path::new(&self.config_snapshot().storage.s3_cache_dir).join("avatars");
        for ext in &["png", "jpg", "jpeg", "webp", "gif"] {
            let path = avatar_dir.join(format!("{}.{}", user_id, ext));
            if path.exists() {
                let _ = tokio::fs::remove_file(&path).await;
            }
        }
        Ok(())
    }

}
