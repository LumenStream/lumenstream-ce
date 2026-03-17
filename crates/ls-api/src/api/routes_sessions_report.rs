async fn report_playing_start(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<PlaybackProgressDto>,
) -> Response {
    report_playback_event_inner(state, headers, uri, payload, "start").await
}

async fn report_playing_progress(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<PlaybackProgressDto>,
) -> Response {
    report_playback_event_inner(state, headers, uri, payload, "progress").await
}

async fn report_playing_stopped(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<PlaybackProgressDto>,
) -> Response {
    report_playback_event_inner(state, headers, uri, payload, "stopped").await
}

#[derive(Debug, Deserialize)]
struct SessionsQuery {
    #[serde(rename = "ControllableByUserId", alias = "controllableByUserId")]
    _controllable_by_user_id: Option<Uuid>,
    #[serde(rename = "DeviceId", alias = "deviceId")]
    device_id: Option<String>,
}

fn default_session_play_state() -> Value {
    json!({
        "CanSeek": false,
        "IsPaused": false,
        "IsMuted": false,
        "PlayMethod": "DirectPlay",
        "RepeatMode": "RepeatNone",
        "SleepTimerMode": "None",
        "SubtitleOffset": 0,
        "Shuffle": false,
        "PlaybackRate": 1,
    })
}

fn derive_internal_device_id(session_id: Uuid, device_id: &str) -> i64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(&session_id, &mut hasher);
    std::hash::Hash::hash(&device_id, &mut hasher);
    let value = std::hash::Hasher::finish(&hasher) & (i64::MAX as u64);
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn session_is_recent(last_seen_at: chrono::DateTime<chrono::Utc>) -> bool {
    let cutoff = chrono::Utc::now() - chrono::Duration::minutes(30);
    last_seen_at >= cutoff
}

fn normalized_session_identity(raw: Option<&str>) -> Option<String> {
    let value = raw?.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn session_device_key(session: &ls_domain::model::AuthSession) -> (Uuid, String) {
    let device_id = normalized_session_identity(session.device_id.as_deref())
        .or_else(|| {
            normalized_session_identity(session.device_name.as_deref())
                .map(|name| format!("name:{name}"))
        })
        .or_else(|| {
            normalized_session_identity(session.client.as_deref())
                .map(|client| format!("client:{client}"))
        })
        .unwrap_or_else(|| format!("session:{}", session.id));
    (session.user_id, device_id)
}

fn session_playback_key(user_id: Uuid, device_name: Option<&str>, client_name: Option<&str>) -> (Uuid, String, String) {
    let device = normalized_session_identity(device_name).unwrap_or_default();
    let client = normalized_session_identity(client_name).unwrap_or_default();
    (user_id, device, client)
}

fn session_play_state_with_playback(
    playback: Option<&ls_domain::model::AdminPlaybackSession>,
) -> Value {
    let mut state = default_session_play_state();
    let Some(state_obj) = state.as_object_mut() else {
        return state;
    };

    state_obj.insert("PositionTicks".to_string(), Value::from(0));

    let Some(playback) = playback else {
        return state;
    };

    state_obj.insert("CanSeek".to_string(), Value::Bool(true));
    state_obj.insert("PositionTicks".to_string(), Value::from(playback.position_ticks.max(0)));
    if let Some(play_method) = playback.play_method.as_deref()
        && !play_method.trim().is_empty()
    {
        state_obj.insert(
            "PlayMethod".to_string(),
            Value::String(play_method.to_string()),
        );
    }

    state
}

async fn get_sessions(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<SessionsQuery>,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let auth_user_id = parse_user_uuid(&auth_user);
    let is_admin = is_super_admin(&auth_user);

    let sessions = match state.infra.list_auth_sessions(200, true).await {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to query sessions: {err}"),
            );
        }
    };

    let mut playback_by_key = std::collections::HashMap::new();
    if let Ok(playback_sessions) = state.infra.list_playback_sessions(500, true).await {
        for playback in playback_sessions {
            let key = session_playback_key(
                playback.user_id,
                playback.device_name.as_deref(),
                playback.client_name.as_deref(),
            );
            playback_by_key.entry(key).or_insert(playback);
        }
    }

    let mut seen_devices = std::collections::HashSet::new();
    let filtered = sessions
        .into_iter()
        .filter(|session| {
            if !is_admin {
                if let Some(uid) = auth_user_id {
                    if session.user_id != uid {
                        return false;
                    }
                } else {
                    return false;
                }
            }

            if let Some(device_id) = query.device_id.as_deref() {
                return session
                    .device_id
                    .as_deref()
                    .map(|v| v == device_id)
                    .unwrap_or(false);
            }
            session_is_recent(session.last_seen_at)
        })
        .filter(|session| seen_devices.insert(session_device_key(session)))
        .map(|session| {
            let device_id = session.device_id.unwrap_or_default();
            let playback_state = playback_by_key.get(&session_playback_key(
                session.user_id,
                session.device_name.as_deref(),
                session.client.as_deref(),
            ));
            SessionInfoDto {
                play_state: Some(session_play_state_with_playback(playback_state)),
                additional_users: Some(Vec::new()),
                remote_end_point: session.remote_addr,
                protocol: Some("HTTP/1.1".to_string()),
                playable_media_types: Some(Vec::new()),
                playlist_index: Some(0),
                playlist_length: Some(0),
                id: session.id.to_string(),
                user_id: session.user_id.to_string(),
                user_name: session.user_name,
                client: session.client.unwrap_or_else(|| "ls-client".to_string()),
                device_name: session.device_name.unwrap_or_else(|| "ls-device".to_string()),
                device_id: device_id.clone(),
                server_id: Some(state.infra.server_id.clone()),
                last_activity_date: Some(format_emby_datetime(session.last_seen_at)),
                application_version: Some(env!("CARGO_PKG_VERSION").to_string()),
                device_type: Some("Unknown".to_string()),
                supported_commands: Some(Vec::new()),
                internal_device_id: Some(derive_internal_device_id(session.id, &device_id)),
                supports_remote_control: Some(false),
            }
        })
        .collect::<Vec<_>>();

    Json(filtered).into_response()
}

#[derive(Debug, Deserialize)]
struct SessionCapabilitiesQuery {
    #[serde(rename = "Id", alias = "id")]
    _id: Option<String>,
    #[serde(rename = "PlayableMediaTypes", alias = "playableMediaTypes")]
    _playable_media_types: Option<String>,
    #[serde(rename = "SupportedCommands", alias = "supportedCommands")]
    _supported_commands: Option<String>,
    #[serde(rename = "SupportsMediaControl", alias = "supportsMediaControl")]
    _supports_media_control: Option<bool>,
    #[serde(rename = "SupportsSync", alias = "supportsSync")]
    _supports_sync: Option<bool>,
    #[serde(
        rename = "SupportsPersistentIdentifier",
        alias = "supportsPersistentIdentifier"
    )]
    _supports_persistent_identifier: Option<bool>,
}

async fn post_sessions_capabilities(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(_query): Query<SessionCapabilitiesQuery>,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }
    StatusCode::NO_CONTENT.into_response()
}

#[derive(Debug, Deserialize)]
struct SessionCapabilitiesFullQuery {
    #[serde(rename = "Id", alias = "id")]
    _id: Option<String>,
}

async fn post_sessions_capabilities_full(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(_query): Query<SessionCapabilitiesFullQuery>,
    Json(_payload): Json<Value>,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }
    StatusCode::NO_CONTENT.into_response()
}

async fn post_session_command(
    State(state): State<ApiContext>,
    AxPath(_session_id): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
    Json(_payload): Json<Value>,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }
    StatusCode::NO_CONTENT.into_response()
}

async fn post_session_command_named(
    State(state): State<ApiContext>,
    AxPath((_session_id, _command)): AxPath<(String, String)>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }
    StatusCode::NO_CONTENT.into_response()
}

#[derive(Debug, Deserialize)]
struct SessionMessageQuery {
    #[serde(rename = "Text", alias = "text")]
    _text: Option<String>,
    #[serde(rename = "Header", alias = "header")]
    _header: Option<String>,
    #[serde(rename = "TimeoutMs", alias = "timeoutMs")]
    _timeout_ms: Option<i64>,
}

async fn post_session_message(
    State(state): State<ApiContext>,
    AxPath(_session_id): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
    Query(_query): Query<SessionMessageQuery>,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }
    StatusCode::NO_CONTENT.into_response()
}

#[derive(Debug, Deserialize)]
struct SessionPlayQuery {
    #[serde(rename = "ItemIds", alias = "itemIds")]
    _item_ids: Option<String>,
    #[serde(rename = "StartPositionTicks", alias = "startPositionTicks")]
    _start_position_ticks: Option<i64>,
    #[serde(rename = "PlayCommand", alias = "playCommand")]
    _play_command: Option<String>,
}

async fn post_session_playing(
    State(state): State<ApiContext>,
    AxPath(_session_id): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
    Query(_query): Query<SessionPlayQuery>,
    Json(_payload): Json<Value>,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }
    StatusCode::NO_CONTENT.into_response()
}

async fn post_session_playing_command(
    State(state): State<ApiContext>,
    AxPath((_session_id, _command)): AxPath<(String, String)>,
    headers: HeaderMap,
    uri: Uri,
    Json(_payload): Json<Value>,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }
    StatusCode::NO_CONTENT.into_response()
}

async fn post_session_system_command(
    State(state): State<ApiContext>,
    AxPath((_session_id, _command)): AxPath<(String, String)>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }
    StatusCode::NO_CONTENT.into_response()
}

async fn post_session_user(
    State(state): State<ApiContext>,
    AxPath((_session_id, _user_id)): AxPath<(String, Uuid)>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }
    StatusCode::NO_CONTENT.into_response()
}

async fn delete_session_user(
    State(state): State<ApiContext>,
    AxPath((_session_id, _user_id)): AxPath<(String, Uuid)>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }
    StatusCode::NO_CONTENT.into_response()
}

#[derive(Debug, Deserialize)]
struct SessionViewingQuery {
    #[serde(rename = "ItemType", alias = "itemType")]
    _item_type: Option<String>,
    #[serde(rename = "ItemId", alias = "itemId")]
    _item_id: Option<String>,
    #[serde(rename = "ItemName", alias = "itemName")]
    _item_name: Option<String>,
}

async fn post_session_viewing(
    State(state): State<ApiContext>,
    AxPath(_session_id): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
    Query(_query): Query<SessionViewingQuery>,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }
    StatusCode::NO_CONTENT.into_response()
}

fn playing_ping_success_response() -> Response {
    StatusCode::NO_CONTENT.into_response()
}

fn apply_playback_payload_client_context(
    payload: &mut PlaybackProgressDto,
    headers: &HeaderMap,
) {
    let context = resolve_emby_client_context(headers);

    let client_missing = payload
        .client
        .as_deref()
        .map(str::trim)
        .map(|value| value.is_empty())
        .unwrap_or(true);
    if client_missing {
        payload.client = Some(context.client);
    }

    let device_name_missing = payload
        .device_name
        .as_deref()
        .map(str::trim)
        .map(|value| value.is_empty())
        .unwrap_or(true);
    if device_name_missing {
        payload.device_name = Some(context.device_name);
    }
}

async fn report_playing_ping(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }

    playing_ping_success_response()
}

async fn report_playback_event_inner(
    state: ApiContext,
    headers: HeaderMap,
    uri: Uri,
    payload: PlaybackProgressDto,
    event_kind: &str,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(user) => user,
        Err(resp) => return resp,
    };

    let user_id = match Uuid::parse_str(&auth_user.id) {
        Ok(v) => v,
        Err(_) => return error_response(StatusCode::UNAUTHORIZED, "invalid user id"),
    };

    let mut payload = payload;
    apply_playback_payload_client_context(&mut payload, &headers);

    match state
        .infra
        .report_playback_event(event_kind, user_id, &payload)
        .await
    {
        Ok(_) => playing_ping_success_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to record playback event: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct AdminCreateLibraryRequest {
    name: String,
    root_path: Option<String>,
    #[serde(default)]
    paths: Option<Vec<String>>,
    library_type: Option<String>,
}
