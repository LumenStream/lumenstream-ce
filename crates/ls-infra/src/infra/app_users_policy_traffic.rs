impl AppInfra {
    pub async fn get_user_stream_policy(
        &self,
        user_id: Uuid,
    ) -> anyhow::Result<Option<UserStreamPolicy>> {
        let row = sqlx::query_as::<_, UserStreamPolicyRow>(
            r#"
SELECT user_id, expires_at, max_concurrent_streams, traffic_quota_bytes, traffic_window_days, updated_at
FROM user_stream_policies
WHERE user_id = $1
LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn upsert_user_stream_policy(
        &self,
        user_id: Uuid,
        patch: UserStreamPolicyUpdate,
    ) -> anyhow::Result<Option<UserStreamPolicy>> {
        let user_exists: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM users WHERE id = $1 LIMIT 1")
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await?;
        if user_exists.is_none() {
            return Ok(None);
        }

        let current = sqlx::query_as::<_, UserStreamPolicyRow>(
            r#"
SELECT user_id, expires_at, max_concurrent_streams, traffic_quota_bytes, traffic_window_days, updated_at
FROM user_stream_policies
WHERE user_id = $1
LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        let default_window =
            normalize_traffic_window_days(self.config_snapshot().security.default_user_traffic_window_days);

        let current_expires = current
            .as_ref()
            .and_then(|row| row.expires_at.as_ref().cloned());
        let current_max_concurrent = current.as_ref().and_then(|row| row.max_concurrent_streams);
        let current_traffic_quota = current.as_ref().and_then(|row| row.traffic_quota_bytes);
        let current_window_days = current
            .as_ref()
            .map(|row| row.traffic_window_days)
            .unwrap_or(default_window);

        let expires_at = patch.expires_at.unwrap_or(current_expires);
        let max_concurrent_streams = patch
            .max_concurrent_streams
            .unwrap_or(current_max_concurrent);
        let traffic_quota_bytes = patch.traffic_quota_bytes.unwrap_or(current_traffic_quota);
        let traffic_window_days = patch.traffic_window_days.unwrap_or(current_window_days);

        if let Some(limit) = max_concurrent_streams {
            if limit < 0 {
                anyhow::bail!("max_concurrent_streams must be >= 0");
            }
        }

        if let Some(quota) = traffic_quota_bytes {
            if quota < 0 {
                anyhow::bail!("traffic_quota_bytes must be >= 0");
            }
        }

        if traffic_window_days <= 0 {
            anyhow::bail!("traffic_window_days must be > 0");
        }

        let row = sqlx::query_as::<_, UserStreamPolicyRow>(
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
RETURNING user_id, expires_at, max_concurrent_streams, traffic_quota_bytes, traffic_window_days, updated_at
            "#,
        )
        .bind(user_id)
        .bind(expires_at)
        .bind(max_concurrent_streams)
        .bind(traffic_quota_bytes)
        .bind(traffic_window_days)
        .fetch_one(&self.pool)
        .await?;

        Ok(Some(row.into()))
    }

    pub async fn get_user_traffic_usage_summary(
        &self,
        user_id: Uuid,
        window_days: Option<i32>,
    ) -> anyhow::Result<UserTrafficUsageSummary> {
        let policy = self.get_user_stream_policy(user_id).await?;
        let fallback_window = policy
            .as_ref()
            .map(|row| row.traffic_window_days)
            .unwrap_or(self.config_snapshot().security.default_user_traffic_window_days);
        let effective_window_days =
            normalize_traffic_window_days(window_days.unwrap_or(fallback_window));

        let rows = sqlx::query_as::<_, UserTrafficUsageDailyRow>(
            r#"
SELECT usage_date, bytes_served, real_bytes_served
FROM user_stream_usage_daily
WHERE user_id = $1
  AND usage_date >= current_date - ($2::INT - 1)
ORDER BY usage_date ASC
            "#,
        )
        .bind(user_id)
        .bind(effective_window_days)
        .fetch_all(&self.pool)
        .await?;

        let used_bytes = rows.iter().fold(0_i64, |acc, row| {
            acc.saturating_add(row.bytes_served.max(0))
        });
        let real_used_bytes = rows.iter().fold(0_i64, |acc, row| {
            acc.saturating_add(row.real_bytes_served.max(0))
        });

        let quota_bytes = policy.as_ref().and_then(|row| row.traffic_quota_bytes);
        let remaining_bytes = quota_bytes.map(|quota| (quota - used_bytes).max(0));

        let daily = rows.into_iter().map(Into::into).collect::<Vec<_>>();

        Ok(UserTrafficUsageSummary {
            user_id,
            window_days: effective_window_days,
            used_bytes,
            real_used_bytes,
            quota_bytes,
            remaining_bytes,
            daily,
        })
    }

    pub async fn get_user_traffic_usage_media_summary(
        &self,
        user_id: Uuid,
        window_days: Option<i32>,
        limit: Option<i64>,
    ) -> anyhow::Result<UserTrafficUsageMediaSummary> {
        let effective_window_days = normalize_traffic_window_days(window_days.unwrap_or(30)).min(30);
        let safe_limit = normalize_traffic_usage_media_limit(limit);

        let summary = self
            .get_user_traffic_usage_summary(user_id, Some(effective_window_days))
            .await?;

        let rows = sqlx::query_as::<_, UserTrafficUsageMediaRow>(
            r#"
SELECT
    t.media_item_id,
    COALESCE(NULLIF(m.name, ''), t.media_item_id::TEXT) AS item_name,
    COALESCE(NULLIF(m.item_type, ''), 'Unknown') AS item_type,
    COALESCE(SUM(t.bytes_served), 0)::BIGINT AS bytes_served,
    COALESCE(SUM(t.real_bytes_served), 0)::BIGINT AS real_bytes_served,
    COUNT(*)::BIGINT AS usage_days,
    MAX(t.usage_date) AS last_usage_date
FROM user_stream_usage_media_daily t
LEFT JOIN media_items m ON m.id = t.media_item_id
WHERE t.user_id = $1
  AND t.usage_date >= current_date - ($2::INT - 1)
GROUP BY t.media_item_id, m.name, m.item_type
ORDER BY bytes_served DESC, last_usage_date DESC, item_name ASC
LIMIT $3
            "#,
        )
        .bind(user_id)
        .bind(effective_window_days)
        .bind(safe_limit)
        .fetch_all(&self.pool)
        .await?;

        let items = rows
            .into_iter()
            .map(UserTrafficUsageMediaItem::from)
            .collect::<Vec<_>>();
        let classified_bytes = items.iter().fold(0_i64, |acc, item| {
            acc.saturating_add(item.bytes_served.max(0))
        });
        let classified_real_bytes = items.iter().fold(0_i64, |acc, item| {
            acc.saturating_add(item.real_bytes_served.max(0))
        });
        let unclassified_bytes = (summary.used_bytes - classified_bytes).max(0);
        let unclassified_real_bytes = (summary.real_used_bytes - classified_real_bytes).max(0);

        Ok(UserTrafficUsageMediaSummary {
            user_id,
            window_days: effective_window_days,
            used_bytes: summary.used_bytes,
            real_used_bytes: summary.real_used_bytes,
            quota_bytes: summary.quota_bytes,
            remaining_bytes: summary.remaining_bytes,
            unclassified_bytes,
            unclassified_real_bytes,
            items,
        })
    }

    pub async fn reset_user_traffic_usage(&self, user_id: Uuid) -> anyhow::Result<u64> {
        let deleted = sqlx::query("DELETE FROM user_stream_usage_daily WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await?
            .rows_affected();
        Ok(deleted)
    }

    pub async fn list_top_traffic_users(
        &self,
        limit: i64,
        window_days: i32,
    ) -> anyhow::Result<Vec<TopTrafficUser>> {
        let safe_limit = limit.clamp(1, 200);
        let safe_window_days = normalize_traffic_window_days(window_days);

        let rows = sqlx::query_as::<_, TopTrafficUserRow>(
            r#"
SELECT
    u.id AS user_id,
    u.username,
    COALESCE(SUM(usd.bytes_served), 0)::BIGINT AS used_bytes
FROM users u
JOIN user_stream_usage_daily usd ON usd.user_id = u.id
WHERE usd.usage_date >= current_date - ($2::INT - 1)
GROUP BY u.id, u.username
ORDER BY used_bytes DESC, u.username ASC
LIMIT $1
            "#,
        )
        .bind(safe_limit)
        .bind(safe_window_days)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn list_top_played_media(
        &self,
        limit: i64,
        stat_date: Option<NaiveDate>,
        window_days: i32,
    ) -> anyhow::Result<TopPlayedMediaSummary> {
        let safe_limit = limit.clamp(1, 100);
        let safe_window_days = normalize_traffic_window_days(window_days).clamp(1, 90);
        let target_date = stat_date.unwrap_or_else(|| Utc::now().date_naive());

        let rows = sqlx::query_as::<_, TopPlayedMediaRow>(
            r#"
SELECT
    e.media_item_id,
    m.name,
    m.item_type,
    m.runtime_ticks,
    m.bitrate,
    CASE
        WHEN COALESCE(m.metadata->>'production_year', '') ~ '^[0-9]{4}$'
            THEN (m.metadata->>'production_year')::INT
        ELSE NULL
    END AS production_year,
    CASE
        WHEN COALESCE(m.metadata->>'community_rating', '') ~ '^-?[0-9]+(\.[0-9]+)?$'
            THEN (m.metadata->>'community_rating')::DOUBLE PRECISION
        ELSE NULL
    END AS community_rating,
    NULLIF(m.metadata->>'overview', '') AS overview,
    COUNT(*)::BIGINT AS play_count,
    COUNT(DISTINCT e.user_id)::BIGINT AS unique_users
FROM media_play_events_daily e
JOIN media_items m ON m.id = e.media_item_id
WHERE e.usage_date <= $2::DATE
  AND e.usage_date >= $2::DATE - ($3::INT - 1)
GROUP BY
    e.media_item_id,
    m.name,
    m.item_type,
    m.runtime_ticks,
    m.bitrate,
    m.metadata
ORDER BY play_count DESC, unique_users DESC, m.name ASC
LIMIT $1
            "#,
        )
        .bind(safe_limit)
        .bind(target_date)
        .bind(safe_window_days)
        .fetch_all(&self.pool)
        .await?;

        Ok(TopPlayedMediaSummary {
            stat_date: target_date.to_string(),
            window_days: safe_window_days,
            items: rows.into_iter().map(Into::into).collect(),
        })
    }

    pub async fn check_stream_admission(&self, user_id: Uuid) -> anyhow::Result<()> {
        if !self.advanced_traffic_controls_enabled() {
            return Ok(());
        }

        let Some(policy) = self.get_user_stream_policy(user_id).await? else {
            return Ok(());
        };

        if let Some(expires_at) = policy.expires_at {
            if expires_at <= Utc::now() {
                return Err(anyhow::Error::new(InfraError::StreamAccessDenied {
                    reason: StreamAccessDeniedReason::AccountExpired,
                }));
            }
        }

        if let Some(max_concurrent_streams) = policy.max_concurrent_streams.filter(|v| *v >= 0) {
            let active = self
                .count_active_playback_sessions_for_user(
                    user_id,
                    STREAM_POLICY_ACTIVE_HEARTBEAT_GRACE_SECONDS,
                )
                .await?;
            if active >= i64::from(max_concurrent_streams) {
                return Err(anyhow::Error::new(InfraError::StreamAccessDenied {
                    reason: StreamAccessDeniedReason::ConcurrentLimitExceeded,
                }));
            }
        }

        if let Some(traffic_quota_bytes) = policy.traffic_quota_bytes.filter(|v| *v >= 0) {
            let used_bytes = self
                .sum_user_traffic_usage_bytes(user_id, policy.traffic_window_days)
                .await?;
            if used_bytes >= traffic_quota_bytes {
                return Err(anyhow::Error::new(InfraError::StreamAccessDenied {
                    reason: StreamAccessDeniedReason::TrafficQuotaExceeded,
                }));
            }
        }

        Ok(())
    }

    pub async fn record_user_stream_bytes(
        &self,
        user_id: Uuid,
        item_id: Option<Uuid>,
        real_bytes_served: u64,
        traffic_multiplier: f64,
    ) -> anyhow::Result<()> {
        if !self.advanced_traffic_controls_enabled() {
            return Ok(());
        }

        if real_bytes_served == 0 {
            return Ok(());
        }

        let charged_bytes_served =
            scale_stream_traffic_bytes(real_bytes_served, normalize_stream_traffic_multiplier(traffic_multiplier));
        let charged_bytes_served_i64 = i64::try_from(charged_bytes_served).unwrap_or(i64::MAX);
        let real_bytes_served_i64 = i64::try_from(real_bytes_served).unwrap_or(i64::MAX);

        sqlx::query(
            r#"
INSERT INTO user_stream_usage_daily (user_id, usage_date, bytes_served, real_bytes_served, updated_at)
VALUES ($1, current_date, $2, $3, now())
ON CONFLICT (user_id, usage_date) DO UPDATE SET
    bytes_served = user_stream_usage_daily.bytes_served + EXCLUDED.bytes_served,
    real_bytes_served = user_stream_usage_daily.real_bytes_served + EXCLUDED.real_bytes_served,
    updated_at = now()
            "#,
        )
        .bind(user_id)
        .bind(charged_bytes_served_i64)
        .bind(real_bytes_served_i64)
        .execute(&self.pool)
        .await?;

        if let Some(media_item_id) = item_id {
            sqlx::query(
                r#"
INSERT INTO user_stream_usage_media_daily (
    user_id,
    media_item_id,
    usage_date,
    bytes_served,
    real_bytes_served,
    updated_at
)
VALUES ($1, $2, current_date, $3, $4, now())
ON CONFLICT (user_id, media_item_id, usage_date) DO UPDATE SET
    bytes_served = user_stream_usage_media_daily.bytes_served + EXCLUDED.bytes_served,
    real_bytes_served = user_stream_usage_media_daily.real_bytes_served + EXCLUDED.real_bytes_served,
    updated_at = now()
                "#,
            )
            .bind(user_id)
            .bind(media_item_id)
            .bind(charged_bytes_served_i64)
            .bind(real_bytes_served_i64)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    fn default_user_stream_policy(&self, user_id: Uuid) -> UserStreamPolicy {
        UserStreamPolicy {
            user_id,
            expires_at: None,
            max_concurrent_streams: normalize_default_optional_i32(
                self.config_snapshot().security.default_user_max_concurrent_streams,
            ),
            traffic_quota_bytes: normalize_default_optional_i64(
                self.config_snapshot().security.default_user_traffic_quota_bytes,
            ),
            traffic_window_days: normalize_traffic_window_days(
                self.config_snapshot().security.default_user_traffic_window_days,
            ),
            updated_at: Utc::now(),
        }
    }

    async fn ensure_default_user_stream_policy(&self, user_id: Uuid) -> anyhow::Result<()> {
        let max_concurrent_streams = normalize_default_optional_i32(
            self.config_snapshot().security.default_user_max_concurrent_streams,
        );
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
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn ensure_default_playlist(&self, user_id: Uuid) -> anyhow::Result<()> {
        let id = Uuid::now_v7();
        let now = Utc::now();
        sqlx::query(
            r#"
INSERT INTO playlists (id, owner_user_id, name, description, is_public, is_default, created_at, updated_at)
VALUES ($1, $2, $3, $4, FALSE, TRUE, $5, $6)
ON CONFLICT (owner_user_id, name) DO NOTHING
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(DEFAULT_FAVORITES_PLAYLIST_NAME)
        .bind(DEFAULT_FAVORITES_PLAYLIST_DESCRIPTION)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn count_active_playback_sessions_for_user(
        &self,
        user_id: Uuid,
        heartbeat_grace_seconds: i64,
    ) -> anyhow::Result<i64> {
        let grace = heartbeat_grace_seconds.max(1);
        let count = sqlx::query_scalar::<_, i64>(
            r#"
SELECT COUNT(*)::BIGINT
FROM playback_sessions
WHERE user_id = $1
  AND is_active = true
  AND last_heartbeat_at >= now() - ($2::BIGINT * interval '1 second')
            "#,
        )
        .bind(user_id)
        .bind(grace)
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }

    async fn sum_user_traffic_usage_bytes(
        &self,
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
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(0);

        Ok(used_bytes.max(0))
    }

}

fn normalize_traffic_usage_media_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(200).clamp(1, 500)
}

fn normalize_stream_traffic_multiplier(raw: f64) -> f64 {
    if !raw.is_finite() {
        return 1.0;
    }
    raw.clamp(0.01, 100.0)
}

fn scale_stream_traffic_bytes(real_bytes_served: u64, traffic_multiplier: f64) -> u64 {
    let scaled = (real_bytes_served as f64) * traffic_multiplier;
    if !scaled.is_finite() || scaled <= 0.0 {
        return 0;
    }
    scaled.round().clamp(0.0, u64::MAX as f64) as u64
}

#[cfg(test)]
mod app_users_policy_traffic_tests {
    use super::*;

    #[test]
    fn normalize_traffic_usage_media_limit_clamps_bounds() {
        assert_eq!(normalize_traffic_usage_media_limit(None), 200);
        assert_eq!(normalize_traffic_usage_media_limit(Some(0)), 1);
        assert_eq!(normalize_traffic_usage_media_limit(Some(42)), 42);
        assert_eq!(normalize_traffic_usage_media_limit(Some(9999)), 500);
    }

    #[test]
    fn normalize_stream_traffic_multiplier_clamps_bounds() {
        assert_eq!(normalize_stream_traffic_multiplier(0.0), 0.01);
        assert_eq!(normalize_stream_traffic_multiplier(1.5), 1.5);
        assert_eq!(normalize_stream_traffic_multiplier(128.0), 100.0);
        assert_eq!(normalize_stream_traffic_multiplier(f64::INFINITY), 1.0);
    }

    #[test]
    fn scale_stream_traffic_bytes_applies_multiplier() {
        assert_eq!(scale_stream_traffic_bytes(100, 1.0), 100);
        assert_eq!(scale_stream_traffic_bytes(100, 1.5), 150);
        assert_eq!(scale_stream_traffic_bytes(100, 0.5), 50);
    }
}
