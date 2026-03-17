//! Billing expiration tasks for the scheduler.

use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub struct ExpiredRechargeOrder {
    pub id: Uuid,
}

/// Result of an expiration task.
#[derive(Debug, Clone, Default)]
pub struct ExpirationResult {
    pub expired_count: u64,
    pub expired_recharge_orders: Vec<ExpiredRechargeOrder>,
}

/// Expire pending recharge orders that have passed their expiration time.
///
/// Sets `status = 'expired'` for orders where `expires_at < now()` and `status = 'pending'`.
pub async fn expire_pending_orders(pool: &PgPool) -> anyhow::Result<ExpirationResult> {
    let expired_ids = sqlx::query_scalar::<_, Uuid>(
        r#"
UPDATE billing_recharge_orders
SET status = 'expired', updated_at = now()
WHERE status = 'pending' AND expires_at < now()
RETURNING id
        "#,
    )
    .fetch_all(pool)
    .await?;

    let expired_count = expired_ids.len() as u64;

    if expired_count > 0 {
        info!(expired_count, "expired pending recharge orders");
    }

    Ok(ExpirationResult {
        expired_count,
        expired_recharge_orders: expired_ids
            .into_iter()
            .map(|id| ExpiredRechargeOrder { id })
            .collect(),
    })
}

/// Expire active subscriptions that have passed their expiration time.
///
/// Sets `status = 'expired'` for subscriptions where `expires_at < now()` and `status = 'active'`.
pub async fn expire_subscriptions(pool: &PgPool) -> anyhow::Result<ExpirationResult> {
    let result = sqlx::query(
        r#"
UPDATE billing_plan_subscriptions
SET status = 'expired', updated_at = now()
WHERE status = 'active' AND expires_at < now()
        "#,
    )
    .execute(pool)
    .await?;

    let expired_count = result.rows_affected();

    if expired_count > 0 {
        info!(expired_count, "expired active subscriptions");
    }

    Ok(ExpirationResult {
        expired_count,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expiration_result_default_is_zero() {
        let result = ExpirationResult::default();
        assert_eq!(result.expired_count, 0);
    }

    #[test]
    fn expiration_result_can_be_constructed() {
        let result = ExpirationResult {
            expired_count: 42,
            expired_recharge_orders: Vec::new(),
        };
        assert_eq!(result.expired_count, 42);
    }
}
