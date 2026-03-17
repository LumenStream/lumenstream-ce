async fn admin_create_library(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<AdminCreateLibraryRequest>,
) -> Response {
    let user = match require_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let library_type = match normalize_library_type(payload.library_type.as_deref()) {
        Some(value) => value,
        None => return error_response(StatusCode::BAD_REQUEST, "invalid library_type"),
    };
    let paths = resolve_create_library_paths(payload.paths.as_deref(), payload.root_path.as_deref());
    if paths.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "paths is required");
    }

    info!(user = %user.name, name = %payload.name, "admin create library");

    match state
        .infra
        .create_library(&payload.name, &paths, library_type)
        .await
    {
        Ok(library) => {
            if let Err(err) = state
                .infra
                .log_audit_event(
                    parse_user_uuid(&user),
                    "admin.library.create",
                    "library",
                    Some(&library.id.to_string()),
                    json!({
                        "name": payload.name,
                        "root_path": library.root_path.clone(),
                        "paths": library.paths.clone(),
                        "library_type": library.library_type.clone(),
                    }),
                )
                .await
            {
                error!(error = %err, "failed to write audit log for create library");
            }
            Json(library).into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to create library: {err}"),
        ),
    }
}

async fn admin_list_libraries(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    match state.infra.list_libraries().await {
        Ok(libraries) => Json(libraries).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to list libraries: {err}"),
        ),
    }
}

#[derive(Debug, Serialize)]
struct AdminLibraryStatusDto {
    id: Uuid,
    name: String,
    root_path: String,
    paths: Vec<String>,
    library_type: String,
    enabled: bool,
    item_count: i64,
    last_item_updated_at: Option<String>,
}

#[derive(Debug, Serialize)]
struct AdminLibraryStatusResponse {
    total: i64,
    enabled: i64,
    items: Vec<AdminLibraryStatusDto>,
}

async fn admin_list_library_status(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    let libraries = match state.infra.list_libraries().await {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to list libraries: {err}"),
            );
        }
    };

    let stats = match state.infra.list_library_item_stats().await {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to query library stats: {err}"),
            );
        }
    };

    let stat_map = stats
        .into_iter()
        .map(|stat| {
            (
                stat.library_id,
                (
                    stat.item_count,
                    stat.last_item_updated_at.map(|v| v.to_rfc3339()),
                ),
            )
        })
        .collect::<HashMap<_, _>>();

    let enabled = libraries.iter().filter(|lib| lib.enabled).count() as i64;
    let items = libraries
        .into_iter()
        .map(|library| {
            let (item_count, last_item_updated_at) =
                stat_map.get(&library.id).cloned().unwrap_or((0, None));

            AdminLibraryStatusDto {
                id: library.id,
                name: library.name,
                root_path: library.root_path,
                paths: library.paths,
                library_type: library.library_type,
                enabled: library.enabled,
                item_count,
                last_item_updated_at,
            }
        })
        .collect::<Vec<_>>();

    Json(AdminLibraryStatusResponse {
        total: items.len() as i64,
        enabled,
        items,
    })
    .into_response()
}

async fn admin_disable_library(
    State(state): State<ApiContext>,
    AxPath(library_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    admin_set_library_enabled(state, library_id, false, headers, uri).await
}

async fn admin_enable_library(
    State(state): State<ApiContext>,
    AxPath(library_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    admin_set_library_enabled(state, library_id, true, headers, uri).await
}

#[derive(Debug, Deserialize, Default)]
struct AdminPatchLibraryRequest {
    name: Option<String>,
    library_type: Option<String>,
    paths: Option<Vec<String>>,
    scraper_policy: Option<Value>,
}

async fn admin_set_library_enabled(
    state: ApiContext,
    library_id: Uuid,
    enabled: bool,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let user = match require_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let updated = match state.infra.set_library_enabled(library_id, enabled).await {
        Ok(Some(v)) => v,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "library not found"),
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to update library status: {err}"),
            );
        }
    };

    if let Err(err) = state
        .infra
        .log_audit_event(
            parse_user_uuid(&user),
            if enabled {
                "admin.library.enable"
            } else {
                "admin.library.disable"
            },
            "library",
            Some(&updated.id.to_string()),
            json!({"enabled": enabled}),
        )
        .await
    {
        error!(error = %err, "failed to write audit log for library status");
    }

    Json(updated).into_response()
}

async fn admin_patch_library(
    State(state): State<ApiContext>,
    AxPath(library_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<AdminPatchLibraryRequest>,
) -> Response {
    let user = match require_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    if payload.name.is_none()
        && payload.library_type.is_none()
        && payload.paths.is_none()
        && payload.scraper_policy.is_none()
    {
        return error_response(StatusCode::BAD_REQUEST, "at least one field is required");
    }

    // Update name if provided
    if let Some(ref name) = payload.name {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return error_response(StatusCode::BAD_REQUEST, "name cannot be empty");
        }
        match state.infra.update_library_name(library_id, trimmed).await {
            Ok(Some(_)) => {}
            Ok(None) => return error_response(StatusCode::NOT_FOUND, "library not found"),
            Err(err) => {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("failed to update library name: {err}"),
                );
            }
        }
    }

    // Update library_type if provided
    if let Some(ref raw_library_type) = payload.library_type {
        let Some(library_type) = normalize_library_type(Some(raw_library_type)) else {
            return error_response(StatusCode::BAD_REQUEST, "invalid library_type");
        };
        match state.infra.update_library_type(library_id, library_type).await {
            Ok(Some(_)) => {}
            Ok(None) => return error_response(StatusCode::NOT_FOUND, "library not found"),
            Err(err) => {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("failed to update library type: {err}"),
                );
            }
        }
    }

    // Update paths if provided
    if let Some(ref paths) = payload.paths {
        let cleaned: Vec<String> = paths
            .iter()
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty())
            .collect();
        if cleaned.is_empty() {
            return error_response(StatusCode::BAD_REQUEST, "paths cannot be empty");
        }
        match state.infra.replace_library_paths(library_id, &cleaned).await {
            Ok(Some(_)) => {}
            Ok(None) => return error_response(StatusCode::NOT_FOUND, "library not found"),
            Err(err) => {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("failed to update library paths: {err}"),
                );
            }
        }
    }

    if let Some(ref scraper_policy) = payload.scraper_policy {
        match state
            .infra
            .update_library_scraper_policy(library_id, scraper_policy)
            .await
        {
            Ok(Some(_)) => {}
            Ok(None) => return error_response(StatusCode::NOT_FOUND, "library not found"),
            Err(err) => {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("failed to update library scraper policy: {err}"),
                );
            }
        }
    }

    // Re-fetch the final state
    let updated = match state.infra.get_library_by_id(library_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "library not found"),
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to fetch library: {err}"),
            );
        }
    };

    if let Err(err) = state
        .infra
        .log_audit_event(
            parse_user_uuid(&user),
            "admin.library.update",
            "library",
            Some(&updated.id.to_string()),
            json!({
                "name": updated.name,
                "library_type": updated.library_type,
                "paths": updated.paths,
                "scraper_policy": updated.scraper_policy,
            }),
        )
        .await
    {
        error!(error = %err, "failed to write audit log for library update");
    }

    Json(updated).into_response()
}

fn normalize_library_type(raw: Option<&str>) -> Option<&'static str> {
    let Some(raw) = raw.map(str::trim) else {
        return Some("Mixed");
    };
    if raw.is_empty() {
        return Some("Mixed");
    }
    if raw.eq_ignore_ascii_case("movie") || raw.eq_ignore_ascii_case("movies") {
        return Some("Movie");
    }
    if raw.eq_ignore_ascii_case("series")
        || raw.eq_ignore_ascii_case("show")
        || raw.eq_ignore_ascii_case("shows")
        || raw.eq_ignore_ascii_case("tv")
        || raw.eq_ignore_ascii_case("tvshows")
    {
        return Some("Series");
    }
    if raw.eq_ignore_ascii_case("mixed") {
        return Some("Mixed");
    }
    None
}

fn resolve_create_library_paths(
    paths: Option<&[String]>,
    root_path: Option<&str>,
) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(paths) = paths {
        out.extend(
            paths
                .iter()
                .map(|path| path.trim().to_string())
                .filter(|path| !path.is_empty()),
        );
    }
    if let Some(root_path) = root_path.map(str::trim).filter(|path| !path.is_empty()) {
        if !out.iter().any(|existing| existing == root_path) {
            out.push(root_path.to_string());
        }
    }
    out
}

fn spawn_process_job(infra: Arc<AppInfra>, job_id: Uuid) {
    tokio::spawn(async move {
        if let Err(err) = infra.process_job(job_id).await {
            error!(error = %err, %job_id, "job process crashed");
        }
    });
}

async fn admin_list_task_definitions(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    match state.infra.list_task_definitions().await {
        Ok(tasks) => Json(tasks).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to list task definitions: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize, Default)]
struct AdminPatchTaskDefinitionRequest {
    enabled: Option<bool>,
    cron_expr: Option<String>,
    default_payload: Option<Value>,
    max_attempts: Option<i32>,
}

async fn admin_patch_task_definition(
    State(state): State<ApiContext>,
    AxPath(task_key): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<AdminPatchTaskDefinitionRequest>,
) -> Response {
    let user = match require_super_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    if let Some(cron_expr) = payload.cron_expr.as_ref() {
        let cron_expr = cron_expr.trim();
        if cron_expr.is_empty() {
            return error_response(StatusCode::BAD_REQUEST, "cron_expr is required");
        }
        if let Err(err) = Schedule::from_str(cron_expr) {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("invalid cron_expr: {err}"),
            );
        }
    }

    if let Some(default_payload) = payload.default_payload.as_ref() {
        if !default_payload.is_object() {
            return error_response(
                StatusCode::BAD_REQUEST,
                "default_payload must be a JSON object",
            );
        }
    }

    let patch = TaskDefinitionUpdate {
        enabled: payload.enabled,
        cron_expr: payload.cron_expr.map(|value| value.trim().to_string()),
        default_payload: payload.default_payload,
        max_attempts: payload.max_attempts,
    };

    match state.infra.update_task_definition(&task_key, patch).await {
        Ok(Some(task)) => {
            let _ = state
                .infra
                .log_audit_event(
                    parse_user_uuid(&user),
                    "admin.task_center.task.update",
                    "task_definition",
                    Some(&task_key),
                    json!({}),
                )
                .await;
            Json(task).into_response()
        }
        Ok(None) => error_response(StatusCode::NOT_FOUND, "task not found"),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to update task definition: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct AdminRunTaskRequest {
    payload_override: Option<Value>,
}

async fn admin_run_task_now(
    State(state): State<ApiContext>,
    AxPath(task_key): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<AdminRunTaskRequest>,
) -> Response {
    let user = match require_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    if let Some(payload_override) = payload.payload_override.as_ref() {
        if !payload_override.is_object() {
            return error_response(
                StatusCode::BAD_REQUEST,
                "payload_override must be a JSON object",
            );
        }
    }

    match state
        .infra
        .run_task_now(&task_key, payload.payload_override.clone())
        .await
    {
        Ok(Some(job)) => {
            let run_id = job.id;
            spawn_process_job(state.infra.clone(), run_id);
            let _ = state
                .infra
                .log_audit_event(
                    parse_user_uuid(&user),
                    "admin.task_center.run",
                    "task_run",
                    Some(&run_id.to_string()),
                    json!({ "task_key": task_key }),
                )
                .await;
            Json(job).into_response()
        }
        Ok(None) => error_response(StatusCode::NOT_FOUND, "task not found"),
        Err(err) => map_task_center_error(&err).unwrap_or_else(|| {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to run task: {err}"),
            )
        }),
    }
}

#[derive(Debug, Deserialize)]
struct AdminTaskRunsQuery {
    limit: Option<i64>,
    task_key: Option<String>,
    status: Option<String>,
    trigger_type: Option<String>,
    /// Comma-separated list of task kinds to exclude, e.g. "retry_dispatch,cleanup_maintenance"
    exclude_kinds: Option<String>,
}

async fn admin_list_task_runs(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<AdminTaskRunsQuery>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 500);
    let exclude_vec: Vec<&str> = query
        .exclude_kinds
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|s| s.split(',').map(str::trim).collect())
        .unwrap_or_default();
    match state
        .infra
        .list_task_runs(
            limit,
            query.task_key.as_deref(),
            query.status.as_deref(),
            query.trigger_type.as_deref(),
            &exclude_vec,
        )
        .await
    {
        Ok(runs) => Json(runs).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to list task runs: {err}"),
        ),
    }
}

async fn admin_get_task_run(
    State(state): State<ApiContext>,
    AxPath(run_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    match state.infra.get_task_run(run_id).await {
        Ok(Some(run)) => Json(run).into_response(),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "task run not found"),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to get task run: {err}"),
        ),
    }
}

async fn admin_cancel_task_run(
    State(state): State<ApiContext>,
    AxPath(run_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let user = match require_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    match state.infra.cancel_task_run(run_id).await {
        Ok(Some(run)) => {
            let _ = state
                .infra
                .log_audit_event(
                    parse_user_uuid(&user),
                    "admin.task_center.run.cancel",
                    "task_run",
                    Some(&run_id.to_string()),
                    json!({}),
                )
                .await;
            Json(run).into_response()
        }
        Ok(None) => error_response(StatusCode::NOT_FOUND, "task run not found"),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to cancel task run: {err}"),
        ),
    }
}

async fn admin_task_runs_ws(
    State(state): State<ApiContext>,
    req: HttpRequest,
    body: web::Payload,
) -> Response {
    let headers = HeaderMap(req.headers().clone());
    let uri = Uri(req.uri().clone());
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    let (response, mut session, mut msg_stream) = match actix_ws::handle(&req, body) {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to upgrade websocket: {err}"),
            );
        }
    };
    let mut rx = state.infra.subscribe_task_runs();

    actix_web::rt::spawn(async move {
        let mut heartbeat = tokio::time::interval(std::time::Duration::from_secs(20));
        loop {
            tokio::select! {
                _ = heartbeat.tick() => {
                    if session.ping(b"heartbeat").await.is_err() {
                        break;
                    }
                }
                maybe_msg = msg_stream.next() => {
                    match maybe_msg {
                        Some(Ok(actix_ws::Message::Close(_))) => break,
                        Some(Ok(actix_ws::Message::Ping(bytes))) => {
                            if session.pong(&bytes).await.is_err() {
                                break;
                            }
                        }
                        Some(Ok(_)) => {}
                        Some(Err(_)) => break,
                        None => break,
                    }
                }
                recv = rx.recv() => {
                    match recv {
                        Ok(event) => {
                            if let Ok(text) = serde_json::to_string(&event) {
                                if session.text(text).await.is_err() {
                                    break;
                                }
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
            }
        }
        let _ = session.close(None).await;
    });

    response
}

#[derive(Debug, Deserialize)]
struct AdminCreateUserRequest {
    username: String,
    password: String,
    role: Option<String>,
    is_admin: Option<bool>,
}
use futures_util::StreamExt;
