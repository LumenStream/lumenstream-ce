fn register_commercial_routes(
    mut scope: actix_web::Scope,
    capabilities: &ls_config::EditionCapabilities,
) -> actix_web::Scope {
    if capabilities.billing_enabled {
        scope = register_billing_routes(scope);
    }

    if capabilities.advanced_traffic_controls_enabled {
        scope = register_advanced_traffic_routes(scope);
    }

    if capabilities.audit_log_export_enabled {
        scope = register_audit_export_routes(scope);
    }

    if capabilities.invite_rewards_enabled {
        scope = register_invite_reward_routes(scope);
    }

    scope
}

fn register_billing_routes(scope: actix_web::Scope) -> actix_web::Scope {
    scope
        .route("/billing/wallet", web::get().to(billing_get_wallet))
        .route("/billing/plans", web::get().to(billing_list_plans))
        .route(
            "/billing/recharge/orders",
            web::post().to(billing_create_recharge_order),
        )
        .route(
            "/billing/recharge/orders/{order_id}",
            web::get().to(billing_get_recharge_order),
        )
        .route(
            "/billing/recharge/orders/{order_id}/ws",
            web::get().to(billing_recharge_order_ws),
        )
        .route(
            "/billing/plans/{plan_id}/purchase",
            web::post().to(billing_purchase_plan),
        )
        .route("/billing/epay/notify", web::post().to(billing_epay_notify))
        .route("/billing/epay/return", web::get().to(billing_epay_return))
        .route(
            "/admin/billing/permission-groups",
            web::get().to(admin_list_billing_permission_groups),
        )
        .route(
            "/admin/billing/permission-groups",
            web::post().to(admin_upsert_billing_permission_group),
        )
        .route(
            "/admin/billing/plans",
            web::get().to(admin_list_billing_plans),
        )
        .route(
            "/admin/billing/plans",
            web::post().to(admin_upsert_billing_plan),
        )
        .route(
            "/admin/billing/recharge-orders",
            web::get().to(admin_list_billing_recharge_orders),
        )
        .route(
            "/admin/billing/users/{user_id}/wallet",
            web::get().to(admin_get_user_wallet),
        )
        .route(
            "/admin/billing/users/{user_id}/ledger",
            web::get().to(admin_list_user_wallet_ledger),
        )
        .route(
            "/admin/billing/users/{user_id}/subscriptions",
            web::get().to(admin_list_user_subscriptions),
        )
        .route(
            "/admin/billing/users/{user_id}/subscriptions",
            web::post().to(admin_grant_user_subscription),
        )
        .route(
            "/admin/billing/users/{user_id}/subscriptions/{sub_id}",
            web::patch().to(admin_update_user_subscription),
        )
        .route(
            "/admin/billing/users/{user_id}/subscriptions/{sub_id}",
            web::delete().to(admin_cancel_user_subscription),
        )
        .route(
            "/admin/billing/users/{user_id}/adjust-balance",
            web::post().to(admin_adjust_user_wallet_balance),
        )
        .route(
            "/admin/billing/config",
            web::get().to(admin_get_billing_config),
        )
        .route(
            "/admin/billing/config",
            web::patch().to(admin_update_billing_config),
        )
}

fn register_advanced_traffic_routes(scope: actix_web::Scope) -> actix_web::Scope {
    scope
        .route("/api/traffic/me/items", web::get().to(get_my_traffic_usage_media))
        .route(
            "/admin/users/{user_id}/stream-policy",
            web::get().to(admin_get_user_stream_policy),
        )
        .route(
            "/admin/users/{user_id}/stream-policy",
            web::post().to(admin_upsert_user_stream_policy),
        )
        .route(
            "/admin/users/{user_id}/traffic-usage",
            web::get().to(admin_get_user_traffic_usage),
        )
        .route(
            "/admin/users/{user_id}/traffic-usage/reset",
            web::post().to(admin_reset_user_traffic_usage),
        )
        .route(
            "/admin/users/traffic-usage/top",
            web::get().to(admin_list_top_traffic_usage),
        )
}

fn register_audit_export_routes(scope: actix_web::Scope) -> actix_web::Scope {
    scope.route(
        "/admin/audit-logs/export",
        web::get().to(admin_export_audit_logs_csv),
    )
}

fn register_invite_reward_routes(scope: actix_web::Scope) -> actix_web::Scope {
    scope.route(
        "/admin/invite/rebates",
        web::get().to(admin_list_invite_rebates),
    )
}
