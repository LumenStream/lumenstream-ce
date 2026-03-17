#[derive(Debug, Clone, FromRow)]
struct AgentRequestRow {
    id: Uuid,
    request_type: String,
    source: String,
    user_id: Option<Uuid>,
    title: String,
    content: String,
    media_type: String,
    tmdb_id: Option<i64>,
    media_item_id: Option<Uuid>,
    series_id: Option<Uuid>,
    season_numbers: Value,
    episode_numbers: Value,
    status_user: String,
    status_admin: String,
    agent_stage: String,
    priority: i32,
    auto_handled: bool,
    admin_note: String,
    agent_note: String,
    provider_payload: Value,
    provider_result: Value,
    last_error: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    closed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow)]
struct AgentRequestEventRow {
    id: Uuid,
    request_id: Uuid,
    event_type: String,
    actor_user_id: Option<Uuid>,
    actor_username: Option<String>,
    summary: String,
    detail: Value,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentRequestListQuery {
    pub limit: Option<i64>,
    pub request_type: Option<String>,
    pub status_admin: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentReviewRequest {
    pub action: String,
    pub note: Option<String>,
}

impl From<AgentRequestRow> for AgentRequest {
    fn from(value: AgentRequestRow) -> Self {
        Self {
            id: value.id,
            request_type: value.request_type,
            source: value.source,
            user_id: value.user_id,
            title: value.title,
            content: value.content,
            media_type: value.media_type,
            tmdb_id: value.tmdb_id,
            media_item_id: value.media_item_id,
            series_id: value.series_id,
            season_numbers: agent_json_i32_list(&value.season_numbers),
            episode_numbers: agent_json_i32_list(&value.episode_numbers),
            status_user: value.status_user,
            status_admin: value.status_admin,
            agent_stage: value.agent_stage,
            priority: value.priority,
            auto_handled: value.auto_handled,
            admin_note: value.admin_note,
            agent_note: value.agent_note,
            provider_payload: value.provider_payload,
            provider_result: value.provider_result,
            last_error: value.last_error,
            created_at: value.created_at,
            updated_at: value.updated_at,
            closed_at: value.closed_at,
        }
    }
}

impl From<AgentRequestEventRow> for AgentRequestEvent {
    fn from(value: AgentRequestEventRow) -> Self {
        Self {
            id: value.id,
            request_id: value.request_id,
            event_type: value.event_type,
            actor_user_id: value.actor_user_id,
            actor_username: value.actor_username,
            summary: value.summary,
            detail: value.detail,
            created_at: value.created_at,
        }
    }
}

#[derive(Debug, Clone)]
struct SeriesGapSummary {
    missing_seasons: Vec<i32>,
    missing_episodes: std::collections::BTreeMap<i32, Vec<i32>>,
}

impl SeriesGapSummary {
    fn is_empty(&self) -> bool {
        self.missing_seasons.is_empty() && self.missing_episodes.values().all(Vec::is_empty)
    }
}

impl AppInfra {
    pub async fn list_user_agent_requests(
        &self,
        user_id: Uuid,
        query: AgentRequestListQuery,
    ) -> anyhow::Result<Vec<AgentRequest>> {
        let limit = query.limit.unwrap_or(50).clamp(1, 200);
        let rows = sqlx::query_as::<_, AgentRequestRow>(
            r#"
SELECT *
FROM agent_requests
WHERE user_id = $1
  AND ($2::text IS NULL OR request_type = $2)
ORDER BY created_at DESC
LIMIT $3
            "#,
        )
        .bind(user_id)
        .bind(query.request_type.as_deref())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn list_admin_agent_requests(
        &self,
        query: AgentRequestListQuery,
    ) -> anyhow::Result<Vec<AgentRequest>> {
        let limit = query.limit.unwrap_or(200).clamp(1, 500);
        let rows = sqlx::query_as::<_, AgentRequestRow>(
            r#"
SELECT *
FROM agent_requests
WHERE ($1::text IS NULL OR request_type = $1)
  AND ($2::text IS NULL OR status_admin = $2)
ORDER BY created_at DESC
LIMIT $3
            "#,
        )
        .bind(query.request_type.as_deref())
        .bind(query.status_admin.as_deref())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn get_agent_request_detail(
        &self,
        request_id: Uuid,
    ) -> anyhow::Result<Option<AgentRequestDetail>> {
        let Some(request) = self.get_agent_request(request_id).await? else {
            return Ok(None);
        };
        let events = self.list_agent_request_events(request_id).await?;
        Ok(Some(self.build_agent_request_detail(request, events)))
    }

    pub async fn get_agent_request_detail_for_user(
        &self,
        user_id: Uuid,
        request_id: Uuid,
    ) -> anyhow::Result<Option<AgentRequestDetail>> {
        let Some(detail) = self.get_agent_request_detail(request_id).await? else {
            return Ok(None);
        };
        if detail.request.user_id != Some(user_id) {
            return Ok(None);
        }
        Ok(Some(detail))
    }

    pub async fn list_agent_provider_statuses(&self) -> anyhow::Result<Vec<AgentProviderStatus>> {
        let mut providers = Vec::new();
        let config = self.config_snapshot();
        let now = Utc::now();

        let tmdb_configured = config.tmdb.enabled && !config.tmdb.api_key.trim().is_empty();
        providers.push(AgentProviderStatus {
            provider_id: "tmdb".to_string(),
            display_name: "TMDB".to_string(),
            provider_kind: "metadata".to_string(),
            enabled: config.tmdb.enabled,
            configured: tmdb_configured,
            healthy: tmdb_configured,
            capabilities: vec![AgentProviderCapability::Metadata.as_str().to_string()],
            message: if tmdb_configured {
                "metadata provider ready".to_string()
            } else if config.tmdb.enabled {
                "tmdb api key missing".to_string()
            } else {
                "provider disabled".to_string()
            },
            checked_at: Some(now),
        });

        providers.push(AgentProviderStatus {
            provider_id: "ls_notifications".to_string(),
            display_name: "LumenStream Notifications".to_string(),
            provider_kind: "notification".to_string(),
            enabled: true,
            configured: true,
            healthy: true,
            capabilities: vec![AgentProviderCapability::Notify.as_str().to_string()],
            message: "internal notification provider ready".to_string(),
            checked_at: Some(now),
        });

        let mut moviepilot = MoviePilotProvider::from_config(&config.agent.moviepilot)?;
        providers.push(moviepilot.status().await);
        Ok(providers)
    }

    pub async fn create_agent_request(
        &self,
        user_id: Uuid,
        input: AgentRequestCreateInput,
    ) -> anyhow::Result<AgentRequestDetail> {
        let title = input.title.trim();
        if title.is_empty() {
            anyhow::bail!("title is required");
        }
        let request_type = normalize_agent_request_type(input.request_type.as_str())
            .ok_or_else(|| anyhow::anyhow!("invalid request_type"))?;
        let media_type = normalize_agent_media_type(input.media_type.as_str());
        let season_numbers = normalize_int_list(&input.season_numbers);
        let episode_numbers = normalize_int_list(&input.episode_numbers);

        if let Some(existing_id) = self
            .find_duplicate_agent_request(
                user_id,
                request_type,
                input.tmdb_id,
                input.media_item_id,
                input.series_id,
            )
            .await?
        {
            self.append_agent_request_event(
                existing_id,
                "request.duplicate",
                Some(user_id),
                None,
                "重复提交已合并到现有工单",
                json!({}),
            )
            .await?;
            return self
                .get_agent_request_detail_for_user(user_id, existing_id)
                .await?
                .context("duplicate request should exist");
        }

        let cfg = self.config_snapshot().agent;
        let status_admin = if cfg.enabled { "new" } else { "review_required" };
        let id = Uuid::now_v7();
        let now = Utc::now();
        sqlx::query(
            r#"
INSERT INTO agent_requests (
    id, request_type, source, user_id, title, content, media_type, tmdb_id, media_item_id, series_id,
    season_numbers, episode_numbers, status_user, status_admin, agent_stage, priority, auto_handled,
    admin_note, agent_note, provider_payload, provider_result, created_at, updated_at
)
VALUES (
    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
    $11, $12, $13, $14, $15, 0, false,
    '', '', '{}'::jsonb, '{}'::jsonb, $16, $16
)
            "#,
        )
        .bind(id)
        .bind(request_type)
        .bind(if input.source.trim().is_empty() {
            "user_submit"
        } else {
            input.source.as_str()
        })
        .bind(user_id)
        .bind(title)
        .bind(input.content.trim())
        .bind(media_type)
        .bind(input.tmdb_id.filter(|value| *value > 0))
        .bind(input.media_item_id)
        .bind(input.series_id)
        .bind(json!(season_numbers))
        .bind(json!(episode_numbers))
        .bind(admin_status_to_user_status(status_admin))
        .bind(status_admin)
        .bind("queued")
        .bind(now)
        .execute(&self.pool)
        .await?;

        self.append_agent_request_event(
            id,
            "request.created",
            Some(user_id),
            None,
            "已创建求片工单",
            json!({ "source": "user_submit" }),
        )
        .await?;

        let _ = self
            .create_notification(
                user_id,
                "请求已提交",
                "系统已接收你的请求，Agent 将自动尝试处理。",
                "agent_request",
                json!({ "request_id": id }),
            )
            .await;

        self
            .get_agent_request_detail(id)
            .await?
            .context("newly created request not found")
    }

    pub async fn enqueue_agent_request_job(&self, request_id: Uuid) -> anyhow::Result<Job> {
        self.enqueue_job("agent_request_process", json!({ "request_id": request_id }), 3)
            .await
    }

    pub async fn review_agent_request(
        &self,
        request_id: Uuid,
        actor_user_id: Option<Uuid>,
        actor_username: Option<&str>,
        action: &str,
        note: Option<&str>,
    ) -> anyhow::Result<Option<AgentRequestDetail>> {
        let Some(current) = self.get_agent_request(request_id).await? else {
            return Ok(None);
        };
        let note = note.unwrap_or("").trim();
        match action {
            "approve" => {
                self.update_request_state_with_event(
                    request_id,
                    "processing",
                    "approved",
                    "queued",
                    current.auto_handled,
                    note,
                    &current.agent_note,
                    &current.provider_result,
                    None,
                    None,
                    "admin.approve",
                    actor_user_id,
                    actor_username,
                    "管理员已批准，任务重新进入自动处理",
                    json!({ "note": note }),
                )
                .await?;
            }
            "reject" => {
                self.update_request_state_with_event(
                    request_id,
                    "failed",
                    "rejected",
                    "closed",
                    current.auto_handled,
                    note,
                    &current.agent_note,
                    &current.provider_result,
                    Some("rejected by admin"),
                    Some(Utc::now()),
                    "admin.reject",
                    actor_user_id,
                    actor_username,
                    "管理员已拒绝该请求",
                    json!({ "note": note }),
                )
                .await?;
                if let Some(user_id) = current.user_id {
                    let _ = self
                        .create_notification(
                            user_id,
                            "请求未通过",
                            if note.is_empty() {
                                "管理员已拒绝该请求。".to_string()
                            } else {
                                format!("管理员已拒绝该请求：{note}")
                            }
                            .as_str(),
                            "agent_request",
                            json!({ "request_id": request_id }),
                        )
                        .await;
                }
            }
            "ignore" => {
                self.update_request_state_with_event(
                    request_id,
                    "closed",
                    "ignored",
                    "closed",
                    current.auto_handled,
                    note,
                    &current.agent_note,
                    &current.provider_result,
                    None,
                    Some(Utc::now()),
                    "admin.ignore",
                    actor_user_id,
                    actor_username,
                    "管理员已忽略该请求",
                    json!({ "note": note }),
                )
                .await?;
            }
            "manual_complete" => {
                self.update_request_state_with_event(
                    request_id,
                    "success",
                    "completed",
                    "closed",
                    current.auto_handled,
                    note,
                    &current.agent_note,
                    &current.provider_result,
                    None,
                    Some(Utc::now()),
                    "admin.manual_complete",
                    actor_user_id,
                    actor_username,
                    "管理员已手动标记完成",
                    json!({ "note": note }),
                )
                .await?;
            }
            _ => anyhow::bail!("invalid review action"),
        }
        self.get_agent_request_detail(request_id).await
    }

    pub async fn retry_agent_request(
        &self,
        request_id: Uuid,
        actor_user_id: Option<Uuid>,
        actor_username: Option<&str>,
    ) -> anyhow::Result<Option<AgentRequestDetail>> {
        let Some(current) = self.get_agent_request(request_id).await? else {
            return Ok(None);
        };
        self.update_request_state_with_event(
            request_id,
            USER_STATUS_PROCESSING,
            "new",
            "queued",
            current.auto_handled,
            &current.admin_note,
            &current.agent_note,
            &current.provider_result,
            None,
            None,
            "admin.retry",
            actor_user_id,
            actor_username,
            "管理员重新触发 Agent 处理",
            json!({}),
        )
        .await?;
        self.get_agent_request_detail(request_id).await
    }

    pub async fn enqueue_agent_missing_scan_job(&self) -> anyhow::Result<Job> {
        self.enqueue_job("agent_missing_scan", json!({}), 1).await
    }

    async fn get_agent_request(&self, request_id: Uuid) -> anyhow::Result<Option<AgentRequest>> {
        let row = sqlx::query_as::<_, AgentRequestRow>(
            "SELECT * FROM agent_requests WHERE id = $1 LIMIT 1",
        )
        .bind(request_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Into::into))
    }

    async fn list_agent_request_events(
        &self,
        request_id: Uuid,
    ) -> anyhow::Result<Vec<AgentRequestEvent>> {
        let rows = sqlx::query_as::<_, AgentRequestEventRow>(
            r#"
SELECT *
FROM agent_request_events
WHERE request_id = $1
ORDER BY created_at ASC
            "#,
        )
        .bind(request_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn append_agent_request_event(
        &self,
        request_id: Uuid,
        event_type: &str,
        actor_user_id: Option<Uuid>,
        actor_username: Option<&str>,
        summary: &str,
        detail: Value,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
INSERT INTO agent_request_events (id, request_id, event_type, actor_user_id, actor_username, summary, detail, created_at)
VALUES ($1, $2, $3, $4, $5, $6, $7, now())
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(request_id)
        .bind(event_type)
        .bind(actor_user_id)
        .bind(actor_username.unwrap_or(""))
        .bind(summary)
        .bind(detail)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn update_request_state_with_event(
        &self,
        request_id: Uuid,
        status_user: &str,
        status_admin: &str,
        agent_stage: &str,
        auto_handled: bool,
        admin_note: &str,
        agent_note: &str,
        provider_result: &Value,
        last_error: Option<&str>,
        closed_at: Option<DateTime<Utc>>,
        event_type: &str,
        actor_user_id: Option<Uuid>,
        actor_username: Option<&str>,
        event_summary: &str,
        event_detail: Value,
    ) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
UPDATE agent_requests
SET status_user = $2,
    status_admin = $3,
    agent_stage = $4,
    auto_handled = $5,
    admin_note = $6,
    agent_note = $7,
    provider_result = $8,
    last_error = $9,
    updated_at = now(),
    closed_at = $10
WHERE id = $1
            "#,
        )
        .bind(request_id)
        .bind(status_user)
        .bind(status_admin)
        .bind(agent_stage)
        .bind(auto_handled)
        .bind(admin_note)
        .bind(agent_note)
        .bind(provider_result)
        .bind(last_error)
        .bind(closed_at)
        .execute(&mut *tx)
        .await?;

        if !event_type.is_empty() {
            sqlx::query(
                r#"
INSERT INTO agent_request_events (id, request_id, event_type, actor_user_id, actor_username, summary, detail, created_at)
VALUES ($1, $2, $3, $4, $5, $6, $7, now())
                "#,
            )
            .bind(Uuid::now_v7())
            .bind(request_id)
            .bind(event_type)
            .bind(actor_user_id)
            .bind(actor_username.unwrap_or(""))
            .bind(event_summary)
            .bind(event_detail)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn find_duplicate_agent_request(
        &self,
        user_id: Uuid,
        request_type: &str,
        tmdb_id: Option<i64>,
        media_item_id: Option<Uuid>,
        series_id: Option<Uuid>,
    ) -> anyhow::Result<Option<Uuid>> {
        let row = sqlx::query_scalar(
            r#"
SELECT id
FROM agent_requests
WHERE user_id = $1
  AND request_type = $2
  AND status_admin IN ('new', 'analyzing', 'auto_processing', 'review_required', 'approved')
  AND (
      ($3::bigint IS NOT NULL AND tmdb_id = $3)
      OR ($4::uuid IS NOT NULL AND media_item_id = $4)
      OR ($5::uuid IS NOT NULL AND series_id = $5)
      OR ($3::bigint IS NULL AND $4::uuid IS NULL AND $5::uuid IS NULL AND title = title)
  )
ORDER BY created_at DESC
LIMIT 1
            "#,
        )
        .bind(user_id)
        .bind(request_type)
        .bind(tmdb_id)
        .bind(media_item_id)
        .bind(series_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn resolve_library_hit(
        &self,
        request: &AgentRequest,
    ) -> anyhow::Result<Option<(Uuid, String)>> {
        if let Some(media_item_id) = request.media_item_id {
            if let Some(row) = sqlx::query_as::<_, (Uuid, String)>(
                "SELECT id, name FROM media_items WHERE id = $1 LIMIT 1",
            )
            .bind(media_item_id)
            .fetch_optional(&self.pool)
            .await?
            {
                return Ok(Some(row));
            }
        }

        let tmdb_id = request.tmdb_id.filter(|value| *value > 0);
        if let Some(tmdb_id) = tmdb_id {
            let tmdb = tmdb_id.to_string();
            if request.media_type.eq_ignore_ascii_case("movie") {
                if let Some(row) = sqlx::query_as::<_, (Uuid, String)>(
                    r#"
SELECT id, name
FROM media_items
WHERE item_type = 'Movie'
  AND metadata->>'tmdb_id' = $1
LIMIT 1
                    "#,
                )
                .bind(&tmdb)
                .fetch_optional(&self.pool)
                .await?
                {
                    return Ok(Some(row));
                }
            } else if let Some(row) = sqlx::query_as::<_, (Uuid, String)>(
                r#"
SELECT id, name
FROM media_items
WHERE item_type = 'Series'
  AND COALESCE(metadata->>'series_tmdb_id', metadata->>'tmdb_id') = $1
LIMIT 1
                "#,
            )
            .bind(&tmdb)
            .fetch_optional(&self.pool)
            .await?
            {
                return Ok(Some(row));
            }
        }

        Ok(None)
    }

    async fn run_agent_request_job(
        &self,
        job_id: Uuid,
        payload: &Value,
    ) -> anyhow::Result<Value> {
        let request_id = payload
            .get("request_id")
            .and_then(Value::as_str)
            .and_then(|value| Uuid::parse_str(value).ok())
            .context("agent request job missing request_id")?;
        self.set_job_progress(job_id, "load", 1, 0, "读取工单", json!({ "request_id": request_id }))
            .await?;
        let Some(mut request) = self.get_agent_request(request_id).await? else {
            anyhow::bail!("agent request not found");
        };

        let cfg = self.config_snapshot().agent;
        if request.tmdb_id.unwrap_or(0) == 0 && cfg.llm.enabled {
            self.set_job_progress(job_id, "llm_parse", 1, 0, "正在进行 LLM 语义解析", json!({}))
                .await?;
            if let Ok(llm) = LlmProvider::new(&cfg.llm) {
                if let Ok(parsed) = llm.parse_intent(&request.content).await {
                    let mut updated = false;
                    if request.media_type.is_empty() || request.media_type == "unknown" {
                        request.media_type = parsed.media_type.clone();
                        updated = true;
                    }
                    if request.title.is_empty() {
                        request.title = parsed.title.clone();
                        updated = true;
                    }
                    if request.season_numbers.is_empty() && !parsed.season_numbers.is_empty() {
                        request.season_numbers = parsed.season_numbers.clone();
                        updated = true;
                    }
                    if request.episode_numbers.is_empty() && !parsed.episode_numbers.is_empty() {
                        request.episode_numbers = parsed.episode_numbers.clone();
                        updated = true;
                    }

                    if updated {
                        sqlx::query(
                            "UPDATE agent_requests SET media_type = $2, title = $3, season_numbers = $4, episode_numbers = $5, updated_at = now() WHERE id = $1"
                        )
                        .bind(request.id)
                        .bind(&request.media_type)
                        .bind(&request.title)
                        .bind(json!(&request.season_numbers))
                        .bind(json!(&request.episode_numbers))
                        .execute(&self.pool)
                        .await?;

                        self.append_agent_request_event(
                            request_id,
                            "agent.llm_parsed",
                            None,
                            Some("system"),
                            "已通过 LLM 解析意图",
                            json!(parsed),
                        )
                        .await?;
                    }
                }
            }
        }

        let workflow_kind = infer_workflow_kind(&request.request_type);
        let required_capabilities = workflow_required_capabilities(&workflow_kind)
            .into_iter()
            .map(|cap| cap.as_str().to_string())
            .collect::<Vec<_>>();

        self.update_request_state_with_event(
            request_id,
            USER_STATUS_PROCESSING,
            "analyzing",
            "library_check",
            request.auto_handled,
            &request.admin_note,
            &request.agent_note,
            &request.provider_result,
            None,
            None,
            "agent.workflow.planned",
            None,
            Some("system"),
            "已生成工作流执行计划",
            json!({
                "workflow_kind": workflow_kind.as_str(),
                "required_capabilities": required_capabilities,
            }),
        )
        .await?;

        if let Some((media_item_id, media_name)) = self.resolve_library_hit(&request).await? {
            let result = json!({
                "matched_media_item_id": media_item_id,
                "matched_media_name": media_name,
                "reason": "library_hit",
            });
            self.update_request_state_with_event(
                request_id,
                "success",
                "completed",
                "verify",
                true,
                &request.admin_note,
                "媒体已在库内，自动关闭工单",
                &result,
                None,
                Some(Utc::now()),
                "agent.library_hit",
                None,
                Some("system"),
                "已命中库内媒体，自动完成",
                result.clone(),
            )
            .await?;
            if let Some(user_id) = request.user_id {
                let _ = self
                    .create_notification(
                        user_id,
                        "请求已完成",
                        "系统检测到目标媒体已在库内，可直接观看。",
                        "agent_request",
                        json!({ "request_id": request_id, "media_item_id": media_item_id }),
                    )
                    .await;
            }
            return Ok(result);
        }

        if matches!(request.request_type.as_str(), "missing_episode" | "missing_season")
            && let Some(series_id) = request.series_id
        {
            let gaps = self.detect_series_gaps(series_id).await?;
            if gaps.is_empty() {
                let result = json!({ "reason": "series_is_complete" });
                self.update_request_state_with_event(
                    request_id,
                    "success",
                    "completed",
                    "verify",
                    true,
                    &request.admin_note,
                    "当前库内季集已完整，自动关闭工单",
                    &result,
                    None,
                    Some(Utc::now()),
                    "agent.series_complete",
                    None,
                    Some("system"),
                    "剧集当前已完整",
                    result.clone(),
                )
                .await?;
                return Ok(result);
            }
        }

        let cfg = self.config_snapshot().agent;
        let provider_statuses = self.list_agent_provider_statuses().await?;
        let available_capabilities = provider_statuses
            .iter()
            .filter(|provider| provider.enabled && provider.configured && provider.healthy)
            .flat_map(|provider| provider.capabilities.iter().cloned())
            .collect::<HashSet<_>>();
        let missing_capabilities = workflow_required_capabilities(&workflow_kind)
            .into_iter()
            .map(|cap| cap.as_str().to_string())
            .filter(|cap| !available_capabilities.contains(cap))
            .collect::<Vec<_>>();
        if !missing_capabilities.is_empty() {
            let result = json!({ "reason": "moviepilot_not_configured" });
            self.update_request_state_with_event(
                request_id,
                USER_STATUS_ACTION_REQUIRED,
                "review_required",
                "manual_review",
                false,
                &request.admin_note,
                "工作流依赖的 Provider 能力不可用，已转人工处理",
                &result,
                Some("required capabilities unavailable"),
                None,
                "agent.review_required",
                None,
                Some("system"),
                "关键 Provider 能力不可用，已转人工处理",
                json!({
                    "provider_statuses": provider_statuses,
                    "missing_capabilities": missing_capabilities,
                }),
            )
            .await?;
            return Ok(result);
        }

        self.set_job_progress(job_id, "moviepilot", 1, 0, "正在搜索资源", json!({}))
            .await?;
        let mut moviepilot = MoviePilotProvider::from_config(&cfg.moviepilot)?.into_client();
        let search_response = if let Some(tmdb_id) = request.tmdb_id.filter(|value| *value > 0) {
            moviepilot
                .search_by_tmdb(tmdb_id, request.season_numbers.first().copied())
                .await
        } else {
            moviepilot.search_by_title(&request.title).await
        };

        let search_response = match search_response {
            Ok(response) => response,
            Err(err) => {
                let result = json!({ "error": err.to_string() });
                self.update_request_state_with_event(
                    request_id,
                    USER_STATUS_ACTION_REQUIRED,
                    "review_required",
                    "mp_search",
                    false,
                    &request.admin_note,
                    "搜索资源失败，已转人工处理",
                    &result,
                    Some(&err.to_string()),
                    None,
                    "agent.moviepilot.search_failed",
                    None,
                    Some("system"),
                    "搜索资源失败",
                    result.clone(),
                )
                .await?;
                return Ok(result);
            }
        };

        let contexts = decode_search_contexts(&search_response.data);
        let season = request.season_numbers.first().copied();
        let best = choose_best_result(
            &contexts,
            &request.media_type,
            season,
            None,
            &cfg.moviepilot.filter,
        );
        let mut result_payload = json!({
            "search_success": search_response.success,
            "search_message": search_response.message,
            "result_count": contexts.len(),
        });

        if cfg.moviepilot.search_download_enabled && let Some(best) = best.as_ref() {
            let download_payload = build_download_payload(best);
            result_payload["selected_result"] = summarize_moviepilot_result(best);
            match moviepilot.submit_download(&download_payload).await {
                Ok(response) => {
                    result_payload["download"] = json!({
                        "success": response.success,
                        "message": response.message,
                    });
                    self.update_request_state_with_event(
                        request_id,
                        "success",
                        "completed",
                        "mp_download",
                        true,
                        &request.admin_note,
                        "已通过 Provider 提交下载",
                        &result_payload,
                        None,
                        Some(Utc::now()),
                        "agent.moviepilot.download_submitted",
                        None,
                        Some("system"),
                        "已提交下载任务",
                        result_payload.clone(),
                    )
                    .await?;
                    if let Some(user_id) = request.user_id {
                        let _ = self
                            .create_notification(
                                user_id,
                                "请求已处理",
                                "Agent 已为该请求提交下载任务。",
                                "agent_request",
                                json!({ "request_id": request_id }),
                            )
                            .await;
                    }
                    return Ok(result_payload);
                }
                Err(err) => {
                    result_payload["download_error"] = json!(err.to_string());
                }
            }
        }

        if cfg.moviepilot.subscribe_fallback_enabled {
            let subscription = build_subscription_payload(
                &request.title,
                &request.media_type,
                request.tmdb_id,
                season,
                &request.content,
                best.as_ref(),
            );
            match moviepilot.create_subscription(&subscription).await {
                Ok(response) => {
                    result_payload["subscription"] = json!({
                        "success": response.success,
                        "message": response.message,
                    });
                    self.update_request_state_with_event(
                        request_id,
                        "success",
                        "completed",
                        "mp_subscribe",
                        true,
                        &request.admin_note,
                        "已创建订阅，等待后续自动补齐",
                        &result_payload,
                        None,
                        Some(Utc::now()),
                        "agent.moviepilot.subscription_created",
                        None,
                        Some("system"),
                        "已创建订阅",
                        result_payload.clone(),
                    )
                    .await?;
                    if let Some(user_id) = request.user_id {
                        let _ = self
                            .create_notification(
                                user_id,
                                "请求已订阅",
                                "Agent 已为该请求创建订阅，后续命中资源时会自动处理。",
                                "agent_request",
                                json!({ "request_id": request_id }),
                            )
                            .await;
                    }
                    return Ok(result_payload);
                }
                Err(err) => {
                    result_payload["subscription_error"] = json!(err.to_string());
                }
            }
        }

        self.update_request_state_with_event(
            request_id,
            USER_STATUS_ACTION_REQUIRED,
            "review_required",
            "manual_review",
            false,
            &request.admin_note,
            "未找到可自动处理的结果，已转人工处理",
            &result_payload,
            Some("agent fallback to review"),
            None,
            "agent.review_required",
            None,
            Some("system"),
            "未找到可自动处理结果，已转人工处理",
            result_payload.clone(),
        )
        .await?;
        Ok(result_payload)
    }

    async fn run_agent_missing_scan_job(
        &self,
        job_id: Uuid,
        _payload: &Value,
    ) -> anyhow::Result<Value> {
        let cfg = self.config_snapshot().agent;
        if !cfg.enabled || !cfg.missing_scan_enabled {
            return Ok(json!({ "skipped": true, "reason": "agent missing scan disabled" }));
        }

        let series_rows = sqlx::query_as::<_, (Uuid, String)>(
            r#"
SELECT id, name
FROM media_items
WHERE item_type = 'Series'
ORDER BY updated_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let total = series_rows.len() as i64;
        let mut created_requests = 0_i64;
        for (idx, (series_id, series_name)) in series_rows.into_iter().enumerate() {
            self.set_job_progress(
                job_id,
                "scan",
                total,
                idx as i64,
                "扫描剧集缺集情况",
                json!({ "series_id": series_id, "series_name": series_name }),
            )
            .await?;
            let gaps = self.detect_series_gaps(series_id).await?;
            if gaps.is_empty() {
                continue;
            }
            if !gaps.missing_seasons.is_empty() {
                if self
                    .create_or_update_auto_gap_request(
                        "missing_season",
                        series_id,
                        &series_name,
                        &gaps.missing_seasons,
                        &[],
                    )
                    .await?
                {
                    created_requests += 1;
                }
            }
            let mut episode_seasons = Vec::new();
            let mut episode_numbers = Vec::new();
            for (season, episodes) in &gaps.missing_episodes {
                if episodes.is_empty() {
                    continue;
                }
                episode_seasons.push(*season);
                episode_numbers.extend_from_slice(episodes);
            }
            if !episode_numbers.is_empty()
                && self
                    .create_or_update_auto_gap_request(
                        "missing_episode",
                        series_id,
                        &series_name,
                        &episode_seasons,
                        &episode_numbers,
                    )
                    .await?
            {
                created_requests += 1;
            }
        }
        self.set_job_progress(job_id, "scan", total, total, "缺集扫描完成", json!({ "created_requests": created_requests }))
            .await?;
        Ok(json!({ "created_requests": created_requests }))
    }

    async fn create_or_update_auto_gap_request(
        &self,
        request_type: &str,
        series_id: Uuid,
        series_name: &str,
        season_numbers: &[i32],
        episode_numbers: &[i32],
    ) -> anyhow::Result<bool> {
        let existing = sqlx::query_scalar::<_, Uuid>(
            r#"
SELECT id
FROM agent_requests
WHERE request_type = $1
  AND series_id = $2
  AND status_admin IN ('new', 'analyzing', 'auto_processing', 'review_required', 'approved')
LIMIT 1
            "#,
        )
        .bind(request_type)
        .bind(series_id)
        .fetch_optional(&self.pool)
        .await?;

        let title = format!("{series_name} {}", if request_type == "missing_season" { "缺季" } else { "缺集" });
        let content = if request_type == "missing_season" {
            format!("自动检测到缺失季：{}", format_int_list(season_numbers, "S"))
        } else {
            format!(
                "自动检测到缺失剧集：{} / {}",
                format_int_list(season_numbers, "S"),
                format_episode_list(episode_numbers)
            )
        };

        if let Some(request_id) = existing {
            sqlx::query(
                r#"
UPDATE agent_requests
SET title = $2,
    content = $3,
    season_numbers = $4,
    episode_numbers = $5,
    updated_at = now()
WHERE id = $1
                "#,
            )
            .bind(request_id)
            .bind(title)
            .bind(content)
            .bind(json!(normalize_int_list(season_numbers)))
            .bind(json!(normalize_int_list(episode_numbers)))
            .execute(&self.pool)
            .await?;
            return Ok(false);
        }

        let request_id = Uuid::now_v7();
        sqlx::query(
            r#"
INSERT INTO agent_requests (
    id, request_type, source, user_id, title, content, media_type, series_id, season_numbers, episode_numbers,
    status_user, status_admin, agent_stage, priority, auto_handled, admin_note, agent_note, provider_payload, provider_result
)
VALUES (
    $1, $2, 'auto_detected', NULL, $3, $4, 'series', $5, $6, $7,
    $8, 'new', 'queued', 10, false, '', '', '{}'::jsonb, '{}'::jsonb
)
            "#,
        )
        .bind(request_id)
        .bind(request_type)
        .bind(title)
        .bind(content)
        .bind(series_id)
        .bind(json!(normalize_int_list(season_numbers)))
        .bind(json!(normalize_int_list(episode_numbers)))
        .bind(admin_status_to_user_status("new"))
        .execute(&self.pool)
        .await?;
        self.append_agent_request_event(
            request_id,
            "request.auto_created",
            None,
            Some("system"),
            "系统自动发现缺集/漏季并创建工单",
            json!({}),
        )
        .await?;
        let _ = self.enqueue_agent_request_job(request_id).await?;
        Ok(true)
    }

    async fn detect_series_gaps(&self, series_id: Uuid) -> anyhow::Result<SeriesGapSummary> {
        let Some((series_name, metadata)) = sqlx::query_as::<_, (String, Value)>(
            "SELECT name, metadata FROM media_items WHERE id = $1 AND item_type = 'Series' LIMIT 1",
        )
        .bind(series_id)
        .fetch_optional(&self.pool)
        .await?
        else {
            return Ok(SeriesGapSummary {
                missing_seasons: Vec::new(),
                missing_episodes: std::collections::BTreeMap::new(),
            });
        };

        let tmdb_id = metadata
            .get("series_tmdb_id")
            .and_then(agent_value_to_i64)
            .or_else(|| metadata.get("tmdb_id").and_then(agent_value_to_i64))
            .filter(|value| *value > 0);
        let Some(tmdb_id) = tmdb_id else {
            debug!(series_id = %series_id, series_name = %series_name, "series gap detection skipped without tmdb id");
            return Ok(SeriesGapSummary {
                missing_seasons: Vec::new(),
                missing_episodes: std::collections::BTreeMap::new(),
            });
        };

        let cfg = self.config_snapshot();
        if !cfg.tmdb.enabled || cfg.tmdb.api_key.trim().is_empty() {
            return Ok(SeriesGapSummary {
                missing_seasons: Vec::new(),
                missing_episodes: std::collections::BTreeMap::new(),
            });
        }

        let endpoint = format!(
            "{TMDB_API_BASE}/tv/{tmdb_id}?language={lang}",
            lang = urlencoding::encode(&cfg.tmdb.language)
        );
        let Some(tv_details) = self.tmdb_get_json_opt(&endpoint).await? else {
            return Ok(SeriesGapSummary {
                missing_seasons: Vec::new(),
                missing_episodes: std::collections::BTreeMap::new(),
            });
        };

        let expected_seasons = tv_details
            .get("number_of_seasons")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            .max(0) as i32;
        let seasons_array = tv_details.get("seasons").and_then(Value::as_array);

        let season_rows = sqlx::query_as::<_, (Option<i32>,)>(
            "SELECT season_number FROM media_items WHERE series_id = $1 AND item_type = 'Season'",
        )
        .bind(series_id)
        .fetch_all(&self.pool)
        .await?;
        let episode_rows = sqlx::query_as::<_, (Option<i32>, Option<i32>)>(
            "SELECT season_number, episode_number FROM media_items WHERE series_id = $1 AND item_type = 'Episode'",
        )
        .bind(series_id)
        .fetch_all(&self.pool)
        .await?;
        let existing_seasons = season_rows
            .into_iter()
            .filter_map(|(season_number,)| season_number)
            .collect::<HashSet<_>>();
        let mut existing_episodes: std::collections::BTreeMap<i32, HashSet<i32>> =
            std::collections::BTreeMap::new();
        for (season_number, episode_number) in episode_rows {
            if let (Some(season_number), Some(episode_number)) = (season_number, episode_number) {
                existing_episodes
                    .entry(season_number)
                    .or_default()
                    .insert(episode_number);
            }
        }

        let mut missing_seasons = Vec::new();
        let mut missing_episodes = std::collections::BTreeMap::new();
        for season in 1..=expected_seasons {
            let has_season = existing_seasons.contains(&season) || existing_episodes.contains_key(&season);
            if !has_season {
                missing_seasons.push(season);
                continue;
            }

            let expected_episode_count = seasons_array
                .and_then(|arr| {
                    arr.iter().find(|s| {
                        s.get("season_number").and_then(Value::as_i64) == Some(season as i64)
                    })
                })
                .and_then(|s| s.get("episode_count").and_then(Value::as_i64))
                .unwrap_or(0) as i32;

            if expected_episode_count <= 0 {
                continue;
            }

            let present = existing_episodes.get(&season).cloned().unwrap_or_default();
            let missing = (1..=expected_episode_count)
                .filter(|episode_number| !present.contains(episode_number))
                .collect::<Vec<_>>();
            if !missing.is_empty() {
                missing_episodes.insert(season, missing);
            }
        }

        Ok(SeriesGapSummary {
            missing_seasons,
            missing_episodes,
        })
    }

    pub async fn test_moviepilot_connection(
        &self,
        config: &ls_config::AgentConfig,
    ) -> anyhow::Result<Value> {
        let mut provider = MoviePilotProvider::from_config(&config.moviepilot)?;
        let status = provider.status().await;
        if !status.healthy {
            anyhow::bail!(status.message);
        }
        Ok(json!({
            "ok": true,
            "base_url": config.moviepilot.base_url,
            "timeout_seconds": config.moviepilot.timeout_seconds,
        }))
    }

    fn build_agent_request_detail(
        &self,
        request: AgentRequest,
        events: Vec<AgentRequestEvent>,
    ) -> AgentRequestDetail {
        let workflow_kind = infer_workflow_kind(&request.request_type);
        let required_capabilities = workflow_required_capabilities(&workflow_kind)
            .into_iter()
            .map(|cap| cap.as_str().to_string())
            .collect::<Vec<_>>();
        let workflow_steps =
            infer_workflow_steps(&workflow_kind, &request.agent_stage, &request.status_admin);
        let manual_actions = infer_manual_actions(&request.status_admin, request.auto_handled);
        AgentRequestDetail {
            request,
            events,
            workflow_kind: workflow_kind.as_str().to_string(),
            workflow_steps,
            required_capabilities,
            manual_actions,
        }
    }
}

fn agent_json_i32_list(value: &Value) -> Vec<i32> {
    value
        .as_array()
        .into_iter()
        .flat_map(|items| items.iter())
        .filter_map(|item| item.as_i64())
        .map(|value| value as i32)
        .filter(|value| *value > 0)
        .collect()
}

fn agent_value_to_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_str().and_then(|text| text.trim().parse::<i64>().ok()))
}

fn normalize_agent_request_type(raw: &str) -> Option<&'static str> {
    match raw.trim() {
        "media_request" => Some("media_request"),
        "feedback" => Some("feedback"),
        "missing_episode" => Some("missing_episode"),
        "missing_season" => Some("missing_season"),
        _ => None,
    }
}

fn normalize_agent_media_type(raw: &str) -> &'static str {
    match raw.trim() {
        "movie" => "movie",
        "series" => "series",
        _ => "unknown",
    }
}

fn format_int_list(values: &[i32], prefix: &str) -> String {
    normalize_int_list(values)
        .into_iter()
        .map(|value| format!("{prefix}{value:02}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_episode_list(values: &[i32]) -> String {
    normalize_int_list(values)
        .into_iter()
        .map(|value| format!("E{value:02}"))
        .collect::<Vec<_>>()
        .join(", ")
}
