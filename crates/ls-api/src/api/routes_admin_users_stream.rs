async fn admin_get_user_stream_policy(
    State(state): State<ApiContext>,
    AxPath(user_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
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

    match state.infra.get_user_stream_policy(user_id).await {
        Ok(policy) => Json(json!({
            "user_id": user_id,
            "username": user.name,
            "policy": policy,
            "defaults": {
                "max_concurrent_streams": state.infra.config_snapshot().security.default_user_max_concurrent_streams,
                "traffic_quota_bytes": state.infra.config_snapshot().security.default_user_traffic_quota_bytes,
                "traffic_window_days": state.infra.config_snapshot().security.default_user_traffic_window_days,
            }
        }))
        .into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to get stream policy: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct AdminUpsertUserStreamPolicyRequest {
    #[serde(default)]
    expires_at: Option<Option<DateTime<Utc>>>,
    #[serde(default)]
    max_concurrent_streams: Option<Option<i32>>,
    #[serde(default)]
    traffic_quota_bytes: Option<Option<i64>>,
    #[serde(default)]
    traffic_window_days: Option<i32>,
}

async fn admin_upsert_user_stream_policy(
    State(state): State<ApiContext>,
    AxPath(user_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<AdminUpsertUserStreamPolicyRequest>,
) -> Response {
    let actor = match require_super_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let updated = match state
        .infra
        .upsert_user_stream_policy(
            user_id,
            UserStreamPolicyUpdate {
                expires_at: payload.expires_at,
                max_concurrent_streams: payload.max_concurrent_streams,
                traffic_quota_bytes: payload.traffic_quota_bytes,
                traffic_window_days: payload.traffic_window_days,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "user not found"),
        Err(err) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("failed to update user stream policy: {err}"),
            );
        }
    };

    if let Err(err) = state
        .infra
        .log_audit_event(
            parse_user_uuid(&actor),
            "admin.user.stream_policy.update",
            "user",
            Some(&user_id.to_string()),
            json!({
                "expires_at": updated.expires_at,
                "max_concurrent_streams": updated.max_concurrent_streams,
                "traffic_quota_bytes": updated.traffic_quota_bytes,
                "traffic_window_days": updated.traffic_window_days,
            }),
        )
        .await
    {
        error!(error = %err, "failed to write audit log for user stream policy update");
    }

    Json(updated).into_response()
}

#[derive(Debug, Deserialize)]
struct AdminUserTrafficUsageQuery {
    window_days: Option<i32>,
}

async fn admin_get_user_traffic_usage(
    State(state): State<ApiContext>,
    AxPath(user_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<AdminUserTrafficUsageQuery>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    match state.infra.get_user_by_id(user_id).await {
        Ok(Some(_)) => {}
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "user not found"),
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to get user: {err}"),
            );
        }
    }

    match state
        .infra
        .get_user_traffic_usage_summary(user_id, query.window_days)
        .await
    {
        Ok(summary) => Json(summary).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to query user traffic usage: {err}"),
        ),
    }
}

async fn admin_reset_user_traffic_usage(
    State(state): State<ApiContext>,
    AxPath(user_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let actor = match require_super_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    match state.infra.get_user_by_id(user_id).await {
        Ok(Some(_)) => {}
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "user not found"),
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to get user: {err}"),
            );
        }
    }

    let deleted = match state.infra.reset_user_traffic_usage(user_id).await {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to reset user traffic usage: {err}"),
            );
        }
    };

    if let Err(err) = state
        .infra
        .log_audit_event(
            parse_user_uuid(&actor),
            "admin.user.traffic_usage.reset",
            "user",
            Some(&user_id.to_string()),
            json!({
                "deleted_rows": deleted,
            }),
        )
        .await
    {
        error!(error = %err, "failed to write audit log for traffic usage reset");
    }

    Json(json!({
        "user_id": user_id,
        "deleted_rows": deleted,
    }))
    .into_response()
}

#[derive(Debug, Deserialize)]
struct AdminTopTrafficUsageQuery {
    limit: Option<i64>,
    window_days: Option<i32>,
}

async fn admin_list_top_traffic_usage(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<AdminTopTrafficUsageQuery>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    let limit = query.limit.unwrap_or(20).clamp(1, 200);
    let window_days = query
        .window_days
        .unwrap_or(state.infra.config_snapshot().security.default_user_traffic_window_days)
        .max(1);

    match state.infra.list_top_traffic_users(limit, window_days).await {
        Ok(items) => Json(json!({
            "limit": limit,
            "window_days": window_days,
            "items": items,
        }))
        .into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to list top traffic users: {err}"),
        ),
    }
}

async fn admin_disable_user(
    State(state): State<ApiContext>,
    AxPath(user_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    admin_set_user_disabled(state, user_id, true, headers, uri).await
}

async fn admin_enable_user(
    State(state): State<ApiContext>,
    AxPath(user_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    admin_set_user_disabled(state, user_id, false, headers, uri).await
}

async fn admin_set_user_disabled(
    state: ApiContext,
    user_id: Uuid,
    disabled: bool,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let user = match require_super_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let updated = match state.infra.set_user_disabled(user_id, disabled).await {
        Ok(Some(v)) => v,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "user not found"),
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to update user status: {err}"),
            );
        }
    };

    if let Err(err) = state
        .infra
        .log_audit_event(
            parse_user_uuid(&user),
            if disabled {
                "admin.user.disable"
            } else {
                "admin.user.enable"
            },
            "user",
            Some(&updated.id),
            json!({
                "is_disabled": disabled,
            }),
        )
        .await
    {
        error!(error = %err, "failed to write audit log for user status");
    }

    Json(updated).into_response()
}

#[derive(Debug, Deserialize)]
struct AdminSessionsQuery {
    limit: Option<i64>,
    active_only: Option<bool>,
}

