async fn admin_list_storage_configs(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<AdminStorageConfigQuery>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    let include_secrets = query.include_secrets.unwrap_or(false);
    match state.infra.list_storage_configs(include_secrets).await {
        Ok(configs) => Json(configs).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to list storage configs: {err}"),
        ),
    }
}

async fn admin_upsert_storage_config(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<AdminUpsertStorageConfigRequest>,
) -> Response {
    let user = match require_super_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let kind = payload.kind.trim();
    let name = payload.name.trim();
    if kind.is_empty() || name.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "kind/name is required");
    }

    let enabled = payload.enabled.unwrap_or(true);
    match state
        .infra
        .upsert_storage_config(kind, name, payload.config.clone(), enabled)
        .await
    {
        Ok(config) => {
            let _ = state
                .infra
                .log_audit_event(
                    parse_user_uuid(&user),
                    "admin.storage_config.upsert",
                    "storage_config",
                    Some(&config.id.to_string()),
                    json!({"kind": kind, "name": name, "enabled": enabled}),
                )
                .await;
            Json(config).into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to upsert storage config: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct AdminUpsertPlaybackDomainRequest {
    id: Option<Uuid>,
    name: String,
    base_url: String,
    enabled: Option<bool>,
    priority: Option<i32>,
    is_default: Option<bool>,
    #[serde(default)]
    lumenbackend_node_id: Option<Option<String>>,
    traffic_multiplier: Option<f64>,
}

async fn admin_list_playback_domains(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    match state.infra.list_playback_domains().await {
        Ok(items) => Json(items).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to list playback domains: {err}"),
        ),
    }
}

async fn admin_upsert_playback_domain(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<AdminUpsertPlaybackDomainRequest>,
) -> Response {
    let user = match require_super_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let patch = PlaybackDomainUpdate {
        name: payload.name,
        base_url: payload.base_url,
        enabled: payload.enabled.unwrap_or(true),
        priority: payload.priority.unwrap_or(0),
        is_default: payload.is_default.unwrap_or(false),
        lumenbackend_node_id: payload.lumenbackend_node_id,
        traffic_multiplier: payload.traffic_multiplier,
    };
    match state.infra.upsert_playback_domain(payload.id, patch).await {
        Ok(domain) => {
            let _ = state
                .infra
                .log_audit_event(
                    parse_user_uuid(&user),
                    "admin.playback_domain.upsert",
                    "playback_domain",
                    Some(&domain.id.to_string()),
                    json!({
                        "name": domain.name,
                        "base_url": domain.base_url,
                        "enabled": domain.enabled,
                        "priority": domain.priority,
                        "is_default": domain.is_default,
                        "lumenbackend_node_id": domain.lumenbackend_node_id,
                        "traffic_multiplier": domain.traffic_multiplier,
                    }),
                )
                .await;
            Json(domain).into_response()
        }
        Err(err) => {
            let msg = err.to_string();
            if msg.contains("playback domain not found") {
                return error_response(StatusCode::NOT_FOUND, "playback domain not found");
            }
            if msg.contains("required") || msg.contains("lumenbackend node not found") {
                return error_response(StatusCode::BAD_REQUEST, &msg);
            }
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to upsert playback domain: {err}"),
            )
        }
    }
}

async fn admin_delete_playback_domain(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    AxPath(domain_id): AxPath<Uuid>,
) -> Response {
    let user = match require_super_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    match state.infra.delete_playback_domain(domain_id).await {
        Ok(true) => {
            let _ = state
                .infra
                .log_audit_event(
                    parse_user_uuid(&user),
                    "admin.playback_domain.delete",
                    "playback_domain",
                    Some(&domain_id.to_string()),
                    json!({"deleted": true}),
                )
                .await;
            Json(json!({"deleted": true})).into_response()
        }
        Ok(false) => error_response(StatusCode::NOT_FOUND, "playback domain not found"),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to delete playback domain: {err}"),
        ),
    }
}

async fn admin_list_lumenbackend_nodes(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    match state.infra.list_lumenbackend_nodes().await {
        Ok(items) => Json(items).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to list lumenbackend nodes: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct AdminCreateLumenBackendNodeRequest {
    node_id: String,
    node_name: Option<String>,
    enabled: Option<bool>,
}

async fn admin_create_lumenbackend_node(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<AdminCreateLumenBackendNodeRequest>,
) -> Response {
    let user = match require_super_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let enabled = payload.enabled.unwrap_or(true);
    match state
        .infra
        .create_lumenbackend_node(payload.node_id.as_str(), payload.node_name.as_deref(), enabled)
        .await
    {
        Ok(node) => {
            let _ = state
                .infra
                .log_audit_event(
                    parse_user_uuid(&user),
                    "admin.lumenbackend.node.create",
                    "lumenbackend_node",
                    Some(node.node_id.as_str()),
                    json!({
                        "node_id": node.node_id,
                        "enabled": node.enabled,
                    }),
                )
                .await;
            Json(node).into_response()
        }
        Err(err) => {
            let msg = err.to_string();
            if msg.contains("required") {
                return error_response(StatusCode::BAD_REQUEST, &msg);
            }
            if msg.contains("duplicate key") {
                return error_response(StatusCode::CONFLICT, "lumenbackend node already exists");
            }
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to create lumenbackend node: {err}"),
            )
        }
    }
}

#[derive(Debug, Deserialize)]
struct AdminLumenBackendNodePath {
    node_id: String,
}

#[derive(Debug, Deserialize)]
struct AdminPatchLumenBackendNodeRequest {
    #[serde(default)]
    node_name: Option<Option<String>>,
    enabled: Option<bool>,
}

async fn admin_patch_lumenbackend_node(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    AxPath(path): AxPath<AdminLumenBackendNodePath>,
    Json(payload): Json<AdminPatchLumenBackendNodeRequest>,
) -> Response {
    let user = match require_super_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    match state
        .infra
        .update_lumenbackend_node(path.node_id.as_str(), payload.node_name, payload.enabled)
        .await
    {
        Ok(Some(node)) => {
            let _ = state
                .infra
                .log_audit_event(
                    parse_user_uuid(&user),
                    "admin.lumenbackend.node.update",
                    "lumenbackend_node",
                    Some(node.node_id.as_str()),
                    json!({
                        "node_id": node.node_id,
                        "enabled": node.enabled,
                    }),
                )
                .await;
            Json(node).into_response()
        }
        Ok(None) => error_response(StatusCode::NOT_FOUND, "lumenbackend node not found"),
        Err(err) => {
            let msg = err.to_string();
            if msg.contains("required") {
                return error_response(StatusCode::BAD_REQUEST, &msg);
            }
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to patch lumenbackend node: {err}"),
            )
        }
    }
}

async fn admin_delete_lumenbackend_node(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    AxPath(path): AxPath<AdminLumenBackendNodePath>,
) -> Response {
    let user = match require_super_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    match state.infra.delete_lumenbackend_node(path.node_id.as_str()).await {
        Ok(true) => {
            let _ = state
                .infra
                .log_audit_event(
                    parse_user_uuid(&user),
                    "admin.lumenbackend.node.delete",
                    "lumenbackend_node",
                    Some(path.node_id.as_str()),
                    json!({"deleted": true}),
                )
                .await;
            Json(json!({"deleted": true})).into_response()
        }
        Ok(false) => error_response(StatusCode::NOT_FOUND, "lumenbackend node not found"),
        Err(err) => {
            let msg = err.to_string();
            if msg.contains("bound") {
                return error_response(StatusCode::BAD_REQUEST, &msg);
            }
            if msg.contains("required") {
                return error_response(StatusCode::BAD_REQUEST, &msg);
            }
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to delete lumenbackend node: {err}"),
            )
        }
    }
}

#[derive(Debug, Deserialize)]
struct AdminLumenBackendNodeConfigQuery {
    include_secrets: Option<bool>,
}

async fn admin_get_lumenbackend_node_schema(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    AxPath(path): AxPath<AdminLumenBackendNodePath>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    match state
        .infra
        .get_lumenbackend_node_runtime_schema(path.node_id.as_str())
        .await
    {
        Ok(Some(schema)) => Json(schema).into_response(),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "runtime schema not reported"),
        Err(err) => {
            let msg = err.to_string();
            if msg.contains("not found") {
                return error_response(StatusCode::NOT_FOUND, &msg);
            }
            if msg.contains("required") {
                return error_response(StatusCode::BAD_REQUEST, &msg);
            }
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to load node runtime schema: {err}"),
            )
        }
    }
}

#[derive(Debug, Deserialize)]
struct AdminUpsertLumenBackendNodeConfigRequest {
    config: Value,
}

async fn admin_get_lumenbackend_node_config(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    AxPath(path): AxPath<AdminLumenBackendNodePath>,
    Query(query): Query<AdminLumenBackendNodeConfigQuery>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    let include_secrets = query.include_secrets.unwrap_or(false);
    match state
        .infra
        .get_lumenbackend_node_runtime_config(path.node_id.as_str(), include_secrets)
        .await
    {
        Ok(config) => Json(config).into_response(),
        Err(err) => {
            let msg = err.to_string();
            if msg.contains("not found") {
                return error_response(StatusCode::NOT_FOUND, &msg);
            }
            if msg.contains("required") {
                return error_response(StatusCode::BAD_REQUEST, &msg);
            }
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to load node runtime config: {err}"),
            )
        }
    }
}

async fn admin_upsert_lumenbackend_node_config(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    AxPath(path): AxPath<AdminLumenBackendNodePath>,
    Json(payload): Json<AdminUpsertLumenBackendNodeConfigRequest>,
) -> Response {
    let user = match require_super_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    match state
        .infra
        .upsert_lumenbackend_node_runtime_config(path.node_id.as_str(), payload.config)
        .await
    {
        Ok(config) => {
            let _ = state
                .infra
                .log_audit_event(
                    parse_user_uuid(&user),
                    "admin.lumenbackend.node_config.upsert",
                    "lumenbackend_node",
                    Some(path.node_id.as_str()),
                    json!({
                        "node_id": path.node_id,
                        "version": config.version,
                    }),
                )
                .await;
            Json(config).into_response()
        }
        Err(err) => {
            let msg = err.to_string();
            if msg.contains("required")
                || msg.contains("must be object")
                || msg.contains("runtime schema")
                || msg.contains("declared")
                || msg.contains("must be")
                || msg.contains("pattern")
            {
                return error_response(StatusCode::BAD_REQUEST, &msg);
            }
            if msg.contains("not found") {
                return error_response(StatusCode::NOT_FOUND, &msg);
            }
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to save node runtime config: {err}"),
            )
        }
    }
}

#[derive(Debug, Deserialize)]
struct LumenBackendRegisterRequest {
    node_id: String,
    node_name: Option<String>,
    node_version: Option<String>,
    capabilities: Option<Value>,
    listen_addr: Option<String>,
    runtime_schema_version: Option<String>,
    runtime_schema_hash: Option<String>,
    runtime_schema: Option<Value>,
}

#[derive(Debug, Serialize)]
struct LumenBackendRegisterResponse {
    accepted: bool,
    poll_interval_sec: u64,
    latest_config_version: i64,
    server_time_utc: String,
}

#[derive(Debug, Deserialize)]
struct LumenBackendHeartbeatRequest {
    node_id: String,
    node_version: Option<String>,
    active_streams: Option<u64>,
    cpu_usage: Option<f64>,
    memory_usage: Option<f64>,
    last_error: Option<String>,
    last_config_version: Option<i64>,
}

#[derive(Debug, Serialize)]
struct LumenBackendHeartbeatResponse {
    config_changed: bool,
    latest_config_version: i64,
    next_poll_after_sec: u64,
}

#[derive(Debug, Deserialize)]
struct LumenBackendRuntimeConfigQuery {
    node_id: String,
    since_version: Option<i64>,
}

#[derive(Debug, Serialize)]
struct LumenBackendRuntimeConfigResponse {
    version: i64,
    changed: bool,
    config: Value,
}

#[derive(Debug, Deserialize)]
struct LumenBackendTrafficRecord {
    user_id: Uuid,
    item_id: Option<Uuid>,
    bytes_served: u64,
    started_at: Option<DateTime<Utc>>,
    ended_at: Option<DateTime<Utc>>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LumenBackendTrafficReportRequest {
    node_id: String,
    records: Vec<LumenBackendTrafficRecord>,
}

#[derive(Debug, Serialize)]
struct LumenBackendTrafficReportResponse {
    accepted_count: usize,
    rejected_count: usize,
}

async fn lumenbackend_register(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<LumenBackendRegisterRequest>,
) -> Response {
    if let Err(resp) = require_lumenbackend_api_key(&state, &headers, &uri).await {
        return resp;
    }

    let node_id = payload.node_id.trim();
    if node_id.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "node_id is required");
    }
    let current_node = match state.infra.get_lumenbackend_node(node_id).await {
        Ok(Some(node)) => node,
        Ok(None) => {
            return error_response(
                StatusCode::NOT_FOUND,
                "node not registered, create it from /admin/playback first",
            );
        }
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to query lumenbackend node: {err}"),
            );
        }
    };
    if !current_node.enabled {
        return error_response(StatusCode::FORBIDDEN, "node is disabled");
    }

    let status = json!({
        "capabilities": payload.capabilities,
        "listen_addr": payload.listen_addr,
        "runtime_schema_version": payload.runtime_schema_version,
        "runtime_schema_hash": payload.runtime_schema_hash,
    });
    let node = match state
        .infra
        .register_lumenbackend_node(LumenBackendNodeRegister {
            node_id: node_id.to_string(),
            name: payload.node_name.clone(),
            version: payload.node_version.clone(),
            status,
        })
        .await
    {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("failed to register lumenbackend node: {err}"),
            );
        }
    };

    if let Some(runtime_schema) = payload.runtime_schema.as_ref() {
        let schema_version = payload
            .runtime_schema_version
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let Some(schema_version) = schema_version else {
            return error_response(
                StatusCode::BAD_REQUEST,
                "runtime_schema_version is required when runtime_schema is provided",
            );
        };
        if let Err(err) = state
            .infra
            .upsert_lumenbackend_node_runtime_schema(
                node_id,
                schema_version,
                payload.runtime_schema_hash.as_deref(),
                runtime_schema.clone(),
            )
            .await
        {
            let msg = err.to_string();
            if msg.contains("required")
                || msg.contains("runtime schema")
                || msg.contains("must be object")
            {
                return error_response(StatusCode::BAD_REQUEST, &msg);
            }
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to save runtime schema: {err}"),
            );
        }
    }

    let runtime = match state
        .infra
        .get_lumenbackend_runtime_config(node_id)
        .await
    {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to resolve runtime config: {err}"),
            );
        }
    };

    let _ = state
        .infra
        .log_audit_event(
            None,
            "internal.lumenbackend.register",
            "lumenbackend_node",
            Some(node.node_id.as_str()),
            json!({"version": node.last_version}),
        )
        .await;

    Json(LumenBackendRegisterResponse {
        accepted: true,
        poll_interval_sec: 10,
        latest_config_version: runtime.version,
        server_time_utc: Utc::now().to_rfc3339(),
    })
    .into_response()
}

async fn lumenbackend_heartbeat(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<LumenBackendHeartbeatRequest>,
) -> Response {
    if let Err(resp) = require_lumenbackend_api_key(&state, &headers, &uri).await {
        return resp;
    }
    let node_id = payload.node_id.trim();
    if node_id.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "node_id is required");
    }

    match state.infra.get_lumenbackend_node(node_id).await {
        Ok(Some(node)) => {
            if !node.enabled {
                return error_response(StatusCode::FORBIDDEN, "node is disabled");
            }
        }
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "node not registered"),
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to query lumenbackend node: {err}"),
            );
        }
    }

    let status = json!({
        "active_streams": payload.active_streams.unwrap_or(0),
        "cpu_usage": payload.cpu_usage,
        "memory_usage": payload.memory_usage,
        "last_error": payload.last_error,
    });
    let node = match state
        .infra
        .heartbeat_lumenbackend_node(LumenBackendNodeHeartbeat {
            node_id: node_id.to_string(),
            version: payload.node_version,
            status,
        })
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "node not registered"),
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to update heartbeat: {err}"),
            );
        }
    };

    let runtime = match state.infra.get_lumenbackend_runtime_config(node_id).await {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to resolve runtime config: {err}"),
            );
        }
    };

    let _ = state
        .infra
        .log_audit_event(
            None,
            "internal.lumenbackend.heartbeat",
            "lumenbackend_node",
            Some(node.node_id.as_str()),
            json!({
                "active_streams": payload.active_streams.unwrap_or(0),
                "config_version": runtime.version,
            }),
        )
        .await;

    Json(LumenBackendHeartbeatResponse {
        config_changed: payload
            .last_config_version
            .map(|v| v < runtime.version)
            .unwrap_or(true),
        latest_config_version: runtime.version,
        next_poll_after_sec: 10,
    })
    .into_response()
}

async fn lumenbackend_runtime_config(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<LumenBackendRuntimeConfigQuery>,
) -> Response {
    if let Err(resp) = require_lumenbackend_api_key(&state, &headers, &uri).await {
        return resp;
    }

    if query.node_id.trim().is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "node_id is required");
    }

    match state.infra.get_lumenbackend_node(query.node_id.as_str()).await {
        Ok(Some(node)) => {
            if !node.enabled {
                return error_response(StatusCode::FORBIDDEN, "node is disabled");
            }
        }
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "node not registered"),
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to query lumenbackend node: {err}"),
            );
        }
    }

    match state
        .infra
        .get_lumenbackend_runtime_config(query.node_id.as_str())
        .await
    {
        Ok(runtime) => Json(LumenBackendRuntimeConfigResponse {
            version: runtime.version,
            changed: query
                .since_version
                .map(|v| v < runtime.version)
                .unwrap_or(true),
            config: runtime.config,
        })
        .into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to resolve runtime config: {err}"),
        ),
    }
}

async fn lumenbackend_report_traffic(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<LumenBackendTrafficReportRequest>,
) -> Response {
    if let Err(resp) = require_lumenbackend_api_key(&state, &headers, &uri).await {
        return resp;
    }
    let node_id = payload.node_id.trim();
    if node_id.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "node_id is required");
    }

    match state.infra.get_lumenbackend_node(node_id).await {
        Ok(Some(node)) => {
            if !node.enabled {
                return error_response(StatusCode::FORBIDDEN, "node is disabled");
            }
        }
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "node not registered"),
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to query lumenbackend node: {err}"),
            );
        }
    }

    let traffic_multiplier = match state
        .infra
        .resolve_playback_domain_for_lumenbackend_node(node_id)
        .await
    {
        Ok(Some(domain)) => domain.traffic_multiplier,
        Ok(None) => 1.0,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to resolve traffic multiplier: {err}"),
            );
        }
    };

    let mut accepted_count = 0usize;
    let mut rejected_count = 0usize;
    for record in payload.records {
        let bytes_served = record.bytes_served;
        if bytes_served <= 0 {
            continue;
        }

        match state
            .infra
            .record_user_stream_bytes(record.user_id, record.item_id, bytes_served, traffic_multiplier)
            .await
        {
            Ok(()) => accepted_count += 1,
            Err(err) => {
                rejected_count += 1;
                warn!(
                    node_id = %node_id,
                    user_id = %record.user_id,
                    item_id = ?record.item_id,
                    traffic_multiplier,
                    started_at = ?record.started_at,
                    ended_at = ?record.ended_at,
                    status = ?record.status,
                    error = %err,
                    "failed to record traffic report"
                );
            }
        }
    }

    Json(LumenBackendTrafficReportResponse {
        accepted_count,
        rejected_count,
    })
    .into_response()
}

#[derive(Debug, Deserialize)]
struct CreatePlaylistRequest {
    name: String,
    description: Option<String>,
    #[serde(default)]
    is_public: bool,
}

#[derive(Debug, Deserialize)]
struct UpdatePlaylistRequest {
    name: Option<String>,
    description: Option<String>,
    is_public: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct AddPlaylistItemRequest {
    item_id: Uuid,
}
