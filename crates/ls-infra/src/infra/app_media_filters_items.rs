#[derive(Debug, Clone, Copy)]
enum ParentScopeSqlBind {
    Uuid(Uuid),
    I32(i32),
}

#[derive(Debug, Clone)]
struct ParentScopeSqlClause {
    condition: String,
    binds: Vec<ParentScopeSqlBind>,
}

impl AppInfra {
    async fn enrich_item_counts(&self, item_id: Uuid, item: &mut BaseItemDto) -> anyhow::Result<()> {
        let to_i32 = |value: i64| i32::try_from(value).unwrap_or(i32::MAX);

        if item.item_type.eq_ignore_ascii_case("Series") {
            let mut season_count: i64 = sqlx::query_scalar(
                r#"
SELECT COUNT(*)
FROM media_items
WHERE item_type = 'Season'
  AND series_id = $1
  AND version_rank = 0
                "#,
            )
            .bind(item_id)
            .fetch_one(&self.pool)
            .await?;

            if season_count == 0 {
                season_count = sqlx::query_scalar(
                    r#"
SELECT COUNT(DISTINCT season_number)
FROM media_items
WHERE item_type = 'Episode'
  AND series_id = $1
  AND season_number IS NOT NULL
  AND version_rank = 0
                    "#,
                )
                .bind(item_id)
                .fetch_one(&self.pool)
                .await?;
            }

            let recursive_count: i64 = sqlx::query_scalar(
                r#"
SELECT COUNT(*)
FROM media_items
WHERE series_id = $1
  AND item_type IN ('Season', 'Episode')
  AND version_rank = 0
                "#,
            )
            .bind(item_id)
            .fetch_one(&self.pool)
            .await?;

            item.child_count = Some(to_i32(season_count));
            item.recursive_item_count = Some(to_i32(recursive_count));
            return Ok(());
        }

        if item.item_type.eq_ignore_ascii_case("Season")
            && let (Some(raw_series_id), Some(season_number)) =
                (item.series_id.as_deref(), item.index_number)
            && let Ok(series_id) = Uuid::parse_str(raw_series_id)
        {
            let episode_count: i64 = sqlx::query_scalar(
                r#"
SELECT COUNT(*)
FROM media_items
WHERE item_type = 'Episode'
  AND series_id = $1
  AND season_number = $2
  AND version_rank = 0
                "#,
            )
            .bind(series_id)
            .bind(season_number)
            .fetch_one(&self.pool)
            .await?;
            let episode_count = to_i32(episode_count);
            item.child_count = Some(episode_count);
            item.recursive_item_count = Some(episode_count);
        }

        Ok(())
    }

    async fn load_item_people_map_from_relations(
        &self,
        item_ids: &[Uuid],
    ) -> anyhow::Result<std::collections::HashMap<Uuid, Vec<ls_domain::jellyfin::BaseItemPersonDto>>>
    {
        if item_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        let rows = sqlx::query_as::<
            _,
            (
                Uuid,
                Uuid,
                String,
                Option<String>,
                Option<String>,
                Option<i64>,
                String,
                Option<String>,
            ),
        >(
            r#"
SELECT mip.media_item_id, p.id, p.name, p.primary_image_tag, p.image_path, p.tmdb_id, mip.person_type, mip.role
FROM media_item_people mip
INNER JOIN people p ON p.id = mip.person_id
WHERE mip.media_item_id = ANY($1)
ORDER BY mip.media_item_id ASC, mip.sort_order ASC, p.name ASC
            "#,
        )
        .bind(item_ids)
        .fetch_all(&self.pool)
        .await?;

        if rows.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        let person_image_cache_dir = self.config_snapshot().tmdb.person_image_cache_dir;
        let mut grouped =
            std::collections::HashMap::<Uuid, Vec<ls_domain::jellyfin::BaseItemPersonDto>>::new();
        for (media_item_id, id, name, primary_image_tag, image_path, tmdb_id, person_type, role) in rows {
            grouped
                .entry(media_item_id)
                .or_default()
                .push(ls_domain::jellyfin::BaseItemPersonDto {
                    name,
                    id: Some(id.to_string()),
                    role,
                    person_type: Some(person_type),
                    primary_image_tag: person_primary_image_tag_for_response(
                        id,
                        primary_image_tag,
                        image_path,
                        tmdb_id,
                        &person_image_cache_dir,
                    ),
                });
        }

        Ok(grouped)
    }

    async fn load_item_people_from_relations(
        &self,
        item_id: Uuid,
    ) -> anyhow::Result<Vec<ls_domain::jellyfin::BaseItemPersonDto>> {
        let map = self.load_item_people_map_from_relations(&[item_id]).await?;
        Ok(map.get(&item_id).cloned().unwrap_or_default())
    }

    async fn hydrate_items_people_from_relations(
        &self,
        items: &mut [BaseItemDto],
    ) -> anyhow::Result<()> {
        let item_ids = items
            .iter()
            .filter_map(|item| Uuid::parse_str(&item.id).ok())
            .collect::<Vec<_>>();
        if item_ids.is_empty() {
            return Ok(());
        }

        let mut related_people = self.load_item_people_map_from_relations(&item_ids).await?;
        if related_people.is_empty() {
            return Ok(());
        }

        apply_item_people_overrides(items, &mut related_people);
        Ok(())
    }

    pub async fn search_hints(
        &self,
        search_term: &str,
        limit: i64,
        include_item_types: &[String],
        parent_id: Option<Uuid>,
    ) -> anyhow::Result<SearchHintResult> {
        let Some(search_backend) = self.search_backend.as_ref() else {
            return Err(anyhow::Error::new(InfraError::SearchUnavailable)
                .context("meilisearch backend not initialized"));
        };

        let limit = limit.clamp(1, 500) as usize;

        // Build filter for item types if specified (must be created before query)
        let filter = if !include_item_types.is_empty() {
            let type_filters: Vec<String> = include_item_types
                .iter()
                .map(|t| format!("item_type = \"{}\"", t))
                .collect();
            Some(type_filters.join(" OR "))
        } else {
            None
        };

        let mut query = search_backend.index.search();
        query
            .with_query(search_term)
            .with_limit(limit)
            .with_attributes_to_retrieve(meilisearch_sdk::search::Selectors::Some(&["id", "item_type"]));

        if let Some(ref f) = filter {
            query.with_filter(f);
        }

        let results = query.execute::<SearchIndexHit>().await.map_err(|err| {
            warn!(error = %err, %search_term, "meilisearch search hints query failed");
            anyhow::Error::new(InfraError::SearchUnavailable).context("meilisearch query failed")
        })?;

        let mut media_ids = Vec::new();
        let mut person_ids = Vec::new();
        for hit in &results.hits {
            if let Ok(id) = Uuid::parse_str(&hit.result.id) {
                if hit.result.item_type.as_deref() == Some("Person") {
                    person_ids.push(id);
                } else {
                    media_ids.push(id);
                }
            }
        }
        let mut all_ids: Vec<Uuid> = results
            .hits
            .iter()
            .filter_map(|hit| Uuid::parse_str(&hit.result.id).ok())
            .collect();

        if all_ids.is_empty() {
            return Ok(SearchHintResult {
                search_hints: vec![],
                total_record_count: 0,
            });
        }

        // Fetch media item details
        let mut row_map: std::collections::HashMap<Uuid, SearchHintRow> =
            std::collections::HashMap::new();
        if !media_ids.is_empty() {
            let placeholders: Vec<String> =
                (1..=media_ids.len()).map(|i| format!("${}", i)).collect();
            let mut query_str = format!(
                "SELECT id, item_type, name, metadata FROM media_items WHERE id IN ({})",
                placeholders.join(", ")
            );
            if parent_id.is_some() {
                let parent_bind_idx = media_ids.len() + 1;
                query_str.push_str(&format!(
                    " AND (library_id = ${0} OR series_id = ${0} OR id = ${0})",
                    parent_bind_idx
                ));
            }
            let mut qb = sqlx::query_as::<_, SearchHintRow>(&query_str);
            for id in &media_ids {
                qb = qb.bind(id);
            }
            if let Some(pid) = parent_id {
                qb = qb.bind(pid);
            }
            let rows = qb.fetch_all(&self.pool).await?;
            row_map.extend(rows.into_iter().map(|r| (r.id, r)));
        }

        // ParentId filtering is only feasible for media items; keep people only without ParentId.
        if !person_ids.is_empty() && parent_id.is_none() {
            let placeholders: Vec<String> =
                (1..=person_ids.len()).map(|i| format!("${}", i)).collect();
            let query_str = format!(
                "SELECT id, 'Person'::text AS item_type, name, metadata FROM people WHERE id IN ({})",
                placeholders.join(", ")
            );
            let mut qb = sqlx::query_as::<_, SearchHintRow>(&query_str);
            for id in &person_ids {
                qb = qb.bind(id);
            }
            let rows = qb.fetch_all(&self.pool).await?;
            row_map.extend(rows.into_iter().map(|r| (r.id, r)));

            // Fetch associated media for each person and inject into results
            let existing_ids: HashSet<Uuid> = media_ids.iter().copied().collect();
            let person_assoc = self
                .fetch_media_ids_for_persons(
                    &person_ids,
                    &existing_ids,
                    include_item_types,
                    PERSON_ASSOCIATED_MEDIA_LIMIT,
                )
                .await?;

            let all_assoc_ids: Vec<Uuid> = person_assoc
                .iter()
                .flat_map(|(_, ids)| ids.iter().copied())
                .collect();
            if !all_assoc_ids.is_empty() {
                let assoc_placeholders: Vec<String> =
                    (1..=all_assoc_ids.len()).map(|i| format!("${}", i)).collect();
                let assoc_query = format!(
                    "SELECT id, item_type, name, metadata FROM media_items WHERE id IN ({}) AND version_rank = 0",
                    assoc_placeholders.join(", ")
                );
                let mut aqb = sqlx::query_as::<_, SearchHintRow>(&assoc_query);
                for id in &all_assoc_ids {
                    aqb = aqb.bind(id);
                }
                let assoc_rows = aqb.fetch_all(&self.pool).await?;
                row_map.extend(assoc_rows.into_iter().map(|r| (r.id, r)));
            }

            // Rebuild all_ids: insert associated media IDs after each person
            all_ids = expand_ids_with_person_media(&all_ids, &person_assoc);
        }

        let hints: Vec<SearchHint> = all_ids
            .iter()
            .filter_map(|id| row_map.get(id))
            .map(|row| {
                let meta = row.metadata.as_ref();
                let production_year = meta
                    .and_then(|m| m.get("production_year"))
                    .and_then(|v| v.as_i64())
                    .map(|v| v as i32);
                let primary_image_tag = meta
                    .and_then(|m| m.get("primary_image_tag"))
                    .and_then(|v| v.as_str())
                    .map(String::from);

                SearchHint {
                    id: row.id.to_string(),
                    name: row.name.clone(),
                    item_type: row.item_type.clone(),
                    production_year,
                    primary_image_tag,
                    thumb_image_tag: None,
                }
            })
            .collect();

        let total = hints.len() as i32;
        Ok(SearchHintResult {
            search_hints: hints,
            total_record_count: total,
        })
    }

    fn parent_scope_sql_clause(
        parent_scope: ParentQueryScope,
        first_bind_idx: usize,
    ) -> ParentScopeSqlClause {
        match parent_scope {
            ParentQueryScope::Library {
                parent_id,
                recursive,
            } => {
                let mut condition = format!("library_id = ${first_bind_idx}");
                if !recursive {
                    condition.push_str(" AND item_type != 'Season' AND item_type != 'Episode'");
                }
                ParentScopeSqlClause {
                    condition,
                    binds: vec![ParentScopeSqlBind::Uuid(parent_id)],
                }
            }
            ParentQueryScope::Series {
                series_id,
                recursive,
            } => ParentScopeSqlClause {
                condition: if recursive {
                    format!(
                        "series_id = ${first_bind_idx} AND item_type IN ('Season', 'Episode')"
                    )
                } else {
                    format!("series_id = ${first_bind_idx} AND item_type = 'Season'")
                },
                binds: vec![ParentScopeSqlBind::Uuid(series_id)],
            },
            ParentQueryScope::Season {
                series_id,
                season_number,
            } => ParentScopeSqlClause {
                condition: format!(
                    "item_type = 'Episode' AND series_id = ${first_bind_idx} AND season_number = ${}",
                    first_bind_idx + 1
                ),
                binds: vec![
                    ParentScopeSqlBind::Uuid(series_id),
                    ParentScopeSqlBind::I32(season_number),
                ],
            },
            ParentQueryScope::Fallback {
                parent_id,
                recursive,
            } => {
                let _ = recursive;
                ParentScopeSqlClause {
                    condition: format!(
                        "(library_id = ${first_bind_idx} OR series_id = ${first_bind_idx})"
                    ),
                    binds: vec![ParentScopeSqlBind::Uuid(parent_id)],
                }
            }
        }
    }

    /// Get available filter values for /Items/Filters endpoint
    pub async fn get_item_filters(
        &self,
        parent_id: Option<Uuid>,
        recursive: bool,
        include_item_types: &[String],
    ) -> anyhow::Result<QueryFilters> {
        // Build WHERE clause
        let mut conditions = vec!["version_rank = 0".to_string()];
        let mut bind_idx = 1;
        let parent_clause = if let Some(parent_id) = parent_id {
            let parent_scope = self.resolve_parent_query_scope(parent_id, recursive).await?;
            let clause = Self::parent_scope_sql_clause(parent_scope, bind_idx);
            bind_idx += clause.binds.len();
            conditions.push(clause.condition.clone());
            Some(clause)
        } else {
            None
        };

        if !include_item_types.is_empty() {
            let placeholders: Vec<String> = include_item_types
                .iter()
                .enumerate()
                .map(|(i, _)| format!("${}", bind_idx + i))
                .collect();
            conditions.push(format!("item_type IN ({})", placeholders.join(", ")));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        // Query for genres (from JSONB array)
        let genres_query = format!(
            r#"
SELECT DISTINCT jsonb_array_elements_text(metadata->'genres') AS genre
FROM media_items
{}
ORDER BY genre
            "#,
            where_clause
        );

        // Query for years
        let years_query = format!(
            r#"
SELECT DISTINCT (metadata->>'production_year')::int AS year
FROM media_items
{}
AND metadata->>'production_year' IS NOT NULL
ORDER BY year DESC
            "#,
            if where_clause.is_empty() {
                "WHERE metadata->>'production_year' IS NOT NULL".to_string()
            } else {
                where_clause.clone()
            }
        );

        // Query for official ratings
        let ratings_query = format!(
            r#"
SELECT DISTINCT metadata->>'official_rating' AS rating
FROM media_items
{}
AND metadata->>'official_rating' IS NOT NULL
ORDER BY rating
            "#,
            if where_clause.is_empty() {
                "WHERE metadata->>'official_rating' IS NOT NULL".to_string()
            } else {
                where_clause.clone()
            }
        );

        // Query for tags (from JSONB array)
        let tags_query = format!(
            r#"
SELECT DISTINCT jsonb_array_elements_text(
    CASE
        WHEN jsonb_typeof(metadata->'tags') = 'array' THEN metadata->'tags'
        ELSE '[]'::jsonb
    END
) AS tag
FROM media_items
{}
ORDER BY tag
            "#,
            where_clause
        );

        // Execute queries
        let mut genres_qb = sqlx::query_scalar::<_, String>(&genres_query);
        let mut years_qb = sqlx::query_scalar::<_, i32>(&years_query);
        let mut ratings_qb = sqlx::query_scalar::<_, String>(&ratings_query);
        let mut tags_qb = sqlx::query_scalar::<_, String>(&tags_query);

        if let Some(parent_clause) = parent_clause.as_ref() {
            for bind in &parent_clause.binds {
                match bind {
                    ParentScopeSqlBind::Uuid(value) => {
                        genres_qb = genres_qb.bind(*value);
                        years_qb = years_qb.bind(*value);
                        ratings_qb = ratings_qb.bind(*value);
                        tags_qb = tags_qb.bind(*value);
                    }
                    ParentScopeSqlBind::I32(value) => {
                        genres_qb = genres_qb.bind(*value);
                        years_qb = years_qb.bind(*value);
                        ratings_qb = ratings_qb.bind(*value);
                        tags_qb = tags_qb.bind(*value);
                    }
                }
            }
        }
        for item_type in include_item_types {
            genres_qb = genres_qb.bind(item_type);
            years_qb = years_qb.bind(item_type);
            ratings_qb = ratings_qb.bind(item_type);
            tags_qb = tags_qb.bind(item_type);
        }

        let genres = genres_qb.fetch_all(&self.pool).await.unwrap_or_default();
        let years = years_qb.fetch_all(&self.pool).await.unwrap_or_default();
        let official_ratings = ratings_qb.fetch_all(&self.pool).await.unwrap_or_default();
        let tags = tags_qb
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .fold(Vec::<String>::new(), |mut acc, value| {
                if !acc
                    .iter()
                    .any(|existing| existing.eq_ignore_ascii_case(&value))
                {
                    acc.push(value);
                }
                acc
            });

        Ok(QueryFilters {
            genres,
            years,
            official_ratings,
            tags,
        })
    }

    /// List genres for /Genres endpoint
    pub async fn list_genres(
        &self,
        parent_id: Option<Uuid>,
        recursive: bool,
        include_item_types: &[String],
        start_index: i64,
        limit: i64,
    ) -> anyhow::Result<QueryResultDto<BaseItemDto>> {
        let limit = limit.clamp(1, 500);
        let start_index = start_index.max(0);

        // Build WHERE clause
        let mut conditions = vec!["version_rank = 0".to_string()];
        let mut bind_idx = 1;
        let parent_clause = if let Some(parent_id) = parent_id {
            let parent_scope = self.resolve_parent_query_scope(parent_id, recursive).await?;
            let clause = Self::parent_scope_sql_clause(parent_scope, bind_idx);
            bind_idx += clause.binds.len();
            conditions.push(clause.condition.clone());
            Some(clause)
        } else {
            None
        };

        if !include_item_types.is_empty() {
            let placeholders: Vec<String> = include_item_types
                .iter()
                .enumerate()
                .map(|(i, _)| format!("${}", bind_idx + i))
                .collect();
            conditions.push(format!("item_type IN ({})", placeholders.join(", ")));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };
        let limit_bind_idx = bind_idx + include_item_types.len();
        let offset_bind_idx = limit_bind_idx + 1;

        let query = format!(
            r#"
SELECT genre, COUNT(*) as item_count
FROM (
    SELECT jsonb_array_elements_text(metadata->'genres') AS genre
    FROM media_items
    {}
) sub
GROUP BY genre
ORDER BY genre
LIMIT ${}
OFFSET ${}
            "#,
            where_clause, limit_bind_idx, offset_bind_idx
        );
        let count_query = format!(
            r#"
SELECT COUNT(*)::BIGINT
FROM (
    SELECT jsonb_array_elements_text(metadata->'genres') AS genre
    FROM media_items
    {}
    GROUP BY genre
) counted
            "#,
            where_clause
        );

        let mut qb = sqlx::query_as::<_, GenreRow>(&query);
        let mut count_qb = sqlx::query_scalar::<_, i64>(&count_query);
        if let Some(parent_clause) = parent_clause.as_ref() {
            for bind in &parent_clause.binds {
                match bind {
                    ParentScopeSqlBind::Uuid(value) => {
                        qb = qb.bind(*value);
                        count_qb = count_qb.bind(*value);
                    }
                    ParentScopeSqlBind::I32(value) => {
                        qb = qb.bind(*value);
                        count_qb = count_qb.bind(*value);
                    }
                };
            }
        }
        for item_type in include_item_types {
            qb = qb.bind(item_type);
            count_qb = count_qb.bind(item_type);
        }
        qb = qb.bind(limit).bind(start_index);

        let total = count_qb.fetch_one(&self.pool).await? as i32;
        let all_genres = qb.fetch_all(&self.pool).await?;

        let items: Vec<BaseItemDto> = all_genres
            .into_iter()
            .map(|row| BaseItemDto {
                id: genre_to_id(&row.genre),
                name: row.genre,
                item_type: "Genre".to_string(),
                path: String::new(),
                is_folder: Some(false),
                media_type: None,
                container: None,
                location_type: None,
                can_delete: Some(false),
                can_download: Some(false),
                collection_type: None,
                runtime_ticks: None,
                bitrate: None,
                media_sources: None,
                user_data: None,
                overview: None,
                premiere_date: None,
                end_date: None,
                production_year: None,
                genres: None,
                tags: None,
                provider_ids: None,
                image_tags: None,
                primary_image_tag: None,
                parent_id: None,
                series_id: None,
                series_name: None,
                season_id: None,
                season_name: None,
                index_number: None,
                parent_index_number: None,
                backdrop_image_tags: None,
                official_rating: None,
                community_rating: None,
                studios: None,
                people: None,
                sort_name: None,
                primary_image_aspect_ratio: None,
                date_created: None,
                child_count: Some(row.item_count as i32),
                recursive_item_count: None,
                play_access: None,
            })
            .collect();

        Ok(QueryResultDto {
            items,
            total_record_count: total,
            start_index: start_index as i32,
        })
    }

    /// List studio names for /Studios endpoint.
    pub async fn list_studio_names(
        &self,
        parent_id: Option<Uuid>,
        recursive: bool,
        include_item_types: &[String],
        start_index: i64,
        limit: i64,
    ) -> anyhow::Result<(Vec<String>, i32)> {
        let limit = limit.clamp(1, 500);
        let start_index = start_index.max(0);

        let mut conditions = vec!["version_rank = 0".to_string()];
        let mut bind_idx = 1;
        let parent_clause = if let Some(parent_id) = parent_id {
            let parent_scope = self.resolve_parent_query_scope(parent_id, recursive).await?;
            let clause = Self::parent_scope_sql_clause(parent_scope, bind_idx);
            bind_idx += clause.binds.len();
            conditions.push(clause.condition.clone());
            Some(clause)
        } else {
            None
        };
        if !include_item_types.is_empty() {
            let placeholders: Vec<String> = include_item_types
                .iter()
                .enumerate()
                .map(|(i, _)| format!("${}", bind_idx + i))
                .collect();
            conditions.push(format!("item_type IN ({})", placeholders.join(", ")));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };
        let limit_bind_idx = bind_idx + include_item_types.len();
        let offset_bind_idx = limit_bind_idx + 1;

        let query = format!(
            r#"
SELECT studio, COUNT(*) as item_count
FROM (
    SELECT
        CASE
            WHEN jsonb_typeof(studio_entry) = 'string'
                THEN trim(both '"' from studio_entry::text)
            WHEN jsonb_typeof(studio_entry) = 'object'
                THEN COALESCE(studio_entry->>'name', studio_entry->>'Name')
            ELSE NULL
        END AS studio
    FROM media_items
    CROSS JOIN LATERAL jsonb_array_elements(COALESCE(metadata->'studios', '[]'::jsonb)) AS studio_entry
    {}
) sub
WHERE studio IS NOT NULL AND studio <> ''
GROUP BY studio
ORDER BY studio
LIMIT ${}
OFFSET ${}
            "#,
            where_clause, limit_bind_idx, offset_bind_idx
        );
        let count_query = format!(
            r#"
SELECT COUNT(*)::BIGINT
FROM (
    SELECT 1
    FROM (
        SELECT
            CASE
                WHEN jsonb_typeof(studio_entry) = 'string'
                    THEN trim(both '"' from studio_entry::text)
                WHEN jsonb_typeof(studio_entry) = 'object'
                    THEN COALESCE(studio_entry->>'name', studio_entry->>'Name')
                ELSE NULL
            END AS studio
        FROM media_items
        CROSS JOIN LATERAL jsonb_array_elements(COALESCE(metadata->'studios', '[]'::jsonb)) AS studio_entry
        {}
    ) sub
    WHERE studio IS NOT NULL AND studio <> ''
    GROUP BY studio
) counted
            "#,
            where_clause
        );

        let mut qb = sqlx::query_as::<_, (String, i64)>(&query);
        let mut count_qb = sqlx::query_scalar::<_, i64>(&count_query);
        if let Some(parent_clause) = parent_clause.as_ref() {
            for bind in &parent_clause.binds {
                match bind {
                    ParentScopeSqlBind::Uuid(value) => {
                        qb = qb.bind(*value);
                        count_qb = count_qb.bind(*value);
                    }
                    ParentScopeSqlBind::I32(value) => {
                        qb = qb.bind(*value);
                        count_qb = count_qb.bind(*value);
                    }
                };
            }
        }
        for item_type in include_item_types {
            qb = qb.bind(item_type);
            count_qb = count_qb.bind(item_type);
        }
        qb = qb.bind(limit).bind(start_index);

        let total = count_qb.fetch_one(&self.pool).await? as i32;
        let all_studios = qb.fetch_all(&self.pool).await?;
        let names = all_studios
            .into_iter()
            .map(|(studio, _)| studio)
            .collect::<Vec<_>>();

        Ok((names, total))
    }

    pub async fn get_item(
        &self,
        user_id: Option<Uuid>,
        item_id: Uuid,
    ) -> anyhow::Result<Option<BaseItemDto>> {
        let row = sqlx::query_as::<_, MediaItemRow>(
            r#"
SELECT id, item_type, name, path, runtime_ticks, bitrate, series_id, season_number, episode_number, library_id, metadata, created_at
FROM media_items
WHERE id = $1
LIMIT 1
            "#,
        )
        .bind(item_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let user_data = if let Some(uid) = user_id {
            self.user_data(uid, item_id).await?
        } else {
            None
        };

        let mut item = item_row_to_dto(row, user_data);
        let related_people = self.load_item_people_from_relations(item_id).await?;
        if !related_people.is_empty() {
            // Prefer canonical relation rows over embedded metadata people to avoid stale image tags.
            item.people = Some(related_people);
        }
        self.enrich_item_counts(item_id, &mut item).await?;
        let mut single = vec![item];
        self.attach_grouped_media_sources(&mut single).await?;
        if let Some(media_sources) = single.first_mut().and_then(|item| item.media_sources.as_mut())
        {
            self.attach_external_subtitles_to_media_sources(media_sources)
                .await?;
        }
        let item = single
            .into_iter()
            .next()
            .expect("single item payload should always exist");

        Ok(Some(item))
    }

    pub async fn item_counts(
        &self,
        user_id: Option<Uuid>,
        is_favorite: Option<bool>,
    ) -> anyhow::Result<ItemCountsDto> {
        let rows = match (user_id, is_favorite) {
            (Some(user_id), Some(is_favorite)) => {
                sqlx::query_as::<_, CountRow>(
                    r#"
SELECT m.item_type, count(*) AS count
FROM media_items m
LEFT JOIN watch_states w ON w.media_item_id = m.id AND w.user_id = $1
WHERE COALESCE(w.is_favorite, FALSE) = $2
  AND m.version_rank = 0
GROUP BY m.item_type
                    "#,
                )
                .bind(user_id)
                .bind(is_favorite)
                .fetch_all(&self.pool)
                .await?
            }
            _ => {
                sqlx::query_as::<_, CountRow>(
                    r#"
SELECT item_type, count(*) AS count
FROM media_items
WHERE version_rank = 0
GROUP BY item_type
                    "#,
                )
                .fetch_all(&self.pool)
                .await?
            }
        };

        let mut dto = ItemCountsDto {
            movie_count: 0,
            series_count: 0,
            episode_count: 0,
            song_count: 0,
            album_count: 0,
            artist_count: 0,
            music_video_count: 0,
            box_set_count: 0,
            book_count: 0,
            game_count: 0,
            game_system_count: 0,
            item_count: 0,
            program_count: 0,
            trailer_count: 0,
        };

        for row in rows {
            dto.item_count += row.count;
            match row.item_type.as_str() {
                "Movie" => dto.movie_count = row.count,
                "Series" => dto.series_count = row.count,
                "Episode" => dto.episode_count = row.count,
                "Song" => dto.song_count = row.count,
                "MusicAlbum" => dto.album_count = row.count,
                "MusicArtist" => dto.artist_count = row.count,
                "MusicVideo" => dto.music_video_count = row.count,
                "BoxSet" => dto.box_set_count = row.count,
                "Book" => dto.book_count = row.count,
                "Game" => dto.game_count = row.count,
                "GameSystem" => dto.game_system_count = row.count,
                "Program" => dto.program_count = row.count,
                "Trailer" => dto.trailer_count = row.count,
                _ => {}
            }
        }

        Ok(dto)
    }

}

fn apply_item_people_overrides(
    items: &mut [BaseItemDto],
    related_people: &mut std::collections::HashMap<Uuid, Vec<ls_domain::jellyfin::BaseItemPersonDto>>,
) {
    for item in items {
        let Ok(item_id) = Uuid::parse_str(&item.id) else {
            continue;
        };
        if let Some(people) = related_people.remove(&item_id) {
            // Prefer canonical relation rows over embedded metadata people to avoid stale image tags.
            item.people = Some(people);
        }
    }
}

fn person_primary_image_tag_for_response(
    person_id: Uuid,
    primary_image_tag: Option<String>,
    image_path: Option<String>,
    tmdb_id: Option<i64>,
    person_image_cache_dir: &str,
) -> Option<String> {
    let primary_image_tag = primary_image_tag
        .map(|raw| raw.trim().to_string())
        .filter(|raw| !raw.is_empty());

    let has_local_image = image_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(Path::new)
        .is_some_and(Path::exists);

    let has_cached_image = tmdb_id
        .map(|id| person_image_cache_path(person_image_cache_dir, id).exists())
        .unwrap_or(false);

    if let Some(tag) = primary_image_tag {
        return Some(tag);
    }

    let suffix = if has_local_image || has_cached_image {
        "image"
    } else {
        "placeholder"
    };
    Some(auth::hash_api_key(&format!(
        "person:{person_id}:{suffix}"
    )))
}
