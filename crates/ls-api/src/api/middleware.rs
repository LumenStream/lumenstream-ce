fn strip_compat_prefix_path(path: &str) -> Option<&str> {
    let stripped = path.strip_prefix('/')?;
    let mut parts = stripped.splitn(2, '/');
    let first = parts.next().unwrap_or_default();
    if first.eq_ignore_ascii_case("emby") || first.eq_ignore_ascii_case("jellyfin") {
        return Some(parts.next().unwrap_or_default());
    }
    None
}

fn compat_root_segment(segment: &str) -> Option<&'static str> {
    if segment.eq_ignore_ascii_case("system") {
        Some("System")
    } else if segment.eq_ignore_ascii_case("users") {
        Some("Users")
    } else if segment.eq_ignore_ascii_case("items") {
        Some("Items")
    } else if segment.eq_ignore_ascii_case("videos") {
        Some("Videos")
    } else if segment.eq_ignore_ascii_case("shows") {
        Some("Shows")
    } else if segment.eq_ignore_ascii_case("sessions") {
        Some("Sessions")
    } else if segment.eq_ignore_ascii_case("useritems") {
        Some("UserItems")
    } else if segment.eq_ignore_ascii_case("displaypreferences") {
        Some("DisplayPreferences")
    } else if segment.eq_ignore_ascii_case("branding") {
        Some("Branding")
    } else {
        None
    }
}

fn compat_segment_case(segment: &str) -> Option<&'static str> {
    // Root segments
    if let Some(root) = compat_root_segment(segment) {
        return Some(root);
    }

    // Common compatibility sub-paths. Keep this list focused and safe; dynamic
    // path params (UUIDs, indices, etc.) should not be modified.
    const CASED_SEGMENTS: &[(&str, &str)] = &[
        // System / Branding / DisplayPreferences
        ("info", "Info"),
        ("public", "Public"),
        ("endpoint", "Endpoint"),
        ("ping", "Ping"),
        ("activitylog", "ActivityLog"),
        ("entries", "Entries"),
        ("configuration", "Configuration"),
        ("logs", "Logs"),
        ("level", "Level"),
        ("log", "Log"),
        ("wakeonlaninfo", "WakeOnLanInfo"),
        ("css", "Css"),
        ("css.css", "Css.css"),
        // Users
        ("authenticatebyname", "AuthenticateByName"),
        ("authenticate", "Authenticate"),
        ("me", "Me"),
        ("connect", "Connect"),
        ("link", "Link"),
        ("forgotpassword", "ForgotPassword"),
        ("pin", "Pin"),
        ("new", "New"),
        ("easypassword", "EasyPassword"),
        ("password", "Password"),
        ("policy", "Policy"),
        ("playbackdomains", "PlaybackDomains"),
        ("select", "Select"),
        ("views", "Views"),
        ("groupingoptions", "GroupingOptions"),
        ("suggestions", "Suggestions"),
        ("root", "Root"),
        ("latest", "Latest"),
        ("intros", "Intros"),
        ("localtrailers", "LocalTrailers"),
        ("specialfeatures", "SpecialFeatures"),
        ("rating", "Rating"),
        ("playingitems", "PlayingItems"),
        ("playeditems", "PlayedItems"),
        ("favoriteitems", "FavoriteItems"),
        // Items / Shows / UserItems
        ("counts", "Counts"),
        ("topplayed", "TopPlayed"),
        ("filters", "Filters"),
        ("filters2", "Filters2"),
        ("prefixes", "Prefixes"),
        ("file", "File"),
        ("externalidinfos", "ExternalIdInfos"),
        ("remotesearch", "RemoteSearch"),
        ("refresh", "Refresh"),
        ("metadataeditor", "MetadataEditor"),
        ("instantmix", "InstantMix"),
        ("deleteinfo", "DeleteInfo"),
        ("similar", "Similar"),
        ("download", "Download"),
        ("ancestors", "Ancestors"),
        ("criticreviews", "CriticReviews"),
        ("thememedia", "ThemeMedia"),
        ("themesongs", "ThemeSongs"),
        ("themevideos", "ThemeVideos"),
        ("remoteimages", "RemoteImages"),
        ("thumbnailset", "ThumbnailSet"),
        ("apply", "Apply"),
        ("providers", "Providers"),
        ("playbackinfo", "PlaybackInfo"),
        ("subtitles", "Subtitles"),
        ("images", "Images"),
        ("nextup", "NextUp"),
        ("resume", "Resume"),
        ("userdata", "UserData"),
        ("episodes", "Episodes"),
        ("seasons", "Seasons"),
        // Sessions
        ("playing", "Playing"),
        ("progress", "Progress"),
        ("stopped", "Stopped"),
        ("capabilities", "Capabilities"),
        ("full", "Full"),
        ("command", "Command"),
        ("message", "Message"),
        ("system", "System"),
        ("users", "Users"),
        ("viewing", "Viewing"),
        ("logout", "Logout"),
    ];

    CASED_SEGMENTS.iter().find_map(|(raw, cased)| {
        if segment.eq_ignore_ascii_case(raw) {
            Some(*cased)
        } else {
            None
        }
    })
}

fn should_normalize_compat_path(path: &str) -> bool {
    let stripped = path.strip_prefix('/').unwrap_or(path);
    let root = stripped.split('/').next().unwrap_or_default();
    compat_root_segment(root).is_some()
}

fn normalize_compat_path_case(path: &str) -> Option<(String, bool)> {
    if path.is_empty() || path == "/" {
        return None;
    }

    if !should_normalize_compat_path(path) {
        return None;
    }

    let mut out = String::with_capacity(path.len());
    out.push('/');

    let mut changed = false;
    let mut saw_subtitles = false;
    for (idx, segment) in path.trim_start_matches('/').split('/').enumerate() {
        if idx > 0 {
            out.push('/');
        }

        // `stream` is ambiguous:
        // - Video stream endpoints use `/Videos/{id}/stream...` (lowercase `stream`)
        // - Subtitle stream endpoints use `/Videos/{id}/Subtitles/{i}/Stream...` (capital `Stream`)
        // Decide based on whether the path includes a `Subtitles` segment.
        if segment.eq_ignore_ascii_case("stream") {
            let replacement = if saw_subtitles { "Stream" } else { "stream" };
            if replacement != segment {
                changed = true;
            }
            out.push_str(replacement);
            continue;
        }

        if segment.len() > 7 && segment[..7].eq_ignore_ascii_case("stream.") {
            let replacement_prefix = if saw_subtitles { "Stream." } else { "stream." };
            if !segment.starts_with(replacement_prefix) {
                changed = true;
            }
            out.push_str(replacement_prefix);
            out.push_str(&segment[7..]);
            continue;
        }

        let replacement = compat_segment_case(segment).unwrap_or(segment);
        if replacement != segment {
            changed = true;
        }
        if replacement == "Subtitles" {
            saw_subtitles = true;
        }
        out.push_str(replacement);
    }

    if changed { Some((out, true)) } else { None }
}

/// Strip `/emby` or `/jellyfin` prefix so a single set of routes handles all three.
/// Also injects `Content-Type: application/json` for body-bearing requests that omit it,
/// so Jellyfin clients (e.g. SenPlayer) that skip the header still parse correctly.
async fn strip_compat_prefix(
    mut req: ServiceRequest,
    next: Next<impl MessageBody + 'static>,
) -> Result<ServiceResponse<BoxBody>, Error> {
    let mut path = req.path().to_string();
    let mut updated = false;

    // Normalize `/emby` and `/jellyfin` prefixes case-insensitively.
    if let Some(rest) = strip_compat_prefix_path(path.as_str()) {
        path = if rest.is_empty() {
            "/".to_string()
        } else {
            format!("/{rest}")
        };
        updated = true;
    }

    // Normalize common compatibility endpoints' path casing (e.g. `/videos` -> `/Videos`).
    if let Some((normalized, _)) = normalize_compat_path_case(path.as_str()) {
        path = normalized;
        updated = true;
    }

    if updated {
        let new_uri: http::Uri = if req.query_string().is_empty() {
            path.parse().unwrap()
        } else {
            format!("{}?{}", path, req.query_string()).parse().unwrap()
        };
        req.head_mut().uri = new_uri.clone();
        req.match_info_mut().get_mut().update(&new_uri);
    }

    // Inject Content-Type for body-bearing methods that omit it
    let method = req.method();
    if matches!(*method, http::Method::POST | http::Method::PUT | http::Method::PATCH) {
        let has_ct = req.headers().contains_key(header::CONTENT_TYPE);
        if !has_ct {
            req.headers_mut().insert(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static("application/json"),
            );
        }
    }

    next.call(req).await.map(|res| res.map_into_boxed_body())
}

async fn request_context_middleware(
    mut req: ServiceRequest,
    next: Next<impl MessageBody + 'static>,
) -> Result<ServiceResponse<BoxBody>, Error> {
    let state = req.app_data::<web::Data<ApiContext>>().cloned();
    if let Some(state) = &state {
        state.metrics.requests_total.fetch_add(1, Ordering::Relaxed);

        let peer_ip = parse_peer_ip(req.connection_info().peer_addr());
        let headers = HeaderMap(req.headers().clone());
        let client_ip = extract_client_ip(&headers, peer_ip, &state.infra.config_snapshot().security);
        req.headers_mut().remove(INTERNAL_CLIENT_IP_HEADER);
        if let Some(ip) = client_ip.as_ref() {
            if let Ok(value) = header::HeaderValue::from_str(ip) {
                req.headers_mut().insert(
                    header::HeaderName::from_static(INTERNAL_CLIENT_IP_HEADER),
                    value,
                );
            }
        }
    }

    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.trim().is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| Uuid::now_v7().to_string());

    let method = req.method().to_string();
    let path = req.path().to_string();
    let started = Instant::now();
    let mut response = next.call(req).await?.map_into_boxed_body();
    let duration_ms = started.elapsed().as_millis() as u64;

    if let Some(state) = &state {
        state.metrics.record_latency(duration_ms);
    }

    if let Ok(value) = header::HeaderValue::from_str(&request_id) {
        response
            .headers_mut()
            .insert(header::HeaderName::from_static("x-request-id"), value);
    }

    let status = response.status().as_u16();
    if let Some(state) = &state {
        match status {
            200..=299 => {
                state.metrics.status_2xx.fetch_add(1, Ordering::Relaxed);
            }
            400..=499 => {
                state.metrics.status_4xx.fetch_add(1, Ordering::Relaxed);
            }
            _ => {
                state.metrics.status_5xx.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    info!(
        request_id = %request_id,
        method = %method,
        path = %path,
        status,
        duration_ms,
        "request handled"
    );

    Ok(response)
}

#[cfg(test)]
mod middleware_compat_case_tests {
    use super::strip_compat_prefix;

    use actix_web::{App, HttpResponse, test, web};
    use actix_web::http::StatusCode;
    use uuid::Uuid;

    async fn ok() -> HttpResponse {
        HttpResponse::Ok().finish()
    }

    async fn no_content() -> HttpResponse {
        HttpResponse::NoContent().finish()
    }

    async fn accepted() -> HttpResponse {
        HttpResponse::Accepted().finish()
    }

    async fn user_items_root(_path: web::Path<(Uuid,)>) -> HttpResponse {
        HttpResponse::Ok().finish()
    }

    async fn user_items_latest(_path: web::Path<(Uuid,)>) -> HttpResponse {
        HttpResponse::NoContent().finish()
    }

    async fn user_item_detail(_path: web::Path<(Uuid, Uuid)>) -> HttpResponse {
        HttpResponse::Created().finish()
    }

    async fn user_views(_path: web::Path<(Uuid,)>) -> HttpResponse {
        HttpResponse::Ok().finish()
    }

    async fn item_detail(_path: web::Path<(Uuid,)>) -> HttpResponse {
        HttpResponse::Created().finish()
    }

    #[actix_web::test]
    async fn compat_prefix_strips_and_normalizes_videos_case() {
        let app = test::init_service(
            App::new()
                .wrap(actix_web::middleware::from_fn(strip_compat_prefix))
                .route(
                    "/Videos/{item_id}/stream.{container}",
                    web::get().to(ok),
                ),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/emby/videos/abc/stream.matroska?api_key=token")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn compat_prefix_is_case_insensitive() {
        let app = test::init_service(
            App::new()
                .wrap(actix_web::middleware::from_fn(strip_compat_prefix))
                .route(
                    "/Videos/{item_id}/stream.{container}",
                    web::get().to(ok),
                ),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/Emby/videos/abc/stream.mkv?api_key=token")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn compat_prefix_preserves_subtitle_stream_case() {
        let app = test::init_service(
            App::new()
                .wrap(actix_web::middleware::from_fn(strip_compat_prefix))
                .route(
                    "/Videos/{item_id}/Subtitles/{subtitle_index}/Stream",
                    web::get().to(ok),
                ),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/emby/videos/abc/subtitles/0/stream")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let req = test::TestRequest::get()
            .uri("/emby/Videos/abc/Subtitles/0/Stream")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn compat_prefix_normalizes_users_items_static_segments_before_uuid_routes() {
        let app = test::init_service(
            App::new()
                .wrap(actix_web::middleware::from_fn(strip_compat_prefix))
                .route("/Users/{user_id}/Items/Root", web::get().to(user_items_root))
                .route("/Users/{user_id}/Items/Latest", web::get().to(user_items_latest))
                .route("/Users/{user_id}/Items/{item_id}", web::get().to(user_item_detail)),
        )
        .await;

        let user_id = Uuid::now_v7();
        let req = test::TestRequest::get()
            .uri(format!("/emby/users/{user_id}/items/root").as_str())
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let req = test::TestRequest::get()
            .uri(format!("/emby/users/{user_id}/items/latest").as_str())
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[actix_web::test]
    async fn compat_prefix_normalizes_users_views_segment() {
        let app = test::init_service(
            App::new()
                .wrap(actix_web::middleware::from_fn(strip_compat_prefix))
                .route("/Users/{user_id}/Views", web::get().to(user_views)),
        )
        .await;

        let user_id = Uuid::now_v7();
        let req = test::TestRequest::get()
            .uri(format!("/emby/users/{user_id}/views").as_str())
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn compat_prefix_normalizes_items_static_segments_before_uuid_routes() {
        let app = test::init_service(
            App::new()
                .wrap(actix_web::middleware::from_fn(strip_compat_prefix))
                .route("/Items/Counts", web::get().to(no_content))
                .route("/Items/TopPlayed", web::get().to(accepted))
                .route("/Items/Filters", web::get().to(ok))
                .route("/Items/Filters2", web::get().to(no_content))
                .route("/Items/{item_id}", web::get().to(item_detail)),
        )
        .await;

        let req = test::TestRequest::get().uri("/emby/items/counts").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        let req = test::TestRequest::get()
            .uri("/emby/items/topplayed")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::ACCEPTED);

        let req = test::TestRequest::get().uri("/emby/items/filters").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let req = test::TestRequest::get()
            .uri("/emby/items/filters2")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }
}
