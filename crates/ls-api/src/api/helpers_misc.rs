fn is_super_admin(user: &UserDto) -> bool {
    matches!(role_from_user(user), UserRole::Admin)
}

fn header_or_default(headers: &HeaderMap, name: &str, default_value: &str) -> String {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_string())
        .unwrap_or_else(|| default_value.to_string())
}

fn header_optional(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

#[derive(Debug, Clone)]
struct EmbyClientContext {
    client: String,
    device_name: String,
    device_id: String,
    application_version: Option<String>,
}

fn parse_native_client_from_user_agent(headers: &HeaderMap) -> Option<(String, Option<String>)> {
    let raw = header_optional(headers, "User-Agent")?;
    let product = raw.split_whitespace().next()?.trim();
    if product.is_empty() {
        return None;
    }

    let (client, version) = match product.split_once('/') {
        Some((name, ver)) => (name.trim(), Some(ver.trim())),
        None => (product, None),
    };

    if client.is_empty() {
        return None;
    }

    let lowered = client.to_ascii_lowercase();
    if matches!(
        lowered.as_str(),
        "mozilla"
            | "applewebkit"
            | "safari"
            | "chrome"
            | "chromium"
            | "firefox"
            | "edg"
            | "edge"
            | "opera"
            | "okhttp"
            | "cfnetwork"
            | "curl"
            | "wget"
    ) {
        return None;
    }

    let version = version
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    Some((client.to_string(), version))
}

fn decode_emby_query_component(raw: &str) -> String {
    let normalized = raw.replace('+', " ");
    urlencoding::decode(&normalized)
        .map(|value| value.into_owned())
        .unwrap_or(normalized)
}

fn query_optional(query_string: Option<&str>, name: &str) -> Option<String> {
    let query = query_string?;
    if query.trim().is_empty() {
        return None;
    }

    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (raw_key, raw_value) = pair.split_once('=').unwrap_or((pair, ""));
        let key = decode_emby_query_component(raw_key);
        if !key.eq_ignore_ascii_case(name) {
            continue;
        }

        let value = decode_emby_query_component(raw_value);
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        return Some(value.to_string());
    }

    None
}

fn resolve_emby_client_context_with_query(
    headers: &HeaderMap,
    query_string: Option<&str>,
) -> EmbyClientContext {
    let user_agent_client = parse_native_client_from_user_agent(headers);
    let user_agent_client_name = user_agent_client.as_ref().map(|(name, _)| name.clone());
    let user_agent_version = user_agent_client.and_then(|(_, version)| version);

    let client = header_optional(headers, "X-Emby-Client")
        .or_else(|| extract_emby_authorization_param_from_headers(headers, "Client"))
        .or_else(|| query_optional(query_string, "X-Emby-Client"))
        .or_else(|| user_agent_client_name.clone())
        .unwrap_or_else(|| "ls-client".to_string());
    let device_name = header_optional(headers, "X-Emby-Device-Name")
        .or_else(|| extract_emby_authorization_param_from_headers(headers, "Device"))
        .or_else(|| query_optional(query_string, "X-Emby-Device-Name"))
        .or_else(|| user_agent_client_name)
        .unwrap_or_else(|| "ls-device".to_string());
    let device_id = header_optional(headers, "X-Emby-Device-Id")
        .or_else(|| extract_emby_authorization_param_from_headers(headers, "DeviceId"))
        .or_else(|| query_optional(query_string, "X-Emby-Device-Id"))
        .unwrap_or_default();
    let application_version = header_optional(headers, "X-Emby-Client-Version")
        .or_else(|| extract_emby_authorization_param_from_headers(headers, "Version"))
        .or_else(|| query_optional(query_string, "X-Emby-Client-Version"))
        .or(user_agent_version);

    EmbyClientContext {
        client,
        device_name,
        device_id,
        application_version,
    }
}

fn resolve_emby_client_context(headers: &HeaderMap) -> EmbyClientContext {
    resolve_emby_client_context_with_query(headers, None)
}

fn format_emby_datetime(value: DateTime<Utc>) -> String {
    let ticks_fraction = value.timestamp_subsec_nanos() / 100;
    format!(
        "{}.{ticks_fraction:07}Z",
        value.format("%Y-%m-%dT%H:%M:%S")
    )
}

#[cfg(test)]
fn parse_range_start(range_header: Option<&str>) -> Option<u64> {
    let header = range_header?;
    if !header.starts_with("bytes=") {
        return None;
    }

    let first = header.trim_start_matches("bytes=").split(',').next()?;
    let mut parts = first.splitn(2, '-');
    parts.next()?.trim().parse::<u64>().ok()
}

#[cfg(test)]
fn parse_range(range_header: Option<&str>, total_size: u64) -> Option<(u64, u64)> {
    let header = range_header?;
    if !header.starts_with("bytes=") {
        return None;
    }

    let first = header.trim_start_matches("bytes=").split(',').next()?;
    let mut parts = first.splitn(2, '-');
    let start_raw = parts.next()?.trim();
    let end_raw = parts.next().unwrap_or_default().trim();

    if start_raw.is_empty() {
        return None;
    }

    let start = start_raw.parse::<u64>().ok()?;
    if start >= total_size {
        return None;
    }

    let end = if end_raw.is_empty() {
        total_size.saturating_sub(1)
    } else {
        end_raw
            .parse::<u64>()
            .ok()?
            .min(total_size.saturating_sub(1))
    };

    if end < start {
        return None;
    }

    Some((start, end))
}

#[cfg(test)]
#[allow(dead_code)]
async fn stream_local_path(
    path: &str,
    range_header: Option<&str>,
) -> anyhow::Result<(Response, u64)> {
    let path = if let Some(rest) = path.strip_prefix("file://") {
        rest
    } else {
        path
    };

    let metadata = tokio::fs::metadata(path).await?;
    if !metadata.is_file() {
        anyhow::bail!("local path is not file");
    }

    let total_size = metadata.len();
    let mut status = StatusCode::OK;
    let mut body = tokio::fs::read(path).await?;
    let mut content_range_header = None;

    if let Some((start, end)) = parse_range(range_header, total_size) {
        status = StatusCode::PARTIAL_CONTENT;
        let start_idx = start as usize;
        let end_idx = end as usize;
        body = body[start_idx..=end_idx].to_vec();
        content_range_header = Some(format!("bytes {}-{}/{}", start, end, total_size));
    }

    let body_len = body.len();
    let mut builder = HttpResponse::build(status);
    builder.insert_header((header::ACCEPT_RANGES, "bytes"));
    builder.insert_header((header::CONTENT_LENGTH, body_len.to_string()));
    if let Some(content_range) = content_range_header {
        if let Ok(value) = header::HeaderValue::from_str(&content_range) {
            builder.insert_header((header::CONTENT_RANGE, value));
        }
    }

    let bytes_served = u64::try_from(body_len).unwrap_or(u64::MAX);
    Ok((builder.body(body), bytes_served))
}

async fn proxy_http_stream(
    url: &str,
    range_header: Option<&str>,
) -> anyhow::Result<(Response, u64)> {
    let client = reqwest::Client::new();
    let mut request = client.get(url);
    if let Some(range) = range_header {
        request = request.header(reqwest::header::RANGE, range);
    }

    let upstream = request.send().await?;
    let status = upstream.status();

    if !(status.is_success() || status == reqwest::StatusCode::PARTIAL_CONTENT) {
        anyhow::bail!("upstream status {}", status);
    }

    let content_type = upstream
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(ToString::to_string);
    let content_range = upstream
        .headers()
        .get(reqwest::header::CONTENT_RANGE)
        .and_then(|v| v.to_str().ok())
        .map(ToString::to_string);
    let content_length = upstream
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .map(ToString::to_string);

    let body = upstream.bytes().await?;

    let bytes_served = u64::try_from(body.len()).unwrap_or(u64::MAX);

    let mut builder =
        HttpResponse::build(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK));
    builder.insert_header((header::ACCEPT_RANGES, "bytes"));

    if let Some(v) = content_type {
        if let Ok(value) = header::HeaderValue::from_str(&v) {
            builder.insert_header((header::CONTENT_TYPE, value));
        }
    }
    if let Some(v) = content_range {
        if let Ok(value) = header::HeaderValue::from_str(&v) {
            builder.insert_header((header::CONTENT_RANGE, value));
        }
    }
    if let Some(v) = content_length {
        if let Ok(value) = header::HeaderValue::from_str(&v) {
            builder.insert_header((header::CONTENT_LENGTH, value));
        }
    }

    Ok((builder.body(body), bytes_served))
}

fn subtitle_codec_from_path(path: &str) -> String {
    std::path::Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
        .unwrap_or_else(|| "srt".to_string())
}

fn subtitle_content_type(codec: &str) -> &'static str {
    match codec.to_ascii_lowercase().as_str() {
        "srt" => "application/x-subrip",
        "ass" | "ssa" => "text/x-ssa",
        "vtt" => "text/vtt",
        "smi" => "application/smil+xml",
        _ => "text/plain; charset=utf-8",
    }
}

fn image_content_type(path: &str) -> &'static str {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    }
}

fn parse_user_uuid(user: &UserDto) -> Option<Uuid> {
    Uuid::parse_str(&user.id).ok()
}

fn split_csv(raw: Option<&str>) -> Vec<String> {
    raw.map(|v| {
        v.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
    })
    .unwrap_or_default()
}

fn parse_uuid_csv(raw: Option<&str>) -> Vec<Uuid> {
    split_csv(raw)
        .into_iter()
        .filter_map(|value| Uuid::parse_str(&value).ok())
        .collect()
}

async fn resolve_item_uuid_or_bad_request(
    state: &ApiContext,
    raw_item_id: &str,
) -> Result<Uuid, Response> {
    let value = raw_item_id.trim();
    if value.is_empty() {
        return Err(error_response(StatusCode::BAD_REQUEST, "invalid item id"));
    }

    match state.infra.resolve_uuid_by_any_item_id(value).await {
        Ok(Some(item_id)) => Ok(item_id),
        Ok(None) => Err(error_response(StatusCode::BAD_REQUEST, "invalid item id")),
        Err(err) => Err(error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to resolve item id: {err}"),
        )),
    }
}

fn normalize_optional_item_id(raw_item_id: Option<&str>) -> Option<&str> {
    let value = raw_item_id?.trim();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

async fn resolve_optional_item_uuid(
    state: &ApiContext,
    raw_item_id: Option<&str>,
) -> Result<Option<Uuid>, Response> {
    let Some(raw_item_id) = normalize_optional_item_id(raw_item_id) else {
        return Ok(None);
    };
    resolve_item_uuid_or_bad_request(state, raw_item_id)
        .await
        .map(Some)
}

async fn compat_item_id_string_for_uuid(
    infra: &AppInfra,
    item_id: Uuid,
    cache: &mut HashMap<Uuid, i64>,
) -> anyhow::Result<String> {
    let compat_id = if let Some(compat_id) = cache.get(&item_id).copied() {
        compat_id
    } else {
        let compat_id = infra.compat_item_id_for_uuid(item_id).await?;
        cache.insert(item_id, compat_id);
        compat_id
    };
    Ok(compat_id.to_string())
}

async fn maybe_compat_item_id_string(
    infra: &AppInfra,
    raw_item_id: &str,
    cache: &mut HashMap<Uuid, i64>,
) -> anyhow::Result<Option<String>> {
    let trimmed = raw_item_id.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let Ok(item_id) = Uuid::parse_str(trimmed) else {
        return Ok(None);
    };
    let compat = compat_item_id_string_for_uuid(infra, item_id, cache).await?;
    Ok(Some(compat))
}

fn compat_item_id_json_value(compat_id: i64) -> Value {
    Value::String(compat_id.to_string())
}

async fn maybe_compat_item_id_value(
    infra: &AppInfra,
    value: &Value,
    cache: &mut HashMap<Uuid, i64>,
) -> anyhow::Result<Option<Value>> {
    let Some(raw_item_id) = value.as_str() else {
        return Ok(None);
    };
    let trimmed = raw_item_id.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let Ok(item_id) = Uuid::parse_str(trimmed) else {
        return Ok(None);
    };
    let compat_id = if let Some(compat_id) = cache.get(&item_id).copied() {
        compat_id
    } else {
        let compat_id = infra.compat_item_id_for_uuid(item_id).await?;
        cache.insert(item_id, compat_id);
        compat_id
    };
    Ok(Some(compat_item_id_json_value(compat_id)))
}

fn media_source_item_id_candidate(
    source: &serde_json::Map<String, Value>,
    item_id_fallback: Option<&Value>,
) -> Option<Value> {
    source
        .get("ItemId")
        .cloned()
        .filter(|value| !value.is_null())
        .or_else(|| item_id_fallback.cloned().filter(|value| !value.is_null()))
}

fn collect_compat_item_id_candidate(
    value: Option<&Value>,
    item_ids: &mut std::collections::HashSet<Uuid>,
) {
    let Some(raw_item_id) = value.and_then(Value::as_str) else {
        return;
    };
    let trimmed = raw_item_id.trim();
    if trimmed.is_empty() {
        return;
    }
    if let Ok(item_id) = Uuid::parse_str(trimmed) {
        item_ids.insert(item_id);
    }
}

fn collect_compat_item_ids_from_item(
    item: &serde_json::Map<String, Value>,
    item_ids: &mut std::collections::HashSet<Uuid>,
) {
    for key in [
        "Id",
        "ItemId",
        "ParentId",
        "SeriesId",
        "SeasonId",
        "ParentBackdropItemId",
        "ParentLogoItemId",
        "PrimaryImageItemId",
        "DisplayPreferencesId",
        "PresentationUniqueKey",
    ] {
        collect_compat_item_id_candidate(item.get(key), item_ids);
    }

    if let Some(user_data) = item.get("UserData").and_then(Value::as_object) {
        collect_compat_item_id_candidate(user_data.get("ItemId"), item_ids);
    }

    let item_id_fallback = item.get("Id");
    if let Some(media_sources) = item.get("MediaSources").and_then(Value::as_array) {
        for source in media_sources {
            let Some(source) = source.as_object() else {
                continue;
            };

            let Some(raw_item_id) = media_source_item_id_candidate(source, item_id_fallback) else {
                continue;
            };
            collect_compat_item_id_candidate(Some(&raw_item_id), item_ids);
        }
    }
}

async fn preload_compat_item_ids_cache(
    infra: &AppInfra,
    payload: &Value,
    cache: &mut HashMap<Uuid, i64>,
) -> anyhow::Result<()> {
    let mut item_ids = std::collections::HashSet::<Uuid>::new();

    match payload {
        Value::Array(items) => {
            for item in items {
                let Some(item_object) = item.as_object() else {
                    continue;
                };
                collect_compat_item_ids_from_item(item_object, &mut item_ids);
            }
        }
        Value::Object(object) => {
            if let Some(items) = object.get("Items").and_then(Value::as_array) {
                for item in items {
                    let Some(item_object) = item.as_object() else {
                        continue;
                    };
                    collect_compat_item_ids_from_item(item_object, &mut item_ids);
                }
            } else {
                collect_compat_item_ids_from_item(object, &mut item_ids);
            }
        }
        _ => {}
    }

    if item_ids.is_empty() {
        return Ok(());
    }

    let resolved = infra
        .compat_item_ids_for_uuids(&item_ids.into_iter().collect::<Vec<_>>())
        .await?;
    cache.extend(resolved);
    Ok(())
}

async fn apply_compat_item_id_fields(
    infra: &AppInfra,
    item: &mut serde_json::Map<String, Value>,
    cache: &mut HashMap<Uuid, i64>,
) -> anyhow::Result<()> {
    for key in [
        "Id",
        "ItemId",
        "ParentId",
        "SeriesId",
        "SeasonId",
        "ParentBackdropItemId",
        "ParentLogoItemId",
        "PrimaryImageItemId",
        "DisplayPreferencesId",
        "PresentationUniqueKey",
    ] {
        let Some(raw) = item.get(key).cloned() else {
            continue;
        };
        if let Some(compat) = maybe_compat_item_id_value(infra, &raw, cache).await? {
            item.insert(key.to_string(), compat);
        }
    }

    if let Some(user_data) = item.get_mut("UserData").and_then(Value::as_object_mut)
        && let Some(raw) = user_data.get("ItemId").cloned()
        && let Some(compat) = maybe_compat_item_id_value(infra, &raw, cache).await?
    {
        user_data.insert("ItemId".to_string(), compat);
    }

    let item_id_fallback = item.get("Id").cloned();
    if let Some(media_sources) = item.get_mut("MediaSources").and_then(Value::as_array_mut) {
        for source in media_sources {
            let Some(source) = source.as_object_mut() else {
                continue;
            };

            let Some(raw_item_id) =
                media_source_item_id_candidate(source, item_id_fallback.as_ref())
            else {
                continue;
            };
            if let Some(compat) = maybe_compat_item_id_value(infra, &raw_item_id, cache).await? {
                source.insert("ItemId".to_string(), compat);
            } else if source
                .get("ItemId")
                .map(Value::is_null)
                .unwrap_or(true)
            {
                source.insert("ItemId".to_string(), raw_item_id);
            }
        }
    }

    Ok(())
}

async fn apply_compat_item_ids_for_response(
    infra: &AppInfra,
    payload: &mut Value,
) -> anyhow::Result<()> {
    let mut cache = HashMap::<Uuid, i64>::new();
    preload_compat_item_ids_cache(infra, payload, &mut cache).await?;
    match payload {
        Value::Array(items) => {
            for item in items {
                if let Some(item_object) = item.as_object_mut() {
                    apply_compat_item_id_fields(infra, item_object, &mut cache).await?;
                }
            }
        }
        Value::Object(object) => {
            if let Some(items) = object.get_mut("Items").and_then(Value::as_array_mut) {
                for item in items {
                    let Some(item_object) = item.as_object_mut() else {
                        continue;
                    };
                    apply_compat_item_id_fields(infra, item_object, &mut cache).await?;
                }
            } else {
                apply_compat_item_id_fields(infra, object, &mut cache).await?;
            }
        }
        _ => {}
    }
    Ok(())
}
