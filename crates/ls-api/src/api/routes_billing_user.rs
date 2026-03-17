async fn billing_get_wallet(
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

    let wallet = match state.infra.ensure_wallet_account(user_id).await {
        Ok(v) => v,
        Err(err) => {
            if let Some(resp) = map_billing_error(&err) {
                return resp;
            }
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to load wallet: {err}"),
            );
        }
    };

    let active_subscription = match state.infra.get_active_subscription(user_id).await {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to load active subscription: {err}"),
            );
        }
    };

    let recent_ledger = match state.infra.list_wallet_ledger(user_id, 20).await {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to load wallet ledger: {err}"),
            );
        }
    };

    Json(json!({
        "wallet": wallet,
        "active_subscription": active_subscription,
        "recent_ledger": recent_ledger,
    }))
    .into_response()
}

async fn billing_list_plans(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }

    match state.infra.list_billing_plans(false).await {
        Ok(plans) => Json(plans).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to list billing plans: {err}"),
        ),
    }
}

async fn billing_create_recharge_order(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<BillingCreateRechargeOrderRequest>,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(user) => user,
        Err(resp) => return resp,
    };

    let user_id = match Uuid::parse_str(&auth_user.id) {
        Ok(v) => v,
        Err(_) => return error_response(StatusCode::UNAUTHORIZED, "invalid user id"),
    };

    let remote_ip = extract_client_ip(&headers, None, &state.infra.config_snapshot().security);

    match state
        .infra
        .create_recharge_order(
            user_id,
            payload.amount,
            payload.channel.as_deref(),
            payload.subject.as_deref(),
            remote_ip.as_deref(),
        )
        .await
    {
        Ok(order) => Json(order).into_response(),
        Err(err) => {
            if let Some(resp) = map_billing_error(&err) {
                return resp;
            }
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to create recharge order: {err}"),
            )
        }
    }
}

async fn billing_get_recharge_order(
    State(state): State<ApiContext>,
    AxPath(order_id): AxPath<Uuid>,
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

    match state
        .infra
        .get_recharge_order_for_user(user_id, order_id)
        .await
    {
        Ok(Some(order)) => Json(order).into_response(),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "recharge order not found"),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to query recharge order: {err}"),
        ),
    }
}

async fn billing_recharge_order_ws(
    State(state): State<ApiContext>,
    AxPath(order_id): AxPath<Uuid>,
    req: HttpRequest,
    body: web::Payload,
) -> Response {
    let headers = HeaderMap(req.headers().clone());
    let uri = Uri(req.uri().clone());
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(user) => user,
        Err(resp) => return resp,
    };

    let user_id = match Uuid::parse_str(&auth_user.id) {
        Ok(v) => v,
        Err(_) => return error_response(StatusCode::UNAUTHORIZED, "invalid user id"),
    };

    let order = match state
        .infra
        .get_recharge_order_for_user(user_id, order_id)
        .await
    {
        Ok(Some(order)) => order,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "recharge order not found"),
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to query recharge order: {err}"),
            );
        }
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
    let mut rx = state.infra.subscribe_recharge_orders();

    actix_web::rt::spawn(async move {
        let mut heartbeat = tokio::time::interval(std::time::Duration::from_secs(20));
        let snapshot_payload = json!({
            "event": "billing.recharge_order.snapshot",
            "order": order,
            "emitted_at": Utc::now().to_rfc3339(),
        });
        if let Ok(snapshot) = serde_json::to_string(&snapshot_payload) {
            if session.text(snapshot).await.is_err() {
                return;
            }
        }

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
                        Ok(event) => {
                            if event.order.id == order_id && event.order.user_id == user_id {
                                if let Ok(text) = serde_json::to_string(&event) {
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

async fn billing_purchase_plan(
    State(state): State<ApiContext>,
    AxPath(plan_id): AxPath<Uuid>,
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

    match state
        .infra
        .purchase_plan_with_balance(user_id, plan_id)
        .await
    {
        Ok(result) => Json(result).into_response(),
        Err(err) => {
            if let Some(resp) = map_billing_error(&err) {
                return resp;
            }
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to purchase billing plan: {err}"),
            )
        }
    }
}

async fn billing_epay_notify(
    State(state): State<ApiContext>,
    form: web::Form<HashMap<String, String>>,
) -> Response {
    let payload = form
        .into_inner()
        .into_iter()
        .collect::<BTreeMap<String, String>>();

    match state.infra.handle_epay_notify(&payload).await {
        Ok(order) => {
            if let Err(err) = state
                .infra
                .log_audit_event(
                    None,
                    "billing.recharge.notify",
                    "billing_recharge_order",
                    Some(&order.id.to_string()),
                    json!({
                        "user_id": order.user_id,
                        "amount": order.amount,
                        "status": order.status,
                    }),
                )
                .await
            {
                error!(error = %err, "failed to write audit log for billing notify");
            }

            HttpResponse::Ok()
                .insert_header((header::CONTENT_TYPE, "text/plain; charset=utf-8"))
                .body("success")
        }
        Err(err) => {
            warn!(error = %err, "failed to process billing epay notify");
            HttpResponse::BadRequest()
                .insert_header((header::CONTENT_TYPE, "text/plain; charset=utf-8"))
                .body("fail")
        }
    }
}
