fn extract_token(headers: &HeaderMap, uri: &Uri) -> Option<String> {
    extract_tokens(headers, uri).into_iter().next()
}

fn extract_tokens(headers: &HeaderMap, uri: &Uri) -> Vec<String> {
    let mut tokens = Vec::new();

    if let Some(v) = headers.get("X-Emby-Token").and_then(|v| v.to_str().ok()) {
        push_token_candidate(&mut tokens, v);
    }

    if let Some(v) = headers.get("X-Api-Key").and_then(|v| v.to_str().ok()) {
        push_token_candidate(&mut tokens, v);
    }

    if let Some(v) = headers
        .get("X-MediaBrowser-Token")
        .and_then(|v| v.to_str().ok())
    {
        push_token_candidate(&mut tokens, v);
    }

    if let Some(v) = headers
        .get("X-Emby-Authorization")
        .and_then(|v| v.to_str().ok())
        && let Some(token) = extract_emby_authorization_token(v)
    {
        push_token_candidate(&mut tokens, &token);
    }

    if let Some(v) = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    {
        let mut parts = v.split_whitespace();
        if parts
            .next()
            .is_some_and(|scheme| scheme.eq_ignore_ascii_case("Bearer"))
            && let Some(token) = parts.next()
        {
            push_token_candidate(&mut tokens, token);
        }
        if let Some(token) = extract_emby_authorization_token(v) {
            push_token_candidate(&mut tokens, &token);
        }
    }

    if let Some(query) = uri.query() {
        for pair in query.split('&') {
            let mut parts = pair.splitn(2, '=');
            let key = parts.next().unwrap_or_default();
            let value = parts.next().unwrap_or_default();
            if key.eq_ignore_ascii_case("api_key")
                || key.eq_ignore_ascii_case("x-emby-token")
                || key.eq_ignore_ascii_case("token")
            {
                let decoded = urlencoding::decode(value)
                    .map(|v| v.into_owned())
                    .unwrap_or_else(|_| value.to_string());
                push_token_candidate(&mut tokens, &decoded);
            }
        }
    }

    tokens
}

fn push_token_candidate(tokens: &mut Vec<String>, raw: &str) {
    let Some(token) = normalize_token(raw) else {
        return;
    };
    if !tokens.iter().any(|existing| existing == &token) {
        tokens.push(token);
    }
}

fn normalize_token(raw: &str) -> Option<String> {
    let token = raw.trim().trim_matches('"').trim_matches('\'').trim();
    if token.is_empty() {
        return None;
    }
    Some(token.to_string())
}

fn extract_emby_authorization_token(raw: &str) -> Option<String> {
    extract_emby_authorization_param(raw, "Token")
}

fn extract_emby_authorization_param(raw: &str, key: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() || key.trim().is_empty() {
        return None;
    }

    for segment in raw.split(',') {
        let Some((left, right)) = segment.split_once('=') else {
            continue;
        };
        let candidate = left.trim().split_whitespace().last().unwrap_or_default();
        if !candidate.eq_ignore_ascii_case(key) {
            continue;
        }
        if let Some(value) = normalize_token(right) {
            return Some(value);
        }
    }
    None
}

fn extract_emby_authorization_param_from_headers(headers: &HeaderMap, key: &str) -> Option<String> {
    headers
        .get("X-Emby-Authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| extract_emby_authorization_param(value, key))
        .or_else(|| {
            headers
                .get(header::AUTHORIZATION)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| extract_emby_authorization_param(value, key))
        })
}

fn extract_client_ip(
    headers: &HeaderMap,
    peer_ip: Option<IpAddr>,
    security: &SecurityConfig,
) -> Option<String> {
    if peer_ip.is_none() {
        if let Some(ip) = header_ip(headers, INTERNAL_CLIENT_IP_HEADER) {
            return Some(ip.to_string());
        }
    }

    let peer_ip = peer_ip?;
    if security.trust_x_forwarded_for
        && ip_matches_allow_entries(peer_ip, &security.trusted_proxies)
    {
        if let Some(ip) = forwarded_client_ip(headers).or_else(|| header_ip(headers, "x-real-ip")) {
            return Some(ip.to_string());
        }
    }

    Some(peer_ip.to_string())
}

fn parse_peer_ip(peer_addr: Option<&str>) -> Option<IpAddr> {
    peer_addr.and_then(parse_ip)
}

fn parse_ip(raw: &str) -> Option<IpAddr> {
    let value = raw.trim();
    if value.is_empty() {
        return None;
    }

    if let Ok(ip) = value.parse::<IpAddr>() {
        return Some(ip);
    }

    if let Some(bracketed) = value.strip_prefix('[').and_then(|v| v.strip_suffix(']')) {
        if let Ok(ip) = bracketed.parse::<IpAddr>() {
            return Some(ip);
        }
    }

    value
        .parse::<std::net::SocketAddr>()
        .ok()
        .map(|addr| addr.ip())
}

fn header_ip(headers: &HeaderMap, name: &str) -> Option<IpAddr> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .and_then(parse_ip)
}

fn forwarded_client_ip(headers: &HeaderMap) -> Option<IpAddr> {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|raw| raw.split(',').next())
        .and_then(parse_ip)
}

fn is_ip_allowed(candidate: &str, allow_entries: &[String]) -> bool {
    parse_ip(candidate)
        .map(|ip| ip_matches_allow_entries(ip, allow_entries))
        .unwrap_or(false)
}

fn ip_matches_allow_entries(candidate: IpAddr, allow_entries: &[String]) -> bool {
    allow_entries
        .iter()
        .any(|entry| ip_matches_allow_entry(candidate, entry))
}

fn ip_matches_allow_entry(candidate: IpAddr, entry: &str) -> bool {
    let entry = entry.trim();
    if entry.is_empty() {
        return false;
    }

    if let Ok(ip) = entry.parse::<IpAddr>() {
        return ip == candidate;
    }

    let Some((network, prefix_len)) = parse_cidr(entry) else {
        return false;
    };

    match (candidate, network) {
        (IpAddr::V4(candidate), IpAddr::V4(network)) => {
            if prefix_len > 32 {
                return false;
            }
            let mask = if prefix_len == 0 {
                0
            } else {
                u32::MAX << (32 - prefix_len)
            };
            (u32::from(candidate) & mask) == (u32::from(network) & mask)
        }
        (IpAddr::V6(candidate), IpAddr::V6(network)) => {
            if prefix_len > 128 {
                return false;
            }
            let mask = if prefix_len == 0 {
                0
            } else {
                u128::MAX << (128 - prefix_len)
            };
            (u128::from(candidate) & mask) == (u128::from(network) & mask)
        }
        _ => false,
    }
}

fn parse_cidr(value: &str) -> Option<(IpAddr, u8)> {
    let (network, prefix) = value.split_once('/')?;
    let network = network.trim().parse::<IpAddr>().ok()?;
    let prefix = prefix.trim().parse::<u8>().ok()?;
    Some((network, prefix))
}

fn parse_user_role(role: Option<&str>, is_admin: Option<bool>) -> UserRole {
    if is_admin.unwrap_or(false) {
        return UserRole::Admin;
    }

    match role.unwrap_or_default().to_ascii_lowercase().as_str() {
        "admin" => UserRole::Admin,
        _ => UserRole::Viewer,
    }
}

fn parse_user_role_strict(role: &str) -> Option<UserRole> {
    match role.trim().to_ascii_lowercase().as_str() {
        "admin" => Some(UserRole::Admin),
        "viewer" => Some(UserRole::Viewer),
        _ => None,
    }
}

fn deserialize_nullable_string_patch<'de, D>(
    deserializer: D,
) -> Result<Option<Option<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer).map(Some)
}

fn role_from_user(user: &UserDto) -> UserRole {
    parse_user_role(
        user.policy.role.as_deref(),
        Some(user.policy.is_administrator),
    )
}
