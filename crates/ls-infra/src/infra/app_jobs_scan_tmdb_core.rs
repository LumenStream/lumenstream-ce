#[derive(Debug, Clone, FromRow)]
struct DueLibraryDispatchRow {
    id: Uuid,
    name: String,
}

impl AppInfra {
    async fn run_scan_job(
        &self,
        job_id: Uuid,
        payload: &Value,
        trigger_type: Option<&str>,
    ) -> anyhow::Result<Value> {
        let probe_policy = if payload
            .get("probe_mediainfo")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            scanner::ScanProbePolicy::Enabled
        } else {
            scanner::ScanProbePolicy::Disabled
        };
        self.run_scan_job_with_probe_policy(
            job_id,
            payload,
            "scan_library",
            probe_policy,
            trigger_type,
        )
        .await
    }

    // Legacy alias: keep processing historical `scan_library_no_probe` jobs by routing
    // them to the merged `scan_library` implementation with probing disabled.
    async fn run_scan_no_probe_job(
        &self,
        job_id: Uuid,
        payload: &Value,
        trigger_type: Option<&str>,
    ) -> anyhow::Result<Value> {
        self.run_scan_job_with_probe_policy(
            job_id,
            payload,
            "scan_library",
            scanner::ScanProbePolicy::Disabled,
            trigger_type,
        )
        .await
    }

    async fn run_scan_job_with_probe_policy(
        &self,
        job_id: Uuid,
        payload: &Value,
        subtask_kind: &str,
        probe_policy: scanner::ScanProbePolicy,
        trigger_type: Option<&str>,
    ) -> anyhow::Result<Value> {
        let mode = payload
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("incremental");
        let path_prefix = payload
            .get("path_prefix")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|v| !v.is_empty());
        let library_id = payload
            .get("library_id")
            .and_then(Value::as_str)
            .and_then(|v| Uuid::parse_str(v).ok());

        // Scheduled incremental "all libraries" scans should only enqueue due libraries.
        if library_id.is_none()
            && matches!(trigger_type, Some("scheduled"))
            && mode.eq_ignore_ascii_case("incremental")
            && path_prefix.is_none()
        {
            return self
                .run_scan_due_libraries(job_id, mode, subtask_kind, probe_policy)
                .await;
        }

        // Manual or explicitly targeted "all libraries" scans keep the original fan-out behavior.
        if library_id.is_none() {
            return self
                .run_scan_all_libraries(job_id, payload, subtask_kind, probe_policy)
                .await;
        }
        let scan_mode = if mode.eq_ignore_ascii_case("full") {
            scanner::ScanMode::Full
        } else {
            scanner::ScanMode::Incremental
        };

        let library = self
            .get_library_by_id(library_id.unwrap())
            .await?
            .context("library not found")?;

        let scan_scope = resolve_scan_scope_path(&library.paths, path_prefix)?;

        sqlx::query(
            r#"
INSERT INTO library_scan_state (library_id, last_scan_started_at, last_scan_mode)
VALUES ($1, now(), $2)
ON CONFLICT(library_id) DO UPDATE SET
    last_scan_started_at = now(),
    last_scan_mode = EXCLUDED.last_scan_mode
            "#,
        )
        .bind(library.id)
        .bind(mode)
        .execute(&self.pool)
        .await?;

        let since = sqlx::query_scalar::<_, Option<DateTime<Utc>>>(
            "SELECT last_scan_finished_at FROM library_scan_state WHERE library_id = $1",
        )
        .bind(library.id)
        .fetch_optional(&self.pool)
        .await?
        .flatten();
        let scan_started_at = Utc::now();

        self.set_job_progress(
            job_id,
            "scan_files",
            1,
            0,
            "开始扫描媒体库",
            json!({ "library_id": library.id, "mode": mode }),
        )
        .await?;

        let summary = scanner::scan_library(
            &self.pool,
            library.id,
            &library.paths,
            &library.library_type,
            scan_scope.as_deref(),
            &self.config_snapshot().scan.subtitle_extensions,
            scan_mode,
            since,
            self.config_snapshot().scan.incremental_grace_seconds,
            &self.config_snapshot().scan.mediainfo_cache_dir,
            scanner::ScanExistingItemPolicy::Skip,
            probe_policy,
            |total, completed| {
                async move {
                    self.abort_if_job_cancel_requested(job_id).await?;
                    self.set_job_progress(
                        job_id,
                        "scan_files",
                        total,
                        completed,
                        "扫描媒体文件",
                        json!({ "library_id": library.id, "mode": mode }),
                    )
                    .await
                }
            },
        )
        .await?;

        sqlx::query(
            r#"
UPDATE library_scan_state
SET last_scan_finished_at = now(),
    last_scan_cursor = now(),
    last_scan_mode = $2
WHERE library_id = $1
            "#,
        )
        .bind(library.id)
        .bind(mode)
        .execute(&self.pool)
        .await?;

        // Incremental Meilisearch index update for items touched by this scan
        let indexed = self
            .update_search_index_since(Some(library.id), scan_started_at)
            .await
            .unwrap_or_else(|err| {
                warn!(error = %err, "failed to update search index after scan");
                0
            });

        let mut scraper_fill_job_id: Option<Uuid> = None;
        if self.scraper_is_enabled() && !self.config_snapshot().tmdb.api_key.trim().is_empty() {
            let scraper_job = self
                .enqueue_scrape_fill_job_for_new_items(Some(library.id), scan_started_at)
                .await?;
            scraper_fill_job_id = Some(scraper_job.id);
        }

        Ok(json!({
            "scanned_files": summary.scanned_files,
            "upserted_items": summary.upserted_items,
            "subtitle_files": summary.subtitle_files,
            "duplicate_merged": summary.duplicate_merged,
            "metadata_missing": summary.metadata_missing,
            "search_indexed": indexed,
            "library_id": library.id,
            "library_name": library.name,
            "mode": mode,
            "path_prefix": path_prefix,
            "effective_scan_root": scan_scope
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| library.paths.first().cloned().unwrap_or_default()),
            "job_id": job_id,
            "scraper_fill_job_id": scraper_fill_job_id,
            "tmdb_fill_job_id": scraper_fill_job_id,
        }))
    }

    async fn run_scan_due_libraries(
        &self,
        job_id: Uuid,
        mode: &str,
        subtask_kind: &str,
        probe_policy: scanner::ScanProbePolicy,
    ) -> anyhow::Result<Value> {
        let due_libraries: Vec<DueLibraryDispatchRow> = sqlx::query_as(
            r#"
SELECT l.id, l.name
FROM libraries l
LEFT JOIN library_scan_state s ON l.id = s.library_id
WHERE l.enabled = true
  AND l.scan_interval_hours > 0
  AND (
    s.last_scan_finished_at IS NULL
    OR s.last_scan_finished_at + (l.scan_interval_hours || ' hours')::interval < now()
  )
ORDER BY l.name ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        if due_libraries.is_empty() {
            self.set_job_progress(
                job_id,
                "enqueue_subtasks",
                1,
                1,
                "暂无到期媒体库扫描",
                json!({}),
            )
            .await?;
            return Ok(json!({
                "job_id": job_id,
                "mode": mode,
                "dispatch_mode": "due_libraries",
                "libraries_scanned": 0,
                "enqueued_jobs": [],
            }));
        }

        let probe_mediainfo = matches!(probe_policy, scanner::ScanProbePolicy::Enabled);
        let total = due_libraries.len() as i64;
        let mut completed = 0_i64;
        let mut enqueued_jobs = Vec::new();

        for lib in &due_libraries {
            self.abort_if_job_cancel_requested(job_id).await?;
            match self
                .enqueue_scan_job_for_kind(
                    subtask_kind,
                    lib.id,
                    Some(mode),
                    None,
                    Some(probe_mediainfo),
                )
                .await
            {
                Ok(job) => {
                    enqueued_jobs.push(json!({
                        "job_id": job.id,
                        "library_id": lib.id,
                        "library_name": lib.name,
                    }));
                }
                Err(err) => {
                    warn!(
                        error = %err,
                        library_id = %lib.id,
                        library_name = %lib.name,
                        "failed to enqueue due scan job for library"
                    );
                }
            }

            completed += 1;
            self.set_job_progress(
                job_id,
                "enqueue_subtasks",
                total,
                completed,
                "分发到期媒体库扫描子任务",
                json!({ "enqueued": enqueued_jobs.len() }),
            )
            .await?;
        }

        Ok(json!({
            "job_id": job_id,
            "mode": mode,
            "dispatch_mode": "due_libraries",
            "libraries_scanned": due_libraries.len(),
            "enqueued_jobs": enqueued_jobs,
        }))
    }

    async fn run_scan_all_libraries(
        &self,
        job_id: Uuid,
        payload: &Value,
        subtask_kind: &str,
        probe_policy: scanner::ScanProbePolicy,
    ) -> anyhow::Result<Value> {
        let mut libraries = self.list_libraries().await?;
        libraries.retain(|library| library.enabled);
        libraries.sort_by(|a, b| a.name.cmp(&b.name));

        if libraries.is_empty() {
            self.set_job_progress(job_id, "enqueue_subtasks", 1, 1, "无可扫描媒体库", json!({}))
                .await?;
            return Ok(json!({
                "job_id": job_id,
                "libraries_scanned": 0,
                "enqueued_jobs": [],
            }));
        }

        let mode = payload
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("incremental");
        let probe_mediainfo = matches!(probe_policy, scanner::ScanProbePolicy::Enabled);
        let mut enqueued_jobs = Vec::new();
        let total = libraries.len() as i64;
        let mut completed = 0_i64;
        for lib in &libraries {
            self.abort_if_job_cancel_requested(job_id).await?;
            match self
                .enqueue_scan_job_for_kind(
                    subtask_kind,
                    lib.id,
                    Some(mode),
                    None,
                    Some(probe_mediainfo),
                )
                .await
            {
                Ok(job) => {
                    enqueued_jobs.push(json!({
                        "job_id": job.id,
                        "library_id": lib.id,
                        "library_name": lib.name,
                    }));
                }
                Err(err) => {
                    warn!(
                        error = %err,
                        library_id = %lib.id,
                        library_name = %lib.name,
                        "failed to enqueue scan job for library"
                    );
                }
            }
            completed += 1;
            self.set_job_progress(
                job_id,
                "enqueue_subtasks",
                total,
                completed,
                "分发扫描子任务",
                json!({ "enqueued": enqueued_jobs.len() }),
            )
            .await?;
        }

        Ok(json!({
            "job_id": job_id,
            "libraries_scanned": libraries.len(),
            "enqueued_jobs": enqueued_jobs,
        }))
    }

    async fn run_metadata_repair_job(&self, job_id: Uuid, payload: &Value) -> anyhow::Result<Value> {
        let library_id = payload
            .get("library_id")
            .and_then(Value::as_str)
            .and_then(|v| Uuid::parse_str(v).ok());

        let repaired = scanner::repair_metadata(
            &self.pool,
            library_id,
            &self.config_snapshot().scan.subtitle_extensions,
            &self.config_snapshot().scan.mediainfo_cache_dir,
            |total, completed| {
                async move {
                    self.abort_if_job_cancel_requested(job_id).await?;
                    self.set_job_progress(
                        job_id,
                        "repair_metadata",
                        total,
                        completed,
                        "修复元数据",
                        json!({ "library_id": library_id }),
                    )
                    .await
                }
            },
        )
        .await?;

        Ok(json!({ "repaired_items": repaired, "library_id": library_id }))
    }

    async fn run_subtitle_sync_job(&self, job_id: Uuid, payload: &Value) -> anyhow::Result<Value> {
        let library_id = payload
            .get("library_id")
            .and_then(Value::as_str)
            .and_then(|v| Uuid::parse_str(v).ok());
        let mode = payload
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("incremental");
        let sync_mode = if mode.eq_ignore_ascii_case("full") {
            scanner::ScanMode::Full
        } else {
            scanner::ScanMode::Incremental
        };

        self.mark_subtitle_sync_started(library_id, mode).await?;

        let updated = scanner::sync_subtitles(
            &self.pool,
            library_id,
            &self.config_snapshot().scan.subtitle_extensions,
            sync_mode,
            self.config_snapshot().scan.incremental_grace_seconds,
            |total, completed| {
                async move {
                    self.abort_if_job_cancel_requested(job_id).await?;
                    self.set_job_progress(
                        job_id,
                        "sync_subtitles",
                        total,
                        completed,
                        "同步字幕",
                        json!({ "library_id": library_id, "mode": mode }),
                    )
                    .await
                }
            },
        )
        .await?;

        self.mark_subtitle_sync_finished(library_id, mode).await?;

        Ok(json!({ "updated_items": updated, "library_id": library_id, "mode": mode }))
    }

    async fn run_scrape_fill_job(&self, job_id: Uuid, payload: &Value) -> anyhow::Result<Value> {
        let library_id = payload
            .get("library_id")
            .and_then(Value::as_str)
            .and_then(|v| Uuid::parse_str(v).ok());
        let new_only = payload
            .get("new_only")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let new_since = payload
            .get("new_since")
            .and_then(Value::as_str)
            .and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok())
            .map(|v| v.with_timezone(&Utc));
        let started_at = Utc::now();

        let items = self
            .find_items_for_tmdb_fill(library_id, new_since, new_only)
            .await?;
        let mut filled = 0_i64;
        let mut skipped = 0_i64;
        let mut failed = 0_i64;

        let total = items.len() as i64;
        if total == 0 {
            self.set_job_progress(job_id, "scrape_fill", 1, 1, "无待补齐项目", json!({}))
                .await?;
        }

        for (idx, item) in items.into_iter().enumerate() {
            self.abort_if_job_cancel_requested(job_id).await?;
            match self.fill_metadata_with_scraper(&item, false).await? {
                TmdbFillStatus::Filled => filled += 1,
                TmdbFillStatus::Skipped => skipped += 1,
                TmdbFillStatus::Failed => failed += 1,
            }
            let completed = idx as i64 + 1;
            if total > 0 && (completed % 25 == 0 || completed == total) {
                self.set_job_progress(
                    job_id,
                    "scrape_fill",
                    total,
                    completed,
                    "补齐刮削元数据",
                    json!({ "filled": filled, "skipped": skipped, "failed": failed }),
                )
                .await?;
            }
        }

        // Incremental Meilisearch index update for media touched by this TMDB fill.
        // For scan follow-up jobs, reuse new_since to cover newly scanned rows too.
        let search_index_since = scrape_fill_search_index_since(started_at, new_since.clone());
        let search_indexed = self
            .update_search_index_since(library_id, search_index_since)
            .await
            .unwrap_or_else(|err| {
                warn!(error = %err, ?library_id, "failed to update search index after scrape fill");
                0
            });

        // Index people updated by this TMDB fill into Meilisearch
        let people_indexed = if let Some(new_since) = new_since {
            self.index_people_since(new_since).await.unwrap_or_else(|err| {
                warn!(error = %err, "failed to index people after scrape fill");
                0
            })
        } else {
            0
        };

        Ok(json!({
            "filled": filled,
            "skipped": skipped,
            "failed": failed,
            "library_id": library_id,
            "new_only": new_only,
            "new_since": new_since.map(|v| v.to_rfc3339()),
            "search_indexed": search_indexed,
            "people_indexed": people_indexed,
        }))
    }

    async fn wait_tmdb_rate_limit(&self) {
        if self.config_snapshot().tmdb.request_interval_ms == 0 {
            return;
        }

        let mut guard = self.tmdb_last_request.lock().await;
        let interval = std::time::Duration::from_millis(self.config_snapshot().tmdb.request_interval_ms);
        if let Some(last) = *guard {
            let elapsed = last.elapsed();
            if elapsed < interval {
                tokio::time::sleep(interval - elapsed).await;
            }
        }
        *guard = Some(TokioInstant::now());
    }

    async fn mark_subtitle_sync_started(
        &self,
        library_id: Option<Uuid>,
        mode: &str,
    ) -> anyhow::Result<()> {
        if let Some(library_id) = library_id {
            sqlx::query(
                r#"
INSERT INTO library_scan_state (library_id, last_subtitle_sync_started_at, last_subtitle_sync_mode)
VALUES ($1, now(), $2)
ON CONFLICT(library_id) DO UPDATE SET
    last_subtitle_sync_started_at = now(),
    last_subtitle_sync_mode = EXCLUDED.last_subtitle_sync_mode
                "#,
            )
            .bind(library_id)
            .bind(mode)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query(
                r#"
INSERT INTO library_scan_state (library_id, last_subtitle_sync_started_at, last_subtitle_sync_mode)
SELECT id, now(), $1
FROM libraries
WHERE enabled = true
ON CONFLICT(library_id) DO UPDATE SET
    last_subtitle_sync_started_at = now(),
    last_subtitle_sync_mode = EXCLUDED.last_subtitle_sync_mode
                "#,
            )
            .bind(mode)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    async fn mark_subtitle_sync_finished(
        &self,
        library_id: Option<Uuid>,
        mode: &str,
    ) -> anyhow::Result<()> {
        if let Some(library_id) = library_id {
            sqlx::query(
                r#"
INSERT INTO library_scan_state (library_id, last_subtitle_sync_finished_at, last_subtitle_sync_mode)
VALUES ($1, now(), $2)
ON CONFLICT(library_id) DO UPDATE SET
    last_subtitle_sync_finished_at = now(),
    last_subtitle_sync_mode = EXCLUDED.last_subtitle_sync_mode
                "#,
            )
            .bind(library_id)
            .bind(mode)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query(
                r#"
INSERT INTO library_scan_state (library_id, last_subtitle_sync_finished_at, last_subtitle_sync_mode)
SELECT id, now(), $1
FROM libraries
WHERE enabled = true
ON CONFLICT(library_id) DO UPDATE SET
    last_subtitle_sync_finished_at = now(),
    last_subtitle_sync_mode = EXCLUDED.last_subtitle_sync_mode
                "#,
            )
            .bind(mode)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    async fn get_tmdb_cache_hit(&self, cache_key: &str) -> anyhow::Result<Option<Option<Value>>> {
        let row = sqlx::query_as::<_, TmdbCacheRow>(
            r#"
SELECT response, has_result, expires_at
FROM tmdb_cache
WHERE cache_key = $1
LIMIT 1
            "#,
        )
        .bind(cache_key)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        if row.expires_at < Utc::now() {
            sqlx::query("DELETE FROM tmdb_cache WHERE cache_key = $1")
                .bind(cache_key)
                .execute(&self.pool)
                .await?;
            return Ok(None);
        }

        sqlx::query(
            "UPDATE tmdb_cache SET hit_count = hit_count + 1, updated_at = now() WHERE cache_key = $1",
        )
        .bind(cache_key)
        .execute(&self.pool)
        .await?;

        if !row.has_result {
            return Ok(Some(None));
        }

        let first = row
            .response
            .get("results")
            .and_then(Value::as_array)
            .and_then(|arr| arr.first())
            .cloned();

        Ok(Some(first))
    }

    async fn upsert_tmdb_cache(
        &self,
        cache_key: &str,
        item_name: &str,
        item_kind: &str,
        response: &Value,
        has_result: bool,
    ) -> anyhow::Result<()> {
        let ttl_seconds = self.config_snapshot().tmdb.cache_ttl_seconds.max(30);
        let expires_at = Utc::now() + Duration::seconds(ttl_seconds);

        sqlx::query(
            r#"
INSERT INTO tmdb_cache (cache_key, query, item_type, response, has_result, expires_at)
VALUES ($1, $2, $3, $4, $5, $6)
ON CONFLICT(cache_key) DO UPDATE SET
    query = EXCLUDED.query,
    item_type = EXCLUDED.item_type,
    response = EXCLUDED.response,
    has_result = EXCLUDED.has_result,
    expires_at = EXCLUDED.expires_at,
    updated_at = now()
            "#,
        )
        .bind(cache_key)
        .bind(item_name)
        .bind(item_kind)
        .bind(response)
        .bind(has_result)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn tmdb_get_json(&self, endpoint: &str) -> anyhow::Result<Value> {
        let attempts = self.config_snapshot().tmdb.retry_attempts.max(1);
        let mut last_error = String::new();

        for attempt in 1..=attempts {
            self.wait_tmdb_rate_limit().await;
            self.metrics
                .tmdb_http_requests_total
                .fetch_add(1, Ordering::Relaxed);

            let response = self
                .http_client
                .get(endpoint)
                .bearer_auth(self.config_snapshot().tmdb.api_key.trim())
                .timeout(std::time::Duration::from_secs(
                    self.config_snapshot().tmdb.timeout_seconds.max(1),
                ))
                .send()
                .await;

            let response = match response {
                Ok(v) => v,
                Err(err) => {
                    last_error = format!("request failed: {err}");
                    if attempt < attempts {
                        let sleep_ms = self
                            .config_snapshot()
                            .tmdb
                            .retry_backoff_ms
                            .saturating_mul(u64::from(attempt));
                        tokio::time::sleep(std::time::Duration::from_millis(sleep_ms)).await;
                    }
                    continue;
                }
            };

            if !response.status().is_success() {
                let status = response.status();
                // 404 is deterministic — retrying won't help
                if status == reqwest::StatusCode::NOT_FOUND {
                    anyhow::bail!("tmdb 404: {endpoint}");
                }
                last_error = format!("tmdb returned status {status}");
                if attempt < attempts {
                    let sleep_ms = self
                        .config_snapshot()
                        .tmdb
                        .retry_backoff_ms
                        .saturating_mul(u64::from(attempt));
                    tokio::time::sleep(std::time::Duration::from_millis(sleep_ms)).await;
                }
                continue;
            }

            let payload: Value = response
                .json()
                .await
                .with_context(|| format!("failed to decode tmdb response: {endpoint}"))?;
            return Ok(payload);
        }

        anyhow::bail!(
            "tmdb request failed: {}",
            if last_error.is_empty() {
                "unknown error".to_string()
            } else {
                last_error
            }
        );
    }

    /// Like `tmdb_get_json`, but returns `Ok(None)` when the endpoint 404s
    /// instead of propagating the error.  Other errors are passed through.
    async fn tmdb_get_json_opt(&self, endpoint: &str) -> anyhow::Result<Option<Value>> {
        match self.tmdb_get_json(endpoint).await {
            Ok(v) => Ok(Some(v)),
            Err(err) if err.to_string().starts_with("tmdb 404: ") => Ok(None),
            Err(err) => Err(err),
        }
    }

    /// Resolve an IMDB ID to a TMDB ID using the TMDB `/find` endpoint.
    /// `media_type` should be `"movie"` or `"tv"`.
    async fn tmdb_find_by_imdb(
        &self,
        imdb_id: &str,
        media_type: &str,
    ) -> anyhow::Result<Option<i64>> {
        let endpoint = format!(
            "{TMDB_API_BASE}/find/{imdb_id}?external_source=imdb_id&language={lang}",
            imdb_id = urlencoding::encode(imdb_id),
            lang = urlencoding::encode(&self.config_snapshot().tmdb.language),
        );
        let payload = self.tmdb_get_json(&endpoint).await?;

        let results_key = if media_type == "movie" {
            "movie_results"
        } else {
            "tv_results"
        };
        let tmdb_id = payload
            .get(results_key)
            .and_then(Value::as_array)
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("id"))
            .and_then(Value::as_i64);

        if let Some(id) = tmdb_id {
            info!(imdb_id, tmdb_id = id, media_type, "resolved imdb_id to tmdb_id via find API");
        }
        Ok(tmdb_id)
    }

    async fn tmdb_search_first(
        &self,
        item_kind: &str,
        item_name: &str,
    ) -> anyhow::Result<Option<Value>> {
        let query_name = {
            let cleaned = search::normalize_media_title(item_name);
            if cleaned.trim().is_empty() {
                item_name.trim().to_string()
            } else {
                cleaned
            }
        };
        let cache_key = tmdb_cache_key(
            item_kind,
            &query_name,
            &self.config_snapshot().tmdb.language,
        );

        if let Some(cached) = self.get_tmdb_cache_hit(&cache_key).await? {
            self.metrics
                .tmdb_cache_hits_total
                .fetch_add(1, Ordering::Relaxed);
            return Ok(cached);
        }

        self.metrics
            .tmdb_cache_misses_total
            .fetch_add(1, Ordering::Relaxed);
        let endpoint = format!(
            "{TMDB_API_BASE}/search/{kind}?query={query}&language={lang}",
            kind = item_kind,
            query = urlencoding::encode(&query_name),
            lang = urlencoding::encode(&self.config_snapshot().tmdb.language)
        );
        let payload = self.tmdb_get_json(&endpoint).await?;
        let first = payload
            .get("results")
            .and_then(Value::as_array)
            .and_then(|arr| arr.first())
            .cloned();
        if first.is_none() {
            info!(item_kind, item_name, query_name, "tmdb search returned no results");
        }
        self.upsert_tmdb_cache(&cache_key, &query_name, item_kind, &payload, first.is_some())
            .await?;
        Ok(first)
    }

    async fn record_tmdb_failure(
        &self,
        item_id: Uuid,
        item_name: &str,
        item_type: &str,
        attempts: i32,
        error_message: &str,
    ) -> anyhow::Result<()> {
        let detail = json!({
            "item_id": item_id,
            "item_name": item_name,
            "item_type": item_type,
            "attempts": attempts,
            "error": error_message,
        });

        sqlx::query(
            r#"
INSERT INTO tmdb_failures (id, media_item_id, item_name, item_type, attempts, error)
VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(item_id)
        .bind(item_name)
        .bind(item_type)
        .bind(attempts)
        .bind(error_message)
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
INSERT INTO system_events (id, event_type, level, source, detail)
VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(Uuid::now_v7())
        .bind("tmdb.fill.failed")
        .bind("error")
        .bind("tmdb")
        .bind(detail)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

}

fn scrape_fill_search_index_since(
    started_at: DateTime<Utc>,
    new_since: Option<DateTime<Utc>>,
) -> DateTime<Utc> {
    new_since.unwrap_or(started_at)
}
