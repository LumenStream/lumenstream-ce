fn edition_capabilities(state: &ApiContext) -> ls_config::EditionCapabilities {
    state.infra.config_snapshot().edition_capabilities()
}

fn mask_admin_user_summary_page_payload(
    capabilities: &ls_config::EditionCapabilities,
    mut payload: Value,
) -> Value {
    let Some(items) = payload.get_mut("items").and_then(Value::as_array_mut) else {
        return payload;
    };

    for item in items {
        let Some(obj) = item.as_object_mut() else {
            continue;
        };
        if !capabilities.billing_enabled {
            obj.insert("subscription_name".to_string(), Value::Null);
        }
        if !capabilities.advanced_traffic_controls_enabled {
            obj.insert("used_bytes".to_string(), json!(0));
        }
    }

    payload
}

fn mask_admin_user_manage_profile_payload(
    capabilities: &ls_config::EditionCapabilities,
    mut payload: Value,
) -> Value {
    let Some(obj) = payload.as_object_mut() else {
        return payload;
    };

    if !capabilities.advanced_traffic_controls_enabled {
        obj.remove("stream_policy");
        obj.remove("traffic_usage");
    }

    if !capabilities.billing_enabled {
        obj.remove("wallet");
        obj.remove("subscriptions");
    }

    payload
}

fn invite_rewards_enabled(capabilities: &ls_config::EditionCapabilities) -> bool {
    capabilities.invite_rewards_enabled
}

fn invite_summary_payload(rewards_enabled: bool, summary: &ls_infra::InviteSummary) -> Value {
    let mut payload = json!({
        "code": summary.code,
        "enabled": summary.enabled,
        "invited_count": summary.invited_count,
    });
    if rewards_enabled {
        payload["rebate_total"] = json!(summary.rebate_total);
        payload["invitee_bonus_enabled"] = json!(summary.invitee_bonus_enabled);
    }
    payload
}

fn invite_settings_payload(rewards_enabled: bool, settings: &InviteConfig) -> Value {
    let mut payload = json!({
        "force_on_register": settings.force_on_register,
    });
    if rewards_enabled {
        payload["invitee_bonus_enabled"] = json!(settings.invitee_bonus_enabled);
        payload["invitee_bonus_amount"] = json!(settings.invitee_bonus_amount);
        payload["inviter_rebate_enabled"] = json!(settings.inviter_rebate_enabled);
        payload["inviter_rebate_rate"] = json!(settings.inviter_rebate_rate);
    }
    payload
}
