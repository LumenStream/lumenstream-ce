fn map_items_query_error(err: &anyhow::Error) -> StatusCode {
    if err.chain().any(|cause| {
        cause
            .downcast_ref::<InfraError>()
            .is_some_and(|infra_err| matches!(infra_err, InfraError::SearchUnavailable))
    }) {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

fn map_stream_admission_error(err: &anyhow::Error) -> Option<Response> {
    let reason = err.chain().find_map(|cause| {
        cause
            .downcast_ref::<InfraError>()
            .and_then(|infra_err| match infra_err {
                InfraError::StreamAccessDenied { reason } => Some(*reason),
                _ => None,
            })
    })?;

    Some(stream_access_denied_response(reason))
}

fn map_billing_error(err: &anyhow::Error) -> Option<Response> {
    let infra_error = err
        .chain()
        .find_map(|cause| cause.downcast_ref::<InfraError>())?;

    let (status, message) = match infra_error {
        InfraError::BillingDisabled => (
            StatusCode::SERVICE_UNAVAILABLE,
            "billing service is disabled",
        ),
        InfraError::BillingPlanNotFound => (StatusCode::NOT_FOUND, "billing plan not found"),
        InfraError::BillingOrderNotFound => {
            (StatusCode::NOT_FOUND, "billing recharge order not found")
        }
        InfraError::BillingInvalidAmount => (StatusCode::BAD_REQUEST, "billing amount is invalid"),
        InfraError::BillingChannelUnsupported => {
            (StatusCode::BAD_REQUEST, "billing channel unsupported")
        }
        InfraError::BillingInsufficientBalance => (
            StatusCode::PAYMENT_REQUIRED,
            "wallet balance is insufficient",
        ),
        InfraError::BillingSignatureInvalid => {
            (StatusCode::BAD_REQUEST, "billing signature is invalid")
        }
        InfraError::BillingOrderAmountMismatch => (
            StatusCode::BAD_REQUEST,
            "billing recharge amount does not match order",
        ),
        _ => return None,
    };

    Some(error_response(status, message))
}

fn map_playlist_error(err: &anyhow::Error) -> Option<Response> {
    let infra_error = err
        .chain()
        .find_map(|cause| cause.downcast_ref::<InfraError>())?;

    let (status, message) = match infra_error {
        InfraError::PlaylistNotFound => (StatusCode::NOT_FOUND, "playlist not found"),
        InfraError::PlaylistAccessDenied => (StatusCode::FORBIDDEN, "playlist access denied"),
        InfraError::PlaylistInvalidInput => (StatusCode::BAD_REQUEST, "playlist input is invalid"),
        InfraError::PlaylistConflict => (StatusCode::CONFLICT, "playlist conflict"),
        InfraError::MediaItemNotFound => (StatusCode::NOT_FOUND, "media item not found"),
        _ => return None,
    };

    Some(error_response(status, message))
}

fn map_invite_error(err: &anyhow::Error) -> Option<Response> {
    let infra_error = err
        .chain()
        .find_map(|cause| cause.downcast_ref::<InfraError>())?;

    let (status, message) = match infra_error {
        InfraError::InviteCodeRequired => (StatusCode::BAD_REQUEST, "invite code is required"),
        InfraError::InviteCodeInvalid => (StatusCode::BAD_REQUEST, "invite code is invalid"),
        InfraError::InviteRelationExists => (
            StatusCode::CONFLICT,
            "invite relationship already exists",
        ),
        InfraError::UserAlreadyExists => (StatusCode::CONFLICT, "username already exists"),
        _ => return None,
    };

    Some(error_response(status, message))
}

fn map_task_center_error(err: &anyhow::Error) -> Option<Response> {
    let infra_error = err
        .chain()
        .find_map(|cause| cause.downcast_ref::<InfraError>())?;

    let (status, message) = match infra_error {
        InfraError::TaskRunAlreadyActive => {
            (StatusCode::CONFLICT, "task already has active run")
        }
        _ => return None,
    };

    Some(error_response(status, message))
}

fn stream_access_denied_response(reason: StreamAccessDeniedReason) -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(json!({
            "error": "stream access denied",
            "reason": reason.as_str(),
        })),
    )
        .into_response()
}

fn has_is_resumable_filter(raw: Option<&str>) -> bool {
    split_csv(raw)
        .iter()
        .any(|v| v.eq_ignore_ascii_case("IsResumable"))
}

fn percentile_from_sorted(samples: &[u64], percentile: f64) -> u64 {
    if samples.is_empty() {
        return 0;
    }

    let p = percentile.clamp(0.0, 1.0);
    let idx = ((samples.len() - 1) as f64 * p).round() as usize;
    samples[idx.min(samples.len() - 1)]
}

fn error_response(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({"error": message}))).into_response()
}
