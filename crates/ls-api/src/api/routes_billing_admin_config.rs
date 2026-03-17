impl AdminBillingConfigResponse {
    fn from_config(cfg: &BillingConfig, mask_key: bool) -> Self {
        Self {
            enabled: cfg.enabled,
            min_recharge_amount: cfg.min_recharge_amount,
            max_recharge_amount: cfg.max_recharge_amount,
            order_expire_minutes: cfg.order_expire_minutes,
            channels: cfg.channels.clone(),
            epay: AdminEpayConfigResponse {
                gateway_url: cfg.epay.gateway_url.clone(),
                pid: cfg.epay.pid.clone(),
                key: if mask_key && !cfg.epay.key.is_empty() {
                    "***".to_string()
                } else {
                    cfg.epay.key.clone()
                },
                notify_url: cfg.epay.notify_url.clone(),
                return_url: cfg.epay.return_url.clone(),
                sitename: cfg.epay.sitename.clone(),
            },
        }
    }
}

#[derive(Debug, Deserialize)]
struct AdminUpdateBillingConfigRequest {
    enabled: Option<bool>,
    min_recharge_amount: Option<Decimal>,
    max_recharge_amount: Option<Decimal>,
    order_expire_minutes: Option<i64>,
    channels: Option<Vec<String>>,
    epay: Option<AdminUpdateEpayConfigRequest>,
}

#[derive(Debug, Deserialize)]
struct AdminUpdateEpayConfigRequest {
    gateway_url: Option<String>,
    pid: Option<String>,
    key: Option<String>,
    notify_url: Option<String>,
    return_url: Option<String>,
    sitename: Option<String>,
}

async fn admin_get_billing_config(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    match state.infra.get_web_settings().await {
        Ok(settings) => Json(AdminBillingConfigResponse::from_config(
            &settings.billing,
            true,
        ))
        .into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to load billing config: {err}"),
        ),
    }
}

async fn admin_update_billing_config(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<AdminUpdateBillingConfigRequest>,
) -> Response {
    let actor = match require_super_admin(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let mut settings = match state.infra.get_web_settings().await {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to load current settings: {err}"),
            );
        }
    };

    apply_billing_config_update(&mut settings.billing, &payload);

    if settings.billing.min_recharge_amount < Decimal::ZERO {
        settings.billing.min_recharge_amount = Decimal::ZERO;
    }
    if settings.billing.max_recharge_amount < settings.billing.min_recharge_amount {
        settings.billing.max_recharge_amount = settings.billing.min_recharge_amount;
    }
    if settings.billing.order_expire_minutes < 1 {
        settings.billing.order_expire_minutes = 1;
    }

    match state.infra.upsert_web_settings(&settings).await {
        Ok(()) => {
            let _ = state
                .infra
                .log_audit_event(
                    parse_user_uuid(&actor),
                    "admin.billing.config.update",
                    "billing_config",
                    Some("global"),
                    json!({
                        "enabled": settings.billing.enabled,
                        "min_recharge_amount": settings.billing.min_recharge_amount,
                        "max_recharge_amount": settings.billing.max_recharge_amount,
                        "order_expire_minutes": settings.billing.order_expire_minutes,
                        "channels": settings.billing.channels,
                    }),
                )
                .await;

            Json(AdminBillingConfigResponse::from_config(
                &settings.billing,
                true,
            ))
            .into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to persist billing config: {err}"),
        ),
    }
}

fn apply_billing_config_update(cfg: &mut BillingConfig, payload: &AdminUpdateBillingConfigRequest) {
    if let Some(v) = payload.enabled {
        cfg.enabled = v;
    }
    if let Some(v) = payload.min_recharge_amount {
        cfg.min_recharge_amount = v;
    }
    if let Some(v) = payload.max_recharge_amount {
        cfg.max_recharge_amount = v;
    }
    if let Some(v) = payload.order_expire_minutes {
        cfg.order_expire_minutes = v;
    }
    if let Some(ref v) = payload.channels {
        cfg.channels = v.clone();
    }
    if let Some(ref epay) = payload.epay {
        apply_epay_config_update(&mut cfg.epay, epay);
    }
}

fn apply_epay_config_update(cfg: &mut EpayConfig, payload: &AdminUpdateEpayConfigRequest) {
    if let Some(ref v) = payload.gateway_url {
        cfg.gateway_url = v.clone();
    }
    if let Some(ref v) = payload.pid {
        cfg.pid = v.clone();
    }
    if let Some(ref v) = payload.key {
        if v != "***" {
            cfg.key = v.clone();
        }
    }
    if let Some(ref v) = payload.notify_url {
        cfg.notify_url = v.clone();
    }
    if let Some(ref v) = payload.return_url {
        cfg.return_url = v.clone();
    }
    if let Some(ref v) = payload.sitename {
        cfg.sitename = v.clone();
    }
}
