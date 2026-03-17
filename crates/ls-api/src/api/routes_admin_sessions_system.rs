async fn admin_list_sessions(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<AdminSessionsQuery>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    let limit = query.limit.unwrap_or(100).clamp(1, 500);
    let active_only = query.active_only.unwrap_or(false);
    match state.infra.list_playback_sessions(limit, active_only).await {
        Ok(sessions) => Json(sessions).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to list sessions: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct AdminApiKeysQuery {
    limit: Option<i64>,
}

async fn admin_list_auth_sessions(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<AdminSessionsQuery>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    let limit = query.limit.unwrap_or(100).clamp(1, 1000);
    let active_only = query.active_only.unwrap_or(false);

    match state.infra.list_auth_sessions(limit, active_only).await {
        Ok(sessions) => Json(sessions).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to list auth sessions: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct AdminCreateApiKeyRequest {
    name: String,
}

async fn admin_list_api_keys(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<AdminApiKeysQuery>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    let limit = query.limit.unwrap_or(100).clamp(1, 1000);

    match state.infra.list_admin_api_keys(limit).await {
        Ok(keys) => Json(keys).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to list api keys: {err}"),
        ),
    }
}

async fn admin_create_api_key(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<AdminCreateApiKeyRequest>,
) -> Response {
    let user = match require_super_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let name = payload.name.trim();
    if name.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "name is required");
    }

    match state.infra.create_admin_api_key(name).await {
        Ok(created) => {
            let _ = state
                .infra
                .log_audit_event(
                    parse_user_uuid(&user),
                    "admin.api_key.create",
                    "admin_api_key",
                    Some(&created.id.to_string()),
                    json!({"name": name}),
                )
                .await;
            Json(created).into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to create api key: {err}"),
        ),
    }
}

async fn admin_delete_api_key(
    State(state): State<ApiContext>,
    AxPath(key_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let user = match require_super_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    match state.infra.delete_admin_api_key(key_id).await {
        Ok(true) => {
            let _ = state
                .infra
                .log_audit_event(
                    parse_user_uuid(&user),
                    "admin.api_key.delete",
                    "admin_api_key",
                    Some(&key_id.to_string()),
                    json!({}),
                )
                .await;
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(false) => error_response(StatusCode::NOT_FOUND, "api key not found"),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to delete api key: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct AdminSettingsQuery {
    include_secrets: Option<bool>,
}

#[derive(Debug, Serialize)]
struct AdminUpsertSettingsResponse {
    settings: WebAppConfig,
    restart_required: bool,
}

async fn admin_get_settings(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<AdminSettingsQuery>,
) -> Response {
    if let Err(resp) = require_super_admin(&state, &headers, &uri).await {
        return resp;
    }

    match state.infra.get_web_settings().await {
        Ok(settings) => {
            let include_secrets = query.include_secrets.unwrap_or(false);
            let body = if include_secrets {
                settings
            } else {
                mask_web_settings(settings)
            };
            Json(body).into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to load settings: {err}"),
        ),
    }
}

async fn admin_upsert_settings(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<WebAppConfig>,
) -> Response {
    let user = match require_super_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let current = match state.infra.get_web_settings().await {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to load current settings: {err}"),
            );
        }
    };

    let merged = merge_secret_placeholders(payload, &current);
    let restart_required = web_settings_restart_required(&current, &merged);
    if merged.auth.bootstrap_admin_user.trim().is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "auth.bootstrap_admin_user is required",
        );
    }

    match state.infra.upsert_web_settings(&merged).await {
        Ok(()) => {
            let _ = state
                .infra
                .log_audit_event(
                    parse_user_uuid(&user),
                    "admin.settings.upsert",
                    "web_settings",
                    Some("global"),
                    json!({"restart_required": restart_required}),
                )
                .await;

            Json(AdminUpsertSettingsResponse {
                settings: mask_web_settings(merged),
                restart_required,
            })
            .into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to persist settings: {err}"),
        ),
    }
}

#[derive(Debug, Serialize)]
struct AdminSystemFlagsResponse {
    strm_only_streaming: bool,
    transcoding_enabled: bool,
    scraper_enabled: bool,
    tmdb_enabled: bool,
    lumenbackend_enabled: bool,
    prefer_segment_gateway: bool,
    metrics_enabled: bool,
}

#[derive(Debug, Deserialize)]
struct AdminUpdateSystemFlagsRequest {
    scraper_enabled: Option<bool>,
    tmdb_enabled: Option<bool>,
    lumenbackend_enabled: Option<bool>,
    prefer_segment_gateway: Option<bool>,
    metrics_enabled: Option<bool>,
}

#[derive(Debug, Serialize)]
struct AdminSystemCapabilitiesResponse {
    edition: String,
    strm_only_streaming: bool,
    transcoding_enabled: bool,
    billing_enabled: bool,
    advanced_traffic_controls_enabled: bool,
    invite_rewards_enabled: bool,
    audit_log_export_enabled: bool,
    request_agent_enabled: bool,
    playback_routing_enabled: bool,
    supported_stream_features: Vec<String>,
}

#[derive(Debug, Serialize)]
struct AdminSystemSummaryResponse {
    generated_at_utc: String,
    server_id: String,
    transcoding_enabled: bool,
    libraries_total: i64,
    libraries_enabled: i64,
    media_items_total: i64,
    users_total: i64,
    users_disabled: i64,
    active_playback_sessions: i64,
    active_auth_sessions: i64,
    jobs_by_status: HashMap<String, i64>,
    infra_metrics: Value,
}

async fn admin_get_system_flags(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    match state.infra.get_web_settings().await {
        Ok(settings) => Json(build_system_flags_response(&settings)).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to load system flags: {err}"),
        ),
    }
}

async fn admin_update_system_flags(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<AdminUpdateSystemFlagsRequest>,
) -> Response {
    let user = match require_super_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let mut settings = match state.infra.get_web_settings().await {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to load current settings: {err}"),
            );
        }
    };

    apply_system_flags_update(&mut settings, &payload);

    match state.infra.upsert_web_settings(&settings).await {
        Ok(()) => {
            let _ = state
                .infra
                .log_audit_event(
                    parse_user_uuid(&user),
                    "admin.system.flags.upsert",
                    "web_settings",
                    Some("global"),
                    json!({
                        "scraper_enabled": settings.scraper.enabled || settings.tmdb.enabled,
                        "tmdb_enabled": settings.tmdb.enabled,
                        "lumenbackend_enabled": settings.storage.lumenbackend_enabled,
                        "prefer_segment_gateway": settings.storage.prefer_segment_gateway,
                        "metrics_enabled": settings.observability.metrics_enabled,
                    }),
                )
                .await;

            Json(build_system_flags_response(&settings)).into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to persist system flags: {err}"),
        ),
    }
}

async fn admin_get_system_capabilities(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    Json(build_system_capabilities_response(&state)).into_response()
}

async fn get_public_system_capabilities(State(state): State<ApiContext>) -> Response {
    Json(build_system_capabilities_response(&state)).into_response()
}

fn build_system_capabilities_response(state: &ApiContext) -> AdminSystemCapabilitiesResponse {
    let edition = state.infra.config_snapshot().edition_capabilities();
    AdminSystemCapabilitiesResponse {
        edition: edition.edition,
        strm_only_streaming: true,
        transcoding_enabled: false,
        billing_enabled: edition.billing_enabled,
        advanced_traffic_controls_enabled: edition.advanced_traffic_controls_enabled,
        invite_rewards_enabled: edition.invite_rewards_enabled,
        audit_log_export_enabled: edition.audit_log_export_enabled,
        request_agent_enabled: edition.request_agent_enabled,
        playback_routing_enabled: edition.playback_routing_enabled,
        supported_stream_features: vec![
            "strm-direct-play".to_string(),
            "http-range".to_string(),
            "segment-gateway".to_string(),
            "distributed-fallback".to_string(),
            "lumenbackend-302-redirect".to_string(),
        ],
    }
}

async fn admin_get_system_summary(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    let libraries_total = match state.infra.count_libraries_total().await {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to count libraries: {err}"),
            );
        }
    };

    let libraries_enabled = match state.infra.count_libraries_enabled().await {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to count enabled libraries: {err}"),
            );
        }
    };

    let media_items_total = match state.infra.count_media_items_total().await {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to count media items: {err}"),
            );
        }
    };

    let users_total = match state.infra.count_users_total().await {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to count users: {err}"),
            );
        }
    };

    let users_disabled = match state.infra.count_users_disabled().await {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to count disabled users: {err}"),
            );
        }
    };

    let active_playback_sessions = match state.infra.count_active_playback_sessions().await {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to count playback sessions: {err}"),
            );
        }
    };

    let active_auth_sessions = match state.infra.count_active_auth_sessions().await {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to count auth sessions: {err}"),
            );
        }
    };

    let jobs_by_status = match state.infra.list_job_status_counts().await {
        Ok(items) => items
            .into_iter()
            .map(|item| (item.status, item.count))
            .collect::<HashMap<_, _>>(),
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to count jobs by status: {err}"),
            );
        }
    };

    Json(AdminSystemSummaryResponse {
        generated_at_utc: Utc::now().to_rfc3339(),
        server_id: state.infra.server_id.clone(),
        transcoding_enabled: false,
        libraries_total,
        libraries_enabled,
        media_items_total,
        users_total,
        users_disabled,
        active_playback_sessions,
        active_auth_sessions,
        jobs_by_status,
        infra_metrics: state.infra.runtime_metrics_snapshot(),
    })
    .into_response()
}

fn build_system_flags_response(settings: &WebAppConfig) -> AdminSystemFlagsResponse {
    let scraper_enabled = settings.scraper.enabled || settings.tmdb.enabled;
    AdminSystemFlagsResponse {
        strm_only_streaming: true,
        transcoding_enabled: false,
        scraper_enabled,
        tmdb_enabled: scraper_enabled,
        lumenbackend_enabled: settings.storage.lumenbackend_enabled,
        prefer_segment_gateway: settings.storage.prefer_segment_gateway,
        metrics_enabled: settings.observability.metrics_enabled,
    }
}

fn apply_system_flags_update(settings: &mut WebAppConfig, payload: &AdminUpdateSystemFlagsRequest) {
    if let Some(v) = payload.scraper_enabled.or(payload.tmdb_enabled) {
        settings.scraper.enabled = v;
        settings.tmdb.enabled = v;
    }
    if let Some(v) = payload.lumenbackend_enabled {
        settings.storage.lumenbackend_enabled = v;
    }
    if let Some(v) = payload.prefer_segment_gateway {
        settings.storage.prefer_segment_gateway = v;
    }
    if let Some(v) = payload.metrics_enabled {
        settings.observability.metrics_enabled = v;
    }
}

fn web_settings_restart_required(current: &WebAppConfig, updated: &WebAppConfig) -> bool {
    current.server.host != updated.server.host
        || current.server.port != updated.server.port
        || current.server.cors_allow_origins != updated.server.cors_allow_origins
        || current.scheduler.enabled != updated.scheduler.enabled
        || current.scheduler.cleanup_interval_seconds != updated.scheduler.cleanup_interval_seconds
        || current.scheduler.job_retry_interval_seconds != updated.scheduler.job_retry_interval_seconds
}

fn mask_web_settings(mut settings: WebAppConfig) -> WebAppConfig {
    if !settings.auth.bootstrap_admin_password.trim().is_empty() {
        settings.auth.bootstrap_admin_password = "***".to_string();
    }

    if !settings.tmdb.api_key.trim().is_empty() {
        settings.tmdb.api_key = "***".to_string();
    }
    if !settings.scraper.tvdb.api_key.trim().is_empty() {
        settings.scraper.tvdb.api_key = "***".to_string();
    }
    if !settings.scraper.tvdb.pin.trim().is_empty() {
        settings.scraper.tvdb.pin = "***".to_string();
    }
    if !settings.scraper.bangumi.access_token.trim().is_empty() {
        settings.scraper.bangumi.access_token = "***".to_string();
    }

    if !settings.billing.epay.key.trim().is_empty() {
        settings.billing.epay.key = "***".to_string();
    }

    if !settings.agent.moviepilot.password.trim().is_empty() {
        settings.agent.moviepilot.password = "***".to_string();
    }

    if !settings
        .storage
        .lumenbackend_stream_signing_key
        .trim()
        .is_empty()
    {
        settings.storage.lumenbackend_stream_signing_key = "***".to_string();
    }

    settings
}

fn merge_secret_placeholders(mut incoming: WebAppConfig, current: &WebAppConfig) -> WebAppConfig {
    if incoming.auth.bootstrap_admin_password.trim() == "***" {
        incoming.auth.bootstrap_admin_password = current.auth.bootstrap_admin_password.clone();
    }

    if incoming.tmdb.api_key.trim() == "***" {
        incoming.tmdb.api_key = current.tmdb.api_key.clone();
    }
    if incoming.scraper.tvdb.api_key.trim() == "***" {
        incoming.scraper.tvdb.api_key = current.scraper.tvdb.api_key.clone();
    }
    if incoming.scraper.tvdb.pin.trim() == "***" {
        incoming.scraper.tvdb.pin = current.scraper.tvdb.pin.clone();
    }
    if incoming.scraper.bangumi.access_token.trim() == "***" {
        incoming.scraper.bangumi.access_token = current.scraper.bangumi.access_token.clone();
    }

    if incoming.billing.epay.key.trim() == "***" {
        incoming.billing.epay.key = current.billing.epay.key.clone();
    }

    if incoming.agent.moviepilot.password.trim() == "***" {
        incoming.agent.moviepilot.password = current.agent.moviepilot.password.clone();
    }

    if incoming.storage.lumenbackend_stream_signing_key.trim() == "***" {
        incoming.storage.lumenbackend_stream_signing_key =
            current.storage.lumenbackend_stream_signing_key.clone();
    }

    incoming
}

#[cfg(test)]
mod routes_admin_sessions_system_tests {
    use super::web_settings_restart_required;
    use ls_config::WebAppConfig;

    #[test]
    fn web_settings_restart_required_for_server_binding_changes() {
        let current = WebAppConfig::default();
        let mut updated = current.clone();
        updated.server.port = 18096;
        assert!(web_settings_restart_required(&current, &updated));
    }

    #[test]
    fn web_settings_restart_not_required_for_tmdb_credentials_change() {
        let current = WebAppConfig::default();
        let mut updated = current.clone();
        updated.tmdb.enabled = true;
        updated.tmdb.api_key = "updated-token".to_string();
        assert!(!web_settings_restart_required(&current, &updated));
    }
}

#[derive(Debug, Serialize)]
struct AdminScraperSettingsResponse {
    settings: WebAppConfig,
    libraries: Vec<ls_domain::model::Library>,
}

#[derive(Debug, Deserialize)]
struct AdminScraperLibraryPolicyInput {
    library_id: Uuid,
    #[serde(default)]
    scraper_policy: Value,
}

#[derive(Debug, Deserialize)]
struct AdminUpsertScraperSettingsRequest {
    settings: WebAppConfig,
    #[serde(default)]
    library_policies: Vec<AdminScraperLibraryPolicyInput>,
}

async fn admin_get_scraper_settings(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_super_admin(&state, &headers, &uri).await {
        return resp;
    }

    let settings = match state.infra.get_web_settings().await {
        Ok(v) => mask_web_settings(v),
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to load scraper settings: {err}"),
            );
        }
    };
    let libraries = match state.infra.list_libraries().await {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to load library scraper policies: {err}"),
            );
        }
    };

    Json(AdminScraperSettingsResponse { settings, libraries }).into_response()
}

async fn admin_upsert_scraper_settings(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<AdminUpsertScraperSettingsRequest>,
) -> Response {
    let user = match require_super_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let current = match state.infra.get_web_settings().await {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to load current scraper settings: {err}"),
            );
        }
    };

    let merged = merge_secret_placeholders(payload.settings, &current);
    if merged.auth.bootstrap_admin_user.trim().is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "auth.bootstrap_admin_user is required",
        );
    }

    if let Err(err) = state.infra.upsert_web_settings(&merged).await {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to persist scraper settings: {err}"),
        );
    }

    for policy in &payload.library_policies {
        if let Err(err) = state
            .infra
            .update_library_scraper_policy(policy.library_id, &policy.scraper_policy)
            .await
        {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to persist library scraper policy: {err}"),
            );
        }
    }

    let libraries = match state.infra.list_libraries().await {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to reload library scraper policies: {err}"),
            );
        }
    };

    let restart_required = web_settings_restart_required(&current, &merged);
    let _ = state
        .infra
        .log_audit_event(
            parse_user_uuid(&user),
            "admin.scraper.settings.upsert",
            "web_settings",
            Some("global"),
            json!({
                "restart_required": restart_required,
                "library_policies_updated": payload.library_policies.len(),
            }),
        )
        .await;

    Json(AdminScraperSettingsResponse {
        settings: mask_web_settings(merged),
        libraries,
    })
    .into_response()
}

async fn admin_list_scraper_providers(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }
    match state.infra.list_scraper_provider_statuses().await {
        Ok(providers) => Json(providers).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to list scraper providers: {err}"),
        ),
    }
}

async fn admin_test_scraper_provider(
    State(state): State<ApiContext>,
    AxPath(provider_id): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }
    match state.infra.test_scraper_provider(&provider_id).await {
        Ok(Some(provider)) => Json(provider).into_response(),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "scraper provider not found"),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to test scraper provider: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct AdminTmdbFailuresQuery {
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct AdminClearTmdbCacheRequest {
    expired_only: Option<bool>,
}

async fn admin_get_scraper_cache_stats(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }
    match state.infra.scraper_cache_stats().await {
        Ok(stats) => Json(stats).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to get scraper cache stats: {err}"),
        ),
    }
}

async fn admin_list_scraper_failures(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<AdminTmdbFailuresQuery>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }
    let limit = query.limit.unwrap_or(100).clamp(1, 500);
    match state.infra.list_scraper_failures(limit).await {
        Ok(failures) => Json(failures).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to list scraper failures: {err}"),
        ),
    }
}

async fn admin_clear_scraper_cache(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<AdminClearTmdbCacheRequest>,
) -> Response {
    let user = match require_super_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let expired_only = payload.expired_only.unwrap_or(false);
    match state.infra.clear_scraper_cache(expired_only).await {
        Ok(removed) => {
            let _ = state
                .infra
                .log_audit_event(
                    parse_user_uuid(&user),
                    "admin.scraper_cache.clear",
                    "scraper_cache",
                    None,
                    json!({"removed": removed, "expired_only": expired_only}),
                )
                .await;
            Json(json!({"removed": removed})).into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to clear scraper cache: {err}"),
        ),
    }
}

async fn admin_clear_scraper_failures(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let user = match require_super_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    match state.infra.clear_scraper_failures().await {
        Ok(removed) => {
            let _ = state
                .infra
                .log_audit_event(
                    parse_user_uuid(&user),
                    "admin.scraper_failures.clear",
                    "scraper_failures",
                    None,
                    json!({"removed": removed}),
                )
                .await;
            Json(json!({"removed": removed})).into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to clear scraper failures: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct AdminItemRescrapeRequest {
    refresh_images: Option<bool>,
}

async fn admin_rescrape_item(
    State(state): State<ApiContext>,
    AxPath(item_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<AdminItemRescrapeRequest>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }
    match state
        .infra
        .rescrape_item_metadata(item_id, payload.refresh_images.unwrap_or(false))
        .await
    {
        Ok(updated) => Json(json!({ "updated": updated })).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to rescrape item metadata: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct AdminStorageConfigQuery {
    include_secrets: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct AdminUpsertStorageConfigRequest {
    kind: String,
    name: String,
    config: Value,
    enabled: Option<bool>,
}

async fn admin_get_tmdb_cache_stats(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    admin_get_scraper_cache_stats(State(state), headers, uri).await
}

async fn admin_list_tmdb_failures(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<AdminTmdbFailuresQuery>,
) -> Response {
    admin_list_scraper_failures(State(state), headers, uri, Query(query)).await
}

async fn admin_clear_tmdb_cache(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<AdminClearTmdbCacheRequest>,
) -> Response {
    admin_clear_scraper_cache(State(state), headers, uri, Json(payload)).await
}

async fn admin_clear_tmdb_failures(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    admin_clear_scraper_failures(State(state), headers, uri).await
}
