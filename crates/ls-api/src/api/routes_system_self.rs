async fn health() -> Response {
    Json(json!({"status": "ok", "service": "lumenstream"})).into_response()
}

async fn metrics_snapshot(State(state): State<ApiContext>) -> Response {
    if !state.infra.config_snapshot().observability.metrics_enabled {
        return error_response(StatusCode::NOT_FOUND, "metrics disabled");
    }

    let mut api_snapshot = state.metrics.snapshot();
    if let Some(obj) = api_snapshot.as_object_mut() {
        obj.insert("infra".to_string(), state.infra.runtime_metrics_snapshot());
    }

    Json(api_snapshot).into_response()
}

fn resolve_system_addresses(state: &ApiContext, headers: Option<&HeaderMap>) -> (String, String) {
    let config = state.infra.config_snapshot();
    if !config.server.base_url.is_empty() {
        let address = config.server.base_url.clone();
        return (address.clone(), address);
    }

    if let Some(headers) = headers
        && let Some(address) = resolve_request_address(headers)
    {
        return (address.clone(), address);
    }

    let host = match config.server.host.as_str() {
        "0.0.0.0" | "::" | "[::]" => "127.0.0.1",
        other => other,
    };
    let address = format!("http://{}:{}", host, config.server.port);
    (address.clone(), address)
}

fn normalize_host_header(raw: &str) -> Option<String> {
    let mut host = raw.split(',').next().unwrap_or_default().trim();
    if host.is_empty() {
        return None;
    }

    host = host.trim_matches('"').trim_matches('\'').trim_end_matches('/');
    if let Some(rest) = host.strip_prefix("http://") {
        host = rest;
    } else if let Some(rest) = host.strip_prefix("https://") {
        host = rest;
    }
    if let Some((value, _)) = host.split_once('/') {
        host = value;
    }

    let host = host.trim().trim_end_matches('.');
    if host.is_empty() {
        None
    } else {
        Some(host.to_string())
    }
}

fn request_host(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-forwarded-host")
        .and_then(|value| value.to_str().ok())
        .and_then(normalize_host_header)
        .or_else(|| {
            headers
                .get(header::HOST)
                .and_then(|value| value.to_str().ok())
                .and_then(normalize_host_header)
        })
}

fn parse_forwarded_proto(headers: &HeaderMap) -> Option<String> {
    if let Some(value) = headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
    {
        let proto = value
            .split(',')
            .next()
            .unwrap_or_default()
            .trim()
            .trim_matches('"');
        if proto.eq_ignore_ascii_case("https") {
            return Some("https".to_string());
        }
        if proto.eq_ignore_ascii_case("http") {
            return Some("http".to_string());
        }
    }

    if let Some(value) = headers.get("forwarded").and_then(|value| value.to_str().ok()) {
        for segment in value.split(',') {
            for part in segment.split(';') {
                let Some((key, raw_value)) = part.split_once('=') else {
                    continue;
                };
                if !key.trim().eq_ignore_ascii_case("proto") {
                    continue;
                }
                let proto = raw_value.trim().trim_matches('"');
                if proto.eq_ignore_ascii_case("https") {
                    return Some("https".to_string());
                }
                if proto.eq_ignore_ascii_case("http") {
                    return Some("http".to_string());
                }
            }
        }
    }

    if let Some(value) = headers.get("cf-visitor").and_then(|value| value.to_str().ok()) {
        let lower = value.to_ascii_lowercase();
        if lower.contains("\"scheme\":\"https\"") {
            return Some("https".to_string());
        }
        if lower.contains("\"scheme\":\"http\"") {
            return Some("http".to_string());
        }
    }

    if let Some(value) = headers
        .get("x-forwarded-ssl")
        .and_then(|value| value.to_str().ok())
        && value.trim().eq_ignore_ascii_case("on")
    {
        return Some("https".to_string());
    }

    if let Some(value) = headers
        .get("x-forwarded-port")
        .and_then(|value| value.to_str().ok())
    {
        let port = value.split(',').next().unwrap_or_default().trim();
        if port == "443" {
            return Some("https".to_string());
        }
        if port == "80" {
            return Some("http".to_string());
        }
    }

    None
}

fn host_is_loopback(host: &str) -> bool {
    let host = host.trim();
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }

    let host_without_port = if let Some(trimmed) = host.strip_prefix('[') {
        trimmed
            .split_once(']')
            .map(|(value, _)| value)
            .unwrap_or(trimmed)
    } else if let Some((left, right)) = host.rsplit_once(':') {
        if right.chars().all(|ch| ch.is_ascii_digit()) && !left.contains(':') {
            left
        } else {
            host
        }
    } else {
        host
    };

    host_without_port.eq_ignore_ascii_case("localhost")
        || host_without_port.eq_ignore_ascii_case("127.0.0.1")
        || host_without_port.eq_ignore_ascii_case("::1")
        || host_without_port.starts_with("127.")
}

fn infer_request_scheme(headers: &HeaderMap, host: &str) -> String {
    if let Some(proto) = parse_forwarded_proto(headers) {
        return proto;
    }

    if host.ends_with(":443") {
        return "https".to_string();
    }
    if host.ends_with(":80") {
        return "http".to_string();
    }
    if host_is_loopback(host) {
        return "http".to_string();
    }
    "https".to_string()
}

fn resolve_request_address(headers: &HeaderMap) -> Option<String> {
    let host = request_host(headers)?;
    let scheme = infer_request_scheme(headers, &host);
    Some(format!("{scheme}://{host}"))
}

fn normalize_mac_for_wol(raw: &str) -> Option<String> {
    let compact = raw
        .trim()
        .chars()
        .filter(|ch| ch.is_ascii_hexdigit())
        .collect::<String>()
        .to_ascii_uppercase();
    if compact.len() == 12 {
        Some(compact)
    } else {
        None
    }
}

fn collect_wake_on_lan_info_blocking() -> Vec<WakeOnLanInfoDto> {
    let interfaces = match std::fs::read_dir("/sys/class/net") {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let mut candidates = interfaces
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "lo" {
                return None;
            }
            let mac_path = entry.path().join("address");
            let raw_mac = std::fs::read_to_string(mac_path).ok()?;
            let mac = normalize_mac_for_wol(&raw_mac)?;
            Some((name, mac))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|(left, _), (right, _)| left.cmp(right));

    candidates
        .into_iter()
        .take(1)
        .map(|(_, mac_address)| WakeOnLanInfoDto {
            broadcast_address: "255.255.255.255".to_string(),
            mac_address,
            port: 9,
        })
        .collect()
}

async fn collect_wake_on_lan_info() -> Vec<WakeOnLanInfoDto> {
    tokio::task::spawn_blocking(collect_wake_on_lan_info_blocking)
        .await
        .unwrap_or_default()
}

fn supports_https(headers: &HeaderMap, wan_address: &str) -> bool {
    let forwarded_proto_https = headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .map(|value| {
            value
                .split(',')
                .any(|part| part.trim().eq_ignore_ascii_case("https"))
        })
        .unwrap_or(false);
    if forwarded_proto_https {
        return true;
    }

    let forwarded_header_https = headers
        .get("forwarded")
        .and_then(|value| value.to_str().ok())
        .map(|value| {
            value.split(';').any(|part| {
                part.split('=').any(|token| token.trim().eq_ignore_ascii_case("https"))
            })
        })
        .unwrap_or(false);
    if forwarded_header_https {
        return true;
    }

    let forwarded_ssl_https = headers
        .get("x-forwarded-ssl")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.trim().eq_ignore_ascii_case("on"))
        .unwrap_or(false);
    if forwarded_ssl_https {
        return true;
    }

    let forwarded_port_https = headers
        .get("x-forwarded-port")
        .and_then(|value| value.to_str().ok())
        .map(|value| {
            value
                .split(',')
                .any(|part| part.trim().eq_ignore_ascii_case("443"))
        })
        .unwrap_or(false);
    if forwarded_port_https {
        return true;
    }

    let host_header_https = headers
        .get("host")
        .and_then(|value| value.to_str().ok())
        .map(|raw_host| {
            let host = raw_host.trim().trim_end_matches('.');
            if host.is_empty() {
                return false;
            }

            if host.eq_ignore_ascii_case("localhost")
                || host.eq_ignore_ascii_case("127.0.0.1")
                || host.eq_ignore_ascii_case("[::1]")
                || host.eq_ignore_ascii_case("::1")
            {
                return false;
            }

            if let Some(port) = host.rsplit_once(':').map(|(_, port)| port.trim()) {
                return port == "443";
            }

            true
        })
        .unwrap_or(false);
    host_header_https || wan_address.starts_with("https://")
}

/// GET /System/Info/Public - Public server info for client connection detection (no auth)
async fn get_system_info_public(State(state): State<ApiContext>, headers: HeaderMap) -> Response {
    let (local_address, wan_address) = resolve_system_addresses(&state, Some(&headers));
    let info = PublicSystemInfoDto {
        local_address: Some(local_address.clone()),
        wan_address: Some(wan_address.clone()),
        server_name: "lumenstream".to_string(),
        version: "4.9.1.26".to_string(),
        id: state.infra.server_id.clone(),
        local_addresses: vec![local_address],
        remote_addresses: vec![wan_address],
    };
    Json(info).into_response()
}

/// GET /System/Info - Full server info (authenticated)
async fn get_system_info(State(state): State<ApiContext>, headers: HeaderMap, uri: Uri) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }

    let (local_address, wan_address) = resolve_system_addresses(&state, Some(&headers));
    let wake_on_lan_info = collect_wake_on_lan_info().await;
    let supports_https = supports_https(&headers, &wan_address);
    let (can_self_restart, has_update_available, hardware_acceleration_requires_premiere) =
        compat_system_info_capability_flags();

    let info = SystemInfoDto {
        system_update_level: Some("Release".to_string()),
        operating_system_display_name: Some(std::env::consts::OS.to_string()),
        package_name: Some("lumenstream".to_string()),
        supports_library_monitor: state.infra.config_snapshot().scheduler.enabled,
        web_socket_port_number: state.infra.config_snapshot().server.port as i32,
        completed_installations: Some(Vec::new()),
        has_image_enhancers: false,
        supports_local_port_configuration: true,
        supports_wake_server: false,
        wake_on_lan_info,
        is_in_maintenance_mode: false,
        server_name: "lumenstream".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        id: state.infra.server_id.clone(),
        local_address: local_address.clone(),
        local_addresses: vec![local_address.clone()],
        startup_wizard_completed: true,
        product_name: "lumenstream".to_string(),
        operating_system: std::env::consts::OS.to_string(),
        program_data_path: "./data".to_string(),
        items_by_name_path: "./data/items-by-name".to_string(),
        cache_path: "./cache".to_string(),
        log_path: "./logs".to_string(),
        internal_metadata_path: "./data/metadata".to_string(),
        transcoding_temp_path: "./cache/transcodes".to_string(),
        http_server_port_number: state.infra.config_snapshot().server.port as i32,
        supports_https,
        https_port_number: 0,
        has_pending_restart: false,
        is_shutting_down: false,
        can_self_restart,
        can_self_update: false,
        can_launch_web_browser: false,
        has_update_available,
        supports_auto_run_at_startup: false,
        hardware_acceleration_requires_premiere,
        wan_address: Some(wan_address.clone()),
        remote_addresses: vec![wan_address],
    };
    Json(info).into_response()
}

fn compat_system_info_capability_flags() -> (bool, bool, bool) {
    // Keep these capability flags aligned with Emby expectations for better
    // third-party client compatibility.
    (true, true, true)
}

#[derive(Debug, Serialize)]
struct EndpointInfoDto {
    #[serde(rename = "IsLocal")]
    is_local: bool,
    #[serde(rename = "IsInNetwork")]
    is_in_network: bool,
}

/// GET /System/Endpoint - Emby endpoint network hint.
async fn get_system_endpoint() -> Response {
    Json(EndpointInfoDto {
        is_local: false,
        is_in_network: false,
    })
    .into_response()
}

#[derive(Debug, Serialize)]
struct BrandingOptionsDto {
    #[serde(rename = "LoginDisclaimer")]
    login_disclaimer: String,
    #[serde(rename = "CustomCss")]
    custom_css: String,
}

/// GET /Branding/Configuration - Branding config used by Emby clients.
async fn get_branding_configuration() -> Response {
    Json(BrandingOptionsDto {
        login_disclaimer: String::new(),
        custom_css: String::new(),
    })
    .into_response()
}

/// GET /Branding/Css(.css) - Branding stylesheet.
async fn get_branding_css() -> Response {
    HttpResponse::Ok().content_type("text/css").body("")
}

#[derive(Debug, Deserialize)]
struct DisplayPreferencesQuery {
    #[serde(rename = "UserId", alias = "userId")]
    _user_id: Option<String>,
    #[serde(rename = "Client", alias = "client")]
    client: Option<String>,
}

#[derive(Debug, Serialize)]
struct DisplayPreferencesDto {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "ViewType")]
    view_type: String,
    #[serde(rename = "SortBy")]
    sort_by: String,
    #[serde(rename = "IndexBy")]
    index_by: String,
    #[serde(rename = "RememberIndexing")]
    remember_indexing: bool,
    #[serde(rename = "PrimaryImageHeight")]
    primary_image_height: i32,
    #[serde(rename = "PrimaryImageWidth")]
    primary_image_width: i32,
    #[serde(rename = "CustomPrefs")]
    custom_prefs: HashMap<String, Value>,
    #[serde(rename = "ScrollDirection")]
    scroll_direction: String,
    #[serde(rename = "ShowBackdrop")]
    show_backdrop: bool,
    #[serde(rename = "RememberSorting")]
    remember_sorting: bool,
    #[serde(rename = "SortOrder")]
    sort_order: String,
    #[serde(rename = "ShowSidebar")]
    show_sidebar: bool,
    #[serde(rename = "Client")]
    client: String,
}

/// GET /DisplayPreferences/{id} - Compatibility display preference payload.
async fn get_display_preferences(
    State(state): State<ApiContext>,
    AxPath(id): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<DisplayPreferencesQuery>,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }

    Json(DisplayPreferencesDto {
        id,
        view_type: "Poster".to_string(),
        sort_by: "SortName".to_string(),
        index_by: "SortName".to_string(),
        remember_indexing: true,
        primary_image_height: 300,
        primary_image_width: 200,
        custom_prefs: HashMap::new(),
        scroll_direction: "Horizontal".to_string(),
        show_backdrop: true,
        remember_sorting: true,
        sort_order: "Ascending".to_string(),
        show_sidebar: true,
        client: query.client.unwrap_or_else(|| "lumenstream".to_string()),
    })
    .into_response()
}

/// POST /DisplayPreferences/{id} - Accept and ignore display preference updates.
async fn post_display_preferences(
    State(state): State<ApiContext>,
    AxPath(_id): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
    Json(_payload): Json<Value>,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }
    StatusCode::NO_CONTENT.into_response()
}

#[derive(Debug, Deserialize)]
struct SystemActivityLogQuery {
    #[serde(rename = "StartIndex", alias = "startIndex")]
    start_index: Option<i64>,
    #[serde(rename = "Limit", alias = "limit")]
    limit: Option<i64>,
    #[serde(rename = "MinDate", alias = "minDate")]
    _min_date: Option<String>,
}

/// GET /System/ActivityLog/Entries - Admin activity log (compat stub).
async fn get_system_activity_log_entries(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<SystemActivityLogQuery>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    let start_index = query.start_index.unwrap_or(0).max(0) as i32;
    let _limit = query.limit.unwrap_or(100).clamp(1, 500);
    Json(QueryResultDto::<Value> {
        items: Vec::new(),
        total_record_count: 0,
        start_index,
    })
    .into_response()
}

/// GET /System/Configuration - Admin system configuration (compat stub).
async fn get_system_configuration(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    Json(json!({
        "ServerName": "lumenstream",
        "EnableUPnP": false,
        "EnableRemoteAccess": true,
        "PublicPort": state.infra.config_snapshot().server.port,
        "HttpServerPortNumber": state.infra.config_snapshot().server.port,
    }))
    .into_response()
}

/// POST /System/Configuration - Accept and ignore system configuration updates.
async fn post_system_configuration(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(_payload): Json<Value>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }
    StatusCode::NO_CONTENT.into_response()
}

/// GET /System/Configuration/{key} - Return key-scoped config payload.
async fn get_system_configuration_key(
    State(state): State<ApiContext>,
    AxPath(_key): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }
    HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body("{}")
}

/// POST /System/Configuration/{key} - Accept and ignore key-scoped config payload.
async fn post_system_configuration_key(
    State(state): State<ApiContext>,
    AxPath(_key): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
    _body: web::Bytes,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }
    StatusCode::NO_CONTENT.into_response()
}

#[derive(Debug, Deserialize)]
struct SystemLogQuery {
    #[serde(rename = "Name", alias = "name")]
    name: Option<String>,
}

/// GET /System/Logs - List log files (compat stub).
async fn get_system_logs(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }
    Json(Vec::<Value>::new()).into_response()
}

/// GET /System/Logs/Log - Download log content by name (compat stub).
async fn get_system_log_content(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<SystemLogQuery>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }
    let _name = query.name.unwrap_or_else(|| "lumenstream.log".to_string());
    HttpResponse::Ok().content_type("text/plain").body("")
}

/// GET /System/WakeOnLanInfo - Wake-on-lan compatibility payload.
async fn get_system_wake_on_lan_info(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }
    Json(collect_wake_on_lan_info().await).into_response()
}

/// POST /System/Restart - Accept restart command (no-op).
async fn post_system_restart(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }
    StatusCode::NO_CONTENT.into_response()
}

/// POST /System/Shutdown - Accept shutdown command (no-op).
async fn post_system_shutdown(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }
    StatusCode::NO_CONTENT.into_response()
}

fn system_ping_response() -> Response {
    HttpResponse::Ok()
        .content_type("text/plain")
        .body("Jellyfin Server")
}

/// GET /System/Ping - Simple heartbeat (no auth)
async fn system_ping() -> Response {
    system_ping_response()
}

/// POST /System/Ping - Jellyfin compatibility heartbeat (no auth)
async fn system_ping_post() -> Response {
    system_ping_response()
}

#[derive(Deserialize)]
struct SetLogLevelRequest {
    level: String,
}

/// GET /System/Logs/Level - Get current log level (admin only)
async fn get_log_level(State(state): State<ApiContext>, headers: HeaderMap, uri: Uri) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    match &state.log_handle {
        Some(handle) => {
            let level = handle.get_level();
            Json(json!({ "level": level })).into_response()
        }
        None => error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "logging management not available",
        ),
    }
}

/// PUT /System/Logs/Level - Set log level at runtime (admin only)
async fn set_log_level(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<SetLogLevelRequest>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    match &state.log_handle {
        Some(handle) => {
            let previous = handle.get_level();
            match handle.set_level(&payload.level) {
                Ok(()) => {
                    info!(previous = %previous, new = %payload.level, "log level changed");
                    Json(json!({
                        "level": payload.level,
                        "previous": previous
                    }))
                    .into_response()
                }
                Err(err) => error_response(
                    StatusCode::BAD_REQUEST,
                    &format!("invalid log level: {err}"),
                ),
            }
        }
        None => error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "logging management not available",
        ),
    }
}

/// GET /System/Logs/Config - Get full logging configuration (admin only)
async fn get_log_config(State(state): State<ApiContext>, headers: HeaderMap, uri: Uri) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    let config = &state.infra.config_snapshot().log;
    let current_level = state
        .log_handle
        .as_ref()
        .map(|h| h.get_level())
        .unwrap_or_else(|| config.level.clone());

    Json(json!({
        "level": current_level,
        "configured_level": config.level,
        "format": config.format,
        "output": config.output,
        "file_path": config.file_path,
        "max_size_mb": config.max_size_mb,
        "max_files": config.max_files
    }))
    .into_response()
}

/// GET /Users/Me - Get current authenticated user
async fn get_current_user(State(state): State<ApiContext>, headers: HeaderMap, uri: Uri) -> Response {
    let user = match require_auth(&state, &headers, &uri).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    Json(user).into_response()
}

#[derive(Debug, Serialize)]
struct MePlaybackDomainsResponse {
    selected_domain_id: Option<Uuid>,
    default_domain_id: Option<Uuid>,
    available: Vec<Value>,
}

#[derive(Debug, Deserialize)]
struct MePlaybackDomainSelectRequest {
    domain_id: Uuid,
}

async fn get_me_playback_domains(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let user_id = match parse_user_uuid(&user) {
        Some(v) => v,
        None => return error_response(StatusCode::UNAUTHORIZED, "invalid user id"),
    };

    let domains = match state.infra.list_playback_domains_for_user(user_id).await {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to list playback domains: {err}"),
            );
        }
    };
    let selected = match state
        .infra
        .get_user_playback_domain_preference(user_id)
        .await
    {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to load playback domain preference: {err}"),
            );
        }
    };
    let selected_domain_id = selected
        .as_ref()
        .and_then(|item| domains.iter().find(|domain| domain.id == item.id))
        .map(|item| item.id);

    let default_domain_id = domains
        .iter()
        .find(|item| item.is_default)
        .map(|item| item.id);
    let available = domains
        .into_iter()
        .filter(|item| item.enabled)
        .map(|item| {
            json!({
                "id": item.id,
                "name": item.name,
                "base_url": item.base_url,
                "enabled": item.enabled,
                "priority": item.priority,
                "is_default": item.is_default,
                "lumenbackend_node_id": item.lumenbackend_node_id,
                "traffic_multiplier": item.traffic_multiplier,
            })
        })
        .collect::<Vec<_>>();

    Json(MePlaybackDomainsResponse {
        selected_domain_id,
        default_domain_id,
        available,
    })
    .into_response()
}

async fn select_me_playback_domain(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<MePlaybackDomainSelectRequest>,
) -> Response {
    let user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let user_id = match parse_user_uuid(&user) {
        Some(v) => v,
        None => return error_response(StatusCode::UNAUTHORIZED, "invalid user id"),
    };

    match state
        .infra
        .set_user_playback_domain_preference(user_id, payload.domain_id)
        .await
    {
        Ok(domain) => {
            let _ = state
                .infra
                .log_audit_event(
                    Some(user_id),
                    "user.playback_domain.select",
                    "playback_domain",
                    Some(&domain.id.to_string()),
                    json!({"domain_name": domain.name}),
                )
                .await;
            Json(json!({
                "selected_domain_id": domain.id,
                "selected_domain_name": domain.name,
            }))
            .into_response()
        }
        Err(err) => {
            let msg = err.to_string();
            if msg.contains("not found") {
                return error_response(StatusCode::NOT_FOUND, "playback domain not found");
            }
            if msg.contains("disabled") {
                return error_response(StatusCode::BAD_REQUEST, "playback domain is disabled");
            }
            if msg.contains("not allowed") {
                return error_response(StatusCode::FORBIDDEN, "playback domain is not allowed");
            }
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to select playback domain: {err}"),
            )
        }
    }
}

fn deserialize_optional_bool_query_compat<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    Ok(match value {
        None | Some(Value::Null) => None,
        Some(Value::Bool(v)) => Some(v),
        Some(Value::Number(v)) => match v.as_i64() {
            Some(0) => Some(false),
            Some(1) => Some(true),
            _ => None,
        },
        Some(Value::String(raw)) => {
            let normalized = raw.trim().to_ascii_lowercase();
            if normalized.is_empty() {
                None
            } else {
                match normalized.as_str() {
                    "1" | "true" | "yes" | "on" => Some(true),
                    "0" | "false" | "no" | "off" => Some(false),
                    _ => None,
                }
            }
        }
        _ => None,
    })
}

#[derive(Debug, Deserialize)]
struct UsersQuery {
    #[serde(
        rename = "IsHidden",
        alias = "isHidden",
        default,
        deserialize_with = "deserialize_optional_bool_query_compat"
    )]
    is_hidden: Option<bool>,
    #[serde(
        rename = "IsDisabled",
        alias = "isDisabled",
        default,
        deserialize_with = "deserialize_optional_bool_query_compat"
    )]
    is_disabled: Option<bool>,
}

/// GET /Users - Admin list all users
async fn get_users(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<UsersQuery>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    match state.infra.list_users_for_admin().await {
        Ok(users) => {
            let filtered = users
                .into_iter()
                .filter(|user| {
                    // lumenstream currently has no hidden-user model; treat all users as visible.
                    if query.is_hidden == Some(true) {
                        return false;
                    }
                    if let Some(is_disabled) = query.is_disabled {
                        return user.policy.is_disabled == is_disabled;
                    }
                    true
                })
                .collect::<Vec<_>>();
            Json(filtered).into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to list users: {err}"),
        ),
    }
}

async fn authenticate_by_name(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    request: HttpRequest,
    body: web::Bytes,
) -> Response {
    let Some(payload) = parse_authenticate_by_name_payload(body.as_ref(), request.query_string()) else {
        return error_response(StatusCode::BAD_REQUEST, "invalid auth payload");
    };

    let client_context =
        resolve_emby_client_context_with_query(&headers, Some(request.query_string()));

    let remote_addr = extract_client_ip(&headers, None, &state.infra.config_snapshot().security);

    let pw = payload.pw.as_deref().or(payload.password.as_deref()).unwrap_or("");
    match state
        .infra
        .authenticate_user(
            &payload.username,
            pw,
            client_context.client.as_str(),
            client_context.device_name.as_str(),
            client_context.device_id.as_str(),
            remote_addr.as_deref(),
            client_context.application_version.as_deref(),
        )
        .await
    {
        Ok(AuthenticateUserResult::Success(result)) => Json(result).into_response(),
        Ok(AuthenticateUserResult::InvalidCredentials) => {
            state.metrics.auth_failures.fetch_add(1, Ordering::Relaxed);
            error_response(StatusCode::UNAUTHORIZED, "invalid username or password")
        }
        Ok(AuthenticateUserResult::PasswordResetRequired) => {
            state.metrics.auth_failures.fetch_add(1, Ordering::Relaxed);
            error_response(
                StatusCode::UNAUTHORIZED,
                "legacy password hash rejected; ask an administrator to reinitialize password via /Users/{userId}/Password",
            )
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("auth failed: {err}"),
        ),
    }
}

fn parse_authenticate_by_name_payload(
    body: &[u8],
    query_string: &str,
) -> Option<AuthenticateByNameRequest> {
    parse_authenticate_by_name_json(body)
        .or_else(|| parse_authenticate_by_name_form_bytes(body))
        .or_else(|| parse_authenticate_by_name_form_encoded(query_string))
}

fn parse_authenticate_by_name_json(body: &[u8]) -> Option<AuthenticateByNameRequest> {
    let raw = std::str::from_utf8(body).ok()?.trim();
    if raw.is_empty() {
        return None;
    }
    serde_json::from_str::<AuthenticateByNameRequest>(raw).ok()
}

fn parse_authenticate_by_name_form_bytes(body: &[u8]) -> Option<AuthenticateByNameRequest> {
    let raw = std::str::from_utf8(body).ok()?.trim();
    if raw.is_empty() {
        return None;
    }
    parse_authenticate_by_name_form_encoded(raw)
}

fn parse_authenticate_by_name_form_encoded(raw: &str) -> Option<AuthenticateByNameRequest> {
    let mut username: Option<String> = None;
    let mut pw: Option<String> = None;
    let mut password: Option<String> = None;

    for pair in raw.split('&') {
        if pair.is_empty() {
            continue;
        }

        let (raw_key, raw_value) = pair.split_once('=').unwrap_or((pair, ""));
        let key = decode_form_component(raw_key);
        let value = decode_form_component(raw_value);

        match key.as_str() {
            "Username" | "username" | "UserName" | "userName" | "Name" | "name" => {
                username = Some(value);
            }
            "Pw" | "pw" => {
                pw = Some(value);
            }
            "Password" | "password" => {
                password = Some(value);
            }
            _ => {}
        }
    }

    let username = username
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)?;

    Some(AuthenticateByNameRequest {
        username,
        pw,
        password,
    })
}

fn decode_form_component(raw: &str) -> String {
    let normalized = raw.replace('+', " ");
    urlencoding::decode(&normalized)
        .map(|value| value.into_owned())
        .unwrap_or(normalized)
}
