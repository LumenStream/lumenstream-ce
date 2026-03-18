fn to_user_dto(row: &UserRow, server_id: &str) -> UserDto {
    let role = UserRole::from_db(&row.role);
    let can_use = !row.is_disabled;
    let is_administrator = matches!(role, UserRole::Admin) || row.is_admin;
    UserDto {
        id: row.id.to_string(),
        name: row.username.clone(),
        has_password: !row.password_hash.is_empty(),
        has_configured_password: !row.password_hash.is_empty(),
        has_configured_easy_password: false,
        enable_auto_login: false,
        server_id: server_id.to_string(),
        server_name: Some("lumenstream".to_string()),
        connect_user_name: None,
        connect_link_type: None,
        primary_image_tag: None,
        last_login_date: None,
        last_activity_date: None,
        configuration: Some(default_user_configuration()),
        primary_image_aspect_ratio: None,
        policy: UserPolicyDto {
            is_administrator,
            is_hidden: false,
            is_hidden_remotely: false,
            is_hidden_from_unused_devices: false,
            is_disabled: row.is_disabled,
            locked_out_date: 0,
            allow_tag_or_rating: false,
            blocked_tags: Vec::new(),
            is_tag_blocking_mode_inclusive: false,
            include_tags: Vec::new(),
            enable_user_preference_access: can_use,
            access_schedules: Vec::new(),
            block_unrated_items: Vec::new(),
            enable_remote_control_of_other_users: false,
            enable_shared_device_control: true,
            enable_remote_access: can_use,
            enable_live_tv_management: can_use,
            enable_live_tv_access: can_use,
            enable_media_playback: can_use,
            enable_audio_playback_transcoding: can_use,
            enable_video_playback_transcoding: can_use,
            enable_playback_remuxing: can_use,
            enable_content_deletion: can_use && is_administrator,
            restricted_features: Vec::new(),
            enable_content_deletion_from_folders: Vec::new(),
            enable_content_downloading: can_use,
            enable_subtitle_downloading: can_use,
            enable_subtitle_management: can_use && is_administrator,
            enable_sync_transcoding: can_use,
            enable_media_conversion: can_use,
            enabled_channels: Vec::new(),
            enable_all_channels: true,
            enabled_folders: Vec::new(),
            enable_all_folders: true,
            invalid_login_attempt_count: 0,
            enable_public_sharing: can_use,
            remote_client_bitrate_limit: 0,
            authentication_provider_id:
                "Emby.Server.Implementations.Library.DefaultAuthenticationProvider".to_string(),
            excluded_sub_folders: Vec::new(),
            simultaneous_stream_limit: 0,
            enabled_devices: Vec::new(),
            enable_all_devices: true,
            allow_camera_upload: can_use,
            allow_sharing_personal_items: false,
            role: Some(role.as_str().to_string()),
        },
    }
}

fn default_user_configuration() -> Value {
    json!({
        "PlayDefaultAudioTrack": true,
        "DisplayMissingEpisodes": false,
        "SubtitleMode": "Smart",
        "OrderedViews": [],
        "LatestItemsExcludes": [],
        "MyMediaExcludes": [],
        "HidePlayedInLatest": true,
        "HidePlayedInMoreLikeThis": false,
        "HidePlayedInSuggestions": false,
        "RememberAudioSelections": true,
        "RememberSubtitleSelections": true,
        "EnableNextEpisodeAutoPlay": true,
        "ResumeRewindSeconds": 0,
        "IntroSkipMode": "ShowButton",
        "EnableLocalPassword": false,
    })
}

fn format_emby_datetime(value: DateTime<Utc>) -> String {
    let ticks_fraction = value.timestamp_subsec_nanos() / 100;
    format!(
        "{}.{ticks_fraction:07}Z",
        value.format("%Y-%m-%dT%H:%M:%S")
    )
}

fn compare_option_string(left: Option<&str>, right: Option<&str>) -> CmpOrdering {
    match (left, right) {
        (Some(a), Some(b)) => a.cmp(b),
        (Some(_), None) => CmpOrdering::Greater,
        (None, Some(_)) => CmpOrdering::Less,
        (None, None) => CmpOrdering::Equal,
    }
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn empty_user_profile(user_id: Uuid) -> UserProfile {
    let now = Utc::now();
    UserProfile {
        user_id,
        email: None,
        display_name: None,
        remark: None,
        created_at: now,
        updated_at: now,
    }
}

fn item_type_is_folder(item_type: &str) -> bool {
    matches!(
        item_type,
        "CollectionFolder" | "Folder" | "Series" | "Season" | "MusicAlbum" | "MusicArtist"
    )
}

fn infer_media_type(item_type: &str) -> Option<&'static str> {
    if matches!(
        item_type,
        "Movie" | "Episode" | "Video" | "MusicVideo" | "Trailer"
    ) {
        return Some("Video");
    }
    if matches!(
        item_type,
        "Song" | "Audio" | "AudioBook" | "MusicAlbum" | "MusicArtist"
    ) {
        return Some("Audio");
    }
    if matches!(item_type, "Photo" | "PhotoAlbum") {
        return Some("Photo");
    }
    None
}

fn infer_media_source_container(path_or_url: &str, metadata: &Value) -> Option<String> {
    if let Some(container) = metadata_string_by_aliases(metadata, &["container", "Container"]) {
        return Some(container.to_ascii_lowercase());
    }

    std::path::Path::new(path_or_url)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::trim)
        .filter(|ext| !ext.is_empty())
        .map(|ext| ext.to_ascii_lowercase())
}

#[cfg(test)]
fn infer_stream_url_from_strm_path(path: &str) -> Option<String> {
    let extension = std::path::Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::trim)
        .unwrap_or_default();
    if !extension.eq_ignore_ascii_case("strm") {
        return None;
    }

    let content = std::fs::read_to_string(path).ok()?;
    content
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .filter(|line| {
            line.starts_with("http://")
                || line.starts_with("https://")
                || line.starts_with("gdrive://")
                || line.starts_with("s3://")
                || line.starts_with("lumenbackend://")
                || line.starts_with("local://")
        })
        .map(str::to_string)
}

fn metadata_value_by_aliases<'a>(metadata: &'a Value, aliases: &[&str]) -> Option<&'a Value> {
    aliases.iter().find_map(|alias| metadata.get(*alias))
}

fn metadata_string_by_aliases(metadata: &Value, aliases: &[&str]) -> Option<String> {
    let value = metadata_value_by_aliases(metadata, aliases)?;
    if let Some(text) = value.as_str().map(str::trim).filter(|v| !v.is_empty()) {
        return Some(text.to_string());
    }
    if let Some(number) = value.as_i64() {
        return Some(number.to_string());
    }
    if let Some(number) = value.as_u64() {
        return Some(number.to_string());
    }
    None
}

fn metadata_string_array_by_aliases(metadata: &Value, aliases: &[&str]) -> Option<Vec<String>> {
    metadata_value_by_aliases(metadata, aliases).and_then(|value| {
        value.as_array().and_then(|arr| {
            let values = arr
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>();
            if values.is_empty() {
                None
            } else {
                Some(values)
            }
        })
    })
}

fn value_as_i32(value: &Value) -> Option<i32> {
    value
        .as_i64()
        .and_then(|raw| i32::try_from(raw).ok())
        .or_else(|| value.as_u64().and_then(|raw| i32::try_from(raw).ok()))
        .or_else(|| value.as_str().and_then(|raw| raw.trim().parse::<i32>().ok()))
}

const TICKS_PER_SECOND: i64 = 10_000_000;
const TICKS_PER_MINUTE: i64 = 60 * TICKS_PER_SECOND;

fn value_as_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|raw| i64::try_from(raw).ok()))
        .or_else(|| {
            value.as_f64().and_then(|raw| {
                if !raw.is_finite() || raw < 0.0 {
                    return None;
                }
                if raw > (i64::MAX as f64) {
                    return None;
                }
                Some(raw as i64)
            })
        })
        .or_else(|| value.as_str().and_then(|raw| raw.trim().parse::<i64>().ok()))
}

fn derive_tmdb_runtime_minutes(tmdb_raw: &Value) -> Option<i64> {
    if let Some(runtime) = tmdb_raw
        .get("runtime")
        .and_then(value_as_i64)
        .filter(|value| *value > 0)
    {
        return Some(runtime);
    }

    let select_first_positive = |value: &Value| {
        value.as_array().and_then(|arr| {
            arr.iter()
                .filter_map(value_as_i64)
                .find(|minutes| *minutes > 0)
        })
    };

    if let Some(minutes) = tmdb_raw
        .get("episode_run_time")
        .and_then(select_first_positive)
    {
        return Some(minutes);
    }

    tmdb_raw
        .get("tv")
        .and_then(|tv| tv.get("episode_run_time"))
        .and_then(select_first_positive)
}

fn derive_runtime_ticks_from_metadata(metadata: &Value) -> Option<i64> {
    // Some pipelines store runtime directly in ticks.
    if let Some(ticks) = metadata_value_by_aliases(
        metadata,
        &["runtime_ticks", "run_time_ticks", "RunTimeTicks", "runTimeTicks"],
    )
    .and_then(value_as_i64)
    .filter(|value| *value > 0)
    {
        return Some(ticks);
    }

    // NFO runtime is usually minutes.
    if let Some(minutes) = metadata
        .get("nfo")
        .and_then(|nfo| metadata_value_by_aliases(nfo, &["runtime", "Runtime"]))
        .and_then(value_as_i64)
        .filter(|value| *value > 0)
    {
        return Some(minutes.saturating_mul(TICKS_PER_MINUTE));
    }

    // TMDB runtime is also minutes.
    metadata
        .get("tmdb_raw")
        .and_then(derive_tmdb_runtime_minutes)
        .filter(|value| *value > 0)
        .map(|minutes| minutes.saturating_mul(TICKS_PER_MINUTE))
}

fn parse_year_prefix_from_date(raw: &str) -> Option<i32> {
    let year = raw
        .trim()
        .split(|ch| ['-', '/', '.'].contains(&ch))
        .next()
        .unwrap_or_default();
    if year.len() != 4 {
        return None;
    }
    year.parse::<i32>().ok()
}

fn person_row_to_dto(row: PersonRow) -> BaseItemDto {
    use std::collections::HashMap;
    let primary_image_tag = row.primary_image_tag;

    BaseItemDto {
        id: row.id.to_string(),
        name: row.name,
        item_type: "Person".to_string(),
        path: row.image_path.unwrap_or_default(),
        is_folder: Some(false),
        media_type: None,
        container: None,
        location_type: Some("FileSystem".to_string()),
        can_delete: Some(false),
        can_download: Some(false),
        collection_type: None,
        runtime_ticks: None,
        bitrate: None,
        media_sources: None,
        user_data: None,
        overview: metadata_string_by_aliases(&row.metadata, &["overview"]),
        premiere_date: None,
        end_date: None,
        production_year: None,
        genres: None,
        tags: None,
        provider_ids: metadata_string_by_aliases(&row.metadata, &["tmdb_id"])
            .map(|tmdb_id| HashMap::from([("Tmdb".to_string(), tmdb_id)])),
        image_tags: primary_image_tag
            .clone()
            .map(|tag| HashMap::from([("Primary".to_string(), tag)])),
        primary_image_tag,
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
        date_created: Some(row.created_at.to_rfc3339()),
        child_count: None,
        recursive_item_count: None,
        play_access: None,
    }
}

fn item_row_to_dto(row: MediaItemRow, user_data: Option<UserDataDto>) -> BaseItemDto {
    use ls_domain::jellyfin::{BaseItemPersonDto, MediaSourceInfoDto, NameGuidPairDto};
    use std::collections::HashMap;

    let meta = &row.metadata;
    let item_type = row.item_type.clone();
    let is_folder = item_type_is_folder(item_type.as_str());
    let media_type = infer_media_type(item_type.as_str()).map(str::to_string);
    let location_type = if row.path.trim().is_empty() {
        None
    } else {
        Some("FileSystem".to_string())
    };
    let media_source_path = media_source_path_from_row(&row.path, None, meta);
    let mediainfo = meta.get("mediainfo").unwrap_or(&Value::Null);
    let parsed_media_streams = parse_media_streams_from_mediainfo(mediainfo);
    let parsed_chapters = parse_chapters_from_mediainfo(mediainfo);
    let mediainfo_runtime_ticks = extract_mediainfo_runtime_ticks(mediainfo);
    let mediainfo_bitrate = extract_mediainfo_bitrate(mediainfo);
    let runtime_ticks = row
        .runtime_ticks
        .or(mediainfo_runtime_ticks)
        .or_else(|| derive_runtime_ticks_from_metadata(meta));
    let bitrate = row.bitrate.or(mediainfo_bitrate);
    let media_source_runtime_ticks =
        normalize_media_runtime_ticks(runtime_ticks, mediainfo_runtime_ticks);
    let media_source_bitrate = normalize_media_bitrate(bitrate, mediainfo_bitrate);
    let container = infer_media_source_container(&media_source_path, meta)
        .or_else(|| extract_mediainfo_container(mediainfo));
    let media_sources = if !is_folder && media_type.is_some() {
        let protocol =
            if media_source_path.starts_with("http://") || media_source_path.starts_with("https://")
            {
                "Http".to_string()
            } else {
                "File".to_string()
            };
        Some(vec![MediaSourceInfoDto {
            id: row.id.to_string(),
            name: None,
            path: Some(media_source_path.clone()),
            protocol,
            container: container.clone(),
            runtime_ticks: media_source_runtime_ticks,
            bitrate: media_source_bitrate,
            supports_direct_play: true,
            supports_direct_stream: true,
            supports_transcoding: false,
            chapters: parsed_chapters,
            media_streams: parsed_media_streams,
        }])
    } else {
        None
    };

    // Extract P0 fields from metadata
    let overview = metadata_string_by_aliases(meta, &["overview"]).or_else(|| {
        meta.get("nfo")
            .and_then(|v| v.get("overview"))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string)
    });
    let premiere_date = metadata_string_by_aliases(meta, &["premiere_date", "air_date"]);
    let end_date = metadata_string_by_aliases(meta, &["end_date"]);
    let production_year = metadata_value_by_aliases(meta, &["production_year"])
        .and_then(value_as_i32)
        .or_else(|| meta.get("nfo").and_then(|v| v.get("year")).and_then(value_as_i32))
        .or_else(|| {
            meta.get("tmdb_raw")
                .and_then(|tmdb| {
                    tmdb.get("release_date")
                        .or_else(|| tmdb.get("first_air_date"))
                        .or_else(|| tmdb.get("tv").and_then(|v| v.get("first_air_date")))
                })
                .and_then(Value::as_str)
                .and_then(parse_year_prefix_from_date)
        });
    let genres = metadata_value_by_aliases(meta, &["genres"]).and_then(|v| {
        v.as_array().map(|arr| {
            arr.iter()
                .filter_map(|g| g.as_str().map(String::from))
                .collect::<Vec<_>>()
        })
    });
    let tags = metadata_string_array_by_aliases(meta, &["tags"]).or_else(|| {
        meta.get("nfo")
            .and_then(|nfo| metadata_string_array_by_aliases(nfo, &["tags"]))
    });
    let provider_ids = metadata_value_by_aliases(meta, &["provider_ids"]).and_then(|v| {
        v.as_object().map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| {
                    v.as_str()
                        .map(|s| (k.clone(), s.to_string()))
                        .or_else(|| v.as_i64().map(|raw| (k.clone(), raw.to_string())))
                        .or_else(|| v.as_u64().map(|raw| (k.clone(), raw.to_string())))
                })
                .collect::<HashMap<_, _>>()
        })
    });
    let official_rating = metadata_string_by_aliases(
        meta,
        &["official_rating", "mpaa", "content_rating", "certification"],
    )
    .or_else(|| {
        meta.get("nfo").and_then(|nfo| {
            metadata_string_by_aliases(
                nfo,
                &["official_rating", "mpaa", "content_rating", "certification"],
            )
        })
    });
    let community_rating = metadata_value_by_aliases(meta, &["community_rating"])
        .and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|raw| raw.parse::<f64>().ok())))
        .or_else(|| {
            meta.get("nfo")
                .and_then(|v| v.get("rating").or_else(|| v.get("community_rating")))
                .and_then(|v| {
                    v.as_f64()
                        .or_else(|| v.as_i64().map(|raw| raw as f64))
                        .or_else(|| v.as_str().and_then(|raw| raw.parse::<f64>().ok()))
                })
        })
        .or_else(|| meta.get("tmdb_raw").and_then(|v| v.get("vote_average")).and_then(Value::as_f64));
    let sort_name = metadata_string_by_aliases(meta, &["sort_name"]);
    let primary_image_aspect_ratio = metadata_value_by_aliases(meta, &["primary_image_aspect_ratio"])
            .and_then(|v| {
                if let Some(value) = v.as_f64() {
                    return Some(value);
                }
                v.as_i64().map(|value| value as f64)
            });

    // Extract studios
    let studios = metadata_value_by_aliases(meta, &["studios"]).and_then(|v| {
        v.as_array().map(|arr| {
            arr.iter()
                .filter_map(|s| {
                    if let Some(name) = s.as_str() {
                        Some(NameGuidPairDto {
                            name: name.to_string(),
                            id: None,
                        })
                    } else if let Some(obj) = s.as_object() {
                        obj.get("name").and_then(|n| n.as_str()).map(|name| NameGuidPairDto {
                            name: name.to_string(),
                            id: obj.get("id").and_then(|i| i.as_str()).map(String::from),
                        })
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        })
    });

    // Extract people (cast/crew)
    let people = metadata_value_by_aliases(meta, &["people"]).and_then(|v| {
        v.as_array().map(|arr| {
            arr.iter()
                .filter_map(|p| {
                    if let Some(name) = p.as_str().map(str::trim).filter(|v| !v.is_empty()) {
                        return Some(BaseItemPersonDto {
                            name: name.to_string(),
                            id: None,
                            role: None,
                            person_type: None,
                            primary_image_tag: None,
                        });
                    }

                    p.as_object().and_then(|obj| {
                        let role = obj
                            .get("role")
                            .or_else(|| obj.get("character"))
                            .and_then(|r| r.as_str())
                            .map(String::from);
                        let person_type = obj
                            .get("type")
                            .or_else(|| obj.get("person_type"))
                            .and_then(|t| t.as_str())
                            .map(String::from)
                            .or_else(|| {
                                role.as_deref().and_then(|value| {
                                    let role = value.to_ascii_lowercase();
                                    if role.contains("director") {
                                        return Some("Director".to_string());
                                    }
                                    if role.contains("writer")
                                        || role.contains("screenplay")
                                        || role.contains("story")
                                        || role.contains("teleplay")
                                    {
                                        return Some("Writer".to_string());
                                    }
                                    if !role.is_empty() {
                                        return Some("Actor".to_string());
                                    }
                                    None
                                })
                            });

                        obj.get("name").and_then(|n| n.as_str())
                            .map(|name| BaseItemPersonDto {
                                name: name.to_string(),
                                id: obj
                                    .get("id")
                                    .or_else(|| obj.get("person_id"))
                                    .and_then(|value| {
                                        value
                                            .as_str()
                                            .map(str::to_string)
                                            .or_else(|| value.as_i64().map(|raw| raw.to_string()))
                                            .or_else(|| value.as_u64().map(|raw| raw.to_string()))
                                    }),
                                role,
                                person_type,
                                primary_image_tag: obj
                                    .get("primary_image_tag")
                                    .and_then(|v| v.as_str())
                                    .map(String::from),
                            })
                    })
                })
                .collect::<Vec<_>>()
        })
    });

    // Image tags - use metadata tag first, fallback to item ID.
    let primary_tag = metadata_string_by_aliases(meta, &["primary_image_tag"])
        .unwrap_or_else(|| row.id.to_string());
    let mut image_tags_map = HashMap::from([("Primary".to_string(), primary_tag.clone())]);
    if let Some(logo_tag) = metadata_string_by_aliases(meta, &["logo_image_tag"])
        .map(|value| value.trim().to_string())
        .filter(|v| !v.is_empty())
    {
        image_tags_map.insert("Logo".to_string(), logo_tag);
    }
    let image_tags = Some(image_tags_map);

    // Backdrop image tags from metadata
    let backdrop_image_tags = metadata_value_by_aliases(meta, &["backdrop_image_tags"]).and_then(
            |v| {
                v.as_array().map(|arr| {
                    arr.iter()
                        .filter_map(|t| t.as_str().map(String::from))
                        .collect::<Vec<_>>()
                })
            },
        );

    // Series/episode fields from row columns
    let series_id_value = row.series_id;
    let series_id = series_id_value.map(|id| id.to_string());
    let series_name = metadata_string_by_aliases(meta, &["series_name"]);
    let season_id = metadata_string_by_aliases(meta, &["season_id"]);
    let season_name = metadata_string_by_aliases(meta, &["season_name"]);
    let metadata_i32 = |key: &str| {
        meta.get(key).and_then(value_as_i32)
    };
    let episode_number = row
        .episode_number
        .or_else(|| metadata_i32("episode_number"))
        .or_else(|| metadata_i32("index_number"));
    let season_number = row
        .season_number
        .or_else(|| metadata_i32("season_number"))
        .or_else(|| metadata_i32("parent_index_number"));
    let (index_number, parent_index_number) = if item_type.eq_ignore_ascii_case("season") {
        (season_number, None)
    } else if item_type.eq_ignore_ascii_case("episode") {
        (episode_number, season_number)
    } else {
        (None, None)
    };

    // Parent ID - episodes should point at season when available.
    let parent_id = if item_type.eq_ignore_ascii_case("episode") {
        season_id
            .clone()
            .or_else(|| series_id.clone())
            .or_else(|| row.library_id.map(|id| id.to_string()))
    } else if item_type.eq_ignore_ascii_case("season") {
        series_id
            .clone()
            .or_else(|| row.library_id.map(|id| id.to_string()))
    } else {
        row.library_id
            .map(|id| id.to_string())
            .or_else(|| series_id.clone())
    };

    // Date created from row
    let date_created = Some(row.created_at.to_rfc3339());

    // Child count from metadata (for series/seasons)
    let child_count = metadata_value_by_aliases(meta, &["child_count", "childCount"])
        .and_then(value_as_i32);
    let recursive_item_count = metadata_value_by_aliases(
        meta,
        &["recursive_item_count", "recursiveItemCount"],
    )
    .and_then(value_as_i32);

    // Play access - "Full" for playable items
    let play_access = if matches!(item_type.as_str(), "Movie" | "Episode" | "Audio") {
        Some("Full".to_string())
    } else {
        None
    };

    BaseItemDto {
        id: row.id.to_string(),
        name: row.name,
        item_type,
        path: row.path,
        is_folder: Some(is_folder),
        media_type: media_type.clone(),
        container,
        location_type,
        can_delete: Some(false),
        can_download: Some(!is_folder && media_type.is_some()),
        collection_type: None,
        runtime_ticks,
        bitrate,
        media_sources,
        user_data,
        overview,
        premiere_date,
        end_date,
        production_year,
        genres,
        tags,
        provider_ids,
        image_tags,
        primary_image_tag: Some(primary_tag),
        parent_id,
        series_id,
        series_name,
        season_id,
        season_name,
        index_number,
        parent_index_number,
        backdrop_image_tags,
        official_rating,
        community_rating,
        studios,
        people,
        sort_name,
        primary_image_aspect_ratio,
        date_created,
        child_count,
        recursive_item_count,
        play_access,
    }
}

fn merge_secret_placeholders(mut incoming: Value, current: &Value) -> Value {
    match (&mut incoming, current) {
        (Value::String(incoming_value), current_value) => {
            if incoming_value.trim() == "***" {
                return current_value.clone();
            }
        }
        (Value::Object(incoming_obj), Value::Object(current_obj)) => {
            for (key, incoming_value) in incoming_obj.iter_mut() {
                if let Some(current_value) = current_obj.get(key) {
                    *incoming_value =
                        merge_secret_placeholders(incoming_value.clone(), current_value);
                }
            }
        }
        (Value::Array(incoming_arr), Value::Array(current_arr)) => {
            for (idx, incoming_value) in incoming_arr.iter_mut().enumerate() {
                if let Some(current_value) = current_arr.get(idx) {
                    *incoming_value =
                        merge_secret_placeholders(incoming_value.clone(), current_value);
                }
            }
        }
        _ => {}
    }
    incoming
}

fn strip_node_runtime_protected_fields(cfg: &mut Value) {
    let Some(obj) = cfg.as_object_mut() else {
        return;
    };
    obj.remove("stream_route");
    obj.remove("stream_token");
    obj.remove("playback_domains");
}

fn apply_global_lumenbackend_stream_fields(
    cfg: &mut Value,
    global_route: &str,
    global_signing_key: &str,
    global_ttl_seconds: u64,
) {
    let Some(obj) = cfg.as_object_mut() else {
        return;
    };
    obj.insert(
        "stream_route".to_string(),
        Value::String(normalize_lumenbackend_route(global_route)),
    );
    obj.insert(
        "stream_token".to_string(),
        json!({
            "enabled": !global_signing_key.trim().is_empty(),
            "signing_key": global_signing_key,
            "max_age_sec": global_ttl_seconds,
        }),
    );
}

fn merge_json_values(base: &mut Value, overlay: &Value) {
    match (base, overlay) {
        (Value::Object(base_obj), Value::Object(overlay_obj)) => {
            for (key, overlay_value) in overlay_obj {
                match base_obj.get_mut(key) {
                    Some(base_value) => merge_json_values(base_value, overlay_value),
                    None => {
                        base_obj.insert(key.clone(), overlay_value.clone());
                    }
                }
            }
        }
        (base_value, overlay_value) => {
            *base_value = overlay_value.clone();
        }
    }
}

fn mask_secret_fields(mut cfg: Value) -> Value {
    fn mask_obj(map: &mut serde_json::Map<String, Value>) {
        let sensitive_keys = [
            "secret",
            "password",
            "token",
            "access_key",
            "secret_key",
            "api_key",
            "dsn",
        ];
        for (k, v) in map.iter_mut() {
            if sensitive_keys
                .iter()
                .any(|needle| k.to_ascii_lowercase().contains(needle))
            {
                if v.is_string() {
                    *v = Value::String("***".to_string());
                }
            }

            match v {
                Value::Object(obj) => mask_obj(obj),
                Value::Array(arr) => {
                    for item in arr {
                        if let Value::Object(obj) = item {
                            mask_obj(obj);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    if let Value::Object(ref mut obj) = cfg {
        mask_obj(obj);
    }

    cfg
}
