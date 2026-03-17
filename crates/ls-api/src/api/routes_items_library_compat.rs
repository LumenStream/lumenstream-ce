#[derive(Debug, Deserialize)]
struct DeleteItemsQuery {
    #[serde(rename = "Ids", alias = "ids")]
    ids: Option<String>,
}

async fn delete_items_bulk(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<DeleteItemsQuery>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    let Some(raw_ids) = query.ids.as_deref() else {
        return error_response(StatusCode::BAD_REQUEST, "Ids is required");
    };
    let mut ids = Vec::<Uuid>::new();
    for raw in split_csv(Some(raw_ids)) {
        match state.infra.resolve_uuid_by_any_item_id(raw.as_str()).await {
            Ok(Some(item_id)) => ids.push(item_id),
            Ok(None) => {}
            Err(err) => {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("failed to resolve item id: {err}"),
                );
            }
        }
    }
    if ids.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "Ids is required");
    }

    match state.infra.delete_items_bulk(&ids).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to delete items: {err}"),
        ),
    }
}

async fn get_items_filters2(
    state: State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    query: Query<ItemsFiltersQuery>,
) -> Response {
    get_items_filters(state, headers, uri, query).await
}

#[derive(Debug, Deserialize)]
struct ItemFileQueryCompat {
    #[serde(rename = "Id", alias = "id", alias = "ItemId", alias = "itemId")]
    item_id: Option<String>,
    #[serde(rename = "Path", alias = "path")]
    _path: Option<String>,
}

fn item_stream_redirect(item_id: Uuid) -> Response {
    HttpResponse::Found()
        .insert_header((header::LOCATION, format!("/Videos/{item_id}/stream")))
        .finish()
}

fn payload_field<'a>(payload: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    let object = payload.as_object()?;
    keys.iter().find_map(|key| object.get(*key))
}

fn payload_trimmed_string(payload: &Value, keys: &[&str]) -> Option<String> {
    payload_field(payload, keys)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToString::to_string)
}

fn payload_i32(payload: &Value, keys: &[&str]) -> Option<i32> {
    let value = payload_field(payload, keys)?;
    if let Some(v) = value.as_i64() {
        return i32::try_from(v).ok();
    }
    if let Some(v) = value.as_u64() {
        return i32::try_from(v).ok();
    }
    value
        .as_str()
        .and_then(|raw| raw.trim().parse::<i64>().ok())
        .and_then(|v| i32::try_from(v).ok())
}

fn payload_f64(payload: &Value, keys: &[&str]) -> Option<f64> {
    let value = payload_field(payload, keys)?;
    if let Some(v) = value.as_f64() {
        return Some(v);
    }
    if let Some(v) = value.as_i64() {
        return Some(v as f64);
    }
    value.as_str().and_then(|raw| raw.trim().parse::<f64>().ok())
}

fn payload_string_or_number(payload: &Value, keys: &[&str]) -> Option<String> {
    let value = payload_field(payload, keys)?;
    value
        .as_str()
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        .map(ToString::to_string)
        .or_else(|| value.as_i64().map(|raw| raw.to_string()))
        .or_else(|| value.as_u64().map(|raw| raw.to_string()))
}

fn normalize_provider_id_key(key: &str) -> String {
    if key.eq_ignore_ascii_case("tmdb") {
        return "Tmdb".to_string();
    }
    if key.eq_ignore_ascii_case("imdb") {
        return "Imdb".to_string();
    }
    if key.eq_ignore_ascii_case("tvdb") {
        return "Tvdb".to_string();
    }
    if key.eq_ignore_ascii_case("anidb") {
        return "AniDB".to_string();
    }
    key.to_string()
}

fn payload_string_map(payload: &Value, keys: &[&str]) -> Option<HashMap<String, String>> {
    let object = payload_field(payload, keys)?.as_object()?;
    let provider_ids = object
        .iter()
        .filter_map(|(key, value)| {
            let value = value
                .as_str()
                .map(str::to_string)
                .or_else(|| value.as_i64().map(|v| v.to_string()))
                .or_else(|| value.as_u64().map(|v| v.to_string()));
            value
                .map(|raw| raw.trim().to_string())
                .filter(|raw| !raw.is_empty())
                .map(|raw| (normalize_provider_id_key(key), raw))
        })
        .collect::<HashMap<_, _>>();
    if provider_ids.is_empty() {
        return None;
    }
    Some(provider_ids)
}

fn payload_string_list(payload: &Value, keys: &[&str]) -> Option<Vec<String>> {
    let values = payload_field(payload, keys)?
        .as_array()?
        .iter()
        .filter_map(|value| value.as_str().map(str::trim))
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if values.is_empty() {
        return None;
    }
    Some(values)
}

fn metadata_patch_from_payload(payload: &Value) -> Option<Value> {
    let mut patch = serde_json::Map::<String, Value>::new();

    if let Some(value) = payload_trimmed_string(payload, &["Overview", "overview"]) {
        patch.insert("overview".to_string(), Value::String(value));
    }
    if let Some(value) = payload_trimmed_string(payload, &["PremiereDate", "premiereDate"]) {
        patch.insert("premiere_date".to_string(), Value::String(value));
    }
    if let Some(value) = payload_i32(payload, &["ProductionYear", "productionYear", "Year"]) {
        patch.insert("production_year".to_string(), json!(value));
    }
    if let Some(value) = payload_i32(payload, &["IndexNumber", "indexNumber"]) {
        patch.insert("index_number".to_string(), json!(value));
    }
    if let Some(value) = payload_i32(payload, &["ParentIndexNumber", "parentIndexNumber"]) {
        patch.insert("parent_index_number".to_string(), json!(value));
    }
    if let Some(value) = payload_trimmed_string(payload, &["OfficialRating", "officialRating"]) {
        patch.insert("official_rating".to_string(), Value::String(value));
    }
    if let Some(value) = payload_f64(payload, &["CommunityRating", "communityRating"]) {
        patch.insert("community_rating".to_string(), json!(value));
    }
    if let Some(value) = payload_trimmed_string(payload, &["SortName", "sortName"]) {
        patch.insert("sort_name".to_string(), Value::String(value));
    }
    if let Some(value) = payload_f64(payload, &["PrimaryImageAspectRatio", "primaryImageAspectRatio"])
    {
        patch.insert("primary_image_aspect_ratio".to_string(), json!(value));
    }
    if let Some(values) = payload_string_list(payload, &["Genres", "genres"]) {
        patch.insert("genres".to_string(), json!(values));
    }
    let mut provider_ids = payload_string_map(payload, &["ProviderIds", "providerIds"]).unwrap_or_default();
    let has_explicit_tmdb_id_field =
        payload_field(payload, &["TmdbId", "tmdbId", "tmdb_id"]).is_some();

    let tmdb_raw = payload_string_or_number(payload, &["TmdbId", "tmdbId", "tmdb_id"]).or_else(|| {
        provider_ids
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case("Tmdb"))
            .map(|(_, value)| value.clone())
    });
    if let Some(tmdb_raw) = tmdb_raw {
        if let Ok(tmdb_id) = tmdb_raw.trim().parse::<i64>() {
            if tmdb_id > 0 {
                patch.insert("tmdb_id".to_string(), json!(tmdb_id));
                provider_ids.insert("Tmdb".to_string(), tmdb_id.to_string());
                if has_explicit_tmdb_id_field {
                    patch.insert(
                        "tmdb_binding_source".to_string(),
                        Value::String("manual".to_string()),
                    );
                }
            }
        }
    }

    let imdb_id = payload_string_or_number(payload, &["ImdbId", "imdbId", "imdb_id"]).or_else(|| {
        provider_ids
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case("Imdb"))
            .map(|(_, value)| value.clone())
    });
    if let Some(imdb_id) = imdb_id {
        let imdb_id = imdb_id.trim();
        if !imdb_id.is_empty() {
            patch.insert("imdb_id".to_string(), Value::String(imdb_id.to_string()));
            provider_ids.insert("Imdb".to_string(), imdb_id.to_string());
        }
    }

    if !provider_ids.is_empty() {
        patch.insert("provider_ids".to_string(), json!(provider_ids));
    }

    if patch.is_empty() {
        None
    } else {
        Some(Value::Object(patch))
    }
}

fn provider_display_name(key: &str) -> String {
    if key.eq_ignore_ascii_case("Tmdb") {
        return "The Movie Database".to_string();
    }
    if key.eq_ignore_ascii_case("Imdb") {
        return "IMDb".to_string();
    }
    if key.eq_ignore_ascii_case("Tvdb") {
        return "TheTVDB".to_string();
    }
    if key.eq_ignore_ascii_case("AniDB") {
        return "AniDB".to_string();
    }
    key.to_string()
}

fn provider_url_format(key: &str) -> Option<String> {
    if key.eq_ignore_ascii_case("Tmdb") {
        return Some("https://www.themoviedb.org/search?query={0}".to_string());
    }
    if key.eq_ignore_ascii_case("Imdb") {
        return Some("https://www.imdb.com/title/{0}".to_string());
    }
    if key.eq_ignore_ascii_case("Tvdb") {
        return Some("https://thetvdb.com/?id={0}&tab=series".to_string());
    }
    None
}

fn default_external_id_provider_keys(item_type: &str) -> &'static [&'static str] {
    if item_type.eq_ignore_ascii_case("Movie") {
        return &["Tmdb", "Imdb"];
    }
    if item_type.eq_ignore_ascii_case("Series")
        || item_type.eq_ignore_ascii_case("Season")
        || item_type.eq_ignore_ascii_case("Episode")
    {
        return &["Tvdb", "Tmdb", "Imdb"];
    }
    &[]
}

fn remote_search_name_from_payload(payload: &Value) -> Option<String> {
    payload_trimmed_string(payload, &["Name", "name"]).or_else(|| {
        payload
            .get("SearchInfo")
            .and_then(|value| payload_trimmed_string(value, &["Name", "name"]))
    })
}

fn remote_search_year_from_payload(payload: &Value) -> Option<i32> {
    payload_i32(payload, &["ProductionYear", "productionYear", "Year", "year"]).or_else(|| {
        payload
            .get("SearchInfo")
            .and_then(|value| payload_i32(value, &["ProductionYear", "productionYear", "Year", "year"]))
    })
}

fn remote_search_provider_ids_from_payload(payload: &Value) -> Option<HashMap<String, String>> {
    payload_string_map(payload, &["ProviderIds", "providerIds"]).or_else(|| {
        payload
            .get("SearchInfo")
            .and_then(|value| payload_string_map(value, &["ProviderIds", "providerIds"]))
    })
}

fn remote_search_include_item_types(search_kind: &str) -> Vec<String> {
    match search_kind {
        "movie" => vec!["Movie".to_string()],
        "series" => vec!["Series".to_string()],
        "season" => vec!["Season".to_string()],
        "episode" => vec!["Episode".to_string()],
        "book" => vec!["Book".to_string()],
        "game" => vec!["Game".to_string()],
        "trailer" => vec!["Trailer".to_string(), "Video".to_string(), "Movie".to_string()],
        "musicvideo" => vec!["MusicVideo".to_string(), "Video".to_string()],
        "musicalbum" => vec!["MusicAlbum".to_string()],
        "boxset" => vec!["BoxSet".to_string(), "CollectionFolder".to_string()],
        _ => Vec::new(),
    }
}

async fn get_items_file(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<ItemFileQueryCompat>,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let Some(raw_item_id) = query.item_id.as_deref() else {
        return error_response(StatusCode::BAD_REQUEST, "missing Id");
    };
    let item_id = match resolve_item_uuid_or_bad_request(&state, raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    let user_id = parse_user_uuid(&auth_user);
    match state.infra.get_item(user_id, item_id).await {
        Ok(Some(_)) => item_stream_redirect(item_id),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "item not found"),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to get item file: {err}"),
        ),
    }
}

async fn post_item(
    State(state): State<ApiContext>,
    AxPath(raw_item_id): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<Value>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };

    let name = payload
        .as_object()
        .and_then(|obj| obj.get("Name").or_else(|| obj.get("name")))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    let patch = metadata_patch_from_payload(&payload);

    let updated = match state
        .infra
        .patch_item_metadata(item_id, name.as_deref(), patch.as_ref())
        .await
    {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to update item metadata: {err}"),
            );
        }
    };
    if !updated {
        return error_response(StatusCode::NOT_FOUND, "item not found");
    }

    StatusCode::NO_CONTENT.into_response()
}

async fn delete_item_compat(
    State(state): State<ApiContext>,
    AxPath(raw_item_id): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };

    match state.infra.delete_item(item_id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => error_response(StatusCode::NOT_FOUND, "item not found"),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to delete item: {err}"),
        ),
    }
}

async fn get_item_prefixes(
    State(state): State<ApiContext>,
    Query(query): Query<ItemsQuery>,
) -> Response {
    let parent_id = match resolve_optional_item_uuid(&state, query.parent_id.as_deref()).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    let options = build_items_query_options(query.user_id, &query, parent_id);
    match state.infra.list_item_name_prefixes(options).await {
        Ok(prefixes) => Json(
            prefixes
                .into_iter()
                .map(|value| json!({ "Name": value, "Value": value }))
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to query item prefixes: {err}"),
        ),
    }
}

async fn get_item_external_id_infos(
    State(state): State<ApiContext>,
    AxPath(raw_item_id): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };

    let item = match state.infra.get_item(None, item_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "item not found"),
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to query item external ids: {err}"),
            );
        }
    };

    let mut provider_keys = item
        .provider_ids
        .unwrap_or_default()
        .into_keys()
        .collect::<std::collections::BTreeSet<_>>();
    for key in default_external_id_provider_keys(&item.item_type) {
        provider_keys.insert((*key).to_string());
    }

    let mut infos = provider_keys
        .into_iter()
        .map(|key| {
            let name = provider_display_name(&key);
            let url_format = provider_url_format(&key);
            json!({
                "Name": name,
                "Key": key,
                "UrlFormatString": url_format,
            })
        })
        .collect::<Vec<_>>();
    infos.sort_by(|a, b| {
        let key_a = a.get("Key").and_then(Value::as_str).unwrap_or_default();
        let key_b = b.get("Key").and_then(Value::as_str).unwrap_or_default();
        key_a.cmp(key_b)
    });

    Json(infos).into_response()
}

#[derive(Debug, Deserialize)]
struct ItemRefreshQuery {
    #[serde(rename = "Recursive", alias = "recursive")]
    recursive: Option<bool>,
    #[serde(rename = "MetadataRefreshMode", alias = "metadataRefreshMode")]
    metadata_refresh_mode: Option<String>,
    #[serde(rename = "ImageRefreshMode", alias = "imageRefreshMode")]
    image_refresh_mode: Option<String>,
    #[serde(rename = "ReplaceAllMetadata", alias = "replaceAllMetadata")]
    replace_all_metadata: Option<bool>,
    #[serde(rename = "ReplaceAllImages", alias = "replaceAllImages")]
    replace_all_images: Option<bool>,
}

async fn post_item_refresh(
    State(state): State<ApiContext>,
    AxPath(raw_item_id): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<ItemRefreshQuery>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };

    let metadata_mode_none = query
        .metadata_refresh_mode
        .as_deref()
        .is_some_and(|v| v.eq_ignore_ascii_case("none"));
    let image_mode_none = query
        .image_refresh_mode
        .as_deref()
        .is_some_and(|v| v.eq_ignore_ascii_case("none"));

    let refresh_metadata = query.replace_all_metadata.unwrap_or(false) || !metadata_mode_none;
    let refresh_images =
        query.replace_all_images.unwrap_or(false) || query.recursive.unwrap_or(false) || !image_mode_none;

    let refresh_metadata_now = refresh_metadata || (!refresh_metadata && !refresh_images);
    if refresh_metadata_now || refresh_images {
        match state
            .infra
            .rescrape_item_metadata(item_id, refresh_images)
            .await
        {
            Ok(true) => {}
            Ok(false) => return error_response(StatusCode::NOT_FOUND, "item not found"),
            Err(err) => {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("failed to refresh item metadata: {err}"),
                );
            }
        }
    }
    StatusCode::NO_CONTENT.into_response()
}

async fn get_item_metadata_editor(
    state: State<ApiContext>,
    AxPath(raw_item_id): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    get_item(state, AxPath(raw_item_id), headers, uri).await
}

#[derive(Debug, Deserialize)]
struct RelatedItemsQuery {
    #[serde(rename = "UserId", alias = "userId")]
    user_id: Option<Uuid>,
    #[serde(rename = "Limit", alias = "limit")]
    limit: Option<i64>,
    #[serde(rename = "Fields", alias = "fields")]
    _fields: Option<String>,
    #[serde(rename = "IncludeItemTypes", alias = "includeItemTypes")]
    include_item_types: Option<String>,
}

fn normalize_related_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(24).clamp(1, 200)
}

async fn collect_related_items(
    state: &ApiContext,
    item_id: Uuid,
    user_id: Option<Uuid>,
    include_item_types: Vec<String>,
    limit: i64,
    sort_random: bool,
) -> anyhow::Result<Vec<BaseItemDto>> {
    let item = match state.infra.get_item(user_id, item_id).await? {
        Some(v) => v,
        None => return Ok(Vec::new()),
    };

    let mut contexts = Vec::<(bool, Uuid)>::new();
    if let Some(series_id) = parse_compat_uuid(item.series_id.as_deref()) {
        contexts.push((true, series_id));
    }
    if let Some(parent_id) = parse_compat_uuid(item.parent_id.as_deref()) {
        contexts.push((false, parent_id));
    }
    if let Some(library_id) = state.infra.item_library_id(item_id).await? {
        if !contexts.iter().any(|(_, id)| *id == library_id) {
            contexts.push((false, library_id));
        }
    }

    let mut seen = std::collections::BTreeSet::new();
    let mut output = Vec::<BaseItemDto>::new();
    for (is_series_filter, context_id) in contexts {
        let options = InfraItemsQuery {
            user_id,
            series_filter: is_series_filter.then_some(context_id),
            parent_id: (!is_series_filter).then_some(context_id),
            include_item_types: include_item_types.clone(),
            exclude_item_types: Vec::new(),
            person_ids: Vec::new(),
            search_term: None,
            limit: (limit + 8).clamp(1, 256),
            start_index: 0,
            is_resumable: false,
            sort_by: if sort_random {
                vec!["Random".to_string()]
            } else {
                vec!["CommunityRating".to_string(), "DateCreated".to_string()]
            },
            sort_order: "Descending".to_string(),
            recursive: true,
            genres: item.genres.clone().unwrap_or_default(),
            tags: item.tags.clone().unwrap_or_default(),
            years: Vec::new(),
            is_favorite: None,
            is_played: None,
            min_community_rating: None,
        };

        let result = state.infra.list_items_with_options(options).await?;
        for candidate in result.items {
            if candidate.id.eq_ignore_ascii_case(&item.id) {
                continue;
            }
            if !seen.insert(candidate.id.clone()) {
                continue;
            }
            output.push(candidate);
            if output.len() >= limit as usize {
                return Ok(output);
            }
        }
    }

    Ok(output)
}

async fn get_item_instant_mix(
    State(state): State<ApiContext>,
    AxPath(raw_item_id): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<RelatedItemsQuery>,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let user_id = query.user_id.or_else(|| parse_user_uuid(&auth_user));
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    let include_item_types = split_csv(query.include_item_types.as_deref());
    let limit = normalize_related_limit(query.limit);

    match collect_related_items(&state, item_id, user_id, include_item_types, limit, true).await {
        Ok(items) => {
            let mut payload = serde_json::to_value(QueryResultDto {
                total_record_count: items.len() as i32,
                start_index: 0,
                items,
            })
            .unwrap_or_else(|_| json!({"Items": [], "TotalRecordCount": 0, "StartIndex": 0}));
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
            &format!("failed to query instant mix: {err}"),
        ),
    }
}

async fn get_item_similar(
    State(state): State<ApiContext>,
    AxPath(raw_item_id): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<RelatedItemsQuery>,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let user_id = query.user_id.or_else(|| parse_user_uuid(&auth_user));
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    let include_item_types = split_csv(query.include_item_types.as_deref());
    let limit = normalize_related_limit(query.limit);

    match collect_related_items(&state, item_id, user_id, include_item_types, limit, false).await {
        Ok(items) => {
            let mut payload = serde_json::to_value(QueryResultDto {
                total_record_count: items.len() as i32,
                start_index: 0,
                items,
            })
            .unwrap_or_else(|_| json!({"Items": [], "TotalRecordCount": 0, "StartIndex": 0}));
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
            &format!("failed to query similar items: {err}"),
        ),
    }
}

async fn get_item_delete_info(
    State(state): State<ApiContext>,
    AxPath(raw_item_id): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let user_id = parse_user_uuid(&auth_user);
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    match state.infra.get_item(user_id, item_id).await {
        Ok(Some(item)) => {
            let mut paths = Vec::<String>::new();
            if !item.path.trim().is_empty() {
                paths.push(item.path);
            }
            Json(json!({ "Paths": paths })).into_response()
        }
        Ok(None) => error_response(StatusCode::NOT_FOUND, "item not found"),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to query delete info: {err}"),
        ),
    }
}

async fn get_item_download(
    State(state): State<ApiContext>,
    AxPath(raw_item_id): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    match state.infra.get_item(parse_user_uuid(&auth_user), item_id).await {
        Ok(Some(_)) => item_stream_redirect(item_id),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "item not found"),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to get download item: {err}"),
        ),
    }
}

async fn get_item_file_by_id(
    State(state): State<ApiContext>,
    AxPath(raw_item_id): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    match state.infra.get_item(parse_user_uuid(&auth_user), item_id).await {
        Ok(Some(_)) => item_stream_redirect(item_id),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "item not found"),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to get item file: {err}"),
        ),
    }
}

async fn get_item_ancestors(
    State(state): State<ApiContext>,
    AxPath(raw_item_id): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<ItemAncestorsQuery>,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let user_id = query
        .user_id
        .or_else(|| Uuid::parse_str(&auth_user.id).ok());
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };

    let item = match state.infra.get_item(user_id, item_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "item not found"),
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to load item ancestors: {err}"),
            );
        }
    };

    let mut ancestors = Vec::<BaseItemDto>::new();
    let mut parent_cursor = item.parent_id;
    while let Some(parent_raw) = parent_cursor {
        let Some(parent_id) = parse_compat_uuid(Some(parent_raw.as_str())) else {
            break;
        };

        match state.infra.get_item(user_id, parent_id).await {
            Ok(Some(parent_item)) => {
                parent_cursor = parent_item.parent_id.clone();
                ancestors.push(parent_item);
                continue;
            }
            Ok(None) => {}
            Err(err) => {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("failed to resolve ancestor item: {err}"),
                );
            }
        }

        match state.infra.get_library_by_id(parent_id).await {
            Ok(Some(library)) => ancestors.push(library_to_base_item_dto(&library)),
            Ok(None) => {}
            Err(err) => {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("failed to resolve ancestor library: {err}"),
                );
            }
        }
        break;
    }

    ancestors.reverse();
    let mut payload = serde_json::to_value(ancestors).unwrap_or_else(|_| Value::Array(Vec::new()));
    if let Err(err) = apply_compat_item_ids_for_response(&state.infra, &mut payload).await {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to normalize item ids: {err}"),
        );
    }
    Json(payload).into_response()
}

#[derive(Debug, Deserialize)]
struct ItemAncestorsQuery {
    #[serde(rename = "UserId", alias = "userId")]
    user_id: Option<Uuid>,
}

fn library_to_base_item_dto(library: &ls_domain::model::Library) -> BaseItemDto {
    BaseItemDto {
        id: library.id.to_string(),
        name: library.name.clone(),
        item_type: "CollectionFolder".to_string(),
        path: library.root_path.clone(),
        is_folder: Some(true),
        media_type: None,
        container: None,
        location_type: Some("FileSystem".to_string()),
        can_delete: Some(false),
        can_download: Some(false),
        collection_type: Some(normalize_emby_collection_type(&library.library_type).to_string()),
        runtime_ticks: None,
        bitrate: None,
        media_sources: None,
        user_data: None,
        overview: None,
        premiere_date: None,
        end_date: None,
        production_year: None,
        genres: None,
        tags: None,
        provider_ids: None,
        image_tags: None,
        primary_image_tag: None,
        parent_id: None,
        series_id: None,
        series_name: None,
        season_id: None,
        season_name: None,
        index_number: None,
        parent_index_number: None,
        backdrop_image_tags: None,
        official_rating: None,
        community_rating: None,
        studios: None,
        people: None,
        sort_name: None,
        primary_image_aspect_ratio: None,
        date_created: Some(library.created_at.to_rfc3339()),
        child_count: None,
        recursive_item_count: None,
        play_access: None,
    }
}

fn normalize_emby_collection_type(raw: &str) -> &'static str {
    if raw.eq_ignore_ascii_case("movie") || raw.eq_ignore_ascii_case("movies") {
        return "movies";
    }
    if raw.eq_ignore_ascii_case("series")
        || raw.eq_ignore_ascii_case("show")
        || raw.eq_ignore_ascii_case("shows")
        || raw.eq_ignore_ascii_case("tv")
        || raw.eq_ignore_ascii_case("tvshows")
    {
        return "tvshows";
    }
    if raw.eq_ignore_ascii_case("playlist") || raw.eq_ignore_ascii_case("playlists") {
        return "playlists";
    }
    if raw.eq_ignore_ascii_case("mixed") {
        return "mixed";
    }
    "mixed"
}

#[derive(Debug, Deserialize)]
struct PagingQuery {
    #[serde(rename = "StartIndex", alias = "startIndex")]
    start_index: Option<i64>,
    #[serde(rename = "Limit", alias = "limit")]
    _limit: Option<i64>,
}

fn empty_paged_result(start_index: i64) -> QueryResultDto<Value> {
    QueryResultDto {
        items: Vec::new(),
        total_record_count: 0,
        start_index: start_index.max(0) as i32,
    }
}

async fn get_item_critic_reviews(
    State(state): State<ApiContext>,
    AxPath(_item_id): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<PagingQuery>,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }
    Json(empty_paged_result(query.start_index.unwrap_or(0))).into_response()
}

#[derive(Debug, Deserialize, Default)]
struct ItemThemeQuery {
    #[serde(rename = "UserId", alias = "userId")]
    user_id: Option<Uuid>,
    #[serde(rename = "InheritFromParent", alias = "inheritFromParent")]
    inherit_from_parent: Option<bool>,
}

fn theme_media_result(items: Vec<BaseItemDto>) -> Value {
    let total = items.len();
    json!({
        "OwnerId": 0,
        "Items": items,
        "TotalRecordCount": total,
    })
}

fn theme_keyword_filter(mut items: Vec<BaseItemDto>, keyword: &str, limit: i64) -> Vec<BaseItemDto> {
    let keyword = keyword.trim().to_ascii_lowercase();
    if keyword.is_empty() {
        items.truncate(limit as usize);
        return items;
    }

    let has_keyword_match = items
        .iter()
        .any(|item| item.name.to_ascii_lowercase().contains(&keyword));
    let mut themed = if has_keyword_match {
        items
            .into_iter()
            .filter(|item| item.name.to_ascii_lowercase().contains(&keyword))
            .collect::<Vec<_>>()
    } else {
        std::mem::take(&mut items)
    };
    themed.truncate(limit as usize);
    themed
}

async fn get_item_theme_media(
    State(state): State<ApiContext>,
    AxPath(raw_item_id): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<ItemThemeQuery>,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    let user_id = query.user_id.or_else(|| parse_user_uuid(&auth_user));
    let limit = 24;

    let songs = match collect_related_items(
        &state,
        item_id,
        user_id,
        vec!["Audio".to_string(), "Song".to_string()],
        limit,
        false,
    )
    .await
    {
        Ok(items) => theme_keyword_filter(items, "theme", limit),
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to query theme songs: {err}"),
            );
        }
    };
    let videos = match collect_related_items(
        &state,
        item_id,
        user_id,
        vec!["Video".to_string(), "MusicVideo".to_string(), "Trailer".to_string()],
        limit,
        false,
    )
    .await
    {
        Ok(items) => theme_keyword_filter(items, "theme", limit),
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to query theme videos: {err}"),
            );
        }
    };
    let soundtrack = if query.inherit_from_parent.unwrap_or(true) {
        match collect_related_items(
            &state,
            item_id,
            user_id,
            vec!["Audio".to_string(), "Song".to_string()],
            limit,
            false,
        )
        .await
        {
            Ok(items) => theme_keyword_filter(items, "soundtrack", limit),
            Err(_) => Vec::new(),
        }
    } else {
        Vec::new()
    };

    let mut payload = json!({
        "ThemeVideosResult": theme_media_result(videos),
        "ThemeSongsResult": theme_media_result(songs),
        "SoundtrackSongsResult": theme_media_result(soundtrack),
    });
    if let Err(err) = apply_compat_item_ids_for_response(&state.infra, &mut payload).await {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to normalize item ids: {err}"),
        );
    }
    Json(payload).into_response()
}

async fn get_item_theme_songs(
    State(state): State<ApiContext>,
    AxPath(raw_item_id): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<ItemThemeQuery>,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    let user_id = query.user_id.or_else(|| parse_user_uuid(&auth_user));
    let limit = 24;

    match collect_related_items(
        &state,
        item_id,
        user_id,
        vec!["Audio".to_string(), "Song".to_string()],
        limit,
        false,
    )
    .await
    {
        Ok(items) => {
            let mut payload = theme_media_result(theme_keyword_filter(items, "theme", limit));
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
            &format!("failed to query theme songs: {err}"),
        ),
    }
}

async fn get_item_theme_videos(
    State(state): State<ApiContext>,
    AxPath(raw_item_id): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<ItemThemeQuery>,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    let user_id = query.user_id.or_else(|| parse_user_uuid(&auth_user));
    let limit = 24;

    match collect_related_items(
        &state,
        item_id,
        user_id,
        vec!["Video".to_string(), "MusicVideo".to_string(), "Trailer".to_string()],
        limit,
        false,
    )
    .await
    {
        Ok(items) => {
            let mut payload = theme_media_result(theme_keyword_filter(items, "theme", limit));
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
            &format!("failed to query theme videos: {err}"),
        ),
    }
}

async fn get_item_images_list(
    State(state): State<ApiContext>,
    AxPath(raw_item_id): AxPath<String>,
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

    let mut images = Vec::<Value>::new();
    for image_type in ["Primary", "Backdrop", "Thumb", "Logo", "Art", "Banner"] {
        if let Ok(Some(_)) = state.infra.image_path_for_item(item_id, image_type, 0).await {
            images.push(json!({
                "ImageType": image_type,
                "ImageIndex": 0,
            }));
        }
    }

    Json(images).into_response()
}

#[derive(Debug, Deserialize, Default)]
struct RemoteImagesQuery {
    #[serde(rename = "Type", alias = "type")]
    image_type: Option<String>,
    #[serde(rename = "StartIndex", alias = "startIndex")]
    start_index: Option<i64>,
    #[serde(rename = "Limit", alias = "limit")]
    limit: Option<i64>,
    #[serde(rename = "ProviderName", alias = "providerName")]
    provider_name: Option<String>,
    #[serde(rename = "IncludeAllLanguages", alias = "includeAllLanguages")]
    _include_all_languages: Option<bool>,
}

fn normalize_remote_images_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(50).clamp(1, 500)
}

fn normalize_remote_images_start_index(start_index: Option<i64>) -> i64 {
    start_index.unwrap_or(0).max(0)
}

fn normalize_remote_image_type(raw: Option<&str>) -> Option<String> {
    let raw = raw?.trim();
    if raw.is_empty() {
        return None;
    }
    for image_type in [
        "Primary",
        "Art",
        "Backdrop",
        "Banner",
        "Logo",
        "Thumb",
        "Disc",
        "Box",
        "Screenshot",
        "Menu",
        "Chapter",
        "BoxRear",
        "Thumbnail",
    ] {
        if image_type.eq_ignore_ascii_case(raw) {
            return Some(image_type.to_string());
        }
    }
    None
}

async fn get_item_remote_images(
    State(state): State<ApiContext>,
    AxPath(raw_item_id): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<RemoteImagesQuery>,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    if let Some(provider_name) = query.provider_name.as_deref() {
        if !provider_name.eq_ignore_ascii_case("LocalImageProvider")
            && !provider_name.eq_ignore_ascii_case("Local")
        {
            return Json(json!({
                "Images": [],
                "TotalRecordCount": 0,
                "Providers": [],
            }))
            .into_response();
        }
    }
    match state.infra.get_item(parse_user_uuid(&auth_user), item_id).await {
        Ok(Some(_)) => {}
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "item not found"),
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to query item: {err}"),
            );
        }
    }

    let requested_types = if query.image_type.is_some() {
        match normalize_remote_image_type(query.image_type.as_deref()) {
            Some(v) => vec![v],
            None => return error_response(StatusCode::BAD_REQUEST, "invalid Type"),
        }
    } else {
        vec![
            "Primary".to_string(),
            "Backdrop".to_string(),
            "Thumb".to_string(),
            "Logo".to_string(),
            "Art".to_string(),
            "Banner".to_string(),
        ]
    };

    let mut images = Vec::<Value>::new();
    for image_type in requested_types {
        for idx in 0..64_i32 {
            match state.infra.image_path_for_item(item_id, &image_type, idx).await {
                Ok(Some(_)) => {
                    let url = format!("/Items/{item_id}/Images/{image_type}/{idx}");
                    images.push(json!({
                        "ProviderName": "LocalImageProvider",
                        "Url": url,
                        "ThumbnailUrl": url,
                        "Type": image_type,
                        "RatingType": "Score",
                    }));
                }
                Ok(None) => break,
                Err(_) => break,
            }
        }
    }

    let total_record_count = images.len() as i32;
    let start_index = normalize_remote_images_start_index(query.start_index) as usize;
    let limit = normalize_remote_images_limit(query.limit) as usize;
    let paged = if start_index >= images.len() {
        Vec::new()
    } else {
        images.into_iter().skip(start_index).take(limit).collect::<Vec<_>>()
    };
    Json(json!({
        "Images": paged,
        "TotalRecordCount": total_record_count,
        "Providers": ["LocalImageProvider"],
    }))
    .into_response()
}

#[derive(Debug, Deserialize)]
struct ThumbnailSetQuery {
    #[serde(rename = "Width", alias = "width")]
    width: Option<i32>,
}

async fn get_item_thumbnail_set(
    State(state): State<ApiContext>,
    AxPath(raw_item_id): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<ThumbnailSetQuery>,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    let width = query.width.unwrap_or(320).clamp(16, 4096);
    HttpResponse::Found()
        .insert_header((
            header::LOCATION,
            format!("/Items/{item_id}/Images/Primary?MaxWidth={width}"),
        ))
        .finish()
}

async fn get_item_remote_images_providers(
    State(state): State<ApiContext>,
    AxPath(raw_item_id): AxPath<String>,
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

    match state.infra.get_item(None, item_id).await {
        Ok(Some(_)) => Json(vec![json!({
            "Name": "LocalImageProvider",
            "SupportedImages": [
                "Primary",
                "Art",
                "Backdrop",
                "Banner",
                "Logo",
                "Thumb",
                "Thumbnail"
            ]
        })])
        .into_response(),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "item not found"),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to query remote image providers: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct RemoteImageDownloadQuery {
    #[serde(rename = "Type", alias = "type")]
    image_type: Option<String>,
    #[serde(rename = "ProviderName", alias = "providerName")]
    provider_name: Option<String>,
    #[serde(rename = "ImageUrl", alias = "imageUrl")]
    image_url: Option<String>,
}

async fn download_remote_image(
    state: &ApiContext,
    image_url: &str,
) -> anyhow::Result<(Vec<u8>, String)> {
    let response = state.infra.http_client.get(image_url).send().await?;
    if !response.status().is_success() {
        anyhow::bail!("upstream status {}", response.status());
    }
    let extension = infer_image_extension_from_content_type(
        response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
    )
    .to_string();
    let body = response.bytes().await?.to_vec();
    Ok((body, extension))
}

async fn save_image_for_item_or_library(
    state: &ApiContext,
    item_id: Uuid,
    image_type: &str,
    body: &[u8],
    extension: &str,
) -> anyhow::Result<bool> {
    if state
        .infra
        .save_library_image(item_id, image_type, body, extension)
        .await?
        .is_some()
    {
        return Ok(true);
    }
    if state
        .infra
        .save_media_item_image(item_id, image_type, body, extension)
        .await?
        .is_some()
    {
        return Ok(true);
    }
    Ok(false)
}

async fn post_item_remote_images_download(
    State(state): State<ApiContext>,
    AxPath(raw_item_id): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<RemoteImageDownloadQuery>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };

    let image_type = match normalize_remote_image_type(query.image_type.as_deref()) {
        Some(v) => v,
        None => return error_response(StatusCode::BAD_REQUEST, "Type is required"),
    };
    if let Some(provider_name) = query.provider_name.as_deref() {
        if !provider_name.eq_ignore_ascii_case("LocalImageProvider")
            && !provider_name.eq_ignore_ascii_case("Local")
            && !query.image_url.as_deref().is_some_and(|v| !v.trim().is_empty())
        {
            return error_response(StatusCode::BAD_REQUEST, "ImageUrl is required");
        }
    }

    let Some(image_url) = query
        .image_url
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    else {
        return error_response(StatusCode::BAD_REQUEST, "ImageUrl is required");
    };

    let (body, extension) = match download_remote_image(&state, image_url).await {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::BAD_GATEWAY,
                &format!("failed to download remote image: {err}"),
            );
        }
    };

    match save_image_for_item_or_library(&state, item_id, &image_type, &body, &extension).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => error_response(StatusCode::NOT_FOUND, "item not found"),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to save remote image: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize, Default)]
struct RemoteSearchSubtitleQuery {
    #[serde(rename = "IsPerfectMatch", alias = "isPerfectMatch")]
    is_perfect_match: Option<bool>,
    #[serde(rename = "IsForced", alias = "isForced")]
    is_forced: Option<bool>,
}

fn language_to_iso3(language: &str) -> String {
    let lang = language.trim().to_ascii_lowercase();
    match lang.as_str() {
        "en" => "eng".to_string(),
        "zh" => "zho".to_string(),
        "ja" => "jpn".to_string(),
        "ko" => "kor".to_string(),
        "fr" => "fra".to_string(),
        "de" => "deu".to_string(),
        "es" => "spa".to_string(),
        "it" => "ita".to_string(),
        "ru" => "rus".to_string(),
        "pt" => "por".to_string(),
        "ar" => "ara".to_string(),
        _ if lang.len() == 3 => lang,
        _ => "und".to_string(),
    }
}

fn subtitle_language_matches(track_language: Option<&str>, requested_language: &str) -> bool {
    let requested = requested_language.trim().to_ascii_lowercase();
    if requested.is_empty() {
        return true;
    }
    if requested == "all" || requested == "*" {
        return true;
    }

    let Some(track_language) = track_language.map(str::trim).filter(|v| !v.is_empty()) else {
        return false;
    };
    let track = track_language.to_ascii_lowercase();
    if track == requested {
        return true;
    }
    language_to_iso3(track_language) == requested
}

fn parse_remote_subtitle_track_id(raw: &str) -> Option<i32> {
    let value = raw.trim();
    if value.is_empty() {
        return None;
    }
    value
        .strip_prefix("local-")
        .unwrap_or(value)
        .parse::<i32>()
        .ok()
}

async fn get_item_remote_search_subtitles(
    State(state): State<ApiContext>,
    AxPath((raw_item_id, language)): AxPath<(String, String)>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<RemoteSearchSubtitleQuery>,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };

    let tracks = match state.infra.list_subtitle_tracks(item_id).await {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to query subtitle tracks: {err}"),
            );
        }
    };
    if query.is_forced.unwrap_or(false) {
        return Json(Vec::<Value>::new()).into_response();
    }

    let mut subtitles = tracks
        .into_iter()
        .filter(|track| subtitle_language_matches(track.language.as_deref(), &language))
        .map(|track| {
            let language_raw = track.language.clone().unwrap_or_else(|| language.clone());
            let is_hash_match = subtitle_language_matches(track.language.as_deref(), &language);
            json!({
                "ThreeLetterISOLanguageName": language_to_iso3(&language_raw),
                "Id": format!("local-{}", track.index),
                "ProviderName": "LocalSubtitles",
                "Name": track.display_title,
                "Format": track.codec,
                "Author": "local",
                "Comment": "",
                "DateCreated": Value::Null,
                "CommunityRating": Value::Null,
                "DownloadCount": Value::Null,
                "IsHashMatch": is_hash_match,
                "IsForced": false,
            })
        })
        .collect::<Vec<_>>();
    if query.is_perfect_match.unwrap_or(false) {
        subtitles.retain(|entry| entry.get("IsHashMatch").and_then(Value::as_bool).unwrap_or(false));
    }

    Json(subtitles).into_response()
}

async fn post_item_remote_search_subtitle_download(
    State(state): State<ApiContext>,
    AxPath((raw_item_id, subtitle_id)): AxPath<(String, String)>,
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

    if let Some(index) = parse_remote_subtitle_track_id(&subtitle_id) {
        match state.infra.subtitle_path_by_index(item_id, index).await {
            Ok(Some(_)) => return StatusCode::NO_CONTENT.into_response(),
            Ok(None) => {}
            Err(err) => {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("failed to resolve subtitle track: {err}"),
                );
            }
        }
    }

    let library_id = match state.infra.item_library_id(item_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "item not found"),
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to resolve item library: {err}"),
            );
        }
    };
    match state.infra.enqueue_subtitle_sync_job(Some(library_id)).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to enqueue subtitle sync: {err}"),
        ),
    }
}

async fn post_item_image_with_index(
    state: State<ApiContext>,
    AxPath((raw_item_id, image_type, _image_index)): AxPath<(String, String, i32)>,
    headers: HeaderMap,
    uri: Uri,
    body: web::Bytes,
) -> Response {
    upload_item_image(state, AxPath((raw_item_id, image_type)), headers, uri, body).await
}

async fn delete_item_image_with_index_compat(
    state: State<ApiContext>,
    AxPath((raw_item_id, image_type, _image_index)): AxPath<(String, String, i32)>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    delete_item_image(state, AxPath((raw_item_id, image_type)), headers, uri).await
}

#[derive(Debug, Deserialize)]
struct MoveItemImageIndexQuery {
    #[serde(rename = "NewIndex", alias = "newIndex")]
    _new_index: Option<i32>,
}

async fn post_item_image_reorder_index(
    State(state): State<ApiContext>,
    AxPath((_item_id, _image_type, _image_index)): AxPath<(String, String, i32)>,
    headers: HeaderMap,
    uri: Uri,
    Query(_query): Query<MoveItemImageIndexQuery>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }
    StatusCode::NO_CONTENT.into_response()
}

async fn stream_item_image_legacy_full_path(
    State(state): State<ApiContext>,
    AxPath((
        raw_item_id,
        image_type,
        image_index,
        tag,
        format,
        max_width,
        max_height,
        percent_played,
        unplayed_count,
    )): AxPath<(String, String, i32, String, String, i32, i32, i32, i32)>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<ImageRequestCompatQuery>,
) -> Response {
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    let mut merged = query;
    if merged.tag.is_none() {
        merged.tag = Some(tag);
    }
    if merged._format.is_none() {
        merged._format = Some(format);
    }
    if merged._max_width.is_none() {
        merged._max_width = Some(max_width.to_string());
    }
    if merged._max_height.is_none() {
        merged._max_height = Some(max_height.to_string());
    }
    if merged._percent_played.is_none() {
        merged._percent_played = Some(percent_played.to_string());
    }
    if merged._unplayed_count.is_none() {
        merged._unplayed_count = Some(unplayed_count.to_string());
    }
    stream_image_inner(state, item_id, image_type, image_index, merged, headers, uri).await
}

async fn post_items_remote_search_generic(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<Value>,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }

    let search_kind = uri
        .path()
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let search_term = remote_search_name_from_payload(&payload);
    let provider_ids = remote_search_provider_ids_from_payload(&payload).unwrap_or_default();
    let production_year = remote_search_year_from_payload(&payload);

    let base_items = if search_kind == "person" || search_kind == "musicartist" {
        match state
            .infra
            .list_persons(search_term.as_deref(), 0, 30)
            .await
        {
            Ok(result) => result.items,
            Err(err) => {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("failed to query remote search persons: {err}"),
                );
            }
        }
    } else {
        let include_item_types = remote_search_include_item_types(search_kind.as_str());

        let options = InfraItemsQuery {
            user_id: None,
            series_filter: None,
            parent_id: None,
            include_item_types,
            exclude_item_types: Vec::new(),
            person_ids: Vec::new(),
            search_term: search_term.clone(),
            limit: 50,
            start_index: 0,
            is_resumable: false,
            sort_by: vec!["CommunityRating".to_string(), "DateCreated".to_string()],
            sort_order: "Descending".to_string(),
            recursive: true,
            genres: Vec::new(),
            tags: Vec::new(),
            years: production_year.map(|v| vec![v]).unwrap_or_default(),
            is_favorite: None,
            is_played: None,
            min_community_rating: None,
        };

        match state.infra.list_items_with_options(options).await {
            Ok(result) => result.items,
            Err(err) => {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("failed to query remote search items: {err}"),
                );
            }
        }
    };

    let provider_name = payload
        .as_object()
        .and_then(|obj| obj.get("SearchProviderName").or_else(|| obj.get("searchProviderName")))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("LocalLibrary")
        .to_string();

    let mut scored = base_items
        .into_iter()
        .map(|item| {
            let mut score = 0_i32;
            if let Some(search_name) = search_term.as_deref() {
                if item.name.eq_ignore_ascii_case(search_name) {
                    score += 20;
                } else if item
                    .name
                    .to_ascii_lowercase()
                    .contains(&search_name.to_ascii_lowercase())
                {
                    score += 5;
                }
            }
            if let Some(year) = production_year {
                if item.production_year == Some(year) {
                    score += 8;
                }
            }
            if !provider_ids.is_empty() {
                let current = item.provider_ids.clone().unwrap_or_default();
                for (key, expected) in &provider_ids {
                    if current
                        .get(key)
                        .is_some_and(|actual| actual.eq_ignore_ascii_case(expected))
                    {
                        score += 30;
                    }
                }
            }

            let image_url = format!("/Items/{}/Images/Primary", item.id);
            let payload = json!({
                "Name": item.name,
                "ProviderIds": item.provider_ids.clone().unwrap_or_default(),
                "ProductionYear": item.production_year,
                "IndexNumber": item.index_number,
                "ParentIndexNumber": item.parent_index_number,
                "PremiereDate": item.premiere_date,
                "ImageUrl": image_url,
                "SearchProviderName": provider_name,
                "Overview": item.overview,
            });
            (score, payload)
        })
        .collect::<Vec<_>>();
    scored.sort_by(|left, right| right.0.cmp(&left.0));
    let results = scored
        .into_iter()
        .map(|(_, payload)| payload)
        .take(30)
        .collect::<Vec<_>>();
    Json(results).into_response()
}

#[derive(Debug, Deserialize)]
struct RemoteSearchImageQuery {
    #[serde(rename = "ImageUrl", alias = "imageUrl")]
    image_url: Option<String>,
    #[serde(rename = "ProviderName", alias = "providerName")]
    provider_name: Option<String>,
}

async fn get_items_remote_search_image(
    Query(query): Query<RemoteSearchImageQuery>,
) -> Response {
    let provider = query
        .provider_name
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or_default();
    if provider.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "ProviderName is required");
    }
    let Some(image_url) = query
        .image_url
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    else {
        return error_response(StatusCode::BAD_REQUEST, "ImageUrl is required");
    };
    if !(image_url.starts_with("http://") || image_url.starts_with("https://")) {
        return error_response(StatusCode::BAD_REQUEST, "ImageUrl must be http(s)");
    }

    match proxy_http_stream(image_url, None).await {
        Ok((response, _)) => response,
        Err(err) => error_response(
            StatusCode::BAD_GATEWAY,
            &format!("failed to fetch remote image: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize, Default)]
struct RemoteSearchApplyQuery {
    #[serde(rename = "ReplaceAllImages", alias = "replaceAllImages")]
    replace_all_images: Option<bool>,
}

async fn post_items_remote_search_apply(
    State(state): State<ApiContext>,
    AxPath(raw_item_id): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<RemoteSearchApplyQuery>,
    Json(payload): Json<Value>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };

    let name = payload
        .as_object()
        .and_then(|obj| obj.get("Name").or_else(|| obj.get("name")))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToString::to_string);
    let patch = metadata_patch_from_payload(&payload);
    let updated = match state
        .infra
        .patch_item_metadata(item_id, name.as_deref(), patch.as_ref())
        .await
    {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to apply remote search metadata: {err}"),
            );
        }
    };
    if !updated {
        return error_response(StatusCode::NOT_FOUND, "item not found");
    }

    if query.replace_all_images.unwrap_or(false) {
        if let Some(image_url) = payload
            .as_object()
            .and_then(|obj| obj.get("ImageUrl").or_else(|| obj.get("imageUrl")))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            match download_remote_image(&state, image_url).await {
                Ok((body, extension)) => {
                    match save_image_for_item_or_library(
                        &state,
                        item_id,
                        "Primary",
                        &body,
                        &extension,
                    )
                    .await
                    {
                        Ok(true) => {}
                        Ok(false) => {
                            return error_response(StatusCode::NOT_FOUND, "item not found");
                        }
                        Err(err) => {
                            return error_response(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            &format!("failed to save remote search image: {err}"),
                        );
                        }
                    }
                }
                Err(err) => {
                    return error_response(
                        StatusCode::BAD_GATEWAY,
                        &format!("failed to download remote search image: {err}"),
                    );
                }
            }
        }
    }

    if let Ok(Some(library_id)) = state.infra.item_library_id(item_id).await {
        if let Err(err) = state
            .infra
            .enqueue_metadata_repair_job(Some(library_id))
            .await
        {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to enqueue metadata repair: {err}"),
            );
        }
    }

    StatusCode::NO_CONTENT.into_response()
}

#[derive(Debug, Deserialize)]
struct LibraryMediaFoldersQuery {
    #[serde(rename = "IsHidden", alias = "isHidden")]
    _is_hidden: Option<bool>,
}

async fn get_libraries_available_options(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }
    Json(json!({
        "TypeOptions": [
            {"Name": "Mixed", "LibraryType": "mixed"},
            {"Name": "Movie", "LibraryType": "movies"},
            {"Name": "Series", "LibraryType": "tvshows"}
        ]
    }))
    .into_response()
}

async fn get_library_selectable_media_folders(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }
    match state.infra.list_libraries().await {
        Ok(libraries) => Json(json!({
            "Items": libraries.into_iter().map(|item| json!({
                "Name": item.name,
                "Path": item.root_path,
            })).collect::<Vec<_>>(),
        }))
        .into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to list selectable media folders: {err}"),
        ),
    }
}

async fn get_library_media_folders(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(_query): Query<LibraryMediaFoldersQuery>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }
    match state.infra.list_libraries().await {
        Ok(libraries) => Json(json!({
            "Items": libraries.into_iter().map(|item| json!({
                "Name": item.name,
                "Path": item.root_path,
                "CollectionType": normalize_emby_collection_type(&item.library_type),
                "LibraryId": item.id,
                "IsHidden": !item.enabled,
            })).collect::<Vec<_>>(),
        }))
        .into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to list media folders: {err}"),
        ),
    }
}

async fn get_library_physical_paths(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }
    match state.infra.list_libraries().await {
        Ok(libraries) => Json(
            libraries
                .into_iter()
                .flat_map(|library| library.paths)
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to list physical paths: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct LibraryRefreshQuery {
    #[serde(rename = "LibraryId", alias = "libraryId")]
    library_id: Option<String>,
}

async fn post_library_refresh(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<LibraryRefreshQuery>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    if let Some(library_id) = match resolve_optional_item_uuid(&state, query.library_id.as_deref()).await {
        Ok(value) => value,
        Err(resp) => return resp,
    } {
        match state
            .infra
            .enqueue_scan_job(library_id, Some("incremental"), None)
            .await
        {
            Ok(_) => return StatusCode::NO_CONTENT.into_response(),
            Err(err) => {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("failed to enqueue library refresh: {err}"),
                );
            }
        }
    }

    let libraries = match state.infra.list_libraries().await {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to list libraries: {err}"),
            );
        }
    };

    for library in libraries.into_iter().filter(|v| v.enabled) {
        if let Err(err) = state
            .infra
            .enqueue_scan_job(library.id, Some("incremental"), None)
            .await
        {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to enqueue refresh for library {}: {err}", library.id),
            );
        }
    }

    StatusCode::NO_CONTENT.into_response()
}

async fn get_library_virtual_folders(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }
    match state.infra.list_libraries().await {
        Ok(libraries) => {
            let mut payload = serde_json::to_value(
                libraries
                    .into_iter()
                    .map(|item| {
                        json!({
                            "Name": item.name,
                            "Locations": item.paths.clone(),
                            "CollectionType": normalize_emby_collection_type(&item.library_type),
                            "ItemId": item.id,
                            "PrimaryImageItemId": item.id,
                            "RefreshStatus": "Idle",
                            "LibraryOptions": {
                                "EnableRealtimeMonitor": false,
                                "PathInfos": item
                                    .paths
                                    .iter()
                                    .map(|path| json!({ "Path": path }))
                                    .collect::<Vec<_>>(),
                            }
                        })
                    })
                    .collect::<Vec<_>>(),
            )
            .unwrap_or_else(|_| Value::Array(Vec::new()));
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
            &format!("failed to list virtual folders: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct AddVirtualFolderRequest {
    #[serde(rename = "Name", alias = "name")]
    name: Option<String>,
    #[serde(rename = "CollectionType", alias = "collectionType")]
    collection_type: Option<String>,
    #[serde(rename = "Paths", alias = "paths")]
    paths: Option<Vec<String>>,
    #[serde(rename = "Path", alias = "path")]
    path: Option<String>,
    #[serde(rename = "RefreshLibrary", alias = "refreshLibrary")]
    refresh_library: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct MediaPathInfoRequest {
    #[serde(rename = "Path", alias = "path")]
    path: Option<String>,
    #[serde(rename = "NetworkPath", alias = "networkPath")]
    _network_path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RenameVirtualFolderRequest {
    #[serde(rename = "Name", alias = "name")]
    name: Option<String>,
    #[serde(rename = "NewName", alias = "newName")]
    new_name: Option<String>,
    #[serde(rename = "RefreshLibrary", alias = "refreshLibrary")]
    refresh_library: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct AddMediaPathRequest {
    #[serde(rename = "Name", alias = "name")]
    name: Option<String>,
    #[serde(rename = "Path", alias = "path")]
    path: Option<String>,
    #[serde(rename = "PathInfo", alias = "pathInfo")]
    path_info: Option<MediaPathInfoRequest>,
    #[serde(rename = "RefreshLibrary", alias = "refreshLibrary")]
    refresh_library: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct DeleteMediaPathQuery {
    #[serde(rename = "Name", alias = "name")]
    name: Option<String>,
    #[serde(rename = "Path", alias = "path")]
    path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateMediaPathRequest {
    #[serde(rename = "Name", alias = "name")]
    name: Option<String>,
    #[serde(rename = "PathInfo", alias = "pathInfo")]
    path_info: Option<MediaPathInfoRequest>,
}

#[derive(Debug, Deserialize)]
struct UpdateLibraryOptionsRequest {
    #[serde(rename = "Id", alias = "id")]
    id: Option<String>,
    #[serde(rename = "LibraryOptions", alias = "libraryOptions")]
    library_options: Option<UpdateLibraryOptionsPayload>,
}

#[derive(Debug, Deserialize)]
struct UpdateLibraryOptionsPayload {
    #[serde(rename = "ContentType", alias = "contentType")]
    content_type: Option<String>,
    #[serde(rename = "PathInfos", alias = "pathInfos")]
    path_infos: Option<Vec<MediaPathInfoRequest>>,
}

fn normalized_text(raw: Option<&str>) -> Option<String> {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn resolve_media_path(path: Option<&str>, path_info: Option<&MediaPathInfoRequest>) -> Option<String> {
    normalized_text(path).or_else(|| {
        path_info.and_then(|v| normalized_text(v.path.as_deref()))
    })
}

fn normalized_paths(paths: Option<&Vec<String>>, path: Option<&str>) -> Vec<String> {
    let mut out = Vec::new();
    let mut dedup = std::collections::HashSet::new();
    if let Some(paths) = paths {
        for candidate in paths {
            let Some(normalized) = normalized_text(Some(candidate.as_str())) else {
                continue;
            };
            let key = normalized.to_lowercase();
            if dedup.insert(key) {
                out.push(normalized);
            }
        }
    }
    if let Some(single) = normalized_text(path) {
        let key = single.to_lowercase();
        if dedup.insert(key) {
            out.push(single);
        }
    }
    out
}

fn library_option_paths(path_infos: Option<&Vec<MediaPathInfoRequest>>) -> Option<Vec<String>> {
    let path_infos = path_infos?;
    let mut out = Vec::new();
    let mut dedup = std::collections::HashSet::new();
    for path_info in path_infos {
        let Some(normalized) = normalized_text(path_info.path.as_deref()) else {
            continue;
        };
        let key = normalized.to_lowercase();
        if dedup.insert(key) {
            out.push(normalized);
        }
    }
    Some(out)
}

async fn post_library_virtual_folders(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<AddVirtualFolderRequest>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    let name = normalized_text(payload.name.as_deref());
    let paths = normalized_paths(payload.paths.as_ref(), payload.path.as_deref());

    let Some(name) = name else {
        return error_response(StatusCode::BAD_REQUEST, "Name is required");
    };
    if paths.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "Path is required");
    }

    let Some(library_type) = normalize_library_type(payload.collection_type.as_deref()) else {
        return error_response(StatusCode::BAD_REQUEST, "invalid CollectionType");
    };

    match state
        .infra
        .create_library(name.as_str(), &paths, library_type)
        .await
    {
        Ok(created) => {
            if payload.refresh_library.unwrap_or(false) {
                if let Err(err) = state
                    .infra
                    .enqueue_scan_job(created.id, Some("incremental"), None)
                    .await
                {
                    return error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        &format!("failed to refresh virtual folder: {err}"),
                    );
                }
            }
            StatusCode::NO_CONTENT.into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to create virtual folder: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct DeleteVirtualFolderQuery {
    #[serde(rename = "Name", alias = "name")]
    name: Option<String>,
}

async fn delete_library_virtual_folders(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<DeleteVirtualFolderQuery>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    let Some(name) = normalized_text(query.name.as_deref()) else {
        return error_response(StatusCode::BAD_REQUEST, "Name is required");
    };
    let library = match state.infra.get_library_by_name(name.as_str()).await {
        Ok(Some(v)) => v,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "virtual folder not found"),
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to query virtual folders: {err}"),
            );
        }
    };
    if let Err(err) = state.infra.set_library_enabled(library.id, false).await {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to disable virtual folder: {err}"),
        );
    }
    StatusCode::NO_CONTENT.into_response()
}

async fn post_library_changed_noop(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(_payload): Json<Value>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }
    StatusCode::NO_CONTENT.into_response()
}

async fn post_library_changed_noop_empty(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }
    StatusCode::NO_CONTENT.into_response()
}

async fn post_library_virtual_folders_library_options(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<UpdateLibraryOptionsRequest>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }

    let Some(library_id) = (match resolve_optional_item_uuid(&state, payload.id.as_deref()).await {
        Ok(value) => value,
        Err(resp) => return resp,
    }) else {
        return error_response(StatusCode::BAD_REQUEST, "Id is required");
    };

    let requested_paths = library_option_paths(
        payload
            .library_options
            .as_ref()
            .and_then(|options| options.path_infos.as_ref()),
    );

    if let Some(content_type) = payload
        .library_options
        .as_ref()
        .and_then(|options| options.content_type.as_deref())
    {
        let Some(library_type) = normalize_library_type(Some(content_type)) else {
            return error_response(StatusCode::BAD_REQUEST, "invalid ContentType");
        };
        match state.infra.update_library_type(library_id, library_type).await {
            Ok(Some(_)) => {}
            Ok(None) => return error_response(StatusCode::NOT_FOUND, "library not found"),
            Err(err) => {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("failed to update library options: {err}"),
                );
            }
        }
    }

    if let Some(paths) = requested_paths {
        return match state.infra.replace_library_paths(library_id, &paths).await {
            Ok(Some(_)) => StatusCode::NO_CONTENT.into_response(),
            Ok(None) => error_response(StatusCode::NOT_FOUND, "library not found"),
            Err(err) => error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to update library options paths: {err}"),
            ),
        };
    }

    match state.infra.get_library_by_id(library_id).await {
        Ok(Some(_)) => StatusCode::NO_CONTENT.into_response(),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "library not found"),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to query library options: {err}"),
        ),
    }
}

async fn post_library_virtual_folders_name(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<RenameVirtualFolderRequest>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }
    let Some(name) = normalized_text(payload.name.as_deref()) else {
        return error_response(StatusCode::BAD_REQUEST, "Name is required");
    };
    let Some(new_name) = normalized_text(payload.new_name.as_deref()) else {
        return error_response(StatusCode::BAD_REQUEST, "NewName is required");
    };

    let library = match state.infra.get_library_by_name(name.as_str()).await {
        Ok(Some(v)) => v,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "virtual folder not found"),
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to query virtual folder: {err}"),
            );
        }
    };

    let updated = match state
        .infra
        .update_library_name(library.id, new_name.as_str())
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "virtual folder not found"),
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to rename virtual folder: {err}"),
            );
        }
    };

    if payload.refresh_library.unwrap_or(false) {
        if let Err(err) = state
            .infra
            .enqueue_scan_job(updated.id, Some("incremental"), None)
            .await
        {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to refresh virtual folder: {err}"),
            );
        }
    }
    StatusCode::NO_CONTENT.into_response()
}

async fn post_library_virtual_folders_paths(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<AddMediaPathRequest>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }
    let Some(name) = normalized_text(payload.name.as_deref()) else {
        return error_response(StatusCode::BAD_REQUEST, "Name is required");
    };
    let Some(path) = resolve_media_path(payload.path.as_deref(), payload.path_info.as_ref()) else {
        return error_response(StatusCode::BAD_REQUEST, "Path is required");
    };

    let library = match state.infra.get_library_by_name(name.as_str()).await {
        Ok(Some(v)) => v,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "virtual folder not found"),
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to query virtual folder: {err}"),
            );
        }
    };

    let updated = match state.infra.add_library_path(library.id, path.as_str()).await {
        Ok(Some(v)) => v,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "virtual folder not found"),
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to add virtual folder path: {err}"),
            );
        }
    };

    if payload.refresh_library.unwrap_or(false) {
        if let Err(err) = state
            .infra
            .enqueue_scan_job(updated.id, Some("incremental"), None)
            .await
        {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to refresh virtual folder: {err}"),
            );
        }
    }
    StatusCode::NO_CONTENT.into_response()
}

async fn delete_library_virtual_folders_paths(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<DeleteMediaPathQuery>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }
    let Some(name) = normalized_text(query.name.as_deref()) else {
        return error_response(StatusCode::BAD_REQUEST, "Name is required");
    };
    let Some(path) = normalized_text(query.path.as_deref()) else {
        return error_response(StatusCode::BAD_REQUEST, "Path is required");
    };

    let library = match state.infra.get_library_by_name(name.as_str()).await {
        Ok(Some(v)) => v,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "virtual folder not found"),
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to query virtual folder: {err}"),
            );
        }
    };

    match state.infra.remove_library_path(library.id, path.as_str()).await {
        Ok(Some(true)) => StatusCode::NO_CONTENT.into_response(),
        Ok(Some(false)) => error_response(StatusCode::NOT_FOUND, "path not found"),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "virtual folder not found"),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to delete virtual folder path: {err}"),
        ),
    }
}

async fn post_library_virtual_folders_paths_update(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Json(payload): Json<UpdateMediaPathRequest>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }
    let Some(name) = normalized_text(payload.name.as_deref()) else {
        return error_response(StatusCode::BAD_REQUEST, "Name is required");
    };
    let Some(path) = resolve_media_path(None, payload.path_info.as_ref()) else {
        return error_response(StatusCode::BAD_REQUEST, "PathInfo.Path is required");
    };

    let library = match state.infra.get_library_by_name(name.as_str()).await {
        Ok(Some(v)) => v,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "virtual folder not found"),
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to query virtual folder: {err}"),
            );
        }
    };

    match state
        .infra
        .replace_primary_library_path(library.id, path.as_str())
        .await
    {
        Ok(Some(_)) => StatusCode::NO_CONTENT.into_response(),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "virtual folder not found"),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to update virtual folder path: {err}"),
        ),
    }
}
