/// GET /Genres - List all genres in library
async fn get_genres(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<GenresQuery>,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }

    let include_item_types = split_csv(query.include_item_types.as_deref());
    let start_index = query.start_index.unwrap_or(0);
    let limit = query.limit.unwrap_or(100);
    let parent_id = match resolve_optional_item_uuid(&state, query.parent_id.as_deref()).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };

    match state
        .infra
        .list_genres(
            parent_id,
            query.recursive.unwrap_or(false),
            &include_item_types,
            start_index,
            limit,
        )
        .await
    {
        Ok(result) => Json(compat_metadata_lookup_query_result_json(
            result,
            &state.infra.server_id,
        ))
        .into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to list genres: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct NamedValueListQuery {
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
    #[serde(rename = "StartIndex", alias = "startIndex", alias = "start_index")]
    start_index: Option<i64>,
    #[serde(rename = "Limit", alias = "limit")]
    limit: Option<i64>,
    #[serde(rename = "Recursive", alias = "recursive")]
    recursive: Option<bool>,
}

#[derive(Debug, Serialize)]
struct NamedValueDto {
    #[serde(rename = "Name")]
    name: String,
}

#[derive(Debug, Serialize)]
struct NamedValueQueryResultDto {
    #[serde(rename = "Items")]
    items: Vec<NamedValueDto>,
    #[serde(rename = "TotalRecordCount")]
    total_record_count: i32,
}

fn paginate_named_values(values: Vec<String>, start_index: i64, limit: i64) -> NamedValueQueryResultDto {
    let start_index = start_index.max(0) as usize;
    let limit = limit.clamp(1, 500) as usize;
    let total_record_count = values.len() as i32;
    let items = values
        .into_iter()
        .skip(start_index)
        .take(limit)
        .map(|name| NamedValueDto { name })
        .collect::<Vec<_>>();

    NamedValueQueryResultDto {
        items,
        total_record_count,
    }
}

/// GET /OfficialRatings - List available official ratings.
async fn get_official_ratings(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<NamedValueListQuery>,
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
        .get_item_filters(parent_id, query.recursive.unwrap_or(false), &include_item_types)
        .await
    {
        Ok(filters) => {
            let ratings = filters
                .official_ratings
                .into_iter()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>();
            Json(paginate_named_values(
                ratings,
                query.start_index.unwrap_or(0),
                query.limit.unwrap_or(100),
            ))
            .into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to list official ratings: {err}"),
        ),
    }
}

/// GET /Tags - List available tags.
async fn get_tags(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<NamedValueListQuery>,
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
        .get_item_filters(parent_id, query.recursive.unwrap_or(false), &include_item_types)
        .await
    {
        Ok(filters) => {
            let tags = filters
                .tags
                .into_iter()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>();
            Json(paginate_named_values(
                tags,
                query.start_index.unwrap_or(0),
                query.limit.unwrap_or(100),
            ))
            .into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to list tags: {err}"),
        ),
    }
}

/// GET /Years - List production years (as Name items for Emby-compatible shape).
async fn get_years(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<NamedValueListQuery>,
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
        .get_item_filters(parent_id, query.recursive.unwrap_or(false), &include_item_types)
        .await
    {
        Ok(filters) => {
            let years = filters
                .years
                .into_iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>();
            Json(paginate_named_values(
                years,
                query.start_index.unwrap_or(0),
                query.limit.unwrap_or(100),
            ))
            .into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to list years: {err}"),
        ),
    }
}

#[derive(Debug, Serialize)]
struct StudioBrowseUserDataDto {
    #[serde(rename = "PlaybackPositionTicks")]
    playback_position_ticks: i64,
    #[serde(rename = "PlayCount")]
    play_count: i32,
    #[serde(rename = "IsFavorite")]
    is_favorite: bool,
    #[serde(rename = "Played")]
    played: bool,
}

#[derive(Debug, Serialize)]
struct StudioBrowseItemDto {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "ServerId")]
    server_id: String,
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "Type")]
    item_type: String,
    #[serde(rename = "UserData")]
    user_data: StudioBrowseUserDataDto,
    #[serde(rename = "ImageTags")]
    image_tags: HashMap<String, String>,
    #[serde(rename = "BackdropImageTags")]
    backdrop_image_tags: Vec<String>,
}

#[derive(Debug, Serialize)]
struct StudioBrowseQueryResultDto {
    #[serde(rename = "Items")]
    items: Vec<StudioBrowseItemDto>,
    #[serde(rename = "TotalRecordCount")]
    total_record_count: i32,
}

/// GET /Studios - List studios used by media metadata.
async fn get_studios(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<NamedValueListQuery>,
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
    let start_index = query.start_index.unwrap_or(0);
    let limit = query.limit.unwrap_or(100);

    match state
        .infra
        .list_studio_names(
            parent_id,
            query.recursive.unwrap_or(false),
            &include_item_types,
            start_index,
            limit,
        )
        .await
    {
        Ok((names, total_record_count)) => {
            let items = names
                .into_iter()
                .map(|name| StudioBrowseItemDto {
                    id: stable_compat_numeric_id(&format!("studio:{name}")).to_string(),
                    name,
                    server_id: state.infra.server_id.clone(),
                    item_type: "Studio".to_string(),
                    user_data: StudioBrowseUserDataDto {
                        playback_position_ticks: 0,
                        play_count: 0,
                        is_favorite: false,
                        played: false,
                    },
                    image_tags: HashMap::new(),
                    backdrop_image_tags: Vec::new(),
                })
                .collect::<Vec<_>>();
            Json(StudioBrowseQueryResultDto {
                items,
                total_record_count,
            })
            .into_response()
        }
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to list studios: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct TopPlayedItemsQuery {
    #[serde(rename = "Limit", alias = "limit")]
    limit: Option<i64>,
    #[serde(rename = "WindowDays", alias = "window_days")]
    window_days: Option<i32>,
    #[serde(rename = "StatDate", alias = "stat_date")]
    stat_date: Option<String>,
}

#[derive(Debug, Serialize)]
struct TopPlayedItemResponse {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Type")]
    item_type: String,
    #[serde(rename = "RunTimeTicks", skip_serializing_if = "Option::is_none")]
    runtime_ticks: Option<i64>,
    #[serde(rename = "Bitrate", skip_serializing_if = "Option::is_none")]
    bitrate: Option<i32>,
    #[serde(rename = "ProductionYear", skip_serializing_if = "Option::is_none")]
    production_year: Option<i32>,
    #[serde(rename = "CommunityRating", skip_serializing_if = "Option::is_none")]
    community_rating: Option<f64>,
    #[serde(rename = "Overview", skip_serializing_if = "Option::is_none")]
    overview: Option<String>,
    #[serde(rename = "PlayCount")]
    play_count: i64,
    #[serde(rename = "UniqueUsers")]
    unique_users: i64,
}

#[derive(Debug, Serialize)]
struct TopPlayedItemsResponse {
    #[serde(rename = "StatDate")]
    stat_date: String,
    #[serde(rename = "WindowDays")]
    window_days: i32,
    #[serde(rename = "Items")]
    items: Vec<TopPlayedItemResponse>,
}

fn normalize_top_played_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(10).clamp(1, 100)
}

fn normalize_top_played_window_days(window_days: Option<i32>) -> i32 {
    window_days.unwrap_or(1).clamp(1, 90)
}

fn normalize_next_up_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(20).clamp(1, 100)
}

fn normalize_next_up_start_index(start_index: Option<i64>) -> i64 {
    start_index.unwrap_or(0).max(0)
}

fn normalize_resume_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(50).clamp(1, 500)
}

fn normalize_resume_start_index(start_index: Option<i64>) -> i64 {
    start_index.unwrap_or(0).max(0)
}

fn resume_name_matches_search(name: &str, search_term: Option<&str>) -> bool {
    let Some(search) = search_term.map(str::trim).filter(|raw| !raw.is_empty()) else {
        return true;
    };

    name.to_ascii_lowercase()
        .contains(&search.to_ascii_lowercase())
}

fn resume_parent_matches(item: &BaseItemDto, parent_id: Option<Uuid>) -> bool {
    let Some(parent_id) = parent_id else {
        return true;
    };

    let parent = parent_id.to_string();
    let parent = parent.as_str();
    item.parent_id.as_deref() == Some(parent)
        || item.series_id.as_deref() == Some(parent)
        || item.season_id.as_deref() == Some(parent)
}

fn resume_item_type_matches(
    item_type: &str,
    include_item_types: &[String],
    exclude_item_types: &[String],
) -> bool {
    if !include_item_types.is_empty()
        && !include_item_types
            .iter()
            .any(|raw| raw.eq_ignore_ascii_case(item_type))
    {
        return false;
    }

    !exclude_item_types
        .iter()
        .any(|raw| raw.eq_ignore_ascii_case(item_type))
}

fn resume_media_type_for_item_type(item_type: &str) -> Option<&'static str> {
    if item_type.eq_ignore_ascii_case("Episode")
        || item_type.eq_ignore_ascii_case("Movie")
        || item_type.eq_ignore_ascii_case("Video")
        || item_type.eq_ignore_ascii_case("MusicVideo")
        || item_type.eq_ignore_ascii_case("Trailer")
    {
        return Some("Video");
    }

    if item_type.eq_ignore_ascii_case("Audio")
        || item_type.eq_ignore_ascii_case("Song")
        || item_type.eq_ignore_ascii_case("MusicAlbum")
        || item_type.eq_ignore_ascii_case("MusicArtist")
        || item_type.eq_ignore_ascii_case("AudioBook")
    {
        return Some("Audio");
    }

    if item_type.eq_ignore_ascii_case("Photo") || item_type.eq_ignore_ascii_case("PhotoAlbum") {
        return Some("Photo");
    }

    if item_type.eq_ignore_ascii_case("Book") {
        return Some("Book");
    }

    None
}

fn resume_media_type_matches(item_type: &str, media_types: &[String]) -> bool {
    if media_types.is_empty() {
        return true;
    }

    if media_types
        .iter()
        .any(|raw| raw.eq_ignore_ascii_case(item_type))
    {
        return true;
    }

    let Some(media_type) = resume_media_type_for_item_type(item_type) else {
        return false;
    };

    media_types
        .iter()
        .any(|raw| raw.eq_ignore_ascii_case(media_type))
}

fn apply_resume_query_filters(
    items: Vec<BaseItemDto>,
    query: &ResumeItemsQuery,
    parent_id: Option<Uuid>,
) -> Vec<BaseItemDto> {
    let include_item_types = split_csv(query.include_item_types.as_deref());
    let exclude_item_types = split_csv(query.exclude_item_types.as_deref());
    let media_types = split_csv(query.media_types.as_deref());
    let search_term = query.search_term.as_deref();
    items
        .into_iter()
        .filter(|item| {
            resume_name_matches_search(&item.name, search_term)
                && resume_parent_matches(item, parent_id)
                && resume_item_type_matches(
                    &item.item_type,
                    &include_item_types,
                    &exclude_item_types,
                )
                && resume_media_type_matches(&item.item_type, &media_types)
        })
        .collect()
}

fn paginate_resume_items<T>(
    items: Vec<T>,
    start_index: i64,
    limit: i64,
    enable_total_record_count: bool,
) -> QueryResultDto<T> {
    let total_record_count = if enable_total_record_count {
        items.len() as i32
    } else {
        0
    };
    let paged_items = items
        .into_iter()
        .skip(start_index as usize)
        .take(limit as usize)
        .collect();

    QueryResultDto {
        items: paged_items,
        total_record_count,
        start_index: start_index as i32,
    }
}

fn apply_resume_query_compatibility(
    items: QueryResultDto<BaseItemDto>,
    query: &ResumeItemsQuery,
    parent_id: Option<Uuid>,
) -> QueryResultDto<BaseItemDto> {
    let filtered_items = apply_resume_query_filters(items.items, query, parent_id);
    let start_index = normalize_resume_start_index(query.start_index);
    let limit = normalize_resume_limit(query.limit);
    let enable_total_record_count = query.enable_total_record_count.unwrap_or(true);
    paginate_resume_items(filtered_items, start_index, limit, enable_total_record_count)
}

fn parse_top_played_stat_date(raw: Option<&str>) -> Result<NaiveDate, String> {
    let Some(value) = raw.map(str::trim).filter(|v| !v.is_empty()) else {
        return Ok(Utc::now().date_naive());
    };

    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| format!("invalid StatDate, expected YYYY-MM-DD, got: {value}"))
}

fn map_top_played_summary(summary: TopPlayedMediaSummary) -> TopPlayedItemsResponse {
    TopPlayedItemsResponse {
        stat_date: summary.stat_date,
        window_days: summary.window_days,
        items: summary
            .items
            .into_iter()
            .map(|item| TopPlayedItemResponse {
                id: item.item_id.to_string(),
                name: item.name,
                item_type: item.item_type,
                runtime_ticks: item.runtime_ticks,
                bitrate: item.bitrate,
                production_year: item.production_year,
                community_rating: item.community_rating,
                overview: item.overview,
                play_count: item.play_count,
                unique_users: item.unique_users,
            })
            .collect(),
    }
}

async fn get_top_played_items(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<TopPlayedItemsQuery>,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }

    let limit = normalize_top_played_limit(query.limit);
    let window_days = normalize_top_played_window_days(query.window_days);
    let stat_date = match parse_top_played_stat_date(query.stat_date.as_deref()) {
        Ok(v) => v,
        Err(err) => return error_response(StatusCode::BAD_REQUEST, &err),
    };

    match state
        .infra
        .list_top_played_media(limit, Some(stat_date), window_days)
        .await
    {
        Ok(summary) => {
            let mut payload = serde_json::to_value(map_top_played_summary(summary))
                .unwrap_or_else(|_| json!({"Items": [], "StatDate": "", "WindowDays": window_days}));
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
            &format!("failed to query top played items: {err}"),
        ),
    }
}

async fn get_show_episodes(
    State(state): State<ApiContext>,
    AxPath(raw_show_id): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<ShowEpisodesQuery>,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(user) => user,
        Err(resp) => return resp,
    };

    let user_id = query
        .user_id
        .or_else(|| Uuid::parse_str(&auth_user.id).ok());
    let show_id = match resolve_item_uuid_or_bad_request(&state, &raw_show_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    let season_id = match resolve_optional_item_uuid(&state, query.season_id.as_deref()).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    let adjacent_to = match resolve_optional_item_uuid(&state, query.adjacent_to.as_deref()).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    let start_item_id =
        match resolve_optional_item_uuid(&state, query.start_item_id.as_deref()).await {
            Ok(value) => value,
            Err(resp) => return resp,
        };
    let options = InfraItemsQuery {
        user_id,
        series_filter: if season_id.is_some() {
            None
        } else {
            Some(show_id)
        },
        parent_id: season_id,
        include_item_types: vec!["Episode".to_string()],
        exclude_item_types: vec![],
        person_ids: vec![],
        search_term: None,
        limit: 500,
        start_index: 0,
        is_resumable: false,
        sort_by: split_csv(query.sort_by.as_deref()),
        sort_order: query
            .sort_order
            .clone()
            .unwrap_or_else(|| "Ascending".to_string()),
        recursive: false,
        genres: vec![],
        tags: vec![],
        years: vec![],
        is_favorite: None,
        is_played: None,
        min_community_rating: None,
    };

    match state.infra.list_items_with_options(options).await {
        Ok(items) => {
            let filtered = filter_show_episodes(items.items, query.season);
            let filtered =
                apply_show_episodes_query_compatibility(filtered, &query, start_item_id, adjacent_to);
            let enable_total_record_count = query.enable_total_record_count.unwrap_or(true);
            let paged = paginate_resume_items(
                filtered,
                normalize_show_episodes_start_index(query.start_index),
                normalize_show_episodes_limit(query.limit),
                enable_total_record_count,
            );
            let mut payload = compat_items_query_result_json(
                paged,
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
            &format!("failed to query episodes: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct ShowEpisodesQuery {
    #[serde(rename = "UserId", alias = "userId", alias = "user_id")]
    user_id: Option<Uuid>,
    #[serde(rename = "Fields", alias = "fields")]
    _fields: Option<String>,
    #[serde(rename = "Season", alias = "season")]
    season: Option<i32>,
    #[serde(rename = "SeasonId", alias = "seasonId")]
    season_id: Option<String>,
    #[serde(rename = "IsMissing", alias = "isMissing")]
    is_missing: Option<bool>,
    #[serde(rename = "AdjacentTo", alias = "adjacentTo")]
    adjacent_to: Option<String>,
    #[serde(rename = "StartItemId", alias = "startItemId")]
    start_item_id: Option<String>,
    #[serde(rename = "StartIndex", alias = "startIndex", alias = "start_index")]
    start_index: Option<i64>,
    #[serde(rename = "Limit", alias = "limit")]
    limit: Option<i64>,
    #[serde(rename = "EnableTotalRecordCount", alias = "enableTotalRecordCount")]
    enable_total_record_count: Option<bool>,
    #[serde(rename = "EnableImages", alias = "enableImages")]
    _enable_images: Option<bool>,
    #[serde(rename = "ImageTypeLimit", alias = "imageTypeLimit")]
    _image_type_limit: Option<i32>,
    #[serde(rename = "EnableImageTypes", alias = "enableImageTypes")]
    _enable_image_types: Option<String>,
    #[serde(rename = "EnableUserData", alias = "enableUserData")]
    _enable_user_data: Option<bool>,
    #[serde(rename = "SortBy", alias = "sortBy")]
    sort_by: Option<String>,
    #[serde(rename = "SortOrder", alias = "sortOrder")]
    sort_order: Option<String>,
}

fn normalize_show_episodes_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(500).clamp(1, 500)
}

fn normalize_show_episodes_start_index(start_index: Option<i64>) -> i64 {
    start_index.unwrap_or(0).max(0)
}

fn filter_show_episodes(
    items: Vec<BaseItemDto>,
    season: Option<i32>,
) -> Vec<BaseItemDto> {
    items
        .into_iter()
        .filter(|item| {
            if let Some(season) = season {
                return item.parent_index_number == Some(season);
            }
            true
        })
        .collect()
}

fn filter_adjacent_show_items(items: Vec<BaseItemDto>, adjacent_to: Uuid) -> Vec<BaseItemDto> {
    let target = adjacent_to.to_string();
    let Some(pos) = items.iter().position(|item| item.id == target) else {
        return items;
    };
    let start = pos.saturating_sub(1);
    let end = (pos + 2).min(items.len());
    items
        .into_iter()
        .enumerate()
        .filter_map(|(idx, item)| {
            if (start..end).contains(&idx) {
                Some(item)
            } else {
                None
            }
        })
        .collect()
}

fn apply_show_episodes_query_compatibility(
    mut items: Vec<BaseItemDto>,
    query: &ShowEpisodesQuery,
    start_item_id: Option<Uuid>,
    adjacent_to: Option<Uuid>,
) -> Vec<BaseItemDto> {
    if query.is_missing == Some(true) {
        return vec![];
    }

    if let Some(start_item_id) = start_item_id {
        let target = start_item_id.to_string();
        let Some(start_pos) = items.iter().position(|item| item.id == target) else {
            return vec![];
        };
        items = items.into_iter().skip(start_pos).collect();
    }

    if let Some(adjacent_to) = adjacent_to {
        items = filter_adjacent_show_items(items, adjacent_to);
    }

    items
}

#[derive(Debug, Deserialize)]
struct SeasonsQuery {
    #[serde(rename = "UserId", alias = "userId", alias = "user_id")]
    user_id: Option<Uuid>,
    #[serde(rename = "Fields", alias = "fields")]
    _fields: Option<String>,
    #[serde(rename = "IsSpecialSeason", alias = "isSpecialSeason")]
    is_special_season: Option<bool>,
    #[serde(rename = "IsMissing", alias = "isMissing")]
    is_missing: Option<bool>,
    #[serde(rename = "AdjacentTo", alias = "adjacentTo")]
    adjacent_to: Option<String>,
    #[serde(rename = "EnableImages", alias = "enableImages")]
    _enable_images: Option<bool>,
    #[serde(rename = "ImageTypeLimit", alias = "imageTypeLimit")]
    _image_type_limit: Option<i32>,
    #[serde(rename = "EnableImageTypes", alias = "enableImageTypes")]
    _enable_image_types: Option<String>,
    #[serde(rename = "EnableUserData", alias = "enableUserData")]
    _enable_user_data: Option<bool>,
    #[serde(rename = "EnableTotalRecordCount", alias = "enableTotalRecordCount")]
    enable_total_record_count: Option<bool>,
}

async fn get_show_seasons(
    State(state): State<ApiContext>,
    AxPath(raw_series_id): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<SeasonsQuery>,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(user) => user,
        Err(resp) => return resp,
    };

    let user_id = query
        .user_id
        .or_else(|| Uuid::parse_str(&auth_user.id).ok());
    let series_id = match resolve_item_uuid_or_bad_request(&state, &raw_series_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    let adjacent_to = match resolve_optional_item_uuid(&state, query.adjacent_to.as_deref()).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };

    match state.infra.list_seasons(series_id, user_id).await {
        Ok(items) => {
            let compat = apply_show_seasons_query_compatibility(items, &query, adjacent_to);
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
            &format!("failed to query seasons: {err}"),
        ),
    }
}

fn apply_show_seasons_query_compatibility(
    mut result: QueryResultDto<BaseItemDto>,
    query: &SeasonsQuery,
    adjacent_to: Option<Uuid>,
) -> QueryResultDto<BaseItemDto> {
    if query.is_missing == Some(true) {
        result.items = vec![];
        result.total_record_count = 0;
        result.start_index = 0;
        return result;
    }

    result.items = result
        .items
        .into_iter()
        .filter(|item| match query.is_special_season {
            Some(true) => item.index_number == Some(0),
            Some(false) => item.index_number != Some(0),
            None => true,
        })
        .collect();

    if let Some(adjacent_to) = adjacent_to {
        result.items = filter_adjacent_seasons(result.items, adjacent_to);
    }

    result.total_record_count = if query.enable_total_record_count == Some(false) {
        0
    } else {
        result.items.len() as i32
    };
    result.start_index = 0;
    result
}

fn filter_adjacent_seasons(items: Vec<BaseItemDto>, adjacent_to: Uuid) -> Vec<BaseItemDto> {
    let target = adjacent_to.to_string();
    let Some(pos) = items.iter().position(|item| item.id == target) else {
        return items;
    };
    let start = pos.saturating_sub(1);
    let end = (pos + 2).min(items.len());
    items
        .into_iter()
        .enumerate()
        .filter_map(|(idx, item)| {
            if (start..end).contains(&idx) {
                Some(item)
            } else {
                None
            }
        })
        .collect()
}

#[derive(Debug, Deserialize)]
struct NextUpQuery {
    #[serde(rename = "UserId", alias = "userId", alias = "user_id")]
    user_id: Option<Uuid>,
    #[serde(rename = "StartIndex", alias = "startIndex", alias = "start_index")]
    start_index: Option<i64>,
    #[serde(rename = "Limit", alias = "limit")]
    limit: Option<i64>,
    #[serde(rename = "Fields", alias = "fields")]
    _fields: Option<String>,
    #[serde(rename = "SeriesId", alias = "seriesId")]
    series_id: Option<String>,
    #[serde(rename = "ParentId", alias = "parentId")]
    parent_id: Option<String>,
    #[serde(rename = "EnableImages", alias = "enableImages")]
    _enable_images: Option<bool>,
    #[serde(rename = "ImageTypeLimit", alias = "imageTypeLimit")]
    _image_type_limit: Option<i32>,
    #[serde(rename = "EnableImageTypes", alias = "enableImageTypes")]
    _enable_image_types: Option<String>,
    #[serde(rename = "EnableUserData", alias = "enableUserData")]
    _enable_user_data: Option<bool>,
    #[serde(rename = "NextUpDateCutoff", alias = "nextUpDateCutoff")]
    _next_up_date_cutoff: Option<String>,
    #[serde(rename = "EnableTotalRecordCount", alias = "enableTotalRecordCount")]
    enable_total_record_count: Option<bool>,
    #[serde(rename = "EnableResumable", alias = "enableResumable")]
    enable_resumable: Option<bool>,
    #[serde(rename = "EnableRewatching", alias = "enableRewatching")]
    enable_rewatching: Option<bool>,
    #[serde(rename = "MediaTypes", alias = "mediaTypes")]
    media_types: Option<String>,
    #[serde(rename = "DisableFirstEpisode", alias = "disableFirstEpisode")]
    disable_first_episode: Option<bool>,
}

fn apply_next_up_query_compatibility(
    items: Vec<BaseItemDto>,
    query: &NextUpQuery,
    parent_id: Option<Uuid>,
) -> Vec<BaseItemDto> {
    let media_types = split_csv(query.media_types.as_deref());
    items
        .into_iter()
        .filter(|item| resume_parent_matches(item, parent_id))
        .filter(|item| {
            if query.disable_first_episode.unwrap_or(false) {
                return item.index_number.unwrap_or(i32::MAX) > 1;
            }
            true
        })
        .filter(|item| resume_media_type_matches(&item.item_type, &media_types))
        .collect()
}

async fn get_shows_next_up(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<NextUpQuery>,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(user) => user,
        Err(resp) => return resp,
    };

    let user_id = query
        .user_id
        .or_else(|| Uuid::parse_str(&auth_user.id).ok());
    let series_id = match resolve_optional_item_uuid(&state, query.series_id.as_deref()).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    let parent_id = match resolve_optional_item_uuid(&state, query.parent_id.as_deref()).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };

    let Some(uid) = user_id else {
        return error_response(StatusCode::BAD_REQUEST, "UserId is required");
    };

    let start_index = normalize_next_up_start_index(query.start_index);
    let limit = normalize_next_up_limit(query.limit);
    let enable_total_record_count = query.enable_total_record_count.unwrap_or(true);
    let enable_resumable = query.enable_resumable.unwrap_or(true);
    let enable_rewatching = query.enable_rewatching.unwrap_or(false);

    match state
        .infra
        .list_next_up(uid, series_id, enable_resumable, enable_rewatching)
        .await
    {
        Ok(mut items) => {
            items.items = apply_next_up_query_compatibility(items.items, &query, parent_id);
            let total_record_count = if enable_total_record_count {
                items.items.len() as i32
            } else {
                0
            };
            items.items = items
                .items
                .into_iter()
                .skip(start_index as usize)
                .take(limit as usize)
                .collect();
            items.start_index = start_index as i32;
            items.total_record_count = total_record_count;
            let mut payload = compat_items_query_result_json(
                items,
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
            &format!("failed to query next up: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct ResumeItemsQuery {
    #[serde(rename = "UserId", alias = "userId", alias = "user_id")]
    user_id: Option<Uuid>,
    #[serde(rename = "StartIndex", alias = "startIndex", alias = "start_index")]
    start_index: Option<i64>,
    #[serde(rename = "Limit", alias = "limit")]
    limit: Option<i64>,
    #[serde(rename = "SearchTerm", alias = "searchTerm", alias = "search_term")]
    search_term: Option<String>,
    #[serde(rename = "ParentId", alias = "parentId", alias = "parent_id")]
    parent_id: Option<String>,
    #[serde(rename = "Fields", alias = "fields")]
    _fields: Option<String>,
    #[serde(rename = "MediaTypes", alias = "mediaTypes")]
    media_types: Option<String>,
    #[serde(rename = "EnableUserData", alias = "enableUserData")]
    _enable_user_data: Option<bool>,
    #[serde(rename = "ImageTypeLimit", alias = "imageTypeLimit")]
    _image_type_limit: Option<i32>,
    #[serde(rename = "EnableImageTypes", alias = "enableImageTypes")]
    _enable_image_types: Option<String>,
    #[serde(rename = "ExcludeItemTypes", alias = "excludeItemTypes")]
    exclude_item_types: Option<String>,
    #[serde(rename = "IncludeItemTypes", alias = "includeItemTypes")]
    include_item_types: Option<String>,
    #[serde(rename = "EnableTotalRecordCount", alias = "enableTotalRecordCount")]
    enable_total_record_count: Option<bool>,
    #[serde(rename = "EnableImages", alias = "enableImages")]
    _enable_images: Option<bool>,
    #[serde(rename = "ExcludeActiveSessions", alias = "excludeActiveSessions")]
    _exclude_active_sessions: Option<bool>,
}

async fn get_user_items_resume(
    State(state): State<ApiContext>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<ResumeItemsQuery>,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(user) => user,
        Err(resp) => return resp,
    };

    let user_id = query
        .user_id
        .or_else(|| Uuid::parse_str(&auth_user.id).ok());
    let Some(user_id) = user_id else {
        return error_response(StatusCode::BAD_REQUEST, "UserId is required");
    };
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
