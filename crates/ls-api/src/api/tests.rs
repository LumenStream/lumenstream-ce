#[cfg(test)]
mod tests {
    use super::{
        AdminBatchUserStatusRequest, AdminBillingConfigResponse, AdminCreateLibraryRequest,
        AdminCreateLumenBackendNodeRequest,
        AdminLibraryStatusDto, AdminLibraryStatusResponse, AdminPatchLibraryRequest,
        AdminPatchLumenBackendNodeRequest,
        AdminPatchTaskDefinitionRequest, AdminPatchUserProfileRequest, AdminRunTaskRequest,
        AdminUpdateBillingConfigRequest, AdminUpdateEpayConfigRequest,
        AdminUpdateSystemFlagsRequest, AdminUpsertBillingPermissionGroupRequest,
        AdminUpsertBillingPlanRequest, AdminUpsertPlaybackDomainRequest,
        AdminUserSummaryQueryRequest, ApiMetrics, BaseItemDto,
        HeaderMap, Uri, apply_billing_config_update,
        apply_items_query_compatibility, apply_next_up_query_compatibility,
        apply_show_episodes_query_compatibility, apply_show_seasons_query_compatibility,
        apply_episode_series_context,
        compat_items_query_result_json, compat_metadata_lookup_query_result_json,
        compat_single_item_json, compat_latest_items_json,
        compat_item_id_json_value, media_source_item_id_candidate,
        compat_system_info_capability_flags,
        default_session_play_state, derive_internal_device_id, session_device_key,
        session_play_state_with_playback,
        session_is_recent,
        apply_epay_config_update, apply_system_flags_update, build_system_flags_response,
        apply_invite_settings_update, invite_settings_payload, invite_summary_payload,
        mask_admin_user_manage_profile_payload, mask_admin_user_summary_page_payload,
        extract_client_ip, extract_emby_authorization_param,
        extract_emby_authorization_param_from_headers, extract_token, has_is_resumable_filter,
        image_content_type,
        image_tag_header_value, infer_image_extension_from_content_type,
        is_supported_library_image_type,
        ip_matches_allow_entries, is_supported_legacy_user_image_index, map_billing_error,
        map_invite_error, map_items_query_error,
        map_playlist_error, map_stream_admission_error, map_task_center_error, mask_web_settings,
        normalize_remote_image_type, normalize_user_image_type,
        compat_public_users_payload,
        build_items_query_options, filter_item_types_by_media_types, filter_query_flag, filter_show_episodes, merge_secret_placeholders,
        normalize_next_up_limit,
        normalize_next_up_start_index, normalize_show_episodes_limit,
        normalize_show_episodes_start_index,
        normalize_resume_limit, normalize_resume_start_index,
        normalize_search_hints_include_item_types, normalize_search_hints_limit,
        normalize_search_hints_start_index, normalize_top_played_limit,
        normalize_top_played_window_days, paginate_resume_items, parse_range, parse_range_start,
        normalize_my_traffic_window_days, MyTrafficUsageMediaQuery,
        parse_compat_uuid,
        parse_authenticate_by_name_payload,
        parse_legacy_favorite_action, parse_legacy_played_action,
        parse_top_played_stat_date, parse_user_role, parse_user_role_strict, parse_uuid_csv,
        normalize_optional_item_id,
        normalize_person_types, normalize_mac_for_wol, resolve_emby_client_context,
        resolve_emby_client_context_with_query,
        resolve_request_address, supports_https,
        apply_playback_payload_client_context, playing_ping_success_response, resolve_new_password, ImageRequestCompatQuery,
        metadata_patch_from_payload, parse_remote_subtitle_track_id, default_external_id_provider_keys,
        PlaybackInfoCompatQuery, PlaybackInfoCompatRequest, StreamVideoCompatQuery,
        apply_playback_original_stream_target, apply_item_external_subtitle_delivery_urls,
        build_playback_original_stream_url, build_subtitle_delivery_url,
        ensure_playback_info_compat_defaults, ensure_playback_info_original_stream_urls,
        media_source_is_remote_for_redirect, normalize_media_source_id_candidate,
        SubtitleStreamCompatQuery,
        percentile_from_sorted, resume_item_type_matches, resume_media_type_matches,
        resume_name_matches_search, find_person_by_name, find_root_collection_item, person_lookup_id, remote_search_name_from_payload,
        remote_search_include_item_types, remote_search_provider_ids_from_payload, remote_search_year_from_payload, split_csv, subtitle_codec_from_path,
        subtitle_language_matches,
        subtitle_content_type,
        system_ping_response, AuthenticateByNameRequest, AuthenticateUserByIdRequest,
        ConnectLinkQuery, CreateUserByName, DisplayPreferencesQuery, ForgotPasswordPinRequest,
        ForgotPasswordRequest, GenresQuery, HideFromResumeQuery, ItemFileQueryCompat, ItemRatingQuery, ItemsFiltersQuery,
        ItemsQuery, LatestItemsQuery, LibraryRefreshQuery, MoveItemImageIndexQuery, NextUpQuery,
        NamedValueListQuery,
        PlayingItemQuery, QueryResultDto, SearchHintsQuery, SeasonsQuery, SessionCapabilitiesQuery,
        ShowEpisodesQuery, SystemActivityLogQuery, SystemLogQuery, UpdateUserPassword,
        StudioBrowseItemDto, StudioBrowseQueryResultDto, StudioBrowseUserDataDto,
        UserConfiguration, UserItemDataUpdateBody, UserPolicyUpdate, UsersQuery,
        normalize_latest_items_limit, infer_latest_item_types_for_library,
        AddMediaPathRequest, AddVirtualFolderRequest,
        DeleteItemsQuery, ItemAncestorsQuery, RenameVirtualFolderRequest,
        UpdateLibraryOptionsRequest, UpdateMediaPathRequest, library_to_base_item_dto,
        paginate_named_values, resolve_media_path,
        serve_image_file, merge_agent_secret_placeholders,
        EmbyCreatePlaylistRequest, EmbyPlaylistItemsQuery, EmbyCreateCollectionQuery, LumenBackendRegisterRequest,
    };
    use actix_web::body::MessageBody;
    use chrono::Utc;
    use actix_web::http::{
        StatusCode,
        header::{self, HeaderValue},
    };
    use ls_config::{AgentConfig, BillingConfig, EpayConfig, SecurityConfig, WebAppConfig};
    use ls_domain::{
        jellyfin::{ItemCountsDto, PlaybackProgressDto, PublicSystemInfoDto},
        model::{AdminPlaybackSession, AuthSession, Library, UserRole},
    };
    use ls_infra::{ImageResizeFormat, InfraError, StreamAccessDeniedReason};
    use serde_json::{Value, json};
    use std::{net::IpAddr, sync::atomic::Ordering};
    use uuid::Uuid;

    fn make_test_item(id: &str, name: &str, item_type: &str) -> BaseItemDto {
        BaseItemDto {
            id: id.to_string(),
            name: name.to_string(),
            item_type: item_type.to_string(),
            path: String::new(),
            is_folder: Some(false),
            media_type: None,
            container: None,
            location_type: None,
            can_delete: Some(false),
            can_download: Some(false),
            collection_type: None,
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
            date_created: None,
            child_count: None,
            recursive_item_count: None,
            play_access: None,
        }
    }

    #[test]
    fn extract_token_prefers_x_emby_token() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::HeaderName::from_static("x-emby-token"),
            HeaderValue::from_static("token_a"),
        );
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer token_b"),
        );
        let uri: Uri = "/Items?api_key=token_c".parse().expect("uri");

        let token = extract_token(&headers, &uri);
        assert_eq!(token.as_deref(), Some("token_a"));
    }

    #[test]
    fn extract_token_reads_bearer_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer token_b"),
        );
        let uri: Uri = "/Items".parse().expect("uri");

        let token = extract_token(&headers, &uri);
        assert_eq!(token.as_deref(), Some("token_b"));
    }

    #[test]
    fn extract_token_reads_x_emby_authorization_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::HeaderName::from_static("x-emby-authorization"),
            HeaderValue::from_static(
                "MediaBrowser Token=\"token_from_emby_auth\", UserId=\"u1\", Client=\"SenPlayer\"",
            ),
        );
        let uri: Uri = "/Items".parse().expect("uri");

        let token = extract_token(&headers, &uri);
        assert_eq!(token.as_deref(), Some("token_from_emby_auth"));
    }

    #[test]
    fn extract_token_reads_vidhub_style_emby_authorization_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::HeaderName::from_static("x-emby-authorization"),
            HeaderValue::from_static(
                "MediaBrowser UserId=\"u1\", Client=\"VidHub\", Device=\"Mac\", DeviceId=\"d1\", Version=\"2.1.4\", Token=\"token_from_vidhub\"",
            ),
        );
        let uri: Uri = "/Shows/NextUp".parse().expect("uri");

        let token = extract_token(&headers, &uri);
        assert_eq!(token.as_deref(), Some("token_from_vidhub"));
    }

    #[test]
    fn extract_token_normalizes_quoted_x_emby_token() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::HeaderName::from_static("x-emby-token"),
            HeaderValue::from_static("\"token_quoted\""),
        );
        let uri: Uri = "/Items".parse().expect("uri");

        let token = extract_token(&headers, &uri);
        assert_eq!(token.as_deref(), Some("token_quoted"));
    }

    #[test]
    fn extract_token_reads_mediabrowser_authorization_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static(
                "MediaBrowser Token=\"token_from_authorization\", Device=\"Mac\"",
            ),
        );
        let uri: Uri = "/Items".parse().expect("uri");

        let token = extract_token(&headers, &uri);
        assert_eq!(token.as_deref(), Some("token_from_authorization"));
    }

    #[test]
    fn extract_token_reads_query_api_key() {
        let headers = HeaderMap::new();
        let uri: Uri = "/Items?api_key=token_query".parse().expect("uri");

        let token = extract_token(&headers, &uri);
        assert_eq!(token.as_deref(), Some("token_query"));
    }

    #[test]
    fn extract_token_reads_query_token() {
        let headers = HeaderMap::new();
        let uri: Uri = "/Items?token=token_query".parse().expect("uri");

        let token = extract_token(&headers, &uri);
        assert_eq!(token.as_deref(), Some("token_query"));
    }

    #[test]
    fn extract_emby_authorization_param_reads_client_context_fields() {
        let raw = "MediaBrowser Version=\"8.3.1\", Device=\"Mac\", DeviceId=\"abc\", Client=\"Infuse-Direct\", Token=\"t1\"";
        assert_eq!(
            extract_emby_authorization_param(raw, "Client").as_deref(),
            Some("Infuse-Direct")
        );
        assert_eq!(
            extract_emby_authorization_param(raw, "Device").as_deref(),
            Some("Mac")
        );
        assert_eq!(
            extract_emby_authorization_param(raw, "DeviceId").as_deref(),
            Some("abc")
        );
        assert_eq!(
            extract_emby_authorization_param(raw, "Version").as_deref(),
            Some("8.3.1")
        );
    }

    #[test]
    fn extract_emby_authorization_param_from_headers_supports_x_emby_and_authorization() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::HeaderName::from_static("x-emby-authorization"),
            HeaderValue::from_static("MediaBrowser Client=\"Infuse-Direct\""),
        );
        assert_eq!(
            extract_emby_authorization_param_from_headers(&headers, "Client").as_deref(),
            Some("Infuse-Direct")
        );

        headers.remove(header::HeaderName::from_static("x-emby-authorization"));
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("MediaBrowser Version=\"9.1.0\", Token=\"abc\""),
        );
        assert_eq!(
            extract_emby_authorization_param_from_headers(&headers, "Version").as_deref(),
            Some("9.1.0")
        );
    }

    #[test]
    fn resolve_emby_client_context_falls_back_to_emby_authorization_values() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::HeaderName::from_static("x-emby-authorization"),
            HeaderValue::from_static(
                "MediaBrowser Version=\"8.3.1\", Device=\"Mac\", DeviceId=\"dev-001\", Client=\"Infuse-Direct\"",
            ),
        );

        let context = resolve_emby_client_context(&headers);
        assert_eq!(context.client, "Infuse-Direct");
        assert_eq!(context.device_name, "Mac");
        assert_eq!(context.device_id, "dev-001");
        assert_eq!(context.application_version.as_deref(), Some("8.3.1"));
    }

    #[test]
    fn resolve_emby_client_context_uses_native_user_agent_fallback() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::USER_AGENT,
            HeaderValue::from_static("VidHub/2.1.4"),
        );

        let context = resolve_emby_client_context(&headers);
        assert_eq!(context.client, "VidHub");
        assert_eq!(context.device_name, "VidHub");
        assert_eq!(context.device_id, "");
        assert_eq!(context.application_version.as_deref(), Some("2.1.4"));
    }

    #[test]
    fn resolve_emby_client_context_ignores_browser_user_agent_fallback() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 Chrome/145.0",
            ),
        );

        let context = resolve_emby_client_context(&headers);
        assert_eq!(context.client, "ls-client");
        assert_eq!(context.device_name, "ls-device");
        assert_eq!(context.device_id, "");
        assert!(context.application_version.is_none());
    }

    #[test]
    fn resolve_emby_client_context_uses_query_fallback_values() {
        let headers = HeaderMap::new();
        let query = "X-Emby-Client=AfuseKt%2F%28Linux%3BAndroid+Release%29Player&X-Emby-Device-Name=Redmi-22011211C&X-Emby-Device-Id=c667446cfbb8ff91&X-Emby-Client-Version=2.9.8.1";

        let context = resolve_emby_client_context_with_query(&headers, Some(query));
        assert_eq!(context.client, "AfuseKt/(Linux;Android Release)Player");
        assert_eq!(context.device_name, "Redmi-22011211C");
        assert_eq!(context.device_id, "c667446cfbb8ff91");
        assert_eq!(context.application_version.as_deref(), Some("2.9.8.1"));
    }

    #[test]
    fn resolve_emby_client_context_prefers_header_over_query_values() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::HeaderName::from_static("x-emby-client"),
            HeaderValue::from_static("HeaderClient"),
        );
        let query =
            "X-Emby-Client=QueryClient&X-Emby-Device-Name=QueryDevice&X-Emby-Client-Version=2.0.0";

        let context = resolve_emby_client_context_with_query(&headers, Some(query));
        assert_eq!(context.client, "HeaderClient");
        assert_eq!(context.device_name, "QueryDevice");
        assert_eq!(context.application_version.as_deref(), Some("2.0.0"));
    }

    #[test]
    fn apply_playback_payload_client_context_fills_missing_fields() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::USER_AGENT,
            HeaderValue::from_static("Infuse-Direct/8.3.1"),
        );
        let mut payload = PlaybackProgressDto {
            play_session_id: None,
            item_id: None,
            position_ticks: None,
            play_method: None,
            device_name: None,
            client: None,
            extra: json!({}),
        };

        apply_playback_payload_client_context(&mut payload, &headers);
        assert_eq!(payload.client.as_deref(), Some("Infuse-Direct"));
        assert_eq!(payload.device_name.as_deref(), Some("Infuse-Direct"));
    }

    #[test]
    fn apply_playback_payload_client_context_keeps_existing_fields() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::HeaderName::from_static("x-emby-client"),
            HeaderValue::from_static("VidHub"),
        );
        headers.insert(
            header::HeaderName::from_static("x-emby-device-name"),
            HeaderValue::from_static("Mac"),
        );
        let mut payload = PlaybackProgressDto {
            play_session_id: None,
            item_id: None,
            position_ticks: None,
            play_method: None,
            device_name: Some("Apple TV".to_string()),
            client: Some("ManualClient".to_string()),
            extra: json!({}),
        };

        apply_playback_payload_client_context(&mut payload, &headers);
        assert_eq!(payload.client.as_deref(), Some("ManualClient"));
        assert_eq!(payload.device_name.as_deref(), Some("Apple TV"));
    }

    #[test]
    fn resolve_request_address_uses_forwarded_headers_for_public_url() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::HeaderName::from_static("x-forwarded-proto"),
            HeaderValue::from_static("https"),
        );
        headers.insert(
            header::HeaderName::from_static("x-forwarded-host"),
            HeaderValue::from_static("ls-api.lumenstream-team.org"),
        );
        assert_eq!(
            resolve_request_address(&headers).as_deref(),
            Some("https://ls-api.lumenstream-team.org")
        );

        let mut fallback = HeaderMap::new();
        fallback.insert(header::HOST, HeaderValue::from_static("127.0.0.1:8096"));
        assert_eq!(
            resolve_request_address(&fallback).as_deref(),
            Some("http://127.0.0.1:8096")
        );
    }

    #[test]
    fn system_ping_response_matches_jellyfin_heartbeat_contract() {
        let response = system_ping_response();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some("text/plain")
        );
        let body = response
            .into_body()
            .try_into_bytes()
            .expect("system ping body bytes");
        assert_eq!(body.as_ref(), b"Jellyfin Server");
    }

    #[test]
    fn public_system_info_includes_both_single_and_array_address_fields() {
        let payload = PublicSystemInfoDto {
            local_address: Some("http://127.0.0.1:8096".to_string()),
            wan_address: Some("https://demo.example.com".to_string()),
            server_name: "lumenstream".to_string(),
            version: "4.9.1.26".to_string(),
            id: "server-id".to_string(),
            local_addresses: vec!["http://127.0.0.1:8096".to_string()],
            remote_addresses: vec!["https://demo.example.com".to_string()],
        };
        let value = serde_json::to_value(payload).expect("serialize public system info");

        assert_eq!(
            value.get("LocalAddress"),
            Some(&json!("http://127.0.0.1:8096"))
        );
        assert_eq!(value.get("WanAddress"), Some(&json!("https://demo.example.com")));
        assert_eq!(
            value.get("LocalAddresses"),
            Some(&json!(["http://127.0.0.1:8096"]))
        );
        assert_eq!(
            value.get("RemoteAddresses"),
            Some(&json!(["https://demo.example.com"]))
        );
        assert_eq!(value.get("Version"), Some(&json!("4.9.1.26")));
    }

    #[test]
    fn compat_public_users_payload_returns_empty_array() {
        assert_eq!(compat_public_users_payload(), json!([]));
    }

    #[test]
    fn system_info_capability_flags_match_compat_defaults() {
        let (can_self_restart, has_update_available, hardware_acceleration_requires_premiere) =
            compat_system_info_capability_flags();
        assert!(can_self_restart);
        assert!(has_update_available);
        assert!(hardware_acceleration_requires_premiere);
    }

    #[test]
    fn default_session_play_state_contains_compat_fields() {
        let play_state = default_session_play_state();
        let object = play_state.as_object().expect("play state object");
        assert_eq!(object.get("CanSeek"), Some(&json!(false)));
        assert_eq!(object.get("IsPaused"), Some(&json!(false)));
        assert_eq!(object.get("IsMuted"), Some(&json!(false)));
        assert_eq!(object.get("PlayMethod"), Some(&json!("DirectPlay")));
        assert_eq!(object.get("RepeatMode"), Some(&json!("RepeatNone")));
        assert_eq!(object.get("SleepTimerMode"), Some(&json!("None")));
        assert_eq!(object.get("SubtitleOffset"), Some(&json!(0)));
        assert_eq!(object.get("Shuffle"), Some(&json!(false)));
        assert_eq!(object.get("PlaybackRate"), Some(&json!(1)));
    }

    #[test]
    fn derive_internal_device_id_is_stable_and_positive() {
        let session_id = Uuid::parse_str("019c93b9-3088-7f32-81dd-4b845cd529e9").expect("uuid");
        let device_id = "CD8325D7-C904-421F-B116-FCEBA5D95C74";
        let first = derive_internal_device_id(session_id, device_id);
        let second = derive_internal_device_id(session_id, device_id);
        assert_eq!(first, second);
        assert!(first >= 0);
    }

    #[test]
    fn session_is_recent_filters_stale_sessions() {
        let now = Utc::now();
        assert!(session_is_recent(now - chrono::Duration::minutes(5)));
        assert!(!session_is_recent(now - chrono::Duration::minutes(31)));
    }

    #[test]
    fn session_device_key_prefers_device_id_and_normalizes_whitespace() {
        let user_id = Uuid::new_v4();
        let first = AuthSession {
            id: Uuid::new_v4(),
            user_id,
            user_name: "alice".to_string(),
            client: Some("iOS".to_string()),
            device_name: Some("Alice iPhone".to_string()),
            device_id: Some("  device-001  ".to_string()),
            remote_addr: None,
            is_active: true,
            created_at: Utc::now(),
            last_seen_at: Utc::now(),
        };
        let second = AuthSession {
            id: Uuid::new_v4(),
            device_id: Some("device-001".to_string()),
            ..first.clone()
        };

        assert_eq!(session_device_key(&first), session_device_key(&second));
    }

    #[test]
    fn session_play_state_with_playback_uses_latest_position_and_play_method() {
        let playback = AdminPlaybackSession {
            id: Uuid::new_v4(),
            play_session_id: "play-1".to_string(),
            user_id: Uuid::new_v4(),
            user_name: "alice".to_string(),
            media_item_id: Some(Uuid::new_v4()),
            media_item_name: Some("Episode 1".to_string()),
            device_name: Some("Alice iPhone".to_string()),
            client_name: Some("iOS".to_string()),
            play_method: Some("Transcode".to_string()),
            position_ticks: 42_000,
            is_active: true,
            last_heartbeat_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let state = session_play_state_with_playback(Some(&playback));
        let object = state.as_object().expect("play state object");
        assert_eq!(object.get("CanSeek"), Some(&json!(true)));
        assert_eq!(object.get("PlayMethod"), Some(&json!("Transcode")));
        assert_eq!(object.get("PositionTicks"), Some(&json!(42_000)));
    }

    #[test]
    fn normalize_mac_for_wol_accepts_common_mac_formats() {
        assert_eq!(
            normalize_mac_for_wol("16:5c:21:b6:bd:a0"),
            Some("165C21B6BDA0".to_string())
        );
        assert_eq!(
            normalize_mac_for_wol("16-5c-21-b6-bd-a0"),
            Some("165C21B6BDA0".to_string())
        );
        assert_eq!(
            normalize_mac_for_wol("165c21b6bda0"),
            Some("165C21B6BDA0".to_string())
        );
        assert_eq!(normalize_mac_for_wol("invalid"), None);
    }

    #[test]
    fn supports_https_checks_forwarded_proto_and_wan_address() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::HeaderName::from_static("x-forwarded-proto"),
            HeaderValue::from_static("https"),
        );
        assert!(supports_https(&headers, "http://127.0.0.1:8096"));

        let mut headers = HeaderMap::new();
        headers.insert(
            header::HeaderName::from_static("x-forwarded-port"),
            HeaderValue::from_static("443"),
        );
        assert!(supports_https(&headers, "http://127.0.0.1:8096"));

        let mut headers = HeaderMap::new();
        headers.insert(
            header::HOST,
            HeaderValue::from_static("ls-api.lumenstream-team.org"),
        );
        assert!(supports_https(&headers, "http://127.0.0.1:8096"));

        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, HeaderValue::from_static("127.0.0.1:8096"));
        assert!(!supports_https(&headers, "http://127.0.0.1:8096"));

        let headers = HeaderMap::new();
        assert!(supports_https(&headers, "https://lumenstream.example.com"));
        assert!(!supports_https(&headers, "http://127.0.0.1:8096"));
    }

    #[test]
    fn playing_ping_success_response_returns_empty_no_content() {
        let response = playing_ping_success_response();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let body = response
            .into_body()
            .try_into_bytes()
            .expect("playing ping response body");
        assert!(body.is_empty());
    }

    #[test]
    fn subtitle_helpers_map_codec_and_content_type() {
        assert_eq!(subtitle_codec_from_path("/tmp/a.zh.ass"), "ass");
        assert_eq!(subtitle_content_type("ass"), "text/x-ssa");
        assert_eq!(subtitle_content_type("srt"), "application/x-subrip");
        assert_eq!(
            subtitle_content_type("unknown"),
            "text/plain; charset=utf-8"
        );
    }

    #[test]
    fn split_csv_and_resumable_filter_helpers() {
        let v = split_csv(Some("Movie, Episode ,,"));
        assert_eq!(v, vec!["Movie".to_string(), "Episode".to_string()]);
        assert!(has_is_resumable_filter(Some("IsResumable,IsFavorite")));
        assert!(!has_is_resumable_filter(Some("IsFavorite")));
    }

    #[test]
    fn filter_query_flag_supports_likes_and_dislikes_aliases() {
        assert_eq!(
            filter_query_flag(Some("Likes,IsResumable"), "Likes", "Dislikes"),
            Some(true)
        );
        assert_eq!(
            filter_query_flag(Some("Dislikes"), "Likes", "Dislikes"),
            Some(false)
        );
        assert_eq!(
            filter_query_flag(Some("IsFavorite"), "Likes", "Dislikes"),
            None
        );
    }

    #[test]
    fn build_items_query_options_maps_likes_dislikes_to_is_favorite() {
        let likes_query: ItemsQuery =
            serde_json::from_value(json!({ "Filters": "Likes", "Limit": 24 }))
                .expect("likes query parse");
        let likes_options = build_items_query_options(None, &likes_query, None);
        assert_eq!(likes_options.is_favorite, Some(true));

        let dislikes_query: ItemsQuery =
            serde_json::from_value(json!({ "Filters": "Dislikes", "Limit": 24 }))
                .expect("dislikes query parse");
        let dislikes_options = build_items_query_options(None, &dislikes_query, None);
        assert_eq!(dislikes_options.is_favorite, Some(false));

        let explicit_query: ItemsQuery = serde_json::from_value(json!({
            "Filters": "Dislikes",
            "IsFavorite": true,
            "Limit": 24
        }))
        .expect("explicit query parse");
        let explicit_options = build_items_query_options(None, &explicit_query, None);
        assert_eq!(explicit_options.is_favorite, Some(true));
    }

    #[test]
    fn build_items_query_options_uses_name_starts_with_and_search_defaults() {
        let query: ItemsQuery = serde_json::from_value(json!({
            "NameStartsWith": "奇缘",
            "Limit": 24
        }))
        .expect("name starts with query parse");

        let options = build_items_query_options(None, &query, None);
        assert_eq!(options.search_term.as_deref(), Some("奇缘"));
        assert_eq!(
            options.include_item_types,
            vec![
                "Folder".to_string(),
                "Movie".to_string(),
                "Series".to_string(),
                "Video".to_string(),
                "Person".to_string(),
            ]
        );
    }

    #[test]
    fn build_items_query_options_keeps_explicit_include_item_types_for_search() {
        let query: ItemsQuery = serde_json::from_value(json!({
            "NameStartsWith": "奇缘",
            "IncludeItemTypes": "Movie,Series",
            "Limit": 24
        }))
        .expect("explicit include query parse");

        let options = build_items_query_options(None, &query, None);
        assert_eq!(options.search_term.as_deref(), Some("奇缘"));
        assert_eq!(
            options.include_item_types,
            vec!["Movie".to_string(), "Series".to_string()]
        );
    }

    #[test]
    fn build_items_query_options_parses_pipe_delimited_tags() {
        let query: ItemsQuery = serde_json::from_value(json!({
            "Tags": "历史|战争,纪录片"
        }))
        .expect("tags query parse");

        let options = build_items_query_options(None, &query, None);
        assert_eq!(
            options.tags,
            vec![
                "历史".to_string(),
                "战争".to_string(),
                "纪录片".to_string()
            ]
        );
    }

    #[test]
    fn parse_uuid_csv_skips_invalid_values() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let raw = format!("{id1},invalid-id,{id2},");
        let parsed = parse_uuid_csv(Some(&raw));

        assert_eq!(parsed, vec![id1, id2]);
    }

    #[test]
    fn normalize_optional_item_id_treats_blank_as_none() {
        assert_eq!(normalize_optional_item_id(None), None);
        assert_eq!(normalize_optional_item_id(Some("")), None);
        assert_eq!(normalize_optional_item_id(Some("   ")), None);
        assert_eq!(normalize_optional_item_id(Some(" 123 ")), Some("123"));
    }

    #[test]
    fn compat_item_id_json_value_is_string() {
        assert_eq!(compat_item_id_json_value(123456789), json!("123456789"));
    }

    #[test]
    fn media_source_item_id_candidate_prefers_source_value() {
        let source = json!({
            "Id": "source-uuid",
            "ItemId": "source-item-id"
        })
        .as_object()
        .cloned()
        .expect("source object");
        let fallback = json!("fallback-item-id");

        let candidate = media_source_item_id_candidate(&source, Some(&fallback));
        assert_eq!(candidate, Some(json!("source-item-id")));
    }

    #[test]
    fn media_source_item_id_candidate_falls_back_to_item_id() {
        let source = json!({
            "Id": "source-uuid",
            "ItemId": null
        })
        .as_object()
        .cloned()
        .expect("source object");
        let fallback = json!("fallback-item-id");

        let candidate = media_source_item_id_candidate(&source, Some(&fallback));
        assert_eq!(candidate, Some(json!("fallback-item-id")));
    }

    #[test]
    fn media_source_item_id_candidate_returns_none_when_missing() {
        let source = json!({
            "Id": "source-uuid"
        })
        .as_object()
        .cloned()
        .expect("source object");

        let candidate = media_source_item_id_candidate(&source, None);
        assert_eq!(candidate, None);
    }

    #[test]
    fn normalize_person_types_keeps_supported_values_only() {
        let values = normalize_person_types(Some(
            "actor,WRITER,screenplay,Guest_Star,unknown,composer,actor",
        ));
        assert_eq!(
            values,
            vec![
                "Actor".to_string(),
                "Composer".to_string(),
                "GuestStar".to_string(),
                "Writer".to_string()
            ]
        );
    }

    #[test]
    fn image_content_type_from_extension() {
        assert_eq!(image_content_type("/tmp/a.jpg"), "image/jpeg");
        assert_eq!(image_content_type("/tmp/a.png"), "image/png");
        assert_eq!(image_content_type("/tmp/a.bin"), "application/octet-stream");
    }

    #[test]
    fn percentile_helper_works_on_sorted_samples() {
        let samples = vec![10_u64, 20, 30, 40, 50];
        assert_eq!(percentile_from_sorted(&samples, 0.0), 10);
        assert_eq!(percentile_from_sorted(&samples, 0.5), 30);
        assert_eq!(percentile_from_sorted(&samples, 0.95), 50);
        assert_eq!(percentile_from_sorted(&samples, 0.99), 50);
    }

    #[test]
    fn range_parsers_handle_valid_and_invalid_values() {
        assert_eq!(parse_range_start(Some("bytes=100-")), Some(100));
        assert_eq!(parse_range_start(Some("bytes=0-1")), Some(0));
        assert_eq!(parse_range_start(Some("invalid")), None);

        assert_eq!(parse_range(Some("bytes=1-3"), 10), Some((1, 3)));
        assert_eq!(parse_range(Some("bytes=8-"), 10), Some((8, 9)));
        assert_eq!(parse_range(Some("bytes=10-"), 10), None);
    }

    #[test]
    fn parse_user_role_honors_explicit_and_fallback_values() {
        assert_eq!(
            parse_user_role(Some("Operator"), Some(true)),
            UserRole::Admin
        );
        assert_eq!(parse_user_role(Some("Admin"), Some(false)), UserRole::Admin);
        assert_eq!(
            parse_user_role(Some("Operator"), Some(false)),
            UserRole::Viewer
        );
        assert_eq!(parse_user_role(None, Some(true)), UserRole::Admin);
        assert_eq!(parse_user_role(Some(""), Some(false)), UserRole::Viewer);
    }

    #[test]
    fn parse_user_role_strict_rejects_invalid_values() {
        assert_eq!(parse_user_role_strict("Admin"), Some(UserRole::Admin));
        assert_eq!(parse_user_role_strict("viewer"), Some(UserRole::Viewer));
        assert_eq!(parse_user_role_strict("operator"), None);
        assert_eq!(parse_user_role_strict("invalid-role"), None);
    }

    #[test]
    fn extract_client_ip_uses_peer_for_direct_client_by_default() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::HeaderName::from_static("x-forwarded-for"),
            HeaderValue::from_static("10.0.0.1, 10.0.0.2"),
        );
        headers.insert(
            header::HeaderName::from_static("x-real-ip"),
            HeaderValue::from_static("192.168.1.10"),
        );

        let security = SecurityConfig::default();

        assert_eq!(
            extract_client_ip(&headers, Some(IpAddr::from([203, 0, 113, 8])), &security).as_deref(),
            Some("203.0.113.8")
        );
    }

    #[test]
    fn extract_client_ip_ignores_forged_header_from_untrusted_proxy() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::HeaderName::from_static("x-forwarded-for"),
            HeaderValue::from_static("10.0.0.1, 10.0.0.2"),
        );

        let mut security = SecurityConfig {
            trust_x_forwarded_for: true,
            ..SecurityConfig::default()
        };
        security.trusted_proxies = vec!["198.51.100.0/24".to_string()];

        assert_eq!(
            extract_client_ip(&headers, Some(IpAddr::from([203, 0, 113, 8])), &security).as_deref(),
            Some("203.0.113.8")
        );
    }

    #[test]
    fn extract_client_ip_uses_forwarded_header_for_trusted_proxy_cidr() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::HeaderName::from_static("x-forwarded-for"),
            HeaderValue::from_static("10.0.0.1, 10.0.0.2"),
        );

        let mut security = SecurityConfig {
            trust_x_forwarded_for: true,
            ..SecurityConfig::default()
        };
        security.trusted_proxies = vec!["203.0.113.0/24".to_string()];

        assert_eq!(
            extract_client_ip(&headers, Some(IpAddr::from([203, 0, 113, 8])), &security).as_deref(),
            Some("10.0.0.1")
        );
    }

    #[test]
    fn allow_entries_support_single_ip_and_cidr() {
        assert_eq!(
            ip_matches_allow_entries(IpAddr::from([203, 0, 113, 8]), &["203.0.113.8".to_string()]),
            true
        );
        assert_eq!(
            ip_matches_allow_entries(
                IpAddr::from([203, 0, 113, 8]),
                &["203.0.113.0/24".to_string()]
            ),
            true
        );
        assert_eq!(
            ip_matches_allow_entries(
                IpAddr::from([203, 0, 113, 8]),
                &["198.51.100.0/24".to_string()]
            ),
            false
        );
    }

    #[test]
    fn api_metrics_snapshot_contains_derived_rates() {
        let metrics = ApiMetrics::default();
        metrics.requests_total.fetch_add(10, Ordering::Relaxed);
        metrics.status_5xx.fetch_add(2, Ordering::Relaxed);
        metrics
            .stream_attempts_total
            .fetch_add(4, Ordering::Relaxed);
        metrics.stream_success_total.fetch_add(3, Ordering::Relaxed);
        metrics
            .stream_cache_hit_total
            .fetch_add(3, Ordering::Relaxed);
        metrics
            .stream_cache_miss_total
            .fetch_add(1, Ordering::Relaxed);

        for sample in [10_u64, 20, 30, 40, 50] {
            metrics.record_latency(sample);
        }

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot["requests_total"].as_u64(), Some(10));
        assert_eq!(snapshot["latency_p95_ms"].as_u64(), Some(50));
        assert_eq!(snapshot["latency_p99_ms"].as_u64(), Some(50));

        let playback_success_rate = snapshot["playback_success_rate"]
            .as_f64()
            .expect("playback_success_rate");
        let cache_hit_rate = snapshot["cache_hit_rate"].as_f64().expect("cache_hit_rate");
        let error_rate = snapshot["error_rate"].as_f64().expect("error_rate");

        assert!((playback_success_rate - 0.75).abs() < 1e-9);
        assert!((cache_hit_rate - 0.75).abs() < 1e-9);
        assert!((error_rate - 0.2).abs() < 1e-9);
    }

    #[test]
    fn admin_create_library_request_supports_optional_library_type() {
        let payload: AdminCreateLibraryRequest = serde_json::from_value(json!({
            "name": "Anime Library",
            "root_path": "/media/anime",
            "paths": ["/media/anime", "/media/anime-extra"],
            "library_type": "Anime"
        }))
        .expect("deserialize create library payload");

        assert_eq!(payload.name, "Anime Library");
        assert_eq!(payload.root_path.as_deref(), Some("/media/anime"));
        assert_eq!(
            payload.paths,
            Some(vec![
                "/media/anime".to_string(),
                "/media/anime-extra".to_string()
            ])
        );
        assert_eq!(payload.library_type.as_deref(), Some("Anime"));
    }

    #[test]
    fn admin_patch_library_request_supports_library_type() {
        let payload: AdminPatchLibraryRequest = serde_json::from_value(json!({
            "library_type": "Series"
        }))
        .expect("deserialize patch library payload");

        assert_eq!(payload.library_type.as_deref(), Some("Series"));
    }

    #[test]
    fn admin_library_status_response_serializes_library_type() {
        let response = AdminLibraryStatusResponse {
            total: 1,
            enabled: 1,
            items: vec![AdminLibraryStatusDto {
                id: Uuid::nil(),
                name: "Main Library".to_string(),
                root_path: "/media/main".to_string(),
                paths: vec!["/media/main".to_string(), "/media/main-2".to_string()],
                library_type: "Mixed".to_string(),
                enabled: true,
                item_count: 42,
                last_item_updated_at: Some("2026-02-21T00:00:00Z".to_string()),
            }],
        };

        let value = serde_json::to_value(response).expect("serialize status response");
        assert_eq!(value["items"][0]["library_type"], "Mixed");
    }

    #[test]
    fn admin_patch_task_definition_request_supports_optional_fields() {
        let payload: AdminPatchTaskDefinitionRequest = serde_json::from_value(json!({
            "enabled": true,
            "cron_expr": "0 */15 * * * *",
            "default_payload": {"library_id": "demo"},
            "max_attempts": 5
        }))
        .expect("deserialize task definition patch payload");

        assert_eq!(payload.enabled, Some(true));
        assert_eq!(payload.cron_expr.as_deref(), Some("0 */15 * * * *"));
        assert_eq!(payload.default_payload, Some(json!({"library_id": "demo"})));
        assert_eq!(payload.max_attempts, Some(5));
    }

    #[test]
    fn admin_run_task_request_supports_payload_override() {
        let payload: AdminRunTaskRequest = serde_json::from_value(json!({
            "payload_override": {
                "library_id": "lib-001",
                "batch_size": 800
            }
        }))
        .expect("deserialize run task payload");

        assert_eq!(
            payload.payload_override,
            Some(json!({
                "library_id": "lib-001",
                "batch_size": 800
            }))
        );
    }

    #[test]
    fn admin_batch_user_status_request_supports_multiple_users() {
        let user_a = Uuid::now_v7();
        let user_b = Uuid::now_v7();
        let payload: AdminBatchUserStatusRequest = serde_json::from_value(json!({
            "user_ids": [user_a, user_b],
            "disabled": true,
        }))
        .expect("deserialize batch user status payload");

        assert_eq!(payload.user_ids, vec![user_a, user_b]);
        assert!(payload.disabled);
    }

    #[test]
    fn admin_user_summary_query_request_parses_filters() {
        let payload: AdminUserSummaryQueryRequest = serde_json::from_value(json!({
            "q": "demo",
            "status": "enabled",
            "role": "Viewer",
            "page": 2,
            "page_size": 50,
            "sort_by": "used_bytes",
            "sort_dir": "desc"
        }))
        .expect("deserialize user summary query");

        assert_eq!(payload.q.as_deref(), Some("demo"));
        assert_eq!(payload.status.as_deref(), Some("enabled"));
        assert_eq!(payload.role.as_deref(), Some("Viewer"));
        assert_eq!(payload.page, Some(2));
        assert_eq!(payload.page_size, Some(50));
        assert_eq!(payload.sort_by.as_deref(), Some("used_bytes"));
        assert_eq!(payload.sort_dir.as_deref(), Some("desc"));
    }

    #[test]
    fn admin_patch_user_profile_request_supports_nullable_fields() {
        let payload: AdminPatchUserProfileRequest = serde_json::from_value(json!({
            "email": null,
            "display_name": "Demo User",
            "remark": "VIP",
            "role": "Viewer",
            "is_disabled": false
        }))
        .expect("deserialize patch user profile");

        assert_eq!(payload.email, Some(None));
        assert_eq!(payload.display_name, Some(Some("Demo User".to_string())));
        assert_eq!(payload.remark, Some(Some("VIP".to_string())));
        assert_eq!(payload.role.as_deref(), Some("Viewer"));
        assert_eq!(payload.is_disabled, Some(false));
    }

    #[test]
    fn apply_system_flags_update_overrides_selected_fields() {
        let mut settings = WebAppConfig::default();
        settings.tmdb.enabled = false;
        settings.storage.lumenbackend_enabled = false;
        settings.storage.prefer_segment_gateway = false;
        settings.observability.metrics_enabled = true;

        let payload = AdminUpdateSystemFlagsRequest {
            scraper_enabled: Some(true),
            tmdb_enabled: Some(true),
            lumenbackend_enabled: Some(true),
            prefer_segment_gateway: Some(true),
            metrics_enabled: Some(false),
        };

        apply_system_flags_update(&mut settings, &payload);
        assert!(settings.scraper.enabled);
        assert!(settings.tmdb.enabled);
        assert!(settings.storage.lumenbackend_enabled);
        assert!(settings.storage.prefer_segment_gateway);
        assert!(!settings.observability.metrics_enabled);
    }

    #[test]
    fn build_system_flags_response_forces_strm_only_and_no_transcoding() {
        let mut settings = WebAppConfig::default();
        settings.tmdb.enabled = true;
        settings.storage.lumenbackend_enabled = true;
        settings.storage.prefer_segment_gateway = true;
        settings.observability.metrics_enabled = false;

        let response = build_system_flags_response(&settings);
        assert!(response.strm_only_streaming);
        assert!(!response.transcoding_enabled);
        assert!(response.scraper_enabled);
        assert!(response.tmdb_enabled);
        assert!(response.lumenbackend_enabled);
        assert!(response.prefer_segment_gateway);
        assert!(!response.metrics_enabled);
    }

    #[test]
    fn mask_web_settings_hides_secret_values() {
        let raw = serde_json::json!({
            "auth": { "bootstrap_admin_password": "secret" },
            "tmdb": { "api_key": "tmdb-secret" },
            "scraper": {
                "tvdb": { "api_key": "tvdb-secret", "pin": "tvdb-pin" },
                "bangumi": { "access_token": "bangumi-token" }
            },
            "billing": { "epay": { "key": "epay-secret" } },
            "storage": { "lumenbackend_stream_signing_key": "stream-secret" }
        });
        let settings = serde_json::from_value(raw).expect("deserialize settings");

        let masked = mask_web_settings(settings);
        assert_eq!(masked.auth.bootstrap_admin_password, "***");
        assert_eq!(masked.tmdb.api_key, "***");
        assert_eq!(masked.scraper.tvdb.api_key, "***");
        assert_eq!(masked.scraper.tvdb.pin, "***");
        assert_eq!(masked.scraper.bangumi.access_token, "***");
        assert_eq!(masked.billing.epay.key, "***");
        assert_eq!(masked.storage.lumenbackend_stream_signing_key, "***");
    }

    #[test]
    fn merge_secret_placeholders_keeps_existing_secret_values() {
        let current = serde_json::from_value(serde_json::json!({
            "auth": { "bootstrap_admin_password": "current-admin" },
            "tmdb": { "api_key": "current-tmdb" },
            "scraper": {
                "tvdb": { "api_key": "current-tvdb", "pin": "current-pin" },
                "bangumi": { "access_token": "current-bangumi" }
            },
            "billing": { "epay": { "key": "current-epay" } },
            "storage": { "lumenbackend_stream_signing_key": "current-stream-secret" }
        }))
        .expect("deserialize current settings");

        let incoming = serde_json::from_value(serde_json::json!({
            "auth": { "bootstrap_admin_password": "***" },
            "tmdb": { "api_key": "***" },
            "scraper": {
                "tvdb": { "api_key": "***", "pin": "***" },
                "bangumi": { "access_token": "***" }
            },
            "billing": { "epay": { "key": "***" } },
            "storage": { "lumenbackend_stream_signing_key": "***" }
        }))
        .expect("deserialize incoming settings");

        let merged = merge_secret_placeholders(incoming, &current);
        assert_eq!(merged.auth.bootstrap_admin_password, "current-admin");
        assert_eq!(merged.tmdb.api_key, "current-tmdb");
        assert_eq!(merged.scraper.tvdb.api_key, "current-tvdb");
        assert_eq!(merged.scraper.tvdb.pin, "current-pin");
        assert_eq!(merged.scraper.bangumi.access_token, "current-bangumi");
        assert_eq!(merged.billing.epay.key, "current-epay");
        assert_eq!(
            merged.storage.lumenbackend_stream_signing_key,
            "current-stream-secret"
        );
    }

    #[test]
    fn merge_agent_secret_placeholders_keeps_existing_moviepilot_password() {
        let current = AgentConfig {
            moviepilot: serde_json::from_value(serde_json::json!({
                "password": "saved-password"
            }))
            .expect("deserialize moviepilot config"),
            ..AgentConfig::default()
        };
        let incoming = AgentConfig {
            moviepilot: serde_json::from_value(serde_json::json!({
                "password": "***"
            }))
            .expect("deserialize incoming moviepilot config"),
            ..AgentConfig::default()
        };

        let merged = merge_agent_secret_placeholders(incoming, &current);
        assert_eq!(merged.moviepilot.password, "saved-password");
    }

    #[test]
    fn merge_agent_secret_placeholders_preserves_new_moviepilot_password() {
        let current = AgentConfig {
            moviepilot: serde_json::from_value(serde_json::json!({
                "password": "saved-password"
            }))
            .expect("deserialize moviepilot config"),
            ..AgentConfig::default()
        };
        let incoming = AgentConfig {
            moviepilot: serde_json::from_value(serde_json::json!({
                "password": "new-password"
            }))
            .expect("deserialize incoming moviepilot config"),
            ..AgentConfig::default()
        };

        let merged = merge_agent_secret_placeholders(incoming, &current);
        assert_eq!(merged.moviepilot.password, "new-password");
    }

    #[test]
    fn map_items_query_error_returns_503_when_search_backend_is_unavailable() {
        let err = anyhow::Error::new(InfraError::SearchUnavailable);
        assert_eq!(map_items_query_error(&err), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn map_items_query_error_returns_500_for_other_errors() {
        let err = anyhow::anyhow!("db failed");
        assert_eq!(
            map_items_query_error(&err),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn map_stream_admission_error_returns_403_with_reason_payload() {
        let err = anyhow::Error::new(InfraError::StreamAccessDenied {
            reason: StreamAccessDeniedReason::TrafficQuotaExceeded,
        });
        let response = map_stream_admission_error(&err).expect("stream denied response");

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn map_stream_admission_error_ignores_non_admission_errors() {
        let err = anyhow::Error::new(InfraError::SearchUnavailable);
        assert!(map_stream_admission_error(&err).is_none());
    }

    #[test]
    fn map_billing_error_returns_payment_required_for_insufficient_balance() {
        let err = anyhow::Error::new(InfraError::BillingInsufficientBalance);
        let response = map_billing_error(&err).expect("billing response");
        assert_eq!(response.status(), StatusCode::PAYMENT_REQUIRED);
    }

    #[test]
    fn map_billing_error_ignores_non_billing_errors() {
        let err = anyhow::Error::new(InfraError::SearchUnavailable);
        assert!(map_billing_error(&err).is_none());
    }

    #[test]
    fn map_invite_error_maps_known_invite_errors() {
        let required = anyhow::Error::new(InfraError::InviteCodeRequired);
        let invalid = anyhow::Error::new(InfraError::InviteCodeInvalid);
        let exists = anyhow::Error::new(InfraError::UserAlreadyExists);

        assert_eq!(
            map_invite_error(&required)
                .expect("required response")
                .status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            map_invite_error(&invalid)
                .expect("invalid response")
                .status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            map_invite_error(&exists)
                .expect("exists response")
                .status(),
            StatusCode::CONFLICT
        );
    }

    #[test]
    fn map_invite_error_ignores_non_invite_errors() {
        let err = anyhow::Error::new(InfraError::BillingDisabled);
        assert!(map_invite_error(&err).is_none());
    }

    #[test]
    fn map_playlist_error_maps_known_playlist_errors() {
        let not_found = anyhow::Error::new(InfraError::PlaylistNotFound);
        let access_denied = anyhow::Error::new(InfraError::PlaylistAccessDenied);
        let conflict = anyhow::Error::new(InfraError::PlaylistConflict);

        assert_eq!(
            map_playlist_error(&not_found)
                .expect("not found response")
                .status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            map_playlist_error(&access_denied)
                .expect("forbidden response")
                .status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            map_playlist_error(&conflict)
                .expect("conflict response")
                .status(),
            StatusCode::CONFLICT
        );
    }

    #[test]
    fn map_task_center_error_maps_active_run_conflict() {
        let err = anyhow::Error::new(InfraError::TaskRunAlreadyActive);
        let response = map_task_center_error(&err).expect("task center response");
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[test]
    fn top_played_query_helpers_clamp_values() {
        assert_eq!(normalize_top_played_limit(None), 10);
        assert_eq!(normalize_top_played_limit(Some(0)), 1);
        assert_eq!(normalize_top_played_limit(Some(500)), 100);

        assert_eq!(normalize_top_played_window_days(None), 1);
        assert_eq!(normalize_top_played_window_days(Some(0)), 1);
        assert_eq!(normalize_top_played_window_days(Some(365)), 90);
    }

    #[test]
    fn item_counts_dto_includes_extended_jellyfin_fields() {
        let dto = ItemCountsDto {
            movie_count: 1,
            series_count: 2,
            episode_count: 3,
            song_count: 4,
            album_count: 5,
            artist_count: 6,
            music_video_count: 7,
            box_set_count: 8,
            book_count: 9,
            game_count: 10,
            game_system_count: 11,
            item_count: 66,
            program_count: 12,
            trailer_count: 13,
        };
        let value = serde_json::to_value(dto).expect("item counts serialize");
        assert_eq!(value["MovieCount"].as_i64(), Some(1));
        assert_eq!(value["MusicVideoCount"].as_i64(), Some(7));
        assert_eq!(value["BoxSetCount"].as_i64(), Some(8));
        assert_eq!(value["BookCount"].as_i64(), Some(9));
        assert_eq!(value["GameCount"].as_i64(), Some(10));
        assert_eq!(value["GameSystemCount"].as_i64(), Some(11));
        assert_eq!(value["ItemCount"].as_i64(), Some(66));
    }

    #[test]
    fn next_up_query_helpers_clamp_values() {
        assert_eq!(normalize_next_up_limit(None), 20);
        assert_eq!(normalize_next_up_limit(Some(0)), 1);
        assert_eq!(normalize_next_up_limit(Some(500)), 100);

        assert_eq!(normalize_next_up_start_index(None), 0);
        assert_eq!(normalize_next_up_start_index(Some(-3)), 0);
        assert_eq!(normalize_next_up_start_index(Some(42)), 42);
    }

    #[test]
    fn search_hints_query_helpers_normalize_inputs() {
        assert_eq!(normalize_search_hints_limit(None), 20);
        assert_eq!(normalize_search_hints_limit(Some(0)), 1);
        assert_eq!(normalize_search_hints_limit(Some(500)), 100);

        assert_eq!(normalize_search_hints_start_index(None), 0);
        assert_eq!(normalize_search_hints_start_index(Some(-7)), 0);
        assert_eq!(normalize_search_hints_start_index(Some(9)), 9);

        let query = SearchHintsQuery {
            start_index: None,
            limit: None,
            _user_id: None,
            search_term: None,
            include_item_types: Some("movie, Person".to_string()),
            exclude_item_types: None,
            media_types: None,
            parent_id: None,
            is_movie: Some(true),
            is_series: Some(true),
            is_news: Some(true),
            is_kids: Some(true),
            is_sports: Some(true),
            include_people: None,
            include_media: None,
            include_genres: None,
            include_studios: None,
            include_artists: None,
        };

        let include_item_types = normalize_search_hints_include_item_types(&query);
        assert!(
            include_item_types
                .iter()
                .any(|v| v.eq_ignore_ascii_case("Movie"))
        );
        assert!(
            include_item_types
                .iter()
                .any(|v| v.eq_ignore_ascii_case("Person"))
        );
        assert!(
            include_item_types
                .iter()
                .any(|v| v.eq_ignore_ascii_case("Series"))
        );
        assert!(
            include_item_types
                .iter()
                .any(|v| v.eq_ignore_ascii_case("Program"))
        );
        assert!(
            include_item_types
                .iter()
                .any(|v| v.eq_ignore_ascii_case("Sports"))
        );
        assert_eq!(
            include_item_types
                .iter()
                .filter(|v| v.eq_ignore_ascii_case("Movie"))
                .count(),
            1
        );
    }

    #[test]
    fn resume_query_helpers_clamp_values() {
        assert_eq!(normalize_resume_limit(None), 50);
        assert_eq!(normalize_resume_limit(Some(0)), 1);
        assert_eq!(normalize_resume_limit(Some(600)), 500);

        assert_eq!(normalize_resume_start_index(None), 0);
        assert_eq!(normalize_resume_start_index(Some(-9)), 0);
        assert_eq!(normalize_resume_start_index(Some(6)), 6);
    }

    #[test]
    fn resume_filter_helpers_match_types_media_and_search_term() {
        assert!(resume_item_type_matches("Episode", &["Episode".to_string()], &[]));
        assert!(!resume_item_type_matches(
            "Movie",
            &["Episode".to_string()],
            &[]
        ));
        assert!(!resume_item_type_matches(
            "Episode",
            &[],
            &["Episode".to_string()]
        ));

        assert!(resume_media_type_matches("Episode", &["Video".to_string()]));
        assert!(resume_media_type_matches("AudioBook", &["Audio".to_string()]));
        assert!(!resume_media_type_matches(
            "Photo",
            &["Video".to_string()]
        ));

        assert!(resume_name_matches_search("My Episode Name", Some("episode")));
        assert!(resume_name_matches_search("My Episode Name", Some("  ")));
        assert!(!resume_name_matches_search(
            "My Episode Name",
            Some("missing")
        ));
    }

    #[test]
    fn items_filters_helper_applies_media_type_intersection() {
        let filtered = filter_item_types_by_media_types(
            vec![
                "Movie".to_string(),
                "Song".to_string(),
                "Photo".to_string(),
            ],
            &["Video".to_string()],
        );
        assert_eq!(filtered, vec!["Movie".to_string()]);

        let expanded = filter_item_types_by_media_types(vec![], &["Audio".to_string()]);
        assert!(expanded.iter().any(|v| v == "Song"));
        assert!(expanded.iter().any(|v| v == "AudioBook"));
        assert!(!expanded.iter().any(|v| v == "Movie"));
    }

    #[test]
    fn items_query_and_filter_queries_accept_extended_aliases() {
        let items_query: ItemsQuery = serde_json::from_value(json!({
            "userId": Uuid::new_v4(),
            "parentId": Uuid::new_v4(),
            "includeItemTypes": "Movie,Episode",
            "exclude_item_types": "Person",
            "mediaTypes": "Video",
            "ids": "id-1,id-2",
            "search_term": "avatar",
            "nameStartsWith": "av",
            "filters": "IsFavorite",
            "limit": 120,
            "start_index": 5,
            "sortBy": "SortName",
            "sort_order": "Descending",
            "recursive": true,
            "genres": "Action",
            "years": "2024",
            "isFavorite": true,
            "enableTotalRecordCount": false,
            "enableImages": true
        }))
        .expect("items query parse");

        assert!(items_query.user_id.is_some());
        assert!(items_query.parent_id.is_some());
        assert_eq!(items_query.exclude_item_types.as_deref(), Some("Person"));
        assert_eq!(items_query.media_types.as_deref(), Some("Video"));
        assert_eq!(items_query.name_starts_with.as_deref(), Some("av"));
        assert_eq!(items_query.enable_total_record_count, Some(false));
        assert_eq!(items_query._enable_images, Some(true));

        let filters_query: ItemsFiltersQuery = serde_json::from_value(json!({
            "user_id": Uuid::new_v4(),
            "parentId": Uuid::new_v4(),
            "include_item_types": "Movie,Episode",
            "media_types": "Video",
            "enableImages": true,
            "fields": "PrimaryImageAspectRatio"
        }))
        .expect("items filters query parse");
        assert!(filters_query._user_id.is_some());
        assert!(filters_query.parent_id.is_some());
        assert_eq!(filters_query._enable_images, Some(true));
        assert_eq!(
            filters_query._fields.as_deref(),
            Some("PrimaryImageAspectRatio")
        );

        let genres_query: GenresQuery = serde_json::from_value(json!({
            "parent_id": Uuid::new_v4(),
            "includeItemTypes": "Movie",
            "startIndex": 3,
            "limit": 40,
            "recursive": true,
            "sortBy": "SortName",
            "imageTypeLimit": 2
        }))
        .expect("genres query parse");
        assert!(genres_query.parent_id.is_some());
        assert_eq!(genres_query.start_index, Some(3));
        assert_eq!(genres_query.limit, Some(40));
        assert_eq!(genres_query.recursive, Some(true));
        assert_eq!(genres_query._sort_by.as_deref(), Some("SortName"));
        assert_eq!(genres_query._image_type_limit, Some(2));

        let named_value_query: NamedValueListQuery = serde_json::from_value(json!({
            "parentId": Uuid::new_v4(),
            "include_item_types": "Movie,Series",
            "mediaTypes": "Video",
            "start_index": 4,
            "limit": 30,
            "Recursive": false
        }))
        .expect("named value query parse");
        assert!(named_value_query.parent_id.is_some());
        assert_eq!(
            named_value_query.include_item_types.as_deref(),
            Some("Movie,Series")
        );
        assert_eq!(named_value_query.media_types.as_deref(), Some("Video"));
        assert_eq!(named_value_query.start_index, Some(4));
        assert_eq!(named_value_query.limit, Some(30));
        assert_eq!(named_value_query.recursive, Some(false));
    }

    #[test]
    fn paginate_named_values_returns_emby_query_result_shape_without_start_index() {
        let result = paginate_named_values(
            vec![
                "2026".to_string(),
                "2025".to_string(),
                "2024".to_string(),
            ],
            1,
            1,
        );
        let value = serde_json::to_value(result).expect("serialize named value query result");

        assert_eq!(value.get("TotalRecordCount"), Some(&json!(3)));
        assert_eq!(value.get("Items"), Some(&json!([{ "Name": "2025" }])));
        assert!(value.get("StartIndex").is_none());
    }

    #[test]
    fn studios_browse_result_serializes_with_emby_expected_core_fields() {
        let result = StudioBrowseQueryResultDto {
            items: vec![StudioBrowseItemDto {
                name: "Pixar".to_string(),
                server_id: "server-id".to_string(),
                id: "12345".to_string(),
                item_type: "Studio".to_string(),
                user_data: StudioBrowseUserDataDto {
                    playback_position_ticks: 0,
                    play_count: 0,
                    is_favorite: false,
                    played: false,
                },
                image_tags: std::collections::HashMap::new(),
                backdrop_image_tags: vec![],
            }],
            total_record_count: 1,
        };
        let value = serde_json::to_value(result).expect("serialize studios browse result");

        assert_eq!(value.get("TotalRecordCount"), Some(&json!(1)));
        assert_eq!(value.get("Items"), Some(&json!([{
            "Name": "Pixar",
            "ServerId": "server-id",
            "Id": "12345",
            "Type": "Studio",
            "UserData": {
                "PlaybackPositionTicks": 0,
                "PlayCount": 0,
                "IsFavorite": false,
                "Played": false
            },
            "ImageTags": {},
            "BackdropImageTags": []
        }])));
    }

    #[test]
    fn apply_items_query_compatibility_filters_types_ids_and_total_count() {
        let result = QueryResultDto {
            items: vec![
                make_test_item("id-1", "Movie One", "Movie"),
                make_test_item("id-2", "Actor A", "Person"),
                make_test_item("id-3", "Movie Two", "Movie"),
            ],
            total_record_count: 3,
            start_index: 0,
        };

        let query: ItemsQuery = serde_json::from_value(json!({
            "excludeItemTypes": "Person",
            "ids": "id-1,id-2",
            "enableTotalRecordCount": false
        }))
        .expect("items query parse");

        let filtered = apply_items_query_compatibility(result, &query);
        assert_eq!(filtered.items.len(), 1);
        assert_eq!(filtered.items[0].id, "id-1");
        assert_eq!(filtered.total_record_count, 0);
    }

    #[test]
    fn apply_items_query_compatibility_preserves_total_for_exclude_only() {
        let result = QueryResultDto {
            items: vec![
                make_test_item("id-1", "Movie One", "Movie"),
                make_test_item("id-2", "Movie Two", "Movie"),
            ],
            total_record_count: 240,
            start_index: 24,
        };

        let query: ItemsQuery = serde_json::from_value(json!({
            "excludeItemTypes": "Person"
        }))
        .expect("items query parse");

        let filtered = apply_items_query_compatibility(result, &query);
        assert_eq!(filtered.items.len(), 2);
        assert_eq!(filtered.total_record_count, 240);
    }

    #[test]
    fn compat_items_query_result_json_compacts_unrequested_fields_for_fields_query() {
        let mut item = make_test_item("id-1", "Movie One", "Movie");
        item.path = "/mnt/media/movie-one.strm".to_string();
        item.container = Some("strm".to_string());
        item.media_sources = Some(vec![ls_domain::jellyfin::MediaSourceInfoDto {
            name: None,
            id:"id-1".to_string(),
            path: Some(item.path.clone()),
            protocol: "File".to_string(),
            container: Some("strm".to_string()),
            runtime_ticks: Some(720_000_000),
            bitrate: Some(1_800_000),
            supports_direct_play: true,
            supports_direct_stream: true,
            supports_transcoding: false,
            chapters: vec![],
            media_streams: vec![],
        }]);
        item.date_created = Some("2026-02-22T11:14:10.856530+00:00".to_string());

        let query: ItemsQuery = serde_json::from_value(json!({
            "Fields": "BasicSyncInfo,CanDelete,Container,PrimaryImageAspectRatio,ProductionYear,Status,EndDate"
        }))
        .expect("items query parse");

        let payload = compat_items_query_result_json(
            QueryResultDto {
                items: vec![item],
                total_record_count: 1,
                start_index: 0,
            },
            query._fields.as_deref(),
            "test-server-id",
            true,
        );

        let response_item = payload["Items"][0]
            .as_object()
            .expect("item payload as object");
        assert_eq!(response_item.get("ServerId"), Some(&json!("test-server-id")));
        assert_eq!(response_item.get("SupportsSync"), Some(&json!(true)));
        assert!(response_item.get("UserData").is_some());
        assert_eq!(response_item.get("Container"), Some(&json!("strm")));
        assert!(response_item.get("MediaSources").is_none());
        assert!(response_item.get("Path").is_none());
        assert!(response_item.get("DateCreated").is_none());
    }

    #[test]
    fn compat_items_query_result_json_keeps_requested_media_sources() {
        let mut item = make_test_item("id-1", "Movie One", "Movie");
        item.container = Some("strm".to_string());
        item.media_sources = Some(vec![ls_domain::jellyfin::MediaSourceInfoDto {
            name: None,
            id:"id-1".to_string(),
            path: Some("/mnt/media/movie-one.strm".to_string()),
            protocol: "File".to_string(),
            container: Some("strm".to_string()),
            runtime_ticks: Some(720_000_000),
            bitrate: Some(1_800_000),
            supports_direct_play: true,
            supports_direct_stream: true,
            supports_transcoding: false,
            chapters: vec![],
            media_streams: vec![],
        }]);

        let query: ItemsQuery = serde_json::from_value(json!({
            "Fields": "BasicSyncInfo,CanDelete,Container,MediaSources"
        }))
        .expect("items query parse");

        let payload = compat_items_query_result_json(
            QueryResultDto {
                items: vec![item],
                total_record_count: 1,
                start_index: 0,
            },
            query._fields.as_deref(),
            "test-server-id",
            true,
        );

        let response_item = payload["Items"][0]
            .as_object()
            .expect("item payload as object");
        assert!(response_item.get("MediaSources").is_some());
        assert_eq!(response_item.get("Container"), Some(&json!("strm")));
    }

    #[test]
    fn compat_items_query_result_json_keeps_media_sources_when_media_streams_requested() {
        let mut item = make_test_item("id-1", "Movie One", "Movie");
        item.container = Some("strm".to_string());
        item.media_sources = Some(vec![ls_domain::jellyfin::MediaSourceInfoDto {
            name: None,
            id:"id-1".to_string(),
            path: Some("/mnt/media/movie-one.strm".to_string()),
            protocol: "File".to_string(),
            container: Some("strm".to_string()),
            runtime_ticks: Some(720_000_000),
            bitrate: Some(1_800_000),
            supports_direct_play: true,
            supports_direct_stream: true,
            supports_transcoding: false,
            chapters: vec![],
            media_streams: vec![],
        }]);

        let query: ItemsQuery = serde_json::from_value(json!({
            "Fields": "MediaStreams"
        }))
        .expect("items query parse");

        let payload = compat_items_query_result_json(
            QueryResultDto {
                items: vec![item],
                total_record_count: 1,
                start_index: 0,
            },
            query._fields.as_deref(),
            "test-server-id",
            true,
        );

        let response_item = payload["Items"][0]
            .as_object()
            .expect("item payload as object");
        assert!(
            response_item.get("MediaSources").is_some(),
            "MediaStreams request should keep MediaSources (Emby behavior)"
        );
        assert!(response_item.get("MediaStreams").is_some());
    }

    #[test]
    fn compat_items_query_result_json_preserves_episode_linkage_fields_with_fields_filter() {
        let mut item = make_test_item("ep-1", "Episode One", "Episode");
        item.parent_id = Some("season-1".to_string());
        item.series_id = Some("series-1".to_string());
        item.season_id = Some("season-1".to_string());
        item.index_number = Some(1);
        item.parent_index_number = Some(1);

        let query: ItemsQuery = serde_json::from_value(json!({
            "Fields": "DateCreated,Etag,Genres,MediaSources,AlternateMediaSources,Overview,ParentId,Path,People,ProviderIds,SortName,RecursiveItemCount,ChildCount,CommunityRating,OfficialRating,PremiereDate"
        }))
        .expect("items query parse");

        let payload = compat_items_query_result_json(
            QueryResultDto {
                items: vec![item],
                total_record_count: 1,
                start_index: 0,
            },
            query._fields.as_deref(),
            "test-server-id",
            true,
        );

        let response_item = payload["Items"][0]
            .as_object()
            .expect("item payload as object");
        assert_eq!(response_item.get("ParentId"), Some(&json!("season-1")));
        assert_eq!(response_item.get("SeriesId"), Some(&json!("series-1")));
        assert_eq!(response_item.get("SeasonId"), Some(&json!("season-1")));
        assert_eq!(response_item.get("IndexNumber"), Some(&json!(1)));
        assert_eq!(response_item.get("ParentIndexNumber"), Some(&json!(1)));
    }

    #[test]
    fn compat_items_query_result_json_infers_top_level_media_streams_from_media_sources() {
        let mut item = make_test_item("id-1", "Episode One", "Episode");
        item.media_sources = Some(vec![ls_domain::jellyfin::MediaSourceInfoDto {
            name: None,
            id:"id-1".to_string(),
            path: Some("/mnt/media/episode-one.strm".to_string()),
            protocol: "File".to_string(),
            container: Some("strm".to_string()),
            runtime_ticks: Some(720_000_000),
            bitrate: Some(1_800_000),
            supports_direct_play: true,
            supports_direct_stream: true,
            supports_transcoding: false,
            chapters: vec![],
            media_streams: vec![ls_domain::jellyfin::MediaStreamDto {
                index: 0,
                stream_type: "Video".to_string(),
                language: Some("und".to_string()),
                is_external: false,
                path: None,
                codec: Some("h264".to_string()),
                display_title: None,
                width: Some(1920),
                height: Some(1080),
                average_frame_rate: None,
                real_frame_rate: None,
                profile: Some("High".to_string()),
                level: Some(41),
                channels: None,
                sample_rate: None,
                channel_layout: None,
                bit_rate: Some(1_800_000),
                color_range: None,
                color_space: None,
                color_transfer: None,
                color_primaries: None,
                bit_depth: None,
                video_range: None,
                video_range_type: None,
                hdr10_plus_present_flag: None,
                dv_version_major: None,
                dv_version_minor: None,
                dv_profile: None,
                dv_level: None,
                rpu_present_flag: None,
                el_present_flag: None,
                bl_present_flag: None,
                dv_bl_signal_compatibility_id: None,
                is_default: Some(true),
                is_forced: Some(false),
            }],
        }]);

        let query: ItemsQuery = serde_json::from_value(json!({
            "Fields": "CanDelete,MediaSources,MediaStreams"
        }))
        .expect("items query parse");

        let payload = compat_items_query_result_json(
            QueryResultDto {
                items: vec![item],
                total_record_count: 1,
                start_index: 0,
            },
            query._fields.as_deref(),
            "test-server-id",
            true,
        );

        let response_item = payload["Items"][0]
            .as_object()
            .expect("item payload as object");
        let media_streams = response_item
            .get("MediaStreams")
            .and_then(Value::as_array)
            .expect("top-level media streams");
        assert_eq!(media_streams.len(), 1);
        assert_eq!(media_streams[0].get("Level"), Some(&json!(41)));
        assert!(media_streams[0].get("Path").is_none());
    }

    #[test]
    fn compat_items_query_result_json_adds_series_status_and_normalizes_end_date() {
        let mut item = make_test_item("series-1", "Series One", "Series");
        item.end_date = Some("2026-02-22T11:14:10.856530+00:00".to_string());

        let query: ItemsQuery = serde_json::from_value(json!({
            "Fields": "Status,EndDate"
        }))
        .expect("items query parse");

        let payload = compat_items_query_result_json(
            QueryResultDto {
                items: vec![item],
                total_record_count: 1,
                start_index: 0,
            },
            query._fields.as_deref(),
            "test-server-id",
            true,
        );

        let response_item = payload["Items"][0]
            .as_object()
            .expect("item payload as object");
        assert_eq!(response_item.get("Status"), Some(&json!("Ended")));
        assert_eq!(response_item.get("AirDays"), Some(&json!([])));
        let end_date = response_item
            .get("EndDate")
            .and_then(Value::as_str)
            .expect("end date");
        assert!(end_date.ends_with('Z'));
    }

    #[test]
    fn compat_items_query_result_json_adds_view_item_defaults() {
        let mut item = make_test_item("view-1", "Movies", "CollectionFolder");
        item.is_folder = Some(true);
        item.sort_name = Some("Movies".to_string());
        item.date_created = Some("2026-02-22T11:14:10.856530+00:00".to_string());

        let payload = compat_items_query_result_json(
            QueryResultDto {
                items: vec![item],
                total_record_count: 1,
                start_index: 0,
            },
            None,
            "test-server-id",
            false,
        );

        let response_item = payload["Items"][0]
            .as_object()
            .expect("view item payload as object");
        assert_eq!(
            response_item.get("DateModified"),
            Some(&json!("2026-02-22T11:14:10.8565300Z"))
        );
        assert_eq!(response_item.get("DisplayPreferencesId"), Some(&json!("view-1")));
        assert_eq!(response_item.get("PresentationUniqueKey"), Some(&json!("view-1")));
        assert_eq!(response_item.get("Guid"), Some(&json!("view-1")));
        assert!(response_item.get("Etag").and_then(Value::as_str).is_some());
        assert_eq!(response_item.get("ExternalUrls"), Some(&json!([])));
        assert_eq!(response_item.get("RemoteTrailers"), Some(&json!([])));
        assert_eq!(response_item.get("Taglines"), Some(&json!([])));
        assert_eq!(response_item.get("LockData"), Some(&json!(false)));
        assert_eq!(response_item.get("LockedFields"), Some(&json!([])));
        assert_eq!(response_item.get("PrimaryImageAspectRatio"), Some(&json!(1.0)));
        let image_tags = response_item
            .get("ImageTags")
            .and_then(Value::as_object)
            .expect("ImageTags");
        assert!(image_tags.get("Primary").and_then(Value::as_str).is_some());
    }

    #[test]
    fn compat_latest_items_json_applies_extended_defaults() {
        let mut item = make_test_item("series-2", "Series Two", "Series");
        item.is_folder = Some(true);
        item.date_created = Some("2026-02-22T11:14:10.856530+00:00".to_string());

        let payload = compat_latest_items_json(vec![item], None, "test-server-id");
        let entries = payload.as_array().expect("latest items array");
        assert_eq!(entries.len(), 1);
        let response_item = entries[0].as_object().expect("latest item payload");

        assert_eq!(response_item.get("ServerId"), Some(&json!("test-server-id")));
        assert!(response_item.get("UserData").is_some());
        assert_eq!(response_item.get("Status"), Some(&json!("Continuing")));
        assert_eq!(response_item.get("AirDays"), Some(&json!([])));
        assert_eq!(response_item.get("RunTimeTicks"), Some(&json!(0)));
        assert_eq!(response_item.get("DisplayPreferencesId"), Some(&json!("series-2")));
        assert_eq!(response_item.get("Guid"), Some(&json!("series-2")));
        assert_eq!(response_item.get("ExternalUrls"), Some(&json!([])));
        assert_eq!(response_item.get("Taglines"), Some(&json!([])));
    }

    #[test]
    fn compat_metadata_lookup_query_result_json_adds_lookup_defaults() {
        let mut item = make_test_item("genre-1", "Action", "Genre");
        item.child_count = Some(12);

        let payload = compat_metadata_lookup_query_result_json(
            QueryResultDto {
                items: vec![item],
                total_record_count: 1,
                start_index: 5,
            },
            "test-server-id",
        );

        let response_item = payload["Items"][0]
            .as_object()
            .expect("metadata item payload as object");
        assert_eq!(response_item.get("ServerId"), Some(&json!("test-server-id")));
        assert_eq!(response_item.get("BackdropImageTags"), Some(&json!([])));
        assert!(!response_item.contains_key("Path"));
        assert!(!response_item.contains_key("ChildCount"));

        let user_data = response_item
            .get("UserData")
            .and_then(Value::as_object)
            .expect("user data");
        assert_eq!(user_data.get("Played"), Some(&json!(false)));
        assert_eq!(user_data.get("PlaybackPositionTicks"), Some(&json!(0)));
        assert_eq!(user_data.get("PlayCount"), Some(&json!(0)));
        assert_eq!(user_data.get("IsFavorite"), Some(&json!(false)));

        let primary_tag = response_item
            .get("ImageTags")
            .and_then(Value::as_object)
            .and_then(|tags| tags.get("Primary"))
            .and_then(Value::as_str)
            .expect("primary image tag");
        assert!(!primary_tag.is_empty());
        assert!(payload.get("StartIndex").is_none());
    }

    #[test]
    fn compat_single_item_json_computes_played_percentage_when_runtime_present() {
        let mut item = make_test_item("id-1", "Movie One", "Movie");
        item.runtime_ticks = Some(200);
        item.user_data = Some(ls_domain::jellyfin::UserDataDto {
            played: false,
            playback_position_ticks: 50,
            is_favorite: Some(false),
        });

        let payload = compat_single_item_json(item, "test-server-id");
        let object = payload.as_object().expect("single item payload as object");
        let user_data = object
            .get("UserData")
            .and_then(Value::as_object)
            .expect("user data");
        assert_eq!(user_data.get("PlayedPercentage"), Some(&json!(25.0)));
    }

    #[test]
    fn compat_metadata_lookup_query_result_json_keeps_existing_primary_image_tag() {
        let mut item = make_test_item("person-1", "Alice", "Person");
        item.image_tags = Some(std::collections::HashMap::from([(
            "Primary".to_string(),
            "existing-tag".to_string(),
        )]));
        item.provider_ids = Some(std::collections::HashMap::from([(
            "Tmdb".to_string(),
            "123".to_string(),
        )]));
        item.primary_image_tag = Some("existing-tag".to_string());
        item.date_created = Some("2026-02-22T11:14:10.856530+00:00".to_string());

        let payload = compat_metadata_lookup_query_result_json(
            QueryResultDto {
                items: vec![item],
                total_record_count: 1,
                start_index: 0,
            },
            "test-server-id",
        );

        let response_item = payload["Items"][0]
            .as_object()
            .expect("metadata item payload as object");
        assert_eq!(
            response_item
                .get("ImageTags")
                .and_then(Value::as_object)
                .and_then(|tags| tags.get("Primary")),
            Some(&json!("existing-tag"))
        );
        assert!(!response_item.contains_key("ProviderIds"));
        assert!(!response_item.contains_key("PrimaryImageTag"));
        assert!(!response_item.contains_key("DateCreated"));
    }

    #[test]
    fn compat_single_item_json_adds_vidhub_safety_fields_for_series_detail() {
        let mut item = make_test_item("series-1", "Series One", "Series");
        item.user_data = None;
        item.date_created = Some("2026-02-22T11:14:10.856530+00:00".to_string());
        item.premiere_date = Some("2024-09-17".to_string());
        item.genres = Some(vec!["Drama".to_string(), "Mystery".to_string()]);
        item.studios = Some(vec![ls_domain::jellyfin::NameGuidPairDto {
            name: "Studio One".to_string(),
            id: None,
        }]);

        let payload = compat_single_item_json(item, "test-server-id");
        let object = payload.as_object().expect("single item payload as object");

        assert_eq!(object.get("ServerId"), Some(&json!("test-server-id")));
        assert_eq!(object.get("SupportsSync"), Some(&json!(true)));
        assert_eq!(object.get("Status"), Some(&json!("Continuing")));
        assert_eq!(object.get("AirDays"), Some(&json!([])));

        let user_data = object
            .get("UserData")
            .and_then(Value::as_object)
            .expect("user data");
        assert_eq!(user_data.get("Played"), Some(&json!(false)));
        assert_eq!(user_data.get("PlaybackPositionTicks"), Some(&json!(0)));
        assert_eq!(user_data.get("PlayCount"), Some(&json!(0)));
        assert_eq!(user_data.get("IsFavorite"), Some(&json!(false)));
        assert_eq!(user_data.get("UnplayedItemCount"), Some(&json!(0)));

        assert_eq!(object.get("RunTimeTicks"), Some(&json!(0)));
        assert_eq!(
            object.get("PrimaryImageAspectRatio"),
            Some(&json!(2.0f64 / 3.0f64))
        );
        assert_eq!(object.get("ChildCount"), Some(&json!(0)));
        assert_eq!(object.get("DisplayOrder"), Some(&json!("Aired")));
        assert_eq!(object.get("LocalTrailerCount"), Some(&json!(0)));
        assert_eq!(object.get("LockData"), Some(&json!(false)));
        assert_eq!(object.get("LockedFields"), Some(&json!([])));
        assert_eq!(object.get("ExternalUrls"), Some(&json!([])));
        assert_eq!(object.get("RemoteTrailers"), Some(&json!([])));
        assert_eq!(object.get("TagItems"), Some(&json!([])));
        assert_eq!(object.get("Taglines"), Some(&json!([])));
        assert_eq!(object.get("OfficialRating"), Some(&json!("")));
        assert_eq!(object.get("OriginalTitle"), Some(&json!("Series One")));
        assert_eq!(object.get("FileName"), Some(&json!("Series One")));
        assert_eq!(object.get("ForcedSortName"), Some(&json!("Series One")));
        assert_eq!(object.get("DisplayPreferencesId"), Some(&json!("series-1")));
        assert_eq!(object.get("PresentationUniqueKey"), Some(&json!("series-1")));
        assert!(object.get("Etag").and_then(Value::as_str).is_some());

        let genre_items = object
            .get("GenreItems")
            .and_then(Value::as_array)
            .expect("genre items");
        assert_eq!(genre_items.len(), 2);
        assert_eq!(genre_items[0].get("Name"), Some(&json!("Drama")));
        assert!(genre_items[0].get("Id").and_then(Value::as_i64).is_some());
        assert_eq!(genre_items[1].get("Name"), Some(&json!("Mystery")));
        assert!(genre_items[1].get("Id").and_then(Value::as_i64).is_some());

        let studios = object
            .get("Studios")
            .and_then(Value::as_array)
            .expect("studios");
        assert_eq!(studios.len(), 1);
        assert_eq!(studios[0].get("Name"), Some(&json!("Studio One")));
        assert!(studios[0].get("Id").and_then(Value::as_i64).is_some());

        let date_created = object
            .get("DateCreated")
            .and_then(Value::as_str)
            .expect("date created");
        assert_eq!(date_created, "2026-02-22T11:14:10.8565300Z");
        assert_eq!(
            object.get("DateModified"),
            Some(&json!("2026-02-22T11:14:10.8565300Z"))
        );
        assert_eq!(
            object.get("PremiereDate"),
            Some(&json!("2024-09-17T00:00:00.0000000Z"))
        );
    }

    #[test]
    fn compat_single_item_json_derives_tag_items_from_tags() {
        let mut item = make_test_item("series-tagged", "Tagged Series", "Series");
        item.tags = Some(vec!["历史".to_string(), "战争".to_string()]);

        let payload = compat_single_item_json(item, "test-server-id");
        let object = payload.as_object().expect("single item payload as object");

        let tag_items = object
            .get("TagItems")
            .and_then(Value::as_array)
            .expect("tag items");
        assert_eq!(tag_items.len(), 2);
        assert_eq!(tag_items[0].get("Name"), Some(&json!("历史")));
        assert!(tag_items[0].get("Id").and_then(Value::as_i64).is_some());
        assert_eq!(tag_items[1].get("Name"), Some(&json!("战争")));
        assert!(tag_items[1].get("Id").and_then(Value::as_i64).is_some());
    }

    #[test]
    fn compat_single_item_json_adds_vidhub_safety_fields_for_episode_detail() {
        let mut item = make_test_item("episode-1", "Episode One", "Episode");
        item.path = "/mnt/media/series/season1/episode-1.strm".to_string();
        item.sort_name = Some("Episode 1".to_string());
        item.date_created = Some("2026-02-22T11:14:10.856530+00:00".to_string());
        item.genres = Some(vec!["Reality-TV".to_string()]);
        item.community_rating = Some(10.0);
        item.series_id = Some("series-1".to_string());
        item.media_sources = Some(vec![ls_domain::jellyfin::MediaSourceInfoDto {
            name: None,
            id:"episode-1".to_string(),
            path: Some(item.path.clone()),
            protocol: "File".to_string(),
            container: Some("strm".to_string()),
            runtime_ticks: Some(38030720000),
            bitrate: Some(3_744_958),
            supports_direct_play: true,
            supports_direct_stream: true,
            supports_transcoding: false,
            chapters: vec![],
            media_streams: vec![
                ls_domain::jellyfin::MediaStreamDto {
                    index: 0,
                    stream_type: "Video".to_string(),
                    language: Some("und".to_string()),
                    is_external: false,
                    path: None,
                    codec: Some("hevc".to_string()),
                    display_title: None,
                    width: Some(1920),
                    height: Some(1080),
                    average_frame_rate: Some(23.976023976023978),
                    real_frame_rate: Some(23.976023976023978),
                    profile: Some("Main 10".to_string()),
                    level: Some(120),
                    channels: None,
                    sample_rate: None,
                    channel_layout: None,
                    bit_rate: Some(3_099_584),
                    color_range: None,
                    color_space: None,
                    color_transfer: None,
                    color_primaries: None,
                    bit_depth: None,
                    video_range: None,
                    video_range_type: None,
                    hdr10_plus_present_flag: None,
                    dv_version_major: None,
                    dv_version_minor: None,
                    dv_profile: None,
                    dv_level: None,
                    rpu_present_flag: None,
                    el_present_flag: None,
                    bl_present_flag: None,
                    dv_bl_signal_compatibility_id: None,
                    is_default: Some(true),
                    is_forced: Some(false),
                },
                ls_domain::jellyfin::MediaStreamDto {
                    index: 1,
                    stream_type: "Audio".to_string(),
                    language: Some("kor".to_string()),
                    is_external: false,
                    path: None,
                    codec: Some("eac3".to_string()),
                    display_title: None,
                    width: None,
                    height: None,
                    average_frame_rate: None,
                    real_frame_rate: None,
                    profile: None,
                    level: None,
                    channels: Some(6),
                    sample_rate: Some(48_000),
                    channel_layout: Some("5.1(side)".to_string()),
                    bit_rate: Some(640_000),
                    color_range: None,
                    color_space: None,
                    color_transfer: None,
                    color_primaries: None,
                    bit_depth: None,
                    video_range: None,
                    video_range_type: None,
                    hdr10_plus_present_flag: None,
                    dv_version_major: None,
                    dv_version_minor: None,
                    dv_profile: None,
                    dv_level: None,
                    rpu_present_flag: None,
                    el_present_flag: None,
                    bl_present_flag: None,
                    dv_bl_signal_compatibility_id: None,
                    is_default: Some(true),
                    is_forced: Some(false),
                },
            ],
        }]);

        let payload = compat_single_item_json(item, "test-server-id");
        let object = payload.as_object().expect("single item payload as object");

        assert_eq!(object.get("ServerId"), Some(&json!("test-server-id")));
        assert_eq!(object.get("SupportsSync"), Some(&json!(true)));
        assert_eq!(object.get("FileName"), Some(&json!("episode-1.strm")));
        assert_eq!(object.get("ForcedSortName"), Some(&json!("Episode 1")));
        assert_eq!(
            object.get("DateModified"),
            Some(&json!("2026-02-22T11:14:10.8565300Z"))
        );
        assert_eq!(object.get("DisplayPreferencesId"), Some(&json!("episode-1")));
        assert_eq!(object.get("PresentationUniqueKey"), Some(&json!("episode-1")));
        assert!(object.get("Etag").and_then(Value::as_str).is_some());
        assert_eq!(object.get("PartCount"), Some(&json!(1)));
        assert_eq!(object.get("Size"), Some(&json!(0)));
        assert_eq!(object.get("Width"), Some(&json!(1920)));
        assert_eq!(object.get("Height"), Some(&json!(1080)));
        assert_eq!(
            object.get("PrimaryImageAspectRatio"),
            Some(&json!(16.0f64 / 9.0f64))
        );
        assert_eq!(object.get("CommunityRating"), Some(&json!(10)));
        assert_eq!(object.get("ParentBackdropItemId"), Some(&json!("series-1")));
        assert_eq!(object.get("ParentLogoItemId"), Some(&json!("series-1")));
        assert_eq!(object.get("ParentBackdropImageTags"), Some(&json!([])));
        assert_eq!(object.get("BackdropImageTags"), Some(&json!([])));
        assert_eq!(object.get("ExternalUrls"), Some(&json!([])));
        assert_eq!(object.get("RemoteTrailers"), Some(&json!([])));
        assert_eq!(object.get("TagItems"), Some(&json!([])));
        assert_eq!(object.get("Taglines"), Some(&json!([])));
        assert_eq!(object.get("LockedFields"), Some(&json!([])));
        assert_eq!(object.get("LockData"), Some(&json!(false)));
        assert_eq!(object.get("LocalTrailerCount"), Some(&json!(0)));
        assert_eq!(object.get("Chapters"), Some(&json!([])));
        assert!(object.get("AirDays").is_none());

        let media_sources = object
            .get("MediaSources")
            .and_then(Value::as_array)
            .expect("episode media sources");
        assert_eq!(media_sources.len(), 1);
        assert_eq!(media_sources[0].get("Type"), Some(&json!("Default")));
        assert_eq!(media_sources[0].get("ItemId"), Some(&json!("episode-1")));
        assert_eq!(media_sources[0].get("Size"), Some(&json!(0)));
        assert_eq!(
            media_sources[0].get("DefaultAudioStreamIndex"),
            Some(&json!(1))
        );
        assert_eq!(media_sources[0].get("Chapters"), Some(&json!([])));
        assert_eq!(
            media_sources[0].get("RequiredHttpHeaders"),
            Some(&json!({}))
        );
        assert_eq!(media_sources[0].get("Formats"), Some(&json!([])));
        assert_eq!(media_sources[0].get("SupportsProbing"), Some(&json!(true)));
        assert_eq!(media_sources[0].get("IsRemote"), Some(&json!(true)));
        assert!(
            media_sources[0].get("DirectStreamUrl").and_then(Value::as_str).is_some(),
            "strm source should have DirectStreamUrl"
        );
        assert_eq!(
            media_sources[0].get("Path"),
            media_sources[0].get("DirectStreamUrl"),
            "remote detail source path should use stream endpoint"
        );

        let source_streams = media_sources[0]
            .get("MediaStreams")
            .and_then(Value::as_array)
            .expect("episode source streams");
        assert_eq!(source_streams[0].get("Protocol"), Some(&json!("File")));
        assert_eq!(
            source_streams[0].get("IsTextSubtitleStream"),
            Some(&json!(false))
        );
        assert_eq!(
            source_streams[0].get("SupportsExternalStream"),
            Some(&json!(false))
        );
        assert_eq!(
            source_streams[1].get("IsTextSubtitleStream"),
            Some(&json!(false))
        );

        let media_streams = object
            .get("MediaStreams")
            .and_then(Value::as_array)
            .expect("episode top-level media streams");
        assert_eq!(media_streams.len(), 2);
        assert_eq!(media_streams[0].get("Level"), Some(&json!(120)));
        assert!(media_streams[0].get("Path").is_none());

        let genre_items = object
            .get("GenreItems")
            .and_then(Value::as_array)
            .expect("genre items");
        assert_eq!(genre_items.len(), 1);
        assert_eq!(genre_items[0].get("Name"), Some(&json!("Reality-TV")));
        assert!(genre_items[0].get("Id").and_then(Value::as_i64).is_some());
    }

    #[test]
    fn compat_single_item_json_adds_vidhub_safety_fields_for_movie_detail() {
        let mut item = make_test_item("movie-1", "Movie One", "Movie");
        item.path = "/mnt/media/movies/movie-1.strm".to_string();
        item.sort_name = Some("Movie Sort".to_string());
        item.date_created = Some("2026-02-22T11:14:10.856530+00:00".to_string());
        item.media_type = Some("Video".to_string());
        item.genres = Some(vec!["Animation".to_string()]);
        item.media_sources = Some(vec![ls_domain::jellyfin::MediaSourceInfoDto {
            name: None,
            id:"movie-1".to_string(),
            path: Some("https://cdn.example.com/movie-1.mp4".to_string()),
            protocol: "Http".to_string(),
            container: Some("mp4".to_string()),
            runtime_ticks: None,
            bitrate: None,
            supports_direct_play: true,
            supports_direct_stream: true,
            supports_transcoding: false,
            chapters: vec![],
            media_streams: vec![],
        }]);

        let payload = compat_single_item_json(item, "test-server-id");
        let object = payload.as_object().expect("single item payload as object");

        assert_eq!(object.get("ServerId"), Some(&json!("test-server-id")));
        assert_eq!(object.get("SupportsSync"), Some(&json!(true)));
        assert_eq!(
            object.get("DateModified"),
            Some(&json!("2026-02-22T11:14:10.8565300Z"))
        );
        assert_eq!(object.get("DisplayPreferencesId"), Some(&json!("movie-1")));
        assert_eq!(object.get("PresentationUniqueKey"), Some(&json!("movie-1")));
        assert!(object.get("Etag").and_then(Value::as_str).is_some());
        assert_eq!(object.get("PartCount"), Some(&json!(1)));
        assert_eq!(object.get("FileName"), Some(&json!("movie-1.strm")));
        assert_eq!(object.get("ForcedSortName"), Some(&json!("Movie Sort")));
        assert_eq!(object.get("OfficialRating"), Some(&json!("")));
        assert_eq!(object.get("OriginalTitle"), Some(&json!("Movie One")));
        assert_eq!(object.get("ProductionLocations"), Some(&json!([])));
        assert_eq!(
            object.get("PrimaryImageAspectRatio"),
            Some(&json!(2.0f64 / 3.0f64))
        );
        assert_eq!(object.get("Bitrate"), Some(&json!(0)));
        assert_eq!(object.get("ExternalUrls"), Some(&json!([])));
        assert_eq!(object.get("RemoteTrailers"), Some(&json!([])));
        assert_eq!(object.get("TagItems"), Some(&json!([])));
        assert_eq!(object.get("Taglines"), Some(&json!([])));
        assert_eq!(object.get("LockedFields"), Some(&json!([])));
        assert_eq!(object.get("LockData"), Some(&json!(false)));
        assert_eq!(object.get("LocalTrailerCount"), Some(&json!(0)));
        assert_eq!(object.get("Chapters"), Some(&json!([])));

        let media_sources = object
            .get("MediaSources")
            .and_then(Value::as_array)
            .expect("movie media sources");
        assert_eq!(media_sources.len(), 1);
        assert_eq!(media_sources[0].get("Type"), Some(&json!("Default")));
        assert_eq!(media_sources[0].get("ItemId"), Some(&json!("movie-1")));
        assert_eq!(media_sources[0].get("Name"), Some(&json!("Movie One")));
        assert_eq!(media_sources[0].get("IsRemote"), Some(&json!(true)));
        assert_eq!(media_sources[0].get("Bitrate"), Some(&json!(0)));
        assert_eq!(media_sources[0].get("RunTimeTicks"), Some(&json!(0)));
        assert_eq!(media_sources[0].get("Chapters"), Some(&json!([])));
        assert_eq!(
            media_sources[0].get("RequiredHttpHeaders"),
            Some(&json!({}))
        );
        assert_eq!(media_sources[0].get("Formats"), Some(&json!([])));
        assert_eq!(media_sources[0].get("SupportsProbing"), Some(&json!(true)));

        let source_streams = media_sources[0]
            .get("MediaStreams")
            .and_then(Value::as_array)
            .expect("movie source streams");
        assert_eq!(source_streams.len(), 1);
        assert_eq!(source_streams[0].get("Type"), Some(&json!("Video")));
        assert_eq!(source_streams[0].get("Protocol"), Some(&json!("Http")));
        assert_eq!(
            source_streams[0].get("IsTextSubtitleStream"),
            Some(&json!(false))
        );
        assert_eq!(source_streams[0].get("DisplayLanguage"), Some(&json!("UND")));
        assert_eq!(source_streams[0].get("AspectRatio"), Some(&json!("0:0")));
        assert_eq!(source_streams[0].get("ExtendedVideoType"), Some(&json!("")));
        assert_eq!(source_streams[0].get("RefFrames"), Some(&json!(0)));

        let media_streams = object
            .get("MediaStreams")
            .and_then(Value::as_array)
            .expect("movie top-level media streams");
        assert_eq!(media_streams.len(), 1);
        assert_eq!(media_streams[0].get("Type"), Some(&json!("Video")));

        let genre_items = object
            .get("GenreItems")
            .and_then(Value::as_array)
            .expect("genre items");
        assert_eq!(genre_items.len(), 1);
        assert_eq!(genre_items[0].get("Name"), Some(&json!("Animation")));
        assert!(genre_items[0].get("Id").and_then(Value::as_i64).is_some());
    }

    #[test]
    fn compat_single_item_json_marks_embedded_subtitles_clearly() {
        let mut item = make_test_item("movie-sub", "Movie Subtitle", "Movie");
        item.path = "/mnt/media/movies/movie-sub.mkv".to_string();
        item.media_type = Some("Video".to_string());
        item.media_sources = Some(vec![ls_domain::jellyfin::MediaSourceInfoDto {
            name: None,
            id: "movie-sub".to_string(),
            path: Some(item.path.clone()),
            protocol: "File".to_string(),
            container: Some("mkv".to_string()),
            runtime_ticks: Some(3_800_000_000),
            bitrate: Some(8_000_000),
            supports_direct_play: true,
            supports_direct_stream: true,
            supports_transcoding: false,
            chapters: vec![],
            media_streams: vec![
                ls_domain::jellyfin::MediaStreamDto {
                    index: 0,
                    stream_type: "Video".to_string(),
                    language: Some("und".to_string()),
                    is_external: false,
                    path: None,
                    codec: Some("hevc".to_string()),
                    display_title: None,
                    width: Some(1920),
                    height: Some(1080),
                    average_frame_rate: Some(23.976023976023978),
                    real_frame_rate: Some(23.976023976023978),
                    profile: Some("Main 10".to_string()),
                    level: Some(120),
                    channels: None,
                    sample_rate: None,
                    channel_layout: None,
                    bit_rate: Some(8_000_000),
                    color_range: None,
                    color_space: None,
                    color_transfer: None,
                    color_primaries: None,
                    bit_depth: None,
                    video_range: None,
                    video_range_type: None,
                    hdr10_plus_present_flag: None,
                    dv_version_major: None,
                    dv_version_minor: None,
                    dv_profile: None,
                    dv_level: None,
                    rpu_present_flag: None,
                    el_present_flag: None,
                    bl_present_flag: None,
                    dv_bl_signal_compatibility_id: None,
                    is_default: Some(true),
                    is_forced: Some(false),
                },
                ls_domain::jellyfin::MediaStreamDto {
                    index: 5,
                    stream_type: "Subtitle".to_string(),
                    language: Some("jpn".to_string()),
                    is_external: false,
                    path: None,
                    codec: Some("hdmv_pgs_subtitle".to_string()),
                    display_title: None,
                    width: None,
                    height: None,
                    average_frame_rate: None,
                    real_frame_rate: None,
                    profile: None,
                    level: None,
                    channels: None,
                    sample_rate: None,
                    channel_layout: None,
                    bit_rate: None,
                    color_range: None,
                    color_space: None,
                    color_transfer: None,
                    color_primaries: None,
                    bit_depth: None,
                    video_range: None,
                    video_range_type: None,
                    hdr10_plus_present_flag: None,
                    dv_version_major: None,
                    dv_version_minor: None,
                    dv_profile: None,
                    dv_level: None,
                    rpu_present_flag: None,
                    el_present_flag: None,
                    bl_present_flag: None,
                    dv_bl_signal_compatibility_id: None,
                    is_default: Some(true),
                    is_forced: Some(false),
                },
                ls_domain::jellyfin::MediaStreamDto {
                    index: 6,
                    stream_type: "Subtitle".to_string(),
                    language: Some("chi".to_string()),
                    is_external: true,
                    path: Some("/mnt/media/movies/movie-sub.zh.ass".to_string()),
                    codec: Some("ass".to_string()),
                    display_title: None,
                    width: None,
                    height: None,
                    average_frame_rate: None,
                    real_frame_rate: None,
                    profile: None,
                    level: None,
                    channels: None,
                    sample_rate: None,
                    channel_layout: None,
                    bit_rate: None,
                    color_range: None,
                    color_space: None,
                    color_transfer: None,
                    color_primaries: None,
                    bit_depth: None,
                    video_range: None,
                    video_range_type: None,
                    hdr10_plus_present_flag: None,
                    dv_version_major: None,
                    dv_version_minor: None,
                    dv_profile: None,
                    dv_level: None,
                    rpu_present_flag: None,
                    el_present_flag: None,
                    bl_present_flag: None,
                    dv_bl_signal_compatibility_id: None,
                    is_default: Some(false),
                    is_forced: Some(false),
                },
            ],
        }]);

        let payload = compat_single_item_json(item, "test-server-id");
        let streams = payload["MediaSources"][0]["MediaStreams"]
            .as_array()
            .expect("media streams");

        assert_eq!(streams[1].get("IsTextSubtitleStream"), Some(&json!(false)));
        assert_eq!(streams[1].get("DeliveryMethod"), Some(&json!("Embed")));
        assert_eq!(
            streams[1].get("SubtitleLocationType"),
            Some(&json!("InternalStream"))
        );

        assert_eq!(streams[2].get("IsTextSubtitleStream"), Some(&json!(true)));
        assert_eq!(streams[2].get("DeliveryMethod"), Some(&json!("External")));
        assert_eq!(
            streams[2].get("SubtitleLocationType"),
            Some(&json!("ExternalFile"))
        );
    }

    #[test]
    fn compat_single_item_json_infers_top_level_chapters_from_media_source() {
        let mut item = make_test_item("movie-2", "Movie Two", "Movie");
        item.path = "/mnt/media/movies/movie-2.strm".to_string();
        item.media_type = Some("Video".to_string());
        item.media_sources = Some(vec![ls_domain::jellyfin::MediaSourceInfoDto {
            name: None,
            id:"movie-2".to_string(),
            path: Some("https://cdn.example.com/movie-2.mp4".to_string()),
            protocol: "Http".to_string(),
            container: Some("mp4".to_string()),
            runtime_ticks: Some(1_200_000_000),
            bitrate: Some(3_500_000),
            supports_direct_play: true,
            supports_direct_stream: true,
            supports_transcoding: false,
            chapters: vec![
                ls_domain::jellyfin::ChapterInfoDto {
                    start_position_ticks: 0,
                    name: Some("Intro".to_string()),
                    image_tag: None,
                    marker_type: Some("Chapter".to_string()),
                    chapter_index: Some(0),
                },
                ls_domain::jellyfin::ChapterInfoDto {
                    start_position_ticks: 600_000_000,
                    name: Some("Act 1".to_string()),
                    image_tag: None,
                    marker_type: Some("Chapter".to_string()),
                    chapter_index: Some(1),
                },
            ],
            media_streams: vec![],
        }]);

        let payload = compat_single_item_json(item, "test-server-id");
        let object = payload.as_object().expect("single item payload as object");
        let chapters = object
            .get("Chapters")
            .and_then(Value::as_array)
            .expect("top-level chapters");
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].get("ChapterIndex"), Some(&json!(0)));
        assert_eq!(chapters[0].get("StartPositionTicks"), Some(&json!(0)));
        assert_eq!(chapters[0].get("Name"), Some(&json!("Intro")));
        assert_eq!(chapters[0].get("MarkerType"), Some(&json!("Chapter")));
        assert_eq!(chapters[1].get("ChapterIndex"), Some(&json!(1)));
        assert_eq!(chapters[1].get("StartPositionTicks"), Some(&json!(600_000_000)));
        assert_eq!(chapters[1].get("Name"), Some(&json!("Act 1")));
    }

    #[test]
    fn compat_single_item_json_derives_external_urls_from_provider_ids() {
        let mut item = make_test_item("movie-3", "Movie Three", "Movie");
        item.media_type = Some("Video".to_string());
        item.provider_ids = Some(std::collections::HashMap::from([
            ("Imdb".to_string(), "tt0111161".to_string()),
            ("Tmdb".to_string(), "278".to_string()),
            ("Tvdb".to_string(), "81189".to_string()),
        ]));

        let payload = compat_single_item_json(item, "test-server-id");
        let object = payload.as_object().expect("single item payload as object");
        let external_urls = object
            .get("ExternalUrls")
            .and_then(Value::as_array)
            .expect("external urls");
        assert!(external_urls.len() >= 3);
        assert!(external_urls.iter().all(|entry| {
            entry.get("Name").and_then(Value::as_str).is_some()
                && entry.get("Url").and_then(Value::as_str).is_some()
        }));
        assert!(external_urls.iter().any(|entry| {
            entry.get("Name") == Some(&json!("IMDb"))
                && entry.get("Url") == Some(&json!("https://www.imdb.com/title/tt0111161"))
        }));
        assert!(external_urls.iter().any(|entry| {
            entry.get("Name") == Some(&json!("TMDb"))
                && entry.get("Url") == Some(&json!("https://www.themoviedb.org/movie/278"))
        }));
        assert!(external_urls.iter().any(|entry| {
            entry.get("Name") == Some(&json!("TVDB"))
                && entry.get("Url") == Some(&json!("https://www.thetvdb.com/?id=81189"))
        }));
    }

    #[test]
    fn compat_single_item_json_keeps_fractional_community_rating_type() {
        let mut item = make_test_item("movie-1", "Movie One", "Movie");
        item.media_type = Some("Video".to_string());
        item.community_rating = Some(9.3);

        let payload = compat_single_item_json(item, "test-server-id");
        let object = payload.as_object().expect("single item payload as object");
        assert_eq!(object.get("CommunityRating"), Some(&json!(9.3)));
    }

    #[test]
    fn apply_episode_series_context_populates_series_tags() {
        let mut payload = json!({
            "Id": "episode-1",
            "Type": "Episode",
            "SeriesId": "series-1"
        });
        let mut series = make_test_item("series-1", "Series One", "Series");
        series.image_tags = Some(std::collections::HashMap::from([
            ("Primary".to_string(), "series-primary-tag".to_string()),
            ("Logo".to_string(), "series-logo-tag".to_string()),
        ]));
        series.backdrop_image_tags = Some(vec!["series-backdrop-tag".to_string()]);

        apply_episode_series_context(&mut payload, &series);

        let object = payload.as_object().expect("episode payload as object");
        assert_eq!(object.get("SeriesName"), Some(&json!("Series One")));
        assert_eq!(object.get("ParentBackdropItemId"), Some(&json!("series-1")));
        assert_eq!(object.get("ParentLogoItemId"), Some(&json!("series-1")));
        assert_eq!(
            object.get("SeriesPrimaryImageTag"),
            Some(&json!("series-primary-tag"))
        );
        assert_eq!(
            object.get("ParentLogoImageTag"),
            Some(&json!("series-logo-tag"))
        );
        assert_eq!(
            object.get("ParentBackdropImageTags"),
            Some(&json!(["series-backdrop-tag"]))
        );
    }

    #[test]
    fn user_item_data_update_body_derives_played_from_extended_fields() {
        let from_play_count: UserItemDataUpdateBody = serde_json::from_value(json!({
            "PlayCount": 3
        }))
        .expect("play count payload parse");
        assert_eq!(from_play_count.effective_played(), Some(true));
        assert!(!from_play_count.is_empty());

        let from_percentage: UserItemDataUpdateBody = serde_json::from_value(json!({
            "playedPercentage": 0
        }))
        .expect("played percentage payload parse");
        assert_eq!(from_percentage.effective_played(), Some(false));

        let from_last_played: UserItemDataUpdateBody = serde_json::from_value(json!({
            "lastPlayedDate": "2026-02-21T10:00:00Z"
        }))
        .expect("last played payload parse");
        assert_eq!(from_last_played.effective_played(), Some(true));
    }

    #[test]
    fn hide_from_resume_query_defaults_to_hide_and_parses_aliases() {
        let default_query: HideFromResumeQuery =
            serde_json::from_value(json!({})).expect("default query parse");
        assert!(default_query.should_hide());

        let explicit_true: HideFromResumeQuery =
            serde_json::from_value(json!({"Hide": true})).expect("explicit true parse");
        assert!(explicit_true.should_hide());

        let explicit_false: HideFromResumeQuery =
            serde_json::from_value(json!({"hide": false})).expect("explicit false parse");
        assert!(!explicit_false.should_hide());
    }

    #[test]
    fn find_person_by_name_is_case_insensitive_exact_match() {
        let persons = vec![
            BaseItemDto {
                id: "1".to_string(),
                name: "Tom Hanks".to_string(),
                item_type: "Person".to_string(),
                path: String::new(),
                is_folder: Some(false),
                media_type: None,
                container: None,
                location_type: None,
                can_delete: Some(false),
                can_download: Some(false),
                collection_type: None,
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
                date_created: None,
                child_count: None,
                recursive_item_count: None,
                play_access: None,
            },
            BaseItemDto {
                id: "2".to_string(),
                name: "Tom Hardy".to_string(),
                item_type: "Person".to_string(),
                path: String::new(),
                is_folder: Some(false),
                media_type: None,
                container: None,
                location_type: None,
                can_delete: Some(false),
                can_download: Some(false),
                collection_type: None,
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
                date_created: None,
                child_count: None,
                recursive_item_count: None,
                play_access: None,
            },
        ];

        let found = find_person_by_name(persons, "tom hanks").expect("person match");
        assert_eq!(found.id, "1");

        let fuzzy = find_person_by_name(vec![make_test_item("3", "Tom Holland", "Person")], "holl")
            .expect("fuzzy person match");
        assert_eq!(fuzzy.id, "3");
    }

    #[test]
    fn person_lookup_id_prefers_resolved_alias_mapping() {
        let resolved = Uuid::new_v4();
        assert_eq!(person_lookup_id("135441", Some(resolved)), Some(resolved));
    }

    #[test]
    fn person_lookup_id_parses_uuid_and_rejects_plain_text() {
        let person_uuid = Uuid::new_v4();
        assert_eq!(
            person_lookup_id(person_uuid.to_string().as_str(), None),
            Some(person_uuid)
        );
        assert_eq!(person_lookup_id("张艺谋", None), None);
    }

    #[test]
    fn find_root_collection_item_matches_view_item_id() {
        let collection_id = Uuid::new_v4();
        let mut collection = make_test_item(&collection_id.to_string(), "电影", "CollectionFolder");
        collection.is_folder = Some(true);

        let root_items = QueryResultDto {
            items: vec![collection],
            total_record_count: 1,
            start_index: 0,
        };

        let found = find_root_collection_item(root_items, collection_id).expect("collection item");
        assert_eq!(found.id, collection_id.to_string());
        assert_eq!(found.item_type, "CollectionFolder");
    }

    #[test]
    fn legacy_user_image_index_helper_rejects_negative_index() {
        assert!(is_supported_legacy_user_image_index(0));
        assert!(is_supported_legacy_user_image_index(5));
        assert!(!is_supported_legacy_user_image_index(-1));
    }

    #[test]
    fn user_image_type_compatibility_accepts_common_jellyfin_types() {
        assert_eq!(normalize_user_image_type("Primary"), Some("primary"));
        assert_eq!(normalize_user_image_type("thumb"), Some("primary"));
        assert_eq!(normalize_user_image_type("Banner"), Some("primary"));
        assert_eq!(normalize_user_image_type("Logo"), Some("primary"));
        assert_eq!(normalize_user_image_type("Backdrop"), Some("primary"));
        assert_eq!(normalize_user_image_type("Disc"), None);
    }

    #[test]
    fn parse_compat_uuid_is_permissive_and_ignores_invalid_inputs() {
        let uid = Uuid::new_v4();
        let wrapped = format!("  {uid}  ");
        assert_eq!(parse_compat_uuid(Some(wrapped.as_str())), Some(uid));
        assert_eq!(parse_compat_uuid(Some("not-a-uuid")), None);
        assert_eq!(parse_compat_uuid(Some("   ")), None);
        assert_eq!(parse_compat_uuid(None), None);
    }

    #[test]
    fn authenticate_by_name_request_accepts_pascal_camel_and_legacy_aliases() {
        let legacy: AuthenticateByNameRequest = serde_json::from_value(json!({
            "Username": "alice",
            "Pw": "legacy-pass"
        }))
        .expect("legacy auth payload parse");
        assert_eq!(legacy.username, "alice");
        assert_eq!(legacy.pw.as_deref(), Some("legacy-pass"));

        let camel: AuthenticateByNameRequest = serde_json::from_value(json!({
            "userName": "bob",
            "password": "camel-pass"
        }))
        .expect("camel auth payload parse");
        assert_eq!(camel.username, "bob");
        assert_eq!(camel.password.as_deref(), Some("camel-pass"));

        let name_alias: AuthenticateByNameRequest = serde_json::from_value(json!({
            "name": "carol",
            "Password": "legacy-password-alias"
        }))
        .expect("name alias auth payload parse");
        assert_eq!(name_alias.username, "carol");
        assert_eq!(name_alias.password.as_deref(), Some("legacy-password-alias"));

        // Both Pw and Password as separate fields — must not fail with duplicate field error
        let both: AuthenticateByNameRequest = serde_json::from_value(json!({
            "Username": "dave",
            "Pw": "pw-value",
            "Password": "password-value"
        }))
        .expect("both Pw and Password should parse");
        assert_eq!(both.pw.as_deref(), Some("pw-value"));
        assert_eq!(both.password.as_deref(), Some("password-value"));
    }

    #[test]
    fn parse_authenticate_by_name_payload_accepts_form_and_query_fallback() {
        let body_payload = parse_authenticate_by_name_payload(
            b"Username=alice&Pw=legacy-pass",
            "",
        )
        .expect("form payload parse");
        assert_eq!(body_payload.username, "alice");
        assert_eq!(body_payload.pw.as_deref(), Some("legacy-pass"));

        let query_payload = parse_authenticate_by_name_payload(
            b"",
            "name=%E5%BC%A0%E8%89%BA%E8%B0%8B&Password=hello%2Bworld",
        )
        .expect("query payload parse");
        assert_eq!(query_payload.username, "张艺谋");
        assert_eq!(query_payload.password.as_deref(), Some("hello+world"));
    }

    #[test]
    fn library_virtual_folder_payloads_accept_pascal_and_camel_aliases() {
        let rename: RenameVirtualFolderRequest = serde_json::from_value(json!({
            "name": "Movies",
            "newName": "Movies-New",
            "refreshLibrary": true
        }))
        .expect("rename payload parse");
        assert_eq!(rename.name.as_deref(), Some("Movies"));
        assert_eq!(rename.new_name.as_deref(), Some("Movies-New"));
        assert_eq!(rename.refresh_library, Some(true));

        let add_path: AddMediaPathRequest = serde_json::from_value(json!({
            "Name": "Movies",
            "pathInfo": {
                "path": "/data/movies"
            },
            "RefreshLibrary": false
        }))
        .expect("add media path payload parse");
        assert_eq!(
            resolve_media_path(add_path.path.as_deref(), add_path.path_info.as_ref()).as_deref(),
            Some("/data/movies")
        );

        let update_path: UpdateMediaPathRequest = serde_json::from_value(json!({
            "name": "Movies",
            "pathInfo": {
                "path": " /mnt/media "
            }
        }))
        .expect("update media path payload parse");
        assert_eq!(
            resolve_media_path(None, update_path.path_info.as_ref()).as_deref(),
            Some("/mnt/media")
        );

        let options: UpdateLibraryOptionsRequest = serde_json::from_value(json!({
            "id": Uuid::new_v4().to_string(),
            "libraryOptions": {
                "contentType": "tvshows",
                "pathInfos": [
                    {"path": "/mnt/a"},
                    {"Path": "/mnt/b"}
                ]
            }
        }))
        .expect("update library options payload parse");
        assert!(options.id.is_some());
        let payload = options.library_options.expect("library options");
        assert_eq!(payload.content_type.as_deref(), Some("tvshows"));
        assert_eq!(
            payload.path_infos.unwrap_or_default().len(),
            2
        );
    }

    #[test]
    fn item_ancestors_query_accepts_user_id_aliases() {
        let user_id = Uuid::new_v4();
        let query: ItemAncestorsQuery = serde_json::from_value(json!({
            "userId": user_id
        }))
        .expect("ancestors query parse");
        assert_eq!(query.user_id, Some(user_id));
    }

    #[test]
    fn library_to_base_item_dto_maps_virtual_folder_shape() {
        let library = Library {
            id: Uuid::new_v4(),
            name: "Movies".to_string(),
            root_path: "/srv/media/movies".to_string(),
            paths: vec!["/srv/media/movies".to_string()],
            library_type: "Movie".to_string(),
            enabled: true,
            scan_interval_hours: 24,
            scraper_policy: json!({}),
            created_at: Utc::now(),
        };

        let dto = library_to_base_item_dto(&library);
        assert_eq!(dto.id, library.id.to_string());
        assert_eq!(dto.item_type, "CollectionFolder");
        assert_eq!(dto.collection_type.as_deref(), Some("movies"));
        assert_eq!(dto.is_folder, Some(true));
        assert_eq!(dto.location_type.as_deref(), Some("FileSystem"));
    }

    #[test]
    fn users_query_accepts_pascal_camel_and_boolish_values() {
        let legacy: UsersQuery = serde_json::from_value(json!({
            "IsHidden": "1",
            "IsDisabled": "FALSE"
        }))
        .expect("legacy users query parse");
        assert_eq!(legacy.is_hidden, Some(true));
        assert_eq!(legacy.is_disabled, Some(false));

        let modern: UsersQuery = serde_json::from_value(json!({
            "isHidden": "off",
            "isDisabled": "yes"
        }))
        .expect("modern users query parse");
        assert_eq!(modern.is_hidden, Some(false));
        assert_eq!(modern.is_disabled, Some(true));
    }

    #[test]
    fn create_user_by_name_accepts_common_name_and_password_aliases() {
        let legacy: CreateUserByName = serde_json::from_value(json!({
            "Name": "viewer-a",
            "Password": "legacy-pass-1"
        }))
        .expect("legacy create user payload parse");
        assert_eq!(legacy.name, "viewer-a");
        assert_eq!(legacy.password.as_deref(), Some("legacy-pass-1"));

        let modern: CreateUserByName = serde_json::from_value(json!({
            "userName": "viewer-b",
            "pw": "legacy-pw-alias"
        }))
        .expect("modern create user payload parse");
        assert_eq!(modern.name, "viewer-b");
        assert_eq!(modern.password.as_deref(), Some("legacy-pw-alias"));
    }

    #[test]
    fn latest_items_query_accepts_emby_aliases_and_limit_is_clamped() {
        let query: LatestItemsQuery = serde_json::from_value(json!({
            "Limit": 9999,
            "parentId": Uuid::new_v4(),
            "includeItemTypes": "Movie,Episode",
            "isPlayed": true
        }))
        .expect("latest items query parse");

        assert_eq!(normalize_latest_items_limit(query.limit), 200);
        assert!(query.parent_id.is_some());
        assert_eq!(query.include_item_types.as_deref(), Some("Movie,Episode"));
        assert_eq!(query.is_played, Some(true));
        assert_eq!(normalize_latest_items_limit(Some(0)), 1);
        assert_eq!(normalize_latest_items_limit(None), 20);
    }

    #[test]
    fn infer_latest_item_types_for_library_maps_common_collection_types() {
        assert_eq!(
            infer_latest_item_types_for_library("Series"),
            vec!["Series".to_string()]
        );
        assert_eq!(
            infer_latest_item_types_for_library("tvshows"),
            vec!["Series".to_string()]
        );
        assert_eq!(
            infer_latest_item_types_for_library("Movie"),
            vec!["Movie".to_string()]
        );
        assert_eq!(
            infer_latest_item_types_for_library("musicvideos"),
            vec!["MusicVideo".to_string()]
        );
        assert!(infer_latest_item_types_for_library("mixed").is_empty());
    }

    #[test]
    fn display_preferences_query_accepts_legacy_and_modern_aliases() {
        let legacy: DisplayPreferencesQuery = serde_json::from_value(json!({
            "UserId": Uuid::new_v4(),
            "Client": "Emby Web"
        }))
        .expect("legacy display preferences query parse");
        assert_eq!(legacy.client.as_deref(), Some("Emby Web"));

        let modern: DisplayPreferencesQuery = serde_json::from_value(json!({
            "userId": Uuid::new_v4(),
            "client": "Emby Android"
        }))
        .expect("modern display preferences query parse");
        assert_eq!(modern.client.as_deref(), Some("Emby Android"));
    }

    #[test]
    fn session_capabilities_query_accepts_known_fields() {
        let query: SessionCapabilitiesQuery = serde_json::from_value(json!({
            "Id": "session-1",
            "PlayableMediaTypes": "Video,Audio",
            "SupportedCommands": "MoveUp,MoveDown",
            "SupportsMediaControl": true,
            "supportsSync": false,
            "supportsPersistentIdentifier": true
        }))
        .expect("session capabilities query parse");

        assert_eq!(query._id.as_deref(), Some("session-1"));
        assert_eq!(query._playable_media_types.as_deref(), Some("Video,Audio"));
        assert_eq!(
            query._supported_commands.as_deref(),
            Some("MoveUp,MoveDown")
        );
        assert_eq!(query._supports_media_control, Some(true));
        assert_eq!(query._supports_sync, Some(false));
        assert_eq!(query._supports_persistent_identifier, Some(true));
    }

    #[test]
    fn forgot_password_payload_accepts_legacy_aliases() {
        let payload: ForgotPasswordRequest = serde_json::from_value(json!({
            "EnteredUsername": "demo-user"
        }))
        .expect("forgot password parse");
        assert_eq!(payload.entered_username.as_deref(), Some("demo-user"));

        let pin: ForgotPasswordPinRequest = serde_json::from_value(json!({
            "pin": "123456"
        }))
        .expect("forgot password pin parse");
        assert_eq!(pin._pin.as_deref(), Some("123456"));
    }

    #[test]
    fn authenticate_user_by_id_request_accepts_pw_aliases() {
        let legacy: AuthenticateUserByIdRequest = serde_json::from_value(json!({
            "Pw": "legacy-pass"
        }))
        .expect("authenticate by id legacy parse");
        assert_eq!(legacy.pw.as_deref(), Some("legacy-pass"));

        let modern: AuthenticateUserByIdRequest = serde_json::from_value(json!({
            "password": "modern-pass"
        }))
        .expect("authenticate by id modern parse");
        assert_eq!(modern.password.as_deref(), Some("modern-pass"));
    }

    #[test]
    fn user_rating_and_playing_query_accept_aliases() {
        let rating: ItemRatingQuery = serde_json::from_value(json!({
            "Likes": true
        }))
        .expect("item rating query parse");
        assert_eq!(rating.likes, Some(true));

        let playing: PlayingItemQuery = serde_json::from_value(json!({
            "mediaSourceId": "ms1",
            "positionTicks": 1200,
            "playMethod": "DirectPlay",
            "playSessionId": "play-1",
            "volumeLevel": 80
        }))
        .expect("playing item query parse");
        assert_eq!(playing.position_ticks, Some(1200));
        assert_eq!(playing.play_method.as_deref(), Some("DirectPlay"));
        assert_eq!(playing.play_session_id.as_deref(), Some("play-1"));
        assert_eq!(playing._volume_level, Some(80));
    }

    #[test]
    fn parse_legacy_favorite_action_accepts_add_and_delete() {
        assert_eq!(parse_legacy_favorite_action("Add"), Some(true));
        assert_eq!(parse_legacy_favorite_action("add"), Some(true));
        assert_eq!(parse_legacy_favorite_action("Delete"), Some(false));
        assert_eq!(parse_legacy_favorite_action("DELETE"), Some(false));
    }

    #[test]
    fn parse_legacy_favorite_action_rejects_unknown_action() {
        assert_eq!(parse_legacy_favorite_action("remove"), None);
        assert_eq!(parse_legacy_favorite_action("favorite"), None);
        assert_eq!(parse_legacy_favorite_action(""), None);
    }

    #[test]
    fn parse_legacy_played_action_accepts_add_and_delete() {
        assert_eq!(parse_legacy_played_action("Add"), Some(true));
        assert_eq!(parse_legacy_played_action("add"), Some(true));
        assert_eq!(parse_legacy_played_action("Delete"), Some(false));
        assert_eq!(parse_legacy_played_action("DELETE"), Some(false));
    }

    #[test]
    fn parse_legacy_played_action_rejects_unknown_action() {
        assert_eq!(parse_legacy_played_action("remove"), None);
        assert_eq!(parse_legacy_played_action("played"), None);
        assert_eq!(parse_legacy_played_action(""), None);
    }

    #[test]
    fn system_compat_query_parsers_accept_common_keys() {
        let activity: SystemActivityLogQuery = serde_json::from_value(json!({
            "StartIndex": 20,
            "limit": 40,
            "minDate": "2026-01-01T00:00:00Z"
        }))
        .expect("system activity log query parse");
        assert_eq!(activity.start_index, Some(20));
        assert_eq!(activity.limit, Some(40));
        assert_eq!(activity._min_date.as_deref(), Some("2026-01-01T00:00:00Z"));

        let log_query: SystemLogQuery = serde_json::from_value(json!({
            "Name": "server.log"
        }))
        .expect("system log query parse");
        assert_eq!(log_query.name.as_deref(), Some("server.log"));

        let connect_link: ConnectLinkQuery = serde_json::from_value(json!({
            "connectUsername": "guest-a"
        }))
        .expect("connect link query parse");
        assert_eq!(connect_link.connect_username.as_deref(), Some("guest-a"));
    }

    #[test]
    fn user_configuration_accepts_extended_fields_and_extra_payload() {
        let payload: UserConfiguration = serde_json::from_value(json!({
            "audioLanguagePreference": "zh-CN",
            "SubtitleLanguagePreference": "en-US",
            "PlayDefaultAudioTrack": "1",
            "rememberAudioSelections": "true",
            "RememberSubtitleSelections": 0,
            "subtitleMode": "Always",
            "EnableNextEpisodeAutoPlay": "off",
            "DisplayMissingEpisodes": "yes",
            "CustomClientToggle": "compat-extra"
        }))
        .expect("user configuration parse");

        assert_eq!(payload.audio_language_preference.as_deref(), Some("zh-CN"));
        assert_eq!(payload.subtitle_language_preference.as_deref(), Some("en-US"));
        assert_eq!(payload.play_default_audio_track, Some(true));
        assert_eq!(payload.remember_audio_selections, Some(true));
        assert_eq!(payload.remember_subtitle_selections, Some(false));
        assert_eq!(payload.subtitle_mode.as_deref(), Some("Always"));
        assert_eq!(payload.enable_next_episode_auto_play, Some(false));
        assert_eq!(payload.display_missing_episodes, Some(true));
        assert_eq!(
            payload.extra.get("CustomClientToggle"),
            Some(&json!("compat-extra"))
        );
    }

    #[test]
    fn user_policy_update_accepts_extended_fields_and_boolish_values() {
        let payload: UserPolicyUpdate = serde_json::from_value(json!({
            "isAdministrator": "1",
            "IsDisabled": "no",
            "enableAllFolders": "true",
            "EnableMediaPlayback": 1,
            "enableRemoteAccess": "off",
            "EnableLiveTvAccess": "yes",
            "EnableContentDeletion": "false",
            "EnableContentDownloading": "true",
            "remoteClientBitrateLimit": "24000000",
            "blockedTags": ["kids"],
            "enabledFolders": ["folder-a", "folder-b"],
            "invalidLoginAttemptCount": 3,
            "loginAttemptsBeforeLockout": "8",
            "PolicyVendorExtension": {"key": "value"}
        }))
        .expect("user policy parse");

        assert_eq!(payload.is_administrator, Some(true));
        assert_eq!(payload.is_disabled, Some(false));
        assert_eq!(payload.enable_all_folders, Some(true));
        assert_eq!(payload.enable_media_playback, Some(true));
        assert_eq!(payload.enable_remote_access, Some(false));
        assert_eq!(payload.enable_live_tv_access, Some(true));
        assert_eq!(payload.enable_content_deletion, Some(false));
        assert_eq!(payload.enable_content_downloading, Some(true));
        assert_eq!(payload.remote_client_bitrate_limit, Some(24_000_000));
        assert_eq!(payload.invalid_login_attempt_count, Some(3));
        assert_eq!(payload.login_attempts_before_lockout, Some(8));
        assert_eq!(payload.blocked_tags, Some(vec!["kids".to_string()]));
        assert_eq!(
            payload.enabled_folders,
            Some(vec!["folder-a".to_string(), "folder-b".to_string()])
        );
        assert_eq!(
            payload.extra.get("PolicyVendorExtension"),
            Some(&json!({"key": "value"}))
        );
    }

    #[test]
    fn playback_info_compat_query_accepts_pascal_and_camel_case_fields() {
        let uid = Uuid::new_v4();
        let legacy: PlaybackInfoCompatQuery = serde_json::from_value(json!({
            "UserId": uid,
            "MediaSourceId": "ms-legacy",
            "MaxStreamingBitrate": "18000000",
            "StartTimeTicks": "1200",
            "EnableDirectPlay": "true"
        }))
        .expect("legacy playback query parse");
        assert_eq!(parse_compat_uuid(legacy.user_id.as_deref()), Some(uid));
        assert_eq!(legacy.media_source_id.as_deref(), Some("ms-legacy"));
        assert_eq!(legacy._max_streaming_bitrate.as_deref(), Some("18000000"));
        assert_eq!(legacy._start_time_ticks.as_deref(), Some("1200"));
        assert_eq!(legacy._enable_direct_play.as_deref(), Some("true"));

        let modern: PlaybackInfoCompatQuery = serde_json::from_value(json!({
            "userId": uid,
            "mediaSourceId": "ms-modern",
            "audioStreamIndex": "2",
            "subtitleStreamIndex": "5",
            "autoOpenLiveStream": "1"
        }))
        .expect("modern playback query parse");
        assert_eq!(parse_compat_uuid(modern.user_id.as_deref()), Some(uid));
        assert_eq!(modern.media_source_id.as_deref(), Some("ms-modern"));
        assert_eq!(modern._audio_stream_index.as_deref(), Some("2"));
        assert_eq!(modern._subtitle_stream_index.as_deref(), Some("5"));
        assert_eq!(modern._auto_open_live_stream.as_deref(), Some("1"));
    }

    #[test]
    fn playback_info_compat_request_accepts_extended_body_fields() {
        let uid = Uuid::new_v4();
        let payload: PlaybackInfoCompatRequest = serde_json::from_value(json!({
            "userId": uid,
            "mediaSourceId": "ms-001",
            "maxStreamingBitrate": 22000000,
            "audioStreamIndex": 1,
            "subtitleStreamIndex": 4,
            "enableDirectPlay": true
        }))
        .expect("playback info body parse");

        assert_eq!(parse_compat_uuid(payload.user_id.as_deref()), Some(uid));
        assert_eq!(payload.media_source_id.as_deref(), Some("ms-001"));
        assert_eq!(payload._max_streaming_bitrate, Some(json!(22000000)));
        assert_eq!(payload._audio_stream_index, Some(json!(1)));
        assert_eq!(payload._subtitle_stream_index, Some(json!(4)));
        assert_eq!(payload._enable_direct_play, Some(json!(true)));
    }

    #[test]
    fn ensure_playback_info_compat_defaults_fills_media_source_shape() {
        let mut payload = json!({
            "MediaSources": [
                {
                    "Id": "source-1",
                    "Path": "https://cdn.example.com/video/source-1.m3u8",
                    "Protocol": "Http",
                    "RunTimeTicks": null,
                    "Bitrate": null,
                    "MediaStreams": [
                        {
                            "Index": 0,
                            "Type": "Video",
                            "IsExternal": false
                        }
                    ]
                }
            ],
            "PlaySessionId": "ps-1"
        });

        ensure_playback_info_compat_defaults(&mut payload, None, None);
        let source = payload["MediaSources"][0]
            .as_object()
            .expect("media source object");
        assert_eq!(source.get("Type"), Some(&json!("Default")));
        assert_eq!(source.get("ItemId"), Some(&json!("source-1")));
        assert_eq!(source.get("Name"), Some(&json!("source-1")));
        assert_eq!(source.get("RunTimeTicks"), Some(&json!(0)));
        assert_eq!(source.get("Bitrate"), Some(&json!(0)));
        assert_eq!(source.get("SupportsDirectPlay"), Some(&json!(true)));
        assert_eq!(source.get("SupportsDirectStream"), Some(&json!(true)));
        assert_eq!(source.get("SupportsTranscoding"), Some(&json!(false)));
        assert_eq!(source.get("Chapters"), Some(&json!([])));
        assert_eq!(source.get("Formats"), Some(&json!([])));
        assert_eq!(source.get("RequiredHttpHeaders"), Some(&json!({})));
        assert_eq!(source.get("SupportsProbing"), Some(&json!(true)));
        assert_eq!(source.get("IsRemote"), Some(&json!(true)));
        assert_eq!(source.get("Size"), Some(&json!(0)));
        assert!(
            source.get("DirectStreamUrl").and_then(Value::as_str).is_some(),
            "remote source should include DirectStreamUrl fallback"
        );
        assert_eq!(
            source.get("Path"),
            source.get("DirectStreamUrl"),
            "remote source path should use stream endpoint for client playback"
        );

        let stream = source
            .get("MediaStreams")
            .and_then(Value::as_array)
            .and_then(|streams| streams.first())
            .and_then(Value::as_object)
            .expect("media stream object");
        assert_eq!(stream.get("Protocol"), Some(&json!("Http")));
        assert_eq!(stream.get("DisplayLanguage"), Some(&json!("UND")));
        assert_eq!(stream.get("IsTextSubtitleStream"), Some(&json!(false)));
        assert_eq!(stream.get("SupportsExternalStream"), Some(&json!(false)));
        assert!(stream.get("AspectRatio").and_then(Value::as_str).is_some());
    }

    #[test]
    fn build_subtitle_delivery_url_includes_media_source_and_encoded_token() {
        let url = build_subtitle_delivery_url(
            "movie-42",
            "source-a",
            7,
            Some("ass"),
            Some("token+/="),
        );
        assert_eq!(
            url,
            "/Videos/movie-42/source-a/Subtitles/7/Stream.ass?api_key=token%2B%2F%3D"
        );
    }

    #[test]
    fn build_subtitle_delivery_url_falls_back_to_item_route_and_srt() {
        let url = build_subtitle_delivery_url("movie-42", "", 3, Some("bad codec"), None);
        assert_eq!(url, "/Videos/movie-42/Subtitles/3/Stream.srt");
    }

    #[test]
    fn apply_item_external_subtitle_delivery_urls_updates_detail_payload_streams() {
        let mut payload = json!({
            "Id": "movie-42",
            "MediaSources": [
                {
                    "Id": "source-subtitles",
                    "MediaStreams": [
                        {
                            "Index": 7,
                            "Type": "Subtitle",
                            "IsExternal": true,
                            "Codec": "ass"
                        }
                    ]
                }
            ],
            "MediaStreams": [
                {
                    "Index": 7,
                    "Type": "Subtitle",
                    "IsExternal": true,
                    "Codec": "ass"
                }
            ]
        });

        apply_item_external_subtitle_delivery_urls(&mut payload, Some("token-sub"), Some("movie-42"));

        assert_eq!(
            payload["MediaSources"][0]["MediaStreams"][0].get("DeliveryUrl"),
            Some(&json!("/Videos/movie-42/source-subtitles/Subtitles/7/Stream.ass?api_key=token-sub"))
        );
        assert_eq!(
            payload["MediaStreams"][0].get("DeliveryUrl"),
            Some(&json!("/Videos/movie-42/source-subtitles/Subtitles/7/Stream.ass?api_key=token-sub"))
        );
    }

    #[test]
    fn ensure_playback_info_compat_defaults_marks_file_sources_as_local() {
        let mut payload = json!({
            "MediaSources": [
                {
                    "Id": "source-local",
                    "Path": "/mnt/media/movies/source-local.mkv",
                    "Protocol": "File",
                    "MediaStreams": []
                }
            ],
            "PlaySessionId": "ps-local"
        });

        ensure_playback_info_compat_defaults(&mut payload, None, None);
        let source = payload["MediaSources"][0]
            .as_object()
            .expect("media source object");
        assert_eq!(source.get("IsRemote"), Some(&json!(false)));
        assert_eq!(source.get("RunTimeTicks"), Some(&json!(0)));
        assert_eq!(source.get("Bitrate"), Some(&json!(0)));
    }

    #[test]
    fn ensure_playback_info_compat_defaults_marks_embedded_subtitles_clearly() {
        let mut payload = json!({
            "MediaSources": [
                {
                    "Id": "source-subtitles",
                    "Path": "/mnt/media/movies/source-subtitles.mkv",
                    "Protocol": "File",
                    "MediaStreams": [
                        {
                            "Index": 5,
                            "Type": "Subtitle",
                            "IsExternal": false,
                            "Codec": "hdmv_pgs_subtitle",
                            "Language": "jpn"
                        },
                        {
                            "Index": 6,
                            "Type": "Subtitle",
                            "IsExternal": false,
                            "Codec": "srt",
                            "Language": "eng"
                        },
                        {
                            "Index": 7,
                            "Type": "Subtitle",
                            "IsExternal": true,
                            "Codec": "ass",
                            "Language": "chi"
                        }
                    ]
                }
            ],
            "PlaySessionId": "ps-subtitle"
        });

        ensure_playback_info_compat_defaults(&mut payload, Some("token-sub"), Some("movie-42"));
        let streams = payload["MediaSources"][0]["MediaStreams"]
            .as_array()
            .expect("media streams");

        assert_eq!(
            streams[0].get("IsTextSubtitleStream"),
            Some(&json!(false))
        );
        assert_eq!(streams[0].get("DeliveryMethod"), Some(&json!("Embed")));
        assert_eq!(
            streams[0].get("SubtitleLocationType"),
            Some(&json!("InternalStream"))
        );
        assert!(streams[0].get("DeliveryUrl").is_none());
        assert_eq!(streams[0].get("SupportsExternalStream"), Some(&json!(false)));

        assert_eq!(streams[1].get("IsTextSubtitleStream"), Some(&json!(true)));
        assert_eq!(streams[1].get("DeliveryMethod"), Some(&json!("Embed")));
        assert_eq!(
            streams[1].get("SubtitleLocationType"),
            Some(&json!("InternalStream"))
        );
        assert!(streams[1].get("DeliveryUrl").is_none());
        assert_eq!(streams[1].get("SupportsExternalStream"), Some(&json!(false)));

        assert_eq!(streams[2].get("IsTextSubtitleStream"), Some(&json!(true)));
        assert_eq!(streams[2].get("DeliveryMethod"), Some(&json!("External")));
        assert_eq!(
            streams[2].get("SubtitleLocationType"),
            Some(&json!("ExternalFile"))
        );
        assert_eq!(streams[2].get("SupportsExternalStream"), Some(&json!(true)));
        assert_eq!(
            streams[2].get("DeliveryUrl"),
            Some(&json!(
                "/Videos/movie-42/source-subtitles/Subtitles/7/Stream.ass?api_key=token-sub"
            ))
        );
    }

    #[test]
    fn ensure_playback_info_compat_defaults_does_not_inject_placeholder_stream() {
        let mut payload = json!({
            "MediaSources": [
                {
                    "Id": "source-empty-streams",
                    "Path": "https://cdn.example.com/video/source-empty-streams.m3u8",
                    "Protocol": "Http",
                    "MediaStreams": []
                }
            ],
            "PlaySessionId": "ps-empty-streams"
        });

        ensure_playback_info_compat_defaults(&mut payload, None, None);
        let streams = payload["MediaSources"][0]["MediaStreams"]
            .as_array()
            .expect("media streams array");
        assert!(
            streams.is_empty(),
            "playback defaults should not fabricate placeholder streams"
        );
    }

    #[test]
    fn ensure_playback_info_compat_defaults_marks_strm_sources_as_remote() {
        let mut payload = json!({
            "MediaSources": [
                {
                    "Id": "source-strm",
                    "Path": "/mnt/media/movies/movie.strm",
                    "Protocol": "Http",
                    "SupportsDirectPlay": false,
                    "SupportsDirectStream": false,
                    "MediaStreams": []
                }
            ],
            "PlaySessionId": "ps-strm"
        });

        ensure_playback_info_compat_defaults(&mut payload, None, None);
        let source = payload["MediaSources"][0]
            .as_object()
            .expect("media source object");
        assert_eq!(source.get("IsRemote"), Some(&json!(true)));
        assert_eq!(source.get("SupportsDirectPlay"), Some(&json!(false)));
        assert_eq!(source.get("SupportsDirectStream"), Some(&json!(false)));
        let stream_url = source
            .get("DirectStreamUrl")
            .and_then(Value::as_str)
            .expect("DirectStreamUrl for strm source");
        assert!(stream_url.contains("/Videos/source-strm/stream"));
    }

    #[test]
    fn ensure_playback_info_compat_defaults_prefers_item_id_for_direct_stream_url() {
        let mut payload = json!({
            "MediaSources": [
                {
                    "Id": "source-alt",
                    "ItemId": "item-main",
                    "Path": "/mnt/media/movies/movie.strm",
                    "Protocol": "Http",
                    "MediaStreams": []
                }
            ],
            "PlaySessionId": "ps-alt"
        });

        ensure_playback_info_compat_defaults(&mut payload, None, None);
        let source = payload["MediaSources"][0]
            .as_object()
            .expect("media source object");
        let stream_url = source
            .get("DirectStreamUrl")
            .and_then(Value::as_str)
            .expect("DirectStreamUrl for strm source");
        assert!(stream_url.contains("/Videos/item-main/stream"));
        assert!(stream_url.contains("MediaSourceId=source-alt"));
    }

    #[test]
    fn ensure_playback_info_compat_defaults_uses_default_item_id_for_stream_url() {
        let mut payload = json!({
            "MediaSources": [
                {
                    "Id": "source-uuid",
                    "Path": "/mnt/media/movies/movie.strm",
                    "Protocol": "Http",
                    "MediaStreams": []
                }
            ],
            "PlaySessionId": "ps-default-item-id"
        });

        ensure_playback_info_compat_defaults(&mut payload, None, Some("125"));
        let source = payload["MediaSources"][0]
            .as_object()
            .expect("media source object");
        assert_eq!(source.get("ItemId"), Some(&json!("125")));
        let stream_url = source
            .get("DirectStreamUrl")
            .and_then(Value::as_str)
            .expect("DirectStreamUrl for strm source");
        assert!(stream_url.contains("/Videos/125/stream"));
        assert!(stream_url.contains("MediaSourceId=source-uuid"));
    }

    #[test]
    fn ensure_playback_info_compat_defaults_normalizes_chapter_shape() {
        let mut payload = json!({
            "MediaSources": [
                {
                    "Id": "source-chapters",
                    "Protocol": "Http",
                    "Chapters": [
                        {
                            "id": 0,
                            "start_time": "0.0",
                            "tags": { "title": "Intro" }
                        },
                        {
                            "id": 1,
                            "start_time": "12.5"
                        }
                    ],
                    "MediaStreams": []
                }
            ],
            "PlaySessionId": "ps-chapters"
        });

        ensure_playback_info_compat_defaults(&mut payload, None, None);
        let chapters = payload["MediaSources"][0]["Chapters"]
            .as_array()
            .expect("playback chapters");
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].get("ChapterIndex"), Some(&json!(0)));
        assert_eq!(chapters[0].get("StartPositionTicks"), Some(&json!(0)));
        assert_eq!(chapters[0].get("Name"), Some(&json!("Intro")));
        assert_eq!(chapters[0].get("MarkerType"), Some(&json!("Chapter")));
        assert_eq!(chapters[1].get("ChapterIndex"), Some(&json!(1)));
        assert_eq!(chapters[1].get("StartPositionTicks"), Some(&json!(125_000_000)));
        assert_eq!(chapters[1].get("Name"), Some(&json!("Chapter 2")));
    }

    #[test]
    fn media_source_is_remote_for_redirect_respects_explicit_flag() {
        let mut source_obj = serde_json::Map::new();
        source_obj.insert("IsRemote".to_string(), Value::Bool(true));
        source_obj.insert(
            "Path".to_string(),
            Value::String("/mnt/media/movie.mkv".to_string()),
        );
        assert!(media_source_is_remote_for_redirect(&source_obj));

        source_obj.insert("IsRemote".to_string(), Value::Bool(false));
        source_obj.insert(
            "Path".to_string(),
            Value::String("https://cdn.example.com/movie.m3u8".to_string()),
        );
        assert!(!media_source_is_remote_for_redirect(&source_obj));
    }

    #[test]
    fn media_source_is_remote_for_redirect_infers_from_protocol_or_path() {
        let source_obj = json!({
            "Protocol": "Http",
            "Path": "/mnt/media/movie.mkv"
        })
        .as_object()
        .cloned()
        .expect("source object");
        assert!(media_source_is_remote_for_redirect(&source_obj));

        let source_obj = json!({
            "Protocol": "File",
            "Path": "/mnt/media/movie.strm"
        })
        .as_object()
        .cloned()
        .expect("source object");
        assert!(media_source_is_remote_for_redirect(&source_obj));

        let source_obj = json!({
            "Protocol": "File",
            "Path": "/mnt/media/movie.mkv"
        })
        .as_object()
        .cloned()
        .expect("source object");
        assert!(!media_source_is_remote_for_redirect(&source_obj));
    }

    #[test]
    fn build_playback_original_stream_url_matches_emby_shape() {
        let url = build_playback_original_stream_url(
            "16644",
            "mkv",
            Some("019c9637-0967-7df0-ac14-5bbe0278fea2"),
            Some("ps-123"),
            "token-abc",
            Some("device-xyz"),
        );
        assert_eq!(
            url,
            "/videos/16644/original.mkv?DeviceId=device-xyz&MediaSourceId=019c9637-0967-7df0-ac14-5bbe0278fea2&PlaySessionId=ps-123&api_key=token-abc"
        );
    }

    #[test]
    fn apply_playback_original_stream_target_overrides_stream_fields() {
        let mut source_obj = json!({
            "DirectStreamUrl": "/Videos/1/stream?Static=true",
            "Path": "/Videos/1/stream?Static=true",
            "AddApiKeyToDirectStreamUrl": true,
            "IsRemote": true,
            "Protocol": "Http",
            "MediaStreams": [
                { "Type": "Video", "Protocol": "Http", "Codec": "h264" },
                { "Type": "Audio", "Protocol": "Http", "Codec": "aac" }
            ]
        })
        .as_object()
        .cloned()
        .expect("source object");

        apply_playback_original_stream_target(
            &mut source_obj,
            "/videos/16644/original.mkv?MediaSourceId=ms-1&api_key=t".to_string(),
        );

        assert_eq!(
            source_obj.get("DirectStreamUrl"),
            Some(&json!("/videos/16644/original.mkv?MediaSourceId=ms-1&api_key=t"))
        );
        assert_eq!(
            source_obj.get("Path"),
            Some(&json!("/videos/16644/original.mkv?MediaSourceId=ms-1&api_key=t"))
        );
        assert_eq!(
            source_obj.get("AddApiKeyToDirectStreamUrl"),
            Some(&json!(false))
        );
        // After rewriting to LumenStream's own endpoint, the source is local from the
        // client's perspective — matches Emby behaviour expected by SenPlayer.
        assert_eq!(source_obj.get("IsRemote"), Some(&json!(false)));
        assert_eq!(source_obj.get("Protocol"), Some(&json!("File")));
        // MediaStreams Protocol must also be synced to "File"
        let streams = source_obj.get("MediaStreams").unwrap().as_array().unwrap();
        for stream in streams {
            assert_eq!(stream.get("Protocol"), Some(&json!("File")));
        }
    }

    #[test]
    fn ensure_playback_info_original_stream_urls_rewrites_remote_sources() {
        let mut payload = json!({
            "PlaySessionId": "ps-test",
            "MediaSources": [
                {
                    "Id": "ms-remote",
                    "Container": "mkv",
                    "IsRemote": true,
                    "Path": "/Videos/uuid/stream?Static=true&MediaSourceId=ms-remote",
                    "DirectStreamUrl": "/Videos/uuid/stream?Static=true&MediaSourceId=ms-remote"
                },
                {
                    "Id": "ms-local",
                    "Container": "mkv",
                    "IsRemote": false,
                    "Path": "/mnt/media/movie.mkv"
                }
            ]
        });

        ensure_playback_info_original_stream_urls(
            &mut payload,
            "16644",
            Some("token-123"),
            Some("device-456"),
        );

        let media_sources = payload["MediaSources"].as_array().expect("media sources");
        let remote = media_sources[0].as_object().expect("remote source");
        assert_eq!(
            remote.get("Path"),
            Some(&json!(
                "/videos/16644/original.mkv?DeviceId=device-456&MediaSourceId=ms-remote&PlaySessionId=ps-test&api_key=token-123"
            ))
        );
        assert_eq!(
            remote.get("DirectStreamUrl"),
            Some(&json!(
                "/videos/16644/original.mkv?DeviceId=device-456&MediaSourceId=ms-remote&PlaySessionId=ps-test&api_key=token-123"
            ))
        );
        // After rewriting to LumenStream endpoint, present as local to the client
        assert_eq!(remote.get("IsRemote"), Some(&json!(false)));
        assert_eq!(remote.get("Protocol"), Some(&json!("File")));

        let local = media_sources[1].as_object().expect("local source");
        assert_eq!(local.get("Path"), Some(&json!("/mnt/media/movie.mkv")));
        // Local source should remain unchanged
        assert_eq!(local.get("IsRemote"), Some(&json!(false)));
    }

    #[test]
    fn stream_video_compat_query_accepts_common_jellyfin_transcode_keys() {
        let query: StreamVideoCompatQuery = serde_json::from_value(json!({
            "MediaSourceId": "ms1",
            "transcodingContainer": "ts",
            "transcodingProtocol": "hls",
            "videoCodec": "h264",
            "audioCodec": "aac",
            "subtitleCodec": "ass",
            "segmentContainer": "ts",
            "minSegments": "2",
            "breakOnNonKeyFrames": "true",
            "liveStreamId": "live-1",
            "api_key": "token-123"
        }))
        .expect("stream query parse");

        assert_eq!(query.media_source_id.as_deref(), Some("ms1"));
        assert_eq!(query._transcoding_container.as_deref(), Some("ts"));
        assert_eq!(query._transcoding_protocol.as_deref(), Some("hls"));
        assert_eq!(query._video_codec.as_deref(), Some("h264"));
        assert_eq!(query._audio_codec.as_deref(), Some("aac"));
        assert_eq!(query._subtitle_codec.as_deref(), Some("ass"));
        assert_eq!(query._segment_container.as_deref(), Some("ts"));
        assert_eq!(query._min_segments.as_deref(), Some("2"));
        assert_eq!(query._break_on_non_key_frames.as_deref(), Some("true"));
        assert_eq!(query._live_stream_id.as_deref(), Some("live-1"));
        assert_eq!(query._api_key.as_deref(), Some("token-123"));
    }

    #[test]
    fn router_registers_head_routes_for_video_stream_endpoints() {
        let router_source = include_str!("router.rs");
        assert!(router_source.contains(
            ".route(\"/Videos/{item_id}/stream\", web::head().to(stream_video))"
        ));
        assert!(router_source.contains(
            ".route(\n            \"/Videos/{item_id}/stream.{container}\",\n            web::head().to(stream_video_with_container),\n        )"
        ));
    }

    #[test]
    fn router_registers_hide_from_resume_route() {
        let router_source = include_str!("router.rs");
        assert!(router_source.contains("/Users/{user_id}/Items/{item_id}/HideFromResume"));
        assert!(router_source.contains("web::post().to(post_user_item_hide_from_resume)"));
    }

    #[test]
    fn router_registers_played_items_legacy_delete_route() {
        let router_source = include_str!("router.rs");
        assert!(router_source.contains("/Users/{user_id}/PlayedItems/{item_id}/{action}"));
        assert!(router_source.contains("web::post().to(post_item_played_legacy_action)"));
    }

    #[test]
    fn normalize_media_source_id_candidate_prefers_resolved_uuid() {
        let resolved = Uuid::new_v4();
        let normalized =
            normalize_media_source_id_candidate(Some("32549"), Some(resolved)).expect("id");
        assert_eq!(normalized, resolved.to_string());
    }

    #[test]
    fn normalize_media_source_id_candidate_keeps_trimmed_raw_id_when_unresolved() {
        let normalized =
            normalize_media_source_id_candidate(Some("  mediasource_123913  "), None).expect("id");
        assert_eq!(normalized, "mediasource_123913");
    }

    #[test]
    fn normalize_media_source_id_candidate_rejects_empty_values() {
        assert_eq!(
            normalize_media_source_id_candidate(Some("   "), None),
            None
        );
        assert_eq!(normalize_media_source_id_candidate(None, None), None);
    }

    #[test]
    fn subtitle_stream_compat_query_accepts_media_source_and_timing_keys() {
        let query: SubtitleStreamCompatQuery = serde_json::from_value(json!({
            "MediaSourceId": "ms-sub-1",
            "deviceId": "device-1",
            "playSessionId": "play-1",
            "tag": "subtitle-etag",
            "copyTimestamps": "true",
            "addVttTimeMap": "1",
            "startPositionTicks": "1200",
            "endPositionTicks": "4200",
            "playbackStartTimeTicks": "800",
            "format": "vtt",
            "api_key": "token-xyz"
        }))
        .expect("subtitle query parse");

        assert_eq!(query._media_source_id.as_deref(), Some("ms-sub-1"));
        assert_eq!(query._device_id.as_deref(), Some("device-1"));
        assert_eq!(query._play_session_id.as_deref(), Some("play-1"));
        assert_eq!(query._tag.as_deref(), Some("subtitle-etag"));
        assert_eq!(query._copy_timestamps.as_deref(), Some("true"));
        assert_eq!(query._add_vtt_time_map.as_deref(), Some("1"));
        assert_eq!(query._start_position_ticks.as_deref(), Some("1200"));
        assert_eq!(query._end_position_ticks.as_deref(), Some("4200"));
        assert_eq!(query._playback_start_time_ticks.as_deref(), Some("800"));
        assert_eq!(query._format.as_deref(), Some("vtt"));
        assert_eq!(query._api_key.as_deref(), Some("token-xyz"));
    }

    #[test]
    fn playback_progress_payload_accepts_camel_and_playback_aliases() {
        let item_id = Uuid::new_v4();
        let payload: PlaybackProgressDto = serde_json::from_value(json!({
            "playSessionId": "ps-camel",
            "itemId": item_id.to_string(),
            "playbackPositionTicks": 4567,
            "playMethod": "DirectStream",
            "deviceName": "Mac",
            "client": "VidHub"
        }))
        .expect("playback payload parse");

        assert_eq!(payload.play_session_id.as_deref(), Some("ps-camel"));
        assert_eq!(payload.item_id, Some(item_id.to_string()));
        assert_eq!(payload.position_ticks, Some(4567));
        assert_eq!(payload.play_method.as_deref(), Some("DirectStream"));
        assert_eq!(payload.device_name.as_deref(), Some("Mac"));
        assert_eq!(payload.client.as_deref(), Some("VidHub"));
    }

    #[test]
    fn image_compat_query_supports_processing_params_and_tag_header() {
        let query: ImageRequestCompatQuery = serde_json::from_value(json!({
            "maxWidth": "1280",
            "maxHeight": "720",
            "quality": "90",
            "format": "jpg",
            "percentPlayed": "80",
            "blur": "12",
            "tag": "cover-abc"
        }))
        .expect("image query parse");

        assert_eq!(query._max_width.as_deref(), Some("1280"));
        assert_eq!(query._max_height.as_deref(), Some("720"));
        assert_eq!(query._quality.as_deref(), Some("90"));
        assert_eq!(query._format.as_deref(), Some("jpg"));
        assert_eq!(query._percent_played.as_deref(), Some("80"));
        assert_eq!(query._blur.as_deref(), Some("12"));
        assert_eq!(query.tag.as_deref(), Some("cover-abc"));

        let etag = image_tag_header_value(query.tag.as_deref()).expect("etag value");
        assert_eq!(etag.to_str().ok(), Some("\"cover-abc\""));
        assert!(image_tag_header_value(Some("   ")).is_none());
    }

    #[test]
    fn image_resize_request_normalizes_query_values() {
        let query: ImageRequestCompatQuery = serde_json::from_value(json!({
            "Width": "2560",
            "Height": "0",
            "MaxWidth": "1024",
            "quality": "150",
            "format": "JPEG",
            "blur": "42",
            "backgroundColor": "#Ff33AA"
        }))
        .expect("image query parse");

        let resize = query.resize_request().expect("resize request");
        assert_eq!(resize.width, Some(2560));
        assert_eq!(resize.height, None);
        assert_eq!(resize.max_width, Some(1024));
        assert_eq!(resize.max_height, None);
        assert_eq!(resize.quality, Some(100));
        assert_eq!(resize.format, Some(ImageResizeFormat::Jpeg));
        assert_eq!(resize.blur, Some(42));
        assert_eq!(resize.background_color.as_deref(), Some("ff33aa"));
    }

    #[test]
    fn image_resize_request_ignores_invalid_processing_values() {
        let query: ImageRequestCompatQuery = serde_json::from_value(json!({
            "width": "-1",
            "height": "abc",
            "quality": "",
            "format": "tiff",
            "blur": "",
            "backgroundColor": "not-a-color"
        }))
        .expect("image query parse");

        assert!(query.resize_request().is_none());
    }

    #[test]
    fn library_image_upload_helpers_accept_common_types_and_content_types() {
        assert!(is_supported_library_image_type("Primary"));
        assert!(is_supported_library_image_type("backdrop"));
        assert!(is_supported_library_image_type("thumbnail"));
        assert!(!is_supported_library_image_type("Disc"));

        assert_eq!(
            infer_image_extension_from_content_type(Some("image/jpeg")),
            "jpg"
        );
        assert_eq!(
            infer_image_extension_from_content_type(Some("image/png")),
            "png"
        );
        assert_eq!(
            infer_image_extension_from_content_type(Some("image/webp")),
            "webp"
        );
        assert_eq!(
            infer_image_extension_from_content_type(Some("application/octet-stream")),
            "jpg"
        );
    }

    #[test]
    fn show_seasons_query_helper_applies_special_missing_and_adjacent_filters() {
        let season_0_id = Uuid::new_v4().to_string();
        let season_1_id = Uuid::new_v4().to_string();
        let season_2_id = Uuid::new_v4().to_string();
        let make_season = |id: &str, index_number: i32| BaseItemDto {
            id: id.to_string(),
            name: format!("Season {index_number}"),
            item_type: "Season".to_string(),
            path: String::new(),
            is_folder: Some(true),
            media_type: None,
            container: None,
            location_type: None,
            can_delete: Some(false),
            can_download: Some(false),
            collection_type: None,
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
            index_number: Some(index_number),
            parent_index_number: None,
            backdrop_image_tags: None,
            official_rating: None,
            community_rating: None,
            studios: None,
            people: None,
            sort_name: None,
            primary_image_aspect_ratio: None,
            date_created: None,
            child_count: None,
            recursive_item_count: None,
            play_access: None,
        };

        let special_only = apply_show_seasons_query_compatibility(
            QueryResultDto {
                items: vec![
                    make_season(&season_0_id, 0),
                    make_season(&season_1_id, 1),
                    make_season(&season_2_id, 2),
                ],
                total_record_count: 3,
                start_index: 0,
            },
            &SeasonsQuery {
                user_id: None,
                _fields: None,
                is_special_season: Some(true),
                is_missing: None,
                adjacent_to: None,
                _enable_images: None,
                _image_type_limit: None,
                _enable_image_types: None,
                _enable_user_data: None,
                enable_total_record_count: None,
            },
            None,
        );
        assert_eq!(special_only.items.len(), 1);
        assert_eq!(special_only.items[0].index_number, Some(0));

        let adjacent = apply_show_seasons_query_compatibility(
            QueryResultDto {
                items: vec![
                    make_season(&season_0_id, 0),
                    make_season(&season_1_id, 1),
                    make_season(&season_2_id, 2),
                ],
                total_record_count: 3,
                start_index: 0,
            },
            &SeasonsQuery {
                user_id: None,
                _fields: None,
                is_special_season: None,
                is_missing: None,
                adjacent_to: Some(season_1_id.clone()),
                _enable_images: None,
                _image_type_limit: None,
                _enable_image_types: None,
                _enable_user_data: None,
                enable_total_record_count: None,
            },
            Uuid::parse_str(&season_1_id).ok(),
        );
        assert_eq!(adjacent.items.len(), 3);

        let missing_only = apply_show_seasons_query_compatibility(
            QueryResultDto {
                items: vec![
                    make_season(&season_0_id, 0),
                    make_season(&season_1_id, 1),
                    make_season(&season_2_id, 2),
                ],
                total_record_count: 3,
                start_index: 0,
            },
            &SeasonsQuery {
                user_id: None,
                _fields: None,
                is_special_season: None,
                is_missing: Some(true),
                adjacent_to: None,
                _enable_images: None,
                _image_type_limit: None,
                _enable_image_types: None,
                _enable_user_data: None,
                enable_total_record_count: None,
            },
            None,
        );
        assert!(missing_only.items.is_empty());
        assert_eq!(missing_only.total_record_count, 0);

        let no_total = apply_show_seasons_query_compatibility(
            QueryResultDto {
                items: vec![make_season(&season_1_id, 1)],
                total_record_count: 1,
                start_index: 0,
            },
            &SeasonsQuery {
                user_id: None,
                _fields: None,
                is_special_season: None,
                is_missing: None,
                adjacent_to: None,
                _enable_images: None,
                _image_type_limit: None,
                _enable_image_types: None,
                _enable_user_data: None,
                enable_total_record_count: Some(false),
            },
            None,
        );
        assert_eq!(no_total.total_record_count, 0);
    }

    #[test]
    fn show_episodes_query_compatibility_applies_missing_start_and_adjacent() {
        let first_id = Uuid::new_v4();
        let second_id = Uuid::new_v4();
        let third_id = Uuid::new_v4();
        let make_items = || {
            let mut first = make_test_item(&first_id.to_string(), "Ep1", "Episode");
            first.index_number = Some(1);
            let mut second = make_test_item(&second_id.to_string(), "Ep2", "Episode");
            second.index_number = Some(2);
            let mut third = make_test_item(&third_id.to_string(), "Ep3", "Episode");
            third.index_number = Some(3);
            vec![first, second, third]
        };

        let missing = apply_show_episodes_query_compatibility(
            make_items(),
            &ShowEpisodesQuery {
                user_id: None,
                _fields: None,
                season: None,
                season_id: None,
                is_missing: Some(true),
                adjacent_to: None,
                start_item_id: None,
                start_index: None,
                limit: None,
                enable_total_record_count: None,
                _enable_images: None,
                _image_type_limit: None,
                _enable_image_types: None,
                _enable_user_data: None,
                sort_by: None,
                sort_order: None,
            },
            None,
            None,
        );
        assert!(missing.is_empty());

        let from_second = apply_show_episodes_query_compatibility(
            make_items(),
            &ShowEpisodesQuery {
                user_id: None,
                _fields: None,
                season: None,
                season_id: None,
                is_missing: None,
                adjacent_to: None,
                start_item_id: Some(second_id.to_string()),
                start_index: None,
                limit: None,
                enable_total_record_count: None,
                _enable_images: None,
                _image_type_limit: None,
                _enable_image_types: None,
                _enable_user_data: None,
                sort_by: None,
                sort_order: None,
            },
            Some(second_id),
            None,
        );
        assert_eq!(from_second.len(), 2);
        assert_eq!(from_second[0].id, second_id.to_string());

        let adjacent = apply_show_episodes_query_compatibility(
            make_items(),
            &ShowEpisodesQuery {
                user_id: None,
                _fields: None,
                season: None,
                season_id: None,
                is_missing: None,
                adjacent_to: Some(second_id.to_string()),
                start_item_id: None,
                start_index: None,
                limit: None,
                enable_total_record_count: None,
                _enable_images: None,
                _image_type_limit: None,
                _enable_image_types: None,
                _enable_user_data: None,
                sort_by: None,
                sort_order: None,
            },
            None,
            Some(second_id),
        );
        assert_eq!(adjacent.len(), 3);
    }

    #[test]
    fn next_up_query_compatibility_filters_parent_media_and_first_episode() {
        let parent_id = Uuid::new_v4();
        let mut first = make_test_item(&Uuid::new_v4().to_string(), "Ep1", "Episode");
        first.index_number = Some(1);
        first.parent_id = Some(parent_id.to_string());
        let mut second = make_test_item(&Uuid::new_v4().to_string(), "Ep2", "Episode");
        second.index_number = Some(2);
        second.parent_id = Some(parent_id.to_string());
        let movie = make_test_item(&Uuid::new_v4().to_string(), "Movie", "Movie");

        let filtered = apply_next_up_query_compatibility(
            vec![first, second, movie],
            &NextUpQuery {
                user_id: None,
                start_index: None,
                limit: None,
                _fields: None,
                series_id: None,
                parent_id: Some(parent_id.to_string()),
                _enable_images: None,
                _image_type_limit: None,
                _enable_image_types: None,
                _enable_user_data: None,
                _next_up_date_cutoff: None,
                enable_total_record_count: None,
                enable_resumable: None,
                enable_rewatching: None,
                media_types: Some("Video".to_string()),
                disable_first_episode: Some(true),
            },
            Some(parent_id),
        );
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].index_number, Some(2));
    }

    #[test]
    fn update_user_password_parses_legacy_and_new_field_names() {
        let legacy: UpdateUserPassword = serde_json::from_value(json!({
            "CurrentPw": "old-pass",
            "NewPw": "new-pass-123"
        }))
        .expect("legacy payload parse");
        assert_eq!(legacy.current_pw.as_deref(), Some("old-pass"));
        assert_eq!(legacy.new_pw.as_deref(), Some("new-pass-123"));
        assert_eq!(legacy.reset_password, None);

        let modern: UpdateUserPassword = serde_json::from_value(json!({
            "CurrentPassword": "old-pass",
            "NewPassword": "new-pass-456",
            "ResetPassword": true
        }))
        .expect("modern payload parse");
        assert_eq!(modern.current_pw.as_deref(), Some("old-pass"));
        assert_eq!(modern.new_pw.as_deref(), Some("new-pass-456"));
        assert_eq!(modern.reset_password, Some(true));

        let camel: UpdateUserPassword = serde_json::from_value(json!({
            "currentPw": "old-pass",
            "newPassword": "new-pass-789",
            "resetPassword": "1"
        }))
        .expect("camel payload parse");
        assert_eq!(camel.current_pw.as_deref(), Some("old-pass"));
        assert_eq!(camel.new_pw.as_deref(), Some("new-pass-789"));
        assert_eq!(camel.reset_password, Some(true));
    }

    #[test]
    fn resolve_new_password_supports_reset_for_admin_only() {
        let with_new = UpdateUserPassword {
            current_pw: None,
            new_pw: Some("hello-123".to_string()),
            reset_password: None,
        };
        assert_eq!(
            resolve_new_password(&with_new, false, false).expect("new password"),
            "hello-123".to_string()
        );

        let admin_reset = UpdateUserPassword {
            current_pw: None,
            new_pw: None,
            reset_password: Some(true),
        };
        let generated = resolve_new_password(&admin_reset, true, true).expect("generated password");
        assert!(generated.len() >= 32);

        let non_admin_reset = resolve_new_password(&admin_reset, true, false);
        assert!(non_admin_reset.is_err());
    }

    #[test]
    fn paginate_resume_items_applies_start_limit_and_total_count_toggle() {
        let paged = paginate_resume_items(vec![1, 2, 3, 4], 1, 2, true);
        assert_eq!(paged.items, vec![2, 3]);
        assert_eq!(paged.start_index, 1);
        assert_eq!(paged.total_record_count, 4);

        let paged_without_total = paginate_resume_items(vec![1, 2, 3], 0, 2, false);
        assert_eq!(paged_without_total.items, vec![1, 2]);
        assert_eq!(paged_without_total.start_index, 0);
        assert_eq!(paged_without_total.total_record_count, 0);
    }

    #[test]
    fn show_episodes_query_helpers_clamp_values() {
        assert_eq!(normalize_show_episodes_limit(None), 500);
        assert_eq!(normalize_show_episodes_limit(Some(0)), 1);
        assert_eq!(normalize_show_episodes_limit(Some(900)), 500);

        assert_eq!(normalize_show_episodes_start_index(None), 0);
        assert_eq!(normalize_show_episodes_start_index(Some(-7)), 0);
        assert_eq!(normalize_show_episodes_start_index(Some(9)), 9);
    }

    #[test]
    fn filter_show_episodes_supports_season_number() {
        let make_item = |id: &str, parent_index_number: Option<i32>| {
            BaseItemDto {
                id: id.to_string(),
                name: id.to_string(),
                item_type: "Episode".to_string(),
                path: format!("/tmp/{id}.mkv"),
                is_folder: Some(false),
                media_type: Some("Video".to_string()),
                container: None,
                location_type: Some("FileSystem".to_string()),
                can_delete: Some(false),
                can_download: Some(true),
                collection_type: None,
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
                parent_index_number,
                backdrop_image_tags: None,
                official_rating: None,
                community_rating: None,
                studios: None,
                people: None,
                sort_name: None,
                primary_image_aspect_ratio: None,
                date_created: None,
                child_count: None,
                recursive_item_count: None,
                play_access: None,
            }
        };

        let by_season_number = filter_show_episodes(
            vec![
                make_item("ep-1", Some(1)),
                make_item("ep-2", Some(2)),
                make_item("ep-3", Some(1)),
            ],
            Some(1),
        );
        assert_eq!(
            by_season_number
                .iter()
                .map(|v| v.id.as_str())
                .collect::<Vec<_>>(),
            vec!["ep-1", "ep-3"]
        );
    }

    #[test]
    fn parse_top_played_stat_date_supports_default_and_iso_date() {
        let explicit = parse_top_played_stat_date(Some("2026-02-11")).expect("explicit date");
        assert_eq!(explicit.to_string(), "2026-02-11");

        let fallback = parse_top_played_stat_date(None).expect("fallback date");
        assert!(fallback.to_string().len() == 10);

        let invalid = parse_top_played_stat_date(Some("11-02-2026"));
        assert!(invalid.is_err());
    }

    #[test]
    fn admin_billing_config_response_masks_epay_key() {
        let cfg = BillingConfig {
            enabled: true,
            min_recharge_amount: rust_decimal::Decimal::new(100, 2),
            max_recharge_amount: rust_decimal::Decimal::new(200000, 2),
            order_expire_minutes: 30,
            channels: vec!["alipay".to_string(), "wxpay".to_string()],
            epay: EpayConfig {
                gateway_url: "https://epay.example.com".to_string(),
                pid: "10001".to_string(),
                key: "secret-key".to_string(),
                notify_url: "https://lumenstream.example.com/billing/epay/notify".to_string(),
                return_url: "https://lumenstream.example.com/billing/epay/return".to_string(),
                sitename: "LumenStream".to_string(),
            },
        };

        let masked = AdminBillingConfigResponse::from_config(&cfg, true);
        assert!(masked.enabled);
        assert_eq!(masked.epay.key, "***");
        assert_eq!(masked.epay.gateway_url, "https://epay.example.com");
        assert_eq!(masked.epay.pid, "10001");

        let unmasked = AdminBillingConfigResponse::from_config(&cfg, false);
        assert_eq!(unmasked.epay.key, "secret-key");
    }

    #[test]
    fn admin_billing_config_response_does_not_mask_empty_key() {
        let cfg = BillingConfig {
            enabled: false,
            min_recharge_amount: rust_decimal::Decimal::new(100, 2),
            max_recharge_amount: rust_decimal::Decimal::new(200000, 2),
            order_expire_minutes: 30,
            channels: vec![],
            epay: EpayConfig {
                gateway_url: String::new(),
                pid: String::new(),
                key: String::new(),
                notify_url: String::new(),
                return_url: String::new(),
                sitename: String::new(),
            },
        };

        let masked = AdminBillingConfigResponse::from_config(&cfg, true);
        assert_eq!(masked.epay.key, "");
    }

    #[test]
    fn apply_billing_config_update_merges_partial_fields() {
        let mut cfg = BillingConfig {
            enabled: false,
            min_recharge_amount: rust_decimal::Decimal::new(100, 2),
            max_recharge_amount: rust_decimal::Decimal::new(200000, 2),
            order_expire_minutes: 30,
            channels: vec!["alipay".to_string()],
            epay: EpayConfig {
                gateway_url: "https://old.example.com".to_string(),
                pid: "old-pid".to_string(),
                key: "old-key".to_string(),
                notify_url: "https://old.example.com/notify".to_string(),
                return_url: "https://old.example.com/return".to_string(),
                sitename: "Old Site".to_string(),
            },
        };

        let payload = AdminUpdateBillingConfigRequest {
            enabled: Some(true),
            min_recharge_amount: Some(rust_decimal::Decimal::new(500, 2)),
            max_recharge_amount: None,
            order_expire_minutes: Some(60),
            channels: Some(vec!["alipay".to_string(), "wxpay".to_string()]),
            epay: Some(AdminUpdateEpayConfigRequest {
                gateway_url: Some("https://new.example.com".to_string()),
                pid: None,
                key: Some("new-key".to_string()),
                notify_url: None,
                return_url: None,
                sitename: Some("New Site".to_string()),
            }),
        };

        apply_billing_config_update(&mut cfg, &payload);

        assert!(cfg.enabled);
        assert_eq!(cfg.min_recharge_amount, rust_decimal::Decimal::new(500, 2));
        assert_eq!(
            cfg.max_recharge_amount,
            rust_decimal::Decimal::new(200000, 2)
        );
        assert_eq!(cfg.order_expire_minutes, 60);
        assert_eq!(
            cfg.channels,
            vec!["alipay".to_string(), "wxpay".to_string()]
        );
        assert_eq!(cfg.epay.gateway_url, "https://new.example.com");
        assert_eq!(cfg.epay.pid, "old-pid");
        assert_eq!(cfg.epay.key, "new-key");
        assert_eq!(cfg.epay.notify_url, "https://old.example.com/notify");
        assert_eq!(cfg.epay.sitename, "New Site");
    }

    #[test]
    fn apply_epay_config_update_preserves_key_when_placeholder() {
        let mut cfg = EpayConfig {
            gateway_url: "https://epay.example.com".to_string(),
            pid: "10001".to_string(),
            key: "original-secret".to_string(),
            notify_url: "https://lumenstream.example.com/notify".to_string(),
            return_url: "https://lumenstream.example.com/return".to_string(),
            sitename: "LumenStream".to_string(),
        };

        let payload = AdminUpdateEpayConfigRequest {
            gateway_url: None,
            pid: Some("20002".to_string()),
            key: Some("***".to_string()),
            notify_url: None,
            return_url: None,
            sitename: None,
        };

        apply_epay_config_update(&mut cfg, &payload);

        assert_eq!(cfg.pid, "20002");
        assert_eq!(cfg.key, "original-secret");
    }

    #[test]
    fn items_library_compat_query_parsers_accept_aliases() {
        let del: DeleteItemsQuery = serde_json::from_value(json!({
            "Ids": "a,b,c"
        }))
        .expect("delete items query parse");
        assert_eq!(del.ids.as_deref(), Some("a,b,c"));

        let file_query: ItemFileQueryCompat = serde_json::from_value(json!({
            "itemId": Uuid::new_v4(),
            "path": "/mnt/media/a.strm"
        }))
        .expect("item file query parse");
        assert!(file_query.item_id.is_some());
        assert_eq!(file_query._path.as_deref(), Some("/mnt/media/a.strm"));

        let refresh: LibraryRefreshQuery = serde_json::from_value(json!({
            "libraryId": Uuid::new_v4()
        }))
        .expect("library refresh query parse");
        assert!(refresh.library_id.is_some());
    }

    #[test]
    fn add_virtual_folder_and_image_reorder_query_parse() {
        let payload: AddVirtualFolderRequest = serde_json::from_value(json!({
            "Name": "Movies",
            "CollectionType": "movies",
            "Paths": ["/data/movies"]
        }))
        .expect("add virtual folder payload parse");
        assert_eq!(payload.name.as_deref(), Some("Movies"));
        assert_eq!(payload.collection_type.as_deref(), Some("movies"));
        assert_eq!(
            payload.paths.unwrap_or_default(),
            vec!["/data/movies".to_string()]
        );

        let move_index: MoveItemImageIndexQuery = serde_json::from_value(json!({
            "newIndex": 2
        }))
        .expect("move item image index query parse");
        assert_eq!(move_index._new_index, Some(2));
    }

    #[test]
    fn metadata_patch_from_payload_maps_common_item_fields() {
        let patch = metadata_patch_from_payload(&json!({
            "Overview": "A great movie",
            "PremiereDate": "2024-01-01",
            "ProductionYear": 2024,
            "OfficialRating": "PG-13",
            "CommunityRating": 8.7,
            "SortName": "Movie, Great",
            "PrimaryImageAspectRatio": 1.78,
            "Genres": ["Drama", "Sci-Fi"],
            "ProviderIds": {
                "Tmdb": "123",
                "Imdb": "tt123456"
            }
        }))
        .expect("metadata patch");

        assert_eq!(patch["overview"], "A great movie");
        assert_eq!(patch["premiere_date"], "2024-01-01");
        assert_eq!(patch["production_year"], 2024);
        assert_eq!(patch["official_rating"], "PG-13");
        assert_eq!(patch["community_rating"], 8.7);
        assert_eq!(patch["sort_name"], "Movie, Great");
        assert_eq!(patch["primary_image_aspect_ratio"], 1.78);
        assert_eq!(patch["genres"][0], "Drama");
        assert_eq!(patch["provider_ids"]["Tmdb"], "123");
        assert_eq!(patch["tmdb_id"], 123);
        assert_eq!(patch["imdb_id"], "tt123456");
        assert!(patch.get("tmdb_binding_source").is_none());
    }

    #[test]
    fn metadata_patch_from_payload_prefers_explicit_tmdb_imdb_fields() {
        let patch = metadata_patch_from_payload(&json!({
            "providerIds": {
                "tmdb": "9999",
                "imdb": "tt9999999"
            },
            "tmdbId": "157336",
            "imdbId": "tt0816692"
        }))
        .expect("metadata patch");

        assert_eq!(patch["tmdb_id"], 157336);
        assert_eq!(patch["imdb_id"], "tt0816692");
        assert_eq!(patch["provider_ids"]["Tmdb"], "157336");
        assert_eq!(patch["provider_ids"]["Imdb"], "tt0816692");
        assert_eq!(patch["tmdb_binding_source"], "manual");
    }

    #[test]
    fn remote_search_payload_helpers_extract_search_info() {
        let payload = json!({
            "SearchInfo": {
                "Name": "Interstellar",
                "Year": 2014,
                "ProviderIds": {
                    "Tmdb": "157336"
                }
            }
        });

        assert_eq!(
            remote_search_name_from_payload(&payload).as_deref(),
            Some("Interstellar")
        );
        assert_eq!(remote_search_year_from_payload(&payload), Some(2014));
        let provider_ids =
            remote_search_provider_ids_from_payload(&payload).expect("provider ids");
        assert_eq!(provider_ids.get("Tmdb").map(String::as_str), Some("157336"));
    }

    #[test]
    fn remote_search_include_item_types_supports_season_and_episode() {
        assert_eq!(
            remote_search_include_item_types("season"),
            vec!["Season".to_string()]
        );
        assert_eq!(
            remote_search_include_item_types("episode"),
            vec!["Episode".to_string()]
        );
    }

    #[test]
    fn default_external_id_provider_keys_include_episode_defaults() {
        assert_eq!(
            default_external_id_provider_keys("Episode"),
            ["Tvdb", "Tmdb", "Imdb"]
        );
        assert_eq!(
            default_external_id_provider_keys("Season"),
            ["Tvdb", "Tmdb", "Imdb"]
        );
        assert_eq!(
            default_external_id_provider_keys("Movie"),
            ["Tmdb", "Imdb"]
        );
    }

    #[test]
    fn remote_subtitle_id_and_language_helpers_work_for_local_tracks() {
        assert_eq!(parse_remote_subtitle_track_id("local-3"), Some(3));
        assert_eq!(parse_remote_subtitle_track_id("2"), Some(2));
        assert_eq!(parse_remote_subtitle_track_id(""), None);

        assert!(subtitle_language_matches(Some("en"), "en"));
        assert!(subtitle_language_matches(Some("en"), "eng"));
        assert!(subtitle_language_matches(Some("zh"), "zho"));
        assert!(!subtitle_language_matches(Some("en"), "zho"));
    }

    #[test]
    fn remote_image_type_normalizer_accepts_known_types() {
        assert_eq!(
            normalize_remote_image_type(Some("primary")).as_deref(),
            Some("Primary")
        );
        assert_eq!(
            normalize_remote_image_type(Some("Backdrop")).as_deref(),
            Some("Backdrop")
        );
        assert!(normalize_remote_image_type(Some("UnknownType")).is_none());
    }

    #[test]
    fn normalize_my_traffic_window_days_clamps_to_30_days() {
        assert_eq!(normalize_my_traffic_window_days(None), 30);
        assert_eq!(normalize_my_traffic_window_days(Some(0)), 1);
        assert_eq!(normalize_my_traffic_window_days(Some(7)), 7);
        assert_eq!(normalize_my_traffic_window_days(Some(90)), 30);
    }

    #[test]
    fn my_traffic_usage_media_query_accepts_window_days_alias() {
        let query: MyTrafficUsageMediaQuery = serde_json::from_value(json!({
            "windowDays": 45,
            "limit": 180
        }))
        .expect("query parse");
        assert_eq!(query.window_days, Some(45));
        assert_eq!(query.limit, Some(180));
    }

    #[test]
    fn admin_upsert_playback_domain_request_accepts_binding_and_multiplier() {
        let payload: AdminUpsertPlaybackDomainRequest = serde_json::from_value(json!({
            "name": "线路A",
            "base_url": "https://line-a.example.com",
            "enabled": true,
            "priority": 100,
            "is_default": false,
            "lumenbackend_node_id": "node-a",
            "traffic_multiplier": 1.3
        }))
        .expect("payload parse");

        assert_eq!(payload.lumenbackend_node_id, Some(Some("node-a".to_string())));
        assert_eq!(payload.traffic_multiplier, Some(1.3));
    }

    #[test]
    fn admin_upsert_playback_domain_request_accepts_null_node_binding_payload() {
        let payload: AdminUpsertPlaybackDomainRequest = serde_json::from_value(json!({
            "name": "线路B",
            "base_url": "https://line-b.example.com",
            "lumenbackend_node_id": null
        }))
        .expect("payload parse");

        assert_eq!(payload.lumenbackend_node_id, None);
        assert_eq!(payload.traffic_multiplier, None);
    }

    #[test]
    fn admin_create_lumenbackend_node_request_accepts_enabled_and_name() {
        let payload: AdminCreateLumenBackendNodeRequest = serde_json::from_value(json!({
            "node_id": "node-01",
            "node_name": "节点一",
            "enabled": true
        }))
        .expect("create node payload parse");

        assert_eq!(payload.node_id, "node-01");
        assert_eq!(payload.node_name.as_deref(), Some("节点一"));
        assert_eq!(payload.enabled, Some(true));
    }

    #[test]
    fn admin_patch_lumenbackend_node_request_accepts_nullable_name() {
        let payload: AdminPatchLumenBackendNodeRequest = serde_json::from_value(json!({
            "node_name": null,
            "enabled": false
        }))
        .expect("patch node payload parse");

        assert_eq!(payload.node_name, None);
        assert_eq!(payload.enabled, Some(false));
    }

    #[test]
    fn lumenbackend_register_request_accepts_runtime_schema_payload() {
        let payload: LumenBackendRegisterRequest = serde_json::from_value(json!({
            "node_id": "node-01",
            "node_name": "Node 01",
            "node_version": "0.2.0",
            "runtime_schema_version": "2026.02.1",
            "runtime_schema_hash": "hash-1",
            "runtime_schema": {
                "sections": [
                    {
                        "id": "basic",
                        "title": "Basic",
                        "fields": [
                            { "key": "server.listen_port", "type": "number", "required": true }
                        ]
                    }
                ]
            }
        }))
        .expect("register payload parse");

        assert_eq!(payload.node_id, "node-01");
        assert_eq!(payload.runtime_schema_version.as_deref(), Some("2026.02.1"));
        assert_eq!(payload.runtime_schema_hash.as_deref(), Some("hash-1"));
        assert!(payload.runtime_schema.is_some());
    }

    #[test]
    fn admin_upsert_billing_plan_request_accepts_permission_group_id() {
        let group_id = Uuid::now_v7();
        let payload: AdminUpsertBillingPlanRequest = serde_json::from_value(json!({
            "code": "std",
            "name": "标准套餐",
            "price": "29.00",
            "duration_days": 30,
            "traffic_quota_bytes": 107374182400u64,
            "traffic_window_days": 30,
            "permission_group_id": group_id,
            "enabled": true
        }))
        .expect("billing payload parse");

        assert_eq!(payload.permission_group_id, Some(group_id));
    }

    #[test]
    fn admin_upsert_billing_permission_group_request_accepts_domain_ids() {
        let domain_a = Uuid::now_v7();
        let domain_b = Uuid::now_v7();
        let payload: AdminUpsertBillingPermissionGroupRequest = serde_json::from_value(json!({
            "code": "mainland",
            "name": "大陆组",
            "enabled": true,
            "domain_ids": [domain_a, domain_b]
        }))
        .expect("permission group payload parse");

        assert_eq!(payload.domain_ids, vec![domain_a, domain_b]);
    }

    #[tokio::test]
    async fn serve_image_file_returns_304_when_etag_matches() {
        let dir = std::env::temp_dir().join("ls-test-img-304");
        let _ = tokio::fs::create_dir_all(&dir).await;
        let img_path = dir.join("test.jpg");
        tokio::fs::write(&img_path, b"fake-jpeg-data").await.unwrap();

        let etag = HeaderValue::from_static("\"abc123\"");
        let mut headers = HeaderMap::new();
        headers.insert(header::IF_NONE_MATCH, HeaderValue::from_static("\"abc123\""));

        let resp = serve_image_file(
            img_path.to_str().unwrap(),
            Some(etag),
            &headers,
        )
        .await;

        assert_eq!(resp.status(), StatusCode::NOT_MODIFIED);
        let body = resp.into_body().try_into_bytes().unwrap();
        assert!(body.is_empty());

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn serve_image_file_returns_200_with_content_length() {
        let dir = std::env::temp_dir().join("ls-test-img-200");
        let _ = tokio::fs::create_dir_all(&dir).await;
        let img_path = dir.join("poster.jpg");
        let data = b"fake-jpeg-content-for-test";
        tokio::fs::write(&img_path, data).await.unwrap();

        let headers = HeaderMap::new();
        let resp = serve_image_file(
            img_path.to_str().unwrap(),
            None,
            &headers,
        )
        .await;

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.headers().get(header::CONTENT_TYPE).unwrap(), "image/jpeg");
        let size = resp.body().size();
        assert_eq!(size, actix_web::body::BodySize::Sized(data.len() as u64));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn serve_image_file_skips_304_when_etag_differs() {
        let dir = std::env::temp_dir().join("ls-test-img-nomatch");
        let _ = tokio::fs::create_dir_all(&dir).await;
        let img_path = dir.join("cover.png");
        tokio::fs::write(&img_path, b"png-bytes").await.unwrap();

        let etag = HeaderValue::from_static("\"v2\"");
        let mut headers = HeaderMap::new();
        headers.insert(header::IF_NONE_MATCH, HeaderValue::from_static("\"v1\""));

        let resp = serve_image_file(
            img_path.to_str().unwrap(),
            Some(etag),
            &headers,
        )
        .await;

        assert_eq!(resp.status(), StatusCode::OK);

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    // ── Emby compat playlist/collection struct tests ──

    #[test]
    fn emby_create_playlist_request_pascal_case() {
        let req: EmbyCreatePlaylistRequest = serde_json::from_value(json!({
            "Name": "My Playlist",
            "Ids": "abc,def",
            "UserId": "u1",
            "MediaType": "Video"
        }))
        .expect("deserialize PascalCase");
        assert_eq!(req.name.as_deref(), Some("My Playlist"));
        assert_eq!(req.ids.as_deref(), Some("abc,def"));
    }

    #[test]
    fn emby_create_playlist_request_camel_case() {
        let req: EmbyCreatePlaylistRequest = serde_json::from_value(json!({
            "name": "Camel Playlist",
            "ids": "x,y,z"
        }))
        .expect("deserialize camelCase");
        assert_eq!(req.name.as_deref(), Some("Camel Playlist"));
        assert_eq!(req.ids.as_deref(), Some("x,y,z"));
    }

    #[test]
    fn emby_create_playlist_request_all_optional() {
        let req: EmbyCreatePlaylistRequest =
            serde_json::from_value(json!({})).expect("deserialize empty");
        assert!(req.name.is_none());
        assert!(req.ids.is_none());
    }

    #[test]
    fn emby_playlist_items_query_pascal_case() {
        let q: EmbyPlaylistItemsQuery = serde_json::from_value(json!({
            "Ids": "id1,id2",
            "UserId": "u1"
        }))
        .expect("deserialize PascalCase");
        assert_eq!(q.ids.as_deref(), Some("id1,id2"));
    }

    #[test]
    fn emby_playlist_items_query_camel_case() {
        let q: EmbyPlaylistItemsQuery = serde_json::from_value(json!({
            "ids": "a,b,c",
            "userId": "u2"
        }))
        .expect("deserialize camelCase");
        assert_eq!(q.ids.as_deref(), Some("a,b,c"));
    }

    #[test]
    fn emby_create_collection_query_pascal_case() {
        let q: EmbyCreateCollectionQuery = serde_json::from_value(json!({
            "Name": "My Collection",
            "Ids": "1,2,3",
            "ParentId": "p1",
            "IsLocked": "true"
        }))
        .expect("deserialize PascalCase");
        assert_eq!(q.name.as_deref(), Some("My Collection"));
        assert_eq!(q.ids.as_deref(), Some("1,2,3"));
    }

    #[test]
    fn emby_create_collection_query_camel_case() {
        let q: EmbyCreateCollectionQuery = serde_json::from_value(json!({
            "name": "Camel Collection",
            "ids": "x,y"
        }))
        .expect("deserialize camelCase");
        assert_eq!(q.name.as_deref(), Some("Camel Collection"));
        assert_eq!(q.ids.as_deref(), Some("x,y"));
    }

    #[test]
    fn emby_create_collection_query_all_optional() {
        let q: EmbyCreateCollectionQuery =
            serde_json::from_value(json!({})).expect("deserialize empty");
        assert!(q.name.is_none());
        assert!(q.ids.is_none());
    }

    #[test]
    fn admin_user_profile_payload_masks_commercial_fields_for_ce() {
        let capabilities = ls_config::EditionCapabilities {
            edition: "ce".to_string(),
            billing_enabled: false,
            advanced_traffic_controls_enabled: false,
            invite_rewards_enabled: false,
            audit_log_export_enabled: false,
            request_agent_enabled: true,
            playback_routing_enabled: true,
        };
        let masked = mask_admin_user_manage_profile_payload(
            &capabilities,
            json!({
                "user": { "Id": "u1" },
                "profile": { "user_id": "u1" },
                "stream_policy": { "max_concurrent_streams": 2 },
                "traffic_usage": { "used_bytes": 100 },
                "wallet": { "balance": "1.00" },
                "subscriptions": [{ "id": "sub-1" }],
                "sessions_summary": { "active_auth_sessions": 1 }
            }),
        );

        assert!(masked.get("stream_policy").is_none());
        assert!(masked.get("traffic_usage").is_none());
        assert!(masked.get("wallet").is_none());
        assert!(masked.get("subscriptions").is_none());
        assert!(masked.get("sessions_summary").is_some());
    }

    #[test]
    fn admin_user_summary_payload_masks_used_bytes_and_subscription_for_ce() {
        let capabilities = ls_config::EditionCapabilities {
            edition: "ce".to_string(),
            billing_enabled: false,
            advanced_traffic_controls_enabled: false,
            invite_rewards_enabled: false,
            audit_log_export_enabled: false,
            request_agent_enabled: true,
            playback_routing_enabled: true,
        };
        let masked = mask_admin_user_summary_page_payload(
            &capabilities,
            json!({
                "page": 1,
                "page_size": 20,
                "total": 1,
                "items": [{
                    "id": "u1",
                    "subscription_name": "VIP",
                    "used_bytes": 1024
                }]
            }),
        );

        assert_eq!(masked["items"][0]["subscription_name"], serde_json::Value::Null);
        assert_eq!(masked["items"][0]["used_bytes"], json!(0));
    }

    #[test]
    fn invite_payloads_hide_rewards_when_disabled() {
        let invite_settings = ls_config::InviteConfig {
            force_on_register: true,
            invitee_bonus_enabled: true,
            invitee_bonus_amount: rust_decimal::Decimal::new(500, 2),
            inviter_rebate_enabled: true,
            inviter_rebate_rate: rust_decimal::Decimal::new(1000, 4),
        };

        let settings_payload = invite_settings_payload(false, &invite_settings);
        assert_eq!(settings_payload.get("force_on_register"), Some(&json!(true)));
        assert!(settings_payload.get("invitee_bonus_enabled").is_none());
        assert!(settings_payload.get("inviter_rebate_rate").is_none());

        let invite_summary = ls_infra::InviteSummary {
            code: "ABC123".to_string(),
            enabled: true,
            invited_count: 3,
            rebate_total: rust_decimal::Decimal::new(1234, 2),
            invitee_bonus_enabled: true,
        };
        let summary_payload = invite_summary_payload(false, &invite_summary);
        assert_eq!(summary_payload.get("code"), Some(&json!("ABC123")));
        assert!(summary_payload.get("rebate_total").is_none());
        assert!(summary_payload.get("invitee_bonus_enabled").is_none());
    }

    #[test]
    fn invite_settings_update_ignores_rewards_for_ce() {
        let mut settings = ls_config::InviteConfig {
            force_on_register: false,
            invitee_bonus_enabled: false,
            invitee_bonus_amount: rust_decimal::Decimal::ZERO,
            inviter_rebate_enabled: false,
            inviter_rebate_rate: rust_decimal::Decimal::ZERO,
        };

        apply_invite_settings_update(
            &mut settings,
            &super::AdminInviteSettingsPatchRequest {
                force_on_register: Some(true),
                invitee_bonus_enabled: Some(true),
                invitee_bonus_amount: Some(rust_decimal::Decimal::new(888, 2)),
                inviter_rebate_enabled: Some(true),
                inviter_rebate_rate: Some(rust_decimal::Decimal::new(2500, 4)),
            },
            false,
        );

        assert!(settings.force_on_register);
        assert!(!settings.invitee_bonus_enabled);
        assert_eq!(settings.invitee_bonus_amount, rust_decimal::Decimal::ZERO);
        assert!(!settings.inviter_rebate_enabled);
        assert_eq!(settings.inviter_rebate_rate, rust_decimal::Decimal::ZERO);
    }
}
