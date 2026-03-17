use std::collections::BTreeMap;

use anyhow::Context;
use chrono::{DateTime, Duration, Utc};
use rust_decimal::{Decimal, RoundingStrategy};
use serde_json::{Value, json};
use sqlx::{FromRow, Postgres, Transaction};
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::{
    AppInfra, BillingPlan, BillingPlanSubscription, BillingPlanUpsert, BillingProration,
    BillingPurchaseResult, BillingRechargeOrder, BillingRechargeOrderFilter, EpayCheckout,
    InfraError, MONEY_SCALE, RechargeOrderEvent, UserStreamPolicyRow, WalletAccount,
    WalletLedgerEntry, normalize_default_optional_i32, normalize_traffic_window_days,
};

impl AppInfra {
    pub fn subscribe_recharge_orders(&self) -> broadcast::Receiver<RechargeOrderEvent> {
        self.recharge_order_tx.subscribe()
    }

    pub(crate) fn publish_recharge_order_event(&self, event: &str, order: BillingRechargeOrder) {
        let _ = self.recharge_order_tx.send(RechargeOrderEvent {
            event: event.to_string(),
            order,
            emitted_at: Utc::now(),
        });
    }

    fn ensure_billing_enabled(&self) -> anyhow::Result<()> {
        if self.billing_feature_enabled() {
            Ok(())
        } else {
            Err(anyhow::Error::new(InfraError::BillingDisabled))
        }
    }

    pub async fn get_wallet_account(&self, user_id: Uuid) -> anyhow::Result<Option<WalletAccount>> {
        let row = sqlx::query_as::<_, WalletAccountRow>(
            r#"
SELECT user_id, balance, total_recharged, total_spent, updated_at
FROM wallet_accounts
WHERE user_id = $1
LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn ensure_wallet_account(
        &self,
        user_id: Uuid,
    ) -> anyhow::Result<Option<WalletAccount>> {
        let user_exists: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM users WHERE id = $1 LIMIT 1")
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await?;
        if user_exists.is_none() {
            return Ok(None);
        }

        sqlx::query(
            r#"
INSERT INTO wallet_accounts (user_id, balance, total_recharged, total_spent, updated_at)
VALUES ($1, 0, 0, 0, now())
ON CONFLICT (user_id) DO NOTHING
            "#,
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        self.get_wallet_account(user_id).await
    }

    pub async fn list_wallet_ledger(
        &self,
        user_id: Uuid,
        limit: i64,
    ) -> anyhow::Result<Vec<WalletLedgerEntry>> {
        let safe_limit = limit.clamp(1, 500);

        let rows = sqlx::query_as::<_, WalletLedgerEntryRow>(
            r#"
SELECT id, user_id, entry_type, amount, balance_after, reference_type, reference_id, note, meta, created_at
FROM wallet_ledger
WHERE user_id = $1
ORDER BY created_at DESC
LIMIT $2
            "#,
        )
        .bind(user_id)
        .bind(safe_limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn list_billing_plans(
        &self,
        include_disabled: bool,
    ) -> anyhow::Result<Vec<BillingPlan>> {
        let rows = sqlx::query_as::<_, BillingPlanRow>(
            r#"
SELECT
    p.id,
    p.code,
    p.name,
    p.price,
    p.duration_days,
    p.traffic_quota_bytes,
    p.traffic_window_days,
    p.permission_group_id,
    g.name AS permission_group_name,
    p.enabled,
    p.updated_at
FROM billing_plans p
LEFT JOIN account_permission_groups g ON g.id = p.permission_group_id
WHERE ($1::BOOLEAN = true OR p.enabled = true)
ORDER BY p.price ASC, p.duration_days ASC, p.created_at ASC
            "#,
        )
        .bind(include_disabled)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn upsert_billing_plan(
        &self,
        payload: BillingPlanUpsert,
    ) -> anyhow::Result<BillingPlan> {
        let code = payload.code.trim().to_ascii_lowercase();
        let name = payload.name.trim().to_string();
        let price = normalize_money(payload.price);
        let duration_days = payload.duration_days;
        let traffic_quota_bytes = payload.traffic_quota_bytes;
        let traffic_window_days = normalize_traffic_window_days(payload.traffic_window_days);
        let permission_group_id = payload.permission_group_id;

        if code.is_empty()
            || name.is_empty()
            || price <= Decimal::ZERO
            || duration_days <= 0
            || traffic_quota_bytes <= 0
        {
            return Err(anyhow::Error::new(InfraError::BillingInvalidAmount));
        }

        if let Some(group_id) = permission_group_id {
            let group_exists: bool = sqlx::query_scalar(
                r#"
SELECT EXISTS(
    SELECT 1
    FROM account_permission_groups
    WHERE id = $1 AND enabled = true
)
                "#,
            )
            .bind(group_id)
            .fetch_one(&self.pool)
            .await?;
            if !group_exists {
                anyhow::bail!("account permission group not found");
            }

            let has_domains: bool = sqlx::query_scalar(
                r#"
SELECT EXISTS(
    SELECT 1
    FROM account_permission_group_playback_domains
    WHERE group_id = $1
)
                "#,
            )
            .bind(group_id)
            .fetch_one(&self.pool)
            .await?;
            if !has_domains {
                anyhow::bail!("account permission group has no playback domains");
            }
        }

        let row = if let Some(plan_id) = payload.id {
            sqlx::query_as::<_, BillingPlanRow>(
                r#"
UPDATE billing_plans
SET
    code = $2,
    name = $3,
    price = $4,
    duration_days = $5,
    traffic_quota_bytes = $6,
    traffic_window_days = $7,
    permission_group_id = $8,
    enabled = $9,
    updated_at = now()
WHERE id = $1
RETURNING
    id,
    code,
    name,
    price,
    duration_days,
    traffic_quota_bytes,
    traffic_window_days,
    permission_group_id,
    NULL::TEXT AS permission_group_name,
    enabled,
    updated_at
                "#,
            )
            .bind(plan_id)
            .bind(&code)
            .bind(&name)
            .bind(price)
            .bind(duration_days)
            .bind(traffic_quota_bytes)
            .bind(traffic_window_days)
            .bind(permission_group_id)
            .bind(payload.enabled)
            .fetch_one(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, BillingPlanRow>(
                r#"
INSERT INTO billing_plans (
    id,
    code,
    name,
    price,
    duration_days,
    traffic_quota_bytes,
    traffic_window_days,
    permission_group_id,
    enabled,
    created_at,
    updated_at
) VALUES (
    $1, $2, $3, $4, $5, $6, $7, $8, $9, now(), now()
)
RETURNING
    id,
    code,
    name,
    price,
    duration_days,
    traffic_quota_bytes,
    traffic_window_days,
    permission_group_id,
    NULL::TEXT AS permission_group_name,
    enabled,
    updated_at
                "#,
            )
            .bind(Uuid::now_v7())
            .bind(&code)
            .bind(&name)
            .bind(price)
            .bind(duration_days)
            .bind(traffic_quota_bytes)
            .bind(traffic_window_days)
            .bind(permission_group_id)
            .bind(payload.enabled)
            .fetch_one(&self.pool)
            .await?
        };

        Ok(row.into())
    }

    pub async fn create_recharge_order(
        &self,
        user_id: Uuid,
        amount: Decimal,
        channel: Option<&str>,
        subject: Option<&str>,
        remote_addr: Option<&str>,
    ) -> anyhow::Result<EpayCheckout> {
        self.ensure_billing_enabled()?;

        let billing_cfg = &self.config_snapshot().billing;
        let epay_cfg = &billing_cfg.epay;
        if epay_cfg.gateway_url.trim().is_empty()
            || epay_cfg.pid.trim().is_empty()
            || epay_cfg.key.trim().is_empty()
            || epay_cfg.notify_url.trim().is_empty()
            || epay_cfg.return_url.trim().is_empty()
        {
            anyhow::bail!("billing.epay settings are incomplete");
        }

        let normalized_amount = normalize_money(amount);
        let min_amount = normalize_money(billing_cfg.min_recharge_amount);
        let max_amount = normalize_money(billing_cfg.max_recharge_amount);

        if normalized_amount <= Decimal::ZERO
            || normalized_amount < min_amount
            || normalized_amount > max_amount
        {
            return Err(anyhow::Error::new(InfraError::BillingInvalidAmount));
        }

        let user_exists: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM users WHERE id = $1 LIMIT 1")
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await?;
        if user_exists.is_none() {
            anyhow::bail!("user not found");
        }

        let requested_channel = channel
            .map(|v| v.trim().to_ascii_lowercase())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| {
                billing_cfg
                    .channels
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "alipay".to_string())
            });
        let channel_allowed = billing_cfg
            .channels
            .iter()
            .any(|entry| entry.eq_ignore_ascii_case(&requested_channel));
        if !channel_allowed {
            return Err(anyhow::Error::new(InfraError::BillingChannelUnsupported));
        }

        let subject = subject
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| "NMS余额充值".to_string());

        let now = Utc::now();
        let out_trade_no = format!(
            "LS{}{}",
            now.format("%Y%m%d%H%M%S"),
            Uuid::now_v7().simple()
        );
        let expires_at = now + Duration::minutes(billing_cfg.order_expire_minutes.max(1));

        let order_row = sqlx::query_as::<_, BillingRechargeOrderRow>(
            r#"
INSERT INTO billing_recharge_orders (
    id,
    user_id,
    out_trade_no,
    channel,
    amount,
    status,
    subject,
    notify_payload,
    provider_trade_no,
    paid_at,
    expires_at,
    created_at,
    updated_at
) VALUES (
    $1, $2, $3, $4, $5, 'pending', $6, '{}'::jsonb, NULL, NULL, $7, now(), now()
)
RETURNING id, user_id, out_trade_no, channel, amount, status, subject, provider_trade_no, paid_at, expires_at, created_at, updated_at
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(user_id)
        .bind(&out_trade_no)
        .bind(&requested_channel)
        .bind(normalized_amount)
        .bind(&subject)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await?;

        let mut pay_params = BTreeMap::new();
        pay_params.insert("pid".to_string(), epay_cfg.pid.clone());
        pay_params.insert("type".to_string(), requested_channel.clone());
        pay_params.insert("out_trade_no".to_string(), out_trade_no);
        pay_params.insert("notify_url".to_string(), epay_cfg.notify_url.clone());
        pay_params.insert("return_url".to_string(), epay_cfg.return_url.clone());
        pay_params.insert("name".to_string(), subject);
        pay_params.insert("money".to_string(), format_money(normalized_amount));
        if !epay_cfg.sitename.trim().is_empty() {
            pay_params.insert("sitename".to_string(), epay_cfg.sitename.clone());
        }
        if let Some(ip) = remote_addr.filter(|v| !v.trim().is_empty()) {
            pay_params.insert("clientip".to_string(), ip.trim().to_string());
        }

        let sign = build_epay_sign(&pay_params, &epay_cfg.key);
        pay_params.insert("sign".to_string(), sign);
        pay_params.insert("sign_type".to_string(), "MD5".to_string());

        let pay_url = build_epay_submit_url(&epay_cfg.gateway_url, &pay_params)?;
        let order: BillingRechargeOrder = order_row.into();
        self.publish_recharge_order_event("billing.recharge_order.created", order.clone());

        Ok(EpayCheckout {
            order,
            pay_url,
            pay_params: serde_json::to_value(pay_params)
                .context("failed to serialize epay checkout params")?,
        })
    }

    pub async fn get_recharge_order_by_id(
        &self,
        order_id: Uuid,
    ) -> anyhow::Result<Option<BillingRechargeOrder>> {
        let row = sqlx::query_as::<_, BillingRechargeOrderRow>(
            r#"
SELECT id, user_id, out_trade_no, channel, amount, status, subject, provider_trade_no, paid_at, expires_at, created_at, updated_at
FROM billing_recharge_orders
WHERE id = $1
LIMIT 1
            "#,
        )
        .bind(order_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn get_recharge_order_for_user(
        &self,
        user_id: Uuid,
        order_id: Uuid,
    ) -> anyhow::Result<Option<BillingRechargeOrder>> {
        let row = sqlx::query_as::<_, BillingRechargeOrderRow>(
            r#"
SELECT id, user_id, out_trade_no, channel, amount, status, subject, provider_trade_no, paid_at, expires_at, created_at, updated_at
FROM billing_recharge_orders
WHERE id = $1 AND user_id = $2
LIMIT 1
            "#,
        )
        .bind(order_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn list_recharge_orders(
        &self,
        filter: BillingRechargeOrderFilter,
    ) -> anyhow::Result<Vec<BillingRechargeOrder>> {
        let safe_limit = filter.limit.clamp(1, 500);
        let status = filter
            .status
            .as_ref()
            .map(|v| v.trim().to_ascii_lowercase())
            .filter(|v| !v.is_empty());

        let rows = sqlx::query_as::<_, BillingRechargeOrderRow>(
            r#"
SELECT id, user_id, out_trade_no, channel, amount, status, subject, provider_trade_no, paid_at, expires_at, created_at, updated_at
FROM billing_recharge_orders
WHERE ($1::UUID IS NULL OR user_id = $1)
  AND ($2::TEXT IS NULL OR status = $2)
ORDER BY created_at DESC
LIMIT $3
            "#,
        )
        .bind(filter.user_id)
        .bind(status)
        .bind(safe_limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn handle_epay_notify(
        &self,
        payload: &BTreeMap<String, String>,
    ) -> anyhow::Result<BillingRechargeOrder> {
        self.ensure_billing_enabled()?;

        let epay_cfg = &self.config_snapshot().billing.epay;
        if epay_cfg.pid.trim().is_empty() || epay_cfg.key.trim().is_empty() {
            anyhow::bail!("billing.epay settings are incomplete");
        }

        let sign = payload
            .get("sign")
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .ok_or_else(|| anyhow::Error::new(InfraError::BillingSignatureInvalid))?;
        let expected_sign = build_epay_sign(payload, &epay_cfg.key);
        if !sign.eq_ignore_ascii_case(&expected_sign) {
            return Err(anyhow::Error::new(InfraError::BillingSignatureInvalid));
        }

        if let Some(pid) = payload.get("pid") {
            if pid.trim() != epay_cfg.pid.trim() {
                return Err(anyhow::Error::new(InfraError::BillingSignatureInvalid));
            }
        }

        let trade_status = payload
            .get("trade_status")
            .map(|v| v.trim())
            .unwrap_or_default();
        if trade_status != "TRADE_SUCCESS" {
            return Err(anyhow::Error::new(InfraError::BillingSignatureInvalid));
        }

        let out_trade_no = payload
            .get("out_trade_no")
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .ok_or_else(|| anyhow::Error::new(InfraError::BillingOrderNotFound))?;

        let paid_amount = payload
            .get("money")
            .ok_or_else(|| anyhow::Error::new(InfraError::BillingOrderAmountMismatch))?
            .parse::<Decimal>()
            .map(normalize_money)
            .map_err(|_| anyhow::Error::new(InfraError::BillingOrderAmountMismatch))?;

        let provider_trade_no = payload
            .get("trade_no")
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());

        let mut tx = self.pool.begin().await?;
        let Some(mut order) = sqlx::query_as::<_, BillingRechargeOrderRow>(
            r#"
SELECT id, user_id, out_trade_no, channel, amount, status, subject, provider_trade_no, paid_at, expires_at, created_at, updated_at
FROM billing_recharge_orders
WHERE out_trade_no = $1
LIMIT 1
FOR UPDATE
            "#,
        )
        .bind(&out_trade_no)
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Err(anyhow::Error::new(InfraError::BillingOrderNotFound));
        };

        if order.status == "paid" {
            tx.commit().await?;
            return Ok(order.into());
        }

        if order.amount != paid_amount {
            return Err(anyhow::Error::new(InfraError::BillingOrderAmountMismatch));
        }

        let notify_payload =
            serde_json::to_value(payload).context("failed to serialize epay notify payload")?;

        order = sqlx::query_as::<_, BillingRechargeOrderRow>(
            r#"
UPDATE billing_recharge_orders
SET
    status = 'paid',
    provider_trade_no = COALESCE($2, provider_trade_no),
    paid_at = COALESCE(paid_at, now()),
    notify_payload = $3,
    updated_at = now()
WHERE id = $1
RETURNING id, user_id, out_trade_no, channel, amount, status, subject, provider_trade_no, paid_at, expires_at, created_at, updated_at
            "#,
        )
        .bind(order.id)
        .bind(provider_trade_no.clone())
        .bind(notify_payload)
        .fetch_one(&mut *tx)
        .await?;

        let mut wallet = Self::ensure_wallet_account_row_locked(&mut tx, order.user_id).await?;
        wallet.balance = normalize_money(wallet.balance + order.amount);
        wallet.total_recharged = normalize_money(wallet.total_recharged + order.amount);

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

        let order_ref = order.id.to_string();
        Self::append_wallet_ledger_tx(
            &mut tx,
            wallet.user_id,
            "recharge",
            order.amount,
            wallet.balance,
            Some("recharge_order"),
            Some(order_ref.as_str()),
            Some("epay recharge success"),
            json!({
                "out_trade_no": order.out_trade_no,
                "provider_trade_no": provider_trade_no,
            }),
        )
        .await?;

        let _ = self
            .apply_inviter_first_recharge_rebate_tx(
                &mut tx,
                wallet.user_id,
                order.id,
                order.amount,
                provider_trade_no.as_deref(),
            )
            .await?;

        tx.commit().await?;
        let order: BillingRechargeOrder = order.into();
        self.publish_recharge_order_event("billing.recharge_order.updated", order.clone());
        Ok(order)
    }

    pub async fn get_active_subscription(
        &self,
        user_id: Uuid,
    ) -> anyhow::Result<Option<BillingPlanSubscription>> {
        let row = sqlx::query_as::<_, BillingPlanSubscriptionRow>(
            r#"
SELECT id, user_id, plan_id, plan_code, plan_name, plan_price, duration_days, traffic_quota_bytes, traffic_window_days, status, started_at, expires_at, replaced_at, updated_at
FROM billing_plan_subscriptions
WHERE user_id = $1 AND status = 'active'
ORDER BY started_at DESC
LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn list_user_subscriptions(
        &self,
        user_id: Uuid,
        limit: i64,
    ) -> anyhow::Result<Vec<BillingPlanSubscription>> {
        let safe_limit = limit.clamp(1, 100);
        let rows = sqlx::query_as::<_, BillingPlanSubscriptionRow>(
            r#"
SELECT id, user_id, plan_id, plan_code, plan_name, plan_price, duration_days, traffic_quota_bytes, traffic_window_days, status, started_at, expires_at, replaced_at, updated_at
FROM billing_plan_subscriptions
WHERE user_id = $1
ORDER BY created_at DESC
LIMIT $2
            "#,
        )
        .bind(user_id)
        .bind(safe_limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn purchase_plan_with_balance(
        &self,
        user_id: Uuid,
        plan_id: Uuid,
    ) -> anyhow::Result<BillingPurchaseResult> {
        self.ensure_billing_enabled()?;

        let mut tx = self.pool.begin().await?;

        let user_exists: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM users WHERE id = $1 LIMIT 1")
                .bind(user_id)
                .fetch_optional(&mut *tx)
                .await?;
        if user_exists.is_none() {
            anyhow::bail!("user not found");
        }

        let Some(plan) = sqlx::query_as::<_, BillingPlanRow>(
            r#"
SELECT
    id,
    code,
    name,
    price,
    duration_days,
    traffic_quota_bytes,
    traffic_window_days,
    permission_group_id,
    NULL::TEXT AS permission_group_name,
    enabled,
    updated_at
FROM billing_plans
WHERE id = $1 AND enabled = true
LIMIT 1
FOR UPDATE
            "#,
        )
        .bind(plan_id)
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Err(anyhow::Error::new(InfraError::BillingPlanNotFound));
        };

        let mut wallet = Self::ensure_wallet_account_row_locked(&mut tx, user_id).await?;

        let active_subscription = sqlx::query_as::<_, BillingPlanSubscriptionRow>(
            r#"
SELECT id, user_id, plan_id, plan_code, plan_name, plan_price, duration_days, traffic_quota_bytes, traffic_window_days, status, started_at, expires_at, replaced_at, updated_at
FROM billing_plan_subscriptions
WHERE user_id = $1 AND status = 'active'
ORDER BY started_at DESC
LIMIT 1
FOR UPDATE
            "#,
        )
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?;

        let now = Utc::now();

        let proration = if let Some(sub) = active_subscription.as_ref() {
            let used_bytes =
                Self::sum_user_traffic_usage_bytes_tx(&mut tx, user_id, sub.traffic_window_days)
                    .await?;
            Some(calculate_proration(now, sub, used_bytes))
        } else {
            None
        };

        if let Some(proration) = proration.as_ref() {
            if proration.credit_amount > Decimal::ZERO {
                wallet.balance = normalize_money(wallet.balance + proration.credit_amount);

                let active_subscription_ref =
                    active_subscription.as_ref().map(|row| row.id.to_string());
                Self::append_wallet_ledger_tx(
                    &mut tx,
                    user_id,
                    "plan_switch_credit",
                    proration.credit_amount,
                    wallet.balance,
                    Some("subscription"),
                    active_subscription_ref.as_deref(),
                    Some("prorated credit from active plan"),
                    json!({
                        "time_ratio": proration.time_ratio,
                        "traffic_ratio": proration.traffic_ratio,
                        "applied_ratio": proration.applied_ratio,
                        "traffic_used_bytes": proration.traffic_used_bytes,
                        "traffic_remaining_bytes": proration.traffic_remaining_bytes,
                    }),
                )
                .await?;
            }
        }

        if wallet.balance < plan.price {
            return Err(anyhow::Error::new(InfraError::BillingInsufficientBalance));
        }

        wallet.balance = normalize_money(wallet.balance - plan.price);
        wallet.total_spent = normalize_money(wallet.total_spent + plan.price);

        sqlx::query(
            r#"
UPDATE wallet_accounts
SET balance = $2, total_recharged = $3, total_spent = $4, updated_at = now()
WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .bind(wallet.balance)
        .bind(wallet.total_recharged)
        .bind(wallet.total_spent)
        .execute(&mut *tx)
        .await?;

        if let Some(sub) = active_subscription.as_ref() {
            sqlx::query(
                r#"
UPDATE billing_plan_subscriptions
SET status = 'replaced', replaced_at = now(), updated_at = now()
WHERE id = $1 AND status = 'active'
                "#,
            )
            .bind(sub.id)
            .execute(&mut *tx)
            .await?;
        }

        let expires_at = now + Duration::days(i64::from(plan.duration_days));
        let new_subscription = sqlx::query_as::<_, BillingPlanSubscriptionRow>(
            r#"
INSERT INTO billing_plan_subscriptions (
    id,
    user_id,
    plan_id,
    plan_code,
    plan_name,
    plan_price,
    duration_days,
    traffic_quota_bytes,
    traffic_window_days,
    status,
    started_at,
    expires_at,
    replaced_at,
    created_at,
    updated_at
) VALUES (
    $1, $2, $3, $4, $5, $6, $7, $8, $9, 'active', $10, $11, NULL, now(), now()
)
RETURNING id, user_id, plan_id, plan_code, plan_name, plan_price, duration_days, traffic_quota_bytes, traffic_window_days, status, started_at, expires_at, replaced_at, updated_at
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(user_id)
        .bind(plan.id)
        .bind(&plan.code)
        .bind(&plan.name)
        .bind(plan.price)
        .bind(plan.duration_days)
        .bind(plan.traffic_quota_bytes)
        .bind(plan.traffic_window_days)
        .bind(now)
        .bind(expires_at)
        .fetch_one(&mut *tx)
        .await?;

        let new_subscription_ref = new_subscription.id.to_string();
        Self::append_wallet_ledger_tx(
            &mut tx,
            user_id,
            "plan_purchase",
            -plan.price,
            wallet.balance,
            Some("subscription"),
            Some(new_subscription_ref.as_str()),
            Some("purchase billing plan by wallet"),
            json!({
                "plan_id": plan.id,
                "plan_code": plan.code,
                "plan_name": plan.name,
            }),
        )
        .await?;

        sqlx::query("DELETE FROM user_stream_usage_daily WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        let current_policy = sqlx::query_as::<_, UserStreamPolicyRow>(
            r#"
SELECT user_id, expires_at, max_concurrent_streams, traffic_quota_bytes, traffic_window_days, updated_at
FROM user_stream_policies
WHERE user_id = $1
LIMIT 1
FOR UPDATE
            "#,
        )
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?;

        let max_concurrent_streams = current_policy
            .as_ref()
            .and_then(|row| row.max_concurrent_streams)
            .or(normalize_default_optional_i32(
                self.config_snapshot()
                    .security
                    .default_user_max_concurrent_streams,
            ));
        let traffic_window_days = normalize_traffic_window_days(plan.traffic_window_days);

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
    $1, $2, $3, $4, $5, now()
)
ON CONFLICT (user_id) DO UPDATE SET
    expires_at = EXCLUDED.expires_at,
    max_concurrent_streams = EXCLUDED.max_concurrent_streams,
    traffic_quota_bytes = EXCLUDED.traffic_quota_bytes,
    traffic_window_days = EXCLUDED.traffic_window_days,
    updated_at = now()
            "#,
        )
        .bind(user_id)
        .bind(Some(expires_at))
        .bind(max_concurrent_streams)
        .bind(Some(plan.traffic_quota_bytes))
        .bind(traffic_window_days)
        .execute(&mut *tx)
        .await?;

        let wallet_snapshot = WalletAccount {
            user_id: wallet.user_id,
            balance: wallet.balance,
            total_recharged: wallet.total_recharged,
            total_spent: wallet.total_spent,
            updated_at: now,
        };

        tx.commit().await?;

        Ok(BillingPurchaseResult {
            wallet: wallet_snapshot,
            subscription: new_subscription.into(),
            charged_amount: plan.price,
            proration,
        })
    }

    pub async fn admin_adjust_wallet_balance(
        &self,
        user_id: Uuid,
        delta: Decimal,
        note: Option<&str>,
    ) -> anyhow::Result<Option<WalletAccount>> {
        let normalized_delta = normalize_money(delta);
        if normalized_delta == Decimal::ZERO {
            return Err(anyhow::Error::new(InfraError::BillingInvalidAmount));
        }

        let mut tx = self.pool.begin().await?;

        let user_exists: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM users WHERE id = $1 LIMIT 1")
                .bind(user_id)
                .fetch_optional(&mut *tx)
                .await?;
        if user_exists.is_none() {
            return Ok(None);
        }

        let mut wallet = Self::ensure_wallet_account_row_locked(&mut tx, user_id).await?;

        let next_balance = normalize_money(wallet.balance + normalized_delta);
        if next_balance < Decimal::ZERO {
            return Err(anyhow::Error::new(InfraError::BillingInsufficientBalance));
        }

        wallet.balance = next_balance;
        if normalized_delta < Decimal::ZERO {
            wallet.total_spent = normalize_money(wallet.total_spent + normalized_delta.abs());
        }

        sqlx::query(
            r#"
UPDATE wallet_accounts
SET balance = $2, total_recharged = $3, total_spent = $4, updated_at = now()
WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .bind(wallet.balance)
        .bind(wallet.total_recharged)
        .bind(wallet.total_spent)
        .execute(&mut *tx)
        .await?;

        Self::append_wallet_ledger_tx(
            &mut tx,
            user_id,
            "admin_adjust",
            normalized_delta,
            wallet.balance,
            Some("admin"),
            None,
            note,
            json!({}),
        )
        .await?;

        tx.commit().await?;

        Ok(Some(wallet.into()))
    }

    async fn ensure_wallet_account_row_locked(
        tx: &mut Transaction<'_, Postgres>,
        user_id: Uuid,
    ) -> anyhow::Result<WalletAccountRow> {
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

        let row = sqlx::query_as::<_, WalletAccountRow>(
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

    pub(crate) async fn append_wallet_ledger_tx(
        tx: &mut Transaction<'_, Postgres>,
        user_id: Uuid,
        entry_type: &str,
        amount: Decimal,
        balance_after: Decimal,
        reference_type: Option<&str>,
        reference_id: Option<&str>,
        note: Option<&str>,
        meta: Value,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
INSERT INTO wallet_ledger (
    id,
    user_id,
    entry_type,
    amount,
    balance_after,
    reference_type,
    reference_id,
    note,
    meta,
    created_at
) VALUES (
    $1, $2, $3, $4, $5, $6, $7, $8, $9, now()
)
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(user_id)
        .bind(entry_type)
        .bind(normalize_money(amount))
        .bind(normalize_money(balance_after))
        .bind(reference_type)
        .bind(reference_id)
        .bind(note)
        .bind(meta)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    async fn sum_user_traffic_usage_bytes_tx(
        tx: &mut Transaction<'_, Postgres>,
        user_id: Uuid,
        window_days: i32,
    ) -> anyhow::Result<i64> {
        let safe_window_days = normalize_traffic_window_days(window_days);
        let used_bytes = sqlx::query_scalar::<_, Option<i64>>(
            r#"
SELECT SUM(bytes_served)::BIGINT
FROM user_stream_usage_daily
WHERE user_id = $1
  AND usage_date >= current_date - ($2::INT - 1)
            "#,
        )
        .bind(user_id)
        .bind(safe_window_days)
        .fetch_one(&mut **tx)
        .await?
        .unwrap_or(0);

        Ok(used_bytes.max(0))
    }

    /// Admin: Grant a subscription to a user without charging their wallet.
    /// Replaces any existing active subscription.
    pub async fn admin_grant_subscription(
        &self,
        user_id: Uuid,
        plan_id: Uuid,
        duration_days_override: Option<i32>,
    ) -> anyhow::Result<Option<BillingPlanSubscription>> {
        let mut tx = self.pool.begin().await?;

        let user_exists: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM users WHERE id = $1 LIMIT 1")
                .bind(user_id)
                .fetch_optional(&mut *tx)
                .await?;
        if user_exists.is_none() {
            return Ok(None);
        }

        let Some(plan) = sqlx::query_as::<_, BillingPlanRow>(
            r#"
SELECT
    id,
    code,
    name,
    price,
    duration_days,
    traffic_quota_bytes,
    traffic_window_days,
    permission_group_id,
    NULL::TEXT AS permission_group_name,
    enabled,
    updated_at
FROM billing_plans
WHERE id = $1
LIMIT 1
            "#,
        )
        .bind(plan_id)
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Err(anyhow::Error::new(InfraError::BillingPlanNotFound));
        };

        // Replace any existing active subscription
        sqlx::query(
            r#"
UPDATE billing_plan_subscriptions
SET status = 'replaced', replaced_at = now(), updated_at = now()
WHERE user_id = $1 AND status = 'active'
            "#,
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        let now = Utc::now();
        let duration_days = duration_days_override
            .filter(|&d| d > 0)
            .unwrap_or(plan.duration_days);
        let expires_at = now + Duration::days(i64::from(duration_days));

        let new_subscription = sqlx::query_as::<_, BillingPlanSubscriptionRow>(
            r#"
INSERT INTO billing_plan_subscriptions (
    id,
    user_id,
    plan_id,
    plan_code,
    plan_name,
    plan_price,
    duration_days,
    traffic_quota_bytes,
    traffic_window_days,
    status,
    started_at,
    expires_at,
    replaced_at,
    created_at,
    updated_at
) VALUES (
    $1, $2, $3, $4, $5, $6, $7, $8, $9, 'active', $10, $11, NULL, now(), now()
)
RETURNING id, user_id, plan_id, plan_code, plan_name, plan_price, duration_days, traffic_quota_bytes, traffic_window_days, status, started_at, expires_at, replaced_at, updated_at
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(user_id)
        .bind(plan.id)
        .bind(&plan.code)
        .bind(&plan.name)
        .bind(plan.price)
        .bind(duration_days)
        .bind(plan.traffic_quota_bytes)
        .bind(plan.traffic_window_days)
        .bind(now)
        .bind(expires_at)
        .fetch_one(&mut *tx)
        .await?;

        // Clear traffic usage for fresh start
        sqlx::query("DELETE FROM user_stream_usage_daily WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        // Update user stream policy
        let current_policy = sqlx::query_as::<_, UserStreamPolicyRow>(
            r#"
SELECT user_id, expires_at, max_concurrent_streams, traffic_quota_bytes, traffic_window_days, updated_at
FROM user_stream_policies
WHERE user_id = $1
LIMIT 1
FOR UPDATE
            "#,
        )
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?;

        let max_concurrent_streams = current_policy
            .as_ref()
            .and_then(|row| row.max_concurrent_streams)
            .or(normalize_default_optional_i32(
                self.config_snapshot()
                    .security
                    .default_user_max_concurrent_streams,
            ));
        let traffic_window_days = normalize_traffic_window_days(plan.traffic_window_days);

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
    $1, $2, $3, $4, $5, now()
)
ON CONFLICT (user_id) DO UPDATE SET
    expires_at = EXCLUDED.expires_at,
    max_concurrent_streams = EXCLUDED.max_concurrent_streams,
    traffic_quota_bytes = EXCLUDED.traffic_quota_bytes,
    traffic_window_days = EXCLUDED.traffic_window_days,
    updated_at = now()
            "#,
        )
        .bind(user_id)
        .bind(Some(expires_at))
        .bind(max_concurrent_streams)
        .bind(Some(plan.traffic_quota_bytes))
        .bind(traffic_window_days)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(Some(new_subscription.into()))
    }

    /// Admin: Update a subscription's expiry date and/or status.
    pub async fn admin_update_subscription(
        &self,
        user_id: Uuid,
        subscription_id: Uuid,
        new_expires_at: Option<DateTime<Utc>>,
        new_status: Option<&str>,
    ) -> anyhow::Result<Option<BillingPlanSubscription>> {
        let valid_statuses = ["active", "replaced", "expired"];
        if let Some(status) = new_status {
            if !valid_statuses.contains(&status) {
                return Err(anyhow::Error::new(InfraError::BillingInvalidAmount));
            }
        }

        let mut tx = self.pool.begin().await?;

        let Some(mut subscription) = sqlx::query_as::<_, BillingPlanSubscriptionRow>(
            r#"
SELECT id, user_id, plan_id, plan_code, plan_name, plan_price, duration_days, traffic_quota_bytes, traffic_window_days, status, started_at, expires_at, replaced_at, updated_at
FROM billing_plan_subscriptions
WHERE id = $1 AND user_id = $2
LIMIT 1
FOR UPDATE
            "#,
        )
        .bind(subscription_id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Ok(None);
        };

        let expires_at = new_expires_at.unwrap_or(subscription.expires_at);
        let status = new_status.unwrap_or(&subscription.status);
        let replaced_at = if status == "replaced" && subscription.replaced_at.is_none() {
            Some(Utc::now())
        } else {
            subscription.replaced_at
        };

        subscription = sqlx::query_as::<_, BillingPlanSubscriptionRow>(
            r#"
UPDATE billing_plan_subscriptions
SET expires_at = $3, status = $4, replaced_at = $5, updated_at = now()
WHERE id = $1 AND user_id = $2
RETURNING id, user_id, plan_id, plan_code, plan_name, plan_price, duration_days, traffic_quota_bytes, traffic_window_days, status, started_at, expires_at, replaced_at, updated_at
            "#,
        )
        .bind(subscription_id)
        .bind(user_id)
        .bind(expires_at)
        .bind(status)
        .bind(replaced_at)
        .fetch_one(&mut *tx)
        .await?;

        // If this was the active subscription and we changed expiry, update stream policy
        if subscription.status == "active" && new_expires_at.is_some() {
            sqlx::query(
                r#"
UPDATE user_stream_policies
SET expires_at = $2, updated_at = now()
WHERE user_id = $1
                "#,
            )
            .bind(user_id)
            .bind(expires_at)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(Some(subscription.into()))
    }

    /// Admin: Cancel (expire) a subscription.
    pub async fn admin_cancel_subscription(
        &self,
        user_id: Uuid,
        subscription_id: Uuid,
    ) -> anyhow::Result<Option<BillingPlanSubscription>> {
        let mut tx = self.pool.begin().await?;

        let Some(subscription) = sqlx::query_as::<_, BillingPlanSubscriptionRow>(
            r#"
UPDATE billing_plan_subscriptions
SET status = 'expired', updated_at = now()
WHERE id = $1 AND user_id = $2
RETURNING id, user_id, plan_id, plan_code, plan_name, plan_price, duration_days, traffic_quota_bytes, traffic_window_days, status, started_at, expires_at, replaced_at, updated_at
            "#,
        )
        .bind(subscription_id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Ok(None);
        };

        // Clear stream policy if this was the active subscription
        sqlx::query(
            r#"
UPDATE user_stream_policies
SET expires_at = now(), updated_at = now()
WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(Some(subscription.into()))
    }

    /// Admin: Get a specific subscription by ID.
    pub async fn admin_get_subscription(
        &self,
        user_id: Uuid,
        subscription_id: Uuid,
    ) -> anyhow::Result<Option<BillingPlanSubscription>> {
        let row = sqlx::query_as::<_, BillingPlanSubscriptionRow>(
            r#"
SELECT id, user_id, plan_id, plan_code, plan_name, plan_price, duration_days, traffic_quota_bytes, traffic_window_days, status, started_at, expires_at, replaced_at, updated_at
FROM billing_plan_subscriptions
WHERE id = $1 AND user_id = $2
LIMIT 1
            "#,
        )
        .bind(subscription_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }
}

fn normalize_money(raw: Decimal) -> Decimal {
    raw.round_dp_with_strategy(MONEY_SCALE, RoundingStrategy::ToZero)
}

fn format_money(amount: Decimal) -> String {
    format!("{:.2}", normalize_money(amount))
}

fn build_epay_sign(params: &BTreeMap<String, String>, key: &str) -> String {
    let mut pairs = Vec::new();
    for (k, v) in params {
        if k == "sign" || k == "sign_type" {
            continue;
        }
        if v.trim().is_empty() {
            continue;
        }
        pairs.push(format!("{k}={v}"));
    }
    pairs.push(format!("key={key}"));
    let sign_raw = pairs.join("&");
    format!("{:x}", crate::md5_compute(sign_raw)).to_uppercase()
}

fn build_epay_submit_url(
    gateway_url: &str,
    params: &BTreeMap<String, String>,
) -> anyhow::Result<String> {
    let trimmed = gateway_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        anyhow::bail!("billing.epay.gateway_url is empty");
    }

    let endpoint = if trimmed.ends_with("submit.php") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/submit.php")
    };

    let mut query_parts = Vec::with_capacity(params.len());
    for (key, value) in params {
        query_parts.push(format!(
            "{}={}",
            urlencoding::encode(key),
            urlencoding::encode(value)
        ));
    }

    Ok(format!("{endpoint}?{}", query_parts.join("&")))
}

fn calculate_proration(
    now: DateTime<Utc>,
    subscription: &BillingPlanSubscriptionRow,
    traffic_used_bytes: i64,
) -> BillingProration {
    let total_seconds = (subscription.expires_at - subscription.started_at)
        .num_seconds()
        .max(1);
    let remaining_seconds = (subscription.expires_at - now).num_seconds().max(0);

    let time_ratio = Decimal::from(remaining_seconds) / Decimal::from(total_seconds);

    let traffic_total = subscription.traffic_quota_bytes.max(1);
    let traffic_remaining = (traffic_total - traffic_used_bytes.max(0)).max(0);
    let traffic_ratio = Decimal::from(traffic_remaining) / Decimal::from(traffic_total);

    let applied_ratio = time_ratio
        .max(Decimal::ZERO)
        .min(Decimal::ONE)
        .min(traffic_ratio.max(Decimal::ZERO).min(Decimal::ONE));
    let credit_amount = normalize_money(subscription.plan_price * applied_ratio);

    BillingProration {
        time_ratio,
        traffic_ratio,
        applied_ratio,
        credit_amount,
        traffic_used_bytes: traffic_used_bytes.max(0),
        traffic_remaining_bytes: traffic_remaining,
    }
}

#[derive(Debug, Clone, FromRow)]
struct WalletAccountRow {
    user_id: Uuid,
    balance: Decimal,
    total_recharged: Decimal,
    total_spent: Decimal,
    updated_at: DateTime<Utc>,
}

impl From<WalletAccountRow> for WalletAccount {
    fn from(value: WalletAccountRow) -> Self {
        Self {
            user_id: value.user_id,
            balance: value.balance,
            total_recharged: value.total_recharged,
            total_spent: value.total_spent,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
struct WalletLedgerEntryRow {
    id: Uuid,
    user_id: Uuid,
    entry_type: String,
    amount: Decimal,
    balance_after: Decimal,
    reference_type: Option<String>,
    reference_id: Option<String>,
    note: Option<String>,
    meta: Value,
    created_at: DateTime<Utc>,
}

impl From<WalletLedgerEntryRow> for WalletLedgerEntry {
    fn from(value: WalletLedgerEntryRow) -> Self {
        Self {
            id: value.id,
            user_id: value.user_id,
            entry_type: value.entry_type,
            amount: value.amount,
            balance_after: value.balance_after,
            reference_type: value.reference_type,
            reference_id: value.reference_id,
            note: value.note,
            meta: value.meta,
            created_at: value.created_at,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
struct BillingPlanRow {
    id: Uuid,
    code: String,
    name: String,
    price: Decimal,
    duration_days: i32,
    traffic_quota_bytes: i64,
    traffic_window_days: i32,
    permission_group_id: Option<Uuid>,
    permission_group_name: Option<String>,
    enabled: bool,
    updated_at: DateTime<Utc>,
}

impl From<BillingPlanRow> for BillingPlan {
    fn from(value: BillingPlanRow) -> Self {
        Self {
            id: value.id,
            code: value.code,
            name: value.name,
            price: value.price,
            duration_days: value.duration_days,
            traffic_quota_bytes: value.traffic_quota_bytes,
            traffic_window_days: value.traffic_window_days,
            permission_group_id: value.permission_group_id,
            permission_group_name: value.permission_group_name,
            enabled: value.enabled,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
struct BillingPlanSubscriptionRow {
    id: Uuid,
    user_id: Uuid,
    plan_id: Uuid,
    plan_code: String,
    plan_name: String,
    plan_price: Decimal,
    duration_days: i32,
    traffic_quota_bytes: i64,
    traffic_window_days: i32,
    status: String,
    started_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    replaced_at: Option<DateTime<Utc>>,
    updated_at: DateTime<Utc>,
}

impl From<BillingPlanSubscriptionRow> for BillingPlanSubscription {
    fn from(value: BillingPlanSubscriptionRow) -> Self {
        Self {
            id: value.id,
            user_id: value.user_id,
            plan_id: value.plan_id,
            plan_code: value.plan_code,
            plan_name: value.plan_name,
            plan_price: value.plan_price,
            duration_days: value.duration_days,
            traffic_quota_bytes: value.traffic_quota_bytes,
            traffic_window_days: value.traffic_window_days,
            status: value.status,
            started_at: value.started_at,
            expires_at: value.expires_at,
            replaced_at: value.replaced_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
struct BillingRechargeOrderRow {
    id: Uuid,
    user_id: Uuid,
    out_trade_no: String,
    channel: String,
    amount: Decimal,
    status: String,
    subject: String,
    provider_trade_no: Option<String>,
    paid_at: Option<DateTime<Utc>>,
    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<BillingRechargeOrderRow> for BillingRechargeOrder {
    fn from(value: BillingRechargeOrderRow) -> Self {
        Self {
            id: value.id,
            user_id: value.user_id,
            out_trade_no: value.out_trade_no,
            channel: value.channel,
            amount: value.amount,
            status: value.status,
            subject: value.subject,
            provider_trade_no: value.provider_trade_no,
            paid_at: value.paid_at,
            expires_at: value.expires_at,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BillingPlanSubscriptionRow, build_epay_sign, build_epay_submit_url, calculate_proration,
        normalize_money,
    };
    use chrono::{Duration, TimeZone, Utc};
    use rust_decimal::Decimal;
    use std::collections::BTreeMap;
    use uuid::Uuid;

    #[test]
    fn normalize_money_truncates_to_two_decimal_places() {
        assert_eq!(normalize_money(Decimal::new(1999, 3)), Decimal::new(199, 2));
        assert_eq!(
            normalize_money(Decimal::new(-1999, 3)),
            Decimal::new(-199, 2)
        );
    }

    #[test]
    fn build_epay_sign_ignores_sign_fields_and_is_stable() {
        let mut params = BTreeMap::new();
        params.insert("pid".to_string(), "10001".to_string());
        params.insert("out_trade_no".to_string(), "NMS001".to_string());
        params.insert("money".to_string(), "10.00".to_string());
        params.insert("sign".to_string(), "old".to_string());
        params.insert("sign_type".to_string(), "MD5".to_string());

        let sign = build_epay_sign(&params, "secret");
        let again = build_epay_sign(&params, "secret");
        assert_eq!(sign, again);
        assert!(!sign.is_empty());
    }

    #[test]
    fn build_epay_submit_url_appends_submit_php() {
        let mut params = BTreeMap::new();
        params.insert("pid".to_string(), "10001".to_string());
        params.insert("money".to_string(), "10.00".to_string());

        let url = build_epay_submit_url("https://pay.example.com", &params).expect("pay url");
        assert!(url.starts_with("https://pay.example.com/submit.php?"));
    }

    #[test]
    fn calculate_proration_uses_min_of_time_and_traffic_ratio() {
        let started_at = Utc
            .with_ymd_and_hms(2026, 1, 1, 0, 0, 0)
            .single()
            .expect("started_at");
        let expires_at = started_at + Duration::days(30);
        let now = started_at + Duration::days(15);

        let sub = BillingPlanSubscriptionRow {
            id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            plan_id: Uuid::now_v7(),
            plan_code: "monthly".to_string(),
            plan_name: "Monthly".to_string(),
            plan_price: Decimal::new(10000, 2),
            duration_days: 30,
            traffic_quota_bytes: 1_000,
            traffic_window_days: 30,
            status: "active".to_string(),
            started_at,
            expires_at,
            replaced_at: None,
            updated_at: started_at,
        };

        let proration = calculate_proration(now, &sub, 800);
        assert_eq!(proration.traffic_remaining_bytes, 200);
        assert!(proration.traffic_ratio < proration.time_ratio);
        assert_eq!(proration.credit_amount, Decimal::new(2000, 2));
    }
}
