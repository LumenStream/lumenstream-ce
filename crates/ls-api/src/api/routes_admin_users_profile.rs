async fn admin_list_users(State(state): State<ApiContext>, headers: HeaderMap, uri: Uri) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    match state.infra.list_users_for_admin().await {
        Ok(users) => Json(users).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to list users: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct AdminUserSummaryQueryRequest {
    q: Option<String>,
    status: Option<String>,
    role: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
    sort_by: Option<String>,
    sort_dir: Option<String>,
}

async fn admin_list_user_summaries(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<AdminUserSummaryQueryRequest>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    let request = AdminUserSummaryQuery {
        q: query.q,
        status: query.status,
        role: query.role,
        page: query.page.unwrap_or(1),
        page_size: query.page_size.unwrap_or(20),
        sort_by: query.sort_by,
        sort_dir: query.sort_dir,
    };

    match state.infra.list_admin_user_summaries(request).await {
        Ok(payload) => {
            let capabilities = edition_capabilities(&state);
            let masked = serde_json::to_value(payload)
                .map(|value| mask_admin_user_summary_page_payload(&capabilities, value))
                .unwrap_or_else(|_| json!({ "page": 1, "page_size": 20, "total": 0, "items": [] }));
            Json(masked).into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to list user summaries: {err}"),
        ),
    }
}

async fn admin_get_user_profile(
    State(state): State<ApiContext>,
    AxPath(user_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    match state.infra.get_admin_user_manage_profile(user_id).await {
        Ok(Some(payload)) => {
            let capabilities = edition_capabilities(&state);
            let masked = serde_json::to_value(payload)
                .map(|value| mask_admin_user_manage_profile_payload(&capabilities, value))
                .unwrap_or_else(|_| json!({}));
            Json(masked).into_response()
        }
        Ok(None) => error_response(StatusCode::NOT_FOUND, "user not found"),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to get user profile: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct AdminPatchUserProfileRequest {
    #[serde(default, deserialize_with = "deserialize_nullable_string_patch")]
    email: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable_string_patch")]
    display_name: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable_string_patch")]
    remark: Option<Option<String>>,
    role: Option<String>,
    is_disabled: Option<bool>,
}

async fn admin_patch_user_profile(
    State(state): State<ApiContext>,
    AxPath(user_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<AdminPatchUserProfileRequest>,
) -> Response {
    let actor = match require_super_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let role = if let Some(raw_role) = payload.role.as_deref() {
        match parse_user_role_strict(raw_role) {
            Some(value) => Some(value),
            None => return error_response(StatusCode::BAD_REQUEST, "invalid role"),
        }
    } else {
        None
    };

    let updated_user = match state
        .infra
        .update_user_role_and_status(user_id, role.clone(), payload.is_disabled)
        .await
    {
        Ok(Some(value)) => value,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "user not found"),
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to update user profile: {err}"),
            );
        }
    };

    let has_profile_patch =
        payload.email.is_some() || payload.display_name.is_some() || payload.remark.is_some();
    if has_profile_patch {
        if let Err(err) = state
            .infra
            .upsert_user_profile(
                user_id,
                UserProfileUpdate {
                    email: payload.email,
                    display_name: payload.display_name,
                    remark: payload.remark,
                },
            )
            .await
        {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to update user profile fields: {err}"),
            );
        }
    }

    let refreshed = match state.infra.get_admin_user_manage_profile(user_id).await {
        Ok(Some(value)) => value,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "user not found"),
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to reload user profile: {err}"),
            );
        }
    };

    if let Err(err) = state
        .infra
        .log_audit_event(
            parse_user_uuid(&actor),
            "admin.user.profile.update",
            "user",
            Some(&updated_user.id),
            json!({
                "role": role.as_ref().map(UserRole::as_str),
                "is_disabled": payload.is_disabled,
                "profile_fields_updated": has_profile_patch,
            }),
        )
        .await
    {
        error!(error = %err, "failed to write audit log for user profile update");
    }

    let capabilities = edition_capabilities(&state);
    let masked = serde_json::to_value(refreshed)
        .map(|value| mask_admin_user_manage_profile_payload(&capabilities, value))
        .unwrap_or_else(|_| json!({}));
    Json(masked).into_response()
}

async fn admin_delete_user(
    State(state): State<ApiContext>,
    AxPath(user_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let actor = match require_super_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    if let Some(actor_id) = parse_user_uuid(&actor) {
        if actor_id == user_id {
            return error_response(StatusCode::BAD_REQUEST, "cannot delete yourself");
        }
    }

    match state.infra.delete_user(user_id).await {
        Ok(true) => {
            if let Err(err) = state
                .infra
                .log_audit_event(
                    parse_user_uuid(&actor),
                    "admin.user.delete",
                    "user",
                    Some(&user_id.to_string()),
                    json!({}),
                )
                .await
            {
                error!(error = %err, "failed to write audit log for admin user delete");
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

async fn admin_create_user(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<AdminCreateUserRequest>,
) -> Response {
    let user = match require_super_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    if payload.username.trim().is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "username is required");
    }
    if payload.password.len() < 6 {
        return error_response(StatusCode::BAD_REQUEST, "password too short");
    }

    let role = parse_user_role(payload.role.as_deref(), payload.is_admin);

    let created = match state
        .infra
        .create_user(payload.username.trim(), &payload.password, role)
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
            parse_user_uuid(&user),
            "admin.user.create",
            "user",
            Some(&created.id),
            json!({
                "username": created.name,
                "is_admin": created.policy.is_administrator,
            }),
        )
        .await
    {
        error!(error = %err, "failed to write audit log for user create");
    }

    Json(created).into_response()
}

#[derive(Debug, Deserialize)]
struct AdminBatchUserStatusRequest {
    user_ids: Vec<Uuid>,
    disabled: bool,
}

#[derive(Debug, Serialize)]
struct AdminBatchUserStatusResponse {
    requested: usize,
    updated: usize,
    users: Vec<UserDto>,
}

async fn admin_batch_set_user_status(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<AdminBatchUserStatusRequest>,
) -> Response {
    let user = match require_super_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    if payload.user_ids.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "user_ids is required");
    }

    if payload.user_ids.len() > 500 {
        return error_response(StatusCode::BAD_REQUEST, "user_ids exceeds limit (500)");
    }

    let updated = match state
        .infra
        .set_users_disabled_bulk(&payload.user_ids, payload.disabled)
        .await
    {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to batch update user status: {err}"),
            );
        }
    };

    if let Err(err) = state
        .infra
        .log_audit_event(
            parse_user_uuid(&user),
            if payload.disabled {
                "admin.user.batch_disable"
            } else {
                "admin.user.batch_enable"
            },
            "user",
            None,
            json!({
                "requested": payload.user_ids.len(),
                "updated": updated.len(),
                "disabled": payload.disabled,
            }),
        )
        .await
    {
        error!(error = %err, "failed to write audit log for batch user status");
    }

    Json(AdminBatchUserStatusResponse {
        requested: payload.user_ids.len(),
        updated: updated.len(),
        users: updated,
    })
    .into_response()
}
