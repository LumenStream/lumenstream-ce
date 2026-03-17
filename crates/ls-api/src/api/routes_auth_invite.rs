#[derive(Debug, Deserialize)]
struct RegisterWithInviteRequest {
    username: String,
    password: String,
    invite_code: Option<String>,
}

async fn register_with_invite(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    Json(payload): Json<RegisterWithInviteRequest>,
) -> Response {
    let username = payload.username.trim();
    if username.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "username is required");
    }

    if payload.password.len() < 6 {
        return error_response(StatusCode::BAD_REQUEST, "password too short (min 6 chars)");
    }

    let invite_code = payload
        .invite_code
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    if state.infra.config_snapshot().auth.invite.force_on_register && invite_code.is_none() {
        return error_response(StatusCode::BAD_REQUEST, "invite code is required");
    }

    let created = match state
        .infra
        .register_user_with_invite(username, &payload.password, invite_code)
        .await
    {
        Ok(user) => user,
        Err(err) => {
            if let Some(resp) = map_invite_error(&err) {
                return resp;
            }
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to register user: {err}"),
            );
        }
    };

    let remote_addr = extract_client_ip(&headers, None, &state.infra.config_snapshot().security);
    let client = header_or_default(&headers, "X-Emby-Client", "ls-web");
    let device_name = header_or_default(&headers, "X-Emby-Device-Name", "ls-web-browser");
    let device_id = header_or_default(&headers, "X-Emby-Device-Id", "ls-web-register");

    if let Err(err) = state
        .infra
        .log_audit_event(
            parse_user_uuid(&created),
            "user.register",
            "user",
            Some(&created.id),
            json!({
                "invite_code_present": invite_code.is_some(),
            }),
        )
        .await
    {
        error!(error = %err, "failed to write audit log for user register");
    }

    match state
        .infra
        .authenticate_user(
            username,
            &payload.password,
            client.as_str(),
            device_name.as_str(),
            device_id.as_str(),
            remote_addr.as_deref(),
            None,
        )
        .await
    {
        Ok(AuthenticateUserResult::Success(result)) => Json(result).into_response(),
        Ok(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "registration succeeded but login bootstrap failed",
        ),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("registration login failed: {err}"),
        ),
    }
}

async fn get_my_invite_summary(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(user) => user,
        Err(resp) => return resp,
    };

    let user_id = match Uuid::parse_str(&auth_user.id) {
        Ok(v) => v,
        Err(_) => return error_response(StatusCode::UNAUTHORIZED, "invalid user id"),
    };
    let capabilities = edition_capabilities(&state);

    match state.infra.get_invite_summary(user_id).await {
        Ok(Some(summary)) => Json(
            invite_summary_payload(invite_rewards_enabled(&capabilities), &summary),
        )
        .into_response(),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "user not found"),
        Err(err) => {
            if let Some(resp) = map_invite_error(&err) {
                return resp;
            }
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to load invite summary: {err}"),
            )
        }
    }
}

async fn reset_my_invite_code(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(user) => user,
        Err(resp) => return resp,
    };

    let user_id = match Uuid::parse_str(&auth_user.id) {
        Ok(v) => v,
        Err(_) => return error_response(StatusCode::UNAUTHORIZED, "invalid user id"),
    };
    let capabilities = edition_capabilities(&state);

    match state.infra.reset_invite_code(user_id).await {
        Ok(Some(summary)) => {
            let _ = state
                .infra
                .log_audit_event(
                    parse_user_uuid(&auth_user),
                    "user.invite.reset",
                    "user",
                    Some(&auth_user.id),
                    json!({}),
                )
                .await;
            Json(invite_summary_payload(
                invite_rewards_enabled(&capabilities),
                &summary,
            ))
            .into_response()
        }
        Ok(None) => error_response(StatusCode::NOT_FOUND, "user not found"),
        Err(err) => {
            if let Some(resp) = map_invite_error(&err) {
                return resp;
            }
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to reset invite code: {err}"),
            )
        }
    }
}

#[derive(Debug, Deserialize)]
struct MyTrafficUsageMediaQuery {
    #[serde(alias = "windowDays")]
    window_days: Option<i32>,
    limit: Option<i64>,
}

fn normalize_my_traffic_window_days(window_days: Option<i32>) -> i32 {
    window_days.unwrap_or(30).clamp(1, 30)
}

async fn get_my_traffic_usage_media(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<MyTrafficUsageMediaQuery>,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(user) => user,
        Err(resp) => return resp,
    };

    let user_id = match Uuid::parse_str(&auth_user.id) {
        Ok(v) => v,
        Err(_) => return error_response(StatusCode::UNAUTHORIZED, "invalid user id"),
    };

    let window_days = normalize_my_traffic_window_days(query.window_days);
    match state
        .infra
        .get_user_traffic_usage_media_summary(user_id, Some(window_days), query.limit)
        .await
    {
        Ok(summary) => Json(summary).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to load traffic usage media summary: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct AdminInviteSettingsPatchRequest {
    force_on_register: Option<bool>,
    invitee_bonus_enabled: Option<bool>,
    invitee_bonus_amount: Option<Decimal>,
    inviter_rebate_enabled: Option<bool>,
    inviter_rebate_rate: Option<Decimal>,
}

#[derive(Debug, Deserialize)]
struct AdminInviteListQuery {
    limit: Option<i64>,
}

async fn admin_get_invite_settings(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_super_admin(&state, &headers, &uri).await {
        return resp;
    }
    let capabilities = edition_capabilities(&state);

    match state.infra.get_web_settings().await {
        Ok(settings) => {
            Json(invite_settings_payload(
                invite_rewards_enabled(&capabilities),
                &settings.auth.invite,
            ))
                .into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to load invite settings: {err}"),
        ),
    }
}

async fn admin_upsert_invite_settings(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<AdminInviteSettingsPatchRequest>,
) -> Response {
    let actor = match require_super_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let capabilities = edition_capabilities(&state);

    let mut settings = match state.infra.get_web_settings().await {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to load current settings: {err}"),
            );
        }
    };

    apply_invite_settings_update(
        &mut settings.auth.invite,
        &payload,
        invite_rewards_enabled(&capabilities),
    );

    match state.infra.upsert_web_settings(&settings).await {
        Ok(()) => {
            let _ = state
                .infra
                .log_audit_event(
                    parse_user_uuid(&actor),
                    "admin.invite.settings.upsert",
                    "web_settings",
                    Some("global"),
                    json!({
                        "force_on_register": settings.auth.invite.force_on_register,
                        "invitee_bonus_enabled": if invite_rewards_enabled(&capabilities) { Some(settings.auth.invite.invitee_bonus_enabled) } else { None },
                        "invitee_bonus_amount": if invite_rewards_enabled(&capabilities) { Some(settings.auth.invite.invitee_bonus_amount) } else { None },
                        "inviter_rebate_enabled": if invite_rewards_enabled(&capabilities) { Some(settings.auth.invite.inviter_rebate_enabled) } else { None },
                        "inviter_rebate_rate": if invite_rewards_enabled(&capabilities) { Some(settings.auth.invite.inviter_rebate_rate) } else { None },
                    }),
                )
                .await;

            Json(invite_settings_payload(
                invite_rewards_enabled(&capabilities),
                &settings.auth.invite,
            ))
            .into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to persist invite settings: {err}"),
        ),
    }
}

async fn admin_list_invite_relations(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<AdminInviteListQuery>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    let limit = query.limit.unwrap_or(100).clamp(1, 500);
    match state.infra.list_invite_relations(limit).await {
        Ok(items) => Json(items).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to list invite relations: {err}"),
        ),
    }
}

async fn admin_list_invite_rebates(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<AdminInviteListQuery>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    let limit = query.limit.unwrap_or(100).clamp(1, 500);
    match state.infra.list_invite_rebates(limit).await {
        Ok(items) => Json(items).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to list invite rebates: {err}"),
        ),
    }
}

fn apply_invite_settings_update(
    settings: &mut InviteConfig,
    payload: &AdminInviteSettingsPatchRequest,
    rewards_enabled: bool,
) {
    if let Some(v) = payload.force_on_register {
        settings.force_on_register = v;
    }
    if rewards_enabled {
        if let Some(v) = payload.invitee_bonus_enabled {
            settings.invitee_bonus_enabled = v;
        }
        if let Some(v) = payload.invitee_bonus_amount {
            settings.invitee_bonus_amount = v.max(Decimal::ZERO).round_dp(2);
        }
        if let Some(v) = payload.inviter_rebate_enabled {
            settings.inviter_rebate_enabled = v;
        }
        if let Some(v) = payload.inviter_rebate_rate {
            settings.inviter_rebate_rate = v.clamp(Decimal::ZERO, Decimal::ONE).round_dp(4);
        }
    } else {
        settings.invitee_bonus_enabled = false;
        settings.invitee_bonus_amount = Decimal::ZERO;
        settings.inviter_rebate_enabled = false;
        settings.inviter_rebate_rate = Decimal::ZERO;
    }
}
