impl AppInfra {
    pub async fn list_auth_sessions(
        &self,
        limit: i64,
        active_only: bool,
    ) -> anyhow::Result<Vec<AuthSession>> {
        let rows = if active_only {
            sqlx::query_as::<_, AuthSessionRow>(
                r#"
SELECT
    s.id,
    s.user_id,
    u.username AS user_name,
    s.client,
    s.device_name,
    s.device_id,
    s.remote_addr,
    s.is_active,
    s.created_at,
    s.last_seen_at
FROM user_sessions s
JOIN users u ON u.id = s.user_id
WHERE s.is_active = true
ORDER BY s.last_seen_at DESC
LIMIT $1
                "#,
            )
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, AuthSessionRow>(
                r#"
SELECT
    s.id,
    s.user_id,
    u.username AS user_name,
    s.client,
    s.device_name,
    s.device_id,
    s.remote_addr,
    s.is_active,
    s.created_at,
    s.last_seen_at
FROM user_sessions s
JOIN users u ON u.id = s.user_id
ORDER BY s.last_seen_at DESC
LIMIT $1
                "#,
            )
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        };

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn list_admin_api_keys(&self, limit: i64) -> anyhow::Result<Vec<AdminApiKey>> {
        let rows = sqlx::query_as::<_, AdminApiKeyRow>(
            r#"
SELECT id, name, created_at, last_used_at
FROM admin_api_keys
ORDER BY created_at DESC
LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn create_admin_api_key(&self, name: &str) -> anyhow::Result<CreatedAdminApiKey> {
        let key_id = Uuid::now_v7();
        let api_key = auth::new_admin_api_key(&self.config_snapshot().auth.admin_api_key_prefix);
        let key_hash = auth::hash_api_key(&api_key);

        let row = sqlx::query_as::<_, AdminApiKeyRow>(
            r#"
INSERT INTO admin_api_keys (id, name, key_hash)
VALUES ($1, $2, $3)
RETURNING id, name, created_at, last_used_at
            "#,
        )
        .bind(key_id)
        .bind(name)
        .bind(key_hash)
        .fetch_one(&self.pool)
        .await?;

        Ok(CreatedAdminApiKey {
            id: row.id,
            name: row.name,
            api_key,
            created_at: row.created_at,
        })
    }

    pub async fn delete_admin_api_key(&self, key_id: Uuid) -> anyhow::Result<bool> {
        let affected = sqlx::query("DELETE FROM admin_api_keys WHERE id = $1")
            .bind(key_id)
            .execute(&self.pool)
            .await?
            .rows_affected();
        Ok(affected > 0)
    }

    pub async fn list_playback_sessions(
        &self,
        limit: i64,
        active_only: bool,
    ) -> anyhow::Result<Vec<AdminPlaybackSession>> {
        let rows = if active_only {
            sqlx::query_as::<_, PlaybackSessionRow>(
                r#"
SELECT
    p.id,
    p.play_session_id,
    p.user_id,
    u.username AS user_name,
    p.media_item_id,
    m.name AS media_item_name,
    p.device_name,
    p.client_name,
    p.play_method,
    p.position_ticks,
    p.is_active,
    p.last_heartbeat_at,
    p.updated_at
FROM playback_sessions p
JOIN users u ON u.id = p.user_id
LEFT JOIN media_items m ON m.id = p.media_item_id
WHERE p.is_active = true
ORDER BY p.updated_at DESC
LIMIT $1
                "#,
            )
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, PlaybackSessionRow>(
                r#"
SELECT
    p.id,
    p.play_session_id,
    p.user_id,
    u.username AS user_name,
    p.media_item_id,
    m.name AS media_item_name,
    p.device_name,
    p.client_name,
    p.play_method,
    p.position_ticks,
    p.is_active,
    p.last_heartbeat_at,
    p.updated_at
FROM playback_sessions p
JOIN users u ON u.id = p.user_id
LEFT JOIN media_items m ON m.id = p.media_item_id
ORDER BY p.updated_at DESC
LIMIT $1
                "#,
            )
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        };

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn log_audit_event(
        &self,
        actor_user_id: Option<Uuid>,
        action: &str,
        target_type: &str,
        target_id: Option<&str>,
        detail: Value,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
INSERT INTO audit_logs (id, actor_user_id, action, target_type, target_id, detail)
VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(actor_user_id)
        .bind(action)
        .bind(target_type)
        .bind(target_id)
        .bind(detail)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_audit_logs(&self, limit: i64) -> anyhow::Result<Vec<AuditLogEntry>> {
        let rows = sqlx::query_as::<_, AuditLogRow>(
            r#"
SELECT
    a.id,
    a.actor_user_id,
    u.username AS actor_username,
    a.action,
    a.target_type,
    a.target_id,
    a.detail,
    a.created_at
FROM audit_logs a
LEFT JOIN users u ON u.id = a.actor_user_id
ORDER BY a.created_at DESC
LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

}
