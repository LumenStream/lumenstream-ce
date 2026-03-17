// Emby-compatible /Playlists and /Collections routes.
// Adapts Emby API conventions (query-param Ids, comma-separated values)
// to the existing LumenStream playlist infrastructure.

async fn resolve_csv_item_ids(state: &ApiContext, raw: Option<&str>) -> Vec<Uuid> {
    let mut out = Vec::new();
    for token in split_csv(raw) {
        if let Ok(Some(uuid)) = state.infra.resolve_uuid_by_any_item_id(token.as_str()).await {
            out.push(uuid);
        }
    }
    out
}

#[derive(Debug, Deserialize)]
struct EmbyCreatePlaylistRequest {
    #[serde(rename = "Name", alias = "name")]
    name: Option<String>,
    #[serde(rename = "Ids", alias = "ids")]
    ids: Option<String>,
    #[serde(rename = "UserId", alias = "userId")]
    _user_id: Option<String>,
    #[serde(rename = "MediaType", alias = "mediaType")]
    _media_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EmbyPlaylistItemsQuery {
    #[serde(rename = "Ids", alias = "ids")]
    ids: Option<String>,
    #[serde(rename = "UserId", alias = "userId")]
    _user_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EmbyCreateCollectionQuery {
    #[serde(rename = "Name", alias = "name")]
    name: Option<String>,
    #[serde(rename = "Ids", alias = "ids")]
    ids: Option<String>,
    #[serde(rename = "ParentId", alias = "parentId")]
    _parent_id: Option<String>,
    #[serde(rename = "IsLocked", alias = "isLocked")]
    _is_locked: Option<String>,
}

// ── Playlist handlers ──

async fn emby_create_playlist(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<EmbyCreatePlaylistRequest>,
) -> Response {
    let user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let Some(user_id) = parse_user_uuid(&user) else {
        return error_response(StatusCode::UNAUTHORIZED, "invalid user");
    };

    let name = match payload.name.as_deref() {
        Some(n) if !n.trim().is_empty() => n.trim(),
        _ => return error_response(StatusCode::BAD_REQUEST, "Name is required"),
    };

    let playlist = match state.infra.create_playlist(user_id, name, None, false).await {
        Ok(p) => p,
        Err(err) => {
            return map_playlist_error(&err).unwrap_or_else(|| {
                error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("{err}"))
            });
        }
    };

    // Add initial items if provided
    let item_ids = resolve_csv_item_ids(&state, payload.ids.as_deref()).await;
    if !item_ids.is_empty() {
        let _ = state
            .infra
            .add_items_to_playlist_batch(user_id, playlist.id, &item_ids)
            .await;
    }

    Json(json!({ "Id": playlist.id.to_string() })).into_response()
}

async fn emby_get_playlist_items(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    AxPath(playlist_id): AxPath<String>,
) -> Response {
    let user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let Some(user_id) = parse_user_uuid(&user) else {
        return error_response(StatusCode::UNAUTHORIZED, "invalid user");
    };
    let Ok(Some(pid)) = state.infra.resolve_uuid_by_any_item_id(&playlist_id).await else {
        return error_response(StatusCode::NOT_FOUND, "playlist not found");
    };

    match state.infra.list_playlist_items_visible_to(user_id, pid).await {
        Ok(items) => {
            let total = items.len();
            Json(json!({ "Items": items, "TotalRecordCount": total })).into_response()
        }
        Err(err) => map_playlist_error(&err).unwrap_or_else(|| {
            error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("{err}"))
        }),
    }
}

async fn emby_add_playlist_items(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    AxPath(playlist_id): AxPath<String>,
    Query(query): Query<EmbyPlaylistItemsQuery>,
) -> Response {
    let user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let Some(user_id) = parse_user_uuid(&user) else {
        return error_response(StatusCode::UNAUTHORIZED, "invalid user");
    };
    let Ok(Some(pid)) = state.infra.resolve_uuid_by_any_item_id(&playlist_id).await else {
        return error_response(StatusCode::NOT_FOUND, "playlist not found");
    };

    let item_ids = resolve_csv_item_ids(&state, query.ids.as_deref()).await;
    if item_ids.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "Ids is required");
    }

    match state.infra.add_items_to_playlist_batch(user_id, pid, &item_ids).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => map_playlist_error(&err).unwrap_or_else(|| {
            error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("{err}"))
        }),
    }
}

async fn emby_delete_playlist_items(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    AxPath(playlist_id): AxPath<String>,
    Query(query): Query<EmbyPlaylistItemsQuery>,
) -> Response {
    let user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let Some(user_id) = parse_user_uuid(&user) else {
        return error_response(StatusCode::UNAUTHORIZED, "invalid user");
    };
    let Ok(Some(pid)) = state.infra.resolve_uuid_by_any_item_id(&playlist_id).await else {
        return error_response(StatusCode::NOT_FOUND, "playlist not found");
    };

    let item_ids = resolve_csv_item_ids(&state, query.ids.as_deref()).await;
    if item_ids.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "Ids is required");
    }

    match state.infra.remove_items_from_playlist_batch(user_id, pid, &item_ids).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => map_playlist_error(&err).unwrap_or_else(|| {
            error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("{err}"))
        }),
    }
}

async fn emby_move_playlist_item(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    AxPath((_playlist_id, _item_id, _new_index)): AxPath<(String, String, String)>,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }
    // Playlist items use an unordered set model (playlist_items PK is playlist_id+media_item_id).
    // Reordering is accepted but treated as no-op for now.
    StatusCode::NO_CONTENT.into_response()
}

// ── Collection handlers ──

async fn emby_create_collection(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<EmbyCreateCollectionQuery>,
) -> Response {
    let user = match require_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let Some(user_id) = parse_user_uuid(&user) else {
        return error_response(StatusCode::UNAUTHORIZED, "invalid user");
    };

    let name = match query.name.as_deref() {
        Some(n) if !n.trim().is_empty() => n.trim(),
        _ => return error_response(StatusCode::BAD_REQUEST, "Name is required"),
    };

    let collection = match state.infra.create_collection(user_id, name).await {
        Ok(c) => c,
        Err(err) => {
            return map_playlist_error(&err).unwrap_or_else(|| {
                error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("{err}"))
            });
        }
    };

    let item_ids = resolve_csv_item_ids(&state, query.ids.as_deref()).await;
    if !item_ids.is_empty() {
        let _ = state
            .infra
            .add_items_to_playlist_batch(user_id, collection.id, &item_ids)
            .await;
    }

    Json(json!({ "Id": collection.id.to_string() })).into_response()
}

async fn emby_add_collection_items(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    AxPath(collection_id): AxPath<String>,
    Query(query): Query<EmbyPlaylistItemsQuery>,
) -> Response {
    let user = match require_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let Some(user_id) = parse_user_uuid(&user) else {
        return error_response(StatusCode::UNAUTHORIZED, "invalid user");
    };
    let Ok(Some(cid)) = state.infra.resolve_uuid_by_any_item_id(&collection_id).await else {
        return error_response(StatusCode::NOT_FOUND, "collection not found");
    };

    let item_ids = resolve_csv_item_ids(&state, query.ids.as_deref()).await;
    if item_ids.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "Ids is required");
    }

    match state.infra.add_items_to_playlist_batch(user_id, cid, &item_ids).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => map_playlist_error(&err).unwrap_or_else(|| {
            error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("{err}"))
        }),
    }
}

async fn emby_delete_collection_items(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    AxPath(collection_id): AxPath<String>,
    Query(query): Query<EmbyPlaylistItemsQuery>,
) -> Response {
    let user = match require_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let Some(user_id) = parse_user_uuid(&user) else {
        return error_response(StatusCode::UNAUTHORIZED, "invalid user");
    };
    let Ok(Some(cid)) = state.infra.resolve_uuid_by_any_item_id(&collection_id).await else {
        return error_response(StatusCode::NOT_FOUND, "collection not found");
    };

    let item_ids = resolve_csv_item_ids(&state, query.ids.as_deref()).await;
    if item_ids.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "Ids is required");
    }

    match state.infra.remove_items_from_playlist_batch(user_id, cid, &item_ids).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => map_playlist_error(&err).unwrap_or_else(|| {
            error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("{err}"))
        }),
    }
}
