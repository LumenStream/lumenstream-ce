async fn require_auth(
    state: &ApiContext,
    headers: &HeaderMap,
    uri: &Uri,
) -> Result<UserDto, Response> {
    let tokens = extract_tokens(headers, uri);
    if tokens.is_empty() {
        return Err(error_response(StatusCode::UNAUTHORIZED, "missing access token"));
    }

    let mut saw_invalid_token = false;
    for token in &tokens {
        match state.infra.resolve_user_from_token(token).await {
            Ok(Some(user)) => return Ok(user),
            Ok(None) => {
                saw_invalid_token = true;
            }
            Err(err) => {
                return Err(error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("token verification failed: {err}"),
                ));
            }
        }
    }

    if saw_invalid_token {
        state.metrics.auth_failures.fetch_add(1, Ordering::Relaxed);
        Err(error_response(
            StatusCode::UNAUTHORIZED,
            "invalid or expired token",
        ))
    } else {
        Err(error_response(
            StatusCode::UNAUTHORIZED,
            "missing access token",
        ))
    }
}

async fn require_admin(
    state: &ApiContext,
    headers: &HeaderMap,
    uri: &Uri,
) -> Result<UserDto, Response> {
    let user = require_auth(state, headers, uri).await?;

    if !is_super_admin(&user) {
        return Err(error_response(StatusCode::FORBIDDEN, "admin role required"));
    }

    if !state.infra.config_snapshot().security.admin_allow_ips.is_empty() {
        let ip = extract_client_ip(headers, None, &state.infra.config_snapshot().security);
        let allowed = ip
            .as_ref()
            .map(|candidate| {
                is_ip_allowed(
                    candidate,
                    state.infra.config_snapshot().security.admin_allow_ips.as_slice(),
                )
            })
            .unwrap_or(false);

        if !allowed {
            return Err(error_response(
                StatusCode::FORBIDDEN,
                "admin access is not allowed from this ip",
            ));
        }
    }

    Ok(user)
}

async fn require_super_admin(
    state: &ApiContext,
    headers: &HeaderMap,
    uri: &Uri,
) -> Result<UserDto, Response> {
    let user = require_admin(state, headers, uri).await?;
    if !is_super_admin(&user) {
        return Err(error_response(StatusCode::FORBIDDEN, "admin role required"));
    }
    Ok(user)
}

async fn require_lumenbackend_api_key(
    state: &ApiContext,
    headers: &HeaderMap,
    uri: &Uri,
) -> Result<(), Response> {
    let token = extract_token(headers, uri)
        .ok_or_else(|| error_response(StatusCode::UNAUTHORIZED, "missing api key"))?;
    match state.infra.verify_admin_api_key(token.as_str()).await {
        Ok(true) => Ok(()),
        Ok(false) => Err(error_response(StatusCode::UNAUTHORIZED, "invalid api key")),
        Err(err) => Err(error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("api key verification failed: {err}"),
        )),
    }
}
