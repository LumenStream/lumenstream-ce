fn compat_public_users_payload() -> Value {
    // Emby-compatible behavior: do not expose public user list.
    Value::Array(Vec::new())
}

async fn get_public_users(_state: State<ApiContext>) -> Response {
    Json(compat_public_users_payload()).into_response()
}

async fn get_user_by_id(
    State(state): State<ApiContext>,
    AxPath(user_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }

    match state.infra.get_user_by_id(user_id).await {
        Ok(Some(user)) => Json(user).into_response(),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "user not found"),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to get user: {err}"),
        ),
    }
}

/// POST /Users/New - Create a new user (Jellyfin compatible)
async fn create_user_jellyfin(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<CreateUserByName>,
) -> Response {
    let admin_user = match require_super_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    if payload.name.trim().is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "Name is required");
    }

    let password = payload.password.as_deref().unwrap_or("");
    if password.len() < 6 && !password.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "Password too short (min 6 chars)");
    }

    let created = match state
        .infra
        .create_user(payload.name.trim(), password, UserRole::Viewer)
        .await
    {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to create user: {err}"),
            );
        }
    };

    if let Err(err) = state
        .infra
        .log_audit_event(
            parse_user_uuid(&admin_user),
            "user.create",
            "user",
            Some(&created.id),
            json!({ "username": created.name }),
        )
        .await
    {
        error!(error = %err, "failed to write audit log for user create");
    }

    Json(created).into_response()
}

/// DELETE /Users/{userId} - Delete a user (admin only)
async fn delete_user_jellyfin(
    State(state): State<ApiContext>,
    AxPath(user_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let admin_user = match require_super_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    // Cannot delete self
    if let Some(admin_id) = parse_user_uuid(&admin_user) {
        if admin_id == user_id {
            return error_response(StatusCode::BAD_REQUEST, "Cannot delete yourself");
        }
    }

    match state.infra.delete_user(user_id).await {
        Ok(true) => {
            if let Err(err) = state
                .infra
                .log_audit_event(
                    parse_user_uuid(&admin_user),
                    "user.delete",
                    "user",
                    Some(&user_id.to_string()),
                    json!({}),
                )
                .await
            {
                error!(error = %err, "failed to write audit log for user delete");
            }
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(false) => error_response(StatusCode::NOT_FOUND, "user not found"),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to delete user: {err}"),
        ),
    }
}

/// POST /Users/{userId}/Password - Update user password
async fn update_user_password(
    State(state): State<ApiContext>,
    AxPath(user_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<UpdateUserPassword>,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let is_admin = is_super_admin(&auth_user);
    let is_self = parse_user_uuid(&auth_user)
        .map(|id| id == user_id)
        .unwrap_or(false);

    if !is_admin && !is_self {
        return error_response(StatusCode::FORBIDDEN, "Cannot update other user's password");
    }

    let reset_password = payload.reset_password.unwrap_or(false);
    let resolved_new_password = match resolve_new_password(&payload, reset_password, is_admin) {
        Ok(v) => v,
        Err(msg) => return error_response(StatusCode::BAD_REQUEST, msg),
    };

    // Non-admin users must provide current password
    if !is_admin && !reset_password {
        let current_pw = payload.current_pw.as_deref().unwrap_or("");
        match state.infra.verify_user_password(user_id, current_pw).await {
            Ok(PasswordCheckResult::Valid) => {}
            Ok(PasswordCheckResult::Invalid) => {
                return error_response(StatusCode::UNAUTHORIZED, "Current password is incorrect");
            }
            Ok(PasswordCheckResult::PasswordResetRequired) => {
                return error_response(
                    StatusCode::FORBIDDEN,
                    "legacy password hash detected; ask an administrator to reinitialize password via /Users/{userId}/Password",
                );
            }
            Err(err) => {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("failed to verify password: {err}"),
                );
            }
        }
    }

    if resolved_new_password.len() < 6 {
        return error_response(
            StatusCode::BAD_REQUEST,
            "New password too short (min 6 chars)",
        );
    }

    match state
        .infra
        .update_user_password(user_id, &resolved_new_password)
        .await
    {
        Ok(Some(_)) => StatusCode::NO_CONTENT.into_response(),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "user not found"),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to update password: {err}"),
        ),
    }
}

fn resolve_new_password(
    payload: &UpdateUserPassword,
    reset_password: bool,
    is_admin: bool,
) -> Result<String, &'static str> {
    if let Some(new_pw) = payload.new_pw.as_deref() {
        return Ok(new_pw.to_string());
    }

    if reset_password && is_admin {
        return Ok(Uuid::new_v4().to_string());
    }

    Err("NewPw or NewPassword is required")
}

/// POST /Users/{userId}/Configuration - Update user configuration
async fn update_user_configuration(
    State(state): State<ApiContext>,
    AxPath(user_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
    Json(_payload): Json<UserConfiguration>,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let is_admin = is_super_admin(&auth_user);
    let is_self = parse_user_uuid(&auth_user)
        .map(|id| id == user_id)
        .unwrap_or(false);

    if !is_admin && !is_self {
        return error_response(
            StatusCode::FORBIDDEN,
            "Cannot update other user's configuration",
        );
    }

    // Verify user exists
    match state.infra.get_user_by_id(user_id).await {
        Ok(Some(_)) => {
            // Configuration is accepted but not persisted (stub for Jellyfin compatibility)
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(None) => error_response(StatusCode::NOT_FOUND, "user not found"),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to get user: {err}"),
        ),
    }
}

/// POST /Users/{userId}/Policy - Update user policy (admin only)
async fn update_user_policy(
    State(state): State<ApiContext>,
    AxPath(user_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<UserPolicyUpdate>,
) -> Response {
    let admin_user = match require_super_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    match state
        .infra
        .update_user_policy(user_id, payload.is_administrator, payload.is_disabled)
        .await
    {
        Ok(Some(user)) => {
            if let Err(err) = state
                .infra
                .log_audit_event(
                    parse_user_uuid(&admin_user),
                    "user.policy.update",
                    "user",
                    Some(&user_id.to_string()),
                    json!({
                        "is_administrator": payload.is_administrator,
                        "is_disabled": payload.is_disabled,
                        "enable_all_folders": payload.enable_all_folders,
                        "enable_media_playback": payload.enable_media_playback,
                        "enable_remote_access": payload.enable_remote_access,
                        "enable_live_tv_access": payload.enable_live_tv_access,
                        "enable_content_deletion": payload.enable_content_deletion,
                        "enable_content_downloading": payload.enable_content_downloading,
                        "remote_client_bitrate_limit": payload.remote_client_bitrate_limit,
                    }),
                )
                .await
            {
                error!(error = %err, "failed to write audit log for policy update");
            }
            Json(user).into_response()
        }
        Ok(None) => error_response(StatusCode::NOT_FOUND, "user not found"),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to update policy: {err}"),
        ),
    }
}

fn normalize_user_image_type(image_type: &str) -> Option<&'static str> {
    match image_type.trim().to_ascii_lowercase().as_str() {
        // lumenstream only stores one avatar slot; map common Jellyfin types to that slot.
        "primary" | "thumb" | "banner" | "logo" | "art" | "backdrop" => Some("primary"),
        _ => None,
    }
}

/// GET /Users/{userId}/Images/{imageType} - Get user avatar image
async fn get_user_image(
    State(state): State<ApiContext>,
    AxPath((user_id, image_type)): AxPath<(Uuid, String)>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<ImageRequestCompatQuery>,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }

    get_user_image_inner(&state, user_id, &image_type, query, &headers).await
}

/// GET /Users/{userId}/Images/{imageType}/{imageIndex} - Legacy avatar path compatibility
async fn get_user_image_with_index(
    State(state): State<ApiContext>,
    AxPath((user_id, image_type, image_index)): AxPath<(Uuid, String, i32)>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<ImageRequestCompatQuery>,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }

    if !is_supported_legacy_user_image_index(image_index) {
        return error_response(StatusCode::NOT_FOUND, "image index not supported");
    }

    get_user_image_inner(&state, user_id, &image_type, query, &headers).await
}

fn is_supported_legacy_user_image_index(image_index: i32) -> bool {
    image_index >= 0
}

async fn get_user_image_inner(
    state: &ApiContext,
    user_id: Uuid,
    image_type: &str,
    query: ImageRequestCompatQuery,
    headers: &HeaderMap,
) -> Response {
    if normalize_user_image_type(image_type).is_none() {
        return error_response(StatusCode::NOT_FOUND, "image type not supported");
    }

    let resize_request = query.resize_request();
    let etag = image_tag_header_value(query.tag.as_deref());
    let cache_control = if etag.is_some() {
        "public, max-age=31536000, immutable"
    } else {
        "public, max-age=300"
    };

    match state.infra.get_user_avatar_path(user_id).await {
        Ok(Some(path)) => {
            serve_image_with_optional_resize(state, &path, resize_request.as_ref(), etag, headers)
                .await
        }
        Ok(None) => {
            // Return default avatar (empty 1x1 transparent PNG)
            let default_avatar: &[u8] = &[
                0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
                0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
                0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78,
                0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
                0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
            ];
            let mut response = HttpResponse::Ok();
            response.insert_header((header::CONTENT_TYPE, "image/png"));
            response.insert_header((header::CACHE_CONTROL, cache_control));
            if let Some(value) = etag {
                response.insert_header((header::ETAG, value));
            }
            response.body(default_avatar.to_vec())
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to get user avatar: {err}"),
        ),
    }
}

/// POST /Users/{userId}/Images/{imageType} - Upload user avatar image
async fn upload_user_image(
    State(state): State<ApiContext>,
    AxPath((user_id, image_type)): AxPath<(Uuid, String)>,
    headers: HeaderMap,
    uri: Uri,
    body: web::Bytes,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };

    // Users can only upload their own avatar, admins can upload any
    let auth_user_id = match Uuid::parse_str(&auth_user.id) {
        Ok(v) => v,
        Err(_) => return error_response(StatusCode::UNAUTHORIZED, "invalid user id"),
    };

    if auth_user_id != user_id && !auth_user.policy.is_administrator {
        return error_response(StatusCode::FORBIDDEN, "cannot modify other user's avatar");
    }

    if normalize_user_image_type(&image_type).is_none() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "unsupported image type",
        );
    }

    if body.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "empty image body");
    }

    // Detect content type from header or magic bytes
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/png");

    let extension = match content_type {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        "image/gif" => "gif",
        _ => "png",
    };

    match state
        .infra
        .save_user_avatar(user_id, &body, extension)
        .await
    {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to save avatar: {err}"),
        ),
    }
}

/// DELETE /Users/{userId}/Images/{imageType} - Delete user avatar image
async fn delete_user_image(
    State(state): State<ApiContext>,
    AxPath((user_id, image_type)): AxPath<(Uuid, String)>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };

    let auth_user_id = match Uuid::parse_str(&auth_user.id) {
        Ok(v) => v,
        Err(_) => return error_response(StatusCode::UNAUTHORIZED, "invalid user id"),
    };

    if auth_user_id != user_id && !auth_user.policy.is_administrator {
        return error_response(StatusCode::FORBIDDEN, "cannot delete other user's avatar");
    }

    if normalize_user_image_type(&image_type).is_none() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "unsupported image type",
        );
    }

    match state.infra.delete_user_avatar(user_id).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to delete avatar: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct ForgotPasswordRequest {
    #[serde(rename = "EnteredUsername", alias = "enteredUsername")]
    entered_username: Option<String>,
}

async fn forgot_password(
    State(state): State<ApiContext>,
    Json(payload): Json<ForgotPasswordRequest>,
) -> Response {
    let _ = payload
        .entered_username
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToString::to_string);

    let _ = state;
    Json(json!({
        "Action": "None",
        "PinFile": Value::Null,
        "PinExpirationDate": Value::Null,
    }))
    .into_response()
}

#[derive(Debug, Deserialize)]
struct ForgotPasswordPinRequest {
    #[serde(rename = "Pin", alias = "pin")]
    _pin: Option<String>,
}

async fn forgot_password_pin(Json(_payload): Json<ForgotPasswordPinRequest>) -> Response {
    Json(json!({
        "Success": false,
        "UsersReset": [],
    }))
    .into_response()
}

#[derive(Debug, Deserialize)]
struct AuthenticateUserByIdRequest {
    #[serde(rename = "Pw", alias = "pw")]
    pw: Option<String>,
    #[serde(rename = "Password", alias = "password")]
    password: Option<String>,
}

async fn authenticate_user_by_id(
    State(state): State<ApiContext>,
    AxPath(user_id): AxPath<Uuid>,
    headers: HeaderMap,
    request: HttpRequest,
    Json(payload): Json<AuthenticateUserByIdRequest>,
) -> Response {
    let user = match state.infra.get_user_by_id(user_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "user not found"),
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to get user: {err}"),
            );
        }
    };

    let client_context =
        resolve_emby_client_context_with_query(&headers, Some(request.query_string()));
    let remote_addr = extract_client_ip(&headers, None, &state.infra.config_snapshot().security);
    let pw = payload.pw.as_deref().or(payload.password.as_deref()).unwrap_or("");

    match state
        .infra
        .authenticate_user(
            &user.name,
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

async fn post_user_by_id(
    State(state): State<ApiContext>,
    AxPath(user_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
    Json(_payload): Json<Value>,
) -> Response {
    if let Err(resp) = require_super_admin(&state, &headers, &uri).await {
        return resp;
    }

    match state.infra.get_user_by_id(user_id).await {
        Ok(Some(_)) => StatusCode::NO_CONTENT.into_response(),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "user not found"),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to get user: {err}"),
        ),
    }
}

fn ensure_self_or_admin(auth_user: &UserDto, target_user_id: Uuid) -> Result<(), Response> {
    let auth_user_id = parse_user_uuid(auth_user)
        .ok_or_else(|| error_response(StatusCode::UNAUTHORIZED, "invalid user id"))?;
    if auth_user_id != target_user_id && !auth_user.policy.is_administrator {
        return Err(error_response(
            StatusCode::FORBIDDEN,
            "cannot modify other user's state",
        ));
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct ConnectLinkQuery {
    #[serde(rename = "ConnectUsername", alias = "connectUsername")]
    connect_username: Option<String>,
}

async fn post_user_connect_link(
    State(state): State<ApiContext>,
    AxPath(user_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<ConnectLinkQuery>,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_self_or_admin(&auth_user, user_id) {
        return resp;
    }

    let user = match state.infra.get_user_by_id(user_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "user not found"),
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to get user: {err}"),
            );
        }
    };

    Json(json!({
        "IsPending": false,
        "IsNewUserInvitation": false,
        "GuestDisplayName": query.connect_username.unwrap_or(user.name),
    }))
    .into_response()
}

async fn delete_user_connect_link(
    State(state): State<ApiContext>,
    AxPath(user_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_self_or_admin(&auth_user, user_id) {
        return resp;
    }

    match state.infra.get_user_by_id(user_id).await {
        Ok(Some(_)) => StatusCode::NO_CONTENT.into_response(),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "user not found"),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to get user: {err}"),
        ),
    }
}

async fn post_user_easy_password(
    State(state): State<ApiContext>,
    AxPath(user_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
    Json(_payload): Json<Value>,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_self_or_admin(&auth_user, user_id) {
        return resp;
    }

    match state.infra.get_user_by_id(user_id).await {
        Ok(Some(_)) => StatusCode::NO_CONTENT.into_response(),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "user not found"),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to get user: {err}"),
        ),
    }
}

async fn post_user_image_with_index(
    state: State<ApiContext>,
    AxPath((user_id, image_type, _image_index)): AxPath<(Uuid, String, i32)>,
    headers: HeaderMap,
    uri: Uri,
    body: web::Bytes,
) -> Response {
    upload_user_image(state, AxPath((user_id, image_type)), headers, uri, body).await
}

async fn delete_user_image_with_index(
    state: State<ApiContext>,
    AxPath((user_id, image_type, _image_index)): AxPath<(Uuid, String, i32)>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    delete_user_image(state, AxPath((user_id, image_type)), headers, uri).await
}

#[derive(Debug, Deserialize)]
struct ItemRatingQuery {
    #[serde(rename = "Likes", alias = "likes")]
    likes: Option<bool>,
}

async fn post_user_item_rating(
    State(state): State<ApiContext>,
    AxPath((user_id, raw_item_id)): AxPath<(Uuid, String)>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<ItemRatingQuery>,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_self_or_admin(&auth_user, user_id) {
        return resp;
    }
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };

    match state
        .infra
        .set_favorite(user_id, item_id, query.likes.unwrap_or(true))
        .await
    {
        Ok(mut data) => {
            let mut cache = HashMap::new();
            if let Some(compat_id) =
                maybe_compat_item_id_string(&state.infra, &data.item_id, &mut cache)
                    .await
                    .ok()
                    .flatten()
            {
                data.item_id = compat_id;
            }
            Json(data).into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to set item rating: {err}"),
        ),
    }
}

async fn delete_user_item_rating(
    State(state): State<ApiContext>,
    AxPath((user_id, raw_item_id)): AxPath<(Uuid, String)>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_self_or_admin(&auth_user, user_id) {
        return resp;
    }
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };

    match state.infra.set_favorite(user_id, item_id, false).await {
        Ok(mut data) => {
            let mut cache = HashMap::new();
            if let Some(compat_id) =
                maybe_compat_item_id_string(&state.infra, &data.item_id, &mut cache)
                    .await
                    .ok()
                    .flatten()
            {
                data.item_id = compat_id;
            }
            Json(data).into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to clear item rating: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct PlayingItemQuery {
    #[serde(rename = "MediaSourceId", alias = "mediaSourceId")]
    _media_source_id: Option<String>,
    #[serde(rename = "PositionTicks", alias = "positionTicks")]
    position_ticks: Option<i64>,
    #[serde(rename = "PlayMethod", alias = "playMethod")]
    play_method: Option<String>,
    #[serde(rename = "PlaySessionId", alias = "playSessionId")]
    play_session_id: Option<String>,
    #[serde(rename = "LiveStreamId", alias = "liveStreamId")]
    _live_stream_id: Option<String>,
    #[serde(rename = "AudioStreamIndex", alias = "audioStreamIndex")]
    _audio_stream_index: Option<i32>,
    #[serde(rename = "SubtitleStreamIndex", alias = "subtitleStreamIndex")]
    _subtitle_stream_index: Option<i32>,
    #[serde(rename = "CanSeek", alias = "canSeek")]
    _can_seek: Option<bool>,
    #[serde(rename = "NextMediaType", alias = "nextMediaType")]
    _next_media_type: Option<String>,
    #[serde(rename = "IsPaused", alias = "isPaused")]
    _is_paused: Option<bool>,
    #[serde(rename = "IsMuted", alias = "isMuted")]
    _is_muted: Option<bool>,
    #[serde(rename = "VolumeLevel", alias = "volumeLevel")]
    _volume_level: Option<i32>,
    #[serde(rename = "RepeatMode", alias = "repeatMode")]
    _repeat_mode: Option<String>,
}

async fn report_user_playing_item_event(
    state: &ApiContext,
    user_id: Uuid,
    item_id: Uuid,
    query: &PlayingItemQuery,
    event_kind: &str,
) -> Response {
    let payload = PlaybackProgressDto {
        play_session_id: query.play_session_id.clone(),
        item_id: Some(item_id.to_string()),
        position_ticks: query.position_ticks,
        play_method: query.play_method.clone(),
        device_name: None,
        client: None,
        extra: json!({}),
    };

    match state
        .infra
        .report_playback_event(event_kind, user_id, &payload)
        .await
    {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to record playback event: {err}"),
        ),
    }
}

async fn post_user_playing_item_start(
    State(state): State<ApiContext>,
    AxPath((user_id, raw_item_id)): AxPath<(Uuid, String)>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<PlayingItemQuery>,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_self_or_admin(&auth_user, user_id) {
        return resp;
    }
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    report_user_playing_item_event(&state, user_id, item_id, &query, "start").await
}

async fn delete_user_playing_item(
    State(state): State<ApiContext>,
    AxPath((user_id, raw_item_id)): AxPath<(Uuid, String)>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<PlayingItemQuery>,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_self_or_admin(&auth_user, user_id) {
        return resp;
    }
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    report_user_playing_item_event(&state, user_id, item_id, &query, "stopped").await
}

async fn post_user_playing_item_progress(
    State(state): State<ApiContext>,
    AxPath((user_id, raw_item_id)): AxPath<(Uuid, String)>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<PlayingItemQuery>,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_self_or_admin(&auth_user, user_id) {
        return resp;
    }
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    report_user_playing_item_event(&state, user_id, item_id, &query, "progress").await
}

async fn get_user_grouping_options(
    State(state): State<ApiContext>,
    AxPath(_user_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }
    Json(Vec::<Value>::new()).into_response()
}

async fn get_user_item_intros(
    State(state): State<ApiContext>,
    AxPath((_user_id, _item_id)): AxPath<(Uuid, String)>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }
    Json(QueryResultDto::<BaseItemDto> {
        items: Vec::new(),
        total_record_count: 0,
        start_index: 0,
    })
    .into_response()
}

async fn get_user_item_local_trailers(
    State(state): State<ApiContext>,
    AxPath((_user_id, _item_id)): AxPath<(Uuid, String)>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }
    Json(Vec::<BaseItemDto>::new()).into_response()
}

async fn get_user_item_special_features(
    State(state): State<ApiContext>,
    AxPath((_user_id, _item_id)): AxPath<(Uuid, String)>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }
    Json(Vec::<BaseItemDto>::new()).into_response()
}

async fn get_user_suggestions(
    State(state): State<ApiContext>,
    AxPath(user_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }

    let options = InfraItemsQuery {
        user_id: Some(user_id),
        series_filter: None,
        parent_id: None,
        include_item_types: Vec::new(),
        exclude_item_types: Vec::new(),
        person_ids: Vec::new(),
        search_term: None,
        limit: 24,
        start_index: 0,
        is_resumable: false,
        sort_by: vec!["DateCreated".to_string()],
        sort_order: "Descending".to_string(),
        recursive: true,
        genres: Vec::new(),
        tags: Vec::new(),
        years: Vec::new(),
        is_favorite: None,
        is_played: None,
        min_community_rating: None,
    };

    match state.infra.list_items_with_options(options).await {
        Ok(items) => {
            let mut payload = serde_json::to_value(items)
                .unwrap_or_else(|_| json!({"Items": [], "TotalRecordCount": 0, "StartIndex": 0}));
            if let Err(err) = apply_compat_item_ids_for_response(&state.infra, &mut payload).await {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("failed to normalize item ids: {err}"),
                );
            }
            Json(payload).into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to query suggestions: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct ItemsQuery {
    #[serde(rename = "UserId", alias = "userId", alias = "user_id")]
    user_id: Option<Uuid>,
    #[serde(rename = "ParentId", alias = "parentId", alias = "parent_id")]
    parent_id: Option<String>,
    #[serde(
        rename = "IncludeItemTypes",
        alias = "includeItemTypes",
        alias = "include_item_types"
    )]
    include_item_types: Option<String>,
    #[serde(
        rename = "ExcludeItemTypes",
        alias = "excludeItemTypes",
        alias = "exclude_item_types"
    )]
    exclude_item_types: Option<String>,
    #[serde(rename = "MediaTypes", alias = "mediaTypes", alias = "media_types")]
    media_types: Option<String>,
    #[serde(rename = "Ids", alias = "ids")]
    ids: Option<String>,
    #[serde(rename = "PersonIds", alias = "personIds", alias = "person_ids")]
    person_ids: Option<String>,
    #[serde(rename = "SearchTerm", alias = "searchTerm", alias = "search_term")]
    search_term: Option<String>,
    #[serde(
        rename = "NameStartsWith",
        alias = "nameStartsWith",
        alias = "name_starts_with"
    )]
    name_starts_with: Option<String>,
    #[serde(rename = "Filters", alias = "filters")]
    filters: Option<String>,
    #[serde(rename = "Limit", alias = "limit")]
    limit: Option<i64>,
    #[serde(rename = "StartIndex", alias = "startIndex", alias = "start_index")]
    start_index: Option<i64>,
    #[serde(rename = "SortBy", alias = "sortBy", alias = "sort_by")]
    sort_by: Option<String>,
    #[serde(
        rename = "SortOrder",
        alias = "sortOrder",
        alias = "sort_order"
    )]
    sort_order: Option<String>,
    #[serde(rename = "Recursive", alias = "recursive")]
    recursive: Option<bool>,
    #[serde(rename = "Genres", alias = "genres")]
    genres: Option<String>,
    #[serde(rename = "Tags", alias = "tags")]
    tags: Option<String>,
    #[serde(rename = "Years", alias = "years")]
    years: Option<String>,
    #[serde(rename = "IsFavorite", alias = "isFavorite", alias = "is_favorite")]
    is_favorite: Option<bool>,
    #[serde(rename = "IsPlayed", alias = "isPlayed", alias = "is_played")]
    is_played: Option<bool>,
    #[serde(
        rename = "MinCommunityRating",
        alias = "minCommunityRating",
        alias = "min_community_rating"
    )]
    min_community_rating: Option<f64>,
    #[serde(
        rename = "EnableTotalRecordCount",
        alias = "enableTotalRecordCount",
        alias = "enable_total_record_count"
    )]
    enable_total_record_count: Option<bool>,
    #[serde(rename = "Fields", alias = "fields")]
    _fields: Option<String>,
    #[serde(rename = "EnableImages", alias = "enableImages")]
    _enable_images: Option<bool>,
    #[serde(rename = "ImageTypeLimit", alias = "imageTypeLimit")]
    _image_type_limit: Option<i32>,
    #[serde(rename = "EnableImageTypes", alias = "enableImageTypes")]
    _enable_image_types: Option<String>,
    #[serde(rename = "EnableUserData", alias = "enableUserData")]
    _enable_user_data: Option<bool>,
    #[serde(
        rename = "GroupProgramsBySeries",
        alias = "groupProgramsBySeries",
        alias = "group_programs_by_series"
    )]
    _group_programs_by_series: Option<bool>,
}
