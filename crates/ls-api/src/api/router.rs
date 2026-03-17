pub fn build_api_router(state: ApiContext) -> impl HttpServiceFactory {
    let cors = build_cors(&state);
    let max_body = state.infra.config_snapshot().server.max_upload_body_bytes;
    let capabilities = state.infra.config_snapshot().edition_capabilities();

    web::scope("")
        .app_data(web::Data::new(state))
        .app_data(web::PayloadConfig::new(max_body))
        .wrap(cors)
        .wrap(middleware::Logger::default())
        .wrap(middleware::from_fn(request_context_middleware))
        .wrap(middleware::from_fn(strip_compat_prefix))
        .service(build_api_scope(web::scope(""), &capabilities))
}

fn build_api_scope(
    scope: actix_web::Scope,
    capabilities: &ls_config::EditionCapabilities,
) -> actix_web::Scope {
    let scope = scope
        .route("/health", web::get().to(health))
        .route(
            "/api/system/capabilities",
            web::get().to(get_public_system_capabilities),
        )
        .route("/metrics", web::get().to(metrics_snapshot))
        .route("/System/Info/Public", web::get().to(get_system_info_public))
        .route("/System/Info", web::get().to(get_system_info))
        .route("/System/Endpoint", web::get().to(get_system_endpoint))
        .route("/System/Ping", web::get().to(system_ping))
        .route("/System/Ping", web::post().to(system_ping_post))
        .route(
            "/System/ActivityLog/Entries",
            web::get().to(get_system_activity_log_entries),
        )
        .route(
            "/System/Configuration",
            web::get().to(get_system_configuration),
        )
        .route(
            "/System/Configuration",
            web::post().to(post_system_configuration),
        )
        .route(
            "/System/Configuration/{key}",
            web::get().to(get_system_configuration_key),
        )
        .route(
            "/System/Configuration/{key}",
            web::post().to(post_system_configuration_key),
        )
        .route("/System/Logs", web::get().to(get_system_logs))
        .route("/System/Logs/Log", web::get().to(get_system_log_content))
        .route(
            "/System/WakeOnLanInfo",
            web::get().to(get_system_wake_on_lan_info),
        )
        .route("/System/Restart", web::post().to(post_system_restart))
        .route("/System/Shutdown", web::post().to(post_system_shutdown))
        .route(
            "/Branding/Configuration",
            web::get().to(get_branding_configuration),
        )
        .route("/Branding/Css", web::get().to(get_branding_css))
        .route("/Branding/Css.css", web::get().to(get_branding_css))
        .route(
            "/DisplayPreferences/{id}",
            web::get().to(get_display_preferences),
        )
        .route(
            "/DisplayPreferences/{display_preferences_id}",
            web::post().to(post_display_preferences),
        )
        .route("/System/Logs/Level", web::get().to(get_log_level))
        .route("/System/Logs/Level", web::put().to(set_log_level))
        .route("/System/Logs/Config", web::get().to(get_log_config))
        .route("/Users/Me", web::get().to(get_current_user))
        .route(
            "/Users/Me/PlaybackDomains",
            web::get().to(get_me_playback_domains),
        )
        .route(
            "/Users/Me/PlaybackDomains/Select",
            web::post().to(select_me_playback_domain),
        )
        .route("/Users", web::get().to(get_users))
        .route(
            "/Users/AuthenticateByName",
            web::post().to(authenticate_by_name),
        )
        .route("/api/auth/register", web::post().to(register_with_invite))
        .route("/api/requests", web::get().to(list_my_agent_requests))
        .route("/api/requests", web::post().to(create_my_agent_request))
        .route("/api/requests/{request_id}", web::get().to(get_my_agent_request))
        .route(
            "/api/requests/{request_id}/resubmit",
            web::post().to(resubmit_my_agent_request),
        )
        .route("/Users/Public", web::get().to(get_public_users))
        .route("/Users/ForgotPassword", web::post().to(forgot_password))
        .route(
            "/Users/ForgotPassword/Pin",
            web::post().to(forgot_password_pin),
        )
        .route("/Users/New", web::post().to(create_user_jellyfin))
        .route("/Users/{user_id}", web::get().to(get_user_by_id))
        .route("/Users/{user_id}", web::post().to(post_user_by_id))
        .route("/Users/{user_id}", web::delete().to(delete_user_jellyfin))
        .route(
            "/Users/{user_id}/Authenticate",
            web::post().to(authenticate_user_by_id),
        )
        .route(
            "/Users/{user_id}/Connect/Link",
            web::post().to(post_user_connect_link),
        )
        .route(
            "/Users/{user_id}/Connect/Link",
            web::delete().to(delete_user_connect_link),
        )
        .route(
            "/Users/{user_id}/EasyPassword",
            web::post().to(post_user_easy_password),
        )
        .route(
            "/Users/{user_id}/Password",
            web::post().to(update_user_password),
        )
        .route(
            "/Users/{user_id}/Configuration",
            web::post().to(update_user_configuration),
        )
        .route(
            "/Users/{user_id}/Policy",
            web::post().to(update_user_policy),
        )
        .route(
            "/Users/{user_id}/Images/{image_type}",
            web::get().to(get_user_image),
        )
        .route(
            "/Users/{user_id}/Images/{image_type}",
            web::post().to(upload_user_image),
        )
        .route(
            "/Users/{user_id}/Images/{image_type}",
            web::delete().to(delete_user_image),
        )
        .route(
            "/Users/{user_id}/Images/{image_type}/{image_index}",
            web::get().to(get_user_image_with_index),
        )
        .route(
            "/Users/{user_id}/Images/{image_type}/{image_index}",
            web::post().to(post_user_image_with_index),
        )
        .route(
            "/Users/{user_id}/Images/{image_type}/{image_index}",
            web::delete().to(delete_user_image_with_index),
        )
        .route("/Items", web::get().to(get_items))
        .route("/Persons", web::get().to(get_persons))
        .route("/Persons/{person_id}", web::get().to(get_person))
        .route(
            "/Persons/{person_id}/Images/{image_type}",
            web::get().to(stream_person_image),
        )
        .route("/Users/{user_id}/Items", web::get().to(get_user_items))
        .route(
            "/Users/{user_id}/Items/Root",
            web::get().to(get_user_root_items),
        )
        .route("/Users/{user_id}/Views", web::get().to(get_user_views))
        .route(
            "/Users/{user_id}/GroupingOptions",
            web::get().to(get_user_grouping_options),
        )
        .route(
            "/Users/{user_id}/Suggestions",
            web::get().to(get_user_suggestions),
        )
        .route(
            "/Users/{user_id}/Items/Latest",
            web::get().to(get_user_latest_items),
        )
        .route(
            "/Users/{user_id}/Items/Resume",
            web::get().to(get_user_resume_items),
        )
        .route(
            "/Users/{user_id}/Items/{item_id}",
            web::get().to(get_user_item),
        )
        .route(
            "/Users/{user_id}/Items/{item_id}/Intros",
            web::get().to(get_user_item_intros),
        )
        .route(
            "/Users/{user_id}/Items/{item_id}/LocalTrailers",
            web::get().to(get_user_item_local_trailers),
        )
        .route(
            "/Users/{user_id}/Items/{item_id}/SpecialFeatures",
            web::get().to(get_user_item_special_features),
        )
        .route(
            "/Users/{user_id}/Items/{item_id}/Rating",
            web::post().to(post_user_item_rating),
        )
        .route(
            "/Users/{user_id}/Items/{item_id}/Rating",
            web::delete().to(delete_user_item_rating),
        )
        .route(
            "/Users/{user_id}/Items/{item_id}/UserData",
            web::get().to(get_user_item_data),
        )
        .route(
            "/Users/{user_id}/Items/{item_id}/HideFromResume",
            web::post().to(post_user_item_hide_from_resume),
        )
        .route(
            "/Users/{user_id}/PlayingItems/{item_id}",
            web::post().to(post_user_playing_item_start),
        )
        .route(
            "/Users/{user_id}/PlayingItems/{item_id}",
            web::delete().to(delete_user_playing_item),
        )
        .route(
            "/Users/{user_id}/PlayingItems/{item_id}/Progress",
            web::post().to(post_user_playing_item_progress),
        )
        .route(
            "/Users/{user_id}/PlayedItems/{item_id}",
            web::post().to(mark_item_played),
        )
        .route(
            "/Users/{user_id}/PlayedItems/{item_id}",
            web::delete().to(mark_item_unplayed),
        )
        .route(
            "/Users/{user_id}/PlayedItems/{item_id}/{action}",
            web::post().to(post_item_played_legacy_action),
        )
        .route(
            "/Users/{user_id}/FavoriteItems/{item_id}",
            web::post().to(add_item_favorite),
        )
        .route(
            "/Users/{user_id}/FavoriteItems/{item_id}",
            web::delete().to(remove_item_favorite),
        )
        .route(
            "/Users/{user_id}/FavoriteItems/{item_id}/{action}",
            web::post().to(post_item_favorite_legacy_action),
        )
        .route("/Items/Counts", web::get().to(get_item_counts))
        .route("/Items/TopPlayed", web::get().to(get_top_played_items))
        .route("/Items/Filters", web::get().to(get_items_filters))
        .route("/Items/Filters2", web::get().to(get_items_filters2))
        .route("/Items/Prefixes", web::get().to(get_item_prefixes))
        .route("/Items/File", web::get().to(get_items_file))
        .route("/Items", web::delete().to(delete_items_bulk))
        .route("/Items/{item_id}", web::get().to(get_item))
        .route("/Items/{item_id}", web::post().to(post_item))
        .route("/Items/{item_id}", web::delete().to(delete_item_compat))
        .route(
            "/Items/{item_id}/ExternalIdInfos",
            web::get().to(get_item_external_id_infos),
        )
        .route(
            "/Items/RemoteSearch/Trailer",
            web::post().to(post_items_remote_search_generic),
        )
        .route(
            "/Items/RemoteSearch/Book",
            web::post().to(post_items_remote_search_generic),
        )
        .route(
            "/Items/RemoteSearch/Movie",
            web::post().to(post_items_remote_search_generic),
        )
        .route(
            "/Items/RemoteSearch/Series",
            web::post().to(post_items_remote_search_generic),
        )
        .route(
            "/Items/RemoteSearch/Season",
            web::post().to(post_items_remote_search_generic),
        )
        .route(
            "/Items/RemoteSearch/Episode",
            web::post().to(post_items_remote_search_generic),
        )
        .route(
            "/Items/RemoteSearch/Game",
            web::post().to(post_items_remote_search_generic),
        )
        .route(
            "/Items/RemoteSearch/BoxSet",
            web::post().to(post_items_remote_search_generic),
        )
        .route(
            "/Items/RemoteSearch/MusicVideo",
            web::post().to(post_items_remote_search_generic),
        )
        .route(
            "/Items/RemoteSearch/Person",
            web::post().to(post_items_remote_search_generic),
        )
        .route(
            "/Items/RemoteSearch/MusicAlbum",
            web::post().to(post_items_remote_search_generic),
        )
        .route(
            "/Items/RemoteSearch/MusicArtist",
            web::post().to(post_items_remote_search_generic),
        )
        .route(
            "/Items/RemoteSearch/Image",
            web::get().to(get_items_remote_search_image),
        )
        .route("/Items/{item_id}/Refresh", web::post().to(post_item_refresh))
        .route(
            "/Items/{item_id}/MetadataEditor",
            web::get().to(get_item_metadata_editor),
        )
        .route(
            "/Items/{item_id}/InstantMix",
            web::get().to(get_item_instant_mix),
        )
        .route(
            "/Items/{item_id}/DeleteInfo",
            web::get().to(get_item_delete_info),
        )
        .route("/Items/{item_id}/Similar", web::get().to(get_item_similar))
        .route("/Items/{item_id}/Download", web::get().to(get_item_download))
        .route("/Items/{item_id}/File", web::get().to(get_item_file_by_id))
        .route(
            "/Items/{item_id}/Ancestors",
            web::get().to(get_item_ancestors),
        )
        .route(
            "/Items/{item_id}/CriticReviews",
            web::get().to(get_item_critic_reviews),
        )
        .route(
            "/Items/{item_id}/ThemeMedia",
            web::get().to(get_item_theme_media),
        )
        .route(
            "/Items/{item_id}/ThemeSongs",
            web::get().to(get_item_theme_songs),
        )
        .route(
            "/Items/{item_id}/ThemeVideos",
            web::get().to(get_item_theme_videos),
        )
        .route("/Items/{item_id}/Images", web::get().to(get_item_images_list))
        .route(
            "/Items/{item_id}/RemoteImages",
            web::get().to(get_item_remote_images),
        )
        .route(
            "/Items/{item_id}/ThumbnailSet",
            web::get().to(get_item_thumbnail_set),
        )
        .route(
            "/Items/RemoteSearch/Apply/{item_id}",
            web::post().to(post_items_remote_search_apply),
        )
        .route(
            "/Items/{item_id}/RemoteImages/Providers",
            web::get().to(get_item_remote_images_providers),
        )
        .route(
            "/Items/{item_id}/RemoteImages/Download",
            web::post().to(post_item_remote_images_download),
        )
        .route(
            "/Items/{item_id}/RemoteSearch/Subtitles/{language}",
            web::get().to(get_item_remote_search_subtitles),
        )
        .route(
            "/Items/{item_id}/RemoteSearch/Subtitles/{subtitle_id}",
            web::post().to(post_item_remote_search_subtitle_download),
        )
        .route(
            "/Items/{item_id}/Images/{image_type}/{image_index}",
            web::post().to(post_item_image_with_index),
        )
        .route(
            "/Items/{item_id}/Images/{image_type}/{image_index}",
            web::delete().to(delete_item_image_with_index_compat),
        )
        .route(
            "/Items/{item_id}/Images/{image_type}/{image_index}/Index",
            web::post().to(post_item_image_reorder_index),
        )
        .route(
            "/Items/{item_id}/Images/{image_type}/{image_index}/{tag}/{format}/{max_width}/{max_height}/{percent_played}/{unplayed_count}",
            web::get().to(stream_item_image_legacy_full_path),
        )
        .route(
            "/Libraries/AvailableOptions",
            web::get().to(get_libraries_available_options),
        )
        .route(
            "/Library/SelectableMediaFolders",
            web::get().to(get_library_selectable_media_folders),
        )
        .route(
            "/Library/MediaFolders",
            web::get().to(get_library_media_folders),
        )
        .route(
            "/Library/PhysicalPaths",
            web::get().to(get_library_physical_paths),
        )
        .route("/Library/Refresh", web::post().to(post_library_refresh))
        .route(
            "/Library/VirtualFolders",
            web::get().to(get_library_virtual_folders),
        )
        .route(
            "/Library/VirtualFolders",
            web::post().to(post_library_virtual_folders),
        )
        .route(
            "/Library/VirtualFolders",
            web::delete().to(delete_library_virtual_folders),
        )
        .route(
            "/Library/Series/Added",
            web::post().to(post_library_changed_noop_empty),
        )
        .route(
            "/Library/Series/Updated",
            web::post().to(post_library_changed_noop_empty),
        )
        .route(
            "/Library/Media/Updated",
            web::post().to(post_library_changed_noop),
        )
        .route(
            "/Library/Movies/Added",
            web::post().to(post_library_changed_noop_empty),
        )
        .route(
            "/Library/Movies/Updated",
            web::post().to(post_library_changed_noop_empty),
        )
        .route(
            "/Library/VirtualFolders/LibraryOptions",
            web::post().to(post_library_virtual_folders_library_options),
        )
        .route(
            "/Library/VirtualFolders/Name",
            web::post().to(post_library_virtual_folders_name),
        )
        .route(
            "/Library/VirtualFolders/Paths",
            web::post().to(post_library_virtual_folders_paths),
        )
        .route(
            "/Library/VirtualFolders/Paths",
            web::delete().to(delete_library_virtual_folders_paths),
        )
        .route(
            "/Library/VirtualFolders/Paths/Update",
            web::post().to(post_library_virtual_folders_paths_update),
        )
        .route("/Search/Hints", web::get().to(get_search_hints))
        .route("/Genres", web::get().to(get_genres))
        .route("/Studios", web::get().to(get_studios))
        .route("/OfficialRatings", web::get().to(get_official_ratings))
        .route("/Tags", web::get().to(get_tags))
        .route("/Years", web::get().to(get_years))
        .route(
            "/Shows/{show_id}/Episodes",
            web::get().to(get_show_episodes),
        )
        .route(
            "/Shows/{series_id}/Seasons",
            web::get().to(get_show_seasons),
        )
        .route("/Shows/NextUp", web::get().to(get_shows_next_up))
        .route("/UserItems/Resume", web::get().to(get_user_items_resume))
        .route(
            "/UserItems/{item_id}/UserData",
            web::post().to(post_user_item_data),
        )
        .route(
            "/Items/{item_id}/PlaybackInfo",
            web::get().to(get_playback_info),
        )
        .route(
            "/Items/{item_id}/PlaybackInfo",
            web::post().to(post_playback_info),
        )
        .route(
            "/Items/{item_id}/Subtitles",
            web::get().to(get_item_subtitles),
        )
        .route(
            "/Items/{item_id}/Subtitles/{subtitle_index}/Stream",
            web::get().to(stream_item_subtitle),
        )
        .route(
            "/Items/{item_id}/Subtitles/{subtitle_index}/Stream.{format}",
            web::get().to(stream_item_subtitle_with_format),
        )
        .route(
            "/Items/{item_id}/{media_source_id}/Subtitles/{subtitle_index}/Stream",
            web::get().to(stream_item_subtitle_with_media_source),
        )
        .route(
            "/Items/{item_id}/{media_source_id}/Subtitles/{subtitle_index}/Stream.{format}",
            web::get().to(stream_item_subtitle_with_media_source_and_format),
        )
        .route(
            "/Items/{item_id}/Images/{image_type}",
            web::get().to(stream_item_image),
        )
        .route(
            "/Items/{item_id}/Images/{image_type}",
            web::post().to(upload_item_image),
        )
        .route(
            "/Items/{item_id}/Images/{image_type}",
            web::delete().to(delete_item_image),
        )
        .route(
            "/Items/{item_id}/Images/{image_type}/{image_index}",
            web::get().to(stream_item_image_with_index),
        )
        .route("/Videos/{item_id}/stream", web::get().to(stream_video))
        .route("/Videos/{item_id}/stream", web::head().to(stream_video))
        .route(
            "/Videos/{item_id}/stream.{container}",
            web::get().to(stream_video_with_container),
        )
        .route(
            "/Videos/{item_id}/stream.{container}",
            web::head().to(stream_video_with_container),
        )
        .route(
            "/Videos/{item_id}/original.{container}",
            web::get().to(stream_video_original_with_container),
        )
        .route(
            "/Videos/{item_id}/original.{container}",
            web::head().to(stream_video_original_with_container),
        )
        .route(
            "/Videos/{item_id}/Subtitles/{subtitle_index}/Stream",
            web::get().to(stream_video_subtitle),
        )
        .route(
            "/Videos/{item_id}/Subtitles/{subtitle_index}/Stream.{format}",
            web::get().to(stream_video_subtitle_with_format),
        )
        .route(
            "/Videos/{item_id}/{media_source_id}/Subtitles/{subtitle_index}/Stream",
            web::get().to(stream_video_subtitle_with_media_source),
        )
        .route(
            "/Videos/{item_id}/{media_source_id}/Subtitles/{subtitle_index}/Stream.{format}",
            web::get().to(stream_video_subtitle_with_media_source_and_format),
        )
        .route("/Sessions/Playing", web::post().to(report_playing_start))
        .route(
            "/Sessions/Playing/Progress",
            web::post().to(report_playing_progress),
        )
        .route(
            "/Sessions/Playing/Stopped",
            web::post().to(report_playing_stopped),
        )
        .route("/Sessions", web::get().to(get_sessions))
        .route(
            "/Sessions/Capabilities",
            web::post().to(post_sessions_capabilities),
        )
        .route(
            "/Sessions/Capabilities/Full",
            web::post().to(post_sessions_capabilities_full),
        )
        .route(
            "/Sessions/{session_id}/Command",
            web::post().to(post_session_command),
        )
        .route(
            "/Sessions/{session_id}/Command/{command}",
            web::post().to(post_session_command_named),
        )
        .route(
            "/Sessions/{session_id}/Message",
            web::post().to(post_session_message),
        )
        .route(
            "/Sessions/{session_id}/Playing",
            web::post().to(post_session_playing),
        )
        .route(
            "/Sessions/{session_id}/Playing/{command}",
            web::post().to(post_session_playing_command),
        )
        .route(
            "/Sessions/{session_id}/System/{command}",
            web::post().to(post_session_system_command),
        )
        .route(
            "/Sessions/{session_id}/Users/{user_id}",
            web::post().to(post_session_user),
        )
        .route(
            "/Sessions/{session_id}/Users/{user_id}",
            web::delete().to(delete_session_user),
        )
        .route(
            "/Sessions/{session_id}/Viewing",
            web::post().to(post_session_viewing),
        )
        .route("/Sessions/Playing/Ping", web::post().to(report_playing_ping))
        .route("/Sessions/Logout", web::post().to(logout_session))
        .route("/api/invite/me", web::get().to(get_my_invite_summary))
        .route("/api/invite/me/reset", web::post().to(reset_my_invite_code))
        .route("/admin/libraries", web::get().to(admin_list_libraries))
        .route("/admin/libraries", web::post().to(admin_create_library))
        .route(
            "/admin/libraries/{library_id}",
            web::patch().to(admin_patch_library),
        )
        .route(
            "/admin/libraries/status",
            web::get().to(admin_list_library_status),
        )
        .route(
            "/admin/libraries/{library_id}/disable",
            web::post().to(admin_disable_library),
        )
        .route(
            "/admin/libraries/{library_id}/enable",
            web::post().to(admin_enable_library),
        )
        .route(
            "/admin/task-center/tasks",
            web::get().to(admin_list_task_definitions),
        )
        .route(
            "/admin/task-center/tasks/{task_key}",
            web::patch().to(admin_patch_task_definition),
        )
        .route(
            "/admin/task-center/tasks/{task_key}/run",
            web::post().to(admin_run_task_now),
        )
        .route(
            "/admin/task-center/runs",
            web::get().to(admin_list_task_runs),
        )
        .route(
            "/admin/task-center/runs/{run_id}",
            web::get().to(admin_get_task_run),
        )
        .route(
            "/admin/task-center/runs/{run_id}/cancel",
            web::post().to(admin_cancel_task_run),
        )
        .route("/admin/task-center/ws", web::get().to(admin_task_runs_ws))
        .route("/admin/users", web::get().to(admin_list_users))
        .route("/admin/users", web::post().to(admin_create_user))
        .route(
            "/admin/users/summary",
            web::get().to(admin_list_user_summaries),
        )
        .route(
            "/admin/users/{user_id}/profile",
            web::get().to(admin_get_user_profile),
        )
        .route(
            "/admin/users/{user_id}/profile",
            web::patch().to(admin_patch_user_profile),
        )
        .route(
            "/admin/users/{user_id}",
            web::delete().to(admin_delete_user),
        )
        .route(
            "/admin/users/batch-status",
            web::post().to(admin_batch_set_user_status),
        )
        .route(
            "/admin/users/{user_id}/disable",
            web::post().to(admin_disable_user),
        )
        .route(
            "/admin/users/{user_id}/enable",
            web::post().to(admin_enable_user),
        )
        .route("/admin/sessions", web::get().to(admin_list_sessions))
        .route(
            "/admin/auth-sessions",
            web::get().to(admin_list_auth_sessions),
        )
        .route("/admin/audit-logs", web::get().to(admin_list_audit_logs))
        .route("/admin/api-keys", web::get().to(admin_list_api_keys))
        .route("/admin/api-keys", web::post().to(admin_create_api_key))
        .route("/admin/api-keys/{key_id}", web::delete().to(admin_delete_api_key))
        .route("/admin/settings", web::get().to(admin_get_settings))
        .route("/admin/settings", web::post().to(admin_upsert_settings))
        .route("/admin/scraper/settings", web::get().to(admin_get_scraper_settings))
        .route("/admin/scraper/settings", web::post().to(admin_upsert_scraper_settings))
        .route("/admin/scraper/providers", web::get().to(admin_list_scraper_providers))
        .route(
            "/admin/scraper/providers/{provider_id}/test",
            web::post().to(admin_test_scraper_provider),
        )
        .route(
            "/admin/scraper/cache-stats",
            web::get().to(admin_get_scraper_cache_stats),
        )
        .route(
            "/admin/scraper/failures",
            web::get().to(admin_list_scraper_failures),
        )
        .route(
            "/admin/scraper/cache/clear",
            web::post().to(admin_clear_scraper_cache),
        )
        .route(
            "/admin/scraper/failures/clear",
            web::post().to(admin_clear_scraper_failures),
        )
        .route(
            "/admin/items/{item_id}/rescrape",
            web::post().to(admin_rescrape_item),
        )
        .route("/admin/requests", web::get().to(admin_list_agent_requests))
        .route("/admin/requests/{request_id}", web::get().to(admin_get_agent_request))
        .route("/admin/agent/providers", web::get().to(admin_list_agent_providers))
        .route(
            "/admin/requests/{request_id}/review",
            web::post().to(admin_review_agent_request),
        )
        .route(
            "/admin/requests/{request_id}/retry",
            web::post().to(admin_retry_agent_request),
        )
        .route("/admin/agent/settings", web::get().to(admin_get_agent_settings))
        .route("/admin/agent/settings", web::post().to(admin_upsert_agent_settings))
        .route(
            "/admin/agent/moviepilot/test",
            web::post().to(admin_test_agent_moviepilot),
        )
        .route(
            "/admin/invite/settings",
            web::get().to(admin_get_invite_settings),
        )
        .route(
            "/admin/invite/settings",
            web::post().to(admin_upsert_invite_settings),
        )
        .route(
            "/admin/invite/relations",
            web::get().to(admin_list_invite_relations),
        )
        .route("/admin/system/flags", web::get().to(admin_get_system_flags))
        .route(
            "/admin/system/flags",
            web::post().to(admin_update_system_flags),
        )
        .route(
            "/admin/system/summary",
            web::get().to(admin_get_system_summary),
        )
        .route(
            "/admin/system/capabilities",
            web::get().to(admin_get_system_capabilities),
        )
        .route(
            "/admin/storage-configs",
            web::get().to(admin_list_storage_configs),
        )
        .route(
            "/admin/storage-configs",
            web::post().to(admin_upsert_storage_config),
        )
        .route(
            "/admin/storage/cache/cleanup",
            web::post().to(admin_cleanup_storage_cache),
        )
        .route(
            "/admin/storage/cache/invalidate",
            web::post().to(admin_invalidate_storage_cache),
        )
        .route(
            "/admin/tmdb/cache-stats",
            web::get().to(admin_get_tmdb_cache_stats),
        )
        .route(
            "/admin/tmdb/failures",
            web::get().to(admin_list_tmdb_failures),
        )
        .route(
            "/admin/tmdb/cache/clear",
            web::post().to(admin_clear_tmdb_cache),
        )
        .route(
            "/admin/tmdb/failures/clear",
            web::post().to(admin_clear_tmdb_failures),
        )
        .route(
            "/admin/playback-domains",
            web::get().to(admin_list_playback_domains),
        )
        .route(
            "/admin/playback-domains",
            web::post().to(admin_upsert_playback_domain),
        )
        .route(
            "/admin/playback-domains/{domain_id}",
            web::delete().to(admin_delete_playback_domain),
        )
        .route(
            "/admin/lumenbackend/nodes",
            web::get().to(admin_list_lumenbackend_nodes),
        )
        .route(
            "/admin/lumenbackend/nodes",
            web::post().to(admin_create_lumenbackend_node),
        )
        .route(
            "/admin/lumenbackend/nodes/{node_id}",
            web::patch().to(admin_patch_lumenbackend_node),
        )
        .route(
            "/admin/lumenbackend/nodes/{node_id}",
            web::delete().to(admin_delete_lumenbackend_node),
        )
        .route(
            "/admin/lumenbackend/nodes/{node_id}/schema",
            web::get().to(admin_get_lumenbackend_node_schema),
        )
        .route(
            "/admin/lumenbackend/nodes/{node_id}/config",
            web::get().to(admin_get_lumenbackend_node_config),
        )
        .route(
            "/admin/lumenbackend/nodes/{node_id}/config",
            web::post().to(admin_upsert_lumenbackend_node_config),
        )
        .route(
            "/internal/lumenbackend/register",
            web::post().to(lumenbackend_register),
        )
        .route(
            "/internal/lumenbackend/heartbeat",
            web::post().to(lumenbackend_heartbeat),
        )
        .route(
            "/internal/lumenbackend/runtime-config",
            web::get().to(lumenbackend_runtime_config),
        )
        .route(
            "/internal/lumenbackend/traffic/report",
            web::post().to(lumenbackend_report_traffic),
        )
        .route("/api/playlists/mine", web::get().to(list_my_playlists))
        .route("/api/playlists", web::post().to(create_playlist))
        // Emby-compatible /Playlists routes
        .route("/Playlists", web::post().to(emby_create_playlist))
        .route(
            "/Playlists/{playlist_id}/Items",
            web::get().to(emby_get_playlist_items),
        )
        .route(
            "/Playlists/{playlist_id}/Items",
            web::post().to(emby_add_playlist_items),
        )
        .route(
            "/Playlists/{playlist_id}/Items",
            web::delete().to(emby_delete_playlist_items),
        )
        .route(
            "/Playlists/{playlist_id}/Items/{item_id}/Move/{new_index}",
            web::post().to(emby_move_playlist_item),
        )
        // Emby-compatible /Collections routes
        .route("/Collections", web::post().to(emby_create_collection))
        .route(
            "/Collections/{collection_id}/Items",
            web::post().to(emby_add_collection_items),
        )
        .route(
            "/Collections/{collection_id}/Items",
            web::delete().to(emby_delete_collection_items),
        )
        .route("/api/playlists/{playlist_id}", web::get().to(get_playlist))
        .route(
            "/api/playlists/{playlist_id}",
            web::patch().to(update_playlist),
        )
        .route(
            "/api/playlists/{playlist_id}",
            web::delete().to(delete_playlist),
        )
        .route(
            "/api/playlists/{playlist_id}/items",
            web::get().to(list_playlist_items),
        )
        .route(
            "/api/playlists/{playlist_id}/items",
            web::post().to(add_playlist_item),
        )
        .route(
            "/api/playlists/{playlist_id}/items/{item_id}",
            web::delete().to(remove_playlist_item),
        )
        .route(
            "/api/users/{user_id}/playlists/public",
            web::get().to(list_user_public_playlists),
        )
        .route("/api/notifications", web::get().to(list_notifications))
        .route("/api/notifications", web::post().to(create_notification))
        .route("/api/notifications/ws", web::get().to(notifications_ws))
        .route(
            "/api/notifications/read-all",
            web::patch().to(mark_all_notifications_read),
        )
        .route(
            "/api/notifications/{notification_id}/read",
            web::patch().to(mark_notification_read),
        );

    register_commercial_routes(scope, capabilities)
}
