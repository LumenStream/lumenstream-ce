#[cfg(test)]
mod tests {
    use super::{
        AppInfra, DEFAULT_FAVORITES_PLAYLIST_DESCRIPTION, DEFAULT_FAVORITES_PLAYLIST_NAME,
        ImageResizeFormat, ImageResizeRequest, InfraError, ItemsQuery, MediaItemRow,
        MovieMatchHints, ParentQueryScope, ParentScopeSqlBind, PersonRow, SubtitleRow,
        TmdbFillItemRow, VersionedMediaSourceRow, append_external_subtitles_to_media_source,
        apply_global_lumenbackend_stream_fields,
        apply_scrape_result_to_metadata,
        apply_item_people_overrides, assess_movie_match, build_meili_filter,
        build_movie_match_hints, build_playback_mediainfo_from_ffprobe, build_resized_image,
        build_schema_default_runtime_config, build_lumenbackend_stream_token, calc_retry_delay_seconds,
        distributed_offset, expand_ids_with_person_media, extract_mediainfo_bitrate,
        extract_mediainfo_container, extract_mediainfo_runtime_ticks, extract_nfo_studios,
        extract_playback_item_id, extract_playback_position_ticks, extract_tmdb_keywords,
        extract_tmdb_people, group_person_media_rows, image_candidates,
        index_external_subtitle_rows, infer_playback_played_flag, infer_stream_url_from_strm_path,
        item_row_to_dto, library_image_base_names, library_image_cache_path,
        library_image_candidates, library_image_target_basename, libvips_size_spec,
        mask_secret_fields, media_item_image_candidates, media_source_path_from_row,
        mediainfo_has_primary_streams, meili_quote, merge_json_values, merge_missing_json,
        merge_secret_placeholders, merge_tmdb_tags_into_metadata,
        metadata_tmdb_movie_conflicts_local_hints, migrate_legacy_default_library_paths,
        normalize_default_optional_i32, normalize_default_optional_i64,
        normalize_emby_collection_type, normalize_image_extension, normalize_media_bitrate,
        normalize_media_runtime_ticks, normalize_playback_mediainfo, normalize_lumenbackend_base_url,
        normalize_lumenbackend_stream_route, normalize_tmdb_match_title, normalize_traffic_window_days,
        parse_chapters_from_mediainfo, parse_media_streams_from_mediainfo,
        parse_runtime_schema_fields, parse_lumenbackend_http_stream_url, parse_lumenbackend_reference,
        person_image_cache_path, person_primary_image_tag_for_response, person_row_to_dto,
        playlist_is_visible_to, read_nfo_imdb_id, read_nfo_tmdb_id, remove_image_candidates,
        resize_target_box, resized_image_cache_key, resized_image_cache_path,
        resized_image_temp_path, resolve_movie_nfo_imdb_id, resolve_movie_nfo_tmdb_id,
        resolve_person_image_path_from_cache_hints, resolve_scan_scope_path,
        resolve_season_nfo_path, resolve_series_nfo_imdb_id, resolve_series_nfo_tmdb_id,
        root_library_primary_image_tag, season_number_from_stem,
        select_best_movie_search_candidate, select_lumenbackend_stream_base_url, select_tmdb_logo_path,
        should_delete_linked_strm, should_fill_tmdb_for_item, strip_node_runtime_protected_fields,
        subtitle_codec_from_path, subtitle_row_to_media_stream, tmdb_cache_key,
        scrape_fill_search_index_since, tmdb_include_image_language, tmdb_logo_extension,
        tmdb_movie_official_rating, tmdb_movie_release_year, tmdb_tv_official_rating,
        validate_runtime_bootstrap_credentials, validate_runtime_config_against_schema,
        write_episode_nfo, write_movie_nfo, write_tvshow_nfo,
    };
    use chrono::{TimeZone, Utc};
    use ls_config::{AppConfig, AuthConfig};
    use ls_domain::jellyfin::{MediaSourceInfoDto, MediaStreamDto, PlaybackProgressDto};
    use ls_scraper::{ScrapePatch, ScrapeResult};
    use serde_json::json;
    use sqlx::postgres::PgPoolOptions;
    use std::sync::{Arc, RwLock};
    use tokio::sync::Mutex;
    use uuid::Uuid;

    #[test]
    fn subtitle_codec_from_path_reads_extension() {
        assert_eq!(subtitle_codec_from_path("/tmp/movie.zh.ass"), "ass");
        assert_eq!(subtitle_codec_from_path("/tmp/movie"), "srt");
    }

    #[test]
    fn apply_scrape_result_to_metadata_primary_overwrites_with_non_null_values() {
        let current = json!({
            "overview": "old",
            "provider_ids": { "Tmdb": "1" }
        });
        let result = ScrapeResult {
            provider_id: "tvdb".to_string(),
            scenario: "series_metadata".to_string(),
            patch: ScrapePatch {
                metadata: json!({
                    "overview": "new",
                    "title": "Series",
                    "provider_ids": { "Tvdb": "100" }
                }),
                provider_ids: std::collections::BTreeMap::from([
                    ("Tvdb".to_string(), "100".to_string()),
                ]),
                ..ScrapePatch::default()
            },
            raw: json!({ "id": 100 }),
            warnings: Vec::new(),
            complete: true,
        };

        let merged = apply_scrape_result_to_metadata(&current, &result, true, "tvdb");
        assert_eq!(merged.get("overview").and_then(|v| v.as_str()), Some("new"));
        assert_eq!(
            merged
                .get("provider_ids")
                .and_then(|v| v.get("Tvdb"))
                .and_then(|v| v.as_str()),
            Some("100")
        );
    }

    #[test]
    fn apply_scrape_result_to_metadata_supplement_preserves_existing_core_fields() {
        let current = json!({
            "overview": "primary",
            "provider_ids": { "Bangumi": "42" }
        });
        let result = ScrapeResult {
            provider_id: "tvdb".to_string(),
            scenario: "series_metadata".to_string(),
            patch: ScrapePatch {
                metadata: json!({
                    "overview": "fallback",
                    "official_rating": "TV-14"
                }),
                provider_ids: std::collections::BTreeMap::from([
                    ("Tvdb".to_string(), "100".to_string()),
                ]),
                ..ScrapePatch::default()
            },
            raw: json!({ "id": 100 }),
            warnings: Vec::new(),
            complete: true,
        };

        let merged = apply_scrape_result_to_metadata(&current, &result, false, "tvdb");
        assert_eq!(merged.get("overview").and_then(|v| v.as_str()), Some("primary"));
        assert_eq!(
            merged.get("official_rating").and_then(|v| v.as_str()),
            Some("TV-14")
        );
        assert_eq!(
            merged
                .get("provider_ids")
                .and_then(|v| v.get("Bangumi"))
                .and_then(|v| v.as_str()),
            Some("42")
        );
        assert_eq!(
            merged
                .get("provider_ids")
                .and_then(|v| v.get("Tvdb"))
                .and_then(|v| v.as_str()),
            Some("100")
        );
    }

    #[test]
    fn infer_stream_url_from_strm_path_reads_http_target() {
        let file_path = std::env::temp_dir().join(format!("ls-strm-{}.strm", Uuid::new_v4()));
        std::fs::write(&file_path, "https://cdn.example.com/video.mp4\n").expect("write strm file");

        let stream_url =
            infer_stream_url_from_strm_path(file_path.to_str().expect("temp strm path as utf-8"));
        assert_eq!(
            stream_url.as_deref(),
            Some("https://cdn.example.com/video.mp4")
        );

        std::fs::remove_file(file_path).expect("remove temp strm file");
    }

    #[test]
    fn infer_stream_url_from_strm_path_ignores_non_stream_targets() {
        let file_path = std::env::temp_dir().join(format!("ls-strm-{}.strm", Uuid::new_v4()));
        std::fs::write(&file_path, "/mnt/media/movie.mkv\n").expect("write strm file");

        let stream_url =
            infer_stream_url_from_strm_path(file_path.to_str().expect("temp strm path as utf-8"));
        assert!(stream_url.is_none());

        std::fs::remove_file(file_path).expect("remove temp strm file");
    }

    #[test]
    fn media_source_path_from_row_prefers_metadata_and_stream_column_without_strm_file_fallback() {
        let file_path = std::env::temp_dir().join(format!("ls-src-{}.strm", Uuid::new_v4()));
        std::fs::write(&file_path, "https://cdn.example.com/fallback.mp4\n")
            .expect("write strm file");

        let metadata = json!({
            "stream_url": "https://cdn.example.com/from-metadata.mp4"
        });
        let from_metadata = media_source_path_from_row(
            file_path.to_str().expect("temp path as utf-8"),
            Some("https://cdn.example.com/from-column.mp4"),
            &metadata,
        );
        assert_eq!(from_metadata, "https://cdn.example.com/from-metadata.mp4");

        let no_metadata = json!({});
        let from_stream_column = media_source_path_from_row(
            file_path.to_str().expect("temp path as utf-8"),
            Some("https://cdn.example.com/from-column.mp4"),
            &no_metadata,
        );
        assert_eq!(
            from_stream_column,
            "https://cdn.example.com/from-column.mp4"
        );

        let from_path = media_source_path_from_row(
            file_path.to_str().expect("temp path as utf-8"),
            None,
            &no_metadata,
        );
        assert_eq!(from_path, file_path.to_str().expect("temp path as utf-8"));

        std::fs::remove_file(file_path).expect("remove temp strm file");
    }

    #[test]
    fn media_source_path_from_row_supports_legacy_strm_url_metadata_alias() {
        let metadata = json!({
            "strm_url": "https://cdn.example.com/from-strm-url.mp4"
        });
        let resolved = media_source_path_from_row("/mnt/media/movie.strm", None, &metadata);
        assert_eq!(resolved, "https://cdn.example.com/from-strm-url.mp4");
    }

    #[test]
    fn normalize_playback_mediainfo_accepts_media_source_with_chapters_payload() {
        let raw = json!({
            "MediaSourceWithChapters": [
                {
                    "MediaSourceInfo": {
                        "MediaStreams": [
                            { "codec_type": "video" }
                        ]
                    }
                }
            ]
        });
        let normalized = normalize_playback_mediainfo(&raw);
        assert!(normalized.is_array());
        assert!(mediainfo_has_primary_streams(&normalized));
    }

    #[test]
    fn build_playback_mediainfo_from_ffprobe_maps_runtime_bitrate_and_chapters() {
        let ffprobe_payload = json!({
            "format": {
                "duration": "7.2",
                "bit_rate": "5000000",
                "format_name": "matroska,webm"
            },
            "streams": [
                { "codec_type": "video", "bit_rate": "4800000" },
                { "codec_type": "audio", "bit_rate": "192000" }
            ],
            "chapters": [
                { "id": 0, "start_time": "0.000000", "tags": { "title": "Intro" } },
                { "id": 1, "start_time": "2.500000", "tags": { "title": "Act 1" } }
            ]
        });

        let generated = build_playback_mediainfo_from_ffprobe(&ffprobe_payload);
        let normalized = normalize_playback_mediainfo(&generated);
        assert_eq!(
            extract_mediainfo_runtime_ticks(&normalized),
            Some(72_000_000)
        );
        assert_eq!(extract_mediainfo_bitrate(&normalized), Some(5_000_000));
        assert!(mediainfo_has_primary_streams(&normalized));
        let chapters = parse_chapters_from_mediainfo(&normalized);
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].chapter_index, Some(0));
        assert_eq!(chapters[0].start_position_ticks, 0);
        assert_eq!(chapters[0].name.as_deref(), Some("Intro"));
        assert_eq!(chapters[0].marker_type.as_deref(), Some("Chapter"));
        assert_eq!(chapters[1].start_position_ticks, 25_000_000);
    }

    fn build_test_infra_for_unit() -> AppInfra {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@127.0.0.1:5432/lumenstream")
            .expect("create lazy postgres pool");
        let (notification_tx, _) = tokio::sync::broadcast::channel(16);
        let (task_run_tx, _) = tokio::sync::broadcast::channel(16);
        let (recharge_order_tx, _) = tokio::sync::broadcast::channel(16);
        let (agent_request_tx, _) = tokio::sync::broadcast::channel(16);
        AppInfra {
            pool,
            config: Arc::new(RwLock::new(AppConfig::default())),
            server_id: "test-server".to_string(),
            http_client: reqwest::Client::new(),
            search_backend: None,
            metrics: Arc::new(super::InfraMetrics::default()),
            tmdb_last_request: Arc::new(Mutex::new(None)),
            resized_image_locks: Arc::new(Mutex::new(std::collections::HashMap::new())),
            notification_tx,
            task_run_tx,
            recharge_order_tx,
            agent_request_tx,
        }
    }

    #[tokio::test]
    async fn select_playback_mediainfo_backfill_candidate_prefers_expected_source_id() {
        let infra = build_test_infra_for_unit();
        let id_a = Uuid::new_v4();
        let id_b = Uuid::new_v4();
        let rows = vec![
            VersionedMediaSourceRow {
                id: id_a,
                name: "A".to_string(),
                path: "/mnt/media/a.mkv".to_string(),
                runtime_ticks: None,
                bitrate: None,
                stream_url: Some("https://cdn.example.com/a.mp4".to_string()),
                metadata: json!({}),
                version_group_id: None,
                version_rank: 0,
            },
            VersionedMediaSourceRow {
                id: id_b,
                name: "B".to_string(),
                path: "/mnt/media/b.mkv".to_string(),
                runtime_ticks: None,
                bitrate: None,
                stream_url: Some("https://cdn.example.com/b.mp4".to_string()),
                metadata: json!({}),
                version_group_id: None,
                version_rank: 1,
            },
        ];

        let selected =
            infra.select_playback_mediainfo_backfill_candidate(&rows, Some(&id_b.to_string()));
        assert_eq!(selected.map(|row| row.id), Some(id_b));
    }

    #[tokio::test]
    async fn select_playback_mediainfo_backfill_candidate_skips_already_probed_sources() {
        let infra = build_test_infra_for_unit();
        let id = Uuid::new_v4();
        let rows = vec![VersionedMediaSourceRow {
            id,
            name: "already-probed".to_string(),
            path: "/mnt/media/movie.mkv".to_string(),
            runtime_ticks: Some(72_000_000),
            bitrate: Some(4_500_000),
            stream_url: Some("https://cdn.example.com/movie.mp4".to_string()),
            metadata: json!({
                "mediainfo": [{
                    "MediaSourceInfo": {
                        "MediaStreams": [
                            {"codec_type": "video"}
                        ]
                    }
                }]
            }),
            version_group_id: None,
            version_rank: 0,
        }];

        let selected =
            infra.select_playback_mediainfo_backfill_candidate(&rows, Some(&id.to_string()));
        assert!(selected.is_none());
    }

    #[test]
    fn should_delete_linked_strm_matches_case_insensitive_extension() {
        assert!(should_delete_linked_strm("/mnt/media/movie.strm"));
        assert!(should_delete_linked_strm("/mnt/media/movie.STRM"));
        assert!(!should_delete_linked_strm("/mnt/media/movie.mkv"));
        assert!(!should_delete_linked_strm("/mnt/media/series-folder"));
    }

    #[test]
    fn parse_media_streams_from_mediainfo_maps_audio_and_subtitle_fields() {
        let mediainfo = json!([
            {
                "MediaSourceInfo": {
                    "MediaStreams": [
                        {
                            "index": 0,
                            "codec_type": "video",
                            "codec_name": "h264",
                            "bit_rate": "4500000",
                            "width": 1920,
                            "height": 1080,
                            "profile": "High",
                            "level": 41,
                            "avg_frame_rate": "24000/1001",
                            "r_frame_rate": "24/1",
                            "pix_fmt": "yuv420p10le",
                            "color_range": "tv",
                            "color_space": "bt2020nc",
                            "color_transfer": "smpte2084",
                            "color_primaries": "bt2020",
                            "side_data_list": [
                                {
                                    "side_data_type": "DOVI configuration record",
                                    "dv_version_major": 1,
                                    "dv_version_minor": 0,
                                    "dv_profile": 8,
                                    "dv_level": 6,
                                    "rpu_present_flag": 1,
                                    "el_present_flag": 0,
                                    "bl_present_flag": 1,
                                    "dv_bl_signal_compatibility_id": 1
                                },
                                {
                                    "side_data_type": "HDR Dynamic Metadata SMPTE2094-40 (HDR10+)"
                                }
                            ]
                        },
                        {
                            "index": 1,
                            "codec_type": "audio",
                            "codec_name": "aac",
                            "channels": 2,
                            "bit_rate": "192000",
                            "sample_rate": "48000",
                            "channel_layout": "stereo",
                            "tags": { "language": "eng" }
                        },
                        {
                            "index": 2,
                            "codec_type": "subtitle",
                            "codec_name": "ass",
                            "tags": { "language": "zho" },
                            "disposition": { "default": 1, "forced": 1 }
                        }
                    ]
                }
            }
        ]);

        let streams = parse_media_streams_from_mediainfo(&mediainfo);
        assert_eq!(streams.len(), 3);

        let video = streams.iter().find(|stream| stream.stream_type == "Video");
        assert!(video.is_some());
        assert_eq!(
            video.and_then(|stream| stream.codec.as_deref()),
            Some("h264")
        );
        assert_eq!(video.and_then(|stream| stream.width), Some(1920));
        assert_eq!(video.and_then(|stream| stream.height), Some(1080));
        assert_eq!(
            video.and_then(|stream| stream.profile.as_deref()),
            Some("High")
        );
        assert_eq!(video.and_then(|stream| stream.level), Some(41));
        assert!(
            video
                .and_then(|stream| stream.average_frame_rate)
                .map(|fps| fps > 23.0 && fps < 24.1)
                .unwrap_or(false)
        );
        assert_eq!(video.and_then(|stream| stream.real_frame_rate), Some(24.0));
        assert_eq!(
            video.and_then(|stream| stream.color_range.as_deref()),
            Some("tv")
        );
        assert_eq!(
            video.and_then(|stream| stream.color_space.as_deref()),
            Some("bt2020nc")
        );
        assert_eq!(
            video.and_then(|stream| stream.color_transfer.as_deref()),
            Some("smpte2084")
        );
        assert_eq!(
            video.and_then(|stream| stream.color_primaries.as_deref()),
            Some("bt2020")
        );
        assert_eq!(video.and_then(|stream| stream.bit_depth), Some(10));
        assert_eq!(
            video.and_then(|stream| stream.video_range.as_deref()),
            Some("HDR")
        );
        assert_eq!(
            video.and_then(|stream| stream.video_range_type.as_deref()),
            Some("DOVIWithHDR10Plus")
        );
        assert_eq!(
            video.and_then(|stream| stream.hdr10_plus_present_flag),
            Some(true)
        );
        assert_eq!(video.and_then(|stream| stream.dv_version_major), Some(1));
        assert_eq!(video.and_then(|stream| stream.dv_version_minor), Some(0));
        assert_eq!(video.and_then(|stream| stream.dv_profile), Some(8));
        assert_eq!(video.and_then(|stream| stream.dv_level), Some(6));
        assert_eq!(video.and_then(|stream| stream.rpu_present_flag), Some(true));
        assert_eq!(video.and_then(|stream| stream.el_present_flag), Some(false));
        assert_eq!(video.and_then(|stream| stream.bl_present_flag), Some(true));
        assert_eq!(
            video.and_then(|stream| stream.dv_bl_signal_compatibility_id),
            Some(1)
        );

        let audio = streams.iter().find(|stream| stream.stream_type == "Audio");
        assert!(audio.is_some());
        assert_eq!(
            audio.and_then(|stream| stream.language.as_deref()),
            Some("eng")
        );
        assert_eq!(audio.and_then(|stream| stream.channels), Some(2));
        assert_eq!(audio.and_then(|stream| stream.sample_rate), Some(48_000));
        assert_eq!(
            audio.and_then(|stream| stream.channel_layout.as_deref()),
            Some("stereo")
        );
        assert_eq!(audio.and_then(|stream| stream.bit_rate), Some(192_000));

        let subtitle = streams
            .iter()
            .find(|stream| stream.stream_type == "Subtitle");
        assert!(subtitle.is_some());
        assert_eq!(
            subtitle.and_then(|stream| stream.display_title.as_deref()),
            Some("ZHO (ASS)")
        );
        assert_eq!(subtitle.and_then(|stream| stream.is_default), Some(true));
        assert_eq!(subtitle.and_then(|stream| stream.is_forced), Some(true));
    }

    #[test]
    fn parse_media_streams_from_mediainfo_supports_legacy_streams_key() {
        let mediainfo = json!([
            {
                "streams": [
                    {
                        "index": 0,
                        "codec_type": "video",
                        "codec_name": "h264"
                    },
                    {
                        "index": 1,
                        "codec_type": "audio",
                        "codec_name": "aac",
                        "channels": 2
                    }
                ]
            }
        ]);

        let streams = parse_media_streams_from_mediainfo(&mediainfo);
        assert_eq!(streams.len(), 2);
        assert!(streams.iter().any(|stream| stream.stream_type == "Video"));
        assert!(streams.iter().any(|stream| stream.stream_type == "Audio"));
    }

    #[test]
    fn parse_chapters_from_mediainfo_maps_start_ticks_and_names() {
        let mediainfo = json!([
            {
                "MediaSourceInfo": {
                    "Chapters": [
                        {
                            "id": 0,
                            "start_time": "0.000000",
                            "tags": { "title": "Intro" }
                        },
                        {
                            "id": 1,
                            "start_time": "63.500000",
                            "tags": { "title": "Act 1" }
                        },
                        {
                            "ChapterIndex": "2",
                            "StartPositionTicks": "900000000",
                            "Name": "Act 2",
                            "MarkerType": "Chapter"
                        }
                    ]
                }
            }
        ]);

        let chapters = parse_chapters_from_mediainfo(&mediainfo);
        assert_eq!(chapters.len(), 3);
        assert_eq!(chapters[0].chapter_index, Some(0));
        assert_eq!(chapters[0].start_position_ticks, 0);
        assert_eq!(chapters[0].name.as_deref(), Some("Intro"));
        assert_eq!(chapters[0].marker_type.as_deref(), Some("Chapter"));
        assert_eq!(chapters[1].chapter_index, Some(1));
        assert_eq!(chapters[1].start_position_ticks, 635_000_000);
        assert_eq!(chapters[1].name.as_deref(), Some("Act 1"));
        assert_eq!(chapters[2].chapter_index, Some(2));
        assert_eq!(chapters[2].start_position_ticks, 900_000_000);
        assert_eq!(chapters[2].name.as_deref(), Some("Act 2"));
    }

    #[test]
    fn subtitle_row_to_media_stream_sets_expected_flags() {
        let stream = subtitle_row_to_media_stream(
            7,
            SubtitleRow {
                path: "/tmp/movie.zh.default.ass".to_string(),
                language: Some("zh".to_string()),
                is_default: true,
            },
        );

        assert_eq!(stream.index, 7);
        assert_eq!(stream.stream_type, "Subtitle");
        assert!(stream.is_external);
        assert_eq!(stream.path.as_deref(), Some("/tmp/movie.zh.default.ass"));
        assert_eq!(stream.codec.as_deref(), Some("ass"));
        assert_eq!(stream.display_title.as_deref(), Some("ZH (ASS)"));
        assert_eq!(stream.is_default, Some(true));
    }

    #[test]
    fn subtitle_row_to_media_stream_infers_language_from_path_when_missing() {
        let stream = subtitle_row_to_media_stream(
            3,
            SubtitleRow {
                path: "/tmp/movie.chi.ass".to_string(),
                language: None,
                is_default: false,
            },
        );

        assert_eq!(stream.language.as_deref(), Some("zh"));
        assert_eq!(stream.display_title.as_deref(), Some("ZH (ASS)"));
    }

    #[test]
    fn index_external_subtitle_rows_uses_playback_indices_and_skips_path_duplicates() {
        let streams = vec![
            ls_domain::jellyfin::MediaStreamDto {
                index: 0,
                stream_type: "Video".to_string(),
                language: None,
                is_external: false,
                path: None,
                codec: Some("h264".to_string()),
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
                is_default: None,
                is_forced: None,
            },
            ls_domain::jellyfin::MediaStreamDto {
                index: 2,
                stream_type: "Subtitle".to_string(),
                language: Some("zh".to_string()),
                is_external: true,
                path: Some("/tmp/movie.zh.ass".to_string()),
                codec: Some("ass".to_string()),
                display_title: Some("ZH (ASS)".to_string()),
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
                is_forced: None,
            },
        ];
        let subtitles = vec![
            SubtitleRow {
                path: "/tmp/movie.zh.ass".to_string(),
                language: Some("zh".to_string()),
                is_default: true,
            },
            SubtitleRow {
                path: "/tmp/movie.en.ass".to_string(),
                language: Some("en".to_string()),
                is_default: false,
            },
        ];

        let indexed = index_external_subtitle_rows(&streams, subtitles);
        assert_eq!(indexed.len(), 1);
        assert_eq!(indexed[0].0, 3);
        assert_eq!(indexed[0].1.path, "/tmp/movie.en.ass");
    }

    #[test]
    fn append_external_subtitles_to_media_source_adds_missing_streams() {
        let mut media_source = MediaSourceInfoDto {
            id: Uuid::new_v4().to_string(),
            name: None,
            path: Some("/tmp/movie.mkv".to_string()),
            protocol: "File".to_string(),
            container: Some("mkv".to_string()),
            runtime_ticks: None,
            bitrate: None,
            supports_direct_play: true,
            supports_direct_stream: true,
            supports_transcoding: false,
            chapters: Vec::new(),
            media_streams: vec![
                MediaStreamDto {
                    index: 0,
                    stream_type: "Video".to_string(),
                    language: None,
                    is_external: false,
                    path: None,
                    codec: Some("h264".to_string()),
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
                    is_forced: None,
                },
                MediaStreamDto {
                    index: 1,
                    stream_type: "Audio".to_string(),
                    language: Some("eng".to_string()),
                    is_external: false,
                    path: None,
                    codec: Some("aac".to_string()),
                    display_title: None,
                    width: None,
                    height: None,
                    average_frame_rate: None,
                    real_frame_rate: None,
                    profile: None,
                    level: None,
                    channels: Some(2),
                    sample_rate: Some(48000),
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
                    is_forced: None,
                },
            ],
        };

        append_external_subtitles_to_media_source(
            &mut media_source,
            vec![SubtitleRow {
                path: "/tmp/movie.zh.srt".to_string(),
                language: Some("zh".to_string()),
                is_default: true,
            }],
        );

        assert_eq!(media_source.media_streams.len(), 3);
        let subtitle = &media_source.media_streams[2];
        assert_eq!(subtitle.index, 2);
        assert_eq!(subtitle.stream_type, "Subtitle");
        assert!(subtitle.is_external);
        assert_eq!(subtitle.codec.as_deref(), Some("srt"));
        assert_eq!(subtitle.display_title.as_deref(), Some("ZH (SRT)"));
    }

    #[test]
    fn extract_mediainfo_core_fields_prefers_media_source_info() {
        let mediainfo = json!([
            {
                "MediaSourceInfo": {
                    "RunTimeTicks": 72_000_000,
                    "Bitrate": "5000000",
                    "Container": "mkv"
                }
            }
        ]);

        assert_eq!(
            extract_mediainfo_runtime_ticks(&mediainfo),
            Some(72_000_000)
        );
        assert_eq!(extract_mediainfo_bitrate(&mediainfo), Some(5_000_000));
        assert_eq!(
            extract_mediainfo_container(&mediainfo).as_deref(),
            Some("mkv")
        );
    }

    #[test]
    fn normalize_media_runtime_ticks_defaults_to_zero_when_missing() {
        assert_eq!(
            normalize_media_runtime_ticks(Some(72_000_000), Some(36_000_000)),
            Some(72_000_000)
        );
        assert_eq!(
            normalize_media_runtime_ticks(None, Some(36_000_000)),
            Some(36_000_000)
        );
        assert_eq!(normalize_media_runtime_ticks(None, None), Some(0));
    }

    #[test]
    fn normalize_media_bitrate_defaults_to_zero_when_missing() {
        assert_eq!(
            normalize_media_bitrate(Some(5_000_000), Some(3_000_000)),
            Some(5_000_000)
        );
        assert_eq!(
            normalize_media_bitrate(None, Some(3_000_000)),
            Some(3_000_000)
        );
        assert_eq!(normalize_media_bitrate(None, None), Some(0));
    }

    #[test]
    fn image_candidates_returns_known_names() {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::write(temp.path().join("movie.jpg"), b"x").expect("write");
        std::fs::write(temp.path().join("poster.jpg"), b"x").expect("write");

        let candidates = image_candidates(temp.path(), "movie", "Primary");
        assert_eq!(candidates.len(), 2);
    }

    #[test]
    fn image_candidates_supports_thumb_and_backdrop_types() {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::write(temp.path().join("movie-thumb.jpg"), b"x").expect("write");
        std::fs::write(temp.path().join("fanart.jpg"), b"x").expect("write");

        let thumb = image_candidates(temp.path(), "movie", "thumb");
        let backdrop = image_candidates(temp.path(), "movie", "backdrop");

        assert_eq!(thumb.len(), 1);
        assert_eq!(backdrop.len(), 1);
    }

    #[test]
    fn image_candidates_supports_logo_type_names() {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::write(temp.path().join("logo.png"), b"x").expect("write");
        std::fs::write(temp.path().join("clearlogo.webp"), b"x").expect("write");

        let logo = image_candidates(temp.path(), "movie", "logo");
        assert_eq!(logo.len(), 2);
    }

    #[test]
    fn remove_image_candidates_deletes_primary_and_logo_files() {
        let temp = tempfile::tempdir().expect("tempdir");
        let poster = temp.path().join("movie.jpg");
        let logo = temp.path().join("clearlogo.png");
        std::fs::write(&poster, b"x").expect("write");
        std::fs::write(&logo, b"x").expect("write");

        remove_image_candidates(temp.path(), "movie", "primary");
        remove_image_candidates(temp.path(), "movie", "logo");

        assert!(!poster.exists());
        assert!(!logo.exists());
    }

    #[test]
    fn image_candidates_supports_season_poster_names() {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::write(temp.path().join("season01.jpg"), b"x").expect("write");

        let primary = image_candidates(temp.path(), "Season 01", "primary");
        assert_eq!(primary.len(), 1);
        assert_eq!(
            primary[0].file_name().and_then(|v| v.to_str()),
            Some("season01.jpg")
        );
    }

    #[test]
    fn season_number_from_stem_supports_common_names() {
        assert_eq!(season_number_from_stem("Season 01"), Some(1));
        assert_eq!(season_number_from_stem("S2"), Some(2));
        assert_eq!(season_number_from_stem("show-s03e08"), Some(3));
        assert_eq!(season_number_from_stem("Movie"), None);
    }

    #[test]
    fn media_item_image_candidates_supports_flat_series_episode_season_images() {
        let temp = tempfile::tempdir().expect("tempdir");
        let episode_path = temp.path().join("show-s01e01.strm");
        std::fs::write(&episode_path, "https://example.com/stream").expect("write strm");
        std::fs::write(temp.path().join("season01.jpg"), b"x").expect("write season poster");

        let candidates = media_item_image_candidates(&episode_path, "Episode", Some(1), "Primary");
        assert!(
            candidates
                .iter()
                .any(|path| path.file_name().and_then(|value| value.to_str())
                    == Some("season01.jpg"))
        );
    }

    #[test]
    fn media_item_image_candidates_supports_season_directory_images() {
        let temp = tempfile::tempdir().expect("tempdir");
        let season_dir = temp.path().join("Season 01");
        std::fs::create_dir_all(&season_dir).expect("create season dir");
        std::fs::write(season_dir.join("season01.jpg"), b"x").expect("write season poster");

        let candidates = media_item_image_candidates(&season_dir, "Season", Some(1), "Primary");
        assert!(
            candidates
                .iter()
                .any(|path| path.file_name().and_then(|value| value.to_str())
                    == Some("season01.jpg"))
        );
    }

    #[test]
    fn item_row_to_dto_reads_canonical_overview_and_people() {
        let row = MediaItemRow {
            id: Uuid::new_v4(),
            item_type: "Movie".to_string(),
            name: "Case Movie".to_string(),
            path: "/tmp/case-movie.strm".to_string(),
            runtime_ticks: None,
            bitrate: None,
            series_id: None,
            season_number: None,
            episode_number: None,
            library_id: None,
            metadata: json!({
                "overview": "canonical overview",
                "people": [
                    {
                        "name": "Actor A",
                        "id": "person-1",
                        "role": "Hero",
                        "type": "Actor",
                        "primary_image_tag": "tag-a"
                    }
                ]
            }),
            created_at: Utc::now(),
        };

        let dto = item_row_to_dto(row, None);
        assert_eq!(dto.overview.as_deref(), Some("canonical overview"));
        assert_eq!(dto.people.as_ref().map(Vec::len), Some(1));
        let actor = dto.people.as_ref().and_then(|people| people.first());
        assert_eq!(actor.map(|person| person.name.as_str()), Some("Actor A"));
        assert_eq!(
            actor.and_then(|person| person.primary_image_tag.as_deref()),
            Some("tag-a")
        );
    }

    #[test]
    fn item_row_to_dto_falls_back_to_nfo_overview_and_rating() {
        let row = MediaItemRow {
            id: Uuid::new_v4(),
            item_type: "Series".to_string(),
            name: "Fallback Series".to_string(),
            path: "/tmp/fallback-series".to_string(),
            runtime_ticks: None,
            bitrate: None,
            series_id: None,
            season_number: None,
            episode_number: None,
            library_id: None,
            metadata: json!({
                "nfo": {
                    "overview": "nfo overview",
                    "rating": 7.8
                }
            }),
            created_at: Utc::now(),
        };

        let dto = item_row_to_dto(row, None);
        assert_eq!(dto.overview.as_deref(), Some("nfo overview"));
        assert_eq!(dto.community_rating, Some(7.8));
    }

    #[test]
    fn item_row_to_dto_falls_back_to_nfo_official_rating() {
        let row = MediaItemRow {
            id: Uuid::new_v4(),
            item_type: "Series".to_string(),
            name: "Fallback Rating Series".to_string(),
            path: "/tmp/fallback-rating-series".to_string(),
            runtime_ticks: None,
            bitrate: None,
            series_id: None,
            season_number: None,
            episode_number: None,
            library_id: None,
            metadata: json!({
                "nfo": {
                    "mpaa": "TV-MA"
                }
            }),
            created_at: Utc::now(),
        };

        let dto = item_row_to_dto(row, None);
        assert_eq!(dto.official_rating.as_deref(), Some("TV-MA"));
    }

    #[test]
    fn item_row_to_dto_falls_back_to_nfo_year_for_production_year() {
        let row = MediaItemRow {
            id: Uuid::new_v4(),
            item_type: "Movie".to_string(),
            name: "Fallback Year Movie".to_string(),
            path: "/tmp/fallback-year.strm".to_string(),
            runtime_ticks: None,
            bitrate: None,
            series_id: None,
            season_number: None,
            episode_number: None,
            library_id: None,
            metadata: json!({
                "nfo": {
                    "year": 2019
                }
            }),
            created_at: Utc::now(),
        };

        let dto = item_row_to_dto(row, None);
        assert_eq!(dto.production_year, Some(2019));
    }

    #[test]
    fn item_row_to_dto_derives_runtime_ticks_from_tmdb_runtime_minutes() {
        let row = MediaItemRow {
            id: Uuid::new_v4(),
            item_type: "Movie".to_string(),
            name: "Runtime Movie".to_string(),
            path: "/tmp/runtime-movie.strm".to_string(),
            runtime_ticks: None,
            bitrate: None,
            series_id: None,
            season_number: None,
            episode_number: None,
            library_id: None,
            metadata: json!({
                "tmdb_raw": {
                    "runtime": 88
                }
            }),
            created_at: Utc::now(),
        };

        let dto = item_row_to_dto(row, None);
        let expected = 88_i64 * 60 * 10_000_000;
        assert_eq!(dto.runtime_ticks, Some(expected));
        assert_eq!(
            dto.media_sources
                .as_ref()
                .and_then(|sources| sources.first())
                .and_then(|source| source.runtime_ticks),
            Some(expected)
        );
    }

    #[test]
    fn item_row_to_dto_maps_tags_from_metadata() {
        let row = MediaItemRow {
            id: Uuid::new_v4(),
            item_type: "Movie".to_string(),
            name: "Tagged Movie".to_string(),
            path: "/tmp/tagged.strm".to_string(),
            runtime_ticks: None,
            bitrate: None,
            series_id: None,
            season_number: None,
            episode_number: None,
            library_id: None,
            metadata: json!({
                "tags": ["历史", "战争"]
            }),
            created_at: Utc::now(),
        };

        let dto = item_row_to_dto(row, None);
        assert_eq!(dto.tags, Some(vec!["历史".to_string(), "战争".to_string()]));
    }

    #[test]
    fn item_row_to_dto_defaults_media_source_runtime_and_bitrate_to_zero() {
        let row = MediaItemRow {
            id: Uuid::new_v4(),
            item_type: "Movie".to_string(),
            name: "Fallback Numeric Movie".to_string(),
            path: "/tmp/fallback-numeric.strm".to_string(),
            runtime_ticks: None,
            bitrate: None,
            series_id: None,
            season_number: None,
            episode_number: None,
            library_id: None,
            metadata: json!({}),
            created_at: Utc::now(),
        };

        let dto = item_row_to_dto(row, None);
        let source = dto
            .media_sources
            .as_ref()
            .and_then(|sources| sources.first())
            .expect("media source");
        assert_eq!(source.runtime_ticks, Some(0));
        assert_eq!(source.bitrate, Some(0));
    }

    #[test]
    fn item_row_to_dto_uses_strm_path_when_metadata_stream_url_missing() {
        let file_path = std::env::temp_dir().join(format!("ls-item-src-{}.strm", Uuid::new_v4()));
        std::fs::write(&file_path, "https://cdn.example.com/fallback.mp4\n")
            .expect("write strm file");

        let row = MediaItemRow {
            id: Uuid::new_v4(),
            item_type: "Movie".to_string(),
            name: "Path Fallback Movie".to_string(),
            path: file_path.to_string_lossy().to_string(),
            runtime_ticks: None,
            bitrate: None,
            series_id: None,
            season_number: None,
            episode_number: None,
            library_id: None,
            metadata: json!({}),
            created_at: Utc::now(),
        };

        let dto = item_row_to_dto(row, None);
        let media_source_path = dto
            .media_sources
            .as_ref()
            .and_then(|sources| sources.first())
            .and_then(|source| source.path.as_deref())
            .expect("media source path");
        assert_eq!(
            media_source_path,
            file_path.to_str().expect("temp path as utf-8")
        );

        std::fs::remove_file(file_path).expect("remove temp strm file");
    }

    #[test]
    fn person_row_to_dto_reads_canonical_overview() {
        let row = PersonRow {
            id: Uuid::new_v4(),
            name: "Case Person".to_string(),
            image_path: None,
            primary_image_tag: None,
            metadata: json!({
                "overview": "Person overview"
            }),
            created_at: Utc::now(),
        };

        let dto = person_row_to_dto(row);
        assert_eq!(dto.overview.as_deref(), Some("Person overview"));
    }

    #[test]
    fn item_row_to_dto_parses_canonical_people_shape() {
        let row = MediaItemRow {
            id: Uuid::new_v4(),
            item_type: "Movie".to_string(),
            name: "Alias Movie".to_string(),
            path: "/tmp/alias-movie.strm".to_string(),
            runtime_ticks: None,
            bitrate: None,
            series_id: None,
            season_number: None,
            episode_number: None,
            library_id: None,
            metadata: json!({
                "overview": "Canonical overview",
                "people": [
                    {
                        "name": "Actor Alias",
                        "id": "person-alias",
                        "type": "Actor",
                        "primary_image_tag": "alias-tag"
                    },
                    {
                        "name": "Director Alias",
                        "role": "Director"
                    }
                ]
            }),
            created_at: Utc::now(),
        };

        let dto = item_row_to_dto(row, None);
        assert_eq!(dto.overview.as_deref(), Some("Canonical overview"));
        let people = dto.people.expect("people from canonical metadata");
        assert_eq!(people.len(), 2);
        assert_eq!(people[0].name, "Actor Alias");
        assert_eq!(people[0].id.as_deref(), Some("person-alias"));
        assert_eq!(people[0].person_type.as_deref(), Some("Actor"));
        assert_eq!(people[0].primary_image_tag.as_deref(), Some("alias-tag"));
        assert_eq!(people[1].name, "Director Alias");
        assert_eq!(people[1].person_type.as_deref(), Some("Director"));
    }

    #[test]
    fn item_row_to_dto_prefers_season_parent_for_episode_payload() {
        let row = MediaItemRow {
            id: Uuid::new_v4(),
            item_type: "Episode".to_string(),
            name: "Episode Parent Fix".to_string(),
            path: "/tmp/show-s01e01.strm".to_string(),
            runtime_ticks: None,
            bitrate: None,
            series_id: Some(Uuid::new_v4()),
            season_number: Some(1),
            episode_number: Some(1),
            library_id: Some(Uuid::new_v4()),
            metadata: json!({
                "season_id": "season-fixed-id"
            }),
            created_at: Utc::now(),
        };

        let dto = item_row_to_dto(row, None);
        assert_eq!(dto.parent_id.as_deref(), Some("season-fixed-id"));
        assert_eq!(dto.season_id.as_deref(), Some("season-fixed-id"));
        assert_eq!(dto.index_number, Some(1));
        assert_eq!(dto.parent_index_number, Some(1));
    }

    #[test]
    fn item_row_to_dto_maps_season_number_to_index_number_for_season_items() {
        let series_id = Uuid::new_v4();
        let row = MediaItemRow {
            id: Uuid::new_v4(),
            item_type: "Season".to_string(),
            name: "Season 01".to_string(),
            path: "/tmp/show/Season 01".to_string(),
            runtime_ticks: None,
            bitrate: None,
            series_id: Some(series_id),
            season_number: Some(1),
            episode_number: None,
            library_id: Some(Uuid::new_v4()),
            metadata: json!({}),
            created_at: Utc::now(),
        };

        let dto = item_row_to_dto(row, None);
        assert_eq!(dto.index_number, Some(1));
        assert_eq!(dto.parent_index_number, None);
        let expected_parent = series_id.to_string();
        assert_eq!(dto.parent_id.as_deref(), Some(expected_parent.as_str()));
    }

    #[test]
    fn apply_item_people_overrides_replaces_metadata_people() {
        let item_id = Uuid::new_v4();
        let row = MediaItemRow {
            id: item_id,
            item_type: "Episode".to_string(),
            name: "Episode With Metadata People".to_string(),
            path: "/tmp/episode.strm".to_string(),
            runtime_ticks: None,
            bitrate: None,
            series_id: None,
            season_number: None,
            episode_number: None,
            library_id: None,
            metadata: json!({
                "people": [
                    {
                        "name": "Metadata Actor",
                        "id": "person-metadata",
                        "type": "Actor",
                        "primary_image_tag": "stale-tag"
                    }
                ]
            }),
            created_at: Utc::now(),
        };

        let mut items = vec![item_row_to_dto(row, None)];
        assert_eq!(
            items[0]
                .people
                .as_ref()
                .and_then(|people| people.first())
                .and_then(|person| person.primary_image_tag.as_deref()),
            Some("stale-tag")
        );

        let mut related_people = std::collections::HashMap::from([(
            item_id,
            vec![ls_domain::jellyfin::BaseItemPersonDto {
                name: "Relation Actor".to_string(),
                id: Some("person-relation".to_string()),
                role: Some("Self".to_string()),
                person_type: Some("Actor".to_string()),
                primary_image_tag: None,
            }],
        )]);

        apply_item_people_overrides(&mut items, &mut related_people);

        assert!(related_people.is_empty());
        assert_eq!(
            items[0]
                .people
                .as_ref()
                .and_then(|people| people.first())
                .map(|person| person.name.as_str()),
            Some("Relation Actor")
        );
        assert_eq!(
            items[0]
                .people
                .as_ref()
                .and_then(|people| people.first())
                .and_then(|person| person.primary_image_tag.as_deref()),
            None
        );
    }

    #[test]
    fn should_fill_tmdb_for_item_refreshes_when_people_missing_ids() {
        let metadata = json!({
            "overview": "existing",
            "tmdb_id": 1,
            "tags": ["历史"],
            "people": [{ "name": "Actor A" }],
            "primary_image_tag": "tag-a",
            "backdrop_image_tags": ["tag-b"],
            "logo_image_tag": "tag-c"
        });

        assert!(should_fill_tmdb_for_item(
            "Movie",
            "/tmp/movie.strm",
            &metadata
        ));
    }

    #[test]
    fn should_fill_tmdb_for_item_refreshes_when_images_are_missing() {
        let temp = tempfile::tempdir().expect("tempdir");
        let media_path = temp.path().join("movie.strm");
        std::fs::write(&media_path, "http://example.com/video").expect("write strm");
        let metadata = json!({
            "overview": "existing",
            "tmdb_id": 1,
            "tags": ["历史"],
            "people": [{ "id": "person-1", "name": "Actor A" }],
            "primary_image_tag": "tag-a",
            "backdrop_image_tags": ["tag-b"],
            "logo_image_tag": "tag-c"
        });

        assert!(should_fill_tmdb_for_item(
            "Movie",
            &media_path.to_string_lossy(),
            &metadata
        ));
    }

    #[test]
    fn should_fill_tmdb_for_item_skips_complete_movie_metadata() {
        let temp = tempfile::tempdir().expect("tempdir");
        let media_path = temp.path().join("movie.strm");
        std::fs::write(&media_path, "http://example.com/video").expect("write strm");
        std::fs::write(temp.path().join("movie.jpg"), b"x").expect("write poster");
        std::fs::write(temp.path().join("fanart.jpg"), b"x").expect("write fanart");
        std::fs::write(temp.path().join("logo.png"), b"x").expect("write logo");
        let metadata = json!({
            "overview": "existing",
            "tmdb_id": 1,
            "tags": ["历史"],
            "people": [{ "id": "person-1", "name": "Actor A" }],
            "primary_image_tag": "tag-a",
            "backdrop_image_tags": ["tag-b"],
            "logo_image_tag": "tag-c"
        });

        assert!(!should_fill_tmdb_for_item(
            "Movie",
            &media_path.to_string_lossy(),
            &metadata
        ));
    }

    #[test]
    fn should_fill_tmdb_for_item_when_tags_are_missing() {
        let metadata = json!({
            "overview": "existing",
            "tmdb_id": 1,
            "people": [{ "id": "person-1", "name": "Actor A" }],
            "primary_image_tag": "tag-a",
            "backdrop_image_tags": ["tag-b"],
            "logo_image_tag": "tag-c"
        });

        assert!(should_fill_tmdb_for_item(
            "Movie",
            "/tmp/movie.strm",
            &metadata
        ));
    }

    #[test]
    fn should_fill_tmdb_for_series_when_logo_file_missing() {
        let temp = tempfile::tempdir().expect("tempdir");
        let series_dir = temp.path().join("show");
        std::fs::create_dir_all(&series_dir).expect("create dir");
        std::fs::write(series_dir.join("poster.jpg"), b"x").expect("write poster");
        std::fs::write(series_dir.join("fanart.jpg"), b"x").expect("write fanart");
        let metadata = json!({
            "overview": "existing",
            "tmdb_id": 1,
            "tags": ["历史"],
            "people": [{ "id": "person-1", "name": "Actor A" }],
            "primary_image_tag": "tag-a",
            "backdrop_image_tags": ["tag-b"],
            "logo_image_tag": "tag-c"
        });

        assert!(should_fill_tmdb_for_item(
            "Series",
            &series_dir.to_string_lossy(),
            &metadata
        ));
    }

    #[test]
    fn metadata_tmdb_movie_conflicts_local_hints_detects_title_year_drift() {
        let metadata = json!({
            "tmdb_id": 1179651,
            "nfo": {
                "title": "汉密尔顿",
                "year": 2020,
                "tmdb_id": "1179651"
            },
            "tmdb_raw": {
                "title": "格式化少女",
                "original_title": "格式化少女",
                "release_date": "2016-07-07"
            }
        });
        assert!(metadata_tmdb_movie_conflicts_local_hints(&metadata));

        let missing_tmdb_year = json!({
            "tmdb_id": 1179651,
            "nfo": {
                "title": "汉密尔顿",
                "year": 2020,
                "tmdb_id": "1179651"
            },
            "tmdb_raw": {
                "title": "格式化少女",
                "original_title": "格式化少女"
            }
        });
        assert!(metadata_tmdb_movie_conflicts_local_hints(
            &missing_tmdb_year
        ));

        let matched = json!({
            "tmdb_id": 634649,
            "nfo": {
                "title": "蜘蛛侠：英雄无归",
                "year": 2021,
                "tmdb_id": "634649"
            },
            "tmdb_raw": {
                "title": "蜘蛛侠：英雄无归",
                "original_title": "Spider-Man: No Way Home",
                "release_date": "2021-12-15"
            }
        });
        assert!(!metadata_tmdb_movie_conflicts_local_hints(&matched));
    }

    #[test]
    fn metadata_tmdb_movie_conflicts_respects_manual_tmdb_binding() {
        let metadata = json!({
            "tmdb_id": 1179651,
            "tmdb_binding_source": "manual",
            "nfo": {
                "title": "汉密尔顿",
                "year": 2020,
                "tmdb_id": "1179651"
            },
            "tmdb_raw": {
                "title": "格式化少女",
                "original_title": "格式化少女",
                "release_date": "2016-07-07"
            }
        });
        assert!(!metadata_tmdb_movie_conflicts_local_hints(&metadata));
    }

    #[test]
    fn should_fill_tmdb_for_item_refreshes_when_movie_tmdb_binding_conflicts() {
        let temp = tempfile::tempdir().expect("tempdir");
        let media_path = temp.path().join("movie.strm");
        std::fs::write(&media_path, "https://example.com/movie.mp4").expect("write strm");
        std::fs::write(temp.path().join("movie.jpg"), b"x").expect("write poster");
        std::fs::write(temp.path().join("fanart.jpg"), b"x").expect("write fanart");
        std::fs::write(temp.path().join("logo.png"), b"x").expect("write logo");

        let metadata = json!({
            "overview": "existing",
            "tmdb_id": 1179651,
            "tags": ["历史"],
            "people": [{ "id": "person-1", "name": "Actor A" }],
            "primary_image_tag": "tag-a",
            "backdrop_image_tags": ["tag-b"],
            "logo_image_tag": "tag-c",
            "nfo": {
                "title": "汉密尔顿",
                "year": 2020,
                "tmdb_id": "1179651"
            },
            "tmdb_raw": {
                "title": "格式化少女",
                "original_title": "格式化少女",
                "release_date": "2016-07-07"
            }
        });
        assert!(should_fill_tmdb_for_item(
            "Movie",
            &media_path.to_string_lossy(),
            &metadata
        ));
    }

    #[test]
    fn build_movie_match_hints_prefers_nfo_and_path_over_item_name() {
        let temp = tempfile::tempdir().expect("tempdir");
        let media_path = temp.path().join("汉密尔顿 (2020) - 2160p.strm");
        std::fs::write(&media_path, "https://example.com/movie.mp4").expect("write strm");

        let item = TmdbFillItemRow {
            id: Uuid::new_v4(),
            library_id: None,
            name: "格式化少女".to_string(),
            item_type: "Movie".to_string(),
            path: media_path.to_string_lossy().to_string(),
            season_number: None,
            episode_number: None,
            metadata: json!({
                "nfo": {
                    "title": "汉密尔顿",
                    "year": 2020
                }
            }),
        };

        let hints = build_movie_match_hints(&item, &media_path);
        assert_eq!(hints.query_title, "汉密尔顿");
        assert!(
            hints
                .normalized_titles
                .iter()
                .any(|value| value == "汉密尔顿")
        );
        assert!(
            !hints
                .normalized_titles
                .iter()
                .any(|value| value == "格式化少女")
        );
    }

    #[test]
    fn tmdb_movie_release_year_prefers_release_dates_payload() {
        let details = json!({
            "release_date": "2025-09-05"
        });
        let release_dates = json!({
            "results": [
                {
                    "iso_3166_1": "US",
                    "release_dates": [{ "release_date": "2020-07-03T00:00:00.000Z" }]
                },
                {
                    "iso_3166_1": "CN",
                    "release_dates": [{ "release_date": "2025-09-05T00:00:00.000Z" }]
                }
            ]
        });
        assert_eq!(
            tmdb_movie_release_year(&details, Some(&release_dates)),
            Some(2020)
        );
    }

    #[test]
    fn tmdb_movie_official_rating_prefers_language_region_with_release_type_priority() {
        let release_dates = json!({
            "results": [
                {
                    "iso_3166_1": "US",
                    "release_dates": [
                        { "type": 4, "certification": "R" },
                        { "type": 3, "certification": "PG-13" }
                    ]
                },
                {
                    "iso_3166_1": "CN",
                    "release_dates": [
                        { "type": 3, "certification": "IIB" }
                    ]
                }
            ]
        });
        assert_eq!(
            tmdb_movie_official_rating(Some(&release_dates), "zh-CN").as_deref(),
            Some("IIB")
        );
        assert_eq!(
            tmdb_movie_official_rating(Some(&release_dates), "en-US").as_deref(),
            Some("PG-13")
        );
    }

    #[test]
    fn tmdb_tv_official_rating_prefers_language_region_and_fallback() {
        let content_ratings = json!({
            "results": [
                { "iso_3166_1": "US", "rating": "TV-MA" },
                { "iso_3166_1": "GB", "rating": "18" }
            ]
        });
        assert_eq!(
            tmdb_tv_official_rating(Some(&content_ratings), "en-GB").as_deref(),
            Some("18")
        );
        assert_eq!(
            tmdb_tv_official_rating(Some(&content_ratings), "zh-CN").as_deref(),
            Some("TV-MA")
        );
    }

    #[test]
    fn select_best_movie_search_candidate_prefers_exact_title_and_year() {
        let hints = MovieMatchHints {
            query_title: "汉密尔顿".to_string(),
            normalized_titles: vec![
                normalize_tmdb_match_title("汉密尔顿").expect("normalize local title"),
            ],
            year: Some(2020),
        };
        let candidates = vec![
            json!({
                "id": 1179651,
                "title": "格式化少女",
                "original_title": "格式化少女",
                "release_date": "2016-07-07",
                "vote_count": 0
            }),
            json!({
                "id": 556574,
                "title": "汉密尔顿",
                "original_title": "Hamilton",
                "release_date": "2020-07-03",
                "vote_count": 520
            }),
        ];
        let selected = select_best_movie_search_candidate(&hints, &candidates)
            .expect("select best search candidate");
        assert_eq!(selected.0, 556574);

        let assessment = assess_movie_match(
            &hints,
            &candidates[1],
            tmdb_movie_release_year(&candidates[1], None),
        );
        assert!(assessment.confident(&hints));
    }

    #[test]
    fn tmdb_logo_helpers_pick_preferred_logo_and_language_query() {
        let payload = json!({
            "images": {
                "logos": [
                    { "file_path": "/logo-neutral.png", "iso_639_1": null, "vote_average": 5.0 },
                    { "file_path": "/logo-en.png", "iso_639_1": "en", "vote_average": 4.0 },
                    { "file_path": "/logo-zh.png", "iso_639_1": "zh", "vote_average": 1.0 }
                ]
            }
        });

        assert_eq!(
            select_tmdb_logo_path(&payload, "zh-CN").as_deref(),
            Some("/logo-zh.png")
        );
        assert_eq!(tmdb_logo_extension("/path/to/logo.webp"), "webp");

        let include_lang = tmdb_include_image_language("zh-CN");
        assert_eq!(include_lang, "en%2Cnull%2Czh%2Czh-cn");
    }

    #[test]
    fn person_image_cache_path_uses_unified_cache_dir() {
        let path = person_image_cache_path("./cache", 12345);
        assert!(path.ends_with("cache/person-12345.jpg"));
    }

    #[test]
    fn resize_target_box_applies_bounds_and_preserves_ratio() {
        let request = ImageResizeRequest {
            width: Some(1200),
            max_width: Some(800),
            max_height: Some(600),
            ..Default::default()
        };
        let target = resize_target_box(1920, 1080, &request).expect("target box");
        assert_eq!(target, (800, 600));
    }

    #[test]
    fn libvips_size_spec_maps_resize_bounds() {
        let with_width = ImageResizeRequest {
            width: Some(640),
            ..Default::default()
        };
        assert_eq!(libvips_size_spec(&with_width).as_deref(), Some("640x"));

        let with_height = ImageResizeRequest {
            max_height: Some(360),
            ..Default::default()
        };
        assert_eq!(libvips_size_spec(&with_height).as_deref(), Some("x360"));

        let with_clamp = ImageResizeRequest {
            width: Some(2000),
            max_width: Some(1000),
            max_height: Some(700),
            ..Default::default()
        };
        assert_eq!(libvips_size_spec(&with_clamp).as_deref(), Some("1000x700"));

        assert!(libvips_size_spec(&ImageResizeRequest::default()).is_none());
    }

    #[test]
    fn resized_image_cache_key_changes_with_metadata_and_resize_params() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("source.jpg");
        std::fs::write(&source, b"abc").expect("write source");

        let source_meta = std::fs::metadata(&source).expect("metadata");
        let request_a = ImageResizeRequest {
            max_width: Some(320),
            ..Default::default()
        };
        let key_a = resized_image_cache_key(&source, &source_meta, &request_a);
        let cache_path = resized_image_cache_path("./cache", &key_a, "jpg");
        let cache_name = cache_path
            .file_name()
            .and_then(|value| value.to_str())
            .expect("cache file name");
        assert_eq!(cache_name, format!("{key_a}.jpg"));

        let request_b = ImageResizeRequest {
            max_width: Some(640),
            ..Default::default()
        };
        let key_b = resized_image_cache_key(&source, &source_meta, &request_b);
        assert_ne!(key_a, key_b);

        std::fs::write(&source, b"abcd").expect("rewrite source");
        let source_meta_new = std::fs::metadata(&source).expect("metadata");
        let key_c = resized_image_cache_key(&source, &source_meta_new, &request_a);
        assert_ne!(key_a, key_c);
    }

    #[test]
    fn resized_image_temp_path_keeps_actual_output_extension() {
        let cache_key = "abc123";
        let cache_path = resized_image_cache_path("./cache", cache_key, "jpg");
        let temp_path = resized_image_temp_path(&cache_path, cache_key, "jpg");

        let file_name = temp_path
            .file_name()
            .and_then(|value| value.to_str())
            .expect("temp file name");
        assert!(file_name.starts_with("abc123.tmp-"));
        assert!(file_name.ends_with(".jpg"));
        assert_eq!(
            temp_path.extension().and_then(|value| value.to_str()),
            Some("jpg")
        );
    }

    #[test]
    fn build_resized_image_writes_target_format_and_dimensions() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("source.png");
        let output = temp.path().join("resized.webp");

        let source_image = image::RgbaImage::from_pixel(400, 200, image::Rgba([255, 0, 0, 255]));
        source_image.save(&source).expect("save source image");

        let request = ImageResizeRequest {
            max_width: Some(120),
            quality: Some(90),
            format: Some(ImageResizeFormat::Webp),
            blur: Some(10),
            ..Default::default()
        };

        build_resized_image(&source, &output, &request, ImageResizeFormat::Webp)
            .expect("build resized image");
        let resized = image::open(&output).expect("open resized");
        assert!(resized.width() <= 120);
        assert!(resized.height() <= 200);
    }

    #[test]
    fn resolve_person_image_path_from_cache_hints_prefers_existing_raw_path() {
        let temp = tempfile::tempdir().expect("tempdir");
        let image_path = temp.path().join("person-local.jpg");
        std::fs::write(&image_path, b"x").expect("write person image");

        let resolved = resolve_person_image_path_from_cache_hints(
            image_path.to_string_lossy().as_ref(),
            temp.path().to_string_lossy().as_ref(),
        );
        assert_eq!(
            resolved.as_deref(),
            Some(image_path.to_string_lossy().as_ref())
        );
    }

    #[test]
    fn resolve_person_image_path_from_cache_hints_falls_back_to_cache_dir_filename() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cache_dir = temp.path().join("cache");
        std::fs::create_dir_all(&cache_dir).expect("create cache dir");
        let image_path = cache_dir.join("person-42.jpg");
        std::fs::write(&image_path, b"x").expect("write person image");

        let resolved = resolve_person_image_path_from_cache_hints(
            "./legacy/person-42.jpg",
            cache_dir.to_string_lossy().as_ref(),
        );
        assert_eq!(
            resolved.as_deref(),
            Some(image_path.to_string_lossy().as_ref())
        );
    }

    #[test]
    fn person_primary_image_tag_for_response_uses_fallback_when_missing_image() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cache_dir = temp.path().join("cache");
        std::fs::create_dir_all(&cache_dir).expect("create cache dir");
        let cache_dir_string = cache_dir.to_string_lossy().to_string();
        let person_id = Uuid::new_v4();

        let cached = person_image_cache_path(&cache_dir_string, 42);
        std::fs::write(&cached, b"x").expect("write cached person image");

        let local_image = temp.path().join("person-local.jpg");
        std::fs::write(&local_image, b"x").expect("write local image");

        assert_eq!(
            person_primary_image_tag_for_response(
                person_id,
                Some("tag-local".to_string()),
                Some(local_image.to_string_lossy().to_string()),
                None,
                &cache_dir_string,
            ),
            Some("tag-local".to_string())
        );
        assert_eq!(
            person_primary_image_tag_for_response(
                person_id,
                Some("tag-cache".to_string()),
                None,
                Some(42),
                &cache_dir_string,
            ),
            Some("tag-cache".to_string())
        );
        let fallback_missing = person_primary_image_tag_for_response(
            person_id,
            Some(" ".to_string()),
            Some(
                temp.path()
                    .join("missing.jpg")
                    .to_string_lossy()
                    .to_string(),
            ),
            Some(999),
            &cache_dir_string,
        );
        assert!(fallback_missing.is_some());

        let fallback_local = person_primary_image_tag_for_response(
            person_id,
            None,
            Some(local_image.to_string_lossy().to_string()),
            None,
            &cache_dir_string,
        );
        assert!(fallback_local.is_some());
        assert_ne!(fallback_missing, fallback_local);
    }

    #[test]
    fn playback_payload_helpers_extract_item_position_and_played_state() {
        let item_id = Uuid::new_v4();
        let payload: PlaybackProgressDto = serde_json::from_value(json!({
            "playSessionId": "ps-1",
            "itemId": item_id.to_string(),
            "playbackPositionTicks": 9_000,
            "nowPlayingItem": {
                "runTimeTicks": 10_000
            }
        }))
        .expect("playback payload parse");

        assert_eq!(
            extract_playback_item_id(&payload),
            Some(item_id.to_string())
        );
        assert_eq!(extract_playback_position_ticks(&payload), 9_000);
        assert!(infer_playback_played_flag("stopped", &payload, 9_600));
        assert!(!infer_playback_played_flag("stopped", &payload, 2_000));
    }

    #[test]
    fn playback_payload_helpers_accept_numeric_item_id() {
        let payload: PlaybackProgressDto = serde_json::from_value(json!({
            "ItemId": 123456789i64,
            "PositionTicks": 200
        }))
        .expect("playback payload parse");

        assert_eq!(
            extract_playback_item_id(&payload),
            Some("123456789".to_string())
        );
        assert_eq!(extract_playback_position_ticks(&payload), 200);
    }

    #[test]
    fn playback_payload_played_hint_overrides_inference() {
        let payload: PlaybackProgressDto = serde_json::from_value(json!({
            "ItemId": Uuid::new_v4().to_string(),
            "PositionTicks": 100,
            "Played": true
        }))
        .expect("playback payload parse");

        assert!(infer_playback_played_flag("progress", &payload, 100));
    }

    #[test]
    fn library_image_candidates_support_primary_and_backdrop_names() {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::write(temp.path().join("folder.jpg"), b"x").expect("write");
        std::fs::write(temp.path().join("fanart.png"), b"x").expect("write");

        let primary = library_image_candidates(temp.path(), "Primary");
        let backdrop = library_image_candidates(temp.path(), "Backdrop");

        assert_eq!(primary.len(), 1);
        assert_eq!(backdrop.len(), 1);
    }

    #[test]
    fn library_image_cache_path_targets_cache_library_images_dir() {
        let library_id = Uuid::new_v4();
        let source = std::path::Path::new("/media/library/folder.JPG");
        let path = library_image_cache_path("./cache", library_id, "Primary", 0, source);
        assert!(path.ends_with(format!("cache/library-images/{library_id}/primary-0.jpg")));
    }

    #[test]
    fn library_image_upload_helpers_normalize_type_and_extension() {
        assert_eq!(library_image_target_basename("Primary"), "folder");
        assert_eq!(library_image_target_basename("Backdrop"), "fanart");
        assert_eq!(library_image_target_basename("UnknownType"), "cover");

        assert_eq!(
            library_image_base_names("primary"),
            &["folder", "poster", "cover", "thumb"]
        );
        assert_eq!(library_image_base_names("fanart"), &["fanart", "backdrop"]);

        assert_eq!(normalize_image_extension("jpeg"), "jpg");
        assert_eq!(normalize_image_extension("PNG"), "png");
        assert_eq!(normalize_image_extension("bad-ext"), "jpg");
    }

    #[test]
    fn root_collection_type_normalizes_common_aliases() {
        assert_eq!(normalize_emby_collection_type("Movie"), "movies");
        assert_eq!(normalize_emby_collection_type("movies"), "movies");
        assert_eq!(normalize_emby_collection_type("Series"), "tvshows");
        assert_eq!(normalize_emby_collection_type("tvshows"), "tvshows");
        assert_eq!(normalize_emby_collection_type("playlists"), "playlists");
        assert_eq!(normalize_emby_collection_type("Mixed"), "mixed");
        assert_eq!(normalize_emby_collection_type("Unknown"), "unknown");
    }

    #[test]
    fn root_library_primary_image_tag_uses_primary_cover_candidates() {
        let temp = tempfile::tempdir().expect("tempdir");
        assert!(root_library_primary_image_tag(temp.path().to_str().unwrap_or_default()).is_none());

        std::fs::write(temp.path().join("folder.jpg"), b"x").expect("write folder image");
        let tag = root_library_primary_image_tag(temp.path().to_str().unwrap_or_default());
        assert!(tag.is_some());
    }

    #[test]
    fn merge_missing_json_preserves_existing_and_fills_blanks() {
        let base = json!({
            "overview": "local",
            "genres": ["Drama"],
            "nested": {
                "a": "keep",
                "b": ""
            }
        });
        let patch = json!({
            "overview": "remote",
            "genres": ["Action"],
            "tmdb_id": 100,
            "nested": {
                "b": "fill",
                "c": "new"
            }
        });

        let merged = merge_missing_json(base, &patch);
        assert_eq!(merged["overview"], "local");
        assert_eq!(merged["genres"], json!(["Drama"]));
        assert_eq!(merged["tmdb_id"], 100);
        assert_eq!(merged["nested"]["a"], "keep");
        assert_eq!(merged["nested"]["b"], "fill");
        assert_eq!(merged["nested"]["c"], "new");
    }

    #[test]
    fn extract_tmdb_people_limits_cast_and_includes_crew_roles() {
        let cast = (0..25)
            .map(|idx| {
                json!({
                    "id": idx + 1,
                    "name": format!("Actor-{idx}"),
                    "character": format!("Role-{idx}")
                })
            })
            .collect::<Vec<_>>();
        let credits = json!({
            "cast": cast,
            "crew": [
                { "id": 9001, "name": "Crew-Director", "job": "Director" },
                { "id": 9002, "name": "Crew-Writer", "job": "Writer" },
                { "id": 9003, "name": "Crew-Editor", "job": "Editor" }
            ]
        });

        let people = extract_tmdb_people(Some(&credits), 20);
        let actor_count = people.iter().filter(|p| p.person_type == "Actor").count();
        assert_eq!(actor_count, 20);
        assert!(people.iter().any(|p| p.person_type == "Director"));
        assert!(people.iter().any(|p| p.person_type == "Writer"));
        assert!(!people.iter().any(|p| p.name == "Crew-Editor"));
    }

    #[test]
    fn extract_tmdb_keywords_supports_movie_and_tv_shapes() {
        let movie_payload = json!({
            "keywords": [
                {"id": 1, "name": "历史"},
                {"id": 2, "name": "战争"},
                {"id": 3, "name": "历史"}
            ]
        });
        let tv_payload = json!({
            "results": [
                {"id": 11, "name": "Mystery"},
                {"id": 12, "name": "mystery"}
            ]
        });

        assert_eq!(
            extract_tmdb_keywords(&movie_payload),
            vec!["历史".to_string(), "战争".to_string()]
        );
        assert_eq!(
            extract_tmdb_keywords(&tv_payload),
            vec!["Mystery".to_string()]
        );
    }

    #[test]
    fn merge_tmdb_tags_into_metadata_unions_without_duplicates() {
        let mut metadata = json!({
            "tags": ["历史", "战争"]
        });
        merge_tmdb_tags_into_metadata(
            &mut metadata,
            &[
                "剧情".to_string(),
                "战争".to_string(),
                "历史".to_string(),
                "  ".to_string(),
            ],
        );
        assert_eq!(metadata["tags"], json!(["历史", "战争", "剧情"]));
    }

    #[test]
    fn resolve_season_nfo_path_prefers_season_nfo_then_variants() {
        let temp = tempfile::tempdir().expect("tempdir");
        let season_dir = temp.path();

        let fallback = resolve_season_nfo_path(season_dir, 1);
        assert_eq!(fallback, season_dir.join("season.nfo"));

        let numbered = season_dir.join("season01.nfo");
        std::fs::write(&numbered, "<season/>").expect("write season01");
        let path = resolve_season_nfo_path(season_dir, 1);
        assert_eq!(path, numbered);

        let preferred = season_dir.join("season.nfo");
        std::fs::write(&preferred, "<season/>").expect("write season");
        let path = resolve_season_nfo_path(season_dir, 1);
        assert_eq!(path, preferred);
    }

    #[test]
    fn resolve_scan_scope_path_supports_relative_subpath() {
        let temp = tempfile::tempdir().expect("tempdir");
        let scope = temp.path().join("show").join("season-1");
        std::fs::create_dir_all(&scope).expect("create nested dir");
        let roots = vec![temp.path().to_string_lossy().to_string()];

        let resolved = resolve_scan_scope_path(&roots, Some("show/season-1"))
            .expect("resolve ok")
            .expect("scope");

        assert!(resolved.ends_with("show/season-1"));
    }

    #[test]
    fn resolve_scan_scope_path_rejects_path_escape() {
        let temp = tempfile::tempdir().expect("tempdir");
        let outside = tempfile::tempdir().expect("outside");
        let roots = vec![temp.path().to_string_lossy().to_string()];

        let result = resolve_scan_scope_path(&roots, Some(&outside.path().to_string_lossy()));

        assert!(result.is_err());
    }

    #[test]
    fn resolve_scan_scope_path_handles_none_and_absolute_inside_root() {
        let temp = tempfile::tempdir().expect("tempdir");
        let nested = temp.path().join("series");
        std::fs::create_dir_all(&nested).expect("mkdir");
        let roots = vec![temp.path().to_string_lossy().to_string()];

        let none_scope = resolve_scan_scope_path(&roots, None).expect("resolve none scope");
        assert!(none_scope.is_none());

        let abs_scope = resolve_scan_scope_path(&roots, Some(&nested.to_string_lossy()))
            .expect("resolve absolute scope")
            .expect("scope");
        assert_eq!(
            abs_scope,
            std::fs::canonicalize(&nested).expect("canonical nested")
        );
    }

    #[test]
    fn resolve_scan_scope_path_rejects_relative_scope_for_multi_roots() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root_a = temp.path().join("a");
        let root_b = temp.path().join("b");
        std::fs::create_dir_all(root_a.join("series")).expect("create a");
        std::fs::create_dir_all(root_b.join("series")).expect("create b");
        let roots = vec![
            root_a.to_string_lossy().to_string(),
            root_b.to_string_lossy().to_string(),
        ];

        let result = resolve_scan_scope_path(&roots, Some("series"));
        assert!(result.is_err());
    }

    #[test]
    fn migrate_legacy_default_library_paths_converts_single_path_field() {
        let mut payload = json!({
            "scan": {
                "default_library_path": " /mnt/media "
            }
        });

        let changed = migrate_legacy_default_library_paths(&mut payload);
        assert!(changed);
        assert_eq!(
            payload["scan"]["default_library_paths"],
            json!(["/mnt/media"])
        );
        assert!(payload["scan"].get("default_library_path").is_none());
    }

    #[test]
    fn migrate_legacy_default_library_paths_drops_stale_legacy_key() {
        let mut payload = json!({
            "scan": {
                "default_library_paths": ["/mnt/a", "/mnt/b"],
                "default_library_path": "/mnt/legacy"
            }
        });

        let changed = migrate_legacy_default_library_paths(&mut payload);
        assert!(changed);
        assert_eq!(
            payload["scan"]["default_library_paths"],
            json!(["/mnt/a", "/mnt/b"])
        );
        assert!(payload["scan"].get("default_library_path").is_none());
    }

    #[test]
    fn tmdb_cache_key_normalizes_case_and_whitespace() {
        let key = tmdb_cache_key("movie", "  Avatar 2  ", "ZH-CN");
        assert_eq!(key, "movie:zh-cn:avatar 2");
    }

    #[test]
    fn scrape_fill_search_index_since_prefers_new_since() {
        let started_at = Utc
            .with_ymd_and_hms(2026, 3, 1, 12, 0, 0)
            .single()
            .expect("valid start timestamp");
        let new_since = Utc
            .with_ymd_and_hms(2026, 3, 1, 11, 30, 0)
            .single()
            .expect("valid new_since timestamp");

        assert_eq!(
            scrape_fill_search_index_since(started_at, Some(new_since)),
            new_since
        );
    }

    #[test]
    fn scrape_fill_search_index_since_falls_back_to_started_at() {
        let started_at = Utc
            .with_ymd_and_hms(2026, 3, 1, 12, 0, 0)
            .single()
            .expect("valid start timestamp");

        assert_eq!(scrape_fill_search_index_since(started_at, None), started_at);
    }

    #[test]
    fn calc_retry_delay_seconds_applies_exponential_backoff_with_cap() {
        assert_eq!(calc_retry_delay_seconds(15, 900, 0), 15);
        assert_eq!(calc_retry_delay_seconds(15, 900, 1), 30);
        assert_eq!(calc_retry_delay_seconds(15, 900, 2), 60);
        assert_eq!(calc_retry_delay_seconds(15, 90, 10), 90);
        assert_eq!(calc_retry_delay_seconds(0, 10, -1), 1);
    }

    #[test]
    fn distributed_offset_is_stable_and_bounded() {
        let first = distributed_offset("gdrive://movie/A", 3);
        let second = distributed_offset("gdrive://movie/A", 3);
        assert_eq!(first, second);
        assert!(first < 3);
        assert_eq!(distributed_offset("any", 1), 0);
    }

    #[test]
    fn playlist_visibility_allows_owner_and_public_list() {
        let owner = Uuid::parse_str("00000000-0000-0000-0000-000000000101").expect("owner uuid");
        let viewer = Uuid::parse_str("00000000-0000-0000-0000-000000000202").expect("viewer uuid");
        assert!(playlist_is_visible_to(owner, owner, false));
        assert!(playlist_is_visible_to(viewer, owner, true));
        assert!(!playlist_is_visible_to(viewer, owner, false));
    }

    #[test]
    fn default_favorites_playlist_labels_match_web_copy() {
        assert_eq!(DEFAULT_FAVORITES_PLAYLIST_NAME, "我的喜欢");
        assert_eq!(DEFAULT_FAVORITES_PLAYLIST_DESCRIPTION, "默认收藏夹");
    }

    #[test]
    fn parse_lumenbackend_reference_supports_explicit_and_default_route() {
        let (route, path) = parse_lumenbackend_reference("cdn/library/movie.mkv", "gdrive");
        assert_eq!(route, "cdn");
        assert_eq!(path, "library/movie.mkv");

        let (route, path) = parse_lumenbackend_reference("library/movie.mkv", "gdrive");
        assert_eq!(route, "gdrive");
        assert_eq!(path, "library/movie.mkv");
    }

    #[test]
    fn parse_lumenbackend_http_stream_url_extracts_route_and_decodes_path() {
        let url = "https://lumenbackend.example.com/v1/streams/gdrive?path=LumenStream%20Media/movie.mkv&api_key=demo";
        let (route, path) = parse_lumenbackend_http_stream_url(url).expect("parse http stream url");
        assert_eq!(route, "v1/streams/gdrive");
        assert_eq!(path, "LumenStream Media/movie.mkv");

        let url = "https://lumenbackend.example.com/GDRIVE?path=LumenStream+Media%2Fmovie.mkv";
        let (route, path) = parse_lumenbackend_http_stream_url(url).expect("parse http stream url");
        assert_eq!(route, "gdrive");
        assert_eq!(path, "LumenStream Media/movie.mkv");
    }

    #[test]
    fn parse_lumenbackend_http_stream_url_rejects_unknown_route_or_missing_path() {
        assert!(parse_lumenbackend_http_stream_url("https://example.com/unknown?path=a").is_none());
        assert!(parse_lumenbackend_http_stream_url("https://example.com/gdrive").is_none());
        assert!(parse_lumenbackend_http_stream_url("https://example.com/gdrive?path=").is_none());
    }

    #[test]
    fn normalize_lumenbackend_base_url_adds_scheme_and_trims_slash() {
        assert_eq!(
            normalize_lumenbackend_base_url("test-server-cdn.lumenstream-team.org/"),
            Some("https://test-server-cdn.lumenstream-team.org".to_string())
        );
        assert_eq!(
            normalize_lumenbackend_base_url("localhost:8080"),
            Some("http://localhost:8080".to_string())
        );
        assert_eq!(
            normalize_lumenbackend_base_url("[::1]:8090"),
            Some("http://[::1]:8090".to_string())
        );
        assert_eq!(
            normalize_lumenbackend_base_url("https://example.com/"),
            Some("https://example.com".to_string())
        );
        assert_eq!(
            normalize_lumenbackend_base_url("//example.com/edge/"),
            Some("https://example.com/edge".to_string())
        );
        assert_eq!(normalize_lumenbackend_base_url("   "), None);
    }

    #[test]
    fn select_lumenbackend_stream_base_url_prefers_playback_domain_and_normalizes() {
        let nodes = vec!["node-a.example.com".to_string()];

        let base_url =
            select_lumenbackend_stream_base_url(Some("test-server-cdn.lumenstream-team.org"), &nodes)
                .expect("base url");
        assert_eq!(base_url, "https://test-server-cdn.lumenstream-team.org");

        let base_url = select_lumenbackend_stream_base_url(None, &nodes).expect("base url");
        assert_eq!(base_url, "https://node-a.example.com");
    }

    #[test]
    fn select_lumenbackend_stream_base_url_falls_back_when_playback_domain_invalid() {
        let nodes = vec!["localhost:8090".to_string()];

        let base_url =
            select_lumenbackend_stream_base_url(Some(" "), &nodes).expect("fallback base url");
        assert_eq!(base_url, "http://localhost:8090");
    }

    #[test]
    fn normalize_lumenbackend_stream_route_maps_cdn_alias_to_default() {
        assert_eq!(
            normalize_lumenbackend_stream_route("cdn", "v1/streams/gdrive"),
            "v1/streams/gdrive"
        );
        assert_eq!(
            normalize_lumenbackend_stream_route("v1/streams/cdn", "v1/streams/gdrive"),
            "v1/streams/gdrive"
        );
        assert_eq!(
            normalize_lumenbackend_stream_route("gdrive", "v1/streams/gdrive"),
            "gdrive"
        );
    }

    #[test]
    fn normalize_policy_default_helpers_handle_negative_values() {
        assert_eq!(normalize_traffic_window_days(0), 1);
        assert_eq!(normalize_traffic_window_days(30), 30);

        assert_eq!(normalize_default_optional_i32(-1), None);
        assert_eq!(normalize_default_optional_i32(0), Some(0));
        assert_eq!(normalize_default_optional_i32(5), Some(5));

        assert_eq!(normalize_default_optional_i64(-1), None);
        assert_eq!(normalize_default_optional_i64(0), Some(0));
        assert_eq!(normalize_default_optional_i64(1024), Some(1024));
    }

    #[test]
    fn runtime_bootstrap_validation_rejects_missing_credentials_without_admin_user() {
        let err = validate_runtime_bootstrap_credentials(false, &AuthConfig::default())
            .expect_err("missing bootstrap credentials should fail when no admin exists");
        assert!(
            err.to_string()
                .contains("auth.bootstrap_admin_user is required")
        );
    }

    #[test]
    fn runtime_bootstrap_validation_rejects_empty_password_without_admin_user() {
        let auth = AuthConfig {
            bootstrap_admin_user: "admin".to_string(),
            bootstrap_admin_password: " ".to_string(),
            ..AuthConfig::default()
        };

        let err = validate_runtime_bootstrap_credentials(false, &auth)
            .expect_err("empty bootstrap password should fail when no admin exists");
        assert!(
            err.to_string()
                .contains("auth.bootstrap_admin_password is required")
        );
    }

    #[test]
    fn runtime_bootstrap_validation_rejects_legacy_default_without_admin_user() {
        let auth = AuthConfig {
            bootstrap_admin_user: "admin".to_string(),
            bootstrap_admin_password: "admin123".to_string(),
            ..AuthConfig::default()
        };

        let err = validate_runtime_bootstrap_credentials(false, &auth)
            .expect_err("legacy default bootstrap credentials should fail");
        assert!(
            err.to_string()
                .contains("auth.bootstrap_admin_password cannot use legacy default credentials")
        );
    }

    #[test]
    fn runtime_bootstrap_validation_allows_missing_credentials_with_existing_admin() {
        assert!(validate_runtime_bootstrap_credentials(true, &AuthConfig::default()).is_ok());
    }

    #[test]
    fn build_lumenbackend_stream_token_is_stable_for_fixed_input() {
        let now = Utc
            .timestamp_opt(1_736_476_800, 0)
            .single()
            .expect("valid timestamp");

        let token = build_lumenbackend_stream_token(
            "secret-key",
            "v1/streams/gdrive",
            "library/show/movie.mkv",
            86_400,
            now,
        )
        .expect("token");
        let second = build_lumenbackend_stream_token(
            "secret-key",
            "v1/streams/gdrive",
            "library/show/movie.mkv",
            86_400,
            now,
        )
        .expect("token");

        assert_eq!(token, second);

        let mut parts = token.splitn(2, '.');
        let exp = parts.next().expect("exp").parse::<i64>().expect("exp int");
        let signature = parts.next().expect("signature");

        assert_eq!(exp, now.timestamp() + 86_400);
        assert!(!signature.is_empty());
        assert!(build_lumenbackend_stream_token("", "gdrive", "a/b", 60, now).is_none());
    }

    #[test]
    fn meili_quote_escapes_backslash_and_double_quote() {
        assert_eq!(meili_quote(r#"A\"B"#), r#""A\\\"B""#);
    }

    #[test]
    fn build_meili_filter_builds_expected_clauses() {
        let series = Uuid::parse_str("00000000-0000-0000-0000-000000000111").expect("uuid");
        let parent = Uuid::parse_str("00000000-0000-0000-0000-000000000222").expect("uuid");
        let options = ItemsQuery {
            series_filter: Some(series),
            parent_id: Some(parent),
            include_item_types: vec!["Movie".to_string(), "Series".to_string()],
            ..ItemsQuery::default()
        };

        let filter = build_meili_filter(&options).expect("filter");
        assert_eq!(
            filter,
            r#"item_type IN ["Movie", "Series"] AND series_id = "00000000-0000-0000-0000-000000000111" AND (library_id = "00000000-0000-0000-0000-000000000222" OR series_id = "00000000-0000-0000-0000-000000000222")"#
        );
    }

    #[test]
    fn build_meili_filter_returns_none_for_empty_options() {
        assert!(build_meili_filter(&ItemsQuery::default()).is_none());
    }

    #[test]
    fn parent_scope_sql_clause_library_non_recursive_excludes_season_and_episode() {
        let parent_id = Uuid::parse_str("00000000-0000-0000-0000-000000000333").expect("uuid");
        let clause = AppInfra::parent_scope_sql_clause(
            ParentQueryScope::Library {
                parent_id,
                recursive: false,
            },
            1,
        );
        assert_eq!(
            clause.condition,
            "library_id = $1 AND item_type != 'Season' AND item_type != 'Episode'"
        );
        assert_eq!(clause.binds.len(), 1);
        assert!(matches!(clause.binds[0], ParentScopeSqlBind::Uuid(v) if v == parent_id));
    }

    #[test]
    fn parent_scope_sql_clause_series_and_season_match_expected_binds() {
        let series_id = Uuid::parse_str("00000000-0000-0000-0000-000000000444").expect("uuid");
        let series_clause = AppInfra::parent_scope_sql_clause(
            ParentQueryScope::Series {
                series_id,
                recursive: false,
            },
            2,
        );
        assert_eq!(
            series_clause.condition,
            "series_id = $2 AND item_type = 'Season'"
        );
        assert_eq!(series_clause.binds.len(), 1);
        assert!(matches!(series_clause.binds[0], ParentScopeSqlBind::Uuid(v) if v == series_id));

        let season_clause = AppInfra::parent_scope_sql_clause(
            ParentQueryScope::Season {
                series_id,
                season_number: 3,
            },
            4,
        );
        assert_eq!(
            season_clause.condition,
            "item_type = 'Episode' AND series_id = $4 AND season_number = $5"
        );
        assert_eq!(season_clause.binds.len(), 2);
        assert!(matches!(
            season_clause.binds[0],
            ParentScopeSqlBind::Uuid(v) if v == series_id
        ));
        assert!(matches!(season_clause.binds[1], ParentScopeSqlBind::I32(3)));
    }

    #[test]
    fn parent_scope_prefers_season_episode_sort_for_series_and_season_only() {
        let library_scope = ParentQueryScope::Library {
            parent_id: Uuid::new_v4(),
            recursive: true,
        };
        assert!(!AppInfra::parent_scope_prefers_season_episode_sort(Some(
            library_scope
        )));
        assert!(!AppInfra::parent_scope_prefers_season_episode_sort(None));

        let series_scope = ParentQueryScope::Series {
            series_id: Uuid::new_v4(),
            recursive: true,
        };
        assert!(AppInfra::parent_scope_prefers_season_episode_sort(Some(
            series_scope
        )));

        let season_scope = ParentQueryScope::Season {
            series_id: Uuid::new_v4(),
            season_number: 2,
        };
        assert!(AppInfra::parent_scope_prefers_season_episode_sort(Some(
            season_scope
        )));
    }

    #[test]
    fn sort_name_order_expression_uses_dedicated_column() {
        assert_eq!(
            AppInfra::sort_name_order_expression(),
            "COALESCE(sort_name, name)"
        );
    }

    #[test]
    fn normalize_items_sort_field_supports_series_and_episode_fields() {
        let series_sort = AppInfra::normalize_items_sort_field("SeriesSortName");
        let parent_index = AppInfra::normalize_items_sort_field("ParentIndexNumber");
        let index_number = AppInfra::normalize_items_sort_field("IndexNumber");
        let date_last_content_added =
            AppInfra::normalize_items_sort_field("DateLastContentAdded");
        let fallback_name = AppInfra::normalize_items_sort_field("name");

        assert_eq!(
            AppInfra::normalize_items_sort_field("seriessortname"),
            series_sort
        );
        assert_eq!(
            AppInfra::normalize_items_sort_field("parentindexnumber"),
            parent_index
        );
        assert_eq!(
            AppInfra::normalize_items_sort_field("indexnumber"),
            index_number
        );
        assert_eq!(
            AppInfra::normalize_items_sort_field("datelastcontentadded"),
            date_last_content_added
        );
        assert_eq!(
            AppInfra::normalize_items_sort_field("unknown-sort"),
            fallback_name
        );
        assert_ne!(series_sort, fallback_name);
        assert_ne!(parent_index, fallback_name);
        assert_ne!(index_number, fallback_name);
        assert_ne!(date_last_content_added, fallback_name);
        assert_eq!(
            date_last_content_added,
            AppInfra::normalize_items_sort_field("DateCreated")
        );
    }

    #[test]
    fn requires_watch_state_join_only_when_user_and_filters_present() {
        let with_user_and_favorite = ItemsQuery {
            user_id: Some(Uuid::new_v4()),
            is_favorite: Some(true),
            ..ItemsQuery::default()
        };
        assert!(AppInfra::requires_watch_state_join(&with_user_and_favorite));
        assert!(!AppInfra::requires_watch_state_match_without_user(
            &with_user_and_favorite
        ));

        let without_user_and_false_favorite = ItemsQuery {
            is_favorite: Some(false),
            ..ItemsQuery::default()
        };
        assert!(!AppInfra::requires_watch_state_join(
            &without_user_and_false_favorite
        ));
        assert!(!AppInfra::requires_watch_state_match_without_user(
            &without_user_and_false_favorite
        ));

        let without_user_and_true_favorite = ItemsQuery {
            is_favorite: Some(true),
            ..ItemsQuery::default()
        };
        assert!(AppInfra::requires_watch_state_match_without_user(
            &without_user_and_true_favorite
        ));
    }

    #[test]
    fn requires_items_compat_fallback_only_when_parent_and_episode_or_season_requested() {
        let parent_id = Uuid::new_v4();
        let season_options = ItemsQuery {
            parent_id: Some(parent_id),
            include_item_types: vec!["Season".to_string()],
            ..ItemsQuery::default()
        };
        assert!(AppInfra::requires_items_compat_fallback(&season_options));

        let episode_options = ItemsQuery {
            parent_id: Some(parent_id),
            include_item_types: vec!["Episode".to_string()],
            ..ItemsQuery::default()
        };
        assert!(AppInfra::requires_items_compat_fallback(&episode_options));

        let normal_options = ItemsQuery {
            include_item_types: vec!["Episode".to_string()],
            ..ItemsQuery::default()
        };
        assert!(!AppInfra::requires_items_compat_fallback(&normal_options));
    }

    #[test]
    fn can_sql_paginate_items_query_only_for_non_fallback_first_page_queries() {
        let options = ItemsQuery {
            start_index: 0,
            include_item_types: vec!["Episode".to_string()],
            ..ItemsQuery::default()
        };
        assert!(AppInfra::can_sql_paginate_items_query(&options, None, &[]));

        let paged_options = ItemsQuery {
            start_index: 50,
            include_item_types: vec!["Episode".to_string()],
            ..ItemsQuery::default()
        };
        assert!(AppInfra::can_sql_paginate_items_query(&paged_options, None, &[]));

        let fallback_options = ItemsQuery {
            parent_id: Some(Uuid::new_v4()),
            include_item_types: vec!["Episode".to_string()],
            ..ItemsQuery::default()
        };
        assert!(!AppInfra::can_sql_paginate_items_query(
            &fallback_options,
            None,
            &[]
        ));

        let meili_options = ItemsQuery {
            include_item_types: vec!["Episode".to_string()],
            ..ItemsQuery::default()
        };
        let meili_ids = vec![Uuid::new_v4()];
        assert!(!AppInfra::can_sql_paginate_items_query(
            &meili_options,
            Some(&meili_ids),
            &[]
        ));
    }

    #[test]
    fn items_query_normalize_deduplicates_person_ids() {
        let person_a = Uuid::parse_str("00000000-0000-0000-0000-0000000000aa").expect("uuid");
        let person_b = Uuid::parse_str("00000000-0000-0000-0000-0000000000bb").expect("uuid");
        let options = ItemsQuery {
            person_ids: vec![person_a, person_b, person_a],
            tags: vec![
                "历史".to_string(),
                "  历史 ".to_string(),
                "战争".to_string(),
            ],
            ..ItemsQuery::default()
        };

        let normalized = options.normalize();
        assert_eq!(normalized.person_ids, vec![person_a, person_b]);
        assert_eq!(
            normalized.tags,
            vec!["历史".to_string(), "战争".to_string()]
        );
    }

    #[test]
    fn mask_secret_fields_masks_nested_sensitive_values() {
        let input = json!({
            "api_key": "abc",
            "nested": {
                "password": "pwd",
                "token": "tok"
            },
            "array": [
                { "secret_key": "xyz" },
                { "normal": "ok" }
            ]
        });

        let masked = mask_secret_fields(input);
        assert_eq!(masked["api_key"], "***");
        assert_eq!(masked["nested"]["password"], "***");
        assert_eq!(masked["nested"]["token"], "***");
        assert_eq!(masked["array"][0]["secret_key"], "***");
        assert_eq!(masked["array"][1]["normal"], "ok");
    }

    #[test]
    fn mask_secret_fields_masks_dsn_values() {
        let input = json!({
            "mysql": { "dsn": "mysql://user:pwd@127.0.0.1:3306/db" },
            "redis": { "dsn": "redis://localhost:6379/0" }
        });
        let masked = mask_secret_fields(input);
        assert_eq!(masked["mysql"]["dsn"], "***");
        assert_eq!(masked["redis"]["dsn"], "***");
    }

    #[test]
    fn merge_secret_placeholders_keeps_existing_secret_value() {
        let current = json!({
            "mysql": { "dsn": "mysql://old" },
            "stream_token": { "signing_key": "old-key" }
        });
        let incoming = json!({
            "mysql": { "dsn": "***" },
            "stream_token": { "signing_key": "***" }
        });
        let merged = merge_secret_placeholders(incoming, &current);
        assert_eq!(merged["mysql"]["dsn"], "mysql://old");
        assert_eq!(merged["stream_token"]["signing_key"], "old-key");
    }

    #[test]
    fn merge_json_values_overrides_and_preserves_unknown_fields() {
        let mut base = json!({
            "mysql": { "dsn": "" },
            "redis": { "dsn": "" },
            "stream_route": "v1/streams/gdrive",
        });
        let overlay = json!({
            "mysql": { "dsn": "mysql://demo" },
            "extra": { "enabled": true }
        });
        merge_json_values(&mut base, &overlay);
        assert_eq!(base["mysql"]["dsn"], "mysql://demo");
        assert_eq!(base["redis"]["dsn"], "");
        assert_eq!(base["extra"]["enabled"], true);
    }

    #[test]
    fn strip_node_runtime_protected_fields_removes_system_managed_entries() {
        let mut cfg = json!({
            "mysql": { "dsn": "mysql://demo" },
            "stream_route": "custom-route",
            "stream_token": { "enabled": true, "signing_key": "demo" },
            "playback_domains": [{ "id": "domain-a" }]
        });
        strip_node_runtime_protected_fields(&mut cfg);
        assert!(cfg.get("stream_route").is_none());
        assert!(cfg.get("stream_token").is_none());
        assert!(cfg.get("playback_domains").is_none());
        assert_eq!(cfg["mysql"]["dsn"], "mysql://demo");
    }

    #[test]
    fn apply_global_lumenbackend_stream_fields_overrides_existing_values() {
        let mut cfg = json!({
            "stream_route": "node-custom",
            "stream_token": {
                "enabled": false,
                "signing_key": "",
                "max_age_sec": 15
            }
        });
        apply_global_lumenbackend_stream_fields(&mut cfg, "v1/streams/gdrive", "global-secret", 86_400);
        assert_eq!(cfg["stream_route"], "v1/streams/gdrive");
        assert_eq!(cfg["stream_token"]["enabled"], true);
        assert_eq!(cfg["stream_token"]["signing_key"], "global-secret");
        assert_eq!(cfg["stream_token"]["max_age_sec"], 86_400);
    }

    #[test]
    fn parse_runtime_schema_fields_rejects_invalid_schema() {
        let schema = json!({
            "sections": [
                {
                    "id": "basic",
                    "title": "Basic",
                    "fields": [
                        {
                            "key": "server.listen_port",
                            "type": "invalid"
                        }
                    ]
                }
            ]
        });

        assert!(parse_runtime_schema_fields(&schema).is_err());
    }

    #[test]
    fn validate_runtime_config_against_schema_rejects_undeclared_keys() {
        let schema = json!({
            "sections": [
                {
                    "id": "basic",
                    "title": "Basic",
                    "fields": [
                        {
                            "key": "server.listen_port",
                            "type": "number",
                            "required": true,
                            "validators": { "min": 1, "max": 65535 }
                        }
                    ]
                }
            ]
        });
        let fields = parse_runtime_schema_fields(&schema).expect("schema fields");

        let payload = json!({
            "server": { "listen_port": 8080 },
            "extra": { "key": true }
        });
        assert!(validate_runtime_config_against_schema(&payload, &fields).is_err());
    }

    #[test]
    fn build_schema_default_runtime_config_writes_nested_defaults() {
        let schema = json!({
            "sections": [
                {
                    "id": "basic",
                    "title": "Basic",
                    "fields": [
                        {
                            "key": "server.listen_host",
                            "type": "string",
                            "default": "0.0.0.0"
                        },
                        {
                            "key": "server.listen_port",
                            "type": "number",
                            "default": 8080
                        }
                    ]
                }
            ]
        });
        let fields = parse_runtime_schema_fields(&schema).expect("schema fields");
        let defaults = build_schema_default_runtime_config(&fields);

        assert_eq!(defaults["server"]["listen_host"], "0.0.0.0");
        assert_eq!(defaults["server"]["listen_port"], 8080);
    }

    #[tokio::test]
    async fn search_ids_from_meili_returns_search_unavailable_when_backend_missing() {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@127.0.0.1:5432/lumenstream")
            .expect("create lazy postgres pool");
        let (notification_tx, _) = tokio::sync::broadcast::channel(16);
        let (task_run_tx, _) = tokio::sync::broadcast::channel(16);
        let (recharge_order_tx, _) = tokio::sync::broadcast::channel(16);
        let (agent_request_tx, _) = tokio::sync::broadcast::channel(16);
        let infra = AppInfra {
            pool,
            config: Arc::new(RwLock::new(AppConfig::default())),
            server_id: "test-server".to_string(),
            http_client: reqwest::Client::new(),
            search_backend: None,
            metrics: Arc::new(super::InfraMetrics::default()),
            tmdb_last_request: Arc::new(Mutex::new(None)),
            resized_image_locks: Arc::new(Mutex::new(std::collections::HashMap::new())),
            notification_tx,
            task_run_tx,
            recharge_order_tx,
            agent_request_tx,
        };

        let err = infra
            .search_ids_from_meili("demo", &ItemsQuery::default())
            .await
            .expect_err("search should fail without backend");
        assert!(err.chain().any(
            |cause| cause.downcast_ref::<InfraError>() == Some(&InfraError::SearchUnavailable)
        ));
    }

    #[tokio::test]
    async fn apply_runtime_web_config_updates_config_snapshot() {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@127.0.0.1:5432/lumenstream")
            .expect("create lazy postgres pool");
        let (notification_tx, _) = tokio::sync::broadcast::channel(16);
        let (task_run_tx, _) = tokio::sync::broadcast::channel(16);
        let (recharge_order_tx, _) = tokio::sync::broadcast::channel(16);
        let (agent_request_tx, _) = tokio::sync::broadcast::channel(16);
        let infra = AppInfra {
            pool,
            config: Arc::new(RwLock::new(AppConfig::default())),
            server_id: "test-server".to_string(),
            http_client: reqwest::Client::new(),
            search_backend: None,
            metrics: Arc::new(super::InfraMetrics::default()),
            tmdb_last_request: Arc::new(Mutex::new(None)),
            resized_image_locks: Arc::new(Mutex::new(std::collections::HashMap::new())),
            notification_tx,
            task_run_tx,
            recharge_order_tx,
            agent_request_tx,
        };

        let mut web = infra.config_snapshot().web_config();
        web.tmdb.enabled = true;
        web.tmdb.api_key = "hot-reload-token".to_string();
        web.storage.prefer_segment_gateway = true;

        infra.apply_runtime_web_config(&web);
        let updated = infra.config_snapshot();
        assert!(updated.tmdb.enabled);
        assert_eq!(updated.tmdb.api_key, "hot-reload-token");
        assert!(updated.storage.prefer_segment_gateway);
    }

    #[test]
    fn write_movie_nfo_includes_studio_and_rating() {
        let temp = tempfile::tempdir().expect("tempdir");
        let nfo_path = temp.path().join("movie.nfo");
        let metadata = json!({
            "sort_name": "Test Movie",
            "overview": "A plot",
            "tmdb_id": 42,
            "official_rating": "PG-13",
            "community_rating": 7.5,
            "genres": ["Drama"],
            "studios": [{ "name": "Studio A" }, { "name": "Studio B" }],
        });
        write_movie_nfo(&nfo_path, &metadata).expect("write nfo");
        let content = std::fs::read_to_string(&nfo_path).expect("read nfo");
        assert!(content.contains("<studio>Studio A</studio>"));
        assert!(content.contains("<studio>Studio B</studio>"));
        assert!(content.contains("<rating>7.5</rating>"));
        assert!(content.contains("<mpaa>PG-13</mpaa>"));
        assert!(content.contains("<genre>Drama</genre>"));
    }

    #[test]
    fn write_movie_nfo_prefers_metadata_tmdb_id_over_existing_file_value() {
        let temp = tempfile::tempdir().expect("tempdir");
        let nfo_path = temp.path().join("movie.nfo");
        std::fs::write(
            &nfo_path,
            "<movie><title>Old</title><tmdbid>111</tmdbid></movie>",
        )
        .expect("write existing nfo");

        let metadata = json!({
            "sort_name": "Old",
            "tmdb_id": 222
        });
        write_movie_nfo(&nfo_path, &metadata).expect("write nfo");
        let content = std::fs::read_to_string(&nfo_path).expect("read nfo");
        assert!(content.contains("<tmdbid>222</tmdbid>"));
        assert!(!content.contains("<tmdbid>111</tmdbid>"));
        assert!(content.contains("<uniqueid type=\"tmdb\" default=\"true\">222</uniqueid>"));
    }

    #[test]
    fn write_tvshow_nfo_includes_genre_studio_rating() {
        let temp = tempfile::tempdir().expect("tempdir");
        let nfo_path = temp.path().join("tvshow.nfo");
        let metadata = json!({
            "series_name": "Test Show",
            "overview": "A plot",
            "tmdb_id": 99,
            "official_rating": "TV-14",
            "community_rating": 8.2,
            "genres": ["Sci-Fi", "Action"],
            "studios": [{ "name": "Net Co" }],
        });
        write_tvshow_nfo(&nfo_path, &metadata).expect("write nfo");
        let content = std::fs::read_to_string(&nfo_path).expect("read nfo");
        assert!(content.contains("<genre>Sci-Fi</genre>"));
        assert!(content.contains("<genre>Action</genre>"));
        assert!(content.contains("<studio>Net Co</studio>"));
        assert!(content.contains("<rating>8.2</rating>"));
        assert!(content.contains("<mpaa>TV-14</mpaa>"));
    }

    #[test]
    fn write_episode_nfo_includes_genre() {
        let temp = tempfile::tempdir().expect("tempdir");
        let nfo_path = temp.path().join("ep.nfo");
        let metadata = json!({
            "title": "Pilot",
            "genres": ["Comedy"],
        });
        write_episode_nfo(&nfo_path, &metadata, Some(1), Some(1)).expect("write nfo");
        let content = std::fs::read_to_string(&nfo_path).expect("read nfo");
        assert!(content.contains("<genre>Comedy</genre>"));
    }

    #[test]
    fn merge_missing_json_fills_nested_nfo_title() {
        let base = json!({ "overview": "existing" });
        let patch = json!({ "nfo": { "title": "Episode Title" } });
        let merged = merge_missing_json(base, &patch);
        assert_eq!(merged["nfo"]["title"], "Episode Title");

        // Existing nfo.title should not be overwritten
        let base2 = json!({ "nfo": { "title": "Original" } });
        let merged2 = merge_missing_json(base2, &patch);
        assert_eq!(merged2["nfo"]["title"], "Original");
    }

    #[test]
    fn extract_nfo_studios_handles_object_and_string_entries() {
        let meta = json!({ "studios": [{ "name": "A" }, "B"] });
        let studios = extract_nfo_studios(&meta);
        assert_eq!(studios, vec!["A".to_string(), "B".to_string()]);

        let empty = json!({});
        assert!(extract_nfo_studios(&empty).is_empty());
    }

    // ── NFO tmdb_id / imdb_id reading tests ──

    #[test]
    fn read_nfo_tmdb_id_extracts_valid_id() {
        let dir = std::env::temp_dir().join(format!("ls-nfo-tmdb-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let nfo = dir.join("movie.nfo");
        std::fs::write(
            &nfo,
            "<movie>\n  <title>Test</title>\n  <tmdbid>12345</tmdbid>\n</movie>",
        )
        .unwrap();

        assert_eq!(read_nfo_tmdb_id(&nfo), Some(12345));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn read_nfo_tmdb_id_returns_none_for_missing_file() {
        let path = std::path::Path::new("/tmp/nonexistent-nfo-file.nfo");
        assert_eq!(read_nfo_tmdb_id(path), None);
    }

    #[test]
    fn read_nfo_tmdb_id_rejects_zero_and_negative() {
        let dir = std::env::temp_dir().join(format!("ls-nfo-zero-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let nfo = dir.join("movie.nfo");

        std::fs::write(&nfo, "<movie><tmdbid>0</tmdbid></movie>").unwrap();
        assert_eq!(read_nfo_tmdb_id(&nfo), None);

        std::fs::write(&nfo, "<movie><tmdbid>-1</tmdbid></movie>").unwrap();
        assert_eq!(read_nfo_tmdb_id(&nfo), None);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn read_nfo_tmdb_id_ignores_non_numeric() {
        let dir = std::env::temp_dir().join(format!("ls-nfo-nan-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let nfo = dir.join("movie.nfo");
        std::fs::write(&nfo, "<movie><tmdbid>abc</tmdbid></movie>").unwrap();

        assert_eq!(read_nfo_tmdb_id(&nfo), None);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn read_nfo_tmdb_id_prefers_uniqueid_and_ignores_actor_tmdbid() {
        let dir = std::env::temp_dir().join(format!("ls-nfo-tmdb-unique-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let nfo = dir.join("tvshow.nfo");
        std::fs::write(
            &nfo,
            r#"<tvshow>
  <actor><name>A</name><tmdbid>1816461</tmdbid></actor>
  <uniqueid type="tmdb">272681</uniqueid>
  <tmdbid>999999</tmdbid>
</tvshow>"#,
        )
        .unwrap();

        assert_eq!(read_nfo_tmdb_id(&nfo), Some(272681));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn read_nfo_tmdb_id_ignores_actor_tmdbid_when_root_tmdbid_exists() {
        let dir = std::env::temp_dir().join(format!("ls-nfo-tmdb-actor-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let nfo = dir.join("tvshow.nfo");
        std::fs::write(
            &nfo,
            r#"<tvshow>
  <actor><name>A</name><tmdbid>1816461</tmdbid></actor>
  <tmdbid>272681</tmdbid>
</tvshow>"#,
        )
        .unwrap();

        assert_eq!(read_nfo_tmdb_id(&nfo), Some(272681));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn read_nfo_imdb_id_extracts_valid_tt_id() {
        let dir = std::env::temp_dir().join(format!("ls-nfo-imdb-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let nfo = dir.join("movie.nfo");
        std::fs::write(&nfo, "<movie><imdbid>tt1234567</imdbid></movie>").unwrap();

        assert_eq!(read_nfo_imdb_id(&nfo), Some("tt1234567".to_string()));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn read_nfo_imdb_id_prefers_uniqueid_and_ignores_actor_imdbid() {
        let dir = std::env::temp_dir().join(format!("ls-nfo-imdb-unique-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let nfo = dir.join("tvshow.nfo");
        std::fs::write(
            &nfo,
            r#"<tvshow>
  <actor><name>A</name><imdbid>nm8361677</imdbid></actor>
  <uniqueid type="imdb">tt32515825</uniqueid>
  <imdbid>nm1111111</imdbid>
</tvshow>"#,
        )
        .unwrap();

        assert_eq!(read_nfo_imdb_id(&nfo), Some("tt32515825".to_string()));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn read_nfo_imdb_id_ignores_actor_imdbid_when_root_imdbid_exists() {
        let dir = std::env::temp_dir().join(format!("ls-nfo-imdb-actor-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let nfo = dir.join("tvshow.nfo");
        std::fs::write(
            &nfo,
            r#"<tvshow>
  <actor><name>A</name><imdbid>nm8361677</imdbid></actor>
  <imdbid>tt1234567</imdbid>
</tvshow>"#,
        )
        .unwrap();

        assert_eq!(read_nfo_imdb_id(&nfo), Some("tt1234567".to_string()));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn read_nfo_imdb_id_rejects_non_tt_prefix() {
        let dir = std::env::temp_dir().join(format!("ls-nfo-imdb-bad-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let nfo = dir.join("movie.nfo");
        std::fs::write(&nfo, "<movie><imdbid>1234567</imdbid></movie>").unwrap();

        assert_eq!(read_nfo_imdb_id(&nfo), None);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn read_nfo_imdb_id_tries_imdb_id_tag_fallback() {
        let dir = std::env::temp_dir().join(format!("ls-nfo-imdb-alt-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let nfo = dir.join("movie.nfo");
        std::fs::write(&nfo, "<movie><imdb_id>tt9999999</imdb_id></movie>").unwrap();

        assert_eq!(read_nfo_imdb_id(&nfo), Some("tt9999999".to_string()));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn resolve_movie_nfo_tmdb_id_checks_stem_then_movie_nfo() {
        let dir = std::env::temp_dir().join(format!("ls-movie-nfo-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        let media = dir.join("My Movie (2024).mkv");
        // No NFO yet → None
        assert_eq!(resolve_movie_nfo_tmdb_id(&media), None);

        // Write movie.nfo in same dir
        std::fs::write(
            dir.join("movie.nfo"),
            "<movie><tmdbid>55555</tmdbid></movie>",
        )
        .unwrap();
        assert_eq!(resolve_movie_nfo_tmdb_id(&media), Some(55555));

        // Stem-based NFO takes priority
        std::fs::write(
            dir.join("My Movie (2024).nfo"),
            "<movie><tmdbid>66666</tmdbid></movie>",
        )
        .unwrap();
        assert_eq!(resolve_movie_nfo_tmdb_id(&media), Some(66666));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn resolve_series_nfo_tmdb_id_checks_tvshow_nfo_variants() {
        let dir = std::env::temp_dir().join(format!("ls-series-nfo-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        // No NFO → None
        assert_eq!(resolve_series_nfo_tmdb_id(&dir), None);

        // show.nfo
        std::fs::write(
            dir.join("show.nfo"),
            "<tvshow><tmdbid>77777</tmdbid></tvshow>",
        )
        .unwrap();
        assert_eq!(resolve_series_nfo_tmdb_id(&dir), Some(77777));

        // tvshow.nfo takes priority (checked first)
        std::fs::write(
            dir.join("tvshow.nfo"),
            "<tvshow><tmdbid>88888</tmdbid></tvshow>",
        )
        .unwrap();
        assert_eq!(resolve_series_nfo_tmdb_id(&dir), Some(88888));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn resolve_movie_nfo_imdb_id_from_stem_nfo() {
        let dir = std::env::temp_dir().join(format!("ls-movie-imdb-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let media = dir.join("film.mkv");
        std::fs::write(
            dir.join("film.nfo"),
            "<movie><imdbid>tt0000001</imdbid></movie>",
        )
        .unwrap();

        assert_eq!(
            resolve_movie_nfo_imdb_id(&media),
            Some("tt0000001".to_string())
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn resolve_series_nfo_imdb_id_from_tvshow_nfo() {
        let dir = std::env::temp_dir().join(format!("ls-series-imdb-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("tvshow.nfo"),
            "<tvshow><imdbid>tt0000002</imdbid></tvshow>",
        )
        .unwrap();

        assert_eq!(
            resolve_series_nfo_imdb_id(&dir),
            Some("tt0000002".to_string())
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn group_person_media_rows_groups_by_person_in_input_order() {
        let p1 = Uuid::new_v4();
        let p2 = Uuid::new_v4();
        let m1 = Uuid::new_v4();
        let m2 = Uuid::new_v4();
        let m3 = Uuid::new_v4();
        let rows = vec![(p1, m1), (p1, m2), (p2, m3)];
        let exclude = std::collections::HashSet::new();

        let result = group_person_media_rows(&rows, &[p2, p1], &exclude, 10);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, p2);
        assert_eq!(result[0].1, vec![m3]);
        assert_eq!(result[1].0, p1);
        assert_eq!(result[1].1, vec![m1, m2]);
    }

    #[test]
    fn group_person_media_rows_excludes_ids_and_respects_limit() {
        let p1 = Uuid::new_v4();
        let m1 = Uuid::new_v4();
        let m2 = Uuid::new_v4();
        let m3 = Uuid::new_v4();
        let rows = vec![(p1, m1), (p1, m2), (p1, m3)];
        let exclude: std::collections::HashSet<Uuid> = [m2].into_iter().collect();

        let result = group_person_media_rows(&rows, &[p1], &exclude, 1);
        assert_eq!(result[0].1, vec![m1]);
    }

    #[test]
    fn group_person_media_rows_person_with_no_media() {
        let p1 = Uuid::new_v4();
        let exclude = std::collections::HashSet::new();

        let result = group_person_media_rows(&[], &[p1], &exclude, 10);
        assert_eq!(result.len(), 1);
        assert!(result[0].1.is_empty());
    }

    #[test]
    fn expand_ids_with_person_media_injects_after_person() {
        let p1 = Uuid::new_v4();
        let media_direct = Uuid::new_v4();
        let assoc1 = Uuid::new_v4();
        let assoc2 = Uuid::new_v4();

        let all_ids = vec![media_direct, p1];
        let person_assoc = vec![(p1, vec![assoc1, assoc2])];

        let expanded = expand_ids_with_person_media(&all_ids, &person_assoc);
        assert_eq!(expanded, vec![media_direct, p1, assoc1, assoc2]);
    }

    #[test]
    fn expand_ids_with_person_media_deduplicates() {
        let p1 = Uuid::new_v4();
        let shared = Uuid::new_v4();
        let assoc1 = Uuid::new_v4();

        // shared appears both as direct media and as person's associated media
        let all_ids = vec![shared, p1];
        let person_assoc = vec![(p1, vec![shared, assoc1])];

        let expanded = expand_ids_with_person_media(&all_ids, &person_assoc);
        // shared should NOT appear twice
        assert_eq!(expanded, vec![shared, p1, assoc1]);
    }

    #[test]
    fn expand_ids_no_person_assoc_returns_original() {
        let m1 = Uuid::new_v4();
        let m2 = Uuid::new_v4();
        let all_ids = vec![m1, m2];

        let expanded = expand_ids_with_person_media(&all_ids, &[]);
        assert_eq!(expanded, vec![m1, m2]);
    }
}
