fn map_codec_type_to_stream_type(codec_type: &str) -> Option<&'static str> {
    if codec_type.eq_ignore_ascii_case("video") {
        return Some("Video");
    }
    if codec_type.eq_ignore_ascii_case("audio") {
        return Some("Audio");
    }
    if codec_type.eq_ignore_ascii_case("subtitle") || codec_type.eq_ignore_ascii_case("subtitles")
    {
        return Some("Subtitle");
    }
    None
}

fn value_to_i32(value: &Value) -> Option<i32> {
    if let Some(v) = value.as_i64() {
        return i32::try_from(v).ok();
    }
    if let Some(v) = value.as_u64() {
        return i32::try_from(v).ok();
    }
    value
        .as_str()
        .and_then(|raw| raw.parse::<i64>().ok())
        .and_then(|v| i32::try_from(v).ok())
}

fn value_to_i64(value: &Value) -> Option<i64> {
    if let Some(v) = value.as_i64() {
        return Some(v);
    }
    if let Some(v) = value.as_u64() {
        return i64::try_from(v).ok();
    }
    value.as_str().and_then(|raw| raw.parse::<i64>().ok())
}

fn value_to_bool(value: &Value) -> Option<bool> {
    if let Some(v) = value.as_bool() {
        return Some(v);
    }
    if let Some(v) = value.as_i64() {
        return Some(v != 0);
    }
    if let Some(v) = value.as_u64() {
        return Some(v != 0);
    }
    value.as_str().and_then(|raw| {
        if raw.eq_ignore_ascii_case("true") || raw == "1" {
            return Some(true);
        }
        if raw.eq_ignore_ascii_case("false") || raw == "0" {
            return Some(false);
        }
        None
    })
}

fn value_to_f64(value: &Value) -> Option<f64> {
    if let Some(v) = value.as_f64() {
        return Some(v);
    }
    if let Some(v) = value.as_i64() {
        return Some(v as f64);
    }
    if let Some(v) = value.as_u64() {
        return Some(v as f64);
    }
    value.as_str().and_then(|raw| raw.parse::<f64>().ok())
}

const TICKS_PER_SECOND_F64: f64 = 10_000_000.0;
const PLAYBACK_MEDIAINFO_PROBE_TIMEOUT_SECONDS: u64 = 12;

fn seconds_to_ticks(seconds: f64) -> Option<i64> {
    if !seconds.is_finite() || seconds < 0.0 {
        return None;
    }
    let ticks = seconds * TICKS_PER_SECOND_F64;
    if ticks > i64::MAX as f64 {
        return None;
    }
    Some(ticks.round() as i64)
}

fn parse_time_base_to_seconds_multiplier(raw: &str) -> Option<f64> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some((num, den)) = trimmed.split_once('/') {
        let numerator = num.trim().parse::<f64>().ok()?;
        let denominator = den.trim().parse::<f64>().ok()?;
        if denominator <= 0.0 {
            return None;
        }
        let value = numerator / denominator;
        if !value.is_finite() || value <= 0.0 {
            return None;
        }
        return Some(value);
    }
    let value = trimmed.parse::<f64>().ok()?;
    if !value.is_finite() || value <= 0.0 {
        return None;
    }
    Some(value)
}

fn parse_frame_rate_value(value: &Value) -> Option<f64> {
    if let Some(parsed) = value_to_f64(value) {
        return Some(parsed).filter(|v| *v > 0.0);
    }
    let raw = value.as_str()?.trim();
    if raw.is_empty() {
        return None;
    }
    if let Some((num, den)) = raw.split_once('/') {
        let numerator = num.trim().parse::<f64>().ok()?;
        let denominator = den.trim().parse::<f64>().ok()?;
        if denominator <= 0.0 {
            return None;
        }
        let parsed = numerator / denominator;
        return (parsed.is_finite() && parsed > 0.0).then_some(parsed);
    }
    raw.parse::<f64>().ok().filter(|v| *v > 0.0)
}

fn value_by_any_key<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().find_map(|key| value.get(*key))
}

fn parse_item_id_value(value: &Value) -> Option<String> {
    if let Some(raw) = value.as_str().map(str::trim).filter(|v| !v.is_empty()) {
        return Some(raw.to_string());
    }
    if let Some(raw) = value.as_i64() {
        return Some(raw.to_string());
    }
    value.as_u64().map(|raw| raw.to_string())
}

fn extract_playback_item_id(payload: &PlaybackProgressDto) -> Option<String> {
    if let Some(item_id) = payload
        .item_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Some(item_id.to_string());
    }

    if let Some(id) =
        value_by_any_key(&payload.extra, &["ItemId", "itemId", "MediaItemId", "mediaItemId"])
            .and_then(parse_item_id_value)
    {
        return Some(id);
    }

    value_by_any_key(&payload.extra, &["NowPlayingItem", "nowPlayingItem"])
        .and_then(|item| value_by_any_key(item, &["Id", "id", "ItemId", "itemId"]))
        .and_then(parse_item_id_value)
}

fn extract_playback_position_ticks(payload: &PlaybackProgressDto) -> i64 {
    let parsed = payload.position_ticks.or_else(|| {
        value_by_any_key(
            &payload.extra,
            &[
                "PositionTicks",
                "positionTicks",
                "PlaybackPositionTicks",
                "playbackPositionTicks",
            ],
        )
        .and_then(value_to_i64)
    });
    parsed.unwrap_or(0).max(0)
}

fn extract_playback_runtime_ticks(payload: &PlaybackProgressDto) -> Option<i64> {
    let direct = value_by_any_key(
        &payload.extra,
        &["RunTimeTicks", "runTimeTicks", "RuntimeTicks", "runtimeTicks"],
    )
    .and_then(value_to_i64);
    if direct.is_some() {
        return direct.filter(|v| *v > 0);
    }
    value_by_any_key(&payload.extra, &["NowPlayingItem", "nowPlayingItem"])
        .and_then(|item| {
            value_by_any_key(item, &["RunTimeTicks", "runTimeTicks", "RuntimeTicks", "runtimeTicks"])
        })
        .and_then(value_to_i64)
        .filter(|v| *v > 0)
}

fn extract_playback_played_hint(payload: &PlaybackProgressDto) -> Option<bool> {
    value_by_any_key(
        &payload.extra,
        &[
            "Played",
            "played",
            "IsPlayed",
            "isPlayed",
            "Finished",
            "finished",
        ],
    )
    .and_then(value_to_bool)
}

fn infer_playback_played_flag(
    event_kind: &str,
    payload: &PlaybackProgressDto,
    position_ticks: i64,
) -> bool {
    if let Some(played) = extract_playback_played_hint(payload) {
        return played;
    }
    if !event_kind.eq_ignore_ascii_case("stopped") {
        return false;
    }
    let Some(runtime_ticks) = extract_playback_runtime_ticks(payload) else {
        return false;
    };
    position_ticks.saturating_mul(100) >= runtime_ticks.saturating_mul(95)
}

fn extract_mediainfo_source(mediainfo: &Value) -> Option<&serde_json::Map<String, Value>> {
    let first = mediainfo.as_array()?.first()?;
    if let Some(media_source) = first
        .get("MediaSourceInfo")
        .or_else(|| first.get("mediaSourceInfo"))
        .and_then(Value::as_object)
    {
        return Some(media_source);
    }
    first.as_object()
}

fn normalize_playback_mediainfo(input: &Value) -> Value {
    if input.is_null() {
        return Value::Null;
    }

    if let Some(arr) = input.as_array() {
        return Value::Array(arr.clone());
    }

    if let Some(obj) = input.as_object() {
        if let Some(value) = obj.get("MediaSourceWithChapters")
            && let Some(arr) = value.as_array()
        {
            return Value::Array(arr.clone());
        }
        if let Some(value) = obj.get("mediaSourceWithChapters")
            && let Some(arr) = value.as_array()
        {
            return Value::Array(arr.clone());
        }
    }

    input.clone()
}

fn extract_mediainfo_runtime_ticks(mediainfo: &Value) -> Option<i64> {
    let source = extract_mediainfo_source(mediainfo)?;
    source
        .get("RunTimeTicks")
        .or_else(|| source.get("runTimeTicks"))
        .and_then(Value::as_i64)
}

fn extract_mediainfo_bitrate(mediainfo: &Value) -> Option<i32> {
    let source = extract_mediainfo_source(mediainfo)?;
    source
        .get("Bitrate")
        .or_else(|| source.get("bitrate"))
        .and_then(value_to_i32)
}

fn normalize_media_runtime_ticks(
    item_runtime_ticks: Option<i64>,
    mediainfo_runtime_ticks: Option<i64>,
) -> Option<i64> {
    item_runtime_ticks.or(mediainfo_runtime_ticks).or(Some(0))
}

fn normalize_media_bitrate(
    item_bitrate: Option<i32>,
    mediainfo_bitrate: Option<i32>,
) -> Option<i32> {
    item_bitrate.or(mediainfo_bitrate).or(Some(0))
}

fn extract_mediainfo_container(mediainfo: &Value) -> Option<String> {
    let source = extract_mediainfo_source(mediainfo)?;
    source
        .get("Container")
        .or_else(|| source.get("container"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|container| !container.trim().is_empty())
}

fn mediainfo_has_primary_streams(mediainfo: &Value) -> bool {
    parse_media_streams_from_mediainfo(mediainfo)
        .iter()
        .any(|stream| {
            stream.stream_type.eq_ignore_ascii_case("Video")
                || stream.stream_type.eq_ignore_ascii_case("Audio")
        })
}

fn parse_ffprobe_duration_seconds_to_ticks(value: &Value) -> Option<i64> {
    let seconds = value_to_f64(value)?;
    if !seconds.is_finite() || seconds <= 0.0 {
        return None;
    }
    seconds_to_ticks(seconds)
}

fn parse_ffprobe_position_seconds_to_ticks(value: &Value) -> Option<i64> {
    let seconds = value_to_f64(value)?;
    if !seconds.is_finite() || seconds < 0.0 {
        return None;
    }
    seconds_to_ticks(seconds)
}

fn ffprobe_chapter_to_playback_mediainfo(chapter: &Value, fallback_index: usize) -> Option<Value> {
    let chapter_obj = chapter.as_object()?;
    let chapter_index = chapter_obj
        .get("id")
        .and_then(value_to_i64)
        .and_then(|value| i32::try_from(value).ok())
        .or_else(|| i32::try_from(fallback_index).ok())
        .unwrap_or(i32::MAX);
    let start_position_ticks = chapter_obj
        .get("start_time")
        .and_then(parse_ffprobe_position_seconds_to_ticks)
        .or_else(|| {
            chapter_obj
                .get("start")
                .and_then(parse_ffprobe_position_seconds_to_ticks)
        })
        .unwrap_or(0);
    let name = chapter_obj
        .get("tags")
        .and_then(Value::as_object)
        .and_then(|tags| tags.get("title"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("Chapter {}", chapter_index.saturating_add(1)));

    Some(json!({
        "ChapterIndex": chapter_index,
        "StartPositionTicks": start_position_ticks,
        "Name": name,
        "MarkerType": "Chapter"
    }))
}

fn build_playback_mediainfo_from_ffprobe(ffprobe_payload: &Value) -> Value {
    let runtime_ticks = ffprobe_payload
        .get("format")
        .and_then(|format| format.get("duration"))
        .and_then(parse_ffprobe_duration_seconds_to_ticks);
    let bitrate = ffprobe_payload
        .get("format")
        .and_then(|format| format.get("bit_rate"))
        .and_then(value_to_i64);
    let container = ffprobe_payload
        .get("format")
        .and_then(|format| format.get("format_name"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let streams = ffprobe_payload
        .get("streams")
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()));
    let chapters = ffprobe_payload
        .get("chapters")
        .and_then(Value::as_array)
        .map(|raw| {
            raw.iter()
                .enumerate()
                .filter_map(|(index, chapter)| ffprobe_chapter_to_playback_mediainfo(chapter, index))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    json!([
        {
            "MediaSourceInfo": {
                "RunTimeTicks": runtime_ticks,
                "Bitrate": bitrate,
                "Container": container,
                "MediaStreams": streams,
                "Chapters": chapters
            }
        }
    ])
}

fn probe_target_is_supported_for_playback(raw: &str) -> bool {
    raw.starts_with("http://")
        || raw.starts_with("https://")
        || std::path::Path::new(raw).is_absolute()
}

fn chapter_name_from_object(chapter_obj: &serde_json::Map<String, Value>) -> Option<String> {
    json_string_by_keys(chapter_obj, &["Name", "name", "Title", "title"]).or_else(|| {
        chapter_obj
            .get("tags")
            .or_else(|| chapter_obj.get("Tags"))
            .and_then(Value::as_object)
            .and_then(|tags| json_string_by_keys(tags, &["title", "Title", "name", "Name"]))
    })
}

fn chapter_start_ticks_from_object(chapter_obj: &serde_json::Map<String, Value>) -> Option<i64> {
    if let Some(value) = chapter_obj
        .get("StartPositionTicks")
        .or_else(|| chapter_obj.get("startPositionTicks"))
        .or_else(|| chapter_obj.get("start_position_ticks"))
        .and_then(value_to_i64)
    {
        return Some(value.max(0));
    }

    if let Some(value) = chapter_obj
        .get("start_time")
        .or_else(|| chapter_obj.get("StartTime"))
        .and_then(value_to_f64)
        .and_then(seconds_to_ticks)
    {
        return Some(value);
    }

    if let Some(value) = chapter_obj
        .get("start")
        .or_else(|| chapter_obj.get("Start"))
        .and_then(value_to_f64)
    {
        if let Some(multiplier) = chapter_obj
            .get("time_base")
            .or_else(|| chapter_obj.get("TimeBase"))
            .and_then(Value::as_str)
            .and_then(parse_time_base_to_seconds_multiplier)
        {
            return seconds_to_ticks(value * multiplier);
        }
        return seconds_to_ticks(value);
    }

    None
}

fn parse_chapters_from_mediainfo(mediainfo: &Value) -> Vec<ChapterInfoDto> {
    let Some(first) = mediainfo.as_array().and_then(|values| values.first()) else {
        return Vec::new();
    };
    let source = extract_mediainfo_source(mediainfo);
    let raw_chapters = source
        .and_then(|value| value.get("Chapters").or_else(|| value.get("chapters")))
        .or_else(|| first.get("Chapters").or_else(|| first.get("chapters")))
        .and_then(Value::as_array);
    let Some(raw_chapters) = raw_chapters else {
        return Vec::new();
    };

    let mut chapters = Vec::new();
    for (fallback_idx, raw_chapter) in raw_chapters.iter().enumerate() {
        let Some(chapter_obj) = raw_chapter.as_object() else {
            continue;
        };
        let chapter_index = chapter_obj
            .get("ChapterIndex")
            .or_else(|| chapter_obj.get("chapterIndex"))
            .or_else(|| chapter_obj.get("id"))
            .and_then(value_to_i32)
            .or_else(|| i32::try_from(fallback_idx).ok())
            .unwrap_or(i32::MAX);
        let Some(start_position_ticks) = chapter_start_ticks_from_object(chapter_obj) else {
            continue;
        };
        let name = chapter_name_from_object(chapter_obj)
            .or_else(|| Some(format!("Chapter {}", chapter_index.saturating_add(1))));
        let marker_type = json_string_by_keys(chapter_obj, &["MarkerType", "markerType"])
            .or_else(|| Some("Chapter".to_string()));
        let image_tag = json_string_by_keys(chapter_obj, &["ImageTag", "imageTag"]);

        chapters.push(ChapterInfoDto {
            start_position_ticks,
            name,
            image_tag,
            marker_type,
            chapter_index: Some(chapter_index),
        });
    }

    chapters.sort_by(|left, right| {
        left.start_position_ticks
            .cmp(&right.start_position_ticks)
            .then_with(|| left.chapter_index.unwrap_or(0).cmp(&right.chapter_index.unwrap_or(0)))
    });
    chapters
}

fn subtitle_display_title(language: Option<&str>, codec: &str) -> String {
    language.map_or_else(
        || codec.to_uppercase(),
        |lang| format!("{} ({})", lang.to_uppercase(), codec.to_uppercase()),
    )
}

fn normalize_subtitle_language_tag(raw: &str) -> Option<String> {
    let language = raw.trim().to_ascii_lowercase();
    if language.is_empty() {
        return None;
    }
    let normalized = match language.as_str() {
        "zh-cn" | "zh-hans" | "zh-hant" | "zho" | "chi" | "chs" | "cht" => "zh",
        "eng" => "en",
        "jpn" => "ja",
        "kor" => "ko",
        "spa" => "es",
        "fra" | "fre" => "fr",
        _ => language.as_str(),
    };
    Some(normalized.to_string())
}

fn infer_subtitle_language_from_filename(path: &str) -> Option<String> {
    let name = Path::new(path).file_name()?.to_string_lossy().to_lowercase();
    let tokens = name
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();

    for token in tokens {
        let lang = match token {
            "zh" | "zho" | "chi" | "chs" | "cht" | "cn" | "chinese" => Some("zh"),
            "en" | "eng" | "english" => Some("en"),
            "ja" | "jp" | "jpn" | "japanese" => Some("ja"),
            "ko" | "kr" | "kor" | "korean" => Some("ko"),
            "es" | "spa" | "spanish" => Some("es"),
            "fr" | "fra" | "fre" | "french" => Some("fr"),
            _ => None,
        };
        if let Some(lang) = lang {
            return Some(lang.to_string());
        }
    }
    None
}

fn effective_subtitle_language_for_path(
    subtitle_path: &str,
    language: Option<&str>,
) -> Option<String> {
    language
        .and_then(normalize_subtitle_language_tag)
        .or_else(|| infer_subtitle_language_from_filename(subtitle_path))
}

fn subtitle_row_to_media_stream(index: i32, subtitle: SubtitleRow) -> MediaStreamDto {
    let codec = subtitle_codec_from_path(&subtitle.path);
    let language = effective_subtitle_language_for_path(&subtitle.path, subtitle.language.as_deref());
    let display_title = subtitle_display_title(language.as_deref(), &codec);
    MediaStreamDto {
        index,
        stream_type: "Subtitle".to_string(),
        language,
        is_external: true,
        path: Some(subtitle.path),
        codec: Some(codec),
        display_title: Some(display_title),
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
        is_default: Some(subtitle.is_default),
        is_forced: None,
    }
}

fn json_string_by_keys(
    obj: &serde_json::Map<String, Value>,
    keys: &[&str],
) -> Option<String> {
    keys.iter()
        .find_map(|key| obj.get(*key))
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|value| !value.trim().is_empty())
}

fn json_bool_by_keys(
    obj: &serde_json::Map<String, Value>,
    keys: &[&str],
) -> Option<bool> {
    keys.iter().find_map(|key| obj.get(*key)).and_then(value_to_bool)
}

fn json_i32_by_keys(
    obj: &serde_json::Map<String, Value>,
    keys: &[&str],
) -> Option<i32> {
    keys.iter().find_map(|key| obj.get(*key)).and_then(value_to_i32)
}

fn infer_bit_depth_from_pixel_format(pixel_format: &str) -> Option<i32> {
    let pf = pixel_format.trim().to_ascii_lowercase();
    if pf.is_empty() {
        return None;
    }

    // Jellyfin uses PixelFormat (pix_fmt) to infer bit depth when explicit bit-depth fields are missing.
    // Keep this intentionally conservative; unknown formats return None.
    match pf.as_str() {
        "yuv420p" | "yuv422p" | "yuv444p" | "yuvj420p" | "yuvj422p" | "yuvj444p" => Some(8),
        _ => {
            // Common patterns: yuv420p10le, yuv444p12le, p010le, p016le
            if pf.contains("p010") || pf.contains("p10") {
                return Some(10);
            }
            if pf.contains("p012") || pf.contains("p12") {
                return Some(12);
            }
            if pf.contains("p014") || pf.contains("p14") {
                return Some(14);
            }
            if pf.contains("p016") || pf.contains("p16") {
                return Some(16);
            }
            None
        }
    }
}

fn side_data_list(stream: &serde_json::Map<String, Value>) -> Option<&Vec<Value>> {
    stream
        .get("SideDataList")
        .or_else(|| stream.get("side_data_list"))
        .or_else(|| stream.get("sideDataList"))
        .and_then(Value::as_array)
}

fn find_side_data_by_type<'a>(
    side_data: &'a [Value],
    wanted_type: &str,
) -> Option<&'a serde_json::Map<String, Value>> {
    side_data.iter().find_map(|entry| {
        let obj = entry.as_object()?;
        let ty = obj
            .get("side_data_type")
            .or_else(|| obj.get("SideDataType"))
            .or_else(|| obj.get("sideDataType"))
            .and_then(Value::as_str)?;
        if ty.eq_ignore_ascii_case(wanted_type) {
            Some(obj)
        } else {
            None
        }
    })
}

fn infer_video_range_type_and_range(
    color_transfer: Option<&str>,
    has_dovi: bool,
    has_hdr10_plus: bool,
) -> (Option<String>, Option<String>) {
    let transfer = color_transfer.unwrap_or_default();
    let is_pq = transfer.eq_ignore_ascii_case("smpte2084");
    let is_hlg = transfer.eq_ignore_ascii_case("arib-std-b67");

    let range_type = if has_dovi {
        if has_hdr10_plus {
            Some("DOVIWithHDR10Plus")
        } else if is_pq {
            Some("DOVIWithHDR10")
        } else if is_hlg {
            Some("DOVIWithHLG")
        } else {
            Some("DOVI")
        }
    } else if has_hdr10_plus {
        Some("HDR10Plus")
    } else if is_pq {
        Some("HDR10")
    } else if is_hlg {
        Some("HLG")
    } else {
        Some("SDR")
    };

    let range = match range_type {
        Some("SDR") => Some("SDR"),
        Some(_) => Some("HDR"),
        None => None,
    };

    (
        range.map(ToString::to_string),
        range_type.map(ToString::to_string),
    )
}

fn normalize_image_type_slug(image_type: &str) -> String {
    let mut slug = image_type
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();
    if slug.is_empty() {
        slug = "primary".to_string();
    }
    slug
}

fn library_image_candidates(dir: &Path, image_type: &str) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut push_existing = |name: &str| {
        let p = dir.join(name);
        if p.exists() {
            files.push(p);
        }
    };

    match image_type.to_ascii_lowercase().as_str() {
        "primary" | "poster" | "thumb" | "thumbnail" => {
            push_existing("folder.jpg");
            push_existing("poster.jpg");
            push_existing("cover.jpg");
            push_existing("folder.png");
            push_existing("poster.png");
            push_existing("cover.png");
            push_existing("thumb.jpg");
            push_existing("thumb.png");
        }
        "backdrop" | "fanart" => {
            push_existing("fanart.jpg");
            push_existing("fanart.png");
            push_existing("backdrop.jpg");
            push_existing("backdrop.png");
        }
        _ => {
            push_existing("folder.jpg");
            push_existing("poster.jpg");
            push_existing("cover.jpg");
            push_existing("folder.png");
            push_existing("poster.png");
            push_existing("cover.png");
        }
    }

    files.sort();
    files.dedup();
    files
}

fn library_image_base_names(image_type: &str) -> &'static [&'static str] {
    match image_type.to_ascii_lowercase().as_str() {
        "primary" | "poster" | "thumb" | "thumbnail" | "logo" | "art" | "banner" => {
            &["folder", "poster", "cover", "thumb"]
        }
        "backdrop" | "fanart" => &["fanart", "backdrop"],
        _ => &["cover"],
    }
}

fn library_image_target_basename(image_type: &str) -> &'static str {
    library_image_base_names(image_type)
        .first()
        .copied()
        .unwrap_or("cover")
}

fn extend_unique_paths(paths: &mut Vec<PathBuf>, candidates: Vec<PathBuf>) {
    for path in candidates {
        if !paths.iter().any(|existing| existing == &path) {
            paths.push(path);
        }
    }
}

fn season_image_candidates(dir: &Path, season_number: i32, image_type: &str) -> Vec<PathBuf> {
    image_candidates(dir, &format!("Season {season_number:02}"), image_type)
}

fn media_item_image_candidates(
    item_path: &Path,
    item_type: &str,
    season_number: Option<i32>,
    image_type: &str,
) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let primary_like =
        image_type.eq_ignore_ascii_case("primary") || image_type.eq_ignore_ascii_case("poster");
    let treat_as_directory = item_path.is_dir() || item_path.extension().is_none();

    if treat_as_directory {
        let stem = item_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        if !stem.is_empty() {
            extend_unique_paths(&mut candidates, image_candidates(item_path, stem, image_type));
            if let Some(parent_dir) = item_path.parent() {
                extend_unique_paths(&mut candidates, image_candidates(parent_dir, stem, image_type));
            }
        }
        if primary_like
            && matches!(item_type.to_ascii_lowercase().as_str(), "season" | "episode")
            && let Some(number) = season_number
        {
            extend_unique_paths(&mut candidates, season_image_candidates(item_path, number, image_type));
            if let Some(parent_dir) = item_path.parent() {
                extend_unique_paths(
                    &mut candidates,
                    season_image_candidates(parent_dir, number, image_type),
                );
            }
        }
        return candidates;
    }

    let Some(dir) = item_path.parent() else {
        return candidates;
    };
    let stem = item_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    extend_unique_paths(&mut candidates, image_candidates(dir, stem, image_type));
    if primary_like
        && item_type.eq_ignore_ascii_case("episode")
        && let Some(number) = season_number
    {
        extend_unique_paths(&mut candidates, season_image_candidates(dir, number, image_type));
        if let Some(parent_dir) = dir.parent() {
            extend_unique_paths(
                &mut candidates,
                season_image_candidates(parent_dir, number, image_type),
            );
        }
    }
    candidates
}

fn media_item_image_base_names(stem: &str, image_type: &str) -> Vec<String> {
    match image_type.to_ascii_lowercase().as_str() {
        "primary" | "poster" => vec![
            stem.to_string(),
            "poster".to_string(),
            "folder".to_string(),
            "cover".to_string(),
        ],
        "thumb" | "thumbnail" => vec![format!("{stem}-thumb"), format!("{stem}.thumb"), "thumb".to_string()],
        "backdrop" | "fanart" => {
            vec!["fanart".to_string(), format!("{stem}-fanart"), format!("{stem}.fanart")]
        }
        "logo" => vec!["logo".to_string(), format!("{stem}-logo"), stem.to_string()],
        "art" => vec!["art".to_string(), stem.to_string()],
        "banner" => vec!["banner".to_string(), stem.to_string()],
        _ => vec![stem.to_string()],
    }
}

fn media_item_image_target_basename(stem: &str, image_type: &str) -> String {
    media_item_image_base_names(stem, image_type)
        .first()
        .cloned()
        .unwrap_or_else(|| stem.to_string())
}

fn normalize_image_extension(extension: &str) -> &'static str {
    match extension.trim().to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" => "jpg",
        "png" => "png",
        "webp" => "webp",
        "gif" => "gif",
        _ => "jpg",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageResizeFormat {
    Jpeg,
    Png,
    Webp,
}

impl ImageResizeFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::Webp => "webp",
        }
    }

    fn key_name(self) -> &'static str {
        match self {
            Self::Jpeg => "jpeg",
            Self::Png => "png",
            Self::Webp => "webp",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImageResizeRequest {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    pub quality: Option<u8>,
    pub format: Option<ImageResizeFormat>,
    pub blur: Option<u16>,
    pub background_color: Option<String>,
}

impl ImageResizeRequest {
    pub fn requires_processing(&self) -> bool {
        self.width.is_some()
            || self.height.is_some()
            || self.max_width.is_some()
            || self.max_height.is_some()
            || self.quality.is_some()
            || self.format.is_some()
            || self.blur.is_some_and(|value| value > 0)
            || self.background_color.is_some()
    }

    fn key_payload(&self) -> String {
        format!(
            "w={};h={};mw={};mh={};q={};fmt={};blur={};bg={}",
            self.width.map_or("-".to_string(), |value| value.to_string()),
            self.height.map_or("-".to_string(), |value| value.to_string()),
            self.max_width
                .map_or("-".to_string(), |value| value.to_string()),
            self.max_height
                .map_or("-".to_string(), |value| value.to_string()),
            self.quality.map_or("-".to_string(), |value| value.to_string()),
            self.format
                .map_or("-".to_string(), |value| value.key_name().to_string()),
            self.blur.map_or("-".to_string(), |value| value.to_string()),
            self.background_color
                .as_deref()
                .map(str::to_ascii_lowercase)
                .unwrap_or_else(|| "-".to_string()),
        )
    }
}

fn resized_image_cache_root(cache_root: &str) -> PathBuf {
    Path::new(cache_root).join("resized-images")
}

fn metadata_modified_nanos(metadata: &std::fs::Metadata) -> u128 {
    metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|value| value.as_nanos())
        .unwrap_or(0)
}

fn resized_image_cache_key(
    source_path: &Path,
    source_meta: &std::fs::Metadata,
    request: &ImageResizeRequest,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source_path.to_string_lossy().as_bytes());
    hasher.update(b"|");
    hasher.update(source_meta.len().to_string().as_bytes());
    hasher.update(b"|");
    hasher.update(metadata_modified_nanos(source_meta).to_string().as_bytes());
    hasher.update(b"|");
    hasher.update(request.key_payload().as_bytes());
    hex::encode(hasher.finalize())
}

fn resize_output_format(source_path: &Path, request: &ImageResizeRequest) -> ImageResizeFormat {
    if let Some(format) = request.format {
        return format;
    }

    match source_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => ImageResizeFormat::Png,
        "webp" => ImageResizeFormat::Webp,
        _ => ImageResizeFormat::Jpeg,
    }
}

fn resized_image_cache_path(cache_root: &str, cache_key: &str, output_ext: &str) -> PathBuf {
    let shard = &cache_key[..2];
    resized_image_cache_root(cache_root)
        .join(shard)
        .join(format!("{cache_key}.{output_ext}"))
}

fn resized_image_temp_path(cache_path: &Path, cache_key: &str, output_ext: &str) -> PathBuf {
    cache_path.with_file_name(format!(
        "{cache_key}.tmp-{}.{}",
        Uuid::now_v7(),
        output_ext
    ))
}

fn parse_hex_color_byte(pair: &str) -> Option<u8> {
    u8::from_str_radix(pair, 16).ok()
}

fn parse_background_rgba(raw: &str) -> Option<[u8; 4]> {
    let value = raw.trim().trim_start_matches('#');
    if value.is_empty() {
        return None;
    }

    match value.len() {
        3 => {
            let r = parse_hex_color_byte(&value[0..1].repeat(2))?;
            let g = parse_hex_color_byte(&value[1..2].repeat(2))?;
            let b = parse_hex_color_byte(&value[2..3].repeat(2))?;
            Some([r, g, b, 255])
        }
        6 => {
            let r = parse_hex_color_byte(&value[0..2])?;
            let g = parse_hex_color_byte(&value[2..4])?;
            let b = parse_hex_color_byte(&value[4..6])?;
            Some([r, g, b, 255])
        }
        8 => {
            let r = parse_hex_color_byte(&value[0..2])?;
            let g = parse_hex_color_byte(&value[2..4])?;
            let b = parse_hex_color_byte(&value[4..6])?;
            let a = parse_hex_color_byte(&value[6..8])?;
            Some([r, g, b, a])
        }
        _ => None,
    }
}

fn resize_target_box(
    source_width: u32,
    source_height: u32,
    request: &ImageResizeRequest,
) -> Option<(u32, u32)> {
    if source_width == 0 || source_height == 0 {
        return None;
    }

    let mut bound_width = request.width.or(request.max_width);
    let mut bound_height = request.height.or(request.max_height);

    if let Some(max_width) = request.max_width {
        bound_width = Some(bound_width.map_or(max_width, |value| value.min(max_width)));
    }
    if let Some(max_height) = request.max_height {
        bound_height = Some(bound_height.map_or(max_height, |value| value.min(max_height)));
    }

    let (mut width, mut height) = match (bound_width, bound_height) {
        (Some(width), Some(height)) => (width.max(1), height.max(1)),
        (Some(width), None) => {
            let ratio = source_height as f64 / source_width as f64;
            let computed_height = (width as f64 * ratio).round() as u32;
            (width.max(1), computed_height.max(1))
        }
        (None, Some(height)) => {
            let ratio = source_width as f64 / source_height as f64;
            let computed_width = (height as f64 * ratio).round() as u32;
            (computed_width.max(1), height.max(1))
        }
        (None, None) => return None,
    };

    width = width.min(source_width);
    height = height.min(source_height);
    Some((width.max(1), height.max(1)))
}

fn apply_background_color(
    image: image::DynamicImage,
    background: [u8; 4],
) -> image::DynamicImage {
    let mut canvas = image::RgbaImage::from_pixel(image.width(), image.height(), image::Rgba(background));
    image::imageops::overlay(&mut canvas, &image.to_rgba8(), 0, 0);
    image::DynamicImage::ImageRgba8(canvas)
}

fn encode_resized_image(
    image: &image::DynamicImage,
    output_path: &Path,
    format: ImageResizeFormat,
    quality: Option<u8>,
) -> anyhow::Result<()> {
    let file = std::fs::File::create(output_path)
        .with_context(|| format!("failed to create output image file: {}", output_path.display()))?;
    let mut writer = std::io::BufWriter::new(file);

    match format {
        ImageResizeFormat::Jpeg => {
            let rgb = image.to_rgb8();
            let quality = quality.unwrap_or(85).clamp(1, 100);
            let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut writer, quality);
            encoder
                .encode(
                    &rgb,
                    rgb.width(),
                    rgb.height(),
                    image::ColorType::Rgb8.into(),
                )
                .context("failed to encode jpeg image")?;
        }
        ImageResizeFormat::Png => {
            image
                .write_to(&mut writer, image::ImageFormat::Png)
                .context("failed to encode png image")?;
        }
        ImageResizeFormat::Webp => {
            image
                .write_to(&mut writer, image::ImageFormat::WebP)
                .context("failed to encode webp image")?;
        }
    }

    Ok(())
}

fn libvips_size_spec(request: &ImageResizeRequest) -> Option<String> {
    let mut width = request.width.or(request.max_width);
    let mut height = request.height.or(request.max_height);
    if let Some(max_width) = request.max_width {
        width = Some(width.map_or(max_width, |value| value.min(max_width)));
    }
    if let Some(max_height) = request.max_height {
        height = Some(height.map_or(max_height, |value| value.min(max_height)));
    }

    match (width, height) {
        (Some(w), Some(h)) => Some(format!("{w}x{h}")),
        (Some(w), None) => Some(format!("{w}x")),
        (None, Some(h)) => Some(format!("x{h}")),
        (None, None) => None,
    }
}

fn build_resized_image_with_libvips(
    source_path: &Path,
    output_path: &Path,
    request: &ImageResizeRequest,
    output_format: ImageResizeFormat,
) -> anyhow::Result<bool> {
    if request.blur.is_some_and(|value| value > 0) || request.background_color.is_some() {
        return Ok(false);
    }

    let mut output_spec = output_path.to_string_lossy().to_string();
    if let Some(quality) = request.quality
        && matches!(output_format, ImageResizeFormat::Jpeg | ImageResizeFormat::Webp)
    {
        output_spec = format!("{output_spec}[Q={}]", quality.clamp(1, 100));
    }

    let status = if let Some(size) = libvips_size_spec(request) {
        match std::process::Command::new("vipsthumbnail")
            .arg(source_path)
            .arg("--size")
            .arg(size)
            .arg("-o")
            .arg(output_spec)
            .status()
        {
            Ok(status) => status,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(err) => return Err(anyhow::anyhow!("failed to execute vipsthumbnail: {err}")),
        }
    } else {
        match std::process::Command::new("vips")
            .arg("copy")
            .arg(source_path)
            .arg(output_spec)
            .status()
        {
            Ok(status) => status,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(err) => return Err(anyhow::anyhow!("failed to execute vips copy: {err}")),
        }
    };

    Ok(status.success())
}

fn build_resized_image(
    source_path: &Path,
    output_path: &Path,
    request: &ImageResizeRequest,
    output_format: ImageResizeFormat,
) -> anyhow::Result<()> {
    if build_resized_image_with_libvips(source_path, output_path, request, output_format)? {
        return Ok(());
    }

    let mut image = image::open(source_path)
        .with_context(|| format!("failed to decode source image: {}", source_path.display()))?;
    let original_width = image.width();
    let original_height = image.height();

    if let Some((bound_width, bound_height)) = resize_target_box(original_width, original_height, request) {
        image = image.resize(bound_width, bound_height, image::imageops::FilterType::Lanczos3);
    }

    if let Some(blur_strength) = request.blur.filter(|value| *value > 0) {
        image = image.blur((blur_strength as f32) / 10.0);
    }

    if let Some(background) = request
        .background_color
        .as_deref()
        .and_then(parse_background_rgba)
    {
        image = apply_background_color(image, background);
    }

    encode_resized_image(&image, output_path, output_format, request.quality)
}

fn library_image_cache_path(
    cache_root: &str,
    library_id: Uuid,
    image_type: &str,
    image_index: i32,
    source_path: &Path,
) -> PathBuf {
    let ext = source_path
        .extension()
        .and_then(|v| v.to_str())
        .filter(|v| !v.trim().is_empty())
        .unwrap_or("jpg");
    Path::new(cache_root)
        .join("library-images")
        .join(library_id.to_string())
        .join(format!(
            "{}-{}.{}",
            normalize_image_type_slug(image_type),
            image_index.max(0),
            ext.to_ascii_lowercase()
        ))
}

async fn ensure_cached_library_image(
    cache_root: &str,
    library_id: Uuid,
    image_type: &str,
    image_index: i32,
    source_path: &Path,
) -> anyhow::Result<PathBuf> {
    let cache_path =
        library_image_cache_path(cache_root, library_id, image_type, image_index, source_path);
    let source_meta = tokio::fs::metadata(source_path)
        .await
        .with_context(|| format!("failed to stat source image: {}", source_path.display()))?;

    let refresh = match tokio::fs::metadata(&cache_path).await {
        Ok(cached_meta) => {
            let size_changed = cached_meta.len() != source_meta.len();
            let source_newer = match (source_meta.modified(), cached_meta.modified()) {
                (Ok(source_mtime), Ok(cache_mtime)) => source_mtime > cache_mtime,
                _ => false,
            };
            size_changed || source_newer
        }
        Err(_) => true,
    };

    if refresh {
        if let Some(parent) = cache_path.parent() {
            tokio::fs::create_dir_all(parent).await.with_context(|| {
                format!("failed to create library image cache dir: {}", parent.display())
            })?;
        }
        tokio::fs::copy(source_path, &cache_path).await.with_context(|| {
            format!(
                "failed to cache library image {} -> {}",
                source_path.display(),
                cache_path.display()
            )
        })?;
    }

    Ok(cache_path)
}

fn parse_media_streams_from_mediainfo(mediainfo: &Value) -> Vec<MediaStreamDto> {
    let Some(source) = extract_mediainfo_source(mediainfo) else {
        return Vec::new();
    };
    let Some(raw_streams) = source
        .get("MediaStreams")
        .or_else(|| source.get("mediaStreams"))
        .or_else(|| source.get("streams"))
        .or_else(|| source.get("Streams"))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };

    let mut streams = Vec::new();
    for (position, raw_stream) in raw_streams.iter().enumerate() {
        let Some(stream) = raw_stream.as_object() else {
            continue;
        };

        let stream_type = stream
            .get("Type")
            .or_else(|| stream.get("type"))
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| {
                stream
                    .get("codec_type")
                    .and_then(Value::as_str)
                    .and_then(map_codec_type_to_stream_type)
                    .map(str::to_string)
            })
            .unwrap_or_else(|| "Unknown".to_string());

        let language = stream
            .get("Language")
            .or_else(|| stream.get("language"))
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| {
                stream
                    .get("tags")
                    .and_then(Value::as_object)
                    .and_then(|tags| tags.get("language"))
                    .and_then(Value::as_str)
                    .map(str::to_string)
            });

        let is_external = stream
            .get("IsExternal")
            .or_else(|| stream.get("is_external"))
            .and_then(value_to_bool)
            .unwrap_or(false);

        let path = stream
            .get("Path")
            .or_else(|| stream.get("path"))
            .and_then(Value::as_str)
            .map(str::to_string);

        let codec = stream
            .get("Codec")
            .or_else(|| stream.get("codec"))
            .or_else(|| stream.get("codec_name"))
            .and_then(Value::as_str)
            .map(str::to_string);

        let is_video = stream_type.eq_ignore_ascii_case("Video");
        let color_range = if is_video {
            json_string_by_keys(stream, &["ColorRange", "color_range", "colorRange"])
        } else {
            None
        };
        let color_space = if is_video {
            json_string_by_keys(stream, &["ColorSpace", "color_space", "colorSpace"])
        } else {
            None
        };
        let color_transfer = if is_video {
            json_string_by_keys(stream, &["ColorTransfer", "color_transfer", "colorTransfer"])
        } else {
            None
        };
        let color_primaries = if is_video {
            json_string_by_keys(stream, &["ColorPrimaries", "color_primaries", "colorPrimaries"])
        } else {
            None
        };

        let mut bit_depth = if is_video {
            json_i32_by_keys(stream, &["BitDepth", "bit_depth", "bitDepth"])
                .or_else(|| json_i32_by_keys(stream, &["BitsPerSample", "bits_per_sample"]))
                .or_else(|| json_i32_by_keys(stream, &["BitsPerRawSample", "bits_per_raw_sample"]))
        } else {
            None
        };

        if bit_depth.is_none() && is_video {
            bit_depth = json_string_by_keys(stream, &["PixelFormat", "pix_fmt", "pixelFormat"])
                .as_deref()
                .and_then(infer_bit_depth_from_pixel_format);
        }

        let mut hdr10_plus_present_flag = if is_video {
            json_bool_by_keys(
                stream,
                &[
                    "Hdr10PlusPresentFlag",
                    "hdr10PlusPresentFlag",
                    "hdr10_plus_present_flag",
                ],
            )
        } else {
            None
        };

        let mut dv_version_major = if is_video {
            json_i32_by_keys(stream, &["DvVersionMajor", "dv_version_major"])
        } else {
            None
        };
        let mut dv_version_minor = if is_video {
            json_i32_by_keys(stream, &["DvVersionMinor", "dv_version_minor"])
        } else {
            None
        };
        let mut dv_profile = if is_video {
            json_i32_by_keys(stream, &["DvProfile", "dv_profile"])
        } else {
            None
        };
        let mut dv_level = if is_video {
            json_i32_by_keys(stream, &["DvLevel", "dv_level"])
        } else {
            None
        };
        let mut rpu_present_flag = if is_video {
            json_bool_by_keys(stream, &["RpuPresentFlag", "rpu_present_flag"])
        } else {
            None
        };
        let mut el_present_flag = if is_video {
            json_bool_by_keys(stream, &["ElPresentFlag", "el_present_flag"])
        } else {
            None
        };
        let mut bl_present_flag = if is_video {
            json_bool_by_keys(stream, &["BlPresentFlag", "bl_present_flag"])
        } else {
            None
        };
        let mut dv_bl_signal_compatibility_id = if is_video {
            json_i32_by_keys(
                stream,
                &["DvBlSignalCompatibilityId", "dv_bl_signal_compatibility_id"],
            )
        } else {
            None
        };

        if is_video {
            if let Some(side_data) = side_data_list(stream) {
                if let Some(dovi) = find_side_data_by_type(side_data, "DOVI configuration record")
                {
                    dv_version_major = dv_version_major.or_else(|| {
                        json_i32_by_keys(dovi, &["dv_version_major", "dvVersionMajor"])
                    });
                    dv_version_minor = dv_version_minor.or_else(|| {
                        json_i32_by_keys(dovi, &["dv_version_minor", "dvVersionMinor"])
                    });
                    dv_profile =
                        dv_profile.or_else(|| json_i32_by_keys(dovi, &["dv_profile", "dvProfile"]));
                    dv_level =
                        dv_level.or_else(|| json_i32_by_keys(dovi, &["dv_level", "dvLevel"]));
                    rpu_present_flag = rpu_present_flag.or_else(|| {
                        json_bool_by_keys(dovi, &["rpu_present_flag", "rpuPresentFlag"])
                    });
                    el_present_flag = el_present_flag.or_else(|| {
                        json_bool_by_keys(dovi, &["el_present_flag", "elPresentFlag"])
                    });
                    bl_present_flag = bl_present_flag.or_else(|| {
                        json_bool_by_keys(dovi, &["bl_present_flag", "blPresentFlag"])
                    });
                    dv_bl_signal_compatibility_id = dv_bl_signal_compatibility_id.or_else(|| {
                        json_i32_by_keys(
                            dovi,
                            &[
                                "dv_bl_signal_compatibility_id",
                                "dvBlSignalCompatibilityId",
                            ],
                        )
                    });
                }

                if hdr10_plus_present_flag != Some(true)
                    && find_side_data_by_type(
                        side_data,
                        "HDR Dynamic Metadata SMPTE2094-40 (HDR10+)",
                    )
                    .is_some()
                {
                    hdr10_plus_present_flag = Some(true);
                }
            }
        }

        let has_dovi = dv_profile.is_some()
            || dv_version_major.is_some()
            || rpu_present_flag == Some(true)
            || bl_present_flag == Some(true);

        let (mut video_range, mut video_range_type) = if is_video {
            (
                json_string_by_keys(stream, &["VideoRange", "video_range", "videoRange"]),
                json_string_by_keys(
                    stream,
                    &["VideoRangeType", "video_range_type", "videoRangeType"],
                ),
            )
        } else {
            (None, None)
        };

        if is_video && (video_range.is_none() || video_range_type.is_none()) {
            let (inferred_range, inferred_type) = infer_video_range_type_and_range(
                color_transfer.as_deref(),
                has_dovi,
                hdr10_plus_present_flag == Some(true),
            );
            video_range = video_range.or(inferred_range);
            video_range_type = video_range_type.or(inferred_type);
        }

        let channels = stream
            .get("Channels")
            .or_else(|| stream.get("channels"))
            .and_then(value_to_i32);
        let sample_rate = stream
            .get("SampleRate")
            .or_else(|| stream.get("sample_rate"))
            .and_then(value_to_i32);
        let channel_layout = stream
            .get("ChannelLayout")
            .or_else(|| stream.get("channel_layout"))
            .and_then(Value::as_str)
            .map(str::to_string)
            .filter(|v| !v.trim().is_empty());
        let width = stream
            .get("Width")
            .or_else(|| stream.get("width"))
            .and_then(value_to_i32);
        let height = stream
            .get("Height")
            .or_else(|| stream.get("height"))
            .and_then(value_to_i32);
        let profile = stream
            .get("Profile")
            .or_else(|| stream.get("profile"))
            .and_then(Value::as_str)
            .map(str::to_string)
            .filter(|v| !v.trim().is_empty());
        let level = stream
            .get("Level")
            .or_else(|| stream.get("level"))
            .and_then(value_to_i32);
        let average_frame_rate = stream
            .get("AverageFrameRate")
            .or_else(|| stream.get("average_frame_rate"))
            .or_else(|| stream.get("avg_frame_rate"))
            .and_then(parse_frame_rate_value);
        let real_frame_rate = stream
            .get("RealFrameRate")
            .or_else(|| stream.get("real_frame_rate"))
            .or_else(|| stream.get("r_frame_rate"))
            .and_then(parse_frame_rate_value);

        let bit_rate = stream
            .get("BitRate")
            .or_else(|| stream.get("bit_rate"))
            .and_then(value_to_i32);

        let is_default = stream
            .get("IsDefault")
            .or_else(|| stream.get("is_default"))
            .and_then(value_to_bool)
            .or_else(|| {
                stream
                    .get("disposition")
                    .and_then(Value::as_object)
                    .and_then(|disposition| disposition.get("default"))
                    .and_then(value_to_bool)
            });
        let is_forced = stream
            .get("IsForced")
            .or_else(|| stream.get("is_forced"))
            .and_then(value_to_bool)
            .or_else(|| {
                stream
                    .get("disposition")
                    .and_then(Value::as_object)
                    .and_then(|disposition| disposition.get("forced"))
                    .and_then(value_to_bool)
            });

        let display_title = stream
            .get("DisplayTitle")
            .or_else(|| stream.get("display_title"))
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| {
                if stream_type.eq_ignore_ascii_case("Subtitle") {
                    let codec_value = codec.as_deref().unwrap_or("subtitle");
                    Some(subtitle_display_title(language.as_deref(), codec_value))
                } else {
                    None
                }
            });

        let index = stream
            .get("Index")
            .or_else(|| stream.get("index"))
            .and_then(value_to_i32)
            .unwrap_or(position as i32);

        streams.push(MediaStreamDto {
            index,
            stream_type,
            language,
            is_external,
            path,
            codec,
            display_title,
            width,
            height,
            average_frame_rate,
            real_frame_rate,
            profile,
            level,
            channels,
            sample_rate,
            channel_layout,
            bit_rate,
            color_range,
            color_space,
            color_transfer,
            color_primaries,
            bit_depth,
            video_range,
            video_range_type,
            hdr10_plus_present_flag,
            dv_version_major,
            dv_version_minor,
            dv_profile,
            dv_level,
            rpu_present_flag,
            el_present_flag,
            bl_present_flag,
            dv_bl_signal_compatibility_id,
            is_default,
            is_forced,
        });
    }

    streams.sort_by_key(|stream| stream.index);
    streams
}

fn index_external_subtitle_rows(
    streams: &[MediaStreamDto],
    subtitles: Vec<SubtitleRow>,
) -> Vec<(i32, SubtitleRow)> {
    let mut subtitle_paths = streams
        .iter()
        .filter(|stream| stream.stream_type.eq_ignore_ascii_case("Subtitle"))
        .filter_map(|stream| {
            stream
                .path
                .as_ref()
                .map(|path| path.trim().to_string())
                .filter(|path| !path.is_empty())
        })
        .collect::<std::collections::HashSet<_>>();
    let mut next_index = streams.iter().map(|stream| stream.index).max().unwrap_or(-1) + 1;
    let mut indexed = Vec::new();
    for subtitle in subtitles {
        let subtitle_path = subtitle.path.trim().to_string();
        if !subtitle_path.is_empty() && !subtitle_paths.insert(subtitle_path) {
            continue;
        }
        indexed.push((next_index, subtitle));
        next_index += 1;
    }
    indexed
}

fn append_external_subtitles_to_media_source(
    media_source: &mut MediaSourceInfoDto,
    subtitles: Vec<SubtitleRow>,
) {
    let indexed_subtitles = index_external_subtitle_rows(&media_source.media_streams, subtitles);
    if indexed_subtitles.is_empty() {
        return;
    }

    for (index, subtitle) in indexed_subtitles {
        media_source
            .media_streams
            .push(subtitle_row_to_media_stream(index, subtitle));
    }
    media_source.media_streams.sort_by_key(|stream| stream.index);
}

fn extract_resolution_label(path: &str) -> Option<String> {
    let stem = std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(path);
    let lower = stem.to_ascii_lowercase();
    for tag in ["2160p", "4k", "1080p", "720p", "480p", "360p"] {
        if lower.contains(tag) {
            let label = if tag == "4k" { "2160p" } else { tag };
            return Some(label.to_uppercase());
        }
    }
    None
}

fn media_source_path_from_row(path: &str, stream_url: Option<&str>, metadata: &Value) -> String {
    let resolved = metadata
        .get("stream_url")
        .or_else(|| metadata.get("strm_url"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            stream_url
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
        .unwrap_or_else(|| path.to_string());
    decode_local_stream_path(&resolved).unwrap_or(resolved)
}

fn playback_mediainfo_from_metadata(metadata: &Value) -> Value {
    metadata
        .get("mediainfo")
        .map(normalize_playback_mediainfo)
        .unwrap_or(Value::Null)
}

fn playback_probe_target_from_row(row: &VersionedMediaSourceRow) -> Option<String> {
    let target = media_source_path_from_row(&row.path, row.stream_url.as_deref(), &row.metadata);
    let trimmed = target.trim();
    if trimmed.is_empty() || !probe_target_is_supported_for_playback(trimmed) {
        return None;
    }
    Some(trimmed.to_string())
}

fn merge_metadata_with_playback_mediainfo(metadata: &Value, mediainfo: Value) -> Value {
    let mut merged = metadata
        .as_object()
        .cloned()
        .unwrap_or_else(serde_json::Map::new);
    merged.insert("mediainfo".to_string(), mediainfo);
    Value::Object(merged)
}

fn merge_runtime_ticks_for_probe(existing: Option<i64>, probed: Option<i64>) -> Option<i64> {
    match (existing, probed) {
        (Some(current), Some(candidate)) if current <= 0 && candidate > 0 => Some(candidate),
        (None, Some(candidate)) if candidate > 0 => Some(candidate),
        _ => existing,
    }
}

fn merge_bitrate_for_probe(existing: Option<i32>, probed: Option<i32>) -> Option<i32> {
    match (existing, probed) {
        (Some(current), Some(candidate)) if current <= 0 && candidate > 0 => Some(candidate),
        (None, Some(candidate)) if candidate > 0 => Some(candidate),
        _ => existing,
    }
}

fn playback_source_rank_bucket(source: &MediaSourceInfoDto, max_bitrate: Option<i64>) -> (i32, i32, i32, i32) {
    let protocol_file = source.protocol.eq_ignore_ascii_case("File");
    let direct_file_bucket = if source.supports_direct_play && protocol_file {
        0
    } else {
        1
    };
    let direct_bucket = if source.supports_direct_play || source.supports_direct_stream {
        0
    } else {
        1
    };
    let protocol_bucket = if protocol_file { 0 } else { 1 };
    let bitrate_bucket = if let (Some(max), Some(bitrate)) = (max_bitrate, source.bitrate) {
        if i64::from(bitrate) <= max {
            0
        } else {
            2
        }
    } else {
        1
    };
    (direct_file_bucket, direct_bucket, protocol_bucket, bitrate_bucket)
}

fn sort_playback_media_sources(media_sources: &mut [MediaSourceInfoDto], max_bitrate: Option<i64>) {
    media_sources.sort_by_key(|source| playback_source_rank_bucket(source, max_bitrate));
}

fn resolve_person_image_path_from_cache_hints(raw_path: &str, cache_dir: &str) -> Option<String> {
    let trimmed = raw_path.trim();
    if trimmed.is_empty() {
        return None;
    }

    let raw = Path::new(trimmed);
    let mut candidates = vec![PathBuf::from(trimmed)];
    if raw.is_relative() {
        if let Some(file_name) = raw.file_name().filter(|value| !value.is_empty()) {
            candidates.push(Path::new(cache_dir).join(file_name));
        }
        let stripped = trimmed.strip_prefix("./").unwrap_or(trimmed);
        if !stripped.is_empty() {
            candidates.push(Path::new(cache_dir).join(stripped));
        }
    }

    for candidate in candidates {
        if candidate.exists() {
            return Some(candidate.to_string_lossy().to_string());
        }
    }

    None
}

impl AppInfra {
    fn ensure_person_placeholder_image_path(&self) -> anyhow::Result<String> {
        const PERSON_PLACEHOLDER_PNG: &[u8] = &[
            137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0,
            0, 1, 8, 6, 0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99,
            248, 255, 255, 63, 0, 5, 254, 2, 254, 167, 53, 129, 174, 0, 0, 0, 0, 73, 69, 78,
            68, 174, 66, 96, 130,
        ];

        let cache_dir = PathBuf::from(&self.config_snapshot().tmdb.person_image_cache_dir);
        std::fs::create_dir_all(&cache_dir)
            .with_context(|| format!("failed to create person image cache dir: {}", cache_dir.display()))?;
        let placeholder = cache_dir.join("person-placeholder.png");
        if !placeholder.exists() {
            std::fs::write(&placeholder, PERSON_PLACEHOLDER_PNG)
                .with_context(|| format!("failed to write placeholder image: {}", placeholder.display()))?;
        }
        Ok(placeholder.to_string_lossy().to_string())
    }

    async fn subtitle_rows_for_item_with_index(
        &self,
        item_id: Uuid,
    ) -> anyhow::Result<Vec<(i32, SubtitleRow)>> {
        let metadata: Option<Value> =
            sqlx::query_scalar("SELECT metadata FROM media_items WHERE id = $1 LIMIT 1")
                .bind(item_id)
                .fetch_optional(&self.pool)
                .await?;
        let mediainfo = metadata
            .as_ref()
            .map(playback_mediainfo_from_metadata)
            .unwrap_or(Value::Null);
        let rows = sqlx::query_as::<_, SubtitleRow>(
            r#"
SELECT path, language, is_default
FROM subtitles
WHERE media_item_id = $1
ORDER BY path ASC
            "#,
        )
        .bind(item_id)
        .fetch_all(&self.pool)
        .await?;
        let streams = parse_media_streams_from_mediainfo(&mediainfo);
        Ok(index_external_subtitle_rows(&streams, rows))
    }

    async fn load_versioned_media_source_rows(
        &self,
        item_id: Uuid,
    ) -> anyhow::Result<Vec<VersionedMediaSourceRow>> {
        let rows = sqlx::query_as::<_, VersionedMediaSourceRow>(
            r#"
WITH anchor AS (
    SELECT id, version_group_id
    FROM media_items
    WHERE id = $1
    LIMIT 1
)
SELECT
    m.id,
    m.name,
    m.path,
    m.runtime_ticks,
    m.bitrate,
    m.stream_url,
    m.metadata,
    m.version_group_id,
    m.version_rank
FROM media_items m
JOIN anchor a ON true
WHERE m.id = a.id
   OR (a.version_group_id IS NOT NULL AND m.version_group_id = a.version_group_id)
ORDER BY m.version_rank ASC, m.id ASC
            "#,
        )
        .bind(item_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn load_subtitles_for_media_items(
        &self,
        item_ids: &[Uuid],
    ) -> anyhow::Result<std::collections::HashMap<Uuid, Vec<SubtitleRow>>> {
        if item_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        let rows = sqlx::query_as::<_, SubtitleWithItemRow>(
            r#"
SELECT media_item_id, path, language, is_default
FROM subtitles
WHERE media_item_id = ANY($1)
ORDER BY media_item_id ASC, path ASC
            "#,
        )
        .bind(item_ids)
        .fetch_all(&self.pool)
        .await?;

        let mut grouped: std::collections::HashMap<Uuid, Vec<SubtitleRow>> =
            std::collections::HashMap::new();
        for row in rows {
            grouped
                .entry(row.media_item_id)
                .or_default()
                .push(SubtitleRow {
                    path: row.path,
                    language: row.language,
                    is_default: row.is_default,
                });
        }
        Ok(grouped)
    }

    pub async fn attach_external_subtitles_to_media_sources(
        &self,
        media_sources: &mut [MediaSourceInfoDto],
    ) -> anyhow::Result<()> {
        let item_ids = media_sources
            .iter()
            .filter_map(|source| Uuid::parse_str(&source.id).ok())
            .collect::<Vec<_>>();
        if item_ids.is_empty() {
            return Ok(());
        }

        let subtitle_map = self.load_subtitles_for_media_items(&item_ids).await?;
        for source in media_sources.iter_mut() {
            let Ok(source_id) = Uuid::parse_str(&source.id) else {
                continue;
            };
            let subtitles = subtitle_map.get(&source_id).cloned().unwrap_or_default();
            append_external_subtitles_to_media_source(source, subtitles);
        }

        Ok(())
    }

    async fn probe_playback_mediainfo_with_ffprobe(&self, probe_target: &str) -> anyhow::Result<Value> {
        let probe_command = tokio::process::Command::new("ffprobe")
            .arg("-v")
            .arg("error")
            .arg("-print_format")
            .arg("json")
            .arg("-show_format")
            .arg("-show_streams")
            .arg("-show_chapters")
            .arg(probe_target)
            .output();
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(PLAYBACK_MEDIAINFO_PROBE_TIMEOUT_SECONDS),
            probe_command,
        )
        .await
        .with_context(|| {
            format!(
                "ffprobe timed out after {} seconds: {probe_target}",
                PLAYBACK_MEDIAINFO_PROBE_TIMEOUT_SECONDS
            )
        })?
        .with_context(|| format!("failed to execute ffprobe: {probe_target}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(
                "ffprobe failed for target {} with status {}: {}",
                probe_target,
                output.status,
                stderr.trim()
            );
        }

        let ffprobe_payload: Value = serde_json::from_slice(&output.stdout)
            .with_context(|| format!("failed to parse ffprobe output: {probe_target}"))?;
        Ok(build_playback_mediainfo_from_ffprobe(&ffprobe_payload))
    }

    async fn backfill_playback_mediainfo_for_row(
        &self,
        row: &mut VersionedMediaSourceRow,
    ) -> anyhow::Result<bool> {
        let current_mediainfo = playback_mediainfo_from_metadata(&row.metadata);
        if mediainfo_has_primary_streams(&current_mediainfo) {
            return Ok(false);
        }

        let Some(probe_target) = playback_probe_target_from_row(row) else {
            return Ok(false);
        };

        let generated_mediainfo = self
            .probe_playback_mediainfo_with_ffprobe(probe_target.as_str())
            .await?;
        let normalized_generated = normalize_playback_mediainfo(&generated_mediainfo);
        let probed_runtime_ticks = extract_mediainfo_runtime_ticks(&normalized_generated);
        let probed_bitrate = extract_mediainfo_bitrate(&normalized_generated);

        sqlx::query(
            r#"
UPDATE media_items
SET runtime_ticks = CASE
        WHEN $2::BIGINT IS NULL THEN runtime_ticks
        WHEN runtime_ticks IS NULL OR runtime_ticks <= 0 THEN $2::BIGINT
        ELSE runtime_ticks
    END,
    bitrate = CASE
        WHEN $3::INT IS NULL THEN bitrate
        WHEN bitrate IS NULL OR bitrate <= 0 THEN $3::INT
        ELSE bitrate
    END,
    metadata = (
        CASE
            WHEN jsonb_typeof(metadata) = 'object' THEN metadata
            ELSE '{}'::jsonb
        END
    ) || jsonb_build_object('mediainfo', $4::jsonb),
    updated_at = now()
WHERE id = $1
            "#,
        )
        .bind(row.id)
        .bind(probed_runtime_ticks)
        .bind(probed_bitrate)
        .bind(generated_mediainfo.clone())
        .execute(&self.pool)
        .await?;

        row.metadata = merge_metadata_with_playback_mediainfo(&row.metadata, generated_mediainfo);
        row.runtime_ticks = merge_runtime_ticks_for_probe(row.runtime_ticks, probed_runtime_ticks);
        row.bitrate = merge_bitrate_for_probe(row.bitrate, probed_bitrate);
        Ok(true)
    }

    fn select_playback_mediainfo_backfill_candidate(
        &self,
        version_rows: &[VersionedMediaSourceRow],
        media_source_id: Option<&str>,
    ) -> Option<VersionedMediaSourceRow> {
        let expected_source_id = media_source_id
            .map(str::trim)
            .filter(|value| !value.is_empty());

        if let Some(expected) = expected_source_id {
            let row = version_rows
                .iter()
                .find(|candidate| candidate.id.to_string().eq_ignore_ascii_case(expected))?;
            let current_mediainfo = playback_mediainfo_from_metadata(&row.metadata);
            if mediainfo_has_primary_streams(&current_mediainfo) {
                return None;
            }
            if playback_probe_target_from_row(row).is_none() {
                return None;
            }
            return Some(row.clone());
        }

        version_rows.iter().find_map(|row| {
            let current_mediainfo = playback_mediainfo_from_metadata(&row.metadata);
            if mediainfo_has_primary_streams(&current_mediainfo) {
                return None;
            }
            playback_probe_target_from_row(row).map(|_| row.clone())
        })
    }

    fn backfill_playback_mediainfo_if_needed(
        &self,
        version_rows: &[VersionedMediaSourceRow],
        media_source_id: Option<&str>,
    ) {
        let Some(mut candidate) =
            self.select_playback_mediainfo_backfill_candidate(version_rows, media_source_id)
        else {
            return;
        };

        let media_item_id = candidate.id;
        let infra = self.clone();
        tokio::spawn(async move {
            if let Err(err) = infra.backfill_playback_mediainfo_for_row(&mut candidate).await {
                warn!(
                    media_item_id = %media_item_id,
                    error = ?err,
                    "failed to backfill mediainfo with on-demand ffprobe"
                );
            }
        });
    }

    fn build_playback_media_source_from_row(
        &self,
        row: &VersionedMediaSourceRow,
        subtitles: Vec<SubtitleRow>,
    ) -> MediaSourceInfoDto {
        let _name = row.name.clone();
        let _stream_url = row.stream_url.as_deref();
        let _version_group_id = row.version_group_id;
        let _version_rank = row.version_rank;
        let mediainfo = playback_mediainfo_from_metadata(&row.metadata);

        let mut streams = parse_media_streams_from_mediainfo(&mediainfo);

        let indexed_subtitles = index_external_subtitle_rows(&streams, subtitles);
        for (index, subtitle) in indexed_subtitles {
            streams.push(subtitle_row_to_media_stream(index, subtitle));
        }
        streams.sort_by_key(|stream| stream.index);

        let media_source_path =
            media_source_path_from_row(&row.path, row.stream_url.as_deref(), &row.metadata);
        let is_strm = media_source_path
            .rsplit('.')
            .next()
            .map(|ext| ext.eq_ignore_ascii_case("strm"))
            .unwrap_or(false);
        let container = extract_mediainfo_container(&mediainfo)
            .or_else(|| {
                row.metadata
                    .get("container")
                    .or_else(|| row.metadata.get("Container"))
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .map(|v| v.to_ascii_lowercase())
            })
            .or_else(|| {
                if is_strm {
                    None
                } else {
                    media_source_path.rsplit('.').next().map(str::to_string)
                }
            });
        let runtime_ticks = normalize_media_runtime_ticks(
            row.runtime_ticks,
            extract_mediainfo_runtime_ticks(&mediainfo),
        );
        let bitrate = normalize_media_bitrate(row.bitrate, extract_mediainfo_bitrate(&mediainfo));
        let chapters = parse_chapters_from_mediainfo(&mediainfo);
        let is_remote = is_strm
            || media_source_path.starts_with("http://")
            || media_source_path.starts_with("https://");

        MediaSourceInfoDto {
            id: row.id.to_string(),
            name: extract_resolution_label(&row.path),
            path: Some(media_source_path),
            protocol: if is_remote {
                "Http".to_string()
            } else {
                "File".to_string()
            },
            container,
            runtime_ticks,
            bitrate,
            supports_direct_play: true,
            supports_direct_stream: true,
            supports_transcoding: false,
            chapters,
            media_streams: streams,
        }
    }

    fn build_listing_media_source_from_row(
        &self,
        row: &RequestedVersionedMediaSourceRow,
    ) -> MediaSourceInfoDto {
        let _name = row.name.clone();
        let _version_rank = row.version_rank;
        let media_source_path =
            media_source_path_from_row(&row.path, row.stream_url.as_deref(), &row.metadata);
        let mediainfo = playback_mediainfo_from_metadata(&row.metadata);
        let parsed_media_streams = parse_media_streams_from_mediainfo(&mediainfo);
        let parsed_chapters = parse_chapters_from_mediainfo(&mediainfo);
        let mediainfo_runtime_ticks = extract_mediainfo_runtime_ticks(&mediainfo);
        let mediainfo_bitrate = extract_mediainfo_bitrate(&mediainfo);
        let runtime_ticks = row
            .runtime_ticks
            .or(mediainfo_runtime_ticks)
            .or_else(|| derive_runtime_ticks_from_metadata(&row.metadata));
        let bitrate = row.bitrate.or(mediainfo_bitrate);
        let container = infer_media_source_container(&media_source_path, &row.metadata)
            .or_else(|| extract_mediainfo_container(&mediainfo));
        let protocol = if media_source_path.starts_with("http://")
            || media_source_path.starts_with("https://")
        {
            "Http".to_string()
        } else {
            "File".to_string()
        };

        MediaSourceInfoDto {
            id: row.id.to_string(),
            name: extract_resolution_label(&row.path),
            path: Some(media_source_path),
            protocol,
            container,
            runtime_ticks: normalize_media_runtime_ticks(runtime_ticks, mediainfo_runtime_ticks),
            bitrate: normalize_media_bitrate(bitrate, mediainfo_bitrate),
            supports_direct_play: true,
            supports_direct_stream: true,
            supports_transcoding: false,
            chapters: parsed_chapters,
            media_streams: parsed_media_streams,
        }
    }

    async fn load_grouped_listing_media_sources(
        &self,
        item_ids: &[Uuid],
    ) -> anyhow::Result<std::collections::HashMap<Uuid, Vec<MediaSourceInfoDto>>> {
        if item_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        let rows = sqlx::query_as::<_, RequestedVersionedMediaSourceRow>(
            r#"
WITH requested AS (
    SELECT id, version_group_id
    FROM media_items
    WHERE id = ANY($1)
),
related AS (
    SELECT
        r.id AS requested_id,
        m.id,
        m.name,
        m.path,
        m.runtime_ticks,
        m.bitrate,
        m.stream_url,
        m.metadata,
        m.version_rank
    FROM requested r
    JOIN media_items m
      ON m.id = r.id
      OR (r.version_group_id IS NOT NULL AND m.version_group_id = r.version_group_id)
),
ranked AS (
    SELECT
        requested_id,
        id,
        name,
        path,
        runtime_ticks,
        bitrate,
        stream_url,
        metadata,
        version_rank,
        COUNT(*) OVER (PARTITION BY requested_id) AS source_count
    FROM related
)
SELECT
    requested_id,
    id,
    name,
    path,
    runtime_ticks,
    bitrate,
    stream_url,
    metadata,
    version_rank
FROM ranked
WHERE source_count > 1
ORDER BY requested_id ASC, version_rank ASC, id ASC
            "#,
        )
        .bind(item_ids)
        .fetch_all(&self.pool)
        .await?;

        let mut grouped = std::collections::HashMap::<Uuid, Vec<MediaSourceInfoDto>>::new();
        for row in rows {
            grouped
                .entry(row.requested_id)
                .or_default()
                .push(self.build_listing_media_source_from_row(&row));
        }
        Ok(grouped)
    }

    pub async fn attach_grouped_media_sources(
        &self,
        items: &mut [BaseItemDto],
    ) -> anyhow::Result<()> {
        let mut requested = items
            .iter()
            .filter(|item| {
                item.item_type.eq_ignore_ascii_case("movie")
                    || item.item_type.eq_ignore_ascii_case("episode")
            })
            .filter_map(|item| Uuid::parse_str(&item.id).ok())
            .collect::<Vec<_>>();
        requested.sort_unstable();
        requested.dedup();
        if requested.is_empty() {
            return Ok(());
        }

        let grouped = self.load_grouped_listing_media_sources(&requested).await?;
        for item in items.iter_mut() {
            let Ok(item_id) = Uuid::parse_str(&item.id) else {
                continue;
            };
            let Some(sources) = grouped.get(&item_id) else {
                continue;
            };
            if sources.len() <= 1 {
                continue;
            }
            item.media_sources = Some(sources.clone());
        }

        Ok(())
    }

    pub async fn playback_info(
        &self,
        item_id: Uuid,
        _user_id: Option<Uuid>,
        media_source_id: Option<&str>,
        max_streaming_bitrate: Option<i64>,
    ) -> anyhow::Result<Option<PlaybackInfoResponseDto>> {
        let version_rows = self.load_versioned_media_source_rows(item_id).await?;
        if version_rows.is_empty() {
            return Ok(None);
        }
        self.backfill_playback_mediainfo_if_needed(&version_rows, media_source_id);
        let item_ids = version_rows.iter().map(|row| row.id).collect::<Vec<_>>();
        let subtitle_map = self.load_subtitles_for_media_items(&item_ids).await?;

        let mut media_sources = version_rows
            .iter()
            .map(|row| {
                let subtitles = subtitle_map.get(&row.id).cloned().unwrap_or_default();
                self.build_playback_media_source_from_row(row, subtitles)
            })
            .collect::<Vec<_>>();
        sort_playback_media_sources(&mut media_sources, max_streaming_bitrate);

        if let Some(expected_source_id) = media_source_id.map(str::trim).filter(|value| !value.is_empty()) {
            media_sources.retain(|source| source.id.eq_ignore_ascii_case(expected_source_id));
            if media_sources.is_empty() {
                return Ok(None);
            }
        }

        Ok(Some(PlaybackInfoResponseDto {
            media_sources,
            play_session_id: Uuid::now_v7().to_string(),
        }))
    }

    pub async fn list_subtitle_tracks(
        &self,
        item_id: Uuid,
    ) -> anyhow::Result<Vec<SubtitleTrackDto>> {
        let indexed_subtitles = self.subtitle_rows_for_item_with_index(item_id).await?;
        let tracks = indexed_subtitles
            .into_iter()
            .map(|(index, row)| {
                let codec = subtitle_codec_from_path(&row.path);
                let language = effective_subtitle_language_for_path(&row.path, row.language.as_deref());
                let display_title = language.as_deref().map_or_else(
                    || codec.to_uppercase(),
                    |lang| format!("{} ({})", lang.to_uppercase(), codec.to_uppercase()),
                );

                SubtitleTrackDto {
                    index,
                    codec,
                    language,
                    display_title,
                    is_external: true,
                    is_default: row.is_default,
                }
            })
            .collect::<Vec<_>>();

        Ok(tracks)
    }

    pub async fn subtitle_path_by_index(
        &self,
        item_id: Uuid,
        subtitle_index: i32,
    ) -> anyhow::Result<Option<String>> {
        if subtitle_index < 0 {
            return Ok(None);
        }

        let indexed = self.subtitle_rows_for_item_with_index(item_id).await?;
        if let Some((_, row)) = indexed.iter().find(|(index, _)| *index == subtitle_index) {
            return Ok(Some(row.path.clone()));
        }

        // Backward compatibility for older clients that still pass 0/1-based subtitle offsets.
        if subtitle_index == 0 {
            return Ok(indexed.first().map(|(_, row)| row.path.clone()));
        }
        if subtitle_index > 0 {
            let offset = (subtitle_index - 1) as usize;
            return Ok(indexed.get(offset).map(|(_, row)| row.path.clone()));
        }
        Ok(None)
    }

    async fn resized_image_lock_for_key(&self, cache_key: &str) -> Arc<Mutex<()>> {
        let mut guard = self.resized_image_locks.lock().await;
        guard
            .entry(cache_key.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    async fn cleanup_resized_image_lock(&self, cache_key: &str, lock: &Arc<Mutex<()>>) {
        let mut guard = self.resized_image_locks.lock().await;
        if let Some(current) = guard.get(cache_key)
            && Arc::ptr_eq(current, lock)
            && Arc::strong_count(current) == 2
        {
            guard.remove(cache_key);
        }
    }

    pub async fn ensure_resized_image(
        &self,
        source_path: &str,
        request: &ImageResizeRequest,
    ) -> anyhow::Result<String> {
        if !request.requires_processing() {
            return Ok(source_path.to_string());
        }

        let source_path_buf = PathBuf::from(source_path);
        let source_meta = tokio::fs::metadata(&source_path_buf).await.with_context(|| {
            format!(
                "failed to stat source image for resize: {}",
                source_path_buf.display()
            )
        })?;
        if !source_meta.is_file() {
            return Err(anyhow::anyhow!(
                "source image path is not a file: {}",
                source_path_buf.display()
            ));
        }

        let cache_root = self.config_snapshot().tmdb.person_image_cache_dir;
        let cache_key = resized_image_cache_key(&source_path_buf, &source_meta, request);
        let output_format = resize_output_format(&source_path_buf, request);
        let cache_path =
            resized_image_cache_path(&cache_root, &cache_key, output_format.extension());

        if cache_path.exists() {
            return Ok(cache_path.to_string_lossy().to_string());
        }

        let lock = self.resized_image_lock_for_key(&cache_key).await;
        let materialize_result = async {
            let _guard = lock.lock().await;
            if cache_path.exists() {
                return Ok::<(), anyhow::Error>(());
            }

            if let Some(parent) = cache_path.parent() {
                tokio::fs::create_dir_all(parent).await.with_context(|| {
                    format!("failed to create resized image cache dir: {}", parent.display())
                })?;
            }

            let temp_path =
                resized_image_temp_path(&cache_path, &cache_key, output_format.extension());
            let source_for_task = source_path_buf.clone();
            let temp_for_task = temp_path.clone();
            let request_for_task = request.clone();

            let task_result = tokio::task::spawn_blocking(move || {
                build_resized_image(
                    &source_for_task,
                    &temp_for_task,
                    &request_for_task,
                    output_format,
                )
            })
            .await
            .context("resized image processing task panicked")?;

            if let Err(err) = task_result {
                let _ = std::fs::remove_file(&temp_path);
                return Err(err);
            }

            if let Err(err) = tokio::fs::rename(&temp_path, &cache_path).await {
                if cache_path.exists() {
                    let _ = tokio::fs::remove_file(&temp_path).await;
                } else {
                    let _ = tokio::fs::remove_file(&temp_path).await;
                    return Err(anyhow::anyhow!(
                        "failed to persist resized cache file {}: {err}",
                        cache_path.display()
                    ));
                }
            }

            Ok(())
        }
        .await;

        self.cleanup_resized_image_lock(&cache_key, &lock).await;
        materialize_result?;
        Ok(cache_path.to_string_lossy().to_string())
    }

    pub async fn image_path_for_item(
        &self,
        item_id: Uuid,
        image_type: &str,
        image_index: i32,
    ) -> anyhow::Result<Option<String>> {
        if image_index < 0 {
            return Ok(None);
        }

        let item_row: Option<(String, String, Option<i32>, Value)> = sqlx::query_as(
            "SELECT path, item_type, season_number, metadata FROM media_items WHERE id = $1 LIMIT 1",
        )
        .bind(item_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some((item_path, item_type, season_number, metadata)) = item_row {
            let metadata_season_number = metadata
                .get("season_number")
                .and_then(value_to_i32)
                .or_else(|| metadata.get("SeasonNumber").and_then(value_to_i32))
                .or_else(|| metadata.get("parent_index_number").and_then(value_to_i32))
                .or_else(|| metadata.get("ParentIndexNumber").and_then(value_to_i32));
            let candidates = media_item_image_candidates(
                Path::new(&item_path),
                &item_type,
                season_number.or(metadata_season_number),
                image_type,
            );
            let idx = image_index as usize;
            return Ok(candidates.get(idx).map(|p| p.to_string_lossy().to_string()));
        }

        // Compatibility fallback: some Emby clients request people images via /Items/{personId}/Images/Primary.
        if image_index == 0
            && (image_type.eq_ignore_ascii_case("primary")
                || image_type.eq_ignore_ascii_case("poster")
                || image_type.eq_ignore_ascii_case("thumb"))
        {
            if let Some(path) = self.resolve_person_primary_image_path(item_id).await? {
                return Ok(Some(path));
            }
        }

        // Compatibility fallback: when item_id is a CollectionFolder (library),
        // serve library cover from root path and persist under cache/library-images.
        let library_root = self.get_library_primary_path(item_id, true).await?;
        let Some(library_root) = library_root else {
            return Ok(None);
        };
        let library_dir = Path::new(&library_root);
        let candidates = library_image_candidates(library_dir, image_type);
        let Some(source_path) = candidates.get(image_index as usize) else {
            return Ok(None);
        };

        match ensure_cached_library_image(
            &self.config_snapshot().tmdb.person_image_cache_dir,
            item_id,
            image_type,
            image_index,
            source_path,
        )
        .await
        {
            Ok(cache_path) => Ok(Some(cache_path.to_string_lossy().to_string())),
            Err(err) => {
                warn!(
                    library_id = %item_id,
                    image_type,
                    image_index,
                    error = %err,
                    "failed to persist library image cache, fallback to source path"
                );
                Ok(Some(source_path.to_string_lossy().to_string()))
            }
        }
    }

    pub async fn save_library_image(
        &self,
        library_id: Uuid,
        image_type: &str,
        data: &[u8],
        extension: &str,
    ) -> anyhow::Result<Option<String>> {
        let library_root = self.get_library_primary_path(library_id, false).await?;
        let Some(library_root) = library_root else {
            return Ok(None);
        };

        let root_dir = Path::new(&library_root);
        tokio::fs::create_dir_all(root_dir)
            .await
            .with_context(|| format!("failed to ensure library root exists: {}", root_dir.display()))?;

        let ext = normalize_image_extension(extension);
        for base in library_image_base_names(image_type) {
            for existing_ext in ["jpg", "jpeg", "png", "webp", "gif"] {
                let path = root_dir.join(format!("{base}.{existing_ext}"));
                let _ = tokio::fs::remove_file(path).await;
            }
        }

        let target_name = library_image_target_basename(image_type);
        let target_path = root_dir.join(format!("{target_name}.{ext}"));
        tokio::fs::write(&target_path, data)
            .await
            .with_context(|| format!("failed to write library image: {}", target_path.display()))?;

        let cache_dir = Path::new(&self.config_snapshot().tmdb.person_image_cache_dir)
            .join("library-images")
            .join(library_id.to_string());
        let _ = tokio::fs::remove_dir_all(&cache_dir).await;

        Ok(Some(target_path.to_string_lossy().to_string()))
    }

    pub async fn save_media_item_image(
        &self,
        item_id: Uuid,
        image_type: &str,
        data: &[u8],
        extension: &str,
    ) -> anyhow::Result<Option<String>> {
        let item_path: Option<String> =
            sqlx::query_scalar("SELECT path FROM media_items WHERE id = $1 LIMIT 1")
                .bind(item_id)
                .fetch_optional(&self.pool)
                .await?;
        let Some(item_path) = item_path else {
            return Ok(None);
        };

        let item_path = Path::new(&item_path);
        let Some(dir) = item_path.parent() else {
            return Ok(None);
        };
        tokio::fs::create_dir_all(dir)
            .await
            .with_context(|| format!("failed to ensure media item directory exists: {}", dir.display()))?;

        let stem = item_path
            .file_stem()
            .and_then(|v| v.to_str())
            .filter(|v| !v.trim().is_empty())
            .unwrap_or("item");
        let ext = normalize_image_extension(extension);
        let base_names = media_item_image_base_names(stem, image_type);

        for base in &base_names {
            for existing_ext in ["jpg", "jpeg", "png", "webp", "gif"] {
                let path = dir.join(format!("{base}.{existing_ext}"));
                let _ = tokio::fs::remove_file(path).await;
            }
        }

        let target_name = media_item_image_target_basename(stem, image_type);
        let target_path = dir.join(format!("{target_name}.{ext}"));
        tokio::fs::write(&target_path, data)
            .await
            .with_context(|| format!("failed to write media item image: {}", target_path.display()))?;

        Ok(Some(target_path.to_string_lossy().to_string()))
    }

    pub async fn delete_library_image(
        &self,
        library_id: Uuid,
        image_type: &str,
    ) -> anyhow::Result<Option<()>> {
        let library_root = self.get_library_primary_path(library_id, false).await?;
        let Some(library_root) = library_root else {
            return Ok(None);
        };

        let root_dir = Path::new(&library_root);
        for base in library_image_base_names(image_type) {
            for ext in ["jpg", "jpeg", "png", "webp", "gif"] {
                let path = root_dir.join(format!("{base}.{ext}"));
                let _ = tokio::fs::remove_file(path).await;
            }
        }

        let cache_dir = Path::new(&self.config_snapshot().tmdb.person_image_cache_dir)
            .join("library-images")
            .join(library_id.to_string());
        let _ = tokio::fs::remove_dir_all(&cache_dir).await;

        Ok(Some(()))
    }

    pub async fn delete_media_item_image(
        &self,
        item_id: Uuid,
        image_type: &str,
    ) -> anyhow::Result<Option<()>> {
        let item_path: Option<String> =
            sqlx::query_scalar("SELECT path FROM media_items WHERE id = $1 LIMIT 1")
                .bind(item_id)
                .fetch_optional(&self.pool)
                .await?;
        let Some(item_path) = item_path else {
            return Ok(None);
        };

        let item_path = Path::new(&item_path);
        let Some(dir) = item_path.parent() else {
            return Ok(None);
        };
        let stem = item_path
            .file_stem()
            .and_then(|v| v.to_str())
            .filter(|v| !v.trim().is_empty())
            .unwrap_or("item");
        let base_names = media_item_image_base_names(stem, image_type);

        for base in &base_names {
            for ext in ["jpg", "jpeg", "png", "webp", "gif"] {
                let path = dir.join(format!("{base}.{ext}"));
                let _ = tokio::fs::remove_file(path).await;
            }
        }

        Ok(Some(()))
    }

    pub async fn list_persons(
        &self,
        search_term: Option<&str>,
        start_index: i64,
        limit: i64,
    ) -> anyhow::Result<QueryResultDto<BaseItemDto>> {
        self.list_persons_with_filters(search_term, start_index, limit, Option::<Uuid>::None, &[])
            .await
    }

    pub async fn list_persons_with_filters(
        &self,
        search_term: Option<&str>,
        start_index: i64,
        limit: i64,
        appears_in_item_id: Option<Uuid>,
        person_types: &[String],
    ) -> anyhow::Result<QueryResultDto<BaseItemDto>> {
        let start_index = start_index.max(0);
        let limit = limit.clamp(1, 500);
        let search_term = search_term.map(str::trim).filter(|v| !v.is_empty());
        let person_types = person_types
            .iter()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string())
            .collect::<Vec<_>>();

        let (total_record_count, rows) = if let Some(item_id) = appears_in_item_id {
            let pattern = search_term.map(|value| format!("%{value}%"));

            let total: i64 = if person_types.is_empty() {
                sqlx::query_scalar(
                    r#"
SELECT COUNT(*)::BIGINT
FROM (
    SELECT p.id
    FROM people p
    INNER JOIN media_item_people mip ON mip.person_id = p.id
    WHERE mip.media_item_id = $1
      AND ($2::TEXT IS NULL OR p.name ILIKE $2)
    GROUP BY p.id
) q
                    "#,
                )
                .bind(item_id)
                .bind(pattern.as_deref())
                .fetch_one(&self.pool)
                .await?
            } else {
                sqlx::query_scalar(
                    r#"
SELECT COUNT(*)::BIGINT
FROM (
    SELECT p.id
    FROM people p
    INNER JOIN media_item_people mip ON mip.person_id = p.id
    WHERE mip.media_item_id = $1
      AND mip.person_type = ANY($2)
      AND ($3::TEXT IS NULL OR p.name ILIKE $3)
    GROUP BY p.id
) q
                    "#,
                )
                .bind(item_id)
                .bind(&person_types)
                .bind(pattern.as_deref())
                .fetch_one(&self.pool)
                .await?
            };

            let rows = if person_types.is_empty() {
                sqlx::query_as::<_, PersonRow>(
                    r#"
SELECT p.id, p.name, p.image_path, p.primary_image_tag, p.metadata, p.created_at
FROM people p
INNER JOIN (
    SELECT person_id, MIN(sort_order) AS min_sort
    FROM media_item_people
    WHERE media_item_id = $1
    GROUP BY person_id
) rel ON rel.person_id = p.id
WHERE ($2::TEXT IS NULL OR p.name ILIKE $2)
ORDER BY rel.min_sort ASC, p.name ASC
LIMIT $3 OFFSET $4
                    "#,
                )
                .bind(item_id)
                .bind(pattern.as_deref())
                .bind(limit)
                .bind(start_index)
                .fetch_all(&self.pool)
                .await?
            } else {
                sqlx::query_as::<_, PersonRow>(
                    r#"
SELECT p.id, p.name, p.image_path, p.primary_image_tag, p.metadata, p.created_at
FROM people p
INNER JOIN (
    SELECT person_id, MIN(sort_order) AS min_sort
    FROM media_item_people
    WHERE media_item_id = $1
      AND person_type = ANY($2)
    GROUP BY person_id
) rel ON rel.person_id = p.id
WHERE ($3::TEXT IS NULL OR p.name ILIKE $3)
ORDER BY rel.min_sort ASC, p.name ASC
LIMIT $4 OFFSET $5
                    "#,
                )
                .bind(item_id)
                .bind(&person_types)
                .bind(pattern.as_deref())
                .bind(limit)
                .bind(start_index)
                .fetch_all(&self.pool)
                .await?
            };

            (total as i32, rows)
        } else if person_types.is_empty() {
            if let Some(search_term) = search_term {
                let pattern = format!("%{search_term}%");
                let total: i64 =
                    sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM people WHERE name ILIKE $1")
                        .bind(&pattern)
                        .fetch_one(&self.pool)
                        .await?;
                let rows = sqlx::query_as::<_, PersonRow>(
                    r#"
SELECT id, name, image_path, primary_image_tag, metadata, created_at
FROM people
WHERE name ILIKE $1
ORDER BY name ASC
LIMIT $2 OFFSET $3
                    "#,
                )
                .bind(&pattern)
                .bind(limit)
                .bind(start_index)
                .fetch_all(&self.pool)
                .await?;
                (total as i32, rows)
            } else {
                let total: i64 = sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM people")
                    .fetch_one(&self.pool)
                    .await?;
                let rows = sqlx::query_as::<_, PersonRow>(
                    r#"
SELECT id, name, image_path, primary_image_tag, metadata, created_at
FROM people
ORDER BY name ASC
LIMIT $1 OFFSET $2
                    "#,
                )
                .bind(limit)
                .bind(start_index)
                .fetch_all(&self.pool)
                .await?;
                (total as i32, rows)
            }
        } else {
            let pattern = search_term.map(|value| format!("%{value}%"));
            let total: i64 = sqlx::query_scalar(
                r#"
SELECT COUNT(*)::BIGINT
FROM (
    SELECT p.id
    FROM people p
    WHERE ($1::TEXT IS NULL OR p.name ILIKE $1)
      AND EXISTS (
          SELECT 1
          FROM media_item_people mip
          WHERE mip.person_id = p.id
            AND mip.person_type = ANY($2)
      )
    GROUP BY p.id
) q
                "#,
            )
            .bind(pattern.as_deref())
            .bind(&person_types)
            .fetch_one(&self.pool)
            .await?;

            let rows = sqlx::query_as::<_, PersonRow>(
                r#"
SELECT p.id, p.name, p.image_path, p.primary_image_tag, p.metadata, p.created_at
FROM people p
WHERE ($1::TEXT IS NULL OR p.name ILIKE $1)
  AND EXISTS (
      SELECT 1
      FROM media_item_people mip
      WHERE mip.person_id = p.id
        AND mip.person_type = ANY($2)
  )
ORDER BY p.name ASC
LIMIT $3 OFFSET $4
                "#,
            )
            .bind(pattern.as_deref())
            .bind(&person_types)
            .bind(limit)
            .bind(start_index)
            .fetch_all(&self.pool)
            .await?;

            (total as i32, rows)
        };

        let items = rows.into_iter().map(person_row_to_dto).collect::<Vec<_>>();
        Ok(QueryResultDto {
            items,
            total_record_count,
            start_index: start_index as i32,
        })
    }

    pub async fn get_person(&self, person_id: Uuid) -> anyhow::Result<Option<BaseItemDto>> {
        let row = sqlx::query_as::<_, PersonRow>(
            r#"
SELECT id, name, image_path, primary_image_tag, metadata, created_at
FROM people
WHERE id = $1
LIMIT 1
            "#,
        )
        .bind(person_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(person_row_to_dto))
    }

    pub async fn image_path_for_person(
        &self,
        person_id: Uuid,
        image_type: &str,
    ) -> anyhow::Result<Option<String>> {
        if !image_type.eq_ignore_ascii_case("primary") {
            return Ok(None);
        }
        self.resolve_person_primary_image_path(person_id).await
    }

    pub async fn stream_url_for_item(&self, item_id: Uuid) -> anyhow::Result<Option<String>> {
        Ok(self
            .resolve_stream_targets(item_id, None)
            .await?
            .into_iter()
            .next())
    }

    pub async fn resolve_stream_targets(
        &self,
        item_id: Uuid,
        range_start: Option<u64>,
    ) -> anyhow::Result<Vec<String>> {
        let row = sqlx::query_as::<_, StreamTargetRow>(
            r#"
SELECT stream_url, metadata
FROM media_items
WHERE id = $1
LIMIT 1
            "#,
        )
        .bind(item_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(Vec::new());
        };

        let Some(raw_url) = row.stream_url else {
            return Ok(Vec::new());
        };

        let mut candidates = Vec::new();

        if self.config_snapshot().storage.prefer_segment_gateway {
            if let Some(segment_url) =
                self.build_segment_gateway_url(&row.metadata, &raw_url, range_start)
            {
                push_unique_target(&mut candidates, segment_url);
            }
        }

        for candidate in self.resolve_storage_urls(&raw_url).await? {
            push_unique_target(&mut candidates, candidate);
        }

        if !is_special_stream_scheme(&raw_url) {
            push_unique_target(&mut candidates, raw_url);
        }

        Ok(candidates)
    }

    async fn resolve_storage_urls(&self, stream_url: &str) -> anyhow::Result<Vec<String>> {
        if let Some(file_id) = stream_url.strip_prefix("gdrive://") {
            let cfg = self.config_snapshot();
            let mut out = Vec::new();
            if cfg.storage.lumenbackend_enabled {
                let route = effective_lumenbackend_route(&cfg.storage.lumenbackend_route);
                out.extend(self.build_lumenbackend_targets(route, file_id, stream_url));
            }

            if !cfg.storage.gdrive_accounts.is_empty() {
                let hash = auth::hash_api_key(file_id.trim());
                let shard = hash.as_bytes().first().copied().unwrap_or(b'0');
                let idx = usize::from(shard) % cfg.storage.gdrive_accounts.len();
                let base = cfg.storage.gdrive_accounts[idx].trim_end_matches('/');
                out.push(format!("{}/drive/{}", base, file_id));
            } else {
                out.push(format!(
                    "https://drive.google.com/uc?id={}&export=download",
                    file_id
                ));
            }

            return Ok(dedup_preserve_order(out));
        }

        if let Some(path) = decode_local_stream_path(stream_url) {
            let cfg = self.config_snapshot();
            if cfg.storage.lumenbackend_enabled {
                let route = normalize_local_stream_route(&cfg.storage.local_stream_route);
                let out = self.build_lumenbackend_targets(route.as_str(), path.as_str(), stream_url);
                if !out.is_empty() {
                    return Ok(out);
                }
            }
            return Ok(Vec::new());
        }

        if let Some(remain) = stream_url.strip_prefix("s3://") {
            let mut parts = remain.splitn(2, '/');
            let bucket = parts.next().unwrap_or_default();
            let key = parts.next().unwrap_or_default();
            if bucket.is_empty() || key.is_empty() {
                return Ok(vec![stream_url.to_string()]);
            }

            let endpoint: Option<String> = sqlx::query_scalar(
                "SELECT config->>'endpoint' FROM storage_configs WHERE kind = 's3' AND enabled = true ORDER BY updated_at DESC LIMIT 1",
            )
            .fetch_optional(&self.pool)
            .await?;

            let endpoint = endpoint.unwrap_or_else(|| "https://s3.amazonaws.com".to_string());
            return Ok(vec![format!(
                "{}/{}/{}",
                endpoint.trim_end_matches('/'),
                bucket,
                key
            )]);
        }

        if let Some(raw) = stream_url.strip_prefix("lumenbackend://") {
            let (route, path) = parse_lumenbackend_reference(raw, &self.config_snapshot().storage.lumenbackend_route);
            let out = self.build_lumenbackend_targets(route.as_str(), path.as_str(), stream_url);
            if !out.is_empty() {
                return Ok(out);
            }
        }

        if std::path::Path::new(stream_url).is_absolute() {
            let cfg = self.config_snapshot();
            if cfg.storage.lumenbackend_enabled {
                let route = normalize_local_stream_route(&cfg.storage.local_stream_route);
                let out = self.build_lumenbackend_targets(route.as_str(), stream_url, stream_url);
                if !out.is_empty() {
                    return Ok(out);
                }
            }
            return Ok(Vec::new());
        }

        Ok(vec![stream_url.to_string()])
    }

    fn build_lumenbackend_targets(&self, route: &str, path: &str, key: &str) -> Vec<String> {
        let cleaned_route = normalize_lumenbackend_route(route);
        let cleaned_path = path.trim();
        if cleaned_route.is_empty() || cleaned_path.is_empty() {
            return Vec::new();
        }

        let cfg = self.config_snapshot();
        let mut nodes = cfg
            .storage
            .lumenbackend_nodes
            .iter()
            .filter_map(|node| normalize_lumenbackend_node(node))
            .collect::<Vec<_>>();
        nodes.dedup();

        if nodes.is_empty() {
            return Vec::new();
        }

        let shift = distributed_offset(key, nodes.len());
        nodes.rotate_left(shift);

        let stream_token = build_lumenbackend_stream_token(
            cfg.storage.lumenbackend_stream_signing_key.as_str(),
            cleaned_route.as_str(),
            cleaned_path,
            cfg.storage.lumenbackend_stream_token_ttl_seconds,
            Utc::now(),
        );

        nodes
            .into_iter()
            .map(|base| {
                let mut url = format!(
                    "{}/{cleaned_route}?path={}",
                    base.trim_end_matches('/'),
                    urlencoding::encode(cleaned_path),
                );
                if let Some(token) = stream_token.as_ref() {
                    url.push_str("&st=");
                    url.push_str(urlencoding::encode(token).as_ref());
                }
                url
            })
            .collect()
    }

    fn build_segment_gateway_url(
        &self,
        metadata: &Value,
        stream_url: &str,
        range_start: Option<u64>,
    ) -> Option<String> {
        let cfg = self.config_snapshot();
        let base = cfg.storage.segment_gateway_base_url.trim();
        if base.is_empty() {
            return None;
        }

        let file_id = metadata
            .get("segment")
            .and_then(|v| v.get("file_id"))
            .and_then(Value::as_str)
            .map(|v| v.to_string())
            .or_else(|| {
                metadata
                    .get("strm_url")
                    .and_then(Value::as_str)
                    .map(|v| auth::hash_api_key(v))
            })
            .unwrap_or_else(|| auth::hash_api_key(stream_url));

        let start = range_start.unwrap_or(0);
        let chunk_index = start / (1024 * 1024);

        if start < 8 * 1024 * 1024 {
            Some(format!(
                "{}/segments/{}/head/{}",
                base.trim_end_matches('/'),
                file_id,
                chunk_index
            ))
        } else {
            Some(format!(
                "{}/segments/{}/tail/{}",
                base.trim_end_matches('/'),
                file_id,
                chunk_index
            ))
        }
    }

    pub async fn report_playback_event(
        &self,
        event_kind: &str,
        user_id: Uuid,
        payload: &PlaybackProgressDto,
    ) -> anyhow::Result<()> {
        let play_session_id = payload
            .play_session_id
            .clone()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| Uuid::now_v7().to_string());

        let item_id = if let Some(raw_item_id) = extract_playback_item_id(payload) {
            self.resolve_uuid_by_any_item_id(&raw_item_id).await?
        } else {
            None
        };
        let position_ticks = extract_playback_position_ticks(payload);
        let played = infer_playback_played_flag(event_kind, payload, position_ticks);
        let play_method = payload
            .play_method
            .clone()
            .unwrap_or_else(|| "DirectPlay".to_string());
        let client_name = payload
            .client
            .clone()
            .unwrap_or_else(|| "ls-client".to_string());
        let device_name = payload
            .device_name
            .clone()
            .unwrap_or_else(|| "ls-device".to_string());

        sqlx::query(
            r#"
INSERT INTO playback_sessions (
    id,
    play_session_id,
    user_id,
    media_item_id,
    device_name,
    client_name,
    play_method,
    position_ticks,
    is_active,
    last_heartbeat_at
) VALUES (
    $1, $2, $3, $4, $5, $6, $7, $8, $9, now()
)
ON CONFLICT(play_session_id) DO UPDATE SET
    user_id = EXCLUDED.user_id,
    media_item_id = EXCLUDED.media_item_id,
    device_name = EXCLUDED.device_name,
    client_name = EXCLUDED.client_name,
    play_method = EXCLUDED.play_method,
    position_ticks = EXCLUDED.position_ticks,
    is_active = EXCLUDED.is_active,
    last_heartbeat_at = now(),
    updated_at = now()
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(&play_session_id)
        .bind(user_id)
        .bind(item_id)
        .bind(device_name)
        .bind(client_name)
        .bind(play_method)
        .bind(position_ticks)
        .bind(event_kind != "stopped")
        .execute(&self.pool)
        .await?;

        if let Some(item_id) = item_id {
            sqlx::query(
                r#"
INSERT INTO watch_states (
    user_id,
    media_item_id,
    playback_position_ticks,
    played,
    last_played_at
) VALUES (
    $1, $2, $3, $4, now()
)
ON CONFLICT(user_id, media_item_id) DO UPDATE SET
    playback_position_ticks = EXCLUDED.playback_position_ticks,
    played = EXCLUDED.played,
    last_played_at = now()
                "#,
            )
            .bind(user_id)
            .bind(item_id)
            .bind(position_ticks)
            .bind(played)
            .execute(&self.pool)
            .await?;

            if event_kind == "start" {
                sqlx::query(
                    r#"
INSERT INTO media_play_events_daily (
    usage_date,
    media_item_id,
    user_id,
    play_session_id,
    play_method,
    created_at
) VALUES (
    current_date,
    $1,
    $2,
    $3,
    $4,
    now()
)
ON CONFLICT (usage_date, media_item_id, play_session_id) DO NOTHING
                    "#,
                )
                .bind(item_id)
                .bind(user_id)
                .bind(&play_session_id)
                .bind(payload.play_method.as_deref())
                .execute(&self.pool)
                .await?;
            }
        }

        Ok(())
    }

    async fn resolve_person_primary_image_path(
        &self,
        person_id: Uuid,
    ) -> anyhow::Result<Option<String>> {
        let row: Option<(Option<String>, Option<i64>, Option<String>, Value)> = sqlx::query_as(
            "SELECT image_path, tmdb_id, profile_path, metadata FROM people WHERE id = $1 LIMIT 1",
        )
        .bind(person_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some((image_path, tmdb_id, profile_path, metadata)) = row else {
            return Ok(None);
        };
        let person_image_cache_dir = self.config_snapshot().tmdb.person_image_cache_dir;
        let tmdb_id = tmdb_id.or_else(|| metadata.get("tmdb_id").and_then(value_to_i64));
        let profile_path = profile_path
            .or_else(|| {
                metadata
                    .get("profile_path")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        if let Some(raw_path) = image_path
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            if let Some(path) = resolve_person_image_path_from_cache_hints(
                raw_path,
                &person_image_cache_dir,
            ) {
                return Ok(Some(path));
            }
        }

        let Some(tmdb_id) = tmdb_id else {
            return match self.ensure_person_placeholder_image_path() {
                Ok(path) => Ok(Some(path)),
                Err(err) => {
                    warn!(
                        person_id = %person_id,
                        error = %err,
                        "failed to materialize person placeholder image"
                    );
                    Ok(None)
                }
            };
        };
        let cache_path = person_image_cache_path(&person_image_cache_dir, tmdb_id);
        if cache_path.exists() {
            return Ok(Some(cache_path.to_string_lossy().to_string()));
        }
        if let Some(profile_path) = profile_path.as_deref() {
            match self
                .ensure_tmdb_image(profile_path, &cache_path, false)
                .await
            {
                Ok(Some(path)) => return Ok(Some(path)),
                Ok(None) => {}
                Err(err) => {
                    warn!(
                        person_id = %person_id,
                        tmdb_id,
                        profile_path,
                        error = %err,
                        "failed to materialize person image in cache dir"
                    );
                }
            }

            let temp_cache_path = std::env::temp_dir()
                .join("ls-person-images")
                .join(format!("person-{tmdb_id}.jpg"));
            match self
                .ensure_tmdb_image(profile_path, &temp_cache_path, false)
                .await
            {
                Ok(Some(path)) => return Ok(Some(path)),
                Ok(None) => {}
                Err(err) => {
                    warn!(
                        person_id = %person_id,
                        tmdb_id,
                        profile_path,
                        error = %err,
                        "failed to materialize person image in temp cache dir"
                    );
                }
            }
        }

        match self.ensure_person_placeholder_image_path() {
            Ok(path) => Ok(Some(path)),
            Err(err) => {
                warn!(
                    person_id = %person_id,
                    tmdb_id,
                    error = %err,
                    "failed to materialize person placeholder image"
                );
                Ok(None)
            }
        }
    }
}

#[cfg(test)]
mod playback_stream_tests {
    use super::*;

    #[test]
    fn test_extract_resolution_label() {
        assert_eq!(
            extract_resolution_label("/data/movie/Film (2020) - 2160p.strm"),
            Some("2160P".to_string())
        );
        assert_eq!(
            extract_resolution_label("/data/movie/Film (2020) - 1080p.strm"),
            Some("1080P".to_string())
        );
        assert_eq!(
            extract_resolution_label("/data/movie/Film (2020) - 720p.mkv"),
            Some("720P".to_string())
        );
        assert_eq!(
            extract_resolution_label("/data/movie/Film (2020) - 4K.strm"),
            Some("2160P".to_string())
        );
        assert_eq!(
            extract_resolution_label("/data/movie/Film (2020).strm"),
            None
        );
    }
}
