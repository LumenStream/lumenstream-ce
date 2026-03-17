const CLAIM_READY_JOB_SQL: &str = r#"
UPDATE jobs
SET status = $1,
    started_at = now(),
    error = NULL,
    attempts = attempts + 1,
    progress = $3
WHERE id = $2
  AND dead_letter = false
  AND status IN ('queued', 'pending')
  AND (next_retry_at IS NULL OR next_retry_at <= now())
  AND (scheduled_for IS NULL OR scheduled_for <= now())
  AND NOT EXISTS (
      SELECT 1
      FROM jobs j2
      WHERE j2.kind = $4
        AND j2.status = 'running'
        AND j2.id <> $2
  )
"#;

const REQUEUE_ORPHANED_RUNNING_JOBS_SQL: &str = r#"
UPDATE jobs
SET status = 'queued',
    cancel_requested = false,
    started_at = NULL,
    finished_at = NULL,
    error = NULL,
    next_retry_at = NULL,
    progress = $1
WHERE status = 'running'
  AND dead_letter = false
"#;

const JOB_CANCELLED_ERROR: &str = "__job_cancelled__";

impl AppInfra {
    pub fn subscribe_task_runs(&self) -> broadcast::Receiver<TaskRunEvent> {
        self.task_run_tx.subscribe()
    }

    fn publish_task_run_event(&self, event: &str, run: Job) {
        let _ = self.task_run_tx.send(TaskRunEvent {
            event: event.to_string(),
            run,
            emitted_at: Utc::now(),
        });
    }

    async fn emit_task_run_event_by_id(&self, event: &str, job_id: Uuid) -> anyhow::Result<()> {
        if let Some(run) = self.get_job(job_id).await? {
            self.publish_task_run_event(event, run);
        }
        Ok(())
    }

    fn make_progress(
        phase: &str,
        total: i64,
        completed: i64,
        message: &str,
        detail: Value,
    ) -> Value {
        let total = total.max(0);
        let completed = completed.clamp(0, total.max(1));
        let percent = if total == 0 {
            0.0
        } else {
            (completed as f64 / total as f64) * 100.0
        };

        json!({
            "phase": phase,
            "total": total,
            "completed": completed,
            "percent": percent.clamp(0.0, 100.0),
            "message": message,
            "detail": detail,
            "updated_at": Utc::now().to_rfc3339(),
        })
    }

    fn queued_progress(kind: &str) -> Value {
        Self::make_progress("queued", 0, 0, "任务已排队", json!({ "kind": kind }))
    }

    pub async fn set_job_progress(
        &self,
        job_id: Uuid,
        phase: &str,
        total: i64,
        completed: i64,
        message: &str,
        detail: Value,
    ) -> anyhow::Result<()> {
        let progress = Self::make_progress(phase, total, completed, message, detail);
        sqlx::query("UPDATE jobs SET progress = $1 WHERE id = $2")
            .bind(progress)
            .bind(job_id)
            .execute(&self.pool)
            .await?;
        self.emit_task_run_event_by_id("task_run.progress", job_id).await
    }

    async fn is_job_cancel_requested(&self, job_id: Uuid) -> anyhow::Result<bool> {
        let requested: Option<bool> =
            sqlx::query_scalar("SELECT cancel_requested FROM jobs WHERE id = $1 LIMIT 1")
                .bind(job_id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(requested.unwrap_or(false))
    }

    async fn abort_if_job_cancel_requested(&self, job_id: Uuid) -> anyhow::Result<()> {
        if self.is_job_cancel_requested(job_id).await? {
            anyhow::bail!(JOB_CANCELLED_ERROR);
        }
        Ok(())
    }

    fn is_cancelled_error(err: &anyhow::Error) -> bool {
        err.to_string().contains(JOB_CANCELLED_ERROR)
    }

    async fn mark_job_cancelled(&self, job_id: Uuid, message: &str) -> anyhow::Result<()> {
        sqlx::query(
            "UPDATE jobs SET status = 'cancelled', finished_at = now(), error = NULL, next_retry_at = NULL, progress = $2 WHERE id = $1",
        )
        .bind(job_id)
        .bind(Self::make_progress(
            "cancelled",
            1,
            1,
            message,
            json!({}),
        ))
        .execute(&self.pool)
        .await?;
        self.emit_task_run_event_by_id("task_run.finished", job_id).await
    }

    pub async fn cancel_task_run(&self, run_id: Uuid) -> anyhow::Result<Option<Job>> {
        let Some(current) = self.get_job(run_id).await? else {
            return Ok(None);
        };

        let status = current.status.as_str();
        if matches!(status, "completed" | "failed" | "cancelled") {
            return Ok(Some(current));
        }

        match status {
            "queued" | "pending" => {
                sqlx::query(
                    "UPDATE jobs SET cancel_requested = true, status = 'cancelled', finished_at = now(), error = NULL, next_retry_at = NULL, progress = $2 WHERE id = $1",
                )
                .bind(run_id)
                .bind(Self::make_progress(
                    "cancelled",
                    1,
                    1,
                    "任务已取消（排队中）",
                    json!({}),
                ))
                .execute(&self.pool)
                .await?;
                self.emit_task_run_event_by_id("task_run.finished", run_id)
                    .await?;
            }
            "running" => {
                sqlx::query(
                    "UPDATE jobs SET cancel_requested = true, progress = $2 WHERE id = $1",
                )
                .bind(run_id)
                .bind(Self::make_progress(
                    "cancelling",
                    1,
                    0,
                    "正在取消任务",
                    json!({}),
                ))
                .execute(&self.pool)
                .await?;
                self.emit_task_run_event_by_id("task_run.progress", run_id)
                    .await?;
            }
            _ => {}
        }

        self.get_job(run_id).await
    }

    pub async fn enqueue_scan_job(
        &self,
        library_id: Uuid,
        mode: Option<&str>,
        path_prefix: Option<&str>,
    ) -> anyhow::Result<Job> {
        self.enqueue_scan_job_for_kind("scan_library", library_id, mode, path_prefix, None)
            .await
    }

    async fn enqueue_scan_job_for_kind(
        &self,
        kind: &str,
        library_id: Uuid,
        mode: Option<&str>,
        path_prefix: Option<&str>,
        probe_mediainfo: Option<bool>,
    ) -> anyhow::Result<Job> {
        let mode = mode.unwrap_or("incremental");
        let path_prefix = path_prefix.map(str::trim).filter(|v| !v.is_empty());

        // Deduplicate high-frequency incremental full-library scan tasks to avoid queue storms.
        if path_prefix.is_none() && mode.eq_ignore_ascii_case("incremental") {
            let existing_job_id: Option<Uuid> = sqlx::query_scalar(
                r#"
SELECT id
FROM jobs
WHERE kind = $1
  AND status IN ('queued', 'pending', 'running')
  AND payload->>'library_id' = $2::text
  AND COALESCE(payload->>'path_prefix', '') = ''
  AND lower(COALESCE(payload->>'mode', 'incremental')) = 'incremental'
ORDER BY created_at ASC
LIMIT 1
                "#,
            )
            .bind(kind)
            .bind(library_id)
            .fetch_optional(&self.pool)
            .await?;

            if let Some(job_id) = existing_job_id {
                if let Some(job) = self.get_job(job_id).await? {
                    return Ok(job);
                }
            }
        }

        self.enqueue_job(
            kind,
            json!({
                "library_id": library_id,
                "mode": mode,
                "path_prefix": path_prefix,
                "probe_mediainfo": probe_mediainfo.unwrap_or(false),
            }),
            3,
        )
        .await
    }

    pub async fn list_task_definitions(&self) -> anyhow::Result<Vec<TaskDefinition>> {
        let rows = sqlx::query_as::<_, TaskDefinitionRow>(
            r#"
SELECT
    task_key,
    display_name,
    enabled,
    cron_expr,
    default_payload,
    max_attempts,
    created_at,
    updated_at
FROM task_definitions
ORDER BY task_key ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn get_task_definition(
        &self,
        task_key: &str,
    ) -> anyhow::Result<Option<TaskDefinition>> {
        let row = sqlx::query_as::<_, TaskDefinitionRow>(
            r#"
SELECT
    task_key,
    display_name,
    enabled,
    cron_expr,
    default_payload,
    max_attempts,
    created_at,
    updated_at
FROM task_definitions
WHERE task_key = $1
LIMIT 1
            "#,
        )
        .bind(task_key)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Into::into))
    }

    pub async fn update_task_definition(
        &self,
        task_key: &str,
        patch: TaskDefinitionUpdate,
    ) -> anyhow::Result<Option<TaskDefinition>> {
        if patch.enabled.is_none()
            && patch.cron_expr.is_none()
            && patch.default_payload.is_none()
            && patch.max_attempts.is_none()
        {
            return self.get_task_definition(task_key).await;
        }

        if let Some(payload) = patch.default_payload.as_ref() {
            if !payload.is_object() {
                anyhow::bail!("default_payload must be a JSON object");
            }
        }

        let mut query = QueryBuilder::<Postgres>::new("UPDATE task_definitions SET ");
        push_task_definition_assignments(&mut query, patch);

        query.push(" WHERE task_key = ").push_bind(task_key).push(
            r#"
 RETURNING
    task_key,
    display_name,
    enabled,
    cron_expr,
    default_payload,
    max_attempts,
    created_at,
    updated_at
                "#,
        );

        let row = query
            .build_query_as::<TaskDefinitionRow>()
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(Into::into))
    }

    pub async fn run_task_now(
        &self,
        task_key: &str,
        payload_override: Option<Value>,
    ) -> anyhow::Result<Option<Job>> {
        let Some(task) = self.get_task_definition(task_key).await? else {
            return Ok(None);
        };

        if self.has_active_task_run(&task.task_key).await? {
            return Err(anyhow::Error::new(InfraError::TaskRunAlreadyActive));
        }

        let payload = merge_task_payload(&task.default_payload, payload_override.as_ref())?;
        let job = self
            .enqueue_task_run(&task.task_key, payload, task.max_attempts, "manual", None)
            .await?;
        Ok(Some(job))
    }

    pub async fn enqueue_scheduled_task_run(
        &self,
        task: &TaskDefinition,
        scheduled_for: DateTime<Utc>,
    ) -> anyhow::Result<Option<Job>> {
        if self.has_active_task_run(&task.task_key).await? {
            return Ok(None);
        }

        let existing: Option<Uuid> = sqlx::query_scalar(
            r#"
SELECT id
FROM jobs
WHERE kind = $1
  AND trigger_type = 'scheduled'
  AND scheduled_for = $2
LIMIT 1
            "#,
        )
        .bind(&task.task_key)
        .bind(scheduled_for)
        .fetch_optional(&self.pool)
        .await?;

        if existing.is_some() {
            return Ok(None);
        }

        let job = self
            .enqueue_task_run(
                &task.task_key,
                task.default_payload.clone(),
                task.max_attempts,
                "scheduled",
                Some(scheduled_for),
            )
            .await?;
        Ok(Some(job))
    }

    pub async fn has_active_task_run(&self, task_key: &str) -> anyhow::Result<bool> {
        let active: Option<bool> = sqlx::query_scalar(
            r#"
SELECT true
FROM jobs
WHERE kind = $1
  AND status IN ('pending', 'queued', 'running')
LIMIT 1
            "#,
        )
        .bind(task_key)
        .fetch_optional(&self.pool)
        .await?;
        Ok(active.unwrap_or(false))
    }

    pub async fn enqueue_metadata_repair_job(
        &self,
        library_id: Option<Uuid>,
    ) -> anyhow::Result<Job> {
        self.enqueue_job("metadata_repair", json!({ "library_id": library_id }), 3)
            .await
    }

    pub async fn enqueue_subtitle_sync_job(&self, library_id: Option<Uuid>) -> anyhow::Result<Job> {
        self.enqueue_job(
            "subtitle_sync",
            json!({ "library_id": library_id, "mode": "incremental" }),
            3,
        )
        .await
    }

    pub async fn enqueue_scrape_fill_job(&self, library_id: Option<Uuid>) -> anyhow::Result<Job> {
        self.enqueue_job("scraper_fill", json!({ "library_id": library_id }), 3)
            .await
    }

    pub async fn enqueue_scrape_fill_job_for_new_items(
        &self,
        library_id: Option<Uuid>,
        new_since: DateTime<Utc>,
    ) -> anyhow::Result<Job> {
        self.enqueue_job(
            "scraper_fill",
            json!({
                "library_id": library_id,
                "new_only": true,
                "new_since": new_since.to_rfc3339(),
            }),
            3,
        )
        .await
    }

    pub async fn enqueue_tmdb_fill_job(&self, library_id: Option<Uuid>) -> anyhow::Result<Job> {
        self.enqueue_scrape_fill_job(library_id).await
    }

    pub async fn enqueue_tmdb_fill_job_for_new_items(
        &self,
        library_id: Option<Uuid>,
        new_since: DateTime<Utc>,
    ) -> anyhow::Result<Job> {
        self.enqueue_scrape_fill_job_for_new_items(library_id, new_since)
            .await
    }

    pub async fn enqueue_search_reindex_job(
        &self,
        library_id: Option<Uuid>,
        batch_size: i64,
    ) -> anyhow::Result<Job> {
        self.enqueue_job(
            "search_reindex",
            json!({
                "library_id": library_id,
                "batch_size": batch_size,
            }),
            3,
        )
        .await
    }

    async fn enqueue_job(
        &self,
        kind: &str,
        payload: Value,
        max_attempts: i32,
    ) -> anyhow::Result<Job> {
        self.enqueue_task_run(kind, payload, max_attempts, "manual", None)
            .await
    }

    async fn enqueue_task_run(
        &self,
        kind: &str,
        payload: Value,
        max_attempts: i32,
        trigger_type: &str,
        scheduled_for: Option<DateTime<Utc>>,
    ) -> anyhow::Result<Job> {
        let job_id = Uuid::now_v7();

        sqlx::query(
            r#"
INSERT INTO jobs (id, kind, status, payload, attempts, max_attempts, dead_letter, trigger_type, scheduled_for, progress)
VALUES ($1, $2, $3, $4, 0, $5, false, $6, $7, $8)
            "#,
        )
        .bind(job_id)
        .bind(kind)
        .bind("queued")
        .bind(payload)
        .bind(max_attempts.clamp(1, 20))
        .bind(trigger_type)
        .bind(scheduled_for)
        .bind(Self::queued_progress(kind))
        .execute(&self.pool)
        .await?;

        let job = self
            .get_job(job_id)
            .await?
            .context("newly created job not found")?;
        self.publish_task_run_event("task_run.created", job.clone());
        Ok(job)
    }

    pub async fn get_job(&self, job_id: Uuid) -> anyhow::Result<Option<Job>> {
        let row = sqlx::query_as::<_, JobRow>(
            r#"
SELECT
    id,
    kind,
    status,
    payload,
    progress,
    result,
    error,
    attempts,
    max_attempts,
    next_retry_at,
    cancel_requested,
    dead_letter,
    trigger_type,
    scheduled_for,
    created_at,
    started_at,
    finished_at
FROM jobs
WHERE id = $1
LIMIT 1
            "#,
        )
        .bind(job_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Into::into))
    }

    pub async fn list_jobs(&self, limit: i64) -> anyhow::Result<Vec<Job>> {
        self.list_task_runs(limit, None, None, None, &[]).await
    }

    pub async fn list_task_runs(
        &self,
        limit: i64,
        task_key: Option<&str>,
        status: Option<&str>,
        trigger_type: Option<&str>,
        exclude_kinds: &[&str],
    ) -> anyhow::Result<Vec<Job>> {
        let mut query = QueryBuilder::<Postgres>::new(
            r#"
SELECT
    id,
    kind,
    status,
    payload,
    progress,
    result,
    error,
    attempts,
    max_attempts,
    next_retry_at,
    cancel_requested,
    dead_letter,
    trigger_type,
    scheduled_for,
    created_at,
    started_at,
    finished_at
FROM jobs
            "#,
        );
        let mut has_where = false;

        if let Some(task_key) = task_key {
            query
                .push(if has_where { " AND " } else { " WHERE " })
                .push("kind = ")
                .push_bind(task_key);
            has_where = true;
        }

        if let Some(status) = status {
            query
                .push(if has_where { " AND " } else { " WHERE " })
                .push("status = ")
                .push_bind(status);
            has_where = true;
        }

        if let Some(trigger_type) = trigger_type {
            query
                .push(if has_where { " AND " } else { " WHERE " })
                .push("trigger_type = ")
                .push_bind(trigger_type);
            has_where = true;
        }

        if !exclude_kinds.is_empty() {
            query.push(if has_where { " AND " } else { " WHERE " });
            query.push("kind NOT IN (");
            let mut sep = query.separated(", ");
            for kind in exclude_kinds {
                sep.push_bind(*kind);
            }
            sep.push_unseparated(")");
        }

        query
            .push(" ORDER BY created_at DESC LIMIT ")
            .push_bind(limit.clamp(1, 1000));

        let rows = query
            .build_query_as::<JobRow>()
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn get_task_run(&self, run_id: Uuid) -> anyhow::Result<Option<Job>> {
        self.get_job(run_id).await
    }

    pub async fn list_jobs_legacy(&self, limit: i64) -> anyhow::Result<Vec<Job>> {
        let rows = sqlx::query_as::<_, JobRow>(
            r#"
SELECT
    id,
    kind,
    status,
    payload,
    progress,
    result,
    error,
    attempts,
    max_attempts,
    next_retry_at,
    cancel_requested,
    dead_letter,
    trigger_type,
    scheduled_for,
    created_at,
    started_at,
    finished_at
FROM jobs
ORDER BY created_at DESC
LIMIT $1
            "#,
        )
        .bind(limit.clamp(1, 1000))
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn list_job_status_counts(&self) -> anyhow::Result<Vec<JobStatusCount>> {
        let rows = sqlx::query_as::<_, JobStatusCountRow>(
            r#"
SELECT status, COUNT(*)::BIGINT AS count
FROM jobs
GROUP BY status
ORDER BY status ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn retry_job(&self, job_id: Uuid) -> anyhow::Result<bool> {
        let affected = sqlx::query(
            "UPDATE jobs SET status = 'queued', next_retry_at = NULL, finished_at = NULL, error = NULL, dead_letter = false, cancel_requested = false, progress = $2 WHERE id = $1",
        )
        .bind(job_id)
        .bind(Self::queued_progress("manual_retry"))
        .execute(&self.pool)
        .await?
        .rows_affected();

        if affected > 0 {
            self.emit_task_run_event_by_id("task_run.progress", job_id)
                .await?;
        }

        Ok(affected > 0)
    }

    pub async fn requeue_orphaned_running_jobs(&self) -> anyhow::Result<u64> {
        let affected = sqlx::query(REQUEUE_ORPHANED_RUNNING_JOBS_SQL)
            .bind(Self::make_progress(
                "queued",
                0,
                0,
                "服务启动恢复：任务重新入队",
                json!({ "reason": "startup_recovery" }),
            ))
            .execute(&self.pool)
            .await?
            .rows_affected();

        if affected > 0 {
            info!(recovered_jobs = affected, "requeued orphaned running jobs");
        }

        Ok(affected)
    }

    pub async fn process_scan_job(&self, job_id: Uuid) -> anyhow::Result<()> {
        self.process_job(job_id).await
    }

    pub async fn process_job(&self, job_id: Uuid) -> anyhow::Result<()> {
        let job = match self.get_job(job_id).await? {
            Some(v) => v,
            None => return Ok(()),
        };

        if job.dead_letter {
            return Ok(());
        }

        let claimed = sqlx::query(CLAIM_READY_JOB_SQL)
            .bind("running")
            .bind(job_id)
            .bind(Self::make_progress(
                "running",
                1,
                0,
                "任务运行中",
                json!({ "kind": job.kind.clone() }),
            ))
            .bind(job.kind.clone())
            .execute(&self.pool)
            .await?
            .rows_affected();
        if claimed == 0 {
            return Ok(());
        }
        self.emit_task_run_event_by_id("task_run.started", job_id).await?;

        self.abort_if_job_cancel_requested(job_id).await?;

        let run_result = match job.kind.as_str() {
            "scan_library" => {
                self.run_scan_job(job_id, &job.payload, job.trigger_type.as_deref())
                    .await
            }
            "scan_library_no_probe" => {
                self.run_scan_no_probe_job(job_id, &job.payload, job.trigger_type.as_deref())
                    .await
            }
            "metadata_repair" => self.run_metadata_repair_job(job_id, &job.payload).await,
            "subtitle_sync" => self.run_subtitle_sync_job(job_id, &job.payload).await,
            "scraper_fill" | "tmdb_fill" => self.run_scrape_fill_job(job_id, &job.payload).await,
            "search_reindex" => self.run_search_reindex_job(job_id, &job.payload).await,
            "cleanup_maintenance" => self.run_cleanup_maintenance_job(job_id, &job.payload).await,
            "retry_dispatch" => self.run_retry_dispatch_job(job_id, &job.payload).await,
            "billing_expire" => self.run_billing_expire_job(job_id, &job.payload).await,
            "agent_request_process" => self.run_agent_request_job(job_id, &job.payload).await,
            "agent_missing_scan" => self.run_agent_missing_scan_job(job_id, &job.payload).await,
            other => Err(anyhow::anyhow!("unknown job kind: {other}")),
        };

        match run_result {
            Ok(result) => {
                sqlx::query(
                    "UPDATE jobs SET status = $1, result = $2, finished_at = now(), error = NULL, next_retry_at = NULL, progress = $4 WHERE id = $3",
                )
                .bind("completed")
                .bind(result)
                .bind(job_id)
                .bind(Self::make_progress("finished", 1, 1, "任务完成", json!({})))
                .execute(&self.pool)
                .await?;
                self.metrics
                    .jobs_success_total
                    .fetch_add(1, Ordering::Relaxed);
                self.emit_task_run_event_by_id("task_run.finished", job_id)
                    .await?;
                Ok(())
            }
            Err(err) => {
                if Self::is_cancelled_error(&err) {
                    return self.mark_job_cancelled(job_id, "任务已取消").await;
                }
                error!(error = %err, %job_id, "job failed");
                self.finish_job_error(job_id, &job, &err.to_string()).await
            }
        }
    }

}

fn push_task_definition_assignments(query: &mut QueryBuilder<Postgres>, patch: TaskDefinitionUpdate) {
    let mut separated = query.separated(", ");

    if let Some(enabled) = patch.enabled {
        separated.push("enabled = ").push_bind_unseparated(enabled);
    }
    if let Some(cron_expr) = patch.cron_expr {
        separated
            .push("cron_expr = ")
            .push_bind_unseparated(cron_expr);
    }
    if let Some(default_payload) = patch.default_payload {
        separated
            .push("default_payload = ")
            .push_bind_unseparated(default_payload);
    }
    if let Some(max_attempts) = patch.max_attempts {
        separated
            .push("max_attempts = ")
            .push_bind_unseparated(max_attempts.clamp(1, 20));
    }
    separated.push("updated_at = now()");
}

#[cfg(test)]
mod app_jobs_queue_tests {
    use super::*;
    use sqlx::Execute;

    #[test]
    fn task_definition_assignments_do_not_insert_separator_before_bind() {
        let patch = TaskDefinitionUpdate {
            enabled: Some(true),
            cron_expr: None,
            default_payload: None,
            max_attempts: None,
        };
        let mut query = QueryBuilder::<Postgres>::new("UPDATE task_definitions SET ");
        push_task_definition_assignments(&mut query, patch);
        query.push(" WHERE task_key = ").push_bind("scan_library");

        let sql = query.build().sql().to_string();
        assert_eq!(
            sql,
            "UPDATE task_definitions SET enabled = $1, updated_at = now() WHERE task_key = $2"
        );
        assert!(!sql.contains("= ,"));
    }

    #[test]
    fn task_definition_assignments_support_multiple_fields_without_extra_commas() {
        let patch = TaskDefinitionUpdate {
            enabled: Some(false),
            cron_expr: Some("0 */30 * * * *".to_string()),
            default_payload: Some(serde_json::json!({ "mode": "incremental" })),
            max_attempts: Some(9),
        };
        let mut query = QueryBuilder::<Postgres>::new("UPDATE task_definitions SET ");
        push_task_definition_assignments(&mut query, patch);
        query.push(" WHERE task_key = ").push_bind("scan_library");

        let sql = query.build().sql().to_string();
        assert_eq!(
            sql,
            "UPDATE task_definitions SET enabled = $1, cron_expr = $2, default_payload = $3, max_attempts = $4, updated_at = now() WHERE task_key = $5"
        );
        assert!(!sql.contains("= ,"));
        assert!(!sql.contains(", ,"));
    }

    #[test]
    fn claim_ready_job_sql_guards_status_and_timing_windows() {
        assert!(CLAIM_READY_JOB_SQL.contains("status IN ('queued', 'pending')"));
        assert!(CLAIM_READY_JOB_SQL.contains("next_retry_at <= now()"));
        assert!(CLAIM_READY_JOB_SQL.contains("scheduled_for <= now()"));
        assert!(CLAIM_READY_JOB_SQL.contains("dead_letter = false"));
        assert!(CLAIM_READY_JOB_SQL.contains("j2.kind = $4"));
        assert!(CLAIM_READY_JOB_SQL.contains("j2.status = 'running'"));
    }

    #[test]
    fn requeue_orphaned_running_jobs_sql_resets_running_rows() {
        assert!(REQUEUE_ORPHANED_RUNNING_JOBS_SQL.contains("status = 'queued'"));
        assert!(REQUEUE_ORPHANED_RUNNING_JOBS_SQL.contains("cancel_requested = false"));
        assert!(REQUEUE_ORPHANED_RUNNING_JOBS_SQL.contains("started_at = NULL"));
        assert!(REQUEUE_ORPHANED_RUNNING_JOBS_SQL.contains("status = 'running'"));
        assert!(REQUEUE_ORPHANED_RUNNING_JOBS_SQL.contains("dead_letter = false"));
    }

    #[test]
    fn make_progress_clamps_percent_and_completed() {
        let progress = AppInfra::make_progress(
            "demo",
            10,
            999,
            "demo progress",
            serde_json::json!({}),
        );
        assert_eq!(progress["total"].as_i64(), Some(10));
        assert_eq!(progress["completed"].as_i64(), Some(10));
        assert_eq!(progress["percent"].as_f64(), Some(100.0));
    }

    #[test]
    fn queued_progress_sets_zero_percent() {
        let progress = AppInfra::queued_progress("scan_library");
        assert_eq!(progress["phase"].as_str(), Some("queued"));
        assert_eq!(progress["total"].as_i64(), Some(0));
        assert_eq!(progress["completed"].as_i64(), Some(0));
        assert_eq!(progress["percent"].as_f64(), Some(0.0));
    }
}
