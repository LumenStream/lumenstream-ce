fn resolve_scan_scope_path(
    root_paths: &[String],
    path_prefix: Option<&str>,
) -> anyhow::Result<Option<PathBuf>> {
    let Some(raw_scope) = path_prefix.map(str::trim).filter(|v| !v.is_empty()) else {
        return Ok(None);
    };

    let roots = root_paths
        .iter()
        .map(|root| root.trim())
        .filter(|root| !root.is_empty())
        .map(|root| {
            std::fs::canonicalize(root)
                .with_context(|| format!("failed to canonicalize library root: {root}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if roots.is_empty() {
        anyhow::bail!("scan scope requires at least one library path");
    }

    let scope_candidate = if Path::new(raw_scope).is_absolute() {
        PathBuf::from(raw_scope)
    } else {
        if roots.len() != 1 {
            anyhow::bail!(
                "relative scan scope is ambiguous for libraries with multiple roots: {}",
                raw_scope
            );
        }
        roots[0].join(raw_scope)
    };

    let scope = std::fs::canonicalize(&scope_candidate).with_context(|| {
        format!(
            "scan scope path does not exist: {}",
            scope_candidate.display()
        )
    })?;

    if !roots.iter().any(|root| scope.starts_with(root)) {
        anyhow::bail!(
            "scan scope path is outside library root: {}",
            scope.display()
        );
    }

    Ok(Some(scope))
}

fn calc_retry_delay_seconds(base_seconds: i64, max_seconds: i64, attempts: i32) -> i64 {
    let base = base_seconds.max(1);
    let max = max_seconds.max(base);
    let exponent = attempts.max(0) as u32;
    base.saturating_mul(2_i64.saturating_pow(exponent)).min(max)
}

#[derive(Debug, Clone)]
struct DefaultTaskDefinitionSeed {
    task_key: &'static str,
    display_name: &'static str,
    enabled: bool,
    cron_expr: &'static str,
    default_payload: Value,
    max_attempts: i32,
}

fn default_task_definitions() -> Vec<DefaultTaskDefinitionSeed> {
    vec![
        DefaultTaskDefinitionSeed {
            task_key: "cleanup_maintenance",
            display_name: "系统清理维护",
            enabled: true,
            cron_expr: "0 0 * * * *",
            default_payload: json!({}),
            max_attempts: 1,
        },
        DefaultTaskDefinitionSeed {
            task_key: "retry_dispatch",
            display_name: "失败任务重试分发",
            enabled: true,
            cron_expr: "0 * * * * *",
            default_payload: json!({ "limit": 100 }),
            max_attempts: 1,
        },
        DefaultTaskDefinitionSeed {
            task_key: "billing_expire",
            display_name: "计费过期处理",
            enabled: true,
            cron_expr: "0 */5 * * * *",
            default_payload: json!({}),
            max_attempts: 1,
        },
        DefaultTaskDefinitionSeed {
            task_key: "scan_library",
            display_name: "媒体库扫描",
            enabled: false,
            cron_expr: "0 */3 * * * *",
            default_payload: json!({ "mode": "incremental" }),
            max_attempts: 3,
        },
        DefaultTaskDefinitionSeed {
            task_key: "metadata_repair",
            display_name: "元数据修复",
            enabled: false,
            cron_expr: "0 30 3 * * *",
            default_payload: json!({}),
            max_attempts: 3,
        },
        DefaultTaskDefinitionSeed {
            task_key: "subtitle_sync",
            display_name: "字幕同步",
            enabled: false,
            cron_expr: "0 45 3 * * *",
            default_payload: json!({ "mode": "incremental" }),
            max_attempts: 3,
        },
        DefaultTaskDefinitionSeed {
            task_key: "scraper_fill",
            display_name: "刮削补齐",
            enabled: false,
            cron_expr: "0 15 4 * * *",
            default_payload: json!({}),
            max_attempts: 3,
        },
        DefaultTaskDefinitionSeed {
            task_key: "search_reindex",
            display_name: "搜索索引重建",
            enabled: false,
            cron_expr: "0 0 2 * * *",
            default_payload: json!({ "batch_size": 500 }),
            max_attempts: 3,
        },
        DefaultTaskDefinitionSeed {
            task_key: "agent_missing_scan",
            display_name: "缺集/漏季 Agent 扫描",
            enabled: false,
            cron_expr: "0 */30 * * * *",
            default_payload: json!({}),
            max_attempts: 1,
        },
    ]
}

fn merge_task_payload(
    default_payload: &Value,
    override_payload: Option<&Value>,
) -> anyhow::Result<Value> {
    if !default_payload.is_object() {
        anyhow::bail!("task default payload must be a JSON object");
    }

    let mut merged = default_payload.clone();
    if let Some(override_payload) = override_payload {
        if !override_payload.is_object() {
            anyhow::bail!("payload override must be a JSON object");
        }
        if let (Some(target), Some(source)) = (merged.as_object_mut(), override_payload.as_object())
        {
            for (key, value) in source {
                target.insert(key.clone(), value.clone());
            }
        }
    }
    Ok(merged)
}

fn push_unique_target(targets: &mut Vec<String>, candidate: String) {
    if candidate.trim().is_empty() {
        return;
    }

    if !targets.iter().any(|existing| existing == &candidate) {
        targets.push(candidate);
    }
}

fn dedup_preserve_order(targets: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for target in targets {
        push_unique_target(&mut out, target);
    }
    out
}

fn is_special_stream_scheme(stream_url: &str) -> bool {
    stream_url.starts_with("gdrive://")
        || stream_url.starts_with("s3://")
        || stream_url.starts_with("lumenbackend://")
}

fn normalize_lumenbackend_node(raw: &str) -> Option<String> {
    normalize_lumenbackend_base_url(raw)
}

fn normalize_lumenbackend_base_url(raw: &str) -> Option<String> {
    let cleaned = raw.trim().trim_end_matches('/');
    if cleaned.is_empty() {
        return None;
    }

    let lower = cleaned.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return Some(cleaned.to_string());
    }
    if cleaned.starts_with("//") {
        return Some(format!("https:{cleaned}"));
    }

    // Default to HTTPS for domain-style inputs; allow explicit `http://` for local/dev use.
    let authority = cleaned.split('/').next().unwrap_or_default().trim();
    let host = if let Some(stripped) = authority.strip_prefix('[') {
        // Bracketed IPv6, optionally with port: `[::1]:8080`
        stripped.split(']').next().unwrap_or(stripped)
    } else if authority.matches(':').count() > 1 {
        // Unbracketed IPv6 without scheme (e.g. `::1`). No port support here.
        authority
    } else {
        authority.split(':').next().unwrap_or(authority)
    };

    let scheme = if host.eq_ignore_ascii_case("localhost")
        || host.starts_with("127.")
        || host.starts_with("0.0.0.0")
        || host.eq_ignore_ascii_case("::1")
    {
        "http"
    } else {
        "https"
    };

    Some(format!("{scheme}://{cleaned}"))
}

fn effective_lumenbackend_route(raw: &str) -> &str {
    let route = raw.trim();
    if route.is_empty() { "gdrive" } else { route }
}

fn normalize_lumenbackend_route(raw: &str) -> String {
    effective_lumenbackend_route(raw)
        .trim_matches('/')
        .to_ascii_lowercase()
}

fn is_known_lumenbackend_route(route: &str) -> bool {
    matches!(route, "gdrive" | "cdn" | "s3")
}

fn parse_lumenbackend_reference(raw: &str, default_route: &str) -> (String, String) {
    let cleaned = raw.trim();
    if cleaned.is_empty() {
        return (normalize_lumenbackend_route(default_route), String::new());
    }

    let mut parts = cleaned.splitn(2, '/');
    let first = parts.next().unwrap_or_default();
    let rest = parts.next().unwrap_or_default();

    if rest.is_empty() {
        (
            normalize_lumenbackend_route(default_route),
            first.trim().to_string(),
        )
    } else {
        let route = normalize_lumenbackend_route(first);
        if is_known_lumenbackend_route(route.as_str()) {
            (route, rest.trim().to_string())
        } else {
            (normalize_lumenbackend_route(default_route), cleaned.to_string())
        }
    }
}

fn is_supported_lumenbackend_http_route(route: &str) -> bool {
    if is_known_lumenbackend_route(route) {
        return true;
    }
    if let Some(remain) = route.strip_prefix("v1/streams/") {
        return is_known_lumenbackend_route(remain);
    }
    false
}

fn decode_lumenbackend_query_value(raw: &str) -> Option<String> {
    let value = raw.trim();
    if value.is_empty() {
        return None;
    }

    // Some clients encode spaces as `+` in query string.
    let value = value.replace('+', " ");
    let decoded = urlencoding::decode(value.as_str()).ok()?;
    let decoded = decoded.trim();
    if decoded.is_empty() {
        return None;
    }
    Some(decoded.to_string())
}

fn extract_query_param(query: &str, key: &str) -> Option<String> {
    for pair in query.split('&') {
        if pair.trim().is_empty() {
            continue;
        }
        let mut parts = pair.splitn(2, '=');
        let k = parts.next().unwrap_or_default();
        let v = parts.next().unwrap_or_default();
        if k.eq_ignore_ascii_case(key) {
            return decode_lumenbackend_query_value(v);
        }
    }
    None
}

fn parse_lumenbackend_http_stream_url(raw_url: &str) -> Option<(String, String)> {
    let trimmed = raw_url.trim();
    let rest = trimmed
        .strip_prefix("http://")
        .or_else(|| trimmed.strip_prefix("https://"))?;
    let rest = rest.split('#').next().unwrap_or(rest);
    let (host_and_path, query) = rest.split_once('?').unwrap_or((rest, ""));

    let path_start = host_and_path.find('/')?;
    let route_raw = host_and_path[path_start..].trim_matches('/').trim();
    if route_raw.is_empty() {
        return None;
    }
    let route = route_raw.to_ascii_lowercase();
    if route.is_empty() || !is_supported_lumenbackend_http_route(route.as_str()) {
        return None;
    }

    let path = extract_query_param(query, "path")?;
    Some((route, path))
}

fn distributed_offset(key: &str, len: usize) -> usize {
    if len <= 1 {
        return 0;
    }

    let hash = auth::hash_api_key(key.trim());
    let prefix = hash.get(..8).unwrap_or(hash.as_str());
    usize::from_str_radix(prefix, 16).unwrap_or(0) % len
}

fn normalize_traffic_window_days(raw_days: i32) -> i32 {
    raw_days.max(1)
}

fn normalize_default_optional_i32(raw: i32) -> Option<i32> {
    if raw < 0 { None } else { Some(raw) }
}

fn normalize_default_optional_i64(raw: i64) -> Option<i64> {
    if raw < 0 { None } else { Some(raw) }
}

fn build_lumenbackend_stream_token(
    signing_key: &str,
    route: &str,
    path: &str,
    ttl_seconds: u64,
    now: DateTime<Utc>,
) -> Option<String> {
    let key = signing_key.trim();
    let normalized_route = normalize_lumenbackend_route(route);
    let normalized_path = path.trim();

    if key.is_empty() || normalized_route.is_empty() || normalized_path.is_empty() {
        return None;
    }

    let exp = now
        .timestamp()
        .saturating_add(i64::try_from(ttl_seconds.max(1)).unwrap_or(i64::MAX));
    let payload = format!("{normalized_route}\n{normalized_path}\n{exp}");

    let mut mac = Hmac::<Sha256>::new_from_slice(key.as_bytes()).ok()?;
    mac.update(payload.as_bytes());
    let signature = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());

    Some(format!("{exp}.{signature}"))
}

fn build_meili_filter(options: &ItemsQuery) -> Option<String> {
    let mut clauses = Vec::new();

    if !options.include_item_types.is_empty() {
        let values = options
            .include_item_types
            .iter()
            .map(|v| meili_quote(v))
            .collect::<Vec<_>>()
            .join(", ");
        clauses.push(format!("item_type IN [{values}]"));
    }

    if let Some(series_id) = options.series_filter {
        clauses.push(format!(
            "series_id = {}",
            meili_quote(&series_id.to_string())
        ));
    }

    if let Some(parent_id) = options.parent_id {
        let quoted = meili_quote(&parent_id.to_string());
        clauses.push(format!("(library_id = {quoted} OR series_id = {quoted})"));
    }

    if clauses.is_empty() {
        None
    } else {
        Some(clauses.join(" AND "))
    }
}

fn meili_quote(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

fn playlist_is_visible_to(viewer_user_id: Uuid, owner_user_id: Uuid, is_public: bool) -> bool {
    viewer_user_id == owner_user_id || is_public
}

fn is_unique_violation(err: &sqlx::Error) -> bool {
    matches!(
        err,
        sqlx::Error::Database(db_err) if db_err.code().as_deref() == Some("23505")
    )
}

fn tmdb_cache_key(item_kind: &str, item_name: &str, language: &str) -> String {
    let normalized_name = item_name.trim().to_lowercase();
    let normalized_language = language.trim().to_lowercase();
    format!("{item_kind}:{normalized_language}:{normalized_name}")
}
