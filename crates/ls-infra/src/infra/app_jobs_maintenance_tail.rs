impl AppInfra {
    async fn run_search_reindex_job(&self, job_id: Uuid, payload: &Value) -> anyhow::Result<Value> {
        let library_id = payload
            .get("library_id")
            .and_then(Value::as_str)
            .and_then(|v| Uuid::parse_str(v).ok());
        let batch_size = payload
            .get("batch_size")
            .and_then(Value::as_i64)
            .unwrap_or(500)
            .clamp(50, 5000);

        let mut cursor: Option<Uuid> = None;
        let mut processed = 0_i64;
        let mut updated = 0_i64;
        let mut indexed = 0_i64;
        let total: i64 = if let Some(library_id) = library_id {
            sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM media_items WHERE library_id = $1")
                .bind(library_id)
                .fetch_one(&self.pool)
                .await?
        } else {
            sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM media_items")
                .fetch_one(&self.pool)
                .await?
        };

        if total == 0 {
            self.set_job_progress(job_id, "reindex_search", 1, 1, "无可重建条目", json!({}))
                .await?;
        }

        loop {
            self.abort_if_job_cancel_requested(job_id).await?;
            let rows: Vec<SearchIndexRow> = match (library_id, cursor) {
                (Some(library_id), Some(cursor)) => {
                    sqlx::query_as::<_, SearchIndexRow>(
                        r#"
SELECT id, name, item_type, library_id, series_id
FROM media_items
WHERE library_id = $1 AND id > $2
ORDER BY id ASC
LIMIT $3
                        "#,
                    )
                    .bind(library_id)
                    .bind(cursor)
                    .bind(batch_size)
                    .fetch_all(&self.pool)
                    .await?
                }
                (Some(library_id), None) => {
                    sqlx::query_as::<_, SearchIndexRow>(
                        r#"
SELECT id, name, item_type, library_id, series_id
FROM media_items
WHERE library_id = $1
ORDER BY id ASC
LIMIT $2
                        "#,
                    )
                    .bind(library_id)
                    .bind(batch_size)
                    .fetch_all(&self.pool)
                    .await?
                }
                (None, Some(cursor)) => {
                    sqlx::query_as::<_, SearchIndexRow>(
                        r#"
SELECT id, name, item_type, library_id, series_id
FROM media_items
WHERE id > $1
ORDER BY id ASC
LIMIT $2
                        "#,
                    )
                    .bind(cursor)
                    .bind(batch_size)
                    .fetch_all(&self.pool)
                    .await?
                }
                (None, None) => {
                    sqlx::query_as::<_, SearchIndexRow>(
                        r#"
SELECT id, name, item_type, library_id, series_id
FROM media_items
ORDER BY id ASC
LIMIT $1
                        "#,
                    )
                    .bind(batch_size)
                    .fetch_all(&self.pool)
                    .await?
                }
            };

            if rows.is_empty() {
                break;
            }

            let mut docs = Vec::new();

            for row in &rows {
                self.abort_if_job_cancel_requested(job_id).await?;
                let keys = search::build_search_keys(&row.name);
                let affected = sqlx::query(
                    r#"
UPDATE media_items
SET search_text = $2,
    search_pinyin = $3,
    search_initials = $4
WHERE id = $1
                    "#,
                )
                .bind(row.id)
                .bind(keys.text.clone())
                .bind(keys.pinyin.clone())
                .bind(keys.initials.clone())
                .execute(&self.pool)
                .await?
                .rows_affected();

                if self.search_backend.is_some() {
                    docs.push(SearchIndexDocument {
                        id: row.id.to_string(),
                        name: row.name.clone(),
                        name_pinyin: keys.pinyin,
                        name_initials: keys.initials,
                        item_type: row.item_type.clone(),
                        library_id: row.library_id.map(|v| v.to_string()),
                        series_id: row.series_id.map(|v| v.to_string()),
                    });
                }

                processed += 1;
                if affected > 0 {
                    updated += 1;
                }
                if total > 0 && (processed % 25 == 0 || processed == total) {
                    self.set_job_progress(
                        job_id,
                        "reindex_search",
                        total,
                        processed,
                        "重建搜索索引",
                        json!({ "updated": updated, "indexed": indexed }),
                    )
                    .await?;
                }
            }

            if let Some(search_backend) = self.search_backend.as_ref() {
                if !docs.is_empty() {
                    let task = search_backend
                        .index
                        .add_or_replace(&docs, Some("id"))
                        .await?;
                    task.wait_for_completion(&search_backend.client, None, None)
                        .await?;
                    indexed += docs.len() as i64;
                }
            }

            cursor = rows.last().map(|row| row.id);
        }

        // Index people into Meilisearch
        let mut people_indexed = 0_i64;
        if let Some(search_backend) = self.search_backend.as_ref() {
            let people_rows: Vec<SearchIndexRow> = sqlx::query_as(
                "SELECT id, name, 'Person'::text AS item_type, NULL::uuid AS library_id, NULL::uuid AS series_id FROM people ORDER BY id ASC",
            )
            .fetch_all(&self.pool)
            .await?;

            if !people_rows.is_empty() {
                let docs: Vec<SearchIndexDocument> = people_rows
                    .iter()
                    .map(|row| {
                        let keys = search::build_search_keys(&row.name);
                        SearchIndexDocument {
                            id: row.id.to_string(),
                            name: row.name.clone(),
                            name_pinyin: keys.pinyin,
                            name_initials: keys.initials,
                            item_type: "Person".to_string(),
                            library_id: None,
                            series_id: None,
                        }
                    })
                    .collect();

                people_indexed = docs.len() as i64;
                let task = search_backend
                    .index
                    .add_or_replace(&docs, Some("id"))
                    .await?;
                task.wait_for_completion(&search_backend.client, None, None)
                    .await?;
            }
        }

        Ok(json!({
            "library_id": library_id,
            "batch_size": batch_size,
            "processed": processed,
            "updated": updated,
            "indexed": indexed,
            "people_indexed": people_indexed,
        }))
    }

    /// Push recently upserted media items to Meilisearch.
    /// Returns the number of documents indexed.
    pub(crate) async fn update_search_index_since(
        &self,
        library_id: Option<Uuid>,
        since: DateTime<Utc>,
    ) -> anyhow::Result<i64> {
        let search_backend = match self.search_backend.as_ref() {
            Some(sb) => sb,
            None => return Ok(0),
        };

        let rows: Vec<SearchIndexRow> = if let Some(lid) = library_id {
            sqlx::query_as(
                r#"
SELECT id, name, item_type, library_id, series_id
FROM media_items
WHERE library_id = $1 AND updated_at >= $2
ORDER BY id ASC
                "#,
            )
            .bind(lid)
            .bind(since)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as(
                r#"
SELECT id, name, item_type, library_id, series_id
FROM media_items
WHERE updated_at >= $1
ORDER BY id ASC
                "#,
            )
            .bind(since)
            .fetch_all(&self.pool)
            .await?
        };

        let mut docs: Vec<SearchIndexDocument> = rows
            .iter()
            .map(|row| {
                let keys = search::build_search_keys(&row.name);
                SearchIndexDocument {
                    id: row.id.to_string(),
                    name: row.name.clone(),
                    name_pinyin: keys.pinyin,
                    name_initials: keys.initials,
                    item_type: row.item_type.clone(),
                    library_id: row.library_id.map(|v| v.to_string()),
                    series_id: row.series_id.map(|v| v.to_string()),
                }
            })
            .collect();

        // Also index recently updated people
        let people_rows: Vec<SearchIndexRow> = sqlx::query_as(
            "SELECT id, name, 'Person'::text AS item_type, NULL::uuid AS library_id, NULL::uuid AS series_id FROM people WHERE updated_at >= $1 ORDER BY id ASC",
        )
        .bind(since)
        .fetch_all(&self.pool)
        .await?;

        for row in &people_rows {
            let keys = search::build_search_keys(&row.name);
            docs.push(SearchIndexDocument {
                id: row.id.to_string(),
                name: row.name.clone(),
                name_pinyin: keys.pinyin,
                name_initials: keys.initials,
                item_type: "Person".to_string(),
                library_id: None,
                series_id: None,
            });
        }

        if docs.is_empty() {
            return Ok(0);
        }

        let count = docs.len() as i64;
        let task = search_backend
            .index
            .add_or_replace(&docs, Some("id"))
            .await?;
        task.wait_for_completion(&search_backend.client, None, None)
            .await?;

        info!(count, library_id = ?library_id, "incremental search index update after scan");
        Ok(count)
    }

    /// Push recently upserted people to Meilisearch.
    pub(crate) async fn index_people_since(&self, since: DateTime<Utc>) -> anyhow::Result<i64> {
        let search_backend = match self.search_backend.as_ref() {
            Some(sb) => sb,
            None => return Ok(0),
        };

        let rows: Vec<SearchIndexRow> = sqlx::query_as(
            "SELECT id, name, 'Person'::text AS item_type, NULL::uuid AS library_id, NULL::uuid AS series_id FROM people WHERE updated_at >= $1 ORDER BY id ASC",
        )
        .bind(since)
        .fetch_all(&self.pool)
        .await?;

        if rows.is_empty() {
            return Ok(0);
        }

        let docs: Vec<SearchIndexDocument> = rows
            .iter()
            .map(|row| {
                let keys = search::build_search_keys(&row.name);
                SearchIndexDocument {
                    id: row.id.to_string(),
                    name: row.name.clone(),
                    name_pinyin: keys.pinyin,
                    name_initials: keys.initials,
                    item_type: "Person".to_string(),
                    library_id: None,
                    series_id: None,
                }
            })
            .collect();

        let count = docs.len() as i64;
        let task = search_backend
            .index
            .add_or_replace(&docs, Some("id"))
            .await?;
        task.wait_for_completion(&search_backend.client, None, None)
            .await?;

        info!(count, "incremental people search index update");
        Ok(count)
    }

    async fn run_cleanup_maintenance_job(
        &self,
        job_id: Uuid,
        payload: &Value,
    ) -> anyhow::Result<Value> {
        let stale_threshold_seconds = payload
            .get("stale_threshold_seconds")
            .and_then(Value::as_i64)
            .unwrap_or(3600)
            .max(60);
        let retention_days = payload
            .get("retention_days")
            .and_then(Value::as_i64)
            .unwrap_or(30)
            .max(1);
        let media_usage_retention_days = 30_i64;

        let metrics = scheduler::cleanup::CleanupMetrics::default();
        let total_steps = 7_i64;
        let mut completed_steps = 0_i64;
        self.abort_if_job_cancel_requested(job_id).await?;
        let expired_tokens =
            scheduler::cleanup::cleanup_expired_tokens(&self.pool, &metrics).await?;
        completed_steps += 1;
        self.set_job_progress(
            job_id,
            "cleanup_expired_tokens",
            total_steps,
            completed_steps,
            "清理过期 token",
            json!({ "expired_tokens_removed": expired_tokens }),
        )
        .await?;
        self.abort_if_job_cancel_requested(job_id).await?;
        let stale_sessions = scheduler::cleanup::cleanup_stale_sessions(
            &self.pool,
            &metrics,
            stale_threshold_seconds,
        )
        .await?;
        completed_steps += 1;
        self.set_job_progress(
            job_id,
            "cleanup_stale_sessions",
            total_steps,
            completed_steps,
            "清理陈旧会话",
            json!({ "stale_sessions_marked": stale_sessions }),
        )
        .await?;
        self.abort_if_job_cancel_requested(job_id).await?;
        let tmdb_cache = scheduler::cleanup::cleanup_tmdb_cache(&self.pool, &metrics).await?;
        completed_steps += 1;
        self.set_job_progress(
            job_id,
            "cleanup_tmdb_cache",
            total_steps,
            completed_steps,
            "清理 TMDB 缓存",
            json!({ "tmdb_cache_removed": tmdb_cache }),
        )
        .await?;
        self.abort_if_job_cancel_requested(job_id).await?;
        let old_jobs =
            scheduler::cleanup::cleanup_old_jobs(&self.pool, &metrics, retention_days).await?;
        completed_steps += 1;
        self.set_job_progress(
            job_id,
            "cleanup_old_jobs",
            total_steps,
            completed_steps,
            "清理旧任务记录",
            json!({ "old_jobs_removed": old_jobs }),
        )
        .await?;
        self.abort_if_job_cancel_requested(job_id).await?;
        let old_logs =
            scheduler::cleanup::cleanup_old_logs(&self.pool, &metrics, retention_days).await?;
        completed_steps += 1;
        self.set_job_progress(
            job_id,
            "cleanup_old_logs",
            total_steps,
            completed_steps,
            "清理旧日志",
            json!({ "old_logs_removed": old_logs }),
        )
        .await?;
        self.abort_if_job_cancel_requested(job_id).await?;
        let old_media_usage = scheduler::cleanup::cleanup_old_user_stream_media_usage(
            &self.pool,
            &metrics,
            media_usage_retention_days,
        )
        .await?;
        completed_steps += 1;
        self.set_job_progress(
            job_id,
            "cleanup_old_user_stream_media_usage",
            total_steps,
            completed_steps,
            "清理旧媒体流量记录",
            json!({
                "old_user_stream_media_usage_removed": old_media_usage,
                "retention_days": media_usage_retention_days,
            }),
        )
        .await?;
        self.abort_if_job_cancel_requested(job_id).await?;
        let orphaned_media =
            scheduler::cleanup::cleanup_orphaned_media_items(&self.pool, &metrics).await?;
        completed_steps += 1;
        self.set_job_progress(
            job_id,
            "cleanup_orphaned_media",
            total_steps,
            completed_steps,
            "清理孤儿媒体项",
            json!({ "orphaned_media_items_removed": orphaned_media }),
        )
        .await?;

        let snapshot = metrics.snapshot();
        Ok(json!({
            "stale_threshold_seconds": stale_threshold_seconds,
            "retention_days": retention_days,
            "expired_tokens_removed": expired_tokens,
            "stale_sessions_marked": stale_sessions,
            "tmdb_cache_removed": tmdb_cache,
            "old_jobs_removed": old_jobs,
            "old_logs_removed": old_logs,
            "old_user_stream_media_usage_removed": old_media_usage,
            "media_usage_retention_days": media_usage_retention_days,
            "orphaned_media_items_removed": orphaned_media,
            "metrics_snapshot": {
                "expired_tokens_removed": snapshot.expired_tokens_removed,
                "stale_sessions_marked": snapshot.stale_sessions_marked,
                "tmdb_cache_removed": snapshot.tmdb_cache_removed,
                "old_jobs_removed": snapshot.old_jobs_removed,
                "old_audit_logs_removed": snapshot.old_audit_logs_removed,
                "old_system_events_removed": snapshot.old_system_events_removed,
                "old_user_stream_media_usage_removed": snapshot.old_user_stream_media_usage_removed,
                "orphaned_media_items_removed": snapshot.orphaned_media_items_removed,
            }
        }))
    }

    async fn run_retry_dispatch_job(&self, job_id: Uuid, payload: &Value) -> anyhow::Result<Value> {
        let limit = payload
            .get("limit")
            .and_then(Value::as_i64)
            .unwrap_or(100)
            .clamp(1, 1000);
        let job_ids = scheduler::job_retry::get_retry_eligible_jobs(&self.pool, limit).await?;
        let jobs_found = job_ids.len() as u64;
        let mut jobs_dispatched = 0_u64;
        let mut retry_failures = 0_u64;
        let mut skipped_recursive = 0_u64;
        if jobs_found == 0 {
            self.set_job_progress(job_id, "dispatch_retry", 1, 1, "无可重试任务", json!({}))
                .await?;
        }

        for retry_job_id in job_ids {
            self.abort_if_job_cancel_requested(job_id).await?;
            let Some(retry_job) = self.get_job(retry_job_id).await? else {
                continue;
            };

            if retry_job.kind == "retry_dispatch" {
                skipped_recursive += 1;
                let handled = jobs_dispatched + retry_failures + skipped_recursive;
                self.set_job_progress(
                    job_id,
                    "dispatch_retry",
                    jobs_found as i64,
                    handled as i64,
                    "分发重试任务",
                    json!({
                        "jobs_dispatched": jobs_dispatched,
                        "retry_failures": retry_failures,
                        "skipped_recursive": skipped_recursive
                    }),
                )
                .await?;
                continue;
            }

            match Box::pin(self.process_job(retry_job_id)).await {
                Ok(()) => jobs_dispatched += 1,
                Err(err) => {
                    retry_failures += 1;
                    warn!(error = %err, retry_job_id = %retry_job_id, "retry-dispatch processing failed");
                }
            }
            let handled = jobs_dispatched + retry_failures + skipped_recursive;
            self.set_job_progress(
                job_id,
                "dispatch_retry",
                jobs_found as i64,
                handled as i64,
                "分发重试任务",
                json!({
                    "jobs_dispatched": jobs_dispatched,
                    "retry_failures": retry_failures,
                    "skipped_recursive": skipped_recursive
                }),
            )
            .await?;
        }

        Ok(json!({
            "limit": limit,
            "jobs_found": jobs_found,
            "jobs_dispatched": jobs_dispatched,
            "retry_failures": retry_failures,
            "skipped_recursive": skipped_recursive,
        }))
    }

    async fn run_billing_expire_job(
        &self,
        job_id: Uuid,
        _payload: &Value,
    ) -> anyhow::Result<Value> {
        self.set_job_progress(job_id, "expire_orders", 2, 0, "处理过期计费订单", json!({}))
            .await?;
        self.abort_if_job_cancel_requested(job_id).await?;
        let expired_orders_result = scheduler::billing::expire_pending_orders(&self.pool).await?;
        let expired_orders = expired_orders_result.expired_count;
        for expired in expired_orders_result.expired_recharge_orders {
            if let Some(order) = self.get_recharge_order_by_id(expired.id).await? {
                self.publish_recharge_order_event("billing.recharge_order.updated", order);
            }
        }
        self.set_job_progress(
            job_id,
            "expire_orders",
            2,
            1,
            "处理过期计费订单",
            json!({ "expired_orders": expired_orders }),
        )
        .await?;
        self.abort_if_job_cancel_requested(job_id).await?;
        let expired_subscriptions = scheduler::billing::expire_subscriptions(&self.pool)
            .await?
            .expired_count;
        self.set_job_progress(
            job_id,
            "expire_subscriptions",
            2,
            2,
            "处理过期订阅",
            json!({ "expired_subscriptions": expired_subscriptions }),
        )
        .await?;

        Ok(json!({
            "expired_orders": expired_orders,
            "expired_subscriptions": expired_subscriptions,
        }))
    }

    async fn finish_job_error(
        &self,
        job_id: Uuid,
        job: &Job,
        error_message: &str,
    ) -> anyhow::Result<()> {
        self.metrics
            .jobs_failure_total
            .fetch_add(1, Ordering::Relaxed);
        let reached_dead_letter = job.attempts + 1 >= job.max_attempts;

        if reached_dead_letter {
            sqlx::query(
                "UPDATE jobs SET status = $1, error = $2, finished_at = now(), dead_letter = true, next_retry_at = NULL, progress = $4 WHERE id = $3",
            )
            .bind("failed")
            .bind(error_message)
            .bind(job_id)
            .bind(Self::make_progress(
                "failed",
                1,
                1,
                "任务失败并进入死信",
                json!({ "error": error_message }),
            ))
            .execute(&self.pool)
            .await?;

            sqlx::query(
                "INSERT INTO job_dead_letters (id, job_id, reason, payload) VALUES ($1, $2, $3, $4)",
            )
            .bind(Uuid::now_v7())
            .bind(job_id)
            .bind(error_message)
            .bind(job.payload.clone())
            .execute(&self.pool)
            .await?;

            self.metrics
                .jobs_dead_letter_total
                .fetch_add(1, Ordering::Relaxed);
            self.emit_task_run_event_by_id("task_run.finished", job_id)
                .await?;
        } else {
            let delay = calc_retry_delay_seconds(
                self.config_snapshot().jobs.retry_base_seconds,
                self.config_snapshot().jobs.retry_max_seconds,
                job.attempts,
            );
            let next_retry = Utc::now() + Duration::seconds(delay);

            sqlx::query(
                "UPDATE jobs SET status = $1, error = $2, finished_at = now(), next_retry_at = $3, progress = $5 WHERE id = $4",
            )
            .bind("queued")
            .bind(error_message)
            .bind(next_retry)
            .bind(job_id)
            .bind(Self::make_progress(
                "retry_pending",
                1,
                0,
                "任务失败，等待重试",
                json!({ "error": error_message, "next_retry_at": next_retry.to_rfc3339() }),
            ))
            .execute(&self.pool)
            .await?;

            self.metrics
                .jobs_retry_scheduled_total
                .fetch_add(1, Ordering::Relaxed);
            self.emit_task_run_event_by_id("task_run.progress", job_id)
                .await?;
        }

        Ok(())
    }

    async fn find_items_for_tmdb_fill(
        &self,
        library_id: Option<Uuid>,
        new_since: Option<DateTime<Utc>>,
        new_only: bool,
    ) -> anyhow::Result<Vec<TmdbFillItemRow>> {
        let rows = match (library_id, new_since) {
            (Some(library_id), Some(new_since)) => {
                sqlx::query_as::<_, TmdbFillItemRow>(
                    r#"
SELECT id, library_id, name, item_type, path, season_number, episode_number, metadata
FROM media_items
WHERE library_id = $1
  AND item_type IN ('Movie', 'Episode', 'Series')
  AND updated_at >= $2
ORDER BY updated_at DESC
LIMIT 2000
                    "#,
                )
                .bind(library_id)
                .bind(new_since)
                .fetch_all(&self.pool)
                .await?
            }
            (Some(library_id), None) => {
                sqlx::query_as::<_, TmdbFillItemRow>(
                    r#"
SELECT id, library_id, name, item_type, path, season_number, episode_number, metadata
FROM media_items
WHERE library_id = $1
  AND item_type IN ('Movie', 'Episode', 'Series')
ORDER BY
  CASE
    WHEN item_type = 'Movie'
      AND metadata->>'tmdb_id' IS NOT NULL
      AND metadata->'nfo'->>'title' IS NOT NULL
      AND COALESCE(trim(metadata->'nfo'->>'title'), '') <> ''
      AND (metadata->'tmdb_raw'->>'title' IS NOT NULL OR metadata->'tmdb_raw'->>'original_title' IS NOT NULL)
      AND lower(regexp_replace(metadata->'nfo'->>'title', '[[:space:][:punct:]]', '', 'g'))
          <> lower(regexp_replace(COALESCE(metadata->'tmdb_raw'->>'title', metadata->'tmdb_raw'->>'original_title'), '[[:space:][:punct:]]', '', 'g'))
      AND (
            NULLIF(regexp_replace(metadata->'nfo'->>'year', '[^0-9]', '', 'g'), '') IS NULL
            OR NULLIF(substring(COALESCE(metadata->'tmdb_raw'->>'release_date', '') FROM 1 FOR 4), '') IS NULL
            OR abs(
                COALESCE(NULLIF(regexp_replace(metadata->'nfo'->>'year', '[^0-9]', '', 'g'), '')::int, -1)
                - COALESCE(NULLIF(substring(COALESCE(metadata->'tmdb_raw'->>'release_date', '') FROM 1 FOR 4), '')::int, -1)
            ) >= 2
      )
      THEN 0
    WHEN metadata->>'tmdb_id' IS NULL THEN 1
    ELSE 2
  END ASC,
  updated_at DESC
LIMIT 4000
                    "#,
                )
                .bind(library_id)
                .fetch_all(&self.pool)
                .await?
            }
            (None, Some(new_since)) => {
                sqlx::query_as::<_, TmdbFillItemRow>(
                    r#"
SELECT id, library_id, name, item_type, path, season_number, episode_number, metadata
FROM media_items
WHERE updated_at >= $1
  AND item_type IN ('Movie', 'Episode', 'Series')
ORDER BY updated_at DESC
LIMIT 2000
                    "#,
                )
                .bind(new_since)
                .fetch_all(&self.pool)
                .await?
            }
            (None, None) => {
                sqlx::query_as::<_, TmdbFillItemRow>(
                    r#"
SELECT id, library_id, name, item_type, path, season_number, episode_number, metadata
FROM media_items
WHERE item_type IN ('Movie', 'Episode', 'Series')
ORDER BY
  CASE
    WHEN item_type = 'Movie'
      AND metadata->>'tmdb_id' IS NOT NULL
      AND metadata->'nfo'->>'title' IS NOT NULL
      AND COALESCE(trim(metadata->'nfo'->>'title'), '') <> ''
      AND (metadata->'tmdb_raw'->>'title' IS NOT NULL OR metadata->'tmdb_raw'->>'original_title' IS NOT NULL)
      AND lower(regexp_replace(metadata->'nfo'->>'title', '[[:space:][:punct:]]', '', 'g'))
          <> lower(regexp_replace(COALESCE(metadata->'tmdb_raw'->>'title', metadata->'tmdb_raw'->>'original_title'), '[[:space:][:punct:]]', '', 'g'))
      AND (
            NULLIF(regexp_replace(metadata->'nfo'->>'year', '[^0-9]', '', 'g'), '') IS NULL
            OR NULLIF(substring(COALESCE(metadata->'tmdb_raw'->>'release_date', '') FROM 1 FOR 4), '') IS NULL
            OR abs(
                COALESCE(NULLIF(regexp_replace(metadata->'nfo'->>'year', '[^0-9]', '', 'g'), '')::int, -1)
                - COALESCE(NULLIF(substring(COALESCE(metadata->'tmdb_raw'->>'release_date', '') FROM 1 FOR 4), '')::int, -1)
            ) >= 2
      )
      THEN 0
    WHEN metadata->>'tmdb_id' IS NULL THEN 1
    ELSE 2
  END ASC,
  updated_at DESC
LIMIT 4000
                    "#,
                )
                .fetch_all(&self.pool)
                .await?
            }
        };

        if new_only && new_since.is_none() {
            return Ok(Vec::new());
        }

        if new_since.is_some() {
            return Ok(rows);
        }

        Ok(rows
            .into_iter()
            .filter(|row| should_fill_tmdb_for_item(&row.item_type, &row.path, &row.metadata))
            .collect())
    }

    async fn is_auth_blocked(
        &self,
        remote_addr: Option<&str>,
        username: &str,
    ) -> anyhow::Result<bool> {
        let window_seconds = self.config_snapshot().auth.risk_window_seconds.max(10);
        let max_failed = self.config_snapshot().auth.max_failed_attempts.max(1);

        let count: i64 = sqlx::query_scalar(
            r#"
SELECT count(*)
FROM auth_risk_events
WHERE created_at >= now() - make_interval(secs => $1)
  AND reason IN (
      'invalid_password',
      'invalid_username',
      'blocked_by_risk_window',
      'legacy_password_hash_rejected'
  )
  AND (
      username = $2
      OR (remote_addr IS NOT NULL AND remote_addr = $3)
  )
            "#,
        )
        .bind(window_seconds as i32)
        .bind(username)
        .bind(remote_addr)
        .fetch_one(&self.pool)
        .await?;

        Ok(count >= i64::from(max_failed))
    }

    async fn record_auth_risk_event(
        &self,
        remote_addr: Option<&str>,
        username: &str,
        reason: &str,
        detail: Value,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
INSERT INTO auth_risk_events (id, remote_addr, username, reason, detail)
VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(remote_addr)
        .bind(username)
        .bind(reason)
        .bind(detail)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

}
