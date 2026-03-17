const LIBRARY_SELECT_BASE_SQL: &str = r#"
SELECT
    l.id,
    l.name,
    l.library_type,
    l.enabled,
    l.scan_interval_hours,
    l.scraper_policy,
    l.created_at,
    COALESCE(
        ARRAY_AGG(lp.path ORDER BY lp.sort_order) FILTER (WHERE lp.path IS NOT NULL),
        ARRAY[]::TEXT[]
    ) AS paths
FROM libraries l
LEFT JOIN library_paths lp ON lp.library_id = l.id
"#;

fn trim_trailing_separators(raw: &str) -> String {
    let mut value = raw.trim().to_string();
    while value.len() > 1 && (value.ends_with('/') || value.ends_with('\\')) {
        value.pop();
    }
    value
}

fn normalize_library_path(raw: &str) -> Option<String> {
    let value = trim_trailing_separators(raw);
    if value.is_empty() {
        return None;
    }

    let candidate = Path::new(&value);
    let absolute = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        std::env::current_dir().ok()?.join(candidate)
    };
    let canonical = std::fs::canonicalize(&absolute).unwrap_or(absolute);
    let normalized = trim_trailing_separators(&canonical.to_string_lossy());
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn normalize_library_path_key(path: &str) -> String {
    path.to_lowercase()
}

fn normalize_library_paths(inputs: &[String]) -> Vec<String> {
    let mut dedup = HashSet::new();
    let mut out = Vec::new();
    for input in inputs {
        let Some(normalized) = normalize_library_path(input) else {
            continue;
        };
        let key = normalize_library_path_key(&normalized);
        if dedup.insert(key) {
            out.push(normalized);
        }
    }
    out
}

async fn replace_library_paths_tx(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    library_id: Uuid,
    paths: &[String],
) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM library_paths WHERE library_id = $1")
        .bind(library_id)
        .execute(tx.as_mut())
        .await?;

    for (idx, path) in paths.iter().enumerate() {
        sqlx::query(
            r#"
INSERT INTO library_paths (id, library_id, path, normalized_path, sort_order)
VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(library_id)
        .bind(path)
        .bind(normalize_library_path_key(path))
        .bind(i32::try_from(idx).unwrap_or(i32::MAX))
        .execute(tx.as_mut())
        .await?;
    }

    Ok(())
}

impl AppInfra {
    pub async fn create_library(
        &self,
        name: &str,
        paths: &[String],
        library_type: &str,
    ) -> anyhow::Result<Library> {
        let id = Uuid::now_v7();
        let normalized_paths = normalize_library_paths(paths);
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            r#"
INSERT INTO libraries (id, name, library_type, enabled)
VALUES ($1, $2, $3, true)
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(library_type)
        .execute(tx.as_mut())
        .await?;

        replace_library_paths_tx(&mut tx, id, &normalized_paths).await?;
        tx.commit().await?;

        self.get_library_by_id(id)
            .await?
            .context("created library missing after insert")
    }

    pub async fn get_library_by_id(&self, library_id: Uuid) -> anyhow::Result<Option<Library>> {
        let row = sqlx::query_as::<_, LibraryRow>(
            &format!(
                r#"
{}
WHERE l.id = $1
GROUP BY l.id, l.name, l.library_type, l.enabled, l.scan_interval_hours, l.scraper_policy, l.created_at
LIMIT 1
                "#,
                LIBRARY_SELECT_BASE_SQL
            ),
        )
        .bind(library_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn get_library_by_name(&self, library_name: &str) -> anyhow::Result<Option<Library>> {
        let row = sqlx::query_as::<_, LibraryRow>(
            &format!(
                r#"
{}
WHERE lower(l.name) = lower($1)
GROUP BY l.id, l.name, l.library_type, l.enabled, l.scan_interval_hours, l.scraper_policy, l.created_at
LIMIT 1
                "#,
                LIBRARY_SELECT_BASE_SQL
            ),
        )
        .bind(library_name)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn list_libraries(&self) -> anyhow::Result<Vec<Library>> {
        let rows = sqlx::query_as::<_, LibraryRow>(
            &format!(
                r#"
{}
GROUP BY l.id, l.name, l.library_type, l.enabled, l.scan_interval_hours, l.scraper_policy, l.created_at
ORDER BY l.created_at DESC
                "#,
                LIBRARY_SELECT_BASE_SQL
            ),
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn set_library_enabled(
        &self,
        library_id: Uuid,
        enabled: bool,
    ) -> anyhow::Result<Option<Library>> {
        let updated_id = sqlx::query_scalar::<_, Uuid>(
            r#"
UPDATE libraries
SET enabled = $2
WHERE id = $1
RETURNING id
            "#,
        )
        .bind(library_id)
        .bind(enabled)
        .fetch_optional(&self.pool)
        .await?;

        if updated_id.is_none() {
            return Ok(None);
        }

        self.get_library_by_id(library_id).await
    }

    pub async fn update_library_name(
        &self,
        library_id: Uuid,
        name: &str,
    ) -> anyhow::Result<Option<Library>> {
        let updated_id = sqlx::query_scalar::<_, Uuid>(
            r#"
UPDATE libraries
SET name = $2
WHERE id = $1
RETURNING id
            "#,
        )
        .bind(library_id)
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        if updated_id.is_none() {
            return Ok(None);
        }

        self.get_library_by_id(library_id).await
    }

    pub async fn update_library_scraper_policy(
        &self,
        library_id: Uuid,
        scraper_policy: &Value,
    ) -> anyhow::Result<Option<Library>> {
        let updated_id = sqlx::query_scalar::<_, Uuid>(
            r#"
UPDATE libraries
SET scraper_policy = $2
WHERE id = $1
RETURNING id
            "#,
        )
        .bind(library_id)
        .bind(scraper_policy)
        .fetch_optional(&self.pool)
        .await?;

        if updated_id.is_none() {
            return Ok(None);
        }

        self.get_library_by_id(library_id).await
    }

    pub async fn add_library_path(
        &self,
        library_id: Uuid,
        path: &str,
    ) -> anyhow::Result<Option<Library>> {
        let Some(normalized_path) = normalize_library_path(path) else {
            anyhow::bail!("path is required");
        };

        let mut tx = self.pool.begin().await?;
        let exists: Option<Uuid> = sqlx::query_scalar("SELECT id FROM libraries WHERE id = $1 LIMIT 1")
            .bind(library_id)
            .fetch_optional(tx.as_mut())
            .await?;
        if exists.is_none() {
            tx.rollback().await?;
            return Ok(None);
        }

        let sort_order = sqlx::query_scalar::<_, i32>(
            r#"
SELECT COALESCE(MAX(sort_order), -1) + 1
FROM library_paths
WHERE library_id = $1
            "#,
        )
        .bind(library_id)
        .fetch_one(tx.as_mut())
        .await?;

        sqlx::query(
            r#"
INSERT INTO library_paths (id, library_id, path, normalized_path, sort_order)
VALUES ($1, $2, $3, $4, $5)
ON CONFLICT (library_id, normalized_path) DO NOTHING
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(library_id)
        .bind(&normalized_path)
        .bind(normalize_library_path_key(&normalized_path))
        .bind(sort_order.max(0))
        .execute(tx.as_mut())
        .await?;

        tx.commit().await?;
        self.get_library_by_id(library_id).await
    }

    pub async fn remove_library_path(
        &self,
        library_id: Uuid,
        path: &str,
    ) -> anyhow::Result<Option<bool>> {
        let Some(normalized_path) = normalize_library_path(path) else {
            anyhow::bail!("path is required");
        };

        let mut tx = self.pool.begin().await?;
        let exists: Option<Uuid> = sqlx::query_scalar("SELECT id FROM libraries WHERE id = $1 LIMIT 1")
            .bind(library_id)
            .fetch_optional(tx.as_mut())
            .await?;
        if exists.is_none() {
            tx.rollback().await?;
            return Ok(None);
        }

        let removed = sqlx::query(
            r#"
DELETE FROM library_paths
WHERE library_id = $1
  AND normalized_path = $2
            "#,
        )
        .bind(library_id)
        .bind(normalize_library_path_key(&normalized_path))
        .execute(tx.as_mut())
        .await?
        .rows_affected()
            > 0;

        if removed {
            sqlx::query(
                r#"
WITH ranked AS (
    SELECT
        id,
        (ROW_NUMBER() OVER (ORDER BY sort_order ASC, created_at ASC, id ASC) - 1)::INT AS next_order
    FROM library_paths
    WHERE library_id = $1
)
UPDATE library_paths lp
SET sort_order = ranked.next_order
FROM ranked
WHERE lp.id = ranked.id
                "#,
            )
            .bind(library_id)
            .execute(tx.as_mut())
            .await?;
        }

        tx.commit().await?;
        Ok(Some(removed))
    }

    pub async fn replace_primary_library_path(
        &self,
        library_id: Uuid,
        path: &str,
    ) -> anyhow::Result<Option<Library>> {
        let Some(normalized_primary) = normalize_library_path(path) else {
            anyhow::bail!("path is required");
        };

        let mut library = match self.get_library_by_id(library_id).await? {
            Some(v) => v,
            None => return Ok(None),
        };

        if library.paths.is_empty() {
            library.paths.push(normalized_primary);
        } else {
            library.paths[0] = normalized_primary;
        }

        self.replace_library_paths(library_id, &library.paths).await
    }

    pub async fn replace_library_paths(
        &self,
        library_id: Uuid,
        paths: &[String],
    ) -> anyhow::Result<Option<Library>> {
        let normalized_paths = normalize_library_paths(paths);
        let mut tx = self.pool.begin().await?;
        let exists: Option<Uuid> = sqlx::query_scalar("SELECT id FROM libraries WHERE id = $1 LIMIT 1")
            .bind(library_id)
            .fetch_optional(tx.as_mut())
            .await?;
        if exists.is_none() {
            tx.rollback().await?;
            return Ok(None);
        }

        replace_library_paths_tx(&mut tx, library_id, &normalized_paths).await?;
        tx.commit().await?;
        self.get_library_by_id(library_id).await
    }

    pub async fn get_library_primary_path(
        &self,
        library_id: Uuid,
        enabled_only: bool,
    ) -> anyhow::Result<Option<String>> {
        let path = sqlx::query_scalar::<_, String>(
            r#"
SELECT lp.path
FROM library_paths lp
JOIN libraries l ON l.id = lp.library_id
WHERE l.id = $1
  AND ($2::BOOLEAN = false OR l.enabled = true)
ORDER BY lp.sort_order ASC
LIMIT 1
            "#,
        )
        .bind(library_id)
        .bind(enabled_only)
        .fetch_optional(&self.pool)
        .await?;

        Ok(path)
    }

    pub async fn update_library_type(
        &self,
        library_id: Uuid,
        library_type: &str,
    ) -> anyhow::Result<Option<Library>> {
        let updated_id = sqlx::query_scalar::<_, Uuid>(
            r#"
UPDATE libraries
SET library_type = $2
WHERE id = $1
RETURNING id
            "#,
        )
        .bind(library_id)
        .bind(library_type)
        .fetch_optional(&self.pool)
        .await?;

        if updated_id.is_none() {
            return Ok(None);
        }

        self.get_library_by_id(library_id).await
    }

    pub async fn list_library_item_stats(&self) -> anyhow::Result<Vec<LibraryItemStat>> {
        let rows = sqlx::query_as::<_, LibraryItemStatRow>(
            r#"
SELECT
    l.id AS library_id,
    COUNT(m.id)::BIGINT AS item_count,
    MAX(m.updated_at) AS last_item_updated_at
FROM libraries l
LEFT JOIN media_items m ON m.library_id = l.id
GROUP BY l.id
ORDER BY l.id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn count_libraries_total(&self) -> anyhow::Result<i64> {
        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*)::BIGINT FROM libraries")
            .fetch_one(&self.pool)
            .await?;
        Ok(count)
    }

    pub async fn count_libraries_enabled(&self) -> anyhow::Result<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*)::BIGINT FROM libraries WHERE enabled = true",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(count)
    }

    pub async fn count_media_items_total(&self) -> anyhow::Result<i64> {
        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*)::BIGINT FROM media_items")
            .fetch_one(&self.pool)
            .await?;
        Ok(count)
    }

    pub async fn count_users_total(&self) -> anyhow::Result<i64> {
        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*)::BIGINT FROM users")
            .fetch_one(&self.pool)
            .await?;
        Ok(count)
    }

    pub async fn count_users_disabled(&self) -> anyhow::Result<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*)::BIGINT FROM users WHERE is_disabled = true",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(count)
    }

    pub async fn count_active_playback_sessions(&self) -> anyhow::Result<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*)::BIGINT FROM playback_sessions WHERE is_active = true",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(count)
    }

    pub async fn count_active_auth_sessions(&self) -> anyhow::Result<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*)::BIGINT FROM user_sessions WHERE is_active = true",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(count)
    }

    pub async fn list_storage_configs(
        &self,
        include_secrets: bool,
    ) -> anyhow::Result<Vec<StorageConfigRecord>> {
        let rows = sqlx::query_as::<_, StorageConfigRow>(
            r#"
SELECT id, kind, name, config, enabled, created_at, updated_at
FROM storage_configs
ORDER BY kind ASC, name ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let out = rows
            .into_iter()
            .map(|mut row| {
                if !include_secrets {
                    row.config = mask_secret_fields(row.config);
                }
                row.into()
            })
            .collect::<Vec<_>>();

        Ok(out)
    }

    pub async fn upsert_storage_config(
        &self,
        kind: &str,
        name: &str,
        config: Value,
        enabled: bool,
    ) -> anyhow::Result<StorageConfigRecord> {
        let row = sqlx::query_as::<_, StorageConfigRow>(
            r#"
INSERT INTO storage_configs (id, kind, name, config, enabled)
VALUES ($1, $2, $3, $4, $5)
ON CONFLICT(kind, name) DO UPDATE SET
    config = EXCLUDED.config,
    enabled = EXCLUDED.enabled,
    updated_at = now()
RETURNING id, kind, name, config, enabled, created_at, updated_at
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(kind)
        .bind(name)
        .bind(config)
        .bind(enabled)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }

    pub async fn list_playback_domains(&self) -> anyhow::Result<Vec<PlaybackDomain>> {
        let rows = sqlx::query_as::<_, PlaybackDomainRow>(
            r#"
SELECT
    id,
    name,
    base_url,
    enabled,
    priority,
    is_default,
    lumenbackend_node_id,
    traffic_multiplier,
    created_at,
    updated_at
FROM playback_domains
ORDER BY is_default DESC, priority DESC, updated_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn list_account_permission_groups(
        &self,
        include_disabled: bool,
    ) -> anyhow::Result<Vec<AccountPermissionGroup>> {
        let rows = sqlx::query_as::<_, AccountPermissionGroupRow>(
            r#"
SELECT
    g.id,
    g.code,
    g.name,
    g.enabled,
    COALESCE(
        ARRAY_AGG(m.domain_id ORDER BY m.domain_id) FILTER (WHERE m.domain_id IS NOT NULL),
        ARRAY[]::UUID[]
    ) AS domain_ids,
    g.updated_at
FROM account_permission_groups g
LEFT JOIN account_permission_group_playback_domains m ON m.group_id = g.id
WHERE ($1::BOOLEAN = true OR g.enabled = true)
GROUP BY g.id, g.code, g.name, g.enabled, g.updated_at
ORDER BY g.enabled DESC, g.updated_at DESC, g.code ASC
            "#,
        )
        .bind(include_disabled)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn upsert_account_permission_group(
        &self,
        payload: AccountPermissionGroupUpsert,
    ) -> anyhow::Result<AccountPermissionGroup> {
        let code = payload.code.trim().to_ascii_lowercase();
        let name = payload.name.trim().to_string();
        if code.is_empty() || name.is_empty() {
            anyhow::bail!("permission group code and name are required");
        }

        let domain_ids = normalize_permission_group_domain_ids(payload.domain_ids);
        if domain_ids.is_empty() {
            anyhow::bail!("permission group must include at least one playback domain");
        }

        let known_domain_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(1) FROM playback_domains WHERE id = ANY($1::UUID[])",
        )
        .bind(&domain_ids)
        .fetch_one(&self.pool)
        .await?;
        if known_domain_count != i64::try_from(domain_ids.len()).unwrap_or(i64::MAX) {
            anyhow::bail!("playback domain not found");
        }

        let mut tx = self.pool.begin().await?;
        let row = if let Some(group_id) = payload.id {
            sqlx::query_as::<_, AccountPermissionGroupRow>(
                r#"
UPDATE account_permission_groups
SET
    code = $2,
    name = $3,
    enabled = $4,
    updated_at = now()
WHERE id = $1
RETURNING id, code, name, enabled, ARRAY[]::UUID[] AS domain_ids, updated_at
                "#,
            )
            .bind(group_id)
            .bind(&code)
            .bind(&name)
            .bind(payload.enabled)
            .fetch_optional(&mut *tx)
            .await?
            .context("permission group not found")?
        } else {
            sqlx::query_as::<_, AccountPermissionGroupRow>(
                r#"
INSERT INTO account_permission_groups (id, code, name, enabled, created_at, updated_at)
VALUES ($1, $2, $3, $4, now(), now())
RETURNING id, code, name, enabled, ARRAY[]::UUID[] AS domain_ids, updated_at
                "#,
            )
            .bind(Uuid::now_v7())
            .bind(&code)
            .bind(&name)
            .bind(payload.enabled)
            .fetch_one(&mut *tx)
            .await?
        };

        sqlx::query("DELETE FROM account_permission_group_playback_domains WHERE group_id = $1")
            .bind(row.id)
            .execute(&mut *tx)
            .await?;

        sqlx::query(
            r#"
INSERT INTO account_permission_group_playback_domains (group_id, domain_id)
SELECT $1, v
FROM UNNEST($2::UUID[]) AS v
ON CONFLICT (group_id, domain_id) DO NOTHING
            "#,
        )
        .bind(row.id)
        .bind(&domain_ids)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        let latest = self
            .get_account_permission_group_by_id(row.id)
            .await?
            .context("permission group lost after upsert")?;
        Ok(latest)
    }

    pub async fn get_account_permission_group_by_id(
        &self,
        group_id: Uuid,
    ) -> anyhow::Result<Option<AccountPermissionGroup>> {
        let row = sqlx::query_as::<_, AccountPermissionGroupRow>(
            r#"
SELECT
    g.id,
    g.code,
    g.name,
    g.enabled,
    COALESCE(
        ARRAY_AGG(m.domain_id ORDER BY m.domain_id) FILTER (WHERE m.domain_id IS NOT NULL),
        ARRAY[]::UUID[]
    ) AS domain_ids,
    g.updated_at
FROM account_permission_groups g
LEFT JOIN account_permission_group_playback_domains m ON m.group_id = g.id
WHERE g.id = $1
GROUP BY g.id, g.code, g.name, g.enabled, g.updated_at
LIMIT 1
            "#,
        )
        .bind(group_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Into::into))
    }

    pub async fn get_playback_domain_by_id(
        &self,
        domain_id: Uuid,
    ) -> anyhow::Result<Option<PlaybackDomain>> {
        let row = sqlx::query_as::<_, PlaybackDomainRow>(
            r#"
SELECT
    id,
    name,
    base_url,
    enabled,
    priority,
    is_default,
    lumenbackend_node_id,
    traffic_multiplier,
    created_at,
    updated_at
FROM playback_domains
WHERE id = $1
LIMIT 1
            "#,
        )
        .bind(domain_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn upsert_playback_domain(
        &self,
        domain_id: Option<Uuid>,
        patch: PlaybackDomainUpdate,
    ) -> anyhow::Result<PlaybackDomain> {
        let mut tx = self.pool.begin().await?;
        let normalized_name = patch.name.trim();
        if normalized_name.is_empty() {
            anyhow::bail!("playback domain name is required");
        }

        let normalized_base_url = patch.base_url.trim().trim_end_matches('/');
        if normalized_base_url.is_empty() {
            anyhow::bail!("playback domain base_url is required");
        }

        let normalized_node_binding = normalize_playback_domain_node_binding(patch.lumenbackend_node_id);
        if let Some(Some(node_id)) = normalized_node_binding.as_ref() {
            let exists: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM lumenbackend_nodes WHERE node_id = $1)")
                    .bind(node_id.as_str())
                    .fetch_one(&mut *tx)
                    .await?;
            if !exists {
                anyhow::bail!("lumenbackend node not found");
            }
        }
        let normalized_multiplier = patch
            .traffic_multiplier
            .map(normalize_playback_domain_traffic_multiplier);

        if patch.is_default {
            sqlx::query("UPDATE playback_domains SET is_default = false, updated_at = now()")
                .execute(&mut *tx)
                .await?;
        }

        let row = if let Some(domain_id) = domain_id {
            let apply_node_binding = normalized_node_binding.is_some();
            let next_node_binding = normalized_node_binding.flatten();
            let apply_traffic_multiplier = normalized_multiplier.is_some();
            sqlx::query_as::<_, PlaybackDomainRow>(
                r#"
UPDATE playback_domains
SET
    name = $2,
    base_url = $3,
    enabled = $4,
    priority = $5,
    is_default = $6,
    lumenbackend_node_id = CASE WHEN $7 THEN $8 ELSE lumenbackend_node_id END,
    traffic_multiplier = CASE WHEN $9 THEN $10 ELSE traffic_multiplier END,
    updated_at = now()
WHERE id = $1
RETURNING
    id,
    name,
    base_url,
    enabled,
    priority,
    is_default,
    lumenbackend_node_id,
    traffic_multiplier,
    created_at,
    updated_at
                "#,
            )
            .bind(domain_id)
            .bind(normalized_name)
            .bind(normalized_base_url)
            .bind(patch.enabled)
            .bind(patch.priority)
            .bind(patch.is_default)
            .bind(apply_node_binding)
            .bind(next_node_binding.as_deref())
            .bind(apply_traffic_multiplier)
            .bind(normalized_multiplier)
            .fetch_optional(&mut *tx)
            .await?
            .context("playback domain not found")?
        } else {
            let node_binding = normalized_node_binding.flatten();
            let traffic_multiplier = normalized_multiplier.unwrap_or(1.0);
            sqlx::query_as::<_, PlaybackDomainRow>(
                r#"
INSERT INTO playback_domains (
    id,
    name,
    base_url,
    enabled,
    priority,
    is_default,
    lumenbackend_node_id,
    traffic_multiplier
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
RETURNING
    id,
    name,
    base_url,
    enabled,
    priority,
    is_default,
    lumenbackend_node_id,
    traffic_multiplier,
    created_at,
    updated_at
                "#,
            )
            .bind(Uuid::now_v7())
            .bind(normalized_name)
            .bind(normalized_base_url)
            .bind(patch.enabled)
            .bind(patch.priority)
            .bind(patch.is_default)
            .bind(node_binding.as_deref())
            .bind(traffic_multiplier)
            .fetch_one(&mut *tx)
            .await?
        };

        if !patch.is_default {
            let has_default: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM playback_domains WHERE is_default = true)",
            )
            .fetch_one(&mut *tx)
            .await?;

            if !has_default {
                sqlx::query(
                    r#"
UPDATE playback_domains
SET is_default = true, updated_at = now()
WHERE id = (
    SELECT id
    FROM playback_domains
    ORDER BY enabled DESC, priority DESC, updated_at DESC
    LIMIT 1
)
                    "#,
                )
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;

        let latest = self
            .get_playback_domain_by_id(row.id)
            .await?
            .context("playback domain lost after upsert")?;
        Ok(latest)
    }

    pub async fn delete_playback_domain(&self, domain_id: Uuid) -> anyhow::Result<bool> {
        let mut tx = self.pool.begin().await?;

        // Prevent deleting the default domain if other domains exist
        let is_default: Option<bool> =
            sqlx::query_scalar("SELECT is_default FROM playback_domains WHERE id = $1")
                .bind(domain_id)
                .fetch_optional(&mut *tx)
                .await?;

        let Some(is_default) = is_default else {
            return Ok(false);
        };

        // Remove references from permission group mappings
        sqlx::query("DELETE FROM account_permission_group_playback_domains WHERE domain_id = $1")
            .bind(domain_id)
            .execute(&mut *tx)
            .await?;

        // Remove user preference references
        sqlx::query("DELETE FROM user_playback_domain_preferences WHERE domain_id = $1")
            .bind(domain_id)
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM playback_domains WHERE id = $1")
            .bind(domain_id)
            .execute(&mut *tx)
            .await?;

        // If deleted domain was default, promote the next best candidate
        if is_default {
            sqlx::query(
                r#"
UPDATE playback_domains
SET is_default = true, updated_at = now()
WHERE id = (
    SELECT id
    FROM playback_domains
    ORDER BY enabled DESC, priority DESC, updated_at DESC
    LIMIT 1
)
                "#,
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(true)
    }

    pub async fn set_user_playback_domain_preference(
        &self,
        user_id: Uuid,
        domain_id: Uuid,
    ) -> anyhow::Result<PlaybackDomain> {
        let domain = self
            .get_playback_domain_by_id(domain_id)
            .await?
        .context("playback domain not found")?;
        if !domain.enabled {
            anyhow::bail!("playback domain is disabled");
        }
        if !self
            .is_playback_domain_allowed_for_user(user_id, domain.id)
            .await?
        {
            anyhow::bail!("playback domain is not allowed for current plan");
        }

        sqlx::query(
            r#"
INSERT INTO user_playback_domain_preferences (user_id, domain_id, updated_at)
VALUES ($1, $2, now())
ON CONFLICT(user_id) DO UPDATE SET
    domain_id = EXCLUDED.domain_id,
    updated_at = now()
            "#,
        )
        .bind(user_id)
        .bind(domain_id)
        .execute(&self.pool)
        .await?;

        Ok(domain)
    }

    pub async fn get_user_playback_domain_preference(
        &self,
        user_id: Uuid,
    ) -> anyhow::Result<Option<PlaybackDomain>> {
        let row = sqlx::query_as::<_, PlaybackDomainRow>(
            r#"
SELECT
    d.id,
    d.name,
    d.base_url,
    d.enabled,
    d.priority,
    d.is_default,
    d.lumenbackend_node_id,
    d.traffic_multiplier,
    d.created_at,
    d.updated_at
FROM user_playback_domain_preferences p
JOIN playback_domains d ON d.id = p.domain_id
WHERE p.user_id = $1
LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Into::into))
    }

    pub async fn resolve_playback_domain_for_user(
        &self,
        user_id: Uuid,
    ) -> anyhow::Result<Option<PlaybackDomain>> {
        let available = self.list_playback_domains_for_user(user_id).await?;
        if available.is_empty() {
            return Ok(None);
        }

        if let Some(selected) = self.get_user_playback_domain_preference(user_id).await?
            && selected.enabled
            && available.iter().any(|item| item.id == selected.id)
        {
            return Ok(Some(selected));
        }

        Ok(available.into_iter().next())
    }

    pub async fn resolve_playback_domain_for_lumenbackend_node(
        &self,
        node_id: &str,
    ) -> anyhow::Result<Option<PlaybackDomain>> {
        let node_id = node_id.trim();
        if node_id.is_empty() {
            return Ok(None);
        }

        let row = sqlx::query_as::<_, PlaybackDomainRow>(
            r#"
SELECT
    id,
    name,
    base_url,
    enabled,
    priority,
    is_default,
    lumenbackend_node_id,
    traffic_multiplier,
    created_at,
    updated_at
FROM playback_domains
WHERE enabled = true
  AND lumenbackend_node_id = $1
ORDER BY is_default DESC, priority DESC, updated_at DESC
LIMIT 1
            "#,
        )
        .bind(node_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn list_playback_domains_for_user(
        &self,
        user_id: Uuid,
    ) -> anyhow::Result<Vec<PlaybackDomain>> {
        let active_group_id = self.resolve_active_permission_group_for_user(user_id).await?;
        let rows = if let Some(group_id) = active_group_id {
            sqlx::query_as::<_, PlaybackDomainRow>(
                r#"
SELECT
    d.id,
    d.name,
    d.base_url,
    d.enabled,
    d.priority,
    d.is_default,
    d.lumenbackend_node_id,
    d.traffic_multiplier,
    d.created_at,
    d.updated_at
FROM playback_domains d
JOIN account_permission_group_playback_domains m ON m.domain_id = d.id
JOIN account_permission_groups g ON g.id = m.group_id
WHERE
    d.enabled = true
    AND g.enabled = true
    AND g.id = $1
ORDER BY d.is_default DESC, d.priority DESC, d.updated_at DESC
                "#,
            )
            .bind(group_id)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, PlaybackDomainRow>(
                r#"
SELECT
    id,
    name,
    base_url,
    enabled,
    priority,
    is_default,
    lumenbackend_node_id,
    traffic_multiplier,
    created_at,
    updated_at
FROM playback_domains
WHERE enabled = true
ORDER BY is_default DESC, priority DESC, updated_at DESC
                "#,
            )
            .fetch_all(&self.pool)
            .await?
        };

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn is_playback_domain_allowed_for_user(
        &self,
        user_id: Uuid,
        domain_id: Uuid,
    ) -> anyhow::Result<bool> {
        let active_group_id = self.resolve_active_permission_group_for_user(user_id).await?;
        if let Some(group_id) = active_group_id {
            let allowed = sqlx::query_scalar(
                r#"
SELECT EXISTS(
    SELECT 1
    FROM account_permission_group_playback_domains m
    JOIN account_permission_groups g ON g.id = m.group_id
    JOIN playback_domains d ON d.id = m.domain_id
    WHERE m.group_id = $1
      AND m.domain_id = $2
      AND g.enabled = true
      AND d.enabled = true
)
                "#,
            )
            .bind(group_id)
            .bind(domain_id)
            .fetch_one(&self.pool)
            .await?;
            Ok(allowed)
        } else {
            let enabled = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM playback_domains WHERE id = $1 AND enabled = true)",
            )
            .bind(domain_id)
            .fetch_one(&self.pool)
            .await?;
            Ok(enabled)
        }
    }

    async fn resolve_active_permission_group_for_user(
        &self,
        user_id: Uuid,
    ) -> anyhow::Result<Option<Uuid>> {
        let group_id = sqlx::query_scalar(
            r#"
SELECT p.permission_group_id
FROM billing_plan_subscriptions s
JOIN billing_plans p ON p.id = s.plan_id
WHERE s.user_id = $1 AND s.status = 'active'
ORDER BY s.started_at DESC
LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        .flatten();

        Ok(group_id)
    }

}

fn normalize_permission_group_domain_ids(mut domain_ids: Vec<Uuid>) -> Vec<Uuid> {
    domain_ids.sort_unstable();
    domain_ids.dedup();
    domain_ids
}

fn normalize_playback_domain_node_binding(input: Option<Option<String>>) -> Option<Option<String>> {
    input.map(|value| {
        value.and_then(|raw| {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
    })
}

fn normalize_playback_domain_traffic_multiplier(raw: f64) -> f64 {
    if !raw.is_finite() {
        return 1.0;
    }
    raw.clamp(0.01, 100.0)
}

#[cfg(test)]
mod app_libraries_storage_domains_tests {
    use super::{
        normalize_permission_group_domain_ids, normalize_playback_domain_node_binding,
        normalize_playback_domain_traffic_multiplier,
    };
    use uuid::Uuid;

    #[test]
    fn normalize_playback_domain_node_binding_handles_trim_and_clear() {
        assert_eq!(
            normalize_playback_domain_node_binding(Some(Some(" node-a ".to_string()))),
            Some(Some("node-a".to_string()))
        );
        assert_eq!(
            normalize_playback_domain_node_binding(Some(Some("   ".to_string()))),
            Some(None)
        );
        assert_eq!(normalize_playback_domain_node_binding(Some(None)), Some(None));
        assert_eq!(normalize_playback_domain_node_binding(None), None);
    }

    #[test]
    fn normalize_playback_domain_traffic_multiplier_clamps() {
        assert_eq!(normalize_playback_domain_traffic_multiplier(0.0), 0.01);
        assert_eq!(normalize_playback_domain_traffic_multiplier(1.75), 1.75);
        assert_eq!(normalize_playback_domain_traffic_multiplier(999.0), 100.0);
        assert_eq!(normalize_playback_domain_traffic_multiplier(f64::NAN), 1.0);
    }

    #[test]
    fn normalize_permission_group_domain_ids_deduplicates() {
        let a = Uuid::now_v7();
        let b = Uuid::now_v7();
        let normalized = normalize_permission_group_domain_ids(vec![a, b, a, b]);
        assert_eq!(normalized.len(), 2);
        assert!(normalized.contains(&a));
        assert!(normalized.contains(&b));
    }
}
