#[derive(Debug, Deserialize, Default)]
struct AgentCreateRequest {
    #[serde(default)]
    request_type: String,
    #[serde(default)]
    source: String,
    title: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    media_type: String,
    tmdb_id: Option<i64>,
    media_item_id: Option<Uuid>,
    series_id: Option<Uuid>,
    #[serde(default)]
    season_numbers: Vec<i32>,
    #[serde(default)]
    episode_numbers: Vec<i32>,
}

#[derive(Debug, Deserialize, Default)]
struct AgentRequestsQuery {
    limit: Option<i64>,
    request_type: Option<String>,
    status_admin: Option<String>,
}

async fn list_my_agent_requests(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<AgentRequestsQuery>,
) -> Response {
    let user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let Some(user_id) = parse_user_uuid(&user) else {
        return error_response(StatusCode::UNAUTHORIZED, "invalid user");
    };
    match state
        .infra
        .list_user_agent_requests(
            user_id,
            AgentRequestListQuery {
                limit: query.limit,
                request_type: query.request_type.clone(),
                status_admin: None,
            },
        )
        .await
    {
        Ok(items) => Json(items).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to list requests: {err}"),
        ),
    }
}

async fn create_my_agent_request(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<AgentCreateRequest>,
) -> Response {
    let user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let Some(user_id) = parse_user_uuid(&user) else {
        return error_response(StatusCode::UNAUTHORIZED, "invalid user");
    };
    let created = match state
        .infra
        .create_agent_request(
            user_id,
            AgentRequestCreateInput {
                request_type: if payload.request_type.trim().is_empty() {
                    "intake".to_string()
                } else {
                    payload.request_type
                },
                source: payload.source,
                title: payload.title,
                content: payload.content,
                media_type: payload.media_type,
                tmdb_id: payload.tmdb_id,
                media_item_id: payload.media_item_id,
                series_id: payload.series_id,
                season_numbers: payload.season_numbers,
                episode_numbers: payload.episode_numbers,
            },
        )
        .await
    {
        Ok(detail) => detail,
        Err(err) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("failed to create request: {err}"),
            )
        }
    };

    if state.infra.config_snapshot().agent.enabled {
        if let Ok(job) = state.infra.enqueue_agent_request_job(created.request.id).await {
            let infra = state.infra.clone();
            tokio::spawn(async move {
                let _ = infra.process_job(job.id).await;
            });
        }
    }

    (StatusCode::CREATED, Json(created)).into_response()
}

async fn get_my_agent_request(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    AxPath(request_id): AxPath<Uuid>,
) -> Response {
    let user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let Some(user_id) = parse_user_uuid(&user) else {
        return error_response(StatusCode::UNAUTHORIZED, "invalid user");
    };
    match state
        .infra
        .get_agent_request_detail_for_user(user_id, request_id)
        .await
    {
        Ok(Some(detail)) => Json(detail).into_response(),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "request not found"),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to load request detail: {err}"),
        ),
    }
}

async fn resubmit_my_agent_request(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    AxPath(request_id): AxPath<Uuid>,
) -> Response {
    let user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let Some(user_id) = parse_user_uuid(&user) else {
        return error_response(StatusCode::UNAUTHORIZED, "invalid user");
    };
    let Some(detail) = (match state
        .infra
        .get_agent_request_detail_for_user(user_id, request_id)
        .await
    {
        Ok(value) => value,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to load request detail: {err}"),
            )
        }
    }) else {
        return error_response(StatusCode::NOT_FOUND, "request not found");
    };
    match state.infra.retry_agent_request(detail.request.id, Some(user_id), Some(&user.name)).await {
        Ok(Some(updated)) => {
            if let Ok(job) = state.infra.enqueue_agent_request_job(updated.request.id).await {
                let infra = state.infra.clone();
                tokio::spawn(async move {
                    let _ = infra.process_job(job.id).await;
                });
            }
            Json(updated).into_response()
        }
        Ok(None) => error_response(StatusCode::NOT_FOUND, "request not found"),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to retry request: {err}"),
        ),
    }
}

async fn admin_list_agent_requests(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<AgentRequestsQuery>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }
    match state
        .infra
        .list_admin_agent_requests(AgentRequestListQuery {
            limit: query.limit,
            request_type: query.request_type.clone(),
            status_admin: query.status_admin.clone(),
        })
        .await
    {
        Ok(items) => Json(items).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to list admin requests: {err}"),
        ),
    }
}

async fn admin_get_agent_request(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    AxPath(request_id): AxPath<Uuid>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }
    match state.infra.get_agent_request_detail(request_id).await {
        Ok(Some(detail)) => Json(detail).into_response(),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "request not found"),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to load admin request detail: {err}"),
        ),
    }
}

async fn admin_list_agent_providers(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }
    match state.infra.list_agent_provider_statuses().await {
        Ok(items) => Json(items).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to list agent providers: {err}"),
        ),
    }
}

async fn admin_review_agent_request(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    AxPath(request_id): AxPath<Uuid>,
    Json(payload): Json<AgentReviewRequest>,
) -> Response {
    let user = match require_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    match state
        .infra
        .review_agent_request(
            request_id,
            parse_user_uuid(&user),
            Some(&user.name),
            payload.action.as_str(),
            payload.note.as_deref(),
        )
        .await
    {
        Ok(Some(detail)) => {
            if payload.action == "approve"
                && let Ok(job) = state.infra.enqueue_agent_request_job(detail.request.id).await
            {
                let infra = state.infra.clone();
                tokio::spawn(async move {
                    let _ = infra.process_job(job.id).await;
                });
            }
            let _ = state
                .infra
                .log_audit_event(
                    parse_user_uuid(&user),
                    "admin.agent_request.review",
                    "agent_request",
                    Some(&request_id.to_string()),
                    json!({ "action": payload.action, "note": payload.note }),
                )
                .await;
            Json(detail).into_response()
        }
        Ok(None) => error_response(StatusCode::NOT_FOUND, "request not found"),
        Err(err) => error_response(
            StatusCode::BAD_REQUEST,
            &format!("failed to review request: {err}"),
        ),
    }
}

async fn admin_retry_agent_request(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    AxPath(request_id): AxPath<Uuid>,
) -> Response {
    let user = match require_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    match state
        .infra
        .retry_agent_request(request_id, parse_user_uuid(&user), Some(&user.name))
        .await
    {
        Ok(Some(detail)) => {
            if let Ok(job) = state.infra.enqueue_agent_request_job(detail.request.id).await {
                let infra = state.infra.clone();
                tokio::spawn(async move {
                    let _ = infra.process_job(job.id).await;
                });
            }
            let _ = state
                .infra
                .log_audit_event(
                    parse_user_uuid(&user),
                    "admin.agent_request.retry",
                    "agent_request",
                    Some(&request_id.to_string()),
                    json!({}),
                )
                .await;
            Json(detail).into_response()
        }
        Ok(None) => error_response(StatusCode::NOT_FOUND, "request not found"),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to retry request: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct AgentMoviePilotTestRequest {
    config: AgentConfig,
}

fn merge_agent_secret_placeholders(mut incoming: AgentConfig, current: &AgentConfig) -> AgentConfig {
    if incoming.moviepilot.password.trim() == "***" {
        incoming.moviepilot.password = current.moviepilot.password.clone();
    }
    incoming
}

async fn admin_get_agent_settings(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_super_admin(&state, &headers, &uri).await {
        return resp;
    }
    match state.infra.get_web_settings().await {
        Ok(mut settings) => {
            if !settings.agent.moviepilot.password.trim().is_empty() {
                settings.agent.moviepilot.password = "***".to_string();
            }
            Json(settings.agent).into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to load agent settings: {err}"),
        ),
    }
}

async fn admin_upsert_agent_settings(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(mut payload): Json<AgentConfig>,
) -> Response {
    let user = match require_super_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let mut settings = match state.infra.get_web_settings().await {
        Ok(value) => value,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to load current settings: {err}"),
            )
        }
    };
    payload = merge_agent_secret_placeholders(payload, &settings.agent);
    settings.agent = payload;
    match state.infra.upsert_web_settings(&settings).await {
        Ok(()) => {
            let _ = state
                .infra
                .log_audit_event(
                    parse_user_uuid(&user),
                    "admin.agent.settings.upsert",
                    "web_settings",
                    Some("global"),
                    json!({ "enabled": settings.agent.enabled }),
                )
                .await;
            let mut agent = settings.agent;
            if !agent.moviepilot.password.trim().is_empty() {
                agent.moviepilot.password = "***".to_string();
            }
            Json(agent).into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to persist agent settings: {err}"),
        ),
    }
}

async fn admin_test_agent_moviepilot(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(mut payload): Json<AgentMoviePilotTestRequest>,
) -> Response {
    if let Err(resp) = require_super_admin(&state, &headers, &uri).await {
        return resp;
    }
    let settings = match state.infra.get_web_settings().await {
        Ok(value) => value,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to load current settings: {err}"),
            )
        }
    };
    payload.config = merge_agent_secret_placeholders(payload.config, &settings.agent);
    match state.infra.test_moviepilot_connection(&payload.config).await {
        Ok(result) => Json(result).into_response(),
        Err(err) => error_response(
            StatusCode::BAD_REQUEST,
            &format!("moviepilot connection test failed: {err}"),
        ),
    }
}
