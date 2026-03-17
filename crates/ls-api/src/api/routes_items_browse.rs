async fn get_items(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<ItemsQuery>,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(user) => user,
        Err(resp) => return resp,
    };

    let user_id = query
        .user_id
        .or_else(|| Uuid::parse_str(&auth_user.id).ok());

    let parent_id = match resolve_optional_item_uuid(&state, query.parent_id.as_deref()).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    let options = build_items_query_options(user_id, &query, parent_id);

    match state.infra.list_items_with_options(options).await {
        Ok(items) => {
            let compat = apply_items_query_compatibility(items, &query);
            let mut payload = compat_items_query_result_json(
                compat,
                query._fields.as_deref(),
                &state.infra.server_id,
                true,
            );
            if let Err(err) = apply_compat_item_ids_for_response(&state.infra, &mut payload).await {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("failed to normalize item ids: {err}"),
                );
            }
            Json(payload).into_response()
        }
        Err(err) => {
            let status = map_items_query_error(&err);
            error_response(status, &format!("failed to query items: {err}"))
        }
    }
}

fn normalize_items_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(100).clamp(1, 500)
}

fn normalize_items_start_index(start_index: Option<i64>) -> i64 {
    start_index.unwrap_or(0).max(0)
}

fn normalize_items_sort_order(sort_order: Option<&str>) -> String {
    let Some(sort_order) = sort_order.map(str::trim).filter(|raw| !raw.is_empty()) else {
        return "Ascending".to_string();
    };
    sort_order.to_string()
}

fn filter_query_flag(filters: Option<&str>, include_flag: &str, exclude_flag: &str) -> Option<bool> {
    if split_csv(filters)
        .iter()
        .any(|value| value.eq_ignore_ascii_case(include_flag))
    {
        return Some(true);
    }
    if split_csv(filters)
        .iter()
        .any(|value| value.eq_ignore_ascii_case(exclude_flag))
    {
        return Some(false);
    }
    None
}

fn normalize_items_include_item_types(query: &ItemsQuery) -> Vec<String> {
    filter_item_types_by_media_types(
        split_csv(query.include_item_types.as_deref()),
        &split_csv(query.media_types.as_deref()),
    )
}

fn normalize_items_search_term(query: &ItemsQuery) -> Option<String> {
    query
        .search_term
        .as_deref()
        .or(query.name_starts_with.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn split_tag_filters(raw: Option<&str>) -> Vec<String> {
    raw.map(|value| {
        value
            .split(|ch| ch == ',' || ch == '|')
            .map(str::trim)
            .filter(|tag| !tag.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>()
    })
    .unwrap_or_default()
}

fn build_items_query_options(
    user_id: Option<Uuid>,
    query: &ItemsQuery,
    parent_id: Option<Uuid>,
) -> InfraItemsQuery {
    let search_term = normalize_items_search_term(query);
    let mut include_item_types = normalize_items_include_item_types(query);
    if include_item_types.is_empty() && search_term.is_some() {
        include_item_types = vec![
            "Folder".to_string(),
            "Movie".to_string(),
            "Series".to_string(),
            "Video".to_string(),
            "Person".to_string(),
        ];
    }

    InfraItemsQuery {
        user_id,
        series_filter: None,
        parent_id,
        include_item_types,
        exclude_item_types: split_csv(query.exclude_item_types.as_deref()),
        person_ids: parse_uuid_csv(query.person_ids.as_deref()),
        search_term,
        limit: normalize_items_limit(query.limit),
        start_index: normalize_items_start_index(query.start_index),
        is_resumable: has_is_resumable_filter(query.filters.as_deref()),
        sort_by: split_csv(query.sort_by.as_deref()),
        sort_order: normalize_items_sort_order(query.sort_order.as_deref()),
        recursive: query.recursive.unwrap_or(false),
        genres: split_csv(query.genres.as_deref()),
        tags: split_tag_filters(query.tags.as_deref()),
        years: split_csv(query.years.as_deref())
            .into_iter()
            .filter_map(|s| s.parse::<i32>().ok())
            .collect(),
        is_favorite: query
            .is_favorite
            .or_else(|| filter_query_flag(query.filters.as_deref(), "IsFavorite", "IsNotFavorite"))
            .or_else(|| filter_query_flag(query.filters.as_deref(), "Likes", "Dislikes")),
        is_played: query
            .is_played
            .or_else(|| filter_query_flag(query.filters.as_deref(), "IsPlayed", "IsUnplayed")),
        min_community_rating: query.min_community_rating,
    }
}

fn item_id_matches_query_ids(item: &BaseItemDto, query_ids: &[String]) -> bool {
    query_ids
        .iter()
        .any(|id| id.eq_ignore_ascii_case(item.id.as_str()))
}

fn apply_items_query_compatibility(
    mut result: QueryResultDto<BaseItemDto>,
    query: &ItemsQuery,
) -> QueryResultDto<BaseItemDto> {
    let exclude_item_types = split_csv(query.exclude_item_types.as_deref());
    let query_ids = split_csv(query.ids.as_deref());
    let should_filter = !exclude_item_types.is_empty() || !query_ids.is_empty();

    if should_filter {
        let original_total_record_count = result.total_record_count;
        result.items = result
            .items
            .into_iter()
            .filter(|item| {
                let excluded = exclude_item_types
                    .iter()
                    .any(|value| value.eq_ignore_ascii_case(&item.item_type));
                let id_filtered = !query_ids.is_empty() && !item_id_matches_query_ids(item, &query_ids);
                !excluded && !id_filtered
            })
            .collect();
        // Preserve infra total count for type-based filtering so paging can continue.
        // Query-id filtering is applied in compatibility layer, so recompute in that case.
        result.total_record_count = if query_ids.is_empty() {
            original_total_record_count
        } else {
            result.items.len() as i32
        };
    }

    if query.enable_total_record_count == Some(false) {
        result.total_record_count = 0;
    }

    result
}

fn normalize_compat_datetime(raw: &str) -> Option<String> {
    let format_utc_ticks = |dt: DateTime<Utc>| {
        // Emby/Jellyfin payloads commonly use 7-digit fractional seconds (100ns ticks).
        let ticks_fraction = dt.timestamp_subsec_nanos() / 100;
        format!(
            "{}.{ticks_fraction:07}Z",
            dt.format("%Y-%m-%dT%H:%M:%S")
        )
    };

    if let Ok(dt) = DateTime::parse_from_rfc3339(raw) {
        return Some(format_utc_ticks(dt.with_timezone(&Utc)));
    }

    // Some metadata paths persist date-only values (YYYY-MM-DD). Normalize to UTC midnight.
    if let Ok(date) = chrono::NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d")
        && let Some(naive) = date.and_hms_opt(0, 0, 0)
    {
        let dt = DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc);
        return Some(format_utc_ticks(dt));
    }

    None
}

fn normalize_item_datetime_fields(item: &mut serde_json::Map<String, Value>) {
    for key in ["DateCreated", "PremiereDate", "EndDate"] {
        let Some(value) = item.get(key).and_then(Value::as_str) else {
            continue;
        };
        let Some(normalized) = normalize_compat_datetime(value) else {
            continue;
        };
        item.insert(key.to_string(), Value::String(normalized));
    }
}

fn parse_requested_item_fields(fields: Option<&str>) -> std::collections::BTreeSet<String> {
    split_csv(fields)
        .into_iter()
        .map(|field| field.trim().to_ascii_lowercase())
        .filter(|field| !field.is_empty())
        .collect()
}

fn item_field_requested(
    requested_fields: &std::collections::BTreeSet<String>,
    field_name: &str,
) -> bool {
    requested_fields.contains(&field_name.to_ascii_lowercase())
}

fn apply_compact_items_field_filter(
    item: &mut serde_json::Map<String, Value>,
    requested_fields: &std::collections::BTreeSet<String>,
) {
    if requested_fields.is_empty() {
        return;
    }

    let removable_fields = [
        ("Path", "path"),
        ("LocationType", "locationtype"),
        ("CanDownload", "candownload"),
        ("MediaType", "mediatype"),
        ("Container", "container"),
        ("Bitrate", "bitrate"),
        ("MediaSources", "mediasources"),
        ("MediaStreams", "mediastreams"),
        ("Overview", "overview"),
        ("PremiereDate", "premieredate"),
        ("EndDate", "enddate"),
        ("ProductionYear", "productionyear"),
        ("Genres", "genres"),
        ("ProviderIds", "providerids"),
        ("PrimaryImageTag", "primaryimagetag"),
        ("OfficialRating", "officialrating"),
        ("CommunityRating", "communityrating"),
        ("Studios", "studios"),
        ("People", "people"),
        ("SortName", "sortname"),
        ("PrimaryImageAspectRatio", "primaryimageaspectratio"),
        ("DateCreated", "datecreated"),
        ("PlayAccess", "playaccess"),
        ("Status", "status"),
    ];

    for (json_field, query_field) in removable_fields {
        let keep = item_field_requested(requested_fields, query_field)
            || (query_field.eq_ignore_ascii_case("mediasources")
                && item_field_requested(requested_fields, "mediastreams"));
        if !keep {
            item.remove(json_field);
        }
    }
}

fn ensure_compat_user_data(item: &mut serde_json::Map<String, Value>) {
    let runtime_ticks = item
        .get("RunTimeTicks")
        .and_then(value_to_i64)
        .or_else(|| {
            item.get("MediaSources")
                .and_then(Value::as_array)
                .and_then(|sources| sources.first())
                .and_then(|source| source.get("RunTimeTicks"))
                .and_then(value_to_i64)
        })
        .unwrap_or(0);

    let default = || {
        let mut map = serde_json::Map::new();
        map.insert("Played".to_string(), Value::Bool(false));
        map.insert("PlaybackPositionTicks".to_string(), Value::from(0));
        map.insert("PlayCount".to_string(), Value::from(0));
        map.insert("IsFavorite".to_string(), Value::Bool(false));
        map
    };

    if let Some(user_data) = item.get_mut("UserData").and_then(Value::as_object_mut) {
        if !user_data.contains_key("PlayCount") {
            user_data.insert("PlayCount".to_string(), Value::from(0));
        }
        if !user_data.contains_key("IsFavorite") {
            user_data.insert("IsFavorite".to_string(), Value::Bool(false));
        }
        if !user_data.contains_key("Played") {
            user_data.insert("Played".to_string(), Value::Bool(false));
        }
        if !user_data.contains_key("PlaybackPositionTicks") {
            user_data.insert("PlaybackPositionTicks".to_string(), Value::from(0));
        }

        if !user_data.contains_key("PlayedPercentage") {
            let position = user_data
                .get("PlaybackPositionTicks")
                .and_then(value_to_i64)
                .unwrap_or(0);
            if position > 0 && runtime_ticks > 0 {
                let percent = (position as f64) * 100.0 / (runtime_ticks as f64);
                let percent = percent.clamp(0.0, 100.0);
                if percent.is_finite() {
                    user_data.insert(
                        "PlayedPercentage".to_string(),
                        Value::from(percent),
                    );
                }
            }
        }
        return;
    }

    item.insert("UserData".to_string(), Value::Object(default()));
}

fn ensure_runtime_ticks_default(item: &mut serde_json::Map<String, Value>) {
    if !item
        .get("RunTimeTicks")
        .map(Value::is_null)
        .unwrap_or(true)
    {
        return;
    }

    let item_type_is_media = item
        .get("Type")
        .and_then(Value::as_str)
        .map(|item_type| {
            matches!(
                item_type,
                "Movie"
                    | "Episode"
                    | "Video"
                    | "MusicVideo"
                    | "Trailer"
                    | "Song"
                    | "Audio"
                    | "Series"
            )
        })
        .unwrap_or(false);
    let has_media_context =
        item_type_is_media || item.contains_key("MediaSources") || item.get("MediaType").is_some();
    if !has_media_context {
        return;
    }

    let runtime_ticks = item
        .get("MediaSources")
        .and_then(Value::as_array)
        .and_then(|sources| sources.first())
        .and_then(|source| source.get("RunTimeTicks"))
        .and_then(value_to_i64)
        .unwrap_or(0);
    item.insert("RunTimeTicks".to_string(), Value::from(runtime_ticks));
}

fn stable_compat_numeric_id(input: &str) -> i64 {
    let mut hash: u64 = 1469598103934665603;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1099511628211);
    }
    (hash & 0x7fff_ffff) as i64
}

fn ensure_name_id_objects_with_fallback(
    values: &mut [Value],
    fallback_prefix: &str,
) {
    for (idx, value) in values.iter_mut().enumerate() {
        let Some(object) = value.as_object_mut() else {
            continue;
        };
        let Some(name) = object.get("Name").and_then(Value::as_str) else {
            continue;
        };

        if object.get("Id").is_none() {
            let key = format!("{fallback_prefix}:{idx}:{name}");
            object.insert("Id".to_string(), Value::from(stable_compat_numeric_id(&key)));
            continue;
        }

        let Some(raw_id) = object.get("Id").cloned() else {
            continue;
        };
        let parsed = raw_id
            .as_i64()
            .or_else(|| raw_id.as_u64().and_then(|value| i64::try_from(value).ok()))
            .or_else(|| raw_id.as_str().and_then(|value| value.parse::<i64>().ok()));
        if let Some(value) = parsed {
            object.insert("Id".to_string(), Value::from(value));
        }
    }
}

fn ensure_tag_items_from_tags(item: &mut serde_json::Map<String, Value>) {
    if !item.contains_key("TagItems") {
        let derived = item
            .get("Tags")
            .and_then(Value::as_array)
            .map(|tags| {
                let mut seen = std::collections::HashSet::new();
                tags.iter()
                    .enumerate()
                    .filter_map(|(idx, tag)| {
                        let name = tag
                            .as_str()
                            .map(str::trim)
                            .filter(|value| !value.is_empty())?;
                        if !seen.insert(name.to_ascii_lowercase()) {
                            return None;
                        }
                        let mut object = serde_json::Map::new();
                        object.insert(
                            "Id".to_string(),
                            Value::from(stable_compat_numeric_id(&format!("tag:{idx}:{name}"))),
                        );
                        object.insert("Name".to_string(), Value::String(name.to_string()));
                        Some(Value::Object(object))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        item.insert("TagItems".to_string(), Value::Array(derived));
    }

    if let Some(tag_items) = item.get_mut("TagItems").and_then(Value::as_array_mut) {
        ensure_name_id_objects_with_fallback(tag_items, "tag");
    }
}

fn compat_value_to_non_empty_string(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        .map(str::to_string)
        .or_else(|| value.as_i64().map(|raw| raw.to_string()))
        .or_else(|| value.as_u64().map(|raw| raw.to_string()))
}

fn provider_id_from_map(
    provider_ids: &serde_json::Map<String, Value>,
    key: &str,
) -> Option<String> {
    provider_ids.iter().find_map(|(candidate, value)| {
        if candidate.eq_ignore_ascii_case(key) {
            compat_value_to_non_empty_string(value)
        } else {
            None
        }
    })
}

fn external_url_name_from_url(url: &str) -> String {
    let lowered = url.to_ascii_lowercase();
    if lowered.contains("imdb.com") {
        return "IMDb".to_string();
    }
    if lowered.contains("themoviedb.org") {
        return "TMDb".to_string();
    }
    if lowered.contains("thetvdb.com") {
        return "TVDB".to_string();
    }
    "Link".to_string()
}

fn infer_external_urls_from_provider_ids(item: &serde_json::Map<String, Value>) -> Vec<Value> {
    let Some(provider_ids) = item.get("ProviderIds").and_then(Value::as_object) else {
        return Vec::new();
    };

    let item_type = item.get("Type").and_then(Value::as_str).unwrap_or_default();
    let tmdb_kind = if item_type.eq_ignore_ascii_case("series")
        || item_type.eq_ignore_ascii_case("season")
        || item_type.eq_ignore_ascii_case("episode")
    {
        "tv"
    } else {
        "movie"
    };

    let mut inferred: Vec<Value> = Vec::new();
    let mut push_unique = |name: &str, url: String| {
        let duplicate = inferred.iter().any(|entry| {
            entry
                .get("Url")
                .and_then(Value::as_str)
                .map(|existing| existing.eq_ignore_ascii_case(url.as_str()))
                .unwrap_or(false)
        });
        if !duplicate {
            inferred.push(json!({
                "Name": name,
                "Url": url,
            }));
        }
    };

    if let Some(imdb) = provider_id_from_map(provider_ids, "imdb") {
        push_unique("IMDb", format!("https://www.imdb.com/title/{imdb}"));
    }
    if let Some(tmdb) = provider_id_from_map(provider_ids, "tmdb") {
        push_unique("TMDb", format!("https://www.themoviedb.org/{tmdb_kind}/{tmdb}"));
    }
    if let Some(tvdb) = provider_id_from_map(provider_ids, "tvdb") {
        push_unique("TVDB", format!("https://www.thetvdb.com/?id={tvdb}"));
    }

    inferred
}

fn ensure_external_url_defaults(external_urls: &mut [Value]) {
    for entry in external_urls.iter_mut() {
        if let Some(url) = entry
            .as_str()
            .map(str::trim)
            .filter(|raw| !raw.is_empty())
            .map(str::to_string)
        {
            *entry = json!({
                "Name": external_url_name_from_url(&url),
                "Url": url,
            });
            continue;
        }

        let Some(object) = entry.as_object_mut() else {
            continue;
        };
        let Some(url) = object
            .get("Url")
            .or_else(|| object.get("url"))
            .and_then(compat_value_to_non_empty_string)
        else {
            continue;
        };
        object.insert("Url".to_string(), Value::String(url.clone()));
        object.remove("url");

        let name = object
            .get("Name")
            .or_else(|| object.get("name"))
            .and_then(compat_value_to_non_empty_string)
            .unwrap_or_else(|| external_url_name_from_url(&url));
        object.insert("Name".to_string(), Value::String(name));
        object.remove("name");
    }
}

fn ensure_external_urls(item: &mut serde_json::Map<String, Value>) {
    let inferred = infer_external_urls_from_provider_ids(item);

    if !matches!(item.get("ExternalUrls"), Some(Value::Array(_))) {
        item.insert("ExternalUrls".to_string(), Value::Array(Vec::new()));
    }

    if let Some(external_urls) = item.get_mut("ExternalUrls").and_then(Value::as_array_mut) {
        if external_urls.is_empty() && !inferred.is_empty() {
            external_urls.extend(inferred);
        }
        ensure_external_url_defaults(external_urls);
    }
}

fn ensure_compat_common_item_defaults(item: &mut serde_json::Map<String, Value>) {
    if !item.contains_key("DateModified")
        && let Some(date_created) = item.get("DateCreated").and_then(Value::as_str)
    {
        item.insert(
            "DateModified".to_string(),
            Value::String(date_created.to_string()),
        );
    }

    if !item.contains_key("DisplayPreferencesId")
        && let Some(item_id) = item.get("Id").and_then(Value::as_str)
    {
        item.insert(
            "DisplayPreferencesId".to_string(),
            Value::String(item_id.to_string()),
        );
    }

    if !item.contains_key("PresentationUniqueKey")
        && let Some(item_id) = item.get("Id").and_then(Value::as_str)
    {
        item.insert(
            "PresentationUniqueKey".to_string(),
            Value::String(item_id.to_string()),
        );
    }

    if !item.contains_key("Guid")
        && let Some(item_id) = item.get("Id").and_then(Value::as_str)
    {
        item.insert("Guid".to_string(), Value::String(item_id.to_string()));
    }

    if !item.contains_key("Etag") {
        let etag_seed = item
            .get("Id")
            .and_then(Value::as_str)
            .or_else(|| item.get("Name").and_then(Value::as_str))
            .unwrap_or("ls-item");
        item.insert(
            "Etag".to_string(),
            Value::String(format!("{:032x}", stable_compat_numeric_id(etag_seed))),
        );
    }

    if !item.contains_key("ForcedSortName") {
        let forced = item
            .get("SortName")
            .and_then(Value::as_str)
            .or_else(|| item.get("Name").and_then(Value::as_str))
            .unwrap_or_default()
            .to_string();
        item.insert("ForcedSortName".to_string(), Value::String(forced));
    }

    ensure_external_urls(item);
    item.entry("RemoteTrailers".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    item.entry("Taglines".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    item.entry("LockedFields".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    item.entry("LockData".to_string())
        .or_insert_with(|| Value::Bool(false));

    let is_folder = item.get("IsFolder").and_then(Value::as_bool).unwrap_or(false);
    if is_folder && !item.contains_key("PrimaryImageAspectRatio") {
        item.insert("PrimaryImageAspectRatio".to_string(), Value::from(1.0f64));
    }

    ensure_compat_primary_image_tag(item);
}

fn ensure_vidhub_series_detail_defaults(item: &mut serde_json::Map<String, Value>) {
    let Some(item_type) = item.get("Type").and_then(Value::as_str) else {
        return;
    };
    if !item_type.eq_ignore_ascii_case("Series") {
        return;
    }

    item.entry("RunTimeTicks".to_string())
        .or_insert_with(|| Value::from(0));
    item.entry("PrimaryImageAspectRatio".to_string())
        .or_insert_with(|| Value::from(2.0f64 / 3.0f64));
    item.entry("ChildCount".to_string())
        .or_insert_with(|| Value::from(0));
    item.entry("DisplayOrder".to_string())
        .or_insert_with(|| Value::String("Aired".to_string()));
    item.entry("LocalTrailerCount".to_string())
        .or_insert_with(|| Value::from(0));
    item.entry("LockData".to_string())
        .or_insert_with(|| Value::Bool(false));
    item.entry("LockedFields".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    item.entry("ExternalUrls".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    item.entry("RemoteTrailers".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    ensure_tag_items_from_tags(item);
    item.entry("Taglines".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    item.entry("OfficialRating".to_string())
        .or_insert_with(|| Value::String(String::new()));

    if !item.contains_key("OriginalTitle")
        && let Some(name) = item.get("Name").and_then(Value::as_str)
    {
        item.insert("OriginalTitle".to_string(), Value::String(name.to_string()));
    }
    if !item.contains_key("FileName")
        && let Some(name) = item.get("Name").and_then(Value::as_str)
    {
        item.insert("FileName".to_string(), Value::String(name.to_string()));
    }
    if !item.contains_key("ForcedSortName") {
        let forced = item
            .get("SortName")
            .and_then(Value::as_str)
            .or_else(|| item.get("Name").and_then(Value::as_str))
            .unwrap_or_default()
            .to_string();
        item.insert("ForcedSortName".to_string(), Value::String(forced));
    }
    if !item.contains_key("DateModified")
        && let Some(date_created) = item.get("DateCreated").and_then(Value::as_str)
    {
        item.insert(
            "DateModified".to_string(),
            Value::String(date_created.to_string()),
        );
    }
    if !item.contains_key("DisplayPreferencesId")
        && let Some(item_id) = item.get("Id").and_then(Value::as_str)
    {
        item.insert(
            "DisplayPreferencesId".to_string(),
            Value::String(item_id.to_string()),
        );
    }
    if !item.contains_key("PresentationUniqueKey")
        && let Some(item_id) = item.get("Id").and_then(Value::as_str)
    {
        item.insert(
            "PresentationUniqueKey".to_string(),
            Value::String(item_id.to_string()),
        );
    }
    if !item.contains_key("Etag")
        && let Some(item_id) = item.get("Id").and_then(Value::as_str)
    {
        item.insert(
            "Etag".to_string(),
            Value::String(format!("{:032x}", stable_compat_numeric_id(item_id))),
        );
    }

    if !item.contains_key("GenreItems")
        && let Some(genres) = item.get("Genres").and_then(Value::as_array)
    {
        let genre_items = genres
            .iter()
            .enumerate()
            .filter_map(|(idx, genre)| {
                genre.as_str().map(|name| {
                    let mut object = serde_json::Map::new();
                    object.insert(
                        "Id".to_string(),
                        Value::from(stable_compat_numeric_id(&format!("genre:{idx}:{name}"))),
                    );
                    object.insert("Name".to_string(), Value::String(name.to_string()));
                    Value::Object(object)
                })
            })
            .collect::<Vec<_>>();
        item.insert("GenreItems".to_string(), Value::Array(genre_items));
    }
    if let Some(genre_items) = item.get_mut("GenreItems").and_then(Value::as_array_mut) {
        ensure_name_id_objects_with_fallback(genre_items, "genre");
    }
    if let Some(studios) = item.get_mut("Studios").and_then(Value::as_array_mut) {
        ensure_name_id_objects_with_fallback(studios, "studio");
    }

    let child_count_fallback = item
        .get("ChildCount")
        .and_then(Value::as_i64)
        .or_else(|| {
            item.get("ChildCount")
                .and_then(Value::as_u64)
                .and_then(|value| i64::try_from(value).ok())
        })
        .unwrap_or(0);
    if let Some(user_data) = item.get_mut("UserData").and_then(Value::as_object_mut) {
        if !user_data.contains_key("UnplayedItemCount") {
            user_data.insert(
                "UnplayedItemCount".to_string(),
                Value::from(child_count_fallback),
            );
        }
    }
}

fn infer_media_streams_from_sources(item: &serde_json::Map<String, Value>) -> Option<Vec<Value>> {
    let media_sources = item.get("MediaSources").and_then(Value::as_array)?;
    Some(
        media_sources
            .iter()
            .filter_map(Value::as_object)
            .filter_map(|source| source.get("MediaStreams").and_then(Value::as_array))
            .flat_map(|streams| streams.iter().cloned())
            .collect::<Vec<_>>(),
    )
}

const CHAPTER_TICKS_PER_SECOND: f64 = 10_000_000.0;

fn chapter_seconds_to_ticks(value: &Value) -> Option<i64> {
    let seconds = value
        .as_f64()
        .or_else(|| value.as_i64().map(|raw| raw as f64))
        .or_else(|| value.as_u64().map(|raw| raw as f64))
        .or_else(|| value.as_str().and_then(|raw| raw.trim().parse::<f64>().ok()))?;
    if !seconds.is_finite() || seconds < 0.0 {
        return None;
    }
    let ticks = seconds * CHAPTER_TICKS_PER_SECOND;
    if ticks > i64::MAX as f64 {
        return None;
    }
    Some(ticks.round() as i64)
}

fn chapter_name_from_object(chapter_obj: &serde_json::Map<String, Value>) -> Option<String> {
    chapter_obj
        .get("Name")
        .or_else(|| chapter_obj.get("name"))
        .or_else(|| chapter_obj.get("Title"))
        .or_else(|| chapter_obj.get("title"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        .map(str::to_string)
        .or_else(|| {
            chapter_obj
                .get("tags")
                .or_else(|| chapter_obj.get("Tags"))
                .and_then(Value::as_object)
                .and_then(|tags| {
                    tags.get("title")
                        .or_else(|| tags.get("Title"))
                        .or_else(|| tags.get("name"))
                        .or_else(|| tags.get("Name"))
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|raw| !raw.is_empty())
                        .map(str::to_string)
                })
        })
}

fn ensure_chapter_defaults(chapters: &mut [Value]) {
    for (fallback_idx, chapter) in chapters.iter_mut().enumerate() {
        let Some(chapter_obj) = chapter.as_object_mut() else {
            continue;
        };

        let chapter_index = chapter_obj
            .get("ChapterIndex")
            .or_else(|| chapter_obj.get("chapterIndex"))
            .or_else(|| chapter_obj.get("id"))
            .and_then(value_to_i64)
            .or_else(|| i64::try_from(fallback_idx).ok())
            .unwrap_or(0);
        chapter_obj.insert("ChapterIndex".to_string(), Value::from(chapter_index));

        let start_position_ticks = chapter_obj
            .get("StartPositionTicks")
            .or_else(|| chapter_obj.get("startPositionTicks"))
            .or_else(|| chapter_obj.get("start_position_ticks"))
            .and_then(value_to_i64)
            .or_else(|| {
                chapter_obj
                    .get("start_time")
                    .or_else(|| chapter_obj.get("StartTime"))
                    .and_then(chapter_seconds_to_ticks)
            })
            .or_else(|| {
                chapter_obj
                    .get("start")
                    .or_else(|| chapter_obj.get("Start"))
                    .and_then(chapter_seconds_to_ticks)
            })
            .unwrap_or(0)
            .max(0);
        chapter_obj.insert(
            "StartPositionTicks".to_string(),
            Value::from(start_position_ticks),
        );

        let name = chapter_name_from_object(chapter_obj)
            .unwrap_or_else(|| format!("Chapter {}", chapter_index.saturating_add(1)));
        chapter_obj.insert("Name".to_string(), Value::String(name));

        chapter_obj
            .entry("MarkerType".to_string())
            .or_insert_with(|| Value::String("Chapter".to_string()));
    }
}

fn infer_chapters_from_sources(item: &serde_json::Map<String, Value>) -> Option<Vec<Value>> {
    let media_sources = item.get("MediaSources").and_then(Value::as_array)?;
    media_sources
        .iter()
        .filter_map(Value::as_object)
        .find_map(|source| source.get("Chapters").and_then(Value::as_array).cloned())
}

fn ensure_top_level_chapters(item: &mut serde_json::Map<String, Value>) {
    if !item.contains_key("Chapters")
        && let Some(chapters) = infer_chapters_from_sources(item)
    {
        item.insert("Chapters".to_string(), Value::Array(chapters));
    }

    if let Some(chapters) = item.get_mut("Chapters").and_then(Value::as_array_mut) {
        ensure_chapter_defaults(chapters);
    }
}

fn ensure_top_level_media_streams(item: &mut serde_json::Map<String, Value>) {
    if item.contains_key("MediaStreams") {
        return;
    }
    if let Some(streams) = infer_media_streams_from_sources(item) {
        item.insert("MediaStreams".to_string(), Value::Array(streams));
    }
}

fn ensure_top_level_media_streams_if_requested(
    item: &mut serde_json::Map<String, Value>,
    requested_fields: &std::collections::BTreeSet<String>,
) {
    if !item_field_requested(requested_fields, "mediastreams") {
        return;
    }
    ensure_top_level_media_streams(item);
}

fn ensure_size_defaults(item: &mut serde_json::Map<String, Value>) {
    let is_folder = item.get("IsFolder").and_then(Value::as_bool).unwrap_or(false);
    if is_folder {
        return;
    }
    let has_media_context = item.contains_key("MediaSources") || item.get("MediaType").is_some();
    if !has_media_context {
        return;
    }

    let mut top_size_candidate = None;

    if let Some(media_sources) = item.get_mut("MediaSources").and_then(Value::as_array_mut) {
        for source in media_sources.iter_mut().filter_map(Value::as_object_mut) {
            if !source.contains_key("Size") {
                source.insert("Size".to_string(), Value::from(0));
            }
            if top_size_candidate.is_none() {
                top_size_candidate = source.get("Size").and_then(value_to_i64);
            }
        }
    }

    item.entry("Size".to_string())
        .or_insert_with(|| Value::from(top_size_candidate.unwrap_or(0)));
}

fn item_is_episode(item: &serde_json::Map<String, Value>) -> bool {
    item.get("Type")
        .and_then(Value::as_str)
        .map(|item_type| item_type.eq_ignore_ascii_case("Episode"))
        .unwrap_or(false)
}

fn item_is_movie(item: &serde_json::Map<String, Value>) -> bool {
    item.get("Type")
        .and_then(Value::as_str)
        .map(|item_type| item_type.eq_ignore_ascii_case("Movie"))
        .unwrap_or(false)
}

fn value_to_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|v| i64::try_from(v).ok()))
}

fn infer_video_dimensions(item: &serde_json::Map<String, Value>) -> Option<(i64, i64)> {
    let media_streams = item.get("MediaStreams").and_then(Value::as_array)?;
    media_streams
        .iter()
        .filter_map(Value::as_object)
        .find(|stream| {
            stream
                .get("Type")
                .and_then(Value::as_str)
                .map(|stream_type| stream_type.eq_ignore_ascii_case("Video"))
                .unwrap_or(false)
        })
        .and_then(|stream| {
            let width = stream.get("Width").and_then(value_to_i64)?;
            let height = stream.get("Height").and_then(value_to_i64)?;
            if width > 0 && height > 0 {
                Some((width, height))
            } else {
                None
            }
        })
}

fn infer_file_name_from_path(path: &str) -> Option<String> {
    path.rsplit(['/', '\\'])
        .find(|segment| !segment.is_empty())
        .map(ToString::to_string)
}

fn infer_default_audio_stream_index(media_streams: &[Value]) -> Option<i64> {
    let mut first_audio_index = None;
    for stream in media_streams.iter().filter_map(Value::as_object) {
        let is_audio = stream
            .get("Type")
            .and_then(Value::as_str)
            .map(|stream_type| stream_type.eq_ignore_ascii_case("Audio"))
            .unwrap_or(false);
        if !is_audio {
            continue;
        }
        let index = stream.get("Index").and_then(value_to_i64)?;
        if first_audio_index.is_none() {
            first_audio_index = Some(index);
        }
        let is_default = stream
            .get("IsDefault")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if is_default {
            return Some(index);
        }
    }
    first_audio_index
}

fn episode_subtitle_codec_is_text(codec: &str) -> bool {
    matches!(
        codec,
        "ass"
            | "ssa"
            | "srt"
            | "subrip"
            | "webvtt"
            | "vtt"
            | "ttml"
            | "tx3g"
            | "mov_text"
            | "smi"
            | "sami"
    )
}

fn episode_subtitle_codec_is_image(codec: &str) -> bool {
    matches!(
        codec,
        "pgssub"
            | "hdmv_pgs_subtitle"
            | "pgs"
            | "dvd_subtitle"
            | "dvb_subtitle"
            | "xsub"
            | "vobsub"
            | "vob_subtitle"
    )
}

fn infer_episode_is_text_subtitle_stream(stream_type: &str, codec: Option<&str>) -> bool {
    if !stream_type.eq_ignore_ascii_case("Subtitle") {
        return false;
    }
    let Some(codec) = codec.map(str::trim).filter(|value| !value.is_empty()) else {
        // Keep legacy behavior for subtitle streams with unknown codec.
        return true;
    };
    let normalized = codec.to_ascii_lowercase();
    if episode_subtitle_codec_is_image(normalized.as_str()) {
        return false;
    }
    if episode_subtitle_codec_is_text(normalized.as_str()) {
        return true;
    }
    true
}

fn ensure_episode_media_stream_defaults(
    media_streams: &mut [Value],
    source_protocol: &str,
) {
    for stream in media_streams.iter_mut() {
        let Some(stream_obj) = stream.as_object_mut() else {
            continue;
        };

        let stream_type = stream_obj
            .get("Type")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let is_subtitle = stream_type.eq_ignore_ascii_case("Subtitle");
        let is_external = stream_obj
            .get("IsExternal")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let codec = stream_obj
            .get("Codec")
            .and_then(Value::as_str)
            .map(str::to_string);
        let is_text_subtitle_stream =
            infer_episode_is_text_subtitle_stream(&stream_type, codec.as_deref());
        let language = stream_obj
            .get("Language")
            .and_then(Value::as_str)
            .unwrap_or("und")
            .to_string();

        stream_obj
            .entry("IsTextSubtitleStream".to_string())
            .or_insert_with(|| Value::Bool(is_text_subtitle_stream));
        if is_subtitle {
            stream_obj
                .entry("DeliveryMethod".to_string())
                .or_insert_with(|| {
                    Value::String(if is_external {
                        "External".to_string()
                    } else {
                        "Embed".to_string()
                    })
                });
            stream_obj
                .entry("SubtitleLocationType".to_string())
                .or_insert_with(|| {
                    Value::String(if is_external {
                        "ExternalFile".to_string()
                    } else {
                        "InternalStream".to_string()
                    })
                });
        }
        stream_obj
            .entry("IsInterlaced".to_string())
            .or_insert_with(|| Value::Bool(false));
        stream_obj
            .entry("IsHearingImpaired".to_string())
            .or_insert_with(|| Value::Bool(false));
        stream_obj
            .entry("IsAnamorphic".to_string())
            .or_insert_with(|| Value::Bool(false));
        stream_obj
            .entry("SupportsExternalStream".to_string())
            .or_insert_with(|| Value::Bool(is_external));
        stream_obj
            .entry("Protocol".to_string())
            .or_insert_with(|| Value::String(source_protocol.to_string()));
        stream_obj
            .entry("AttachmentSize".to_string())
            .or_insert_with(|| Value::from(0));
        stream_obj
            .entry("Language".to_string())
            .or_insert_with(|| Value::String(language.clone()));
        stream_obj
            .entry("DisplayLanguage".to_string())
            .or_insert_with(|| Value::String(language.to_ascii_uppercase()));
        stream_obj
            .entry("Codec".to_string())
            .or_insert_with(|| Value::String(String::new()));
        stream_obj
            .entry("DisplayTitle".to_string())
            .or_insert_with(|| Value::String(stream_type.clone()));
        stream_obj
            .entry("AverageFrameRate".to_string())
            .or_insert_with(|| Value::from(0.0f64));
        stream_obj
            .entry("RealFrameRate".to_string())
            .or_insert_with(|| Value::from(0.0f64));
        stream_obj
            .entry("BitDepth".to_string())
            .or_insert_with(|| Value::from(0));
        stream_obj
            .entry("BitRate".to_string())
            .or_insert_with(|| Value::from(0));
        stream_obj
            .entry("Level".to_string())
            .or_insert_with(|| Value::from(0));
        stream_obj
            .entry("Width".to_string())
            .or_insert_with(|| Value::from(0));
        stream_obj
            .entry("Height".to_string())
            .or_insert_with(|| Value::from(0));
        stream_obj
            .entry("Profile".to_string())
            .or_insert_with(|| Value::String(String::new()));
        stream_obj
            .entry("PixelFormat".to_string())
            .or_insert_with(|| Value::String(String::new()));
        stream_obj
            .entry("VideoRange".to_string())
            .or_insert_with(|| Value::String(String::new()));
        stream_obj
            .entry("ExtendedVideoType".to_string())
            .or_insert_with(|| Value::String(String::new()));
        stream_obj
            .entry("ExtendedVideoSubType".to_string())
            .or_insert_with(|| Value::String(String::new()));
        stream_obj
            .entry("ExtendedVideoSubTypeDescription".to_string())
            .or_insert_with(|| Value::String(String::new()));
        stream_obj
            .entry("RefFrames".to_string())
            .or_insert_with(|| Value::from(0));
        stream_obj
            .entry("TimeBase".to_string())
            .or_insert_with(|| Value::String("1/1000".to_string()));
        stream_obj
            .entry("IsDefault".to_string())
            .or_insert_with(|| Value::Bool(false));
        stream_obj
            .entry("IsForced".to_string())
            .or_insert_with(|| Value::Bool(false));

        let width = stream_obj.get("Width").and_then(value_to_i64).unwrap_or(0);
        let height = stream_obj.get("Height").and_then(value_to_i64).unwrap_or(0);
        let aspect_ratio = if width > 0 && height > 0 {
            format!("{}:{}", width, height)
        } else {
            "0:0".to_string()
        };
        stream_obj
            .entry("AspectRatio".to_string())
            .or_insert_with(|| Value::String(aspect_ratio));
    }
}

fn ensure_episode_media_source_defaults(item: &mut serde_json::Map<String, Value>) {
    let item_id = item
        .get("Id")
        .and_then(Value::as_str)
        .map(str::to_string);
    let item_name = item
        .get("Name")
        .and_then(Value::as_str)
        .map(str::to_string);
    let chapters_fallback = item
        .get("Chapters")
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()));
    let item_runtime_ticks = item.get("RunTimeTicks").and_then(value_to_i64).unwrap_or(0);

    let Some(media_sources) = item.get_mut("MediaSources").and_then(Value::as_array_mut) else {
        return;
    };

    for source in media_sources.iter_mut() {
        let Some(source_obj) = source.as_object_mut() else {
            continue;
        };

        let source_protocol = source_obj
            .get("Protocol")
            .and_then(Value::as_str)
            .unwrap_or("File")
            .to_string();
        let source_path = source_obj
            .get("Path")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let path_is_strm = source_path
            .rsplit('.')
            .next()
            .map(|ext| ext.eq_ignore_ascii_case("strm"))
            .unwrap_or(false);
        let is_remote = source_protocol.eq_ignore_ascii_case("Http")
            || source_protocol.eq_ignore_ascii_case("Https")
            || source_path.starts_with("http://")
            || source_path.starts_with("https://")
            || path_is_strm;

        if let Some(item_id) = item_id.as_ref() {
            source_obj
                .entry("ItemId".to_string())
                .or_insert_with(|| Value::String(item_id.clone()));
        }
        if let Some(item_name) = item_name.as_ref() {
            source_obj
                .entry("Name".to_string())
                .or_insert_with(|| Value::String(item_name.clone()));
        }

        if is_remote && !source_obj.contains_key("DirectStreamUrl") {
            let sid = source_obj
                .get("Id")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let stream_item_id = item_id.as_deref().unwrap_or(sid);
            let stream_url = format!(
                "/Videos/{}/stream?Static=true&MediaSourceId={}",
                stream_item_id, sid
            );
            source_obj.insert(
                "DirectStreamUrl".to_string(),
                Value::String(stream_url),
            );
        }
        if is_remote
            && let Some(stream_url) = source_obj
                .get("DirectStreamUrl")
                .and_then(Value::as_str)
                .map(str::to_string)
        {
            source_obj.insert("Path".to_string(), Value::String(stream_url));
        }

        source_obj
            .entry("Type".to_string())
            .or_insert_with(|| Value::String("Default".to_string()));
        source_obj
            .entry("AddApiKeyToDirectStreamUrl".to_string())
            .or_insert_with(|| Value::Bool(false));
        source_obj
            .entry("IsRemote".to_string())
            .or_insert_with(|| Value::Bool(is_remote));
        source_obj
            .entry("IsInfiniteStream".to_string())
            .or_insert_with(|| Value::Bool(false));
        source_obj
            .entry("HasMixedProtocols".to_string())
            .or_insert_with(|| Value::Bool(false));
        source_obj
            .entry("ReadAtNativeFramerate".to_string())
            .or_insert_with(|| Value::Bool(false));
        source_obj
            .entry("RequiresClosing".to_string())
            .or_insert_with(|| Value::Bool(false));
        source_obj
            .entry("RequiresLooping".to_string())
            .or_insert_with(|| Value::Bool(false));
        source_obj
            .entry("RequiresOpening".to_string())
            .or_insert_with(|| Value::Bool(false));
        source_obj
            .entry("SupportsProbing".to_string())
            .or_insert_with(|| Value::Bool(true));
        source_obj
            .entry("RequiredHttpHeaders".to_string())
            .or_insert_with(|| Value::Object(serde_json::Map::new()));
        source_obj
            .entry("Formats".to_string())
            .or_insert_with(|| Value::Array(Vec::new()));
        source_obj
            .entry("Chapters".to_string())
            .or_insert_with(|| chapters_fallback.clone());
        if let Some(chapters) = source_obj.get_mut("Chapters").and_then(Value::as_array_mut) {
            ensure_chapter_defaults(chapters);
        }

        if source_obj
            .get("Bitrate")
            .map(Value::is_null)
            .unwrap_or(true)
        {
            source_obj.insert("Bitrate".to_string(), Value::from(0));
        }
        if source_obj
            .get("RunTimeTicks")
            .map(Value::is_null)
            .unwrap_or(true)
        {
            source_obj.insert("RunTimeTicks".to_string(), Value::from(item_runtime_ticks));
        }

        if !matches!(source_obj.get("MediaStreams"), Some(Value::Array(_))) {
            source_obj.insert("MediaStreams".to_string(), Value::Array(Vec::new()));
        }

        let has_default_audio_stream_index = source_obj.contains_key("DefaultAudioStreamIndex");
        let inferred_default_audio_stream_index = if let Some(media_streams) = source_obj
            .get_mut("MediaStreams")
            .and_then(Value::as_array_mut)
        {
            if media_streams.is_empty() {
                media_streams.push(json!({
                    "Index": 0,
                    "Type": "Video",
                    "IsExternal": false,
                }));
            }
            ensure_episode_media_stream_defaults(media_streams, &source_protocol);
            if has_default_audio_stream_index {
                None
            } else {
                infer_default_audio_stream_index(media_streams)
            }
        } else {
            None
        };

        if !has_default_audio_stream_index
            && let Some(index) = inferred_default_audio_stream_index
        {
            source_obj.insert("DefaultAudioStreamIndex".to_string(), Value::from(index));
        }
    }
}

fn ensure_vidhub_video_detail_defaults(item: &mut serde_json::Map<String, Value>) {
    let is_episode = item_is_episode(item);
    let is_movie = item_is_movie(item);
    if !is_episode && !is_movie {
        return;
    }

    ensure_episode_media_source_defaults(item);
    ensure_top_level_media_streams(item);
    ensure_top_level_chapters(item);

    item.entry("BackdropImageTags".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    item.entry("ExternalUrls".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    item.entry("RemoteTrailers".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    ensure_tag_items_from_tags(item);
    item.entry("Taglines".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    item.entry("LockedFields".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    item.entry("LockData".to_string())
        .or_insert_with(|| Value::Bool(false));
    item.entry("LocalTrailerCount".to_string())
        .or_insert_with(|| Value::from(0));
    item.entry("ParentBackdropImageTags".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    item.entry("Chapters".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if is_movie {
        item.entry("ProductionLocations".to_string())
            .or_insert_with(|| Value::Array(Vec::new()));
        item.entry("OfficialRating".to_string())
            .or_insert_with(|| Value::String(String::new()));
        item.entry("PrimaryImageAspectRatio".to_string())
            .or_insert_with(|| Value::from(2.0f64 / 3.0f64));
        if !item.contains_key("OriginalTitle")
            && let Some(name) = item.get("Name").and_then(Value::as_str)
        {
            item.insert("OriginalTitle".to_string(), Value::String(name.to_string()));
        }
    }

    if !item.contains_key("PartCount") {
        let part_count = item
            .get("MediaSources")
            .and_then(Value::as_array)
            .map(|sources| sources.len())
            .unwrap_or(1)
            .max(1);
        item.insert("PartCount".to_string(), Value::from(part_count));
    }

    if !item.contains_key("DateModified")
        && let Some(date_created) = item.get("DateCreated").and_then(Value::as_str)
    {
        item.insert(
            "DateModified".to_string(),
            Value::String(date_created.to_string()),
        );
    }
    if !item.contains_key("DisplayPreferencesId")
        && let Some(item_id) = item.get("Id").and_then(Value::as_str)
    {
        item.insert(
            "DisplayPreferencesId".to_string(),
            Value::String(item_id.to_string()),
        );
    }
    if !item.contains_key("PresentationUniqueKey")
        && let Some(item_id) = item.get("Id").and_then(Value::as_str)
    {
        item.insert(
            "PresentationUniqueKey".to_string(),
            Value::String(item_id.to_string()),
        );
    }
    if !item.contains_key("Etag")
        && let Some(item_id) = item.get("Id").and_then(Value::as_str)
    {
        item.insert(
            "Etag".to_string(),
            Value::String(format!("{:032x}", stable_compat_numeric_id(item_id))),
        );
    }

    if !item.contains_key("FileName")
        && let Some(path) = item.get("Path").and_then(Value::as_str)
        && let Some(file_name) = infer_file_name_from_path(path)
    {
        item.insert("FileName".to_string(), Value::String(file_name));
    }
    if !item.contains_key("ForcedSortName") {
        let forced = item
            .get("SortName")
            .and_then(Value::as_str)
            .or_else(|| item.get("Name").and_then(Value::as_str))
            .unwrap_or_default()
            .to_string();
        item.insert("ForcedSortName".to_string(), Value::String(forced));
    }

    if !item.contains_key("GenreItems")
        && let Some(genres) = item.get("Genres").and_then(Value::as_array)
    {
        let genre_items = genres
            .iter()
            .enumerate()
            .filter_map(|(idx, genre)| {
                genre.as_str().map(|name| {
                    let mut object = serde_json::Map::new();
                    object.insert(
                        "Id".to_string(),
                        Value::from(stable_compat_numeric_id(&format!("genre:{idx}:{name}"))),
                    );
                    object.insert("Name".to_string(), Value::String(name.to_string()));
                    Value::Object(object)
                })
            })
            .collect::<Vec<_>>();
        item.insert("GenreItems".to_string(), Value::Array(genre_items));
    }

    if let Some(genre_items) = item.get_mut("GenreItems").and_then(Value::as_array_mut) {
        ensure_name_id_objects_with_fallback(genre_items, "genre");
    }
    if let Some(studios) = item.get_mut("Studios").and_then(Value::as_array_mut) {
        ensure_name_id_objects_with_fallback(studios, "studio");
    }

    if let Some((width, height)) = infer_video_dimensions(item) {
        item.entry("Width".to_string())
            .or_insert_with(|| Value::from(width));
        item.entry("Height".to_string())
            .or_insert_with(|| Value::from(height));
        if !item.contains_key("PrimaryImageAspectRatio") {
            item.insert(
                "PrimaryImageAspectRatio".to_string(),
                Value::from(width as f64 / height as f64),
            );
        }
    }

    if is_movie
        && item
            .get("Bitrate")
            .map(Value::is_null)
            .unwrap_or(true)
    {
        let bitrate = item
            .get("MediaSources")
            .and_then(Value::as_array)
            .and_then(|sources| {
                sources
                    .iter()
                    .filter_map(Value::as_object)
                    .find_map(|source| source.get("Bitrate").and_then(value_to_i64))
            })
            .unwrap_or(0);
        item.insert("Bitrate".to_string(), Value::from(bitrate));
    }

    if is_episode
        && let Some(series_id) = item
            .get("SeriesId")
            .and_then(Value::as_str)
            .map(str::to_string)
    {
        item.entry("ParentBackdropItemId".to_string())
            .or_insert_with(|| Value::String(series_id.clone()));
        item.entry("ParentLogoItemId".to_string())
            .or_insert_with(|| Value::String(series_id));
    }
}

fn normalize_community_rating_type(item: &mut serde_json::Map<String, Value>) {
    let Some(value) = item.get("CommunityRating").cloned() else {
        return;
    };

    let normalized = match value {
        Value::Number(number) => {
            if number.as_i64().is_some() || number.as_u64().is_some() {
                return;
            }
            let Some(raw) = number.as_f64() else {
                return;
            };
            if !raw.is_finite() {
                return;
            }
            let rounded = raw.round();
            if (raw - rounded).abs() <= f64::EPSILON
                && rounded >= i64::MIN as f64
                && rounded <= i64::MAX as f64
            {
                Some(Value::from(rounded as i64))
            } else {
                None
            }
        }
        Value::String(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                return;
            }
            if let Ok(value) = trimmed.parse::<i64>() {
                Some(Value::from(value))
            } else if let Ok(value) = trimmed.parse::<f64>() {
                if !value.is_finite() {
                    None
                } else {
                    let rounded = value.round();
                    if (value - rounded).abs() <= f64::EPSILON
                        && rounded >= i64::MIN as f64
                        && rounded <= i64::MAX as f64
                    {
                        Some(Value::from(rounded as i64))
                    } else {
                        Some(Value::from(value))
                    }
                }
            } else {
                None
            }
        }
        _ => None,
    };

    if let Some(normalized) = normalized {
        item.insert("CommunityRating".to_string(), normalized);
    }
}

fn derive_series_status(item: &serde_json::Map<String, Value>) -> Option<&'static str> {
    let item_type = item.get("Type").and_then(Value::as_str)?;
    if !item_type.eq_ignore_ascii_case("Series") {
        return None;
    }

    if item.get("EndDate").is_some() {
        Some("Ended")
    } else {
        Some("Continuing")
    }
}

fn compat_item_json_object(
    item: BaseItemDto,
    requested_fields: &std::collections::BTreeSet<String>,
    server_id: &str,
    should_apply_compact_filter: bool,
    include_series_defaults: bool,
) -> serde_json::Map<String, Value> {
    let mut object = match serde_json::to_value(item) {
        Ok(Value::Object(object)) => object,
        Ok(_) | Err(_) => serde_json::Map::new(),
    };

    normalize_item_datetime_fields(&mut object);
    ensure_runtime_ticks_default(&mut object);
    ensure_compat_user_data(&mut object);
    ensure_compat_common_item_defaults(&mut object);
    object.insert("ServerId".to_string(), Value::String(server_id.to_string()));
    object.insert("SupportsSync".to_string(), Value::Bool(true));

    if (include_series_defaults || item_field_requested(requested_fields, "status"))
        && let Some(status) = derive_series_status(&object)
    {
        object
            .entry("Status".to_string())
            .or_insert_with(|| Value::String(status.to_string()));
    }
    if derive_series_status(&object).is_some()
        && (include_series_defaults
            || item_field_requested(requested_fields, "airdays")
            || item_field_requested(requested_fields, "status"))
    {
        object
            .entry("AirDays".to_string())
            .or_insert_with(|| Value::Array(Vec::new()));
    }
    if include_series_defaults {
        ensure_vidhub_series_detail_defaults(&mut object);
        ensure_vidhub_video_detail_defaults(&mut object);
    }
    ensure_size_defaults(&mut object);
    normalize_community_rating_type(&mut object);
    ensure_top_level_media_streams_if_requested(&mut object, requested_fields);

    if should_apply_compact_filter {
        apply_compact_items_field_filter(&mut object, requested_fields);
    }

    object
}

fn compat_single_item_json(item: BaseItemDto, server_id: &str) -> Value {
    let requested_fields = std::collections::BTreeSet::new();
    Value::Object(compat_item_json_object(
        item,
        &requested_fields,
        server_id,
        false,
        true,
    ))
}

fn apply_item_external_subtitle_delivery_urls(
    payload: &mut Value,
    access_token: Option<&str>,
    default_item_id: Option<&str>,
) {
    let Some(item) = payload.as_object_mut() else {
        return;
    };
    let item_id = item
        .get("Id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            default_item_id
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
        .map(str::to_string);
    let Some(item_id) = item_id else {
        return;
    };

    let sole_source_id = item
        .get("MediaSources")
        .and_then(Value::as_array)
        .filter(|sources| sources.len() == 1)
        .and_then(|sources| sources.first())
        .and_then(Value::as_object)
        .and_then(|source| source.get("Id"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    if let Some(media_sources) = item.get_mut("MediaSources").and_then(Value::as_array_mut) {
        for source in media_sources.iter_mut() {
            let Some(source_obj) = source.as_object_mut() else {
                continue;
            };
            let source_id = source_obj
                .get("Id")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or_default()
                .to_string();
            let Some(streams) = source_obj.get_mut("MediaStreams").and_then(Value::as_array_mut) else {
                continue;
            };
            for stream in streams.iter_mut() {
                let Some(stream_obj) = stream.as_object_mut() else {
                    continue;
                };
                let is_subtitle = stream_obj
                    .get("Type")
                    .and_then(Value::as_str)
                    .is_some_and(|value| value.eq_ignore_ascii_case("Subtitle"));
                let is_external = stream_obj
                    .get("IsExternal")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                if !is_subtitle || !is_external || stream_obj.contains_key("DeliveryUrl") {
                    continue;
                }
                let Some(stream_index) = stream_obj
                    .get("Index")
                    .and_then(value_to_i64)
                    .and_then(|value| i32::try_from(value).ok())
                else {
                    continue;
                };
                let codec = stream_obj.get("Codec").and_then(Value::as_str);
                let delivery_url = build_subtitle_delivery_url(
                    &item_id,
                    &source_id,
                    stream_index,
                    codec,
                    access_token,
                );
                stream_obj.insert(
                    "DeliveryUrl".to_string(),
                    Value::String(delivery_url),
                );
            }
        }
    }

    if let Some(source_id) = sole_source_id.as_deref()
        && let Some(streams) = item.get_mut("MediaStreams").and_then(Value::as_array_mut)
    {
        for stream in streams.iter_mut() {
            let Some(stream_obj) = stream.as_object_mut() else {
                continue;
            };
            let is_subtitle = stream_obj
                .get("Type")
                .and_then(Value::as_str)
                .is_some_and(|value| value.eq_ignore_ascii_case("Subtitle"));
            let is_external = stream_obj
                .get("IsExternal")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if !is_subtitle || !is_external || stream_obj.contains_key("DeliveryUrl") {
                continue;
            }
            let Some(stream_index) = stream_obj
                .get("Index")
                .and_then(value_to_i64)
                .and_then(|value| i32::try_from(value).ok())
            else {
                continue;
            };
            let codec = stream_obj.get("Codec").and_then(Value::as_str);
            let delivery_url = build_subtitle_delivery_url(
                &item_id,
                source_id,
                stream_index,
                codec,
                access_token,
            );
            stream_obj.insert(
                "DeliveryUrl".to_string(),
                Value::String(delivery_url),
            );
        }
    }
}

fn payload_item_type_is(payload: &Value, expected: &str) -> bool {
    payload
        .as_object()
        .and_then(|obj| obj.get("Type"))
        .and_then(Value::as_str)
        .map(|item_type| item_type.eq_ignore_ascii_case(expected))
        .unwrap_or(false)
}

fn payload_series_id(payload: &Value) -> Option<Uuid> {
    payload
        .as_object()
        .and_then(|obj| obj.get("SeriesId"))
        .and_then(Value::as_str)
        .and_then(|raw| Uuid::parse_str(raw).ok())
}

fn apply_episode_series_context(payload: &mut Value, series: &BaseItemDto) {
    if !payload_item_type_is(payload, "Episode") {
        return;
    }
    let Some(item) = payload.as_object_mut() else {
        return;
    };

    item.entry("ParentBackdropItemId".to_string())
        .or_insert_with(|| Value::String(series.id.clone()));
    item.entry("ParentLogoItemId".to_string())
        .or_insert_with(|| Value::String(series.id.clone()));

    if !series.name.trim().is_empty() {
        item.entry("SeriesName".to_string())
            .or_insert_with(|| Value::String(series.name.clone()));
    }

    if let Some(backdrop_tags) = series.backdrop_image_tags.as_ref()
        && !backdrop_tags.is_empty()
    {
        item.entry("ParentBackdropImageTags".to_string())
            .or_insert_with(|| {
                Value::Array(
                    backdrop_tags
                        .iter()
                        .cloned()
                        .map(Value::String)
                        .collect::<Vec<_>>(),
                )
            });
    }

    if let Some(image_tags) = series.image_tags.as_ref() {
        if let Some(primary_tag) = image_tags.get("Primary") {
            item.entry("SeriesPrimaryImageTag".to_string())
                .or_insert_with(|| Value::String(primary_tag.clone()));
        }
        if let Some(logo_tag) = image_tags.get("Logo") {
            item.entry("ParentLogoImageTag".to_string())
                .or_insert_with(|| Value::String(logo_tag.clone()));
        }
    }
}

async fn enrich_episode_payload_with_series_context(
    state: &ApiContext,
    user_id: Option<Uuid>,
    payload: &mut Value,
) {
    if !payload_item_type_is(payload, "Episode") {
        return;
    }
    let Some(series_id) = payload_series_id(payload) else {
        return;
    };

    if let Ok(Some(series_item)) = state.infra.get_item(user_id, series_id).await {
        apply_episode_series_context(payload, &series_item);
    }
}

fn compat_items_query_result_json(
    result: QueryResultDto<BaseItemDto>,
    fields: Option<&str>,
    server_id: &str,
    compact_fields_mode: bool,
) -> Value {
    let requested_fields = parse_requested_item_fields(fields);
    let should_apply_compact_filter = compact_fields_mode && !requested_fields.is_empty();

    let items = result
        .items
        .into_iter()
        .map(|item| {
            Value::Object(compat_item_json_object(
                item,
                &requested_fields,
                server_id,
                should_apply_compact_filter,
                false,
            ))
        })
        .collect::<Vec<_>>();

    json!({
        "Items": items,
        "TotalRecordCount": result.total_record_count,
        "StartIndex": result.start_index
    })
}

fn compat_latest_items_json(items: Vec<BaseItemDto>, fields: Option<&str>, server_id: &str) -> Value {
    let requested_fields = parse_requested_item_fields(fields);
    let payload = items
        .into_iter()
        .map(|item| {
            Value::Object(compat_item_json_object(
                item,
                &requested_fields,
                server_id,
                false,
                true,
            ))
        })
        .collect::<Vec<_>>();
    Value::Array(payload)
}

fn ensure_compat_primary_image_tag(item: &mut serde_json::Map<String, Value>) {
    let fallback_seed = item
        .get("Id")
        .and_then(Value::as_str)
        .or_else(|| item.get("Name").and_then(Value::as_str))
        .unwrap_or("ls-item");
    let fallback_primary_tag = format!("{:032x}", stable_compat_numeric_id(fallback_seed));

    if let Some(image_tags) = item.get_mut("ImageTags").and_then(Value::as_object_mut) {
        image_tags
            .entry("Primary".to_string())
            .or_insert_with(|| Value::String(fallback_primary_tag));
        return;
    }

    let mut image_tags = serde_json::Map::new();
    image_tags.insert("Primary".to_string(), Value::String(fallback_primary_tag));
    item.insert("ImageTags".to_string(), Value::Object(image_tags));
}

fn trim_metadata_lookup_item_fields(item: &mut serde_json::Map<String, Value>) {
    for field in [
        "Path",
        "IsFolder",
        "CanDelete",
        "CanDownload",
        "LocationType",
        "DateCreated",
        "ProviderIds",
        "PrimaryImageTag",
        "ChildCount",
    ] {
        item.remove(field);
    }
}

fn compat_metadata_lookup_query_result_json(
    result: QueryResultDto<BaseItemDto>,
    server_id: &str,
) -> Value {
    let items = result
        .items
        .into_iter()
        .map(|item| {
            let mut object = match serde_json::to_value(item) {
                Ok(Value::Object(object)) => object,
                Ok(_) | Err(_) => serde_json::Map::new(),
            };
            normalize_item_datetime_fields(&mut object);
            ensure_compat_user_data(&mut object);
            object.insert("ServerId".to_string(), Value::String(server_id.to_string()));
            object
                .entry("BackdropImageTags".to_string())
                .or_insert_with(|| Value::Array(Vec::new()));
            ensure_compat_primary_image_tag(&mut object);
            trim_metadata_lookup_item_fields(&mut object);
            Value::Object(object)
        })
        .collect::<Vec<_>>();

    json!({
        "Items": items,
        "TotalRecordCount": result.total_record_count
    })
}

fn normalize_person_types(person_types: Option<&str>) -> Vec<String> {
    split_csv(person_types)
        .into_iter()
        .filter_map(|raw| {
            if raw.eq_ignore_ascii_case("actor") {
                return Some("Actor".to_string());
            }
            if raw.eq_ignore_ascii_case("director") {
                return Some("Director".to_string());
            }
            if raw.eq_ignore_ascii_case("writer")
                || raw.eq_ignore_ascii_case("screenplay")
                || raw.eq_ignore_ascii_case("story")
                || raw.eq_ignore_ascii_case("teleplay")
            {
                return Some("Writer".to_string());
            }
            if raw.eq_ignore_ascii_case("producer") {
                return Some("Producer".to_string());
            }
            if raw.eq_ignore_ascii_case("gueststar") || raw.eq_ignore_ascii_case("guest_star") {
                return Some("GuestStar".to_string());
            }
            if raw.eq_ignore_ascii_case("composer") {
                return Some("Composer".to_string());
            }
            if raw.eq_ignore_ascii_case("conductor") {
                return Some("Conductor".to_string());
            }
            if raw.eq_ignore_ascii_case("lyricist") {
                return Some("Lyricist".to_string());
            }
            None
        })
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[derive(Debug, Deserialize)]
struct PersonsQuery {
    #[serde(rename = "SearchTerm", alias = "searchTerm", alias = "search_term")]
    search_term: Option<String>,
    #[serde(rename = "StartIndex", alias = "startIndex", alias = "start_index")]
    start_index: Option<i64>,
    #[serde(rename = "Limit", alias = "limit")]
    limit: Option<i64>,
    #[serde(rename = "Fields", alias = "fields")]
    _fields: Option<String>,
    #[serde(rename = "Filters", alias = "filters")]
    _filters: Option<String>,
    #[serde(rename = "IsFavorite", alias = "isFavorite")]
    _is_favorite: Option<bool>,
    #[serde(rename = "EnableUserData", alias = "enableUserData")]
    _enable_user_data: Option<bool>,
    #[serde(rename = "PersonTypes", alias = "personTypes")]
    person_types: Option<String>,
    #[serde(rename = "AppearsInItemId", alias = "appearsInItemId")]
    appears_in_item_id: Option<String>,
}

async fn get_persons(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<PersonsQuery>,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }

    let start_index = query.start_index.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(100).clamp(1, 500);
    let person_types = normalize_person_types(query.person_types.as_deref());
    let appears_in_item_id =
        match resolve_optional_item_uuid(&state, query.appears_in_item_id.as_deref()).await {
            Ok(value) => value,
            Err(resp) => return resp,
        };

    match state
        .infra
        .list_persons_with_filters(
            query.search_term.as_deref(),
            start_index,
            limit,
            appears_in_item_id,
            &person_types,
        )
        .await
    {
        Ok(items) => Json(compat_metadata_lookup_query_result_json(
            items,
            &state.infra.server_id,
        ))
        .into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to query persons: {err}"),
        ),
    }
}

async fn get_person(
    State(state): State<ApiContext>,
    AxPath(person): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }

    let resolved_person_id = match state.infra.resolve_uuid_by_any_item_id(person.as_str()).await {
        Ok(value) => value,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to resolve person id: {err}"),
            );
        }
    };

    let person_result = if let Some(person_id) =
        person_lookup_id(person.as_str(), resolved_person_id)
    {
        match state.infra.get_person(person_id).await {
            Ok(Some(person)) => Ok(Some(person)),
            Ok(None) => lookup_person_by_name(&state.infra, person.as_str()).await,
            Err(err) => Err(err),
        }
    } else {
        lookup_person_by_name(&state.infra, person.as_str()).await
    };

    match person_result {
        Ok(Some(person)) => Json(person).into_response(),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "person not found"),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to query person: {err}"),
        ),
    }
}

fn person_lookup_id(raw_person: &str, resolved_item_id: Option<Uuid>) -> Option<Uuid> {
    resolved_item_id.or_else(|| Uuid::parse_str(raw_person.trim()).ok())
}

async fn lookup_person_by_name(infra: &AppInfra, person_name: &str) -> anyhow::Result<Option<BaseItemDto>> {
    match infra.list_persons(Some(person_name), 0, 50).await {
        Ok(result) => Ok(find_person_by_name(result.items, person_name)),
        Err(err) => Err(err),
    }
}

fn find_person_by_name(items: Vec<BaseItemDto>, name: &str) -> Option<BaseItemDto> {
    let needle = name.trim();
    if needle.is_empty() {
        return None;
    }

    let needle_lower = needle.to_ascii_lowercase();
    let mut fuzzy_match: Option<BaseItemDto> = None;

    for item in items {
        if item.name.eq_ignore_ascii_case(needle) {
            return Some(item);
        }
        if fuzzy_match.is_none() && item.name.to_ascii_lowercase().contains(&needle_lower) {
            fuzzy_match = Some(item);
        }
    }

    fuzzy_match
}

async fn get_user_items(
    State(state): State<ApiContext>,
    AxPath(user_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<ItemsQuery>,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }

    let parent_id = match resolve_optional_item_uuid(&state, query.parent_id.as_deref()).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    let options = build_items_query_options(Some(user_id), &query, parent_id);

    match state.infra.list_items_with_options(options).await {
        Ok(items) => {
            let compat = apply_items_query_compatibility(items, &query);
            let mut payload = compat_items_query_result_json(
                compat,
                query._fields.as_deref(),
                &state.infra.server_id,
                true,
            );
            if let Err(err) = apply_compat_item_ids_for_response(&state.infra, &mut payload).await {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("failed to normalize item ids: {err}"),
                );
            }
            Json(payload).into_response()
        }
        Err(err) => {
            let status = map_items_query_error(&err);
            error_response(status, &format!("failed to query user items: {err}"))
        }
    }
}

async fn get_user_root_items(
    State(state): State<ApiContext>,
    AxPath(_user_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }

    match state.infra.list_root_items().await {
        Ok(items) => {
            let mut payload = compat_items_query_result_json(items, None, &state.infra.server_id, false);
            if let Err(err) = apply_compat_item_ids_for_response(&state.infra, &mut payload).await {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("failed to normalize item ids: {err}"),
                );
            }
            Json(payload).into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to query root items: {err}"),
        ),
    }
}

async fn get_user_views(
    State(state): State<ApiContext>,
    AxPath(_user_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }

    match state.infra.list_root_items().await {
        Ok(items) => {
            let mut payload = compat_items_query_result_json(items, None, &state.infra.server_id, false);
            if let Err(err) = apply_compat_item_ids_for_response(&state.infra, &mut payload).await {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("failed to normalize item ids: {err}"),
                );
            }
            Json(payload).into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to query views: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct LatestItemsQuery {
    #[serde(rename = "Limit", alias = "limit")]
    limit: Option<i64>,
    #[serde(rename = "ParentId", alias = "parentId")]
    parent_id: Option<String>,
    #[serde(
        rename = "IncludeItemTypes",
        alias = "includeItemTypes",
        alias = "include_item_types"
    )]
    include_item_types: Option<String>,
    #[serde(rename = "IsPlayed", alias = "isPlayed")]
    is_played: Option<bool>,
    #[serde(rename = "Fields", alias = "fields")]
    _fields: Option<String>,
    #[serde(rename = "IsFolder", alias = "isFolder")]
    _is_folder: Option<bool>,
    #[serde(rename = "GroupItems", alias = "groupItems")]
    _group_items: Option<bool>,
    #[serde(rename = "EnableImages", alias = "enableImages")]
    _enable_images: Option<bool>,
    #[serde(rename = "ImageTypeLimit", alias = "imageTypeLimit")]
    _image_type_limit: Option<i32>,
    #[serde(rename = "EnableImageTypes", alias = "enableImageTypes")]
    _enable_image_types: Option<String>,
    #[serde(rename = "EnableUserData", alias = "enableUserData")]
    _enable_user_data: Option<bool>,
}

fn normalize_latest_items_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(20).clamp(1, 200)
}

fn infer_latest_item_types_for_library(library_type: &str) -> Vec<String> {
    let normalized = library_type.trim().to_ascii_lowercase();
    if matches!(normalized.as_str(), "movie" | "movies") {
        return vec!["Movie".to_string()];
    }
    if matches!(normalized.as_str(), "series" | "show" | "shows" | "tv" | "tvshows") {
        return vec!["Series".to_string()];
    }
    if matches!(normalized.as_str(), "musicvideo" | "musicvideos") {
        return vec!["MusicVideo".to_string()];
    }
    Vec::new()
}

async fn normalize_latest_include_item_types(
    state: &ApiContext,
    parent_id: Option<Uuid>,
    explicit_types: Vec<String>,
) -> Vec<String> {
    if !explicit_types.is_empty() {
        return explicit_types;
    }
    let Some(parent_id) = parent_id else {
        return explicit_types;
    };

    match state.infra.get_library_by_id(parent_id).await {
        Ok(Some(library)) => infer_latest_item_types_for_library(&library.library_type),
        Ok(None) | Err(_) => explicit_types,
    }
}

async fn get_user_latest_items(
    State(state): State<ApiContext>,
    AxPath(user_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<LatestItemsQuery>,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }

    let parent_id = match resolve_optional_item_uuid(&state, query.parent_id.as_deref()).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    let include_item_types = normalize_latest_include_item_types(
        &state,
        parent_id,
        split_csv(query.include_item_types.as_deref()),
    )
    .await;
    let limit = normalize_latest_items_limit(query.limit);

    match state
        .infra
        .list_latest_items_for_user(
            user_id,
            parent_id,
            include_item_types,
            query.is_played,
            limit,
        )
        .await
    {
        Ok(items) => {
            let mut payload = compat_latest_items_json(items, query._fields.as_deref(), &state.infra.server_id);
            if let Err(err) = apply_compat_item_ids_for_response(&state.infra, &mut payload).await {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("failed to normalize item ids: {err}"),
                );
            }
            Json(payload).into_response()
        }
        Err(err) => {
            let status = map_items_query_error(&err);
            error_response(status, &format!("failed to query latest user items: {err}"))
        }
    }
}

async fn get_user_resume_items(
    State(state): State<ApiContext>,
    AxPath(user_id): AxPath<Uuid>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<ResumeItemsQuery>,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }

    let parent_id = match resolve_optional_item_uuid(&state, query.parent_id.as_deref()).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };

    match state.infra.list_resume_items(user_id).await {
        Ok(items) => {
            let compat = apply_resume_query_compatibility(items, &query, parent_id);
            let mut payload = compat_items_query_result_json(
                compat,
                query._fields.as_deref(),
                &state.infra.server_id,
                true,
            );
            if let Err(err) = apply_compat_item_ids_for_response(&state.infra, &mut payload).await {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("failed to normalize item ids: {err}"),
                );
            }
            Json(payload).into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to query resume items: {err}"),
        ),
    }
}

fn find_root_collection_item(
    root_items: QueryResultDto<BaseItemDto>,
    item_id: Uuid,
) -> Option<BaseItemDto> {
    let target = item_id.to_string();
    root_items.items.into_iter().find(|item| item.id == target)
}

async fn get_user_item(
    State(state): State<ApiContext>,
    AxPath((user_id, raw_item_id)): AxPath<(Uuid, String)>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };

    let item_result = match state.infra.get_item(Some(user_id), item_id).await {
        Ok(Some(item)) => Ok(Some(item)),
        Ok(None) => state
            .infra
            .list_root_items()
            .await
            .map(|root_items| find_root_collection_item(root_items, item_id)),
        Err(err) => Err(err),
    };

    match item_result {
        Ok(Some(item)) => {
            let mut payload = compat_single_item_json(item, &state.infra.server_id);
            enrich_episode_payload_with_series_context(&state, Some(user_id), &mut payload).await;
            let token = extract_token(&headers, &uri);
            apply_item_external_subtitle_delivery_urls(
                &mut payload,
                token.as_deref(),
                Some(raw_item_id.as_str()),
            );
            if let Err(err) = apply_compat_item_ids_for_response(&state.infra, &mut payload).await {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("failed to normalize item ids: {err}"),
                );
            }
            Json(payload).into_response()
        }
        Ok(None) => error_response(StatusCode::NOT_FOUND, "item not found"),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to get item: {err}"),
        ),
    }
}

/// GET /Items/{itemId} - Get current user's view of an item
async fn get_item(
    State(state): State<ApiContext>,
    AxPath(raw_item_id): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(user) => user,
        Err(resp) => return resp,
    };
    let user_id = parse_user_uuid(&auth_user);
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };

    let item_result = match state.infra.get_item(user_id, item_id).await {
        Ok(Some(item)) => Ok(Some(item)),
        Ok(None) => state
            .infra
            .list_root_items()
            .await
            .map(|root_items| find_root_collection_item(root_items, item_id)),
        Err(err) => Err(err),
    };

    match item_result {
        Ok(Some(item)) => {
            let mut payload = compat_single_item_json(item, &state.infra.server_id);
            enrich_episode_payload_with_series_context(&state, user_id, &mut payload).await;
            let token = extract_token(&headers, &uri);
            apply_item_external_subtitle_delivery_urls(
                &mut payload,
                token.as_deref(),
                Some(raw_item_id.as_str()),
            );
            if let Err(err) = apply_compat_item_ids_for_response(&state.infra, &mut payload).await {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("failed to normalize item ids: {err}"),
                );
            }
            Json(payload).into_response()
        }
        Ok(None) => error_response(StatusCode::NOT_FOUND, "item not found"),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to get item: {err}"),
        ),
    }
}

/// GET /Users/{userId}/Items/{itemId}/UserData - Get user data for an item
async fn get_user_item_data(
    State(state): State<ApiContext>,
    AxPath((user_id, raw_item_id)): AxPath<(Uuid, String)>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };

    match state.infra.get_user_item_data(user_id, item_id).await {
        Ok(mut data) => {
            let mut cache = HashMap::new();
            if let Some(compat_id) =
                maybe_compat_item_id_string(&state.infra, &data.item_id, &mut cache)
                    .await
                    .ok()
                    .flatten()
            {
                data.item_id = compat_id;
            }
            Json(data).into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to get user item data: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct UserItemDataUpdateBody {
    #[serde(rename = "Played", alias = "played")]
    played: Option<bool>,
    #[serde(rename = "PlaybackPositionTicks", alias = "playbackPositionTicks")]
    playback_position_ticks: Option<i64>,
    #[serde(rename = "IsFavorite", alias = "isFavorite")]
    is_favorite: Option<bool>,
    #[serde(rename = "PlayCount", alias = "playCount")]
    play_count: Option<i64>,
    #[serde(rename = "PlayedPercentage", alias = "playedPercentage")]
    played_percentage: Option<f64>,
    #[serde(rename = "LastPlayedDate", alias = "lastPlayedDate")]
    last_played_date: Option<String>,
}

impl UserItemDataUpdateBody {
    fn effective_played(&self) -> Option<bool> {
        if self.played.is_some() {
            return self.played;
        }
        if let Some(play_count) = self.play_count {
            return Some(play_count > 0);
        }
        if let Some(played_percentage) = self.played_percentage {
            if played_percentage >= 100.0 {
                return Some(true);
            }
            if played_percentage <= 0.0 {
                return Some(false);
            }
        }
        if self
            .last_played_date
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
        {
            return Some(true);
        }
        None
    }

    fn to_update(&self) -> ls_infra::UserItemDataUpdate {
        ls_infra::UserItemDataUpdate {
            played: self.effective_played(),
            playback_position_ticks: self.playback_position_ticks,
            is_favorite: self.is_favorite,
        }
    }

    fn is_empty(&self) -> bool {
        self.effective_played().is_none()
            && self.playback_position_ticks.is_none()
            && self.is_favorite.is_none()
    }
}

#[derive(Debug, Deserialize, Default)]
struct HideFromResumeQuery {
    #[serde(rename = "Hide", alias = "hide")]
    hide: Option<bool>,
}

impl HideFromResumeQuery {
    fn should_hide(&self) -> bool {
        self.hide.unwrap_or(true)
    }
}

/// POST /UserItems/{itemId}/UserData - Update current user's item userdata
async fn post_user_item_data(
    State(state): State<ApiContext>,
    AxPath(raw_item_id): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<UserItemDataUpdateBody>,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(user) => user,
        Err(resp) => return resp,
    };
    let Some(user_id) = parse_user_uuid(&auth_user) else {
        return error_response(StatusCode::UNAUTHORIZED, "invalid user");
    };
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    if payload.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "at least one of Played, IsFavorite, PlaybackPositionTicks is required",
        );
    }

    let update = payload.to_update();

    match state.infra.update_user_item_data(user_id, item_id, update).await {
        Ok(mut data) => {
            let mut cache = HashMap::new();
            if let Some(compat_id) =
                maybe_compat_item_id_string(&state.infra, &data.item_id, &mut cache)
                    .await
                    .ok()
                    .flatten()
            {
                data.item_id = compat_id;
            }
            Json(data).into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to update user item data: {err}"),
        ),
    }
}

/// POST /Users/{userId}/Items/{itemId}/HideFromResume - Toggle resume visibility
async fn post_user_item_hide_from_resume(
    State(state): State<ApiContext>,
    AxPath((user_id, raw_item_id)): AxPath<(Uuid, String)>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<HideFromResumeQuery>,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };

    let result = if query.should_hide() {
        state
            .infra
            .update_user_item_data(
                user_id,
                item_id,
                ls_infra::UserItemDataUpdate {
                    played: None,
                    playback_position_ticks: Some(0),
                    is_favorite: None,
                },
            )
            .await
    } else {
        state.infra.get_user_item_data(user_id, item_id).await
    };

    match result {
        Ok(mut data) => {
            let mut cache = HashMap::new();
            if let Some(compat_id) =
                maybe_compat_item_id_string(&state.infra, &data.item_id, &mut cache)
                    .await
                    .ok()
                    .flatten()
            {
                data.item_id = compat_id;
            }
            Json(data).into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to update hide-from-resume state: {err}"),
        ),
    }
}

/// POST /Users/{userId}/PlayedItems/{itemId} - Mark item as played
async fn mark_item_played(
    State(state): State<ApiContext>,
    AxPath((user_id, raw_item_id)): AxPath<(Uuid, String)>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };

    match state.infra.mark_played(user_id, item_id).await {
        Ok(mut data) => {
            let mut cache = HashMap::new();
            if let Some(compat_id) =
                maybe_compat_item_id_string(&state.infra, &data.item_id, &mut cache)
                    .await
                    .ok()
                    .flatten()
            {
                data.item_id = compat_id;
            }
            Json(data).into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to mark item played: {err}"),
        ),
    }
}

/// DELETE /Users/{userId}/PlayedItems/{itemId} - Mark item as unplayed
async fn mark_item_unplayed(
    State(state): State<ApiContext>,
    AxPath((user_id, raw_item_id)): AxPath<(Uuid, String)>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };

    match state.infra.mark_unplayed(user_id, item_id).await {
        Ok(mut data) => {
            let mut cache = HashMap::new();
            if let Some(compat_id) =
                maybe_compat_item_id_string(&state.infra, &data.item_id, &mut cache)
                    .await
                    .ok()
                    .flatten()
            {
                data.item_id = compat_id;
            }
            Json(data).into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to mark item unplayed: {err}"),
        ),
    }
}

fn parse_legacy_played_action(action: &str) -> Option<bool> {
    match action.trim().to_ascii_lowercase().as_str() {
        "add" => Some(true),
        "delete" => Some(false),
        _ => None,
    }
}

/// POST /Users/{userId}/PlayedItems/{itemId}/{action} - Legacy Emby played toggle
async fn post_item_played_legacy_action(
    State(state): State<ApiContext>,
    AxPath((user_id, raw_item_id, action)): AxPath<(Uuid, String, String)>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };

    let Some(mark_played) = parse_legacy_played_action(&action) else {
        return error_response(
            StatusCode::NOT_FOUND,
            "unsupported played action, expected Add/Delete",
        );
    };

    let result = if mark_played {
        state.infra.mark_played(user_id, item_id).await
    } else {
        state.infra.mark_unplayed(user_id, item_id).await
    };

    match result {
        Ok(mut data) => {
            let mut cache = HashMap::new();
            if let Some(compat_id) =
                maybe_compat_item_id_string(&state.infra, &data.item_id, &mut cache)
                    .await
                    .ok()
                    .flatten()
            {
                data.item_id = compat_id;
            }
            Json(data).into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to update played state: {err}"),
        ),
    }
}

/// POST /Users/{userId}/FavoriteItems/{itemId} - Add item to favorites
async fn add_item_favorite(
    State(state): State<ApiContext>,
    AxPath((user_id, raw_item_id)): AxPath<(Uuid, String)>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_self_or_admin(&auth_user, user_id) {
        return resp;
    }
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };

    match state.infra.set_favorite(user_id, item_id, true).await {
        Ok(mut data) => {
            let mut cache = HashMap::new();
            if let Some(compat_id) =
                maybe_compat_item_id_string(&state.infra, &data.item_id, &mut cache)
                    .await
                    .ok()
                    .flatten()
            {
                data.item_id = compat_id;
            }
            Json(data).into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to add favorite: {err}"),
        ),
    }
}

fn parse_legacy_favorite_action(action: &str) -> Option<bool> {
    match action.trim().to_ascii_lowercase().as_str() {
        "add" => Some(true),
        "delete" => Some(false),
        _ => None,
    }
}

/// POST /Users/{userId}/FavoriteItems/{itemId}/{action} - Legacy Emby favorite toggle
async fn post_item_favorite_legacy_action(
    State(state): State<ApiContext>,
    AxPath((user_id, raw_item_id, action)): AxPath<(Uuid, String, String)>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_self_or_admin(&auth_user, user_id) {
        return resp;
    }
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };

    let Some(is_favorite) = parse_legacy_favorite_action(&action) else {
        return error_response(
            StatusCode::NOT_FOUND,
            "unsupported favorite action, expected Add/Delete",
        );
    };

    match state.infra.set_favorite(user_id, item_id, is_favorite).await {
        Ok(mut data) => {
            let mut cache = HashMap::new();
            if let Some(compat_id) =
                maybe_compat_item_id_string(&state.infra, &data.item_id, &mut cache)
                    .await
                    .ok()
                    .flatten()
            {
                data.item_id = compat_id;
            }
            Json(data).into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to update favorite: {err}"),
        ),
    }
}

/// DELETE /Users/{userId}/FavoriteItems/{itemId} - Remove item from favorites
async fn remove_item_favorite(
    State(state): State<ApiContext>,
    AxPath((user_id, raw_item_id)): AxPath<(Uuid, String)>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_self_or_admin(&auth_user, user_id) {
        return resp;
    }
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };

    match state.infra.set_favorite(user_id, item_id, false).await {
        Ok(mut data) => {
            let mut cache = HashMap::new();
            if let Some(compat_id) =
                maybe_compat_item_id_string(&state.infra, &data.item_id, &mut cache)
                    .await
                    .ok()
                    .flatten()
            {
                data.item_id = compat_id;
            }
            Json(data).into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to remove favorite: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct ItemCountsQuery {
    #[serde(rename = "UserId", alias = "userId")]
    user_id: Option<Uuid>,
    #[serde(rename = "IsFavorite", alias = "isFavorite")]
    is_favorite: Option<bool>,
}

async fn get_item_counts(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<ItemCountsQuery>,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(user) => user,
        Err(resp) => return resp,
    };
    let user_id = query
        .user_id
        .or_else(|| Uuid::parse_str(&auth_user.id).ok());

    match state.infra.item_counts(user_id, query.is_favorite).await {
        Ok(counts) => Json(counts).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to query counts: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct SearchHintsQuery {
    #[serde(rename = "StartIndex", alias = "startIndex", alias = "start_index")]
    start_index: Option<i64>,
    #[serde(rename = "Limit", alias = "limit")]
    limit: Option<i64>,
    #[serde(rename = "UserId", alias = "userId", alias = "user_id")]
    _user_id: Option<Uuid>,
    #[serde(rename = "SearchTerm", alias = "searchTerm", alias = "search_term")]
    search_term: Option<String>,
    #[serde(
        rename = "IncludeItemTypes",
        alias = "includeItemTypes",
        alias = "include_item_types"
    )]
    include_item_types: Option<String>,
    #[serde(
        rename = "ExcludeItemTypes",
        alias = "excludeItemTypes",
        alias = "exclude_item_types"
    )]
    exclude_item_types: Option<String>,
    #[serde(rename = "MediaTypes", alias = "mediaTypes", alias = "media_types")]
    media_types: Option<String>,
    #[serde(rename = "ParentId", alias = "parentId", alias = "parent_id")]
    parent_id: Option<String>,
    #[serde(rename = "IsMovie", alias = "isMovie", alias = "is_movie")]
    is_movie: Option<bool>,
    #[serde(rename = "IsSeries", alias = "isSeries", alias = "is_series")]
    is_series: Option<bool>,
    #[serde(rename = "IsNews", alias = "isNews", alias = "is_news")]
    is_news: Option<bool>,
    #[serde(rename = "IsKids", alias = "isKids", alias = "is_kids")]
    is_kids: Option<bool>,
    #[serde(rename = "IsSports", alias = "isSports", alias = "is_sports")]
    is_sports: Option<bool>,
    #[serde(
        rename = "IncludePeople",
        alias = "includePeople",
        alias = "include_people"
    )]
    include_people: Option<bool>,
    #[serde(
        rename = "IncludeMedia",
        alias = "includeMedia",
        alias = "include_media"
    )]
    include_media: Option<bool>,
    #[serde(
        rename = "IncludeGenres",
        alias = "includeGenres",
        alias = "include_genres"
    )]
    include_genres: Option<bool>,
    #[serde(
        rename = "IncludeStudios",
        alias = "includeStudios",
        alias = "include_studios"
    )]
    include_studios: Option<bool>,
    #[serde(
        rename = "IncludeArtists",
        alias = "includeArtists",
        alias = "include_artists"
    )]
    include_artists: Option<bool>,
}

/// GET /Search/Hints - Quick search suggestions
async fn get_search_hints(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<SearchHintsQuery>,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }

    let search_term = match query
        .search_term
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(term) => term.to_string(),
        None => return error_response(StatusCode::BAD_REQUEST, "SearchTerm is required"),
    };

    let start_index = normalize_search_hints_start_index(query.start_index);
    let page_limit = normalize_search_hints_limit(query.limit);
    let request_limit = (start_index + page_limit).clamp(1, 500);
    let include_media = query.include_media.unwrap_or(true);
    let include_people = query.include_people.unwrap_or(true);
    let include_genres = query.include_genres.unwrap_or(true);
    let include_studios = query.include_studios.unwrap_or(true);
    let include_artists = query.include_artists.unwrap_or(true);
    let include_item_types = normalize_search_hints_include_item_types(&query);
    let exclude_item_types = split_csv(query.exclude_item_types.as_deref());
    let media_types = split_csv(query.media_types.as_deref());
    let parent_id = match resolve_optional_item_uuid(&state, query.parent_id.as_deref()).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };

    match state
        .infra
        .search_hints(
            &search_term,
            request_limit,
            &include_item_types,
            parent_id,
        )
        .await
    {
        Ok(mut result) => {
            let mut filtered = result
                .search_hints
                .into_iter()
                .filter(|hint| {
                    if !include_people && hint.item_type.eq_ignore_ascii_case("Person") {
                        return false;
                    }
                    if !include_media && !hint.item_type.eq_ignore_ascii_case("Person") {
                        return false;
                    }
                    if !exclude_item_types.is_empty()
                        && exclude_item_types
                            .iter()
                            .any(|raw| raw.eq_ignore_ascii_case(&hint.item_type))
                    {
                        return false;
                    }
                    if !media_types.is_empty()
                        && !resume_media_type_matches(&hint.item_type, &media_types)
                    {
                        return false;
                    }
                    if !include_genres && hint.item_type.eq_ignore_ascii_case("Genre") {
                        return false;
                    }
                    if !include_studios && hint.item_type.eq_ignore_ascii_case("Studio") {
                        return false;
                    }
                    if !include_artists
                        && (hint.item_type.eq_ignore_ascii_case("Artist")
                            || hint.item_type.eq_ignore_ascii_case("MusicArtist")
                            || hint.item_type.eq_ignore_ascii_case("AlbumArtist"))
                    {
                        return false;
                    }
                    true
                })
                .collect::<Vec<_>>();
            let total_record_count = filtered.len() as i32;
            filtered = filtered
                .into_iter()
                .skip(start_index as usize)
                .take(page_limit as usize)
                .collect();
            result.search_hints = filtered;
            result.total_record_count = total_record_count;
            Json(result).into_response()
        }
        Err(err) => {
            let status = map_items_query_error(&err);
            error_response(status, &format!("failed to search: {err}"))
        }
    }
}

fn normalize_search_hints_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(20).clamp(1, 100)
}

fn normalize_search_hints_start_index(start_index: Option<i64>) -> i64 {
    start_index.unwrap_or(0).max(0)
}

fn normalize_search_hints_include_item_types(query: &SearchHintsQuery) -> Vec<String> {
    let mut include_item_types = split_csv(query.include_item_types.as_deref());
    if query.is_movie.unwrap_or(false) {
        include_item_types.push("Movie".to_string());
    }
    if query.is_series.unwrap_or(false) {
        include_item_types.push("Series".to_string());
    }
    if query.is_news.unwrap_or(false) {
        include_item_types.push("Program".to_string());
    }
    if query.is_kids.unwrap_or(false) {
        include_item_types.push("Program".to_string());
    }
    if query.is_sports.unwrap_or(false) {
        include_item_types.push("Sports".to_string());
        include_item_types.push("Program".to_string());
    }
    include_item_types.sort_unstable_by_key(|v| v.to_ascii_lowercase());
    include_item_types.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    include_item_types
}

#[derive(Debug, Deserialize)]
struct ItemsFiltersQuery {
    #[serde(rename = "UserId", alias = "userId", alias = "user_id")]
    _user_id: Option<Uuid>,
    #[serde(rename = "ParentId", alias = "parentId", alias = "parent_id")]
    parent_id: Option<String>,
    #[serde(
        rename = "IncludeItemTypes",
        alias = "includeItemTypes",
        alias = "include_item_types"
    )]
    include_item_types: Option<String>,
    #[serde(rename = "MediaTypes", alias = "mediaTypes", alias = "media_types")]
    media_types: Option<String>,
    #[serde(
        rename = "ExcludeItemTypes",
        alias = "excludeItemTypes",
        alias = "exclude_item_types"
    )]
    _exclude_item_types: Option<String>,
    #[serde(rename = "Recursive", alias = "recursive")]
    _recursive: Option<bool>,
    #[serde(rename = "SortBy", alias = "sortBy")]
    _sort_by: Option<String>,
    #[serde(rename = "SortOrder", alias = "sortOrder")]
    _sort_order: Option<String>,
    #[serde(rename = "EnableImages", alias = "enableImages")]
    _enable_images: Option<bool>,
    #[serde(rename = "ImageTypeLimit", alias = "imageTypeLimit")]
    _image_type_limit: Option<i32>,
    #[serde(rename = "EnableImageTypes", alias = "enableImageTypes")]
    _enable_image_types: Option<String>,
    #[serde(rename = "Fields", alias = "fields")]
    _fields: Option<String>,
}

/// GET /Items/Filters - Available filter values for a library
async fn get_items_filters(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<ItemsFiltersQuery>,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }

    let include_item_types = filter_item_types_by_media_types(
        split_csv(query.include_item_types.as_deref()),
        &split_csv(query.media_types.as_deref()),
    );
    let parent_id = match resolve_optional_item_uuid(&state, query.parent_id.as_deref()).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };

    match state
        .infra
        .get_item_filters(parent_id, false, &include_item_types)
        .await
    {
        Ok(filters) => Json(filters).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to get filters: {err}"),
        ),
    }
}

fn filter_item_types_by_media_types(
    include_item_types: Vec<String>,
    media_types: &[String],
) -> Vec<String> {
    if media_types.is_empty() {
        return include_item_types;
    }

    const CANDIDATES: &[&str] = &[
        "Movie",
        "Series",
        "Episode",
        "Video",
        "MusicVideo",
        "Trailer",
        "Audio",
        "Song",
        "MusicAlbum",
        "MusicArtist",
        "AudioBook",
        "Photo",
        "PhotoAlbum",
        "Book",
    ];

    if include_item_types.is_empty() {
        return CANDIDATES
            .iter()
            .filter(|item_type| resume_media_type_matches(item_type, media_types))
            .map(|item_type| (*item_type).to_string())
            .collect();
    }

    include_item_types
        .into_iter()
        .filter(|item_type| resume_media_type_matches(item_type, media_types))
        .collect()
}

#[derive(Debug, Deserialize)]
struct GenresQuery {
    #[serde(rename = "ParentId", alias = "parentId", alias = "parent_id")]
    parent_id: Option<String>,
    #[serde(
        rename = "IncludeItemTypes",
        alias = "includeItemTypes",
        alias = "include_item_types"
    )]
    include_item_types: Option<String>,
    #[serde(rename = "StartIndex", alias = "startIndex", alias = "start_index")]
    start_index: Option<i64>,
    #[serde(rename = "Limit", alias = "limit")]
    limit: Option<i64>,
    #[serde(rename = "Recursive", alias = "recursive")]
    recursive: Option<bool>,
    #[serde(
        rename = "ExcludeItemTypes",
        alias = "excludeItemTypes",
        alias = "exclude_item_types"
    )]
    _exclude_item_types: Option<String>,
    #[serde(rename = "IsFavorite", alias = "isFavorite")]
    _is_favorite: Option<bool>,
    #[serde(rename = "Fields", alias = "fields")]
    _fields: Option<String>,
    #[serde(rename = "SortBy", alias = "sortBy")]
    _sort_by: Option<String>,
    #[serde(rename = "SortOrder", alias = "sortOrder")]
    _sort_order: Option<String>,
    #[serde(rename = "ImageTypeLimit", alias = "imageTypeLimit")]
    _image_type_limit: Option<i32>,
    #[serde(rename = "EnableImageTypes", alias = "enableImageTypes")]
    _enable_image_types: Option<String>,
}
