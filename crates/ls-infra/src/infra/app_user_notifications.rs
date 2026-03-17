const UPSERT_USER_ITEM_DATA_SQL: &str = r#"
INSERT INTO watch_states (
    user_id,
    media_item_id,
    played,
    playback_position_ticks,
    is_favorite,
    play_count,
    last_played_at
) VALUES (
    $1,
    $2,
    COALESCE($3, FALSE),
    COALESCE($4, 0),
    COALESCE($5, FALSE),
    CASE WHEN COALESCE($3, FALSE) THEN 1 ELSE 0 END,
    CASE WHEN COALESCE($3, FALSE) THEN now() ELSE NULL END
)
ON CONFLICT(user_id, media_item_id) DO UPDATE SET
    played = COALESCE($3, watch_states.played),
    playback_position_ticks = CASE
        WHEN $3 IS FALSE THEN 0
        ELSE COALESCE($4, watch_states.playback_position_ticks)
    END,
    is_favorite = COALESCE($5, watch_states.is_favorite),
    play_count = CASE
        WHEN $3 IS TRUE AND watch_states.played IS DISTINCT FROM TRUE
            THEN watch_states.play_count + 1
        WHEN $3 IS FALSE THEN 0
        ELSE watch_states.play_count
    END,
    last_played_at = CASE
        WHEN $3 IS TRUE THEN now()
        WHEN $3 IS FALSE THEN NULL
        ELSE watch_states.last_played_at
    END
"#;

const MARK_PLAYED_WATCH_STATE_SQL: &str = r#"
INSERT INTO watch_states (user_id, media_item_id, played, play_count, last_played_at)
VALUES ($1, $2, true, 1, now())
ON CONFLICT(user_id, media_item_id) DO UPDATE SET
    played = true,
    play_count = watch_states.play_count + 1,
    last_played_at = now()
"#;

const MARK_UNPLAYED_WATCH_STATE_SQL: &str = r#"
INSERT INTO watch_states (
    user_id,
    media_item_id,
    played,
    playback_position_ticks,
    play_count,
    last_played_at
)
VALUES ($1, $2, false, 0, 0, NULL)
ON CONFLICT(user_id, media_item_id) DO UPDATE SET
    played = false,
    playback_position_ticks = 0,
    play_count = 0,
    last_played_at = NULL
"#;

impl AppInfra {
    pub fn arc(self) -> Arc<Self> {
        Arc::new(self)
    }

    async fn user_data(&self, user_id: Uuid, item_id: Uuid) -> anyhow::Result<Option<UserDataDto>> {
        let row = sqlx::query_as::<_, WatchStateRow>(
            r#"
SELECT playback_position_ticks, played, is_favorite
FROM watch_states
WHERE user_id = $1 AND media_item_id = $2
LIMIT 1
            "#,
        )
        .bind(user_id)
        .bind(item_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|v| UserDataDto {
            played: v.played,
            playback_position_ticks: v.playback_position_ticks,
            is_favorite: v.is_favorite,
        }))
    }

    async fn user_data_map(
        &self,
        user_id: Uuid,
        item_ids: &[Uuid],
    ) -> anyhow::Result<std::collections::HashMap<Uuid, UserDataDto>> {
        if item_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        let rows = sqlx::query_as::<_, (Uuid, i64, bool, Option<bool>)>(
            r#"
SELECT media_item_id, playback_position_ticks, played, is_favorite
FROM watch_states
WHERE user_id = $1 AND media_item_id = ANY($2)
            "#,
        )
        .bind(user_id)
        .bind(item_ids)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(media_item_id, playback_position_ticks, played, is_favorite)| {
                (
                    media_item_id,
                    UserDataDto {
                        played,
                        playback_position_ticks,
                        is_favorite,
                    },
                )
            })
            .collect())
    }

    /// Get full user item data for playstate endpoints
    pub async fn get_user_item_data(
        &self,
        user_id: Uuid,
        item_id: Uuid,
    ) -> anyhow::Result<UserItemDataDto> {
        let row = sqlx::query_as::<_, UserItemDataRow>(
            r#"
SELECT playback_position_ticks, play_count, is_favorite, played, last_played_at
FROM watch_states
WHERE user_id = $1 AND media_item_id = $2
LIMIT 1
            "#,
        )
        .bind(user_id)
        .bind(item_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row
            .map(|v| UserItemDataDto {
                playback_position_ticks: v.playback_position_ticks,
                play_count: v.play_count,
                is_favorite: v.is_favorite.unwrap_or(false),
                played: v.played,
                last_played_date: v.last_played_at.map(|dt| dt.to_rfc3339()),
                item_id: item_id.to_string(),
            })
            .unwrap_or_else(|| UserItemDataDto {
                playback_position_ticks: 0,
                play_count: 0,
                is_favorite: false,
                played: false,
                last_played_date: None,
                item_id: item_id.to_string(),
            }))
    }

    /// Upsert user item data fields for compatibility endpoints.
    pub async fn update_user_item_data(
        &self,
        user_id: Uuid,
        item_id: Uuid,
        update: UserItemDataUpdate,
    ) -> anyhow::Result<UserItemDataDto> {
        if update.played.is_none()
            && update.playback_position_ticks.is_none()
            && update.is_favorite.is_none()
        {
            return self.get_user_item_data(user_id, item_id).await;
        }

        let played = update.played;
        let is_favorite = update.is_favorite;
        let playback_position_ticks = update.playback_position_ticks.map(|value| value.max(0));

        sqlx::query(UPSERT_USER_ITEM_DATA_SQL)
        .bind(user_id)
        .bind(item_id)
        .bind(played)
        .bind(playback_position_ticks)
        .bind(is_favorite)
        .execute(&self.pool)
        .await?;

        self.get_user_item_data(user_id, item_id).await
    }

    /// Mark an item as played
    pub async fn mark_played(
        &self,
        user_id: Uuid,
        item_id: Uuid,
    ) -> anyhow::Result<UserItemDataDto> {
        sqlx::query(MARK_PLAYED_WATCH_STATE_SQL)
        .bind(user_id)
        .bind(item_id)
        .execute(&self.pool)
        .await?;

        self.get_user_item_data(user_id, item_id).await
    }

    /// Mark an item as unplayed
    pub async fn mark_unplayed(
        &self,
        user_id: Uuid,
        item_id: Uuid,
    ) -> anyhow::Result<UserItemDataDto> {
        sqlx::query(MARK_UNPLAYED_WATCH_STATE_SQL)
        .bind(user_id)
        .bind(item_id)
        .execute(&self.pool)
        .await?;

        self.get_user_item_data(user_id, item_id).await
    }

    /// Set favorite status for an item
    pub async fn set_favorite(
        &self,
        user_id: Uuid,
        item_id: Uuid,
        is_favorite: bool,
    ) -> anyhow::Result<UserItemDataDto> {
        sqlx::query(
            r#"
INSERT INTO watch_states (user_id, media_item_id, is_favorite)
VALUES ($1, $2, $3)
ON CONFLICT(user_id, media_item_id) DO UPDATE SET
    is_favorite = $3
            "#,
        )
        .bind(user_id)
        .bind(item_id)
        .bind(is_favorite)
        .execute(&self.pool)
        .await?;

        if is_favorite {
            self.ensure_default_playlist(user_id).await?;
            sqlx::query(
                r#"
WITH favorite_playlist AS (
    SELECT id
    FROM playlists
    WHERE owner_user_id = $1
      AND name = $2
    LIMIT 1
)
INSERT INTO playlist_items (playlist_id, media_item_id, added_at)
SELECT id, $3, now()
FROM favorite_playlist
ON CONFLICT (playlist_id, media_item_id) DO NOTHING
                "#,
            )
            .bind(user_id)
            .bind(DEFAULT_FAVORITES_PLAYLIST_NAME)
            .bind(item_id)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query(
                r#"
DELETE FROM playlist_items
WHERE media_item_id = $3
  AND playlist_id IN (
      SELECT id
      FROM playlists
      WHERE owner_user_id = $1
        AND name = $2
  )
                "#,
            )
            .bind(user_id)
            .bind(DEFAULT_FAVORITES_PLAYLIST_NAME)
            .bind(item_id)
            .execute(&self.pool)
            .await?;
        }

        self.get_user_item_data(user_id, item_id).await
    }

    pub fn subscribe_notifications(&self) -> broadcast::Receiver<Notification> {
        self.notification_tx.subscribe()
    }

    pub async fn create_notification(
        &self,
        user_id: Uuid,
        title: &str,
        message: &str,
        notification_type: &str,
        meta: Value,
    ) -> anyhow::Result<Notification> {
        let id = Uuid::now_v7();
        let now = Utc::now();

        sqlx::query(
            r#"
INSERT INTO notifications (id, user_id, title, message, notification_type, meta, created_at)
VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(title)
        .bind(message)
        .bind(notification_type)
        .bind(&meta)
        .bind(now)
        .execute(&self.pool)
        .await?;

        let notification = Notification {
            id,
            user_id,
            title: title.to_string(),
            message: message.to_string(),
            notification_type: notification_type.to_string(),
            is_read: false,
            meta,
            created_at: now,
            read_at: None,
        };

        let _ = self.notification_tx.send(notification.clone());

        Ok(notification)
    }

    pub async fn list_notifications(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> anyhow::Result<(Vec<Notification>, i64)> {
        let limit = limit.clamp(1, 100);
        let offset = offset.max(0);

        let total: i64 =
            sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM notifications WHERE user_id = $1")
                .bind(user_id)
                .fetch_one(&self.pool)
                .await
                .unwrap_or(0);

        let rows = sqlx::query_as::<_, NotificationRow>(
            r#"
SELECT id, user_id, title, message, notification_type, is_read, meta, created_at, read_at
FROM notifications
WHERE user_id = $1
ORDER BY created_at DESC
LIMIT $2 OFFSET $3
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let notifications = rows.into_iter().map(|r| r.into()).collect();
        Ok((notifications, total))
    }

    pub async fn mark_notification_read(&self, user_id: Uuid, id: Uuid) -> anyhow::Result<bool> {
        let result = sqlx::query(
            r#"
UPDATE notifications
SET is_read = true, read_at = now()
WHERE id = $1 AND user_id = $2 AND is_read = false
            "#,
        )
        .bind(id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn mark_all_notifications_read(&self, user_id: Uuid) -> anyhow::Result<u64> {
        let result = sqlx::query(
            r#"
UPDATE notifications
SET is_read = true, read_at = now()
WHERE user_id = $1 AND is_read = false
            "#,
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod app_user_notifications_tests {
    use super::{MARK_UNPLAYED_WATCH_STATE_SQL, UPSERT_USER_ITEM_DATA_SQL};

    #[test]
    fn update_user_item_data_sql_resets_watch_state_when_played_is_false() {
        assert!(UPSERT_USER_ITEM_DATA_SQL.contains("WHEN $3 IS FALSE THEN 0"));
        assert!(UPSERT_USER_ITEM_DATA_SQL.contains("WHEN $3 IS FALSE THEN NULL"));
    }

    #[test]
    fn mark_unplayed_sql_clears_play_count_and_last_played() {
        assert!(MARK_UNPLAYED_WATCH_STATE_SQL.contains("play_count = 0"));
        assert!(MARK_UNPLAYED_WATCH_STATE_SQL.contains("last_played_at = NULL"));
    }
}
