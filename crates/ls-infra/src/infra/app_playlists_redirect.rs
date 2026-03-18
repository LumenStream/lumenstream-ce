const PLAYLIST_SELECT_BY_ID_SQL: &str = r#"
SELECT
    p.id,
    p.owner_user_id,
    p.name,
    p.description,
    p.is_public,
    p.is_default,
    p.playlist_type,
    p.created_at,
    p.updated_at,
    COUNT(pi.media_item_id)::BIGINT AS item_count
FROM playlists p
LEFT JOIN playlist_items pi ON pi.playlist_id = p.id
WHERE p.id = $1
GROUP BY p.id, p.owner_user_id, p.name, p.description, p.is_public, p.is_default, p.playlist_type, p.created_at, p.updated_at
LIMIT 1
"#;

const PLAYLIST_SELECT_USER_LIST_SQL: &str = r#"
SELECT
    p.id,
    p.owner_user_id,
    p.name,
    p.description,
    p.is_public,
    p.is_default,
    p.playlist_type,
    p.created_at,
    p.updated_at,
    COUNT(pi.media_item_id)::BIGINT AS item_count
FROM playlists p
LEFT JOIN playlist_items pi ON pi.playlist_id = p.id
WHERE p.owner_user_id = $1
GROUP BY p.id, p.owner_user_id, p.name, p.description, p.is_public, p.is_default, p.playlist_type, p.created_at, p.updated_at
ORDER BY p.is_default DESC, p.updated_at DESC, p.created_at DESC
"#;

const PLAYLIST_SELECT_PUBLIC_LIST_SQL: &str = r#"
SELECT
    p.id,
    p.owner_user_id,
    p.name,
    p.description,
    p.is_public,
    p.is_default,
    p.playlist_type,
    p.created_at,
    p.updated_at,
    COUNT(pi.media_item_id)::BIGINT AS item_count
FROM playlists p
LEFT JOIN playlist_items pi ON pi.playlist_id = p.id
WHERE p.owner_user_id = $1
  AND p.is_public = true
GROUP BY p.id, p.owner_user_id, p.name, p.description, p.is_public, p.is_default, p.playlist_type, p.created_at, p.updated_at
ORDER BY p.is_default DESC, p.updated_at DESC, p.created_at DESC
"#;

impl AppInfra {
    async fn get_playlist_by_id(&self, playlist_id: Uuid) -> anyhow::Result<Option<Playlist>> {
        let row = sqlx::query_as::<_, PlaylistRow>(PLAYLIST_SELECT_BY_ID_SQL)
        .bind(playlist_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    async fn ensure_playlist_owner(
        &self,
        owner_user_id: Uuid,
        playlist_id: Uuid,
    ) -> anyhow::Result<Playlist> {
        let Some(playlist) = self.get_playlist_by_id(playlist_id).await? else {
            return Err(anyhow::Error::new(InfraError::PlaylistNotFound));
        };
        if playlist.owner_user_id != owner_user_id {
            return Err(anyhow::Error::new(InfraError::PlaylistAccessDenied));
        }
        Ok(playlist)
    }

    async fn ensure_playlist_visible(
        &self,
        viewer_user_id: Uuid,
        playlist_id: Uuid,
    ) -> anyhow::Result<Playlist> {
        let Some(playlist) = self.get_playlist_by_id(playlist_id).await? else {
            return Err(anyhow::Error::new(InfraError::PlaylistNotFound));
        };
        if !playlist_is_visible_to(viewer_user_id, playlist.owner_user_id, playlist.is_public) {
            return Err(anyhow::Error::new(InfraError::PlaylistAccessDenied));
        }
        Ok(playlist)
    }

    pub async fn create_playlist(
        &self,
        owner_user_id: Uuid,
        name: &str,
        description: Option<&str>,
        is_public: bool,
    ) -> anyhow::Result<Playlist> {
        self.create_playlist_typed(owner_user_id, name, description, is_public, "playlist")
            .await
    }

    pub async fn create_collection(
        &self,
        owner_user_id: Uuid,
        name: &str,
    ) -> anyhow::Result<Playlist> {
        self.create_playlist_typed(owner_user_id, name, None, true, "collection")
            .await
    }

    async fn create_playlist_typed(
        &self,
        owner_user_id: Uuid,
        name: &str,
        description: Option<&str>,
        is_public: bool,
        playlist_type: &str,
    ) -> anyhow::Result<Playlist> {
        let name = name.trim();
        if name.is_empty() {
            return Err(anyhow::Error::new(InfraError::PlaylistInvalidInput)
                .context("playlist name is required"));
        }
        let description = description.unwrap_or_default().trim();

        let id = Uuid::now_v7();
        let now = Utc::now();
        let insert = sqlx::query(
            r#"
INSERT INTO playlists (id, owner_user_id, name, description, is_public, playlist_type, created_at, updated_at)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(id)
        .bind(owner_user_id)
        .bind(name)
        .bind(description)
        .bind(is_public)
        .bind(playlist_type)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await;

        if let Err(err) = insert {
            if is_unique_violation(&err) {
                return Err(anyhow::Error::new(InfraError::PlaylistConflict)
                    .context("playlist name already exists"));
            }
            return Err(err.into());
        }

        self.get_playlist_by_id(id)
            .await?
            .context("playlist not found after create")
    }

    pub async fn list_user_playlists(&self, owner_user_id: Uuid) -> anyhow::Result<Vec<Playlist>> {
        let rows = sqlx::query_as::<_, PlaylistRow>(PLAYLIST_SELECT_USER_LIST_SQL)
        .bind(owner_user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn list_public_playlists(
        &self,
        owner_user_id: Uuid,
    ) -> anyhow::Result<Vec<Playlist>> {
        let rows = sqlx::query_as::<_, PlaylistRow>(PLAYLIST_SELECT_PUBLIC_LIST_SQL)
        .bind(owner_user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn get_playlist_visible_to(
        &self,
        viewer_user_id: Uuid,
        playlist_id: Uuid,
    ) -> anyhow::Result<Playlist> {
        self.ensure_playlist_visible(viewer_user_id, playlist_id)
            .await
    }

    pub async fn update_playlist(
        &self,
        owner_user_id: Uuid,
        playlist_id: Uuid,
        patch: PlaylistUpdate,
    ) -> anyhow::Result<Playlist> {
        let current = self
            .ensure_playlist_owner(owner_user_id, playlist_id)
            .await?;

        // Default playlist cannot be renamed
        let next_name = if current.is_default {
            current.name.as_str()
        } else {
            patch
                .name
                .as_deref()
                .map(str::trim)
                .unwrap_or(current.name.as_str())
        };
        if next_name.is_empty() {
            return Err(anyhow::Error::new(InfraError::PlaylistInvalidInput)
                .context("playlist name is required"));
        }

        let next_description = patch
            .description
            .as_deref()
            .map(str::trim)
            .unwrap_or(current.description.as_str());
        let next_public = patch.is_public.unwrap_or(current.is_public);

        let update = sqlx::query(
            r#"
UPDATE playlists
SET
    name = $2,
    description = $3,
    is_public = $4,
    updated_at = now()
WHERE id = $1
            "#,
        )
        .bind(playlist_id)
        .bind(next_name)
        .bind(next_description)
        .bind(next_public)
        .execute(&self.pool)
        .await;

        if let Err(err) = update {
            if is_unique_violation(&err) {
                return Err(anyhow::Error::new(InfraError::PlaylistConflict)
                    .context("playlist name already exists"));
            }
            return Err(err.into());
        }

        self.get_playlist_by_id(playlist_id)
            .await?
            .context("playlist not found after update")
    }

    pub async fn delete_playlist(
        &self,
        owner_user_id: Uuid,
        playlist_id: Uuid,
    ) -> anyhow::Result<bool> {
        let playlist = self
            .ensure_playlist_owner(owner_user_id, playlist_id)
            .await?;
        if playlist.is_default {
            return Err(anyhow::Error::new(InfraError::PlaylistCannotDeleteDefault));
        }
        let result = sqlx::query("DELETE FROM playlists WHERE id = $1")
            .bind(playlist_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn add_item_to_playlist(
        &self,
        owner_user_id: Uuid,
        playlist_id: Uuid,
        media_item_id: Uuid,
    ) -> anyhow::Result<PlaylistItem> {
        let _ = self
            .ensure_playlist_owner(owner_user_id, playlist_id)
            .await?;
        let media_exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM media_items WHERE id = $1)")
                .bind(media_item_id)
                .fetch_one(&self.pool)
                .await?;
        if !media_exists {
            return Err(anyhow::Error::new(InfraError::MediaItemNotFound));
        }

        let row = match sqlx::query_as::<_, PlaylistItemRow>(
            r#"
INSERT INTO playlist_items (playlist_id, media_item_id, added_at)
VALUES ($1, $2, now())
RETURNING playlist_id, media_item_id, added_at
            "#,
        )
        .bind(playlist_id)
        .bind(media_item_id)
        .fetch_one(&self.pool)
        .await
        {
            Ok(v) => v,
            Err(err) if is_unique_violation(&err) => {
                return Err(anyhow::Error::new(InfraError::PlaylistConflict)
                    .context("item already exists in playlist"));
            }
            Err(err) => return Err(err.into()),
        };

        Ok(row.into())
    }

    pub async fn remove_item_from_playlist(
        &self,
        owner_user_id: Uuid,
        playlist_id: Uuid,
        media_item_id: Uuid,
    ) -> anyhow::Result<bool> {
        let _ = self
            .ensure_playlist_owner(owner_user_id, playlist_id)
            .await?;
        let result = sqlx::query(
            r#"
DELETE FROM playlist_items
WHERE playlist_id = $1 AND media_item_id = $2
            "#,
        )
        .bind(playlist_id)
        .bind(media_item_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn add_items_to_playlist_batch(
        &self,
        owner_user_id: Uuid,
        playlist_id: Uuid,
        media_item_ids: &[Uuid],
    ) -> anyhow::Result<u64> {
        let _ = self
            .ensure_playlist_owner(owner_user_id, playlist_id)
            .await?;
        let mut added = 0u64;
        for &mid in media_item_ids {
            let result = sqlx::query(
                r#"
INSERT INTO playlist_items (playlist_id, media_item_id, added_at)
VALUES ($1, $2, now())
ON CONFLICT (playlist_id, media_item_id) DO NOTHING
                "#,
            )
            .bind(playlist_id)
            .bind(mid)
            .execute(&self.pool)
            .await?;
            added += result.rows_affected();
        }
        Ok(added)
    }

    pub async fn remove_items_from_playlist_batch(
        &self,
        owner_user_id: Uuid,
        playlist_id: Uuid,
        media_item_ids: &[Uuid],
    ) -> anyhow::Result<u64> {
        let _ = self
            .ensure_playlist_owner(owner_user_id, playlist_id)
            .await?;
        let mut removed = 0u64;
        for &mid in media_item_ids {
            let result = sqlx::query(
                "DELETE FROM playlist_items WHERE playlist_id = $1 AND media_item_id = $2",
            )
            .bind(playlist_id)
            .bind(mid)
            .execute(&self.pool)
            .await?;
            removed += result.rows_affected();
        }
        Ok(removed)
    }

    pub async fn list_playlist_items_visible_to(
        &self,
        viewer_user_id: Uuid,
        playlist_id: Uuid,
    ) -> anyhow::Result<Vec<BaseItemDto>> {
        let _ = self
            .ensure_playlist_visible(viewer_user_id, playlist_id)
            .await?;
        let rows = sqlx::query_as::<_, MediaItemRow>(
            r#"
SELECT
    m.id,
    m.item_type,
    m.name,
    m.path,
    m.runtime_ticks,
    m.bitrate,
    m.series_id,
    m.season_number,
    m.episode_number,
    m.library_id,
    m.metadata,
    m.created_at
FROM playlist_items pi
JOIN media_items m ON m.id = pi.media_item_id
WHERE pi.playlist_id = $1
ORDER BY pi.added_at DESC
            "#,
        )
        .bind(playlist_id)
        .fetch_all(&self.pool)
        .await?;

        let row_ids = rows.iter().map(|row| row.id).collect::<Vec<_>>();
        let mut user_data_map = self.user_data_map(viewer_user_id, &row_ids).await?;
        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let user_data = user_data_map.remove(&row.id);
            items.push(item_row_to_dto(row, user_data));
        }
        self.attach_grouped_media_sources(&mut items).await?;
        Ok(items)
    }

    pub async fn resolve_stream_redirect_target(
        &self,
        item_id: Uuid,
        media_source_id: Option<&str>,
        user_id: Uuid,
        access_token: &str,
    ) -> anyhow::Result<Option<String>> {
        let selected_source_item_id = match self
            .playback_info(item_id, Some(user_id), media_source_id, None)
            .await?
        {
            Some(info) => info
                .media_sources
                .first()
                .and_then(|source| Uuid::parse_str(&source.id).ok())
                .unwrap_or(item_id),
            None => return Ok(None),
        };

        let row = sqlx::query_as::<_, StreamTargetRow>(
            r#"
SELECT stream_url, metadata
FROM media_items
WHERE id = $1
LIMIT 1
            "#,
        )
        .bind(selected_source_item_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let Some(raw_url) = row.stream_url else {
            return Ok(None);
        };

        let (route, path) = if let Some(file_id) = raw_url.strip_prefix("gdrive://") {
            (
                normalize_lumenbackend_route(&self.config_snapshot().storage.lumenbackend_route),
                file_id.trim().to_string(),
            )
        } else if let Some(path) = decode_local_stream_path(raw_url.as_str()) {
            (
                normalize_local_stream_route(&self.config_snapshot().storage.local_stream_route),
                path,
            )
        } else if let Some(raw) = raw_url.strip_prefix("lumenbackend://") {
            parse_lumenbackend_reference(raw, &self.config_snapshot().storage.lumenbackend_route)
        } else if raw_url.starts_with("http://") || raw_url.starts_with("https://") {
            if let Some((route, path)) = parse_lumenbackend_http_stream_url(raw_url.as_str()) {
                (route, path)
            } else {
                return Ok(Some(raw_url));
            }
        } else if std::path::Path::new(raw_url.as_str()).is_absolute() {
            (
                normalize_local_stream_route(&self.config_snapshot().storage.local_stream_route),
                raw_url.clone(),
            )
        } else {
            return Ok(None);
        };

        if route.is_empty() || path.trim().is_empty() {
            return Ok(None);
        }

        let cfg = self.config_snapshot();
        // `cdn` is a convenience redirect endpoint in lumenbackend; the actual stream handler is
        // configured via `storage.lumenbackend_route` (default: `v1/streams/gdrive`). For token-based
        // auth, we must generate a token that matches the final stream route.
        let route = normalize_lumenbackend_stream_route(route.as_str(), &cfg.storage.lumenbackend_route);
        let domain_base_url = self
            .resolve_playback_domain_for_user(user_id)
            .await?
            .map(|domain| domain.base_url);
        let Some(base_url) = select_lumenbackend_stream_base_url(
            domain_base_url.as_deref(),
            &cfg.storage.lumenbackend_nodes,
        ) else {
            return Ok(None);
        };

        let mut target = format!(
            "{}/{route}?path={}&api_key={}&user_id={}&item_id={}",
            base_url.trim_end_matches('/'),
            urlencoding::encode(path.trim()),
            urlencoding::encode(access_token.trim()),
            user_id,
            selected_source_item_id
        );
        if let Some(token) = build_lumenbackend_stream_token(
            cfg.storage.lumenbackend_stream_signing_key.as_str(),
            route.as_str(),
            path.as_str(),
            cfg.storage.lumenbackend_stream_token_ttl_seconds,
            Utc::now(),
        ) {
            target.push_str("&st=");
            target.push_str(urlencoding::encode(token.as_str()).as_ref());
        }

        Ok(Some(target))
    }

}

fn select_lumenbackend_stream_base_url(
    playback_domain_base_url: Option<&str>,
    nodes: &[String],
) -> Option<String> {
    if let Some(raw) = playback_domain_base_url {
        if let Some(normalized) = normalize_lumenbackend_base_url(raw) {
            return Some(normalized);
        }
    }

    nodes
        .iter()
        .filter_map(|raw| normalize_lumenbackend_node(raw))
        .next()
}

fn normalize_lumenbackend_stream_route(raw_route: &str, default_route: &str) -> String {
    let route = normalize_lumenbackend_route(raw_route);
    if route == "cdn" || route == "v1/streams/cdn" {
        normalize_lumenbackend_route(default_route)
    } else {
        route
    }
}

#[cfg(test)]
mod playlist_query_tests {
    use super::{
        PLAYLIST_SELECT_BY_ID_SQL, PLAYLIST_SELECT_PUBLIC_LIST_SQL, PLAYLIST_SELECT_USER_LIST_SQL,
    };

    #[test]
    fn playlist_row_queries_include_playlist_type_in_select_and_group_by() {
        for query in [
            PLAYLIST_SELECT_BY_ID_SQL,
            PLAYLIST_SELECT_USER_LIST_SQL,
            PLAYLIST_SELECT_PUBLIC_LIST_SQL,
        ] {
            assert!(
                query.matches("p.playlist_type").count() >= 2,
                "playlist query must include playlist_type in SELECT and GROUP BY"
            );
        }
    }
}
