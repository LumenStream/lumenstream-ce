async fn billing_epay_return(Query(query): Query<HashMap<String, String>>) -> Response {
    Json(json!({
        "status": "ok",
        "trade_status": query.get("trade_status"),
        "out_trade_no": query.get("out_trade_no"),
    }))
    .into_response()
}

async fn admin_list_billing_permission_groups(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<AdminBillingPermissionGroupsQuery>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    let include_disabled = query.include_disabled.unwrap_or(false);
    match state
        .infra
        .list_account_permission_groups(include_disabled)
        .await
    {
        Ok(groups) => Json(groups).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to list account permission groups: {err}"),
        ),
    }
}

async fn admin_upsert_billing_permission_group(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<AdminUpsertBillingPermissionGroupRequest>,
) -> Response {
    let actor = match require_super_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let request = AccountPermissionGroupUpsert {
        id: payload.id,
        code: payload.code,
        name: payload.name,
        domain_ids: payload.domain_ids,
        enabled: payload.enabled.unwrap_or(true),
    };

    match state.infra.upsert_account_permission_group(request).await {
        Ok(group) => {
            let _ = state
                .infra
                .log_audit_event(
                    parse_user_uuid(&actor),
                    "admin.billing.permission_group.upsert",
                    "account_permission_group",
                    Some(&group.id.to_string()),
                    json!({
                        "code": group.code,
                        "enabled": group.enabled,
                        "domain_count": group.domain_ids.len(),
                    }),
                )
                .await;
            Json(group).into_response()
        }
        Err(err) => {
            let msg = err.to_string();
            if msg.contains("required")
                || msg.contains("not found")
                || msg.contains("playback domain")
                || msg.contains("at least one")
            {
                return error_response(StatusCode::BAD_REQUEST, &msg);
            }
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to upsert account permission group: {err}"),
            )
        }
    }
}

async fn admin_list_billing_plans(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<AdminBillingPlansQuery>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    let include_disabled = query.include_disabled.unwrap_or(false);
    match state.infra.list_billing_plans(include_disabled).await {
        Ok(plans) => Json(plans).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to list billing plans: {err}"),
        ),
    }
}

async fn admin_upsert_billing_plan(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<AdminUpsertBillingPlanRequest>,
) -> Response {
    let actor = match require_super_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let request = BillingPlanUpsert {
        id: payload.id,
        code: payload.code,
        name: payload.name,
        price: payload.price,
        duration_days: payload.duration_days,
        traffic_quota_bytes: payload.traffic_quota_bytes,
        traffic_window_days: payload.traffic_window_days,
        permission_group_id: payload.permission_group_id,
        enabled: payload.enabled.unwrap_or(true),
    };

    match state.infra.upsert_billing_plan(request).await {
        Ok(plan) => {
            let _ = state
                .infra
                .log_audit_event(
                    parse_user_uuid(&actor),
                    "admin.billing.plan.upsert",
                    "billing_plan",
                    Some(&plan.id.to_string()),
                    json!({
                        "code": plan.code,
                        "enabled": plan.enabled,
                        "price": plan.price,
                    }),
                )
                .await;

            Json(plan).into_response()
        }
        Err(err) => {
            if let Some(resp) = map_billing_error(&err) {
                return resp;
            }
            let msg = err.to_string();
            if msg.contains("permission group") {
                return error_response(StatusCode::BAD_REQUEST, &msg);
            }
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to upsert billing plan: {err}"),
            )
        }
    }
}

async fn admin_list_billing_recharge_orders(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<AdminBillingRechargeOrdersQuery>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    let filter = BillingRechargeOrderFilter {
        user_id: query.user_id,
        status: query.status.clone(),
        limit: query.limit.unwrap_or(100),
    };

    match state.infra.list_recharge_orders(filter).await {
        Ok(orders) => Json(orders).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to query recharge orders: {err}"),
        ),
    }
}

async fn admin_get_user_wallet(
    State(state): State<ApiContext>,
    AxPath(user_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    match state.infra.ensure_wallet_account(user_id).await {
        Ok(Some(wallet)) => Json(wallet).into_response(),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "user not found"),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to query wallet: {err}"),
        ),
    }
}

async fn admin_list_user_wallet_ledger(
    State(state): State<ApiContext>,
    AxPath(user_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<AdminWalletLedgerQuery>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    match state
        .infra
        .list_wallet_ledger(user_id, query.limit.unwrap_or(100))
        .await
    {
        Ok(entries) => Json(entries).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to query wallet ledger: {err}"),
        ),
    }
}

async fn admin_list_user_subscriptions(
    State(state): State<ApiContext>,
    AxPath(user_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<AdminUserSubscriptionsQuery>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    match state
        .infra
        .list_user_subscriptions(user_id, query.limit.unwrap_or(50))
        .await
    {
        Ok(subscriptions) => Json(subscriptions).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to query user subscriptions: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct AdminGrantSubscriptionRequest {
    plan_id: Uuid,
    duration_days: Option<i32>,
}

async fn admin_grant_user_subscription(
    State(state): State<ApiContext>,
    AxPath(user_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<AdminGrantSubscriptionRequest>,
) -> Response {
    let actor = match require_super_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    match state
        .infra
        .admin_grant_subscription(user_id, payload.plan_id, payload.duration_days)
        .await
    {
        Ok(Some(subscription)) => {
            let _ = state
                .infra
                .log_audit_event(
                    parse_user_uuid(&actor),
                    "admin.billing.subscription.grant",
                    "user",
                    Some(&user_id.to_string()),
                    json!({
                        "subscription_id": subscription.id,
                        "plan_id": payload.plan_id,
                        "duration_days": payload.duration_days,
                    }),
                )
                .await;

            Json(subscription).into_response()
        }
        Ok(None) => error_response(StatusCode::NOT_FOUND, "user not found"),
        Err(err) => {
            if let Some(resp) = map_billing_error(&err) {
                return resp;
            }
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to grant subscription: {err}"),
            )
        }
    }
}

#[derive(Debug, Deserialize)]
struct AdminUpdateSubscriptionRequest {
    expires_at: Option<DateTime<Utc>>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AdminSubscriptionPath {
    user_id: Uuid,
    sub_id: Uuid,
}

async fn admin_update_user_subscription(
    State(state): State<ApiContext>,
    AxPath(path): AxPath<AdminSubscriptionPath>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<AdminUpdateSubscriptionRequest>,
) -> Response {
    let actor = match require_super_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    match state
        .infra
        .admin_update_subscription(
            path.user_id,
            path.sub_id,
            payload.expires_at,
            payload.status.as_deref(),
        )
        .await
    {
        Ok(Some(subscription)) => {
            let _ = state
                .infra
                .log_audit_event(
                    parse_user_uuid(&actor),
                    "admin.billing.subscription.update",
                    "subscription",
                    Some(&path.sub_id.to_string()),
                    json!({
                        "user_id": path.user_id,
                        "expires_at": payload.expires_at,
                        "status": payload.status,
                    }),
                )
                .await;

            Json(subscription).into_response()
        }
        Ok(None) => error_response(StatusCode::NOT_FOUND, "subscription not found"),
        Err(err) => {
            if let Some(resp) = map_billing_error(&err) {
                return resp;
            }
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to update subscription: {err}"),
            )
        }
    }
}

async fn admin_cancel_user_subscription(
    State(state): State<ApiContext>,
    AxPath(path): AxPath<AdminSubscriptionPath>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let actor = match require_super_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    match state
        .infra
        .admin_cancel_subscription(path.user_id, path.sub_id)
        .await
    {
        Ok(Some(subscription)) => {
            let _ = state
                .infra
                .log_audit_event(
                    parse_user_uuid(&actor),
                    "admin.billing.subscription.cancel",
                    "subscription",
                    Some(&path.sub_id.to_string()),
                    json!({
                        "user_id": path.user_id,
                    }),
                )
                .await;

            Json(subscription).into_response()
        }
        Ok(None) => error_response(StatusCode::NOT_FOUND, "subscription not found"),
        Err(err) => {
            if let Some(resp) = map_billing_error(&err) {
                return resp;
            }
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to cancel subscription: {err}"),
            )
        }
    }
}

async fn admin_adjust_user_wallet_balance(
    State(state): State<ApiContext>,
    AxPath(user_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<AdminAdjustWalletBalanceRequest>,
) -> Response {
    let actor = match require_super_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    match state
        .infra
        .admin_adjust_wallet_balance(user_id, payload.amount, payload.note.as_deref())
        .await
    {
        Ok(Some(wallet)) => {
            let _ = state
                .infra
                .log_audit_event(
                    parse_user_uuid(&actor),
                    "admin.billing.wallet.adjust",
                    "user",
                    Some(&user_id.to_string()),
                    json!({
                        "amount": payload.amount,
                        "note": payload.note,
                    }),
                )
                .await;

            Json(wallet).into_response()
        }
        Ok(None) => error_response(StatusCode::NOT_FOUND, "user not found"),
        Err(err) => {
            if let Some(resp) = map_billing_error(&err) {
                return resp;
            }
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to adjust wallet balance: {err}"),
            )
        }
    }
}

#[derive(Debug, Serialize)]
struct AdminBillingConfigResponse {
    enabled: bool,
    min_recharge_amount: Decimal,
    max_recharge_amount: Decimal,
    order_expire_minutes: i64,
    channels: Vec<String>,
    epay: AdminEpayConfigResponse,
}

#[derive(Debug, Serialize)]
struct AdminEpayConfigResponse {
    gateway_url: String,
    pid: String,
    key: String,
    notify_url: String,
    return_url: String,
    sitename: String,
}
