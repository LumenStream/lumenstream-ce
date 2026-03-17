async fn list_my_playlists(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let Some(user_id) = parse_user_uuid(&user) else {
        return error_response(StatusCode::UNAUTHORIZED, "invalid user");
    };

    match state.infra.list_user_playlists(user_id).await {
        Ok(playlists) => Json(playlists).into_response(),
        Err(err) => map_playlist_error(&err).unwrap_or_else(|| {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to list playlists: {err}"),
            )
        }),
    }
}

async fn list_user_public_playlists(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    AxPath(user_id): AxPath<Uuid>,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }

    match state.infra.list_public_playlists(user_id).await {
        Ok(playlists) => Json(playlists).into_response(),
        Err(err) => map_playlist_error(&err).unwrap_or_else(|| {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to list public playlists: {err}"),
            )
        }),
    }
}

async fn create_playlist(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<CreatePlaylistRequest>,
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
        .create_playlist(
            user_id,
            payload.name.as_str(),
            payload.description.as_deref(),
            payload.is_public,
        )
        .await
    {
        Ok(playlist) => (StatusCode::CREATED, Json(playlist)).into_response(),
        Err(err) => map_playlist_error(&err).unwrap_or_else(|| {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to create playlist: {err}"),
            )
        }),
    }
}

async fn get_playlist(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    AxPath(playlist_id): AxPath<Uuid>,
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
        .get_playlist_visible_to(user_id, playlist_id)
        .await
    {
        Ok(playlist) => Json(playlist).into_response(),
        Err(err) => map_playlist_error(&err).unwrap_or_else(|| {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to get playlist: {err}"),
            )
        }),
    }
}

async fn update_playlist(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    AxPath(playlist_id): AxPath<Uuid>,
    Json(payload): Json<UpdatePlaylistRequest>,
) -> Response {
    let user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let Some(user_id) = parse_user_uuid(&user) else {
        return error_response(StatusCode::UNAUTHORIZED, "invalid user");
    };

    let patch = PlaylistUpdate {
        name: payload.name,
        description: payload.description,
        is_public: payload.is_public,
    };

    match state
        .infra
        .update_playlist(user_id, playlist_id, patch)
        .await
    {
        Ok(playlist) => Json(playlist).into_response(),
        Err(err) => map_playlist_error(&err).unwrap_or_else(|| {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to update playlist: {err}"),
            )
        }),
    }
}

async fn delete_playlist(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    AxPath(playlist_id): AxPath<Uuid>,
) -> Response {
    let user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let Some(user_id) = parse_user_uuid(&user) else {
        return error_response(StatusCode::UNAUTHORIZED, "invalid user");
    };

    match state.infra.delete_playlist(user_id, playlist_id).await {
        Ok(deleted) => Json(json!({"deleted": deleted})).into_response(),
        Err(err) => map_playlist_error(&err).unwrap_or_else(|| {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to delete playlist: {err}"),
            )
        }),
    }
}

async fn list_playlist_items(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    AxPath(playlist_id): AxPath<Uuid>,
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
        .list_playlist_items_visible_to(user_id, playlist_id)
        .await
    {
        Ok(items) => Json(json!({
            "items": items,
            "total": items.len(),
        }))
        .into_response(),
        Err(err) => map_playlist_error(&err).unwrap_or_else(|| {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to list playlist items: {err}"),
            )
        }),
    }
}

async fn add_playlist_item(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    AxPath(playlist_id): AxPath<Uuid>,
    Json(payload): Json<AddPlaylistItemRequest>,
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
        .add_item_to_playlist(user_id, playlist_id, payload.item_id)
        .await
    {
        Ok(item) => (StatusCode::CREATED, Json(item)).into_response(),
        Err(err) => map_playlist_error(&err).unwrap_or_else(|| {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to add playlist item: {err}"),
            )
        }),
    }
}

async fn remove_playlist_item(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    AxPath((playlist_id, item_id)): AxPath<(Uuid, Uuid)>,
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
        .remove_item_from_playlist(user_id, playlist_id, item_id)
        .await
    {
        Ok(removed) => Json(json!({"removed": removed})).into_response(),
        Err(err) => map_playlist_error(&err).unwrap_or_else(|| {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to remove playlist item: {err}"),
            )
        }),
    }
}

#[derive(Debug, Deserialize)]
struct NotificationsQuery {
    limit: Option<i64>,
    offset: Option<i64>,
}

async fn list_notifications(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<NotificationsQuery>,
) -> Response {
    let user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let Some(user_id) = parse_user_uuid(&user) else {
        return error_response(StatusCode::UNAUTHORIZED, "invalid user");
    };

    let limit = query.limit.unwrap_or(20);
    let offset = query.offset.unwrap_or(0);

    match state.infra.list_notifications(user_id, limit, offset).await {
        Ok((notifications, total)) => Json(json!({
            "items": notifications,
            "total": total,
            "limit": limit,
            "offset": offset,
        }))
        .into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to list notifications: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct CreateNotificationRequest {
    user_id: Uuid,
    title: String,
    message: String,
    #[serde(default = "default_notification_type")]
    notification_type: String,
    #[serde(default)]
    meta: Value,
}

fn default_notification_type() -> String {
    "info".to_string()
}

async fn create_notification(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<CreateNotificationRequest>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    if payload.title.trim().is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "title is required");
    }
    if payload.message.trim().is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "message is required");
    }

    match state
        .infra
        .create_notification(
            payload.user_id,
            payload.title.trim(),
            payload.message.trim(),
            &payload.notification_type,
            payload.meta,
        )
        .await
    {
        Ok(notification) => (StatusCode::CREATED, Json(notification)).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to create notification: {err}"),
        ),
    }
}

async fn mark_notification_read(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    AxPath(notification_id): AxPath<Uuid>,
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
        .mark_notification_read(user_id, notification_id)
        .await
    {
        Ok(updated) => Json(json!({"updated": updated})).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to mark notification read: {err}"),
        ),
    }
}

async fn mark_all_notifications_read(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let Some(user_id) = parse_user_uuid(&user) else {
        return error_response(StatusCode::UNAUTHORIZED, "invalid user");
    };

    match state.infra.mark_all_notifications_read(user_id).await {
        Ok(count) => Json(json!({"updated_count": count})).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to mark all notifications read: {err}"),
        ),
    }
}

async fn notifications_ws(
    State(state): State<ApiContext>,
    req: HttpRequest,
    body: web::Payload,
) -> Response {
    let headers = HeaderMap(req.headers().clone());
    let uri = Uri(req.uri().clone());
    let user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let Some(user_id) = parse_user_uuid(&user) else {
        return error_response(StatusCode::UNAUTHORIZED, "invalid user");
    };

    let (response, mut session, mut msg_stream) = match actix_ws::handle(&req, body) {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to upgrade websocket: {err}"),
            );
        }
    };
    let mut rx = state.infra.subscribe_notifications();

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
                        Ok(notification) => {
                            if notification.user_id == user_id {
                                if let Ok(text) = serde_json::to_string(&notification) {
                                    if session.text(text).await.is_err() {
                                        break;
                                    }
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
struct AdminCleanupStorageCacheRequest {
    max_age_seconds: Option<i64>,
}
