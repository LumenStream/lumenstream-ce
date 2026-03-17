#[derive(Debug, Clone, Copy)]
enum ParentQueryScope {
    Library { parent_id: Uuid, recursive: bool },
    Series { series_id: Uuid, recursive: bool },
    Season {
        series_id: Uuid,
        season_number: i32,
    },
    Fallback { parent_id: Uuid, recursive: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ItemsSortField {
    SortName,
    DateCreated,
    PremiereDate,
    ProductionYear,
    CommunityRating,
    Random,
    SeriesSortName,
    ParentIndexNumber,
    IndexNumber,
    Name,
}

impl AppInfra {
    async fn resolve_parent_query_scope(
        &self,
        parent_id: Uuid,
        recursive: bool,
    ) -> anyhow::Result<ParentQueryScope> {
        let parent_row: Option<(String, Option<Uuid>, Option<i32>)> = sqlx::query_as(
            "SELECT item_type, series_id, season_number FROM media_items WHERE id = $1 LIMIT 1",
        )
        .bind(parent_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some((item_type, series_id, season_number)) = parent_row else {
            let library_exists: Option<Uuid> =
                sqlx::query_scalar("SELECT id FROM libraries WHERE id = $1 LIMIT 1")
                    .bind(parent_id)
                    .fetch_optional(&self.pool)
                    .await?;
            if library_exists.is_some() {
                return Ok(ParentQueryScope::Library {
                    parent_id,
                    recursive,
                });
            }
            return Ok(ParentQueryScope::Fallback {
                parent_id,
                recursive,
            });
        };

        if item_type.eq_ignore_ascii_case("CollectionFolder") {
            return Ok(ParentQueryScope::Library {
                parent_id,
                recursive,
            });
        }
        if item_type.eq_ignore_ascii_case("Series") {
            return Ok(ParentQueryScope::Series {
                series_id: parent_id,
                recursive,
            });
        }
        if item_type.eq_ignore_ascii_case("Season")
            && let (Some(series_id), Some(season_number)) = (series_id, season_number)
        {
            return Ok(ParentQueryScope::Season {
                series_id,
                season_number,
            });
        }

        Ok(ParentQueryScope::Fallback {
            parent_id,
            recursive,
        })
    }

    fn apply_parent_scope_to_items_query(
        qb: &mut QueryBuilder<Postgres>,
        parent_scope: ParentQueryScope,
    ) {
        match parent_scope {
            ParentQueryScope::Library {
                parent_id,
                recursive,
            } => {
                qb.push(" AND m.library_id = ").push_bind(parent_id);
                if !recursive {
                    qb.push(" AND m.item_type != 'Season' AND m.item_type != 'Episode'");
                }
            }
            ParentQueryScope::Series {
                series_id,
                recursive,
            } => {
                qb.push(" AND m.series_id = ").push_bind(series_id);
                if recursive {
                    qb.push(" AND m.item_type IN ('Season', 'Episode')");
                } else {
                    qb.push(" AND m.item_type = 'Season'");
                }
            }
            ParentQueryScope::Season {
                series_id,
                season_number,
            } => {
                qb.push(" AND m.item_type = 'Episode' AND m.series_id = ")
                    .push_bind(series_id)
                    .push(" AND m.season_number = ")
                    .push_bind(season_number);
            }
            ParentQueryScope::Fallback {
                parent_id,
                recursive,
            } => {
                let _ = recursive;
                qb.push(" AND (m.library_id = ")
                    .push_bind(parent_id)
                    .push(" OR m.series_id = ")
                    .push_bind(parent_id)
                    .push(")");
            }
        }
    }

    fn parent_scope_prefers_season_episode_sort(parent_scope: Option<ParentQueryScope>) -> bool {
        matches!(
            parent_scope,
            Some(ParentQueryScope::Series { .. } | ParentQueryScope::Season { .. })
        )
    }

    #[cfg(test)]
    fn sort_name_order_expression() -> &'static str {
        "COALESCE(sort_name, name)"
    }

    fn normalize_items_sort_field(sort_field: &str) -> ItemsSortField {
        if sort_field.eq_ignore_ascii_case("SortName") {
            ItemsSortField::SortName
        } else if sort_field.eq_ignore_ascii_case("DateCreated") {
            ItemsSortField::DateCreated
        } else if sort_field.eq_ignore_ascii_case("DateLastContentAdded") {
            ItemsSortField::DateCreated
        } else if sort_field.eq_ignore_ascii_case("PremiereDate") {
            ItemsSortField::PremiereDate
        } else if sort_field.eq_ignore_ascii_case("ProductionYear") {
            ItemsSortField::ProductionYear
        } else if sort_field.eq_ignore_ascii_case("CommunityRating") {
            ItemsSortField::CommunityRating
        } else if sort_field.eq_ignore_ascii_case("Random") {
            ItemsSortField::Random
        } else if sort_field.eq_ignore_ascii_case("SeriesSortName") {
            ItemsSortField::SeriesSortName
        } else if sort_field.eq_ignore_ascii_case("ParentIndexNumber") {
            ItemsSortField::ParentIndexNumber
        } else if sort_field.eq_ignore_ascii_case("IndexNumber") {
            ItemsSortField::IndexNumber
        } else {
            ItemsSortField::Name
        }
    }

    fn items_sort_fields(sort_by: &[String]) -> Vec<ItemsSortField> {
        sort_by
            .iter()
            .map(|value| Self::normalize_items_sort_field(value))
            .collect()
    }

    fn needs_series_sort_name_join(sort_fields: &[ItemsSortField]) -> bool {
        sort_fields
            .iter()
            .any(|field| matches!(field, ItemsSortField::SeriesSortName))
    }

    fn requires_watch_state_join(options: &ItemsQuery) -> bool {
        options.user_id.is_some()
            && (options.is_resumable || options.is_played.is_some() || options.is_favorite.is_some())
    }

    fn requires_watch_state_match_without_user(options: &ItemsQuery) -> bool {
        options.user_id.is_none()
            && (options.is_resumable
                || matches!(options.is_played, Some(true))
                || matches!(options.is_favorite, Some(true)))
    }

    fn requires_items_compat_fallback(options: &ItemsQuery) -> bool {
        let common = options.parent_id.is_some()
            && options.series_filter.is_none()
            && options.search_term.is_none()
            && options.person_ids.is_empty()
            && options.tags.is_empty();
        if !common {
            return false;
        }

        let season_fallback_requested = !options.include_item_types.is_empty()
            && options
                .include_item_types
                .iter()
                .all(|item_type| item_type.eq_ignore_ascii_case("Season"));
        let episode_fallback_requested = !options.include_item_types.is_empty()
            && options.include_item_types.iter().any(|item_type| {
                item_type.eq_ignore_ascii_case("Episode")
                    || item_type.eq_ignore_ascii_case("Video")
            });

        season_fallback_requested || episode_fallback_requested
    }

    fn can_sql_paginate_items_query(
        options: &ItemsQuery,
        meili_all_ids: Option<&Vec<Uuid>>,
        meili_person_ids: &[Uuid],
    ) -> bool {
        meili_all_ids.is_none()
            && meili_person_ids.is_empty()
            && !Self::requires_items_compat_fallback(options)
    }

    pub async fn compat_item_ids_for_uuids(
        &self,
        entity_ids: &[Uuid],
    ) -> anyhow::Result<HashMap<Uuid, i64>> {
        if entity_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let mut unique_ids = Vec::with_capacity(entity_ids.len());
        let mut seen = HashSet::with_capacity(entity_ids.len());
        for entity_id in entity_ids {
            if seen.insert(*entity_id) {
                unique_ids.push(*entity_id);
            }
        }

        sqlx::query(
            r#"
INSERT INTO item_id_aliases (entity_id)
SELECT * FROM UNNEST($1::uuid[])
ON CONFLICT(entity_id) DO NOTHING
            "#,
        )
        .bind(&unique_ids)
        .execute(&self.pool)
        .await?;

        let rows = sqlx::query_as::<_, (Uuid, i64)>(
            "SELECT entity_id, compat_id FROM item_id_aliases WHERE entity_id = ANY($1)",
        )
        .bind(&unique_ids)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().collect())
    }

    pub async fn compat_item_id_for_uuid(&self, entity_id: Uuid) -> anyhow::Result<i64> {
        let compat_map = self.compat_item_ids_for_uuids(&[entity_id]).await?;
        compat_map
            .get(&entity_id)
            .copied()
            .context("missing compat item id alias")
    }

    pub async fn resolve_uuid_by_compat_item_id(
        &self,
        compat_id: i64,
    ) -> anyhow::Result<Option<Uuid>> {
        if compat_id <= 0 {
            return Ok(None);
        }

        let entity_id: Option<Uuid> =
            sqlx::query_scalar("SELECT entity_id FROM item_id_aliases WHERE compat_id = $1 LIMIT 1")
                .bind(compat_id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(entity_id)
    }

    pub async fn resolve_uuid_by_any_item_id(
        &self,
        raw_item_id: &str,
    ) -> anyhow::Result<Option<Uuid>> {
        let value = raw_item_id.trim();
        if value.is_empty() {
            return Ok(None);
        }

        if let Ok(uuid) = Uuid::parse_str(value) {
            return Ok(Some(uuid));
        }

        let Ok(compat_id) = value.parse::<i64>() else {
            return Ok(None);
        };
        self.resolve_uuid_by_compat_item_id(compat_id).await
    }

    pub async fn delete_item(&self, item_id: Uuid) -> anyhow::Result<bool> {
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM media_items WHERE id = $1)")
            .bind(item_id)
            .fetch_one(&self.pool)
            .await?;
        if !exists {
            return Ok(false);
        }

        let _ = self.delete_items_bulk(&[item_id]).await?;
        Ok(true)
    }

    pub async fn delete_items_bulk(&self, item_ids: &[Uuid]) -> anyhow::Result<i64> {
        if item_ids.is_empty() {
            return Ok(0);
        }

        let targets = sqlx::query_as::<_, (Uuid, String)>(
            r#"
SELECT DISTINCT id, path
FROM media_items
WHERE id = ANY($1)
   OR (
        series_id = ANY($1)
    AND item_type IN ('Season', 'Episode')
   )
            "#,
        )
        .bind(item_ids)
        .fetch_all(&self.pool)
        .await?;
        if targets.is_empty() {
            return Ok(0);
        }

        let delete_ids = targets.iter().map(|(id, _)| *id).collect::<Vec<_>>();
        let strm_paths = targets
            .iter()
            .map(|(_, path)| path)
            .filter(|path| should_delete_linked_strm(path))
            .cloned()
            .collect::<Vec<_>>();

        let result = sqlx::query("DELETE FROM media_items WHERE id = ANY($1)")
            .bind(&delete_ids)
            .execute(&self.pool)
            .await?;
        remove_linked_strm_files(&strm_paths).await;

        Ok(result.rows_affected() as i64)
    }

    pub async fn patch_item_metadata(
        &self,
        item_id: Uuid,
        name: Option<&str>,
        patch: Option<&Value>,
    ) -> anyhow::Result<bool> {
        let Some((current_name, current_metadata)) =
            sqlx::query_as::<_, (String, Value)>("SELECT name, metadata FROM media_items WHERE id = $1 LIMIT 1")
                .bind(item_id)
                .fetch_optional(&self.pool)
                .await?
        else {
            return Ok(false);
        };

        let merged_metadata = patch
            .map(|value| merge_item_metadata_patch(current_metadata.clone(), value))
            .unwrap_or(current_metadata);
        let updated_name = name
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(current_name.as_str());

        let result = sqlx::query(
            r#"
UPDATE media_items
SET name = $2, metadata = $3, updated_at = now()
WHERE id = $1
            "#,
        )
        .bind(item_id)
        .bind(updated_name)
        .bind(merged_metadata)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn item_library_id(&self, item_id: Uuid) -> anyhow::Result<Option<Uuid>> {
        let library_id: Option<Uuid> =
            sqlx::query_scalar("SELECT library_id FROM media_items WHERE id = $1 LIMIT 1")
                .bind(item_id)
                .fetch_optional(&self.pool)
                .await?
                .flatten();
        Ok(library_id)
    }

    pub async fn list_item_name_prefixes(
        &self,
        options: ItemsQuery,
    ) -> anyhow::Result<Vec<String>> {
        let items = self.list_items_with_options(options).await?;
        let mut seen = std::collections::BTreeSet::new();
        for item in items.items {
            let Some(prefix) = item
                .name
                .trim_start()
                .chars()
                .next()
                .map(|ch| ch.to_ascii_uppercase().to_string())
            else {
                continue;
            };
            seen.insert(prefix);
        }
        Ok(seen.into_iter().collect())
    }

    pub async fn list_root_items(&self) -> anyhow::Result<QueryResultDto<BaseItemDto>> {
        let mut rows = self
            .list_libraries()
            .await?
            .into_iter()
            .filter(|library| library.enabled)
            .collect::<Vec<_>>();
        rows.sort_by(|a, b| a.name.cmp(&b.name));

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let child_count: i64 = sqlx::query_scalar(
                r#"
SELECT COUNT(*)
FROM media_items
WHERE library_id = $1
  AND item_type IN ('Movie', 'Series')
  AND version_rank = 0
                "#,
            )
            .bind(row.id)
            .fetch_one(&self.pool)
            .await?;

            let primary_image_tag = root_library_primary_image_tag(&row.root_path);
            let image_tags = primary_image_tag
                .clone()
                .map(|tag| std::collections::HashMap::from([("Primary".to_string(), tag)]))
                .or_else(|| Some(std::collections::HashMap::new()));

            items.push(BaseItemDto {
                id: row.id.to_string(),
                name: row.name.clone(),
                item_type: "CollectionFolder".to_string(),
                path: row.root_path,
                is_folder: Some(true),
                media_type: None,
                container: None,
                location_type: Some("FileSystem".to_string()),
                can_delete: Some(false),
                can_download: Some(false),
                collection_type: Some(normalize_emby_collection_type(&row.library_type)),
                runtime_ticks: None,
                bitrate: None,
                media_sources: None,
                user_data: Some(UserDataDto {
                    played: false,
                    playback_position_ticks: 0,
                    is_favorite: Some(false),
                }),
                overview: None,
                premiere_date: None,
                end_date: None,
                production_year: None,
                genres: None,
                tags: None,
                provider_ids: Some(std::collections::HashMap::new()),
                image_tags,
                primary_image_tag,
                parent_id: Some("2".to_string()),
                series_id: None,
                series_name: None,
                season_id: None,
                season_name: None,
                index_number: None,
                parent_index_number: None,
                backdrop_image_tags: Some(Vec::new()),
                official_rating: None,
                community_rating: None,
                studios: None,
                people: None,
                sort_name: Some(row.name),
                primary_image_aspect_ratio: None,
                date_created: Some(row.created_at.to_rfc3339()),
                child_count: Some(i32::try_from(child_count).unwrap_or(i32::MAX)),
                recursive_item_count: None,
                play_access: None,
            });
        }

        Ok(QueryResultDto {
            total_record_count: items.len() as i32,
            start_index: 0,
            items,
        })
    }

    /// List persisted season rows for a series.
    pub async fn list_seasons(
        &self,
        series_id: Uuid,
        user_id: Option<Uuid>,
    ) -> anyhow::Result<QueryResultDto<BaseItemDto>> {
        // Canonical semantics: seasons are first-class rows (no virtual season synthesis).
        let rows = sqlx::query_as::<_, SeasonRow>(
            r#"
SELECT
    id,
    series_id,
    season_number,
    name,
    path,
    metadata,
    created_at
FROM media_items
WHERE series_id = $1
  AND item_type = 'Season'
ORDER BY
    season_number ASC NULLS LAST,
    created_at ASC
            "#,
        )
        .bind(series_id)
        .fetch_all(&self.pool)
        .await?;

        // Get series name for display
        let series_name: Option<String> = sqlx::query_scalar(
            "SELECT name FROM media_items WHERE id = $1 AND item_type = 'Series'",
        )
        .bind(series_id)
        .fetch_optional(&self.pool)
        .await?;

        let season_ids = rows.iter().map(|row| row.id).collect::<Vec<_>>();
        let mut season_user_data = if let Some(user_id) = user_id {
            self.user_data_map(user_id, &season_ids).await?
        } else {
            std::collections::HashMap::new()
        };
        let season_numbers = rows
            .iter()
            .filter_map(|row| row.season_number)
            .collect::<Vec<_>>();
        let episode_count_rows = if season_numbers.is_empty() {
            Vec::new()
        } else {
            sqlx::query_as::<_, (i32, i64)>(
                r#"
SELECT season_number, COUNT(*)::BIGINT AS count
FROM media_items
WHERE series_id = $1
  AND item_type = 'Episode'
  AND season_number = ANY($2)
  AND version_rank = 0
GROUP BY season_number
                "#,
            )
            .bind(series_id)
            .bind(&season_numbers)
            .fetch_all(&self.pool)
            .await?
        };
        let episode_count_by_season: std::collections::HashMap<i32, i64> =
            episode_count_rows.into_iter().collect();

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let user_data = season_user_data.remove(&row.id);

            // Count episodes in this season
            let Some(season_number) = row.season_number else {
                continue;
            };
            let episode_count = *episode_count_by_season.get(&season_number).unwrap_or(&0);

            let season_name = if row.name.trim().is_empty() {
                format!("Season {season_number:02}")
            } else {
                row.name.clone()
            };
            let location_type = if row.path.trim().is_empty() {
                None
            } else {
                Some("FileSystem".to_string())
            };

            items.push(BaseItemDto {
                id: row.id.to_string(),
                name: season_name.clone(),
                item_type: "Season".to_string(),
                path: row.path,
                is_folder: Some(true),
                media_type: None,
                container: None,
                location_type,
                can_delete: Some(false),
                can_download: Some(false),
                collection_type: None,
                runtime_ticks: None,
                bitrate: None,
                media_sources: None,
                user_data,
                overview: None,
                premiere_date: None,
                end_date: None,
                production_year: None,
                genres: None,
                tags: None,
                provider_ids: None,
                image_tags: Some(std::collections::HashMap::from([(
                    "Primary".to_string(),
                    row.id.to_string(),
                )])),
                primary_image_tag: Some(row.id.to_string()),
                parent_id: Some(series_id.to_string()),
                series_id: Some(series_id.to_string()),
                series_name: series_name.clone(),
                season_id: None,
                season_name: Some(season_name),
                index_number: Some(season_number),
                parent_index_number: None,
                backdrop_image_tags: None,
                official_rating: None,
                community_rating: None,
                studios: None,
                people: None,
                sort_name: None,
                primary_image_aspect_ratio: None,
                date_created: Some(row.created_at.to_rfc3339()),
                child_count: Some(episode_count as i32),
                recursive_item_count: Some(episode_count as i32),
                play_access: None,
            });
        }

        Ok(QueryResultDto {
            total_record_count: items.len() as i32,
            start_index: 0,
            items,
        })
    }

    /// List next-up episode candidates for each in-progress series.
    pub async fn list_next_up(
        &self,
        user_id: Uuid,
        series_filter: Option<Uuid>,
        include_resumable: bool,
        include_rewatching: bool,
    ) -> anyhow::Result<QueryResultDto<BaseItemDto>> {
        // Find the next unwatched episode after the last watched for each series
        let rows = sqlx::query_as::<_, NextUpRow>(
            r#"
WITH last_watched AS (
    -- Get the last watched episode per series
    SELECT DISTINCT ON (m.series_id)
        m.series_id,
        m.season_number AS last_season,
        m.episode_number AS last_episode
    FROM watch_states w
    JOIN media_items m ON m.id = w.media_item_id
    WHERE w.user_id = $1
      AND m.item_type = 'Episode'
      AND m.series_id IS NOT NULL
      AND (w.played = true OR w.playback_position_ticks > 0)
      AND ($2::uuid IS NULL OR m.series_id = $2)
      AND m.version_rank = 0
    ORDER BY m.series_id, w.last_played_at DESC NULLS LAST
),
next_episodes AS (
    -- Find the next episode after the last watched
    SELECT DISTINCT ON (m.series_id)
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
    FROM media_items m
    JOIN last_watched lw ON m.series_id = lw.series_id
    LEFT JOIN watch_states w ON w.media_item_id = m.id AND w.user_id = $1
    WHERE m.item_type = 'Episode'
      AND m.version_rank = 0
      AND (
          w.played IS NULL
          OR w.played = false
          OR ($3::boolean = true AND w.played = true)
      )
      AND ($4::boolean = true OR COALESCE(w.playback_position_ticks, 0) = 0)
      AND (
          m.season_number > lw.last_season
          OR (m.season_number = lw.last_season AND m.episode_number > lw.last_episode)
      )
    ORDER BY m.series_id, m.season_number ASC, m.episode_number ASC
)
SELECT * FROM next_episodes
            "#,
        )
        .bind(user_id)
        .bind(series_filter)
        .bind(include_rewatching)
        .bind(include_resumable)
        .fetch_all(&self.pool)
        .await?;

        let next_up_ids = rows.iter().map(|row| row.id).collect::<Vec<_>>();
        let mut user_data_map = self.user_data_map(user_id, &next_up_ids).await?;
        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let user_data = user_data_map.remove(&row.id);
            let media_row = MediaItemRow {
                id: row.id,
                item_type: row.item_type,
                name: row.name,
                path: row.path,
                runtime_ticks: row.runtime_ticks,
                bitrate: row.bitrate,
                series_id: row.series_id,
                season_number: row.season_number,
                episode_number: row.episode_number,
                library_id: row.library_id,
                metadata: row.metadata,
                created_at: row.created_at,
            };
            items.push(item_row_to_dto(media_row, user_data));
        }
        self.attach_grouped_media_sources(&mut items).await?;

        Ok(QueryResultDto {
            total_record_count: items.len() as i32,
            start_index: 0,
            items,
        })
    }

    pub async fn list_resume_items(
        &self,
        user_id: Uuid,
    ) -> anyhow::Result<QueryResultDto<BaseItemDto>> {
        let rows = sqlx::query_as::<_, ResumeItemRow>(
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
    m.created_at,
    w.playback_position_ticks,
    w.played
FROM watch_states w
JOIN media_items m ON m.id = w.media_item_id
WHERE w.user_id = $1
  AND w.playback_position_ticks > 0
  AND COALESCE(w.played, false) = false
  AND m.version_rank = 0
ORDER BY w.last_played_at DESC NULLS LAST, m.updated_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut items = rows
            .into_iter()
            .map(|row| {
                let media_row = MediaItemRow {
                    id: row.id,
                    item_type: row.item_type,
                    name: row.name,
                    path: row.path,
                    runtime_ticks: row.runtime_ticks,
                    bitrate: row.bitrate,
                    series_id: row.series_id,
                    season_number: row.season_number,
                    episode_number: row.episode_number,
                    library_id: row.library_id,
                    metadata: row.metadata,
                    created_at: row.created_at,
                };
                let user_data = Some(UserDataDto {
                    played: row.played,
                    playback_position_ticks: row.playback_position_ticks,
                    is_favorite: None, // Resume items don't need is_favorite
                });
                item_row_to_dto(media_row, user_data)
            })
            .collect::<Vec<_>>();
        self.attach_grouped_media_sources(&mut items).await?;

        Ok(QueryResultDto {
            total_record_count: items.len() as i32,
            start_index: 0,
            items,
        })
    }

    pub async fn list_items(
        &self,
        user_id: Option<Uuid>,
        series_filter: Option<Uuid>,
    ) -> anyhow::Result<QueryResultDto<BaseItemDto>> {
        self.list_items_with_options(ItemsQuery {
            user_id,
            series_filter,
            parent_id: None,
            include_item_types: vec![],
            exclude_item_types: vec![],
            person_ids: vec![],
            search_term: None,
            limit: 500,
            start_index: 0,
            is_resumable: false,
            sort_by: vec![],
            sort_order: "Ascending".to_string(),
            recursive: false,
            genres: vec![],
            tags: vec![],
            years: vec![],
            is_favorite: None,
            is_played: None,
            min_community_rating: None,
        })
        .await
    }

    pub async fn list_latest_items_for_user(
        &self,
        user_id: Uuid,
        parent_id: Option<Uuid>,
        include_item_types: Vec<String>,
        is_played: Option<bool>,
        limit: i64,
    ) -> anyhow::Result<Vec<BaseItemDto>> {
        let limit = limit.clamp(1, 200);
        let include_item_types = include_item_types
            .into_iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();

        let parent_scope = if let Some(parent_id) = parent_id {
            Some(self.resolve_parent_query_scope(parent_id, true).await?)
        } else {
            None
        };

        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
            "SELECT m.id, m.item_type, m.name, m.path, m.runtime_ticks, m.bitrate, m.series_id, m.season_number, m.episode_number, m.library_id, m.metadata, m.created_at FROM media_items m WHERE m.version_rank = 0",
        );

        if let Some(parent_scope) = parent_scope {
            Self::apply_parent_scope_to_items_query(&mut qb, parent_scope);
        }
        if !include_item_types.is_empty() {
            qb.push(" AND m.item_type = ANY(")
                .push_bind(include_item_types)
                .push(")");
        }

        qb.push(" ORDER BY m.created_at DESC NULLS LAST, m.name ASC");
        qb.push(" LIMIT ").push_bind(limit);

        let rows = qb
            .build_query_as::<MediaItemRow>()
            .fetch_all(&self.pool)
            .await?;

        let row_ids = rows.iter().map(|row| row.id).collect::<Vec<_>>();
        let mut user_data_map = self.user_data_map(user_id, &row_ids).await?;
        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let user_data = user_data_map.remove(&row.id);
            if let Some(expected_played) = is_played {
                let played = user_data.as_ref().map(|data| data.played).unwrap_or(false);
                if played != expected_played {
                    continue;
                }
            }
            items.push(item_row_to_dto(row, user_data));
            if items.len() >= limit as usize {
                break;
            }
        }

        self.hydrate_items_people_from_relations(&mut items).await?;
        self.attach_grouped_media_sources(&mut items).await?;
        Ok(items)
    }

    pub async fn list_items_with_options(
        &self,
        options: ItemsQuery,
    ) -> anyhow::Result<QueryResultDto<BaseItemDto>> {
        let options = options.normalize();
        let meili_result = if let Some(search_term) = options.search_term.as_deref() {
            Some(self.search_ids_from_meili(search_term, &options).await?)
        } else {
            None
        };

        if matches!(meili_result.as_ref(), Some((all, _, _)) if all.is_empty()) {
            return Ok(QueryResultDto {
                total_record_count: 0,
                start_index: options.start_index as i32,
                items: Vec::new(),
            });
        }

        let (meili_all_ids, meili_media_ids, meili_person_ids) = match meili_result {
            Some((a, m, p)) => (Some(a), Some(m), p),
            None => (None, None, Vec::new()),
        };

        let parent_scope = if let Some(parent_id) = options.parent_id {
            Some(
                self.resolve_parent_query_scope(parent_id, options.recursive)
                    .await?,
            )
        } else {
            None
        };

        if Self::requires_watch_state_match_without_user(&options) {
            return Ok(QueryResultDto {
                total_record_count: 0,
                start_index: options.start_index as i32,
                items: Vec::new(),
            });
        }

        let sort_fields = Self::items_sort_fields(&options.sort_by);
        let requires_watch_state_join = Self::requires_watch_state_join(&options);
        let can_sql_paginate =
            Self::can_sql_paginate_items_query(&options, meili_all_ids.as_ref(), &meili_person_ids);

        let mut qb: QueryBuilder<Postgres> = if can_sql_paginate {
            QueryBuilder::new(
                "SELECT m.id, m.item_type, m.name, m.path, m.runtime_ticks, m.bitrate, m.series_id, m.season_number, m.episode_number, m.library_id, m.metadata, m.created_at, COUNT(*) OVER() AS total_count FROM media_items m",
            )
        } else {
            QueryBuilder::new(
                "SELECT m.id, m.item_type, m.name, m.path, m.runtime_ticks, m.bitrate, m.series_id, m.season_number, m.episode_number, m.library_id, m.metadata, m.created_at FROM media_items m",
            )
        };
        if Self::needs_series_sort_name_join(&sort_fields) {
            qb.push(
                " LEFT JOIN media_items series_item ON series_item.id = m.series_id AND series_item.version_rank = 0",
            );
        }
        if let Some(user_id) = options.user_id.filter(|_| requires_watch_state_join) {
            qb.push(" LEFT JOIN watch_states w ON w.media_item_id = m.id AND w.user_id = ")
                .push_bind(user_id);
        }
        qb.push(" WHERE m.version_rank = 0");

        if let Some(series_id) = options.series_filter {
            qb.push(" AND m.series_id = ").push_bind(series_id);
        }

        if let Some(parent_scope) = parent_scope {
            Self::apply_parent_scope_to_items_query(&mut qb, parent_scope);
        }

        if !options.include_item_types.is_empty() {
            qb.push(" AND m.item_type = ANY(")
                .push_bind(options.include_item_types.clone())
                .push(")");
        }

        if !options.exclude_item_types.is_empty() {
            qb.push(" AND m.item_type != ALL(")
                .push_bind(options.exclude_item_types.clone())
                .push(")");
        }

        if !options.person_ids.is_empty() {
            qb.push(
                " AND m.id IN (SELECT media_item_id FROM media_item_people WHERE person_id = ANY(",
            )
            .push_bind(options.person_ids.clone())
            .push("))");
        }

        if let Some(ids) = meili_media_ids.as_ref() {
            qb.push(" AND m.id = ANY(").push_bind(ids.clone()).push(")");
        }

        // Wave 3: Genre filter (JSONB array contains)
        if !options.genres.is_empty() {
            qb.push(" AND (");
            for (i, genre) in options.genres.iter().enumerate() {
                if i > 0 {
                    qb.push(" OR ");
                }
                qb.push("m.metadata->'genres' @> ")
                    .push_bind(serde_json::json!([genre]));
            }
            qb.push(")");
        }

        if !options.tags.is_empty() {
            qb.push(" AND (");
            for (i, tag) in options.tags.iter().enumerate() {
                if i > 0 {
                    qb.push(" OR ");
                }
                qb.push(
                    "EXISTS (SELECT 1 FROM jsonb_array_elements_text(CASE WHEN jsonb_typeof(m.metadata->'tags') = 'array' THEN m.metadata->'tags' ELSE '[]'::jsonb END) AS tag_value(value) WHERE LOWER(tag_value.value) = LOWER(",
                )
                .push_bind(tag)
                .push("))");
            }
            qb.push(")");
        }

        if !options.years.is_empty() {
            qb.push(" AND (m.metadata->>'production_year')::int = ANY(")
                .push_bind(options.years.clone())
                .push(")");
        }

        if let Some(min_rating) = options.min_community_rating {
            qb.push(" AND (m.metadata->>'community_rating')::float >= ")
                .push_bind(min_rating);
        }

        if options.is_resumable {
            qb.push(" AND COALESCE(w.played, FALSE) = FALSE AND COALESCE(w.playback_position_ticks, 0) > 0");
        }
        if let Some(is_played) = options.is_played {
            qb.push(" AND COALESCE(w.played, FALSE) = ")
                .push_bind(is_played);
        }
        if let Some(is_favorite) = options.is_favorite {
            qb.push(" AND COALESCE(w.is_favorite, FALSE) = ")
                .push_bind(is_favorite);
        }

        // Sorting logic
        if let Some(ids) = meili_media_ids.as_ref().filter(|ids| !ids.is_empty()) {
            qb.push(" ORDER BY array_position(")
                .push_bind(ids.clone())
                .push(", m.id) ASC, m.updated_at DESC, m.name ASC");
        } else if options.series_filter.is_some()
            || Self::parent_scope_prefers_season_episode_sort(parent_scope)
        {
            qb.push(" ORDER BY m.season_number NULLS FIRST, m.episode_number NULLS FIRST, m.name ASC");
        } else if !sort_fields.is_empty() {
            qb.push(" ORDER BY ");
            let desc = options.sort_order.eq_ignore_ascii_case("Descending");
            let order_suffix = if desc { " DESC" } else { " ASC" };
            let nulls_suffix = if desc { " NULLS LAST" } else { " NULLS FIRST" };

            for (i, sort_field) in sort_fields.iter().enumerate() {
                if i > 0 {
                    qb.push(", ");
                }
                match sort_field {
                    ItemsSortField::SortName => {
                        qb.push("COALESCE(m.sort_name, m.name)").push(order_suffix);
                    }
                    ItemsSortField::DateCreated => {
                        qb.push("m.created_at").push(order_suffix).push(nulls_suffix);
                    }
                    ItemsSortField::PremiereDate => {
                        qb.push("m.metadata->>'premiere_date'")
                            .push(order_suffix)
                            .push(nulls_suffix);
                    }
                    ItemsSortField::ProductionYear => {
                        qb.push("(m.metadata->>'production_year')::int")
                            .push(order_suffix)
                            .push(nulls_suffix);
                    }
                    ItemsSortField::CommunityRating => {
                        qb.push("(m.metadata->>'community_rating')::float")
                            .push(order_suffix)
                            .push(nulls_suffix);
                    }
                    ItemsSortField::Random => {
                        qb.push("RANDOM()");
                    }
                    ItemsSortField::SeriesSortName => {
                        qb.push("COALESCE(series_item.sort_name, series_item.name, m.sort_name, m.name)")
                            .push(order_suffix);
                    }
                    ItemsSortField::ParentIndexNumber => {
                        qb.push("m.season_number")
                            .push(order_suffix)
                            .push(nulls_suffix);
                    }
                    ItemsSortField::IndexNumber => {
                        qb.push("m.episode_number")
                            .push(order_suffix)
                            .push(nulls_suffix);
                    }
                    ItemsSortField::Name => {
                        qb.push("m.name").push(order_suffix);
                    }
                }
            }
        } else {
            qb.push(" ORDER BY m.created_at DESC, m.name ASC");
        }

        if can_sql_paginate {
            qb.push(" LIMIT ").push_bind(options.limit);
            qb.push(" OFFSET ").push_bind(options.start_index);
        }

        let mut sql_total_count = None;
        let rows = if can_sql_paginate {
            let paged_rows = qb
                .build_query_as::<MediaItemWithTotalRow>()
                .fetch_all(&self.pool)
                .await?;
            sql_total_count = paged_rows.first().map(|row| row.total_count as i32);
            paged_rows
                .into_iter()
                .map(MediaItemRow::from)
                .collect::<Vec<_>>()
        } else {
            qb.build_query_as::<MediaItemRow>()
                .fetch_all(&self.pool)
                .await?
        };

        let mut user_data_by_item_id = std::collections::HashMap::new();
        if let Some(user_id) = options.user_id {
            let item_ids = rows.iter().map(|row| row.id).collect::<Vec<_>>();
            user_data_by_item_id = self.user_data_map(user_id, &item_ids).await?;
        }

        let mut filtered = Vec::with_capacity(rows.len());
        for row in rows {
            let user_data = user_data_by_item_id.remove(&row.id);
            filtered.push(item_row_to_dto(row, user_data));
        }
        self.hydrate_items_people_from_relations(&mut filtered).await?;

        if filtered.is_empty()
            && options.start_index == 0
            && options.parent_id.is_some()
            && options
                .include_item_types
                .iter()
                .all(|item_type| item_type.eq_ignore_ascii_case("Season"))
            && !options.include_item_types.is_empty()
            && options.series_filter.is_none()
            && options.search_term.is_none()
            && options.person_ids.is_empty()
            && options.tags.is_empty()
        {
            let Some(parent_id) = options.parent_id else {
                unreachable!("guarded by parent_id.is_some()");
            };
            let parent_item_type: Option<String> =
                sqlx::query_scalar("SELECT item_type FROM media_items WHERE id = $1 LIMIT 1")
                    .bind(parent_id)
                    .fetch_optional(&self.pool)
                    .await?;
            if parent_item_type
                .as_deref()
                .map(|value| value.eq_ignore_ascii_case("Series"))
                .unwrap_or(false)
            {
                let seasons = self.list_seasons(parent_id, options.user_id).await?;
                let total = seasons.total_record_count;
                let start = options.start_index as usize;
                let limit = options.limit as usize;
                let items = if start >= seasons.items.len() {
                    Vec::new()
                } else {
                    seasons
                        .items
                        .into_iter()
                        .skip(start)
                        .take(limit)
                        .collect::<Vec<_>>()
                };
                return Ok(QueryResultDto {
                    total_record_count: total,
                    start_index: options.start_index as i32,
                    items,
                });
            }
        }

        if filtered.is_empty()
            && options.start_index == 0
            && options.parent_id.is_some()
            && options.series_filter.is_none()
            && options.search_term.is_none()
            && options.person_ids.is_empty()
            && options.tags.is_empty()
            && (options.include_item_types.is_empty()
                || options.include_item_types.iter().any(|item_type| {
                    item_type.eq_ignore_ascii_case("Episode")
                        || item_type.eq_ignore_ascii_case("Video")
                }))
        {
            let Some(parent_id) = options.parent_id else {
                unreachable!("guarded by parent_id.is_some()");
            };
            let parent_row: Option<(String, Option<Uuid>, Option<i32>)> = sqlx::query_as(
                "SELECT item_type, series_id, season_number FROM media_items WHERE id = $1 LIMIT 1",
            )
            .bind(parent_id)
            .fetch_optional(&self.pool)
            .await?;
            if let Some((parent_item_type, parent_series_id, parent_season_number)) = parent_row
                && parent_item_type.eq_ignore_ascii_case("Season")
            {
                if let (Some(series_id), Some(season_number)) = (parent_series_id, parent_season_number)
                {
                    let episode_rows = sqlx::query_as::<_, MediaItemRow>(
                        r#"
SELECT
    id,
    item_type,
    name,
    path,
    runtime_ticks,
    bitrate,
    series_id,
    season_number,
    episode_number,
    library_id,
    metadata,
    created_at
FROM media_items
WHERE item_type = 'Episode'
  AND series_id = $1
  AND season_number = $2
ORDER BY episode_number ASC NULLS LAST, name ASC
                        "#,
                    )
                    .bind(series_id)
                    .bind(season_number)
                    .fetch_all(&self.pool)
                    .await?;
                    let mut episode_user_data = std::collections::HashMap::new();
                    if let Some(user_id) = options.user_id {
                        let episode_ids = episode_rows.iter().map(|row| row.id).collect::<Vec<_>>();
                        episode_user_data = self.user_data_map(user_id, &episode_ids).await?;
                    }
                    let mut scoped = Vec::with_capacity(episode_rows.len());
                    for row in episode_rows {
                        let user_data = episode_user_data.remove(&row.id);
                        if options.is_resumable {
                            let Some(data) = user_data.as_ref() else {
                                continue;
                            };
                            if data.played || data.playback_position_ticks <= 0 {
                                continue;
                            }
                        }
                        if let Some(is_played) = options.is_played {
                            let played = user_data.as_ref().map(|d| d.played).unwrap_or(false);
                            if played != is_played {
                                continue;
                            }
                        }
                        if let Some(is_favorite) = options.is_favorite {
                            let favorite = user_data
                                .as_ref()
                                .and_then(|d| d.is_favorite)
                                .unwrap_or(false);
                            if favorite != is_favorite {
                                continue;
                            }
                        }
                        let mut item = item_row_to_dto(row, user_data);
                        let season_id = parent_id.to_string();
                        item.parent_id = Some(season_id.clone());
                        item.season_id = item
                            .season_id
                            .or(Some(season_id));
                        item.series_id = item
                            .series_id
                            .or(Some(series_id.to_string()));
                        item.parent_index_number = item
                            .parent_index_number
                            .or(Some(season_number));
                        scoped.push(item);
                    }
                    self.hydrate_items_people_from_relations(&mut scoped).await?;
                    let total = scoped.len() as i32;
                    let start = options.start_index as usize;
                    let limit = options.limit as usize;
                    let items = if start >= scoped.len() {
                        Vec::new()
                    } else {
                        scoped
                            .into_iter()
                            .skip(start)
                            .take(limit)
                            .collect::<Vec<_>>()
                    };
                    let mut items = items;
                    self.attach_grouped_media_sources(&mut items).await?;
                    return Ok(QueryResultDto {
                        total_record_count: total,
                        start_index: options.start_index as i32,
                        items,
                    });
                }
            }
        }

        // Merge person results + their associated media if search returned any person IDs
        if !meili_person_ids.is_empty() {
            let person_rows = sqlx::query_as::<_, PersonRow>(
                "SELECT id, name, image_path, primary_image_tag, metadata, created_at FROM people WHERE id = ANY($1)",
            )
            .bind(&meili_person_ids)
            .fetch_all(&self.pool)
            .await?;
            let mut person_map: std::collections::HashMap<Uuid, BaseItemDto> = person_rows
                .into_iter()
                .map(|r| (r.id, person_row_to_dto(r)))
                .collect();

            // Collect IDs already present in direct media results for dedup
            let existing_media_ids: HashSet<Uuid> = filtered
                .iter()
                .filter_map(|dto| Uuid::parse_str(&dto.id).ok())
                .collect();

            // Fetch associated media for each person
            let person_assoc = self
                .fetch_media_ids_for_persons(
                    &meili_person_ids,
                    &existing_media_ids,
                    &options.include_item_types,
                    PERSON_ASSOCIATED_MEDIA_LIMIT,
                )
                .await?;

            // Batch-load all associated media rows
            let all_assoc_ids: Vec<Uuid> = person_assoc
                .iter()
                .flat_map(|(_, ids)| ids.iter().copied())
                .collect();
            let mut assoc_map: std::collections::HashMap<Uuid, BaseItemDto> =
                std::collections::HashMap::new();
            if !all_assoc_ids.is_empty() {
                let assoc_rows = sqlx::query_as::<_, MediaItemRow>(
                    "SELECT id, item_type, name, path, runtime_ticks, bitrate, series_id, season_number, episode_number, library_id, metadata, created_at FROM media_items WHERE id = ANY($1) AND version_rank = 0",
                )
                .bind(&all_assoc_ids)
                .fetch_all(&self.pool)
                .await?;

                let mut assoc_user_data = std::collections::HashMap::new();
                if let Some(user_id) = options.user_id {
                    let assoc_ids = assoc_rows.iter().map(|row| row.id).collect::<Vec<_>>();
                    assoc_user_data = self.user_data_map(user_id, &assoc_ids).await?;
                }

                let mut assoc_ids_order = Vec::with_capacity(assoc_rows.len());
                let mut assoc_dtos = Vec::with_capacity(assoc_rows.len());
                for row in assoc_rows {
                    let rid = row.id;
                    let user_data = assoc_user_data.remove(&rid);
                    assoc_ids_order.push(rid);
                    assoc_dtos.push(item_row_to_dto(row, user_data));
                }
                self.hydrate_items_people_from_relations(&mut assoc_dtos).await.ok();
                assoc_map = assoc_ids_order
                    .into_iter()
                    .zip(assoc_dtos)
                    .collect();
            }

            // Build per-person associated DTOs in sort_order
            let mut person_assoc_dtos: std::collections::HashMap<Uuid, Vec<BaseItemDto>> =
                std::collections::HashMap::new();
            for (pid, ids) in &person_assoc {
                let dtos: Vec<BaseItemDto> = ids
                    .iter()
                    .filter_map(|mid| assoc_map.remove(mid))
                    .collect();
                if !dtos.is_empty() {
                    person_assoc_dtos.insert(*pid, dtos);
                }
            }

            // Re-merge: media + person + person's associated works in Meili order
            if let Some(all_ids) = meili_all_ids.as_ref() {
                let mut media_map: std::collections::HashMap<String, BaseItemDto> = filtered
                    .into_iter()
                    .map(|dto| (dto.id.clone(), dto))
                    .collect();
                let mut merged = Vec::with_capacity(all_ids.len());
                let mut seen: HashSet<String> = HashSet::new();
                for id in all_ids {
                    let id_str = id.to_string();
                    if let Some(dto) = media_map.remove(&id_str) {
                        seen.insert(id_str);
                        merged.push(dto);
                    } else if let Some(dto) = person_map.remove(id) {
                        merged.push(dto);
                        // Inject associated media right after the person card
                        if let Some(assoc) = person_assoc_dtos.remove(id) {
                            for adto in assoc {
                                if seen.insert(adto.id.clone()) {
                                    merged.push(adto);
                                }
                            }
                        }
                    }
                }
                filtered = merged;
            } else {
                filtered.extend(person_map.into_values());
            }
        }

        let total = sql_total_count.unwrap_or(filtered.len() as i32);
        let mut items = if can_sql_paginate {
            filtered
        } else {
            let start = options.start_index as usize;
            let limit = options.limit as usize;
            if start >= filtered.len() {
                Vec::new()
            } else {
                filtered
                    .into_iter()
                    .skip(start)
                    .take(limit)
                    .collect::<Vec<_>>()
            }
        };
        self.attach_grouped_media_sources(&mut items).await?;

        Ok(QueryResultDto {
            total_record_count: total,
            start_index: options.start_index as i32,
            items,
        })
    }

    /// For each person, fetch associated media item IDs via `media_item_people`.
    /// Returns `(person_id, [media_id…])` pairs in the same order as `person_ids`.
    async fn fetch_media_ids_for_persons(
        &self,
        person_ids: &[Uuid],
        exclude_ids: &HashSet<Uuid>,
        item_types: &[String],
        limit_per_person: i64,
    ) -> anyhow::Result<Vec<(Uuid, Vec<Uuid>)>> {
        if person_ids.is_empty() {
            return Ok(Vec::new());
        }

        let types: Vec<String> = if item_types.is_empty() {
            vec!["Movie".to_string(), "Series".to_string()]
        } else {
            item_types.to_vec()
        };

        let rows: Vec<(Uuid, Uuid)> = sqlx::query_as(
            r#"
SELECT mip.person_id, mi.id
FROM media_item_people mip
INNER JOIN media_items mi ON mi.id = mip.media_item_id
WHERE mip.person_id = ANY($1)
  AND mi.version_rank = 0
  AND mi.item_type = ANY($2)
ORDER BY mip.person_id, mip.sort_order ASC
            "#,
        )
        .bind(person_ids)
        .bind(&types)
        .fetch_all(&self.pool)
        .await?;

        Ok(group_person_media_rows(&rows, person_ids, exclude_ids, limit_per_person))
    }

    /// Returns (all_ordered_ids, media_ids, person_ids) preserving Meilisearch ranking.
    async fn search_ids_from_meili(
        &self,
        search_term: &str,
        options: &ItemsQuery,
    ) -> anyhow::Result<(Vec<Uuid>, Vec<Uuid>, Vec<Uuid>)> {
        let Some(search_backend) = self.search_backend.as_ref() else {
            return Err(anyhow::Error::new(InfraError::SearchUnavailable)
                .context("meilisearch backend not initialized"));
        };

        let mut query = search_backend.index.search();
        query
            .with_query(search_term)
            .with_limit(MEILI_MAX_HITS)
            .with_attributes_to_retrieve(meilisearch_sdk::search::Selectors::Some(&["id", "item_type"]));

        let meili_filter = build_meili_filter(options);
        if let Some(filter) = meili_filter.as_deref() {
            query.with_filter(filter);
        }

        let results = query.execute::<SearchIndexHit>().await.map_err(|err| {
            warn!(error = %err, %search_term, "meilisearch query failed");
            anyhow::Error::new(InfraError::SearchUnavailable).context("meilisearch query failed")
        })?;

        let mut all_ids = Vec::with_capacity(results.hits.len());
        let mut media_ids = Vec::new();
        let mut person_ids = Vec::new();
        for hit in results.hits {
            let Ok(id) = Uuid::parse_str(&hit.result.id) else {
                continue;
            };
            all_ids.push(id);
            if hit.result.item_type.as_deref() == Some("Person") {
                person_ids.push(id);
            } else {
                media_ids.push(id);
            }
        }

        Ok((all_ids, media_ids, person_ids))
    }

}

/// Pure grouping logic: group `(person_id, media_id)` rows by person, exclude IDs,
/// truncate per person, and return in `person_ids` input order.
fn group_person_media_rows(
    rows: &[(Uuid, Uuid)],
    person_ids: &[Uuid],
    exclude_ids: &HashSet<Uuid>,
    limit_per_person: i64,
) -> Vec<(Uuid, Vec<Uuid>)> {
    let mut grouped: std::collections::HashMap<Uuid, Vec<Uuid>> =
        std::collections::HashMap::new();
    for &(pid, mid) in rows {
        if !exclude_ids.contains(&mid) {
            let entry = grouped.entry(pid).or_default();
            if (entry.len() as i64) < limit_per_person {
                entry.push(mid);
            }
        }
    }
    person_ids
        .iter()
        .map(|pid| (*pid, grouped.remove(pid).unwrap_or_default()))
        .collect()
}

/// Expand an ordered ID list by inserting associated media IDs after each person ID.
/// Deduplicates: each ID appears at most once in the output.
fn expand_ids_with_person_media(
    all_ids: &[Uuid],
    person_assoc: &[(Uuid, Vec<Uuid>)],
) -> Vec<Uuid> {
    let assoc_map: std::collections::HashMap<Uuid, &Vec<Uuid>> =
        person_assoc.iter().map(|(pid, ids)| (*pid, ids)).collect();
    let mut seen = HashSet::new();
    let mut expanded = Vec::with_capacity(all_ids.len());
    for id in all_ids {
        if seen.insert(*id) {
            expanded.push(*id);
        }
        if let Some(assoc_ids) = assoc_map.get(id) {
            for aid in *assoc_ids {
                if seen.insert(*aid) {
                    expanded.push(*aid);
                }
            }
        }
    }
    expanded
}

fn merge_item_metadata_patch(current: Value, patch: &Value) -> Value {
    match (current, patch) {
        (Value::Object(mut current_obj), Value::Object(patch_obj)) => {
            for (key, patch_value) in patch_obj {
                let next = current_obj
                    .remove(key)
                    .map(|current_value| merge_item_metadata_patch(current_value, patch_value))
                    .unwrap_or_else(|| patch_value.clone());
                current_obj.insert(key.clone(), next);
            }
            Value::Object(current_obj)
        }
        (_, patch_value) => patch_value.clone(),
    }
}

fn should_delete_linked_strm(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("strm"))
}

async fn remove_linked_strm_files(paths: &[String]) {
    for path in paths {
        match tokio::fs::remove_file(path).await {
            Ok(_) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => {
                warn!(error = %err, path, "failed to delete linked strm file");
            }
        }
    }
}

fn normalize_emby_collection_type(raw: &str) -> String {
    let value = raw.trim();
    if value.eq_ignore_ascii_case("movie") || value.eq_ignore_ascii_case("movies") {
        return "movies".to_string();
    }
    if value.eq_ignore_ascii_case("series")
        || value.eq_ignore_ascii_case("show")
        || value.eq_ignore_ascii_case("shows")
        || value.eq_ignore_ascii_case("tv")
        || value.eq_ignore_ascii_case("tvshows")
    {
        return "tvshows".to_string();
    }
    if value.eq_ignore_ascii_case("playlist") || value.eq_ignore_ascii_case("playlists") {
        return "playlists".to_string();
    }
    if value.eq_ignore_ascii_case("mixed") {
        return "mixed".to_string();
    }
    value.to_ascii_lowercase()
}

fn root_library_primary_image_tag(root_path: &str) -> Option<String> {
    if root_path.trim().is_empty() {
        return None;
    }
    let path = Path::new(root_path);
    let source = [
        "folder.jpg",
        "folder.png",
        "poster.jpg",
        "poster.png",
        "cover.jpg",
        "cover.png",
        "thumb.jpg",
        "thumb.png",
    ]
    .iter()
    .map(|name| path.join(name))
    .find(|candidate| candidate.exists())?;

    let metadata = std::fs::metadata(&source).ok();
    let size = metadata.as_ref().map(|v| v.len()).unwrap_or(0);
    let modified = metadata
        .and_then(|v| v.modified().ok())
        .and_then(|v| v.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|v| v.as_secs())
        .unwrap_or(0);
    let fingerprint = format!("{}:{size}:{modified}", source.display());
    Some(format!("{:x}", md5_compute(fingerprint.as_bytes())))
}
