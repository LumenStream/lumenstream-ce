#[derive(Debug, Deserialize, Default)]
struct PlaybackInfoCompatQuery {
    #[serde(rename = "UserId", alias = "userId")]
    user_id: Option<String>,
    #[serde(rename = "DeviceId", alias = "deviceId")]
    _device_id: Option<String>,
    #[serde(rename = "PlaySessionId", alias = "playSessionId")]
    _play_session_id: Option<String>,
    #[serde(rename = "MediaSourceId", alias = "mediaSourceId")]
    media_source_id: Option<String>,
    #[serde(rename = "MaxStreamingBitrate", alias = "maxStreamingBitrate")]
    _max_streaming_bitrate: Option<String>,
    #[serde(rename = "StartTimeTicks", alias = "startTimeTicks")]
    _start_time_ticks: Option<String>,
    #[serde(rename = "AudioStreamIndex", alias = "audioStreamIndex")]
    _audio_stream_index: Option<String>,
    #[serde(rename = "SubtitleStreamIndex", alias = "subtitleStreamIndex")]
    _subtitle_stream_index: Option<String>,
    #[serde(rename = "MaxAudioChannels", alias = "maxAudioChannels")]
    _max_audio_channels: Option<String>,
    #[serde(rename = "EnableDirectPlay", alias = "enableDirectPlay")]
    _enable_direct_play: Option<String>,
    #[serde(rename = "EnableDirectStream", alias = "enableDirectStream")]
    _enable_direct_stream: Option<String>,
    #[serde(rename = "EnableTranscoding", alias = "enableTranscoding")]
    _enable_transcoding: Option<String>,
    #[serde(rename = "AllowVideoStreamCopy", alias = "allowVideoStreamCopy")]
    _allow_video_stream_copy: Option<String>,
    #[serde(rename = "AllowAudioStreamCopy", alias = "allowAudioStreamCopy")]
    _allow_audio_stream_copy: Option<String>,
    #[serde(rename = "AutoOpenLiveStream", alias = "autoOpenLiveStream")]
    _auto_open_live_stream: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct PlaybackInfoCompatRequest {
    #[serde(rename = "UserId", alias = "userId")]
    user_id: Option<String>,
    #[serde(rename = "MediaSourceId", alias = "mediaSourceId")]
    media_source_id: Option<String>,
    #[serde(rename = "MaxStreamingBitrate", alias = "maxStreamingBitrate")]
    _max_streaming_bitrate: Option<Value>,
    #[serde(rename = "StartTimeTicks", alias = "startTimeTicks")]
    _start_time_ticks: Option<Value>,
    #[serde(rename = "AudioStreamIndex", alias = "audioStreamIndex")]
    _audio_stream_index: Option<Value>,
    #[serde(rename = "SubtitleStreamIndex", alias = "subtitleStreamIndex")]
    _subtitle_stream_index: Option<Value>,
    #[serde(rename = "MaxAudioChannels", alias = "maxAudioChannels")]
    _max_audio_channels: Option<Value>,
    #[serde(rename = "DeviceProfile", alias = "deviceProfile")]
    _device_profile: Option<Value>,
    #[serde(rename = "DeviceId", alias = "deviceId")]
    _device_id: Option<String>,
    #[serde(rename = "EnableDirectPlay", alias = "enableDirectPlay")]
    _enable_direct_play: Option<Value>,
    #[serde(rename = "EnableDirectStream", alias = "enableDirectStream")]
    _enable_direct_stream: Option<Value>,
    #[serde(rename = "EnableTranscoding", alias = "enableTranscoding")]
    _enable_transcoding: Option<Value>,
    #[serde(rename = "AllowVideoStreamCopy", alias = "allowVideoStreamCopy")]
    _allow_video_stream_copy: Option<Value>,
    #[serde(rename = "AllowAudioStreamCopy", alias = "allowAudioStreamCopy")]
    _allow_audio_stream_copy: Option<Value>,
    #[serde(rename = "AutoOpenLiveStream", alias = "autoOpenLiveStream")]
    _auto_open_live_stream: Option<Value>,
}

#[derive(Debug, Deserialize, Default)]
struct StreamVideoCompatQuery {
    #[serde(rename = "UserId", alias = "userId")]
    _user_id: Option<String>,
    #[serde(rename = "MediaSourceId", alias = "mediaSourceId")]
    media_source_id: Option<String>,
    #[serde(rename = "DeviceId", alias = "deviceId")]
    _device_id: Option<String>,
    #[serde(rename = "PlaySessionId", alias = "playSessionId")]
    _play_session_id: Option<String>,
    #[serde(rename = "Static", alias = "static")]
    _static_flag: Option<String>,
    #[serde(rename = "Tag", alias = "tag")]
    _tag: Option<String>,
    #[serde(rename = "Container", alias = "container")]
    _container: Option<String>,
    #[serde(rename = "TranscodingContainer", alias = "transcodingContainer")]
    _transcoding_container: Option<String>,
    #[serde(rename = "TranscodingProtocol", alias = "transcodingProtocol")]
    _transcoding_protocol: Option<String>,
    #[serde(rename = "VideoCodec", alias = "videoCodec")]
    _video_codec: Option<String>,
    #[serde(rename = "AudioCodec", alias = "audioCodec")]
    _audio_codec: Option<String>,
    #[serde(rename = "SubtitleCodec", alias = "subtitleCodec")]
    _subtitle_codec: Option<String>,
    #[serde(rename = "MaxStreamingBitrate", alias = "maxStreamingBitrate")]
    _max_streaming_bitrate: Option<String>,
    #[serde(rename = "MaxAudioChannels", alias = "maxAudioChannels")]
    _max_audio_channels: Option<String>,
    #[serde(rename = "AudioStreamIndex", alias = "audioStreamIndex")]
    _audio_stream_index: Option<String>,
    #[serde(rename = "SubtitleStreamIndex", alias = "subtitleStreamIndex")]
    _subtitle_stream_index: Option<String>,
    #[serde(rename = "SegmentContainer", alias = "segmentContainer")]
    _segment_container: Option<String>,
    #[serde(rename = "MinSegments", alias = "minSegments")]
    _min_segments: Option<String>,
    #[serde(rename = "BreakOnNonKeyFrames", alias = "breakOnNonKeyFrames")]
    _break_on_non_key_frames: Option<String>,
    #[serde(rename = "LiveStreamId", alias = "liveStreamId")]
    _live_stream_id: Option<String>,
    #[serde(rename = "api_key", alias = "apiKey")]
    _api_key: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct SubtitleStreamCompatQuery {
    #[serde(rename = "MediaSourceId", alias = "mediaSourceId")]
    _media_source_id: Option<String>,
    #[serde(rename = "DeviceId", alias = "deviceId")]
    _device_id: Option<String>,
    #[serde(rename = "PlaySessionId", alias = "playSessionId")]
    _play_session_id: Option<String>,
    #[serde(rename = "Tag", alias = "tag")]
    _tag: Option<String>,
    #[serde(rename = "CopyTimestamps", alias = "copyTimestamps")]
    _copy_timestamps: Option<String>,
    #[serde(rename = "AddVttTimeMap", alias = "addVttTimeMap")]
    _add_vtt_time_map: Option<String>,
    #[serde(rename = "StartPositionTicks", alias = "startPositionTicks")]
    _start_position_ticks: Option<String>,
    #[serde(rename = "EndPositionTicks", alias = "endPositionTicks")]
    _end_position_ticks: Option<String>,
    #[serde(
        rename = "PlaybackStartTimeTicks",
        alias = "playbackStartTimeTicks"
    )]
    _playback_start_time_ticks: Option<String>,
    #[serde(rename = "Format", alias = "format")]
    _format: Option<String>,
    #[serde(rename = "api_key", alias = "apiKey")]
    _api_key: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct ImageRequestCompatQuery {
    #[serde(rename = "MaxWidth", alias = "maxWidth")]
    _max_width: Option<String>,
    #[serde(rename = "MaxHeight", alias = "maxHeight")]
    _max_height: Option<String>,
    #[serde(rename = "Width", alias = "width")]
    _width: Option<String>,
    #[serde(rename = "Height", alias = "height")]
    _height: Option<String>,
    #[serde(rename = "Quality", alias = "quality")]
    _quality: Option<String>,
    #[serde(rename = "Format", alias = "format")]
    _format: Option<String>,
    #[serde(rename = "Tag", alias = "tag")]
    tag: Option<String>,
    #[serde(rename = "PercentPlayed", alias = "percentPlayed")]
    _percent_played: Option<String>,
    #[serde(rename = "UnplayedCount", alias = "unplayedCount")]
    _unplayed_count: Option<String>,
    #[serde(rename = "Blur", alias = "blur")]
    _blur: Option<String>,
    #[serde(rename = "BackgroundColor", alias = "backgroundColor")]
    _background_color: Option<String>,
}

fn parse_positive_u32_param(raw: Option<&str>, max_value: u32) -> Option<u32> {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|value| *value > 0)
        .map(|value| value.min(max_value))
}

fn parse_quality_param(raw: Option<&str>) -> Option<u8> {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|value| value.parse::<u8>().ok())
        .map(|value| value.clamp(1, 100))
}

fn parse_blur_param(raw: Option<&str>) -> Option<u16> {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|value| value.parse::<u16>().ok())
        .map(|value| value.min(200))
}

fn normalize_background_color(raw: Option<&str>) -> Option<String> {
    let value = raw?.trim().trim_start_matches('#');
    if value.is_empty() {
        return None;
    }
    let is_hex = value.chars().all(|value| value.is_ascii_hexdigit());
    if !is_hex {
        return None;
    }
    match value.len() {
        3 | 6 | 8 => Some(value.to_ascii_lowercase()),
        _ => None,
    }
}

fn parse_resize_format(raw: Option<&str>) -> Option<ls_infra::ImageResizeFormat> {
    match raw
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("jpg") | Some("jpeg") => Some(ls_infra::ImageResizeFormat::Jpeg),
        Some("png") => Some(ls_infra::ImageResizeFormat::Png),
        Some("webp") => Some(ls_infra::ImageResizeFormat::Webp),
        _ => None,
    }
}

impl ImageRequestCompatQuery {
    fn resize_request(&self) -> Option<ls_infra::ImageResizeRequest> {
        let request = ls_infra::ImageResizeRequest {
            width: parse_positive_u32_param(self._width.as_deref(), 4096),
            height: parse_positive_u32_param(self._height.as_deref(), 4096),
            max_width: parse_positive_u32_param(self._max_width.as_deref(), 4096),
            max_height: parse_positive_u32_param(self._max_height.as_deref(), 4096),
            quality: parse_quality_param(self._quality.as_deref()),
            format: parse_resize_format(self._format.as_deref()),
            blur: parse_blur_param(self._blur.as_deref()),
            background_color: normalize_background_color(self._background_color.as_deref()),
        };

        request.requires_processing().then_some(request)
    }
}

fn parse_compat_uuid(raw: Option<&str>) -> Option<Uuid> {
    let value = raw?.trim();
    if value.is_empty() {
        return None;
    }

    Uuid::parse_str(value).ok()
}

fn parse_compat_i64(raw: Option<&str>) -> Option<i64> {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|value| value.parse::<i64>().ok())
}

fn playback_value_to_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|raw| i64::try_from(raw).ok()))
        .or_else(|| value.as_str().and_then(|raw| raw.trim().parse::<i64>().ok()))
}

const PLAYBACK_CHAPTER_TICKS_PER_SECOND: f64 = 10_000_000.0;

fn playback_value_to_f64(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_i64().map(|raw| raw as f64))
        .or_else(|| value.as_u64().map(|raw| raw as f64))
        .or_else(|| value.as_str().and_then(|raw| raw.trim().parse::<f64>().ok()))
}

fn playback_seconds_to_ticks(value: &Value) -> Option<i64> {
    let seconds = playback_value_to_f64(value)?;
    if !seconds.is_finite() || seconds < 0.0 {
        return None;
    }
    let ticks = seconds * PLAYBACK_CHAPTER_TICKS_PER_SECOND;
    if ticks > i64::MAX as f64 {
        return None;
    }
    Some(ticks.round() as i64)
}

fn playback_chapter_name(chapter_obj: &serde_json::Map<String, Value>) -> Option<String> {
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

fn ensure_playback_chapter_defaults(chapters: &mut [Value]) {
    for (fallback_idx, chapter) in chapters.iter_mut().enumerate() {
        let Some(chapter_obj) = chapter.as_object_mut() else {
            continue;
        };

        let chapter_index = chapter_obj
            .get("ChapterIndex")
            .or_else(|| chapter_obj.get("chapterIndex"))
            .or_else(|| chapter_obj.get("id"))
            .and_then(playback_value_to_i64)
            .or_else(|| i64::try_from(fallback_idx).ok())
            .unwrap_or(0);
        chapter_obj.insert("ChapterIndex".to_string(), Value::from(chapter_index));

        let start_position_ticks = chapter_obj
            .get("StartPositionTicks")
            .or_else(|| chapter_obj.get("startPositionTicks"))
            .or_else(|| chapter_obj.get("start_position_ticks"))
            .and_then(playback_value_to_i64)
            .or_else(|| {
                chapter_obj
                    .get("start_time")
                    .or_else(|| chapter_obj.get("StartTime"))
                    .and_then(playback_seconds_to_ticks)
            })
            .or_else(|| {
                chapter_obj
                    .get("start")
                    .or_else(|| chapter_obj.get("Start"))
                    .and_then(playback_seconds_to_ticks)
            })
            .unwrap_or(0)
            .max(0);
        chapter_obj.insert(
            "StartPositionTicks".to_string(),
            Value::from(start_position_ticks),
        );

        let name = playback_chapter_name(chapter_obj)
            .unwrap_or_else(|| format!("Chapter {}", chapter_index.saturating_add(1)));
        chapter_obj.insert("Name".to_string(), Value::String(name));

        chapter_obj
            .entry("MarkerType".to_string())
            .or_insert_with(|| Value::String("Chapter".to_string()));
    }
}

fn playback_infer_default_audio_stream_index(media_streams: &[Value]) -> Option<i64> {
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
        let index = stream.get("Index").and_then(playback_value_to_i64)?;
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

fn subtitle_codec_is_text(codec: &str) -> bool {
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

fn subtitle_codec_is_image(codec: &str) -> bool {
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

fn infer_is_text_subtitle_stream(stream_type: &str, codec: Option<&str>) -> bool {
    if !stream_type.eq_ignore_ascii_case("Subtitle") {
        return false;
    }
    let Some(codec) = codec.map(str::trim).filter(|value| !value.is_empty()) else {
        // Keep legacy behavior for subtitle streams with unknown codec.
        return true;
    };
    let normalized = codec.to_ascii_lowercase();
    if subtitle_codec_is_image(normalized.as_str()) {
        return false;
    }
    if subtitle_codec_is_text(normalized.as_str()) {
        return true;
    }
    true
}

fn normalize_subtitle_delivery_codec(codec: Option<&str>) -> String {
    let value = codec
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_else(|| "srt".to_string());
    if value.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-') {
        value
    } else {
        "srt".to_string()
    }
}

fn build_subtitle_delivery_url(
    item_id: &str,
    source_id: &str,
    stream_index: i32,
    codec: Option<&str>,
    access_token: Option<&str>,
) -> String {
    let normalized_item_id = item_id.trim();
    let normalized_source_id = source_id.trim();
    let codec = normalize_subtitle_delivery_codec(codec);
    let base = if normalized_source_id.is_empty() {
        format!("/Videos/{normalized_item_id}/Subtitles/{stream_index}/Stream.{codec}")
    } else {
        format!(
            "/Videos/{normalized_item_id}/{normalized_source_id}/Subtitles/{stream_index}/Stream.{codec}"
        )
    };

    let mut query = Vec::new();
    if let Some(token) = access_token.map(str::trim).filter(|value| !value.is_empty()) {
        query.push(format!("api_key={}", urlencoding::encode(token)));
    }

    if query.is_empty() {
        base
    } else {
        format!("{base}?{}", query.join("&"))
    }
}

fn ensure_playback_media_stream_defaults(
    stream_obj: &mut serde_json::Map<String, Value>,
    source_protocol: &str,
    item_id: &str,
    source_id: &str,
    access_token: Option<&str>,
) {
    let stream_type = stream_obj
        .get("Type")
        .and_then(Value::as_str)
        .unwrap_or("Unknown")
        .to_string();
    let language = stream_obj
        .get("Language")
        .and_then(Value::as_str)
        .unwrap_or("und")
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
    let is_text_subtitle_stream = infer_is_text_subtitle_stream(&stream_type, codec.as_deref());

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
        .entry("Protocol".to_string())
        .or_insert_with(|| Value::String(source_protocol.to_string()));
    stream_obj
        .entry("SupportsExternalStream".to_string())
        .or_insert_with(|| Value::Bool(is_subtitle && is_external));
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
        if is_external
            && let Some(stream_index) = stream_obj
                .get("Index")
                .and_then(playback_value_to_i64)
                .and_then(|value| i32::try_from(value).ok())
        {
            let delivery_url = build_subtitle_delivery_url(
                item_id,
                source_id,
                stream_index,
                codec.as_deref(),
                access_token,
            );
            stream_obj
                .entry("DeliveryUrl".to_string())
                .or_insert_with(|| Value::String(delivery_url));
        }
    }
    stream_obj
        .entry("IsAnamorphic".to_string())
        .or_insert_with(|| Value::Bool(false));
    stream_obj
        .entry("IsHearingImpaired".to_string())
        .or_insert_with(|| Value::Bool(false));
    stream_obj
        .entry("IsInterlaced".to_string())
        .or_insert_with(|| Value::Bool(false));
    stream_obj
        .entry("IsDefault".to_string())
        .or_insert_with(|| Value::Bool(false));
    stream_obj
        .entry("IsForced".to_string())
        .or_insert_with(|| Value::Bool(false));
    stream_obj
        .entry("AttachmentSize".to_string())
        .or_insert_with(|| Value::from(0));
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

    let width = stream_obj
        .get("Width")
        .and_then(playback_value_to_i64)
        .unwrap_or(0);
    let height = stream_obj
        .get("Height")
        .and_then(playback_value_to_i64)
        .unwrap_or(0);
    let aspect_ratio = if width > 0 && height > 0 {
        format!("{}:{}", width, height)
    } else {
        "0:0".to_string()
    };
    stream_obj
        .entry("AspectRatio".to_string())
        .or_insert_with(|| Value::String(aspect_ratio));
}

fn ensure_playback_media_source_defaults(
    source_obj: &mut serde_json::Map<String, Value>,
    access_token: Option<&str>,
    default_item_id: Option<&str>,
) {
    let source_protocol = source_obj
        .get("Protocol")
        .and_then(Value::as_str)
        .unwrap_or("File")
        .to_string();
    let source_path = source_obj
        .get("Path")
        .and_then(Value::as_str)
        .unwrap_or_default();
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

    let source_id = source_obj
        .get("Id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let item_id = source_obj
        .get("ItemId")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            default_item_id
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
        .unwrap_or_else(|| source_id.clone());

    if is_remote && !source_obj.contains_key("DirectStreamUrl") {
        let mut stream_url = format!(
            "/Videos/{}/stream?Static=true&MediaSourceId={}",
            item_id, source_id
        );
        if let Some(token) = access_token {
            stream_url.push_str("&api_key=");
            stream_url.push_str(token);
        }
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
        .entry("Name".to_string())
        .or_insert_with(|| Value::String(source_id.clone()));
    source_obj
        .entry("ItemId".to_string())
        .or_insert_with(|| Value::String(item_id.clone()));
    source_obj
        .entry("Type".to_string())
        .or_insert_with(|| Value::String("Default".to_string()));
    let has_direct_stream_url = source_obj.contains_key("DirectStreamUrl");
    source_obj
        .entry("SupportsDirectPlay".to_string())
        .or_insert_with(|| Value::Bool(!is_remote || has_direct_stream_url));
    source_obj
        .entry("SupportsDirectStream".to_string())
        .or_insert_with(|| Value::Bool(!is_remote || has_direct_stream_url));
    source_obj
        .entry("SupportsTranscoding".to_string())
        .or_insert_with(|| Value::Bool(false));
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
        .or_insert_with(|| json!({}));
    source_obj
        .entry("Formats".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    source_obj
        .entry("Chapters".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if let Some(chapters) = source_obj.get_mut("Chapters").and_then(Value::as_array_mut) {
        ensure_playback_chapter_defaults(chapters);
    }
    source_obj
        .entry("Size".to_string())
        .or_insert_with(|| Value::from(0));

    if source_obj
        .get("RunTimeTicks")
        .map(Value::is_null)
        .unwrap_or(true)
    {
        source_obj.insert("RunTimeTicks".to_string(), Value::from(0));
    }
    if source_obj
        .get("Bitrate")
        .map(Value::is_null)
        .unwrap_or(true)
    {
        source_obj.insert("Bitrate".to_string(), Value::from(0));
    }

    if !matches!(source_obj.get("MediaStreams"), Some(Value::Array(_))) {
        source_obj.insert("MediaStreams".to_string(), Value::Array(Vec::new()));
    }

    let has_default_audio_stream_index = source_obj.contains_key("DefaultAudioStreamIndex");
    let inferred_default_audio_stream_index = if let Some(media_streams) = source_obj
        .get_mut("MediaStreams")
        .and_then(Value::as_array_mut)
    {
        for stream in media_streams.iter_mut() {
            let Some(stream_obj) = stream.as_object_mut() else {
                continue;
            };
            ensure_playback_media_stream_defaults(
                stream_obj,
                &source_protocol,
                item_id.as_str(),
                source_id.as_str(),
                access_token,
            );
        }

        if has_default_audio_stream_index {
            None
        } else {
            playback_infer_default_audio_stream_index(media_streams)
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

fn ensure_playback_info_compat_defaults(
    payload: &mut Value,
    access_token: Option<&str>,
    default_item_id: Option<&str>,
) {
    let Some(object) = payload.as_object_mut() else {
        return;
    };
    let Some(media_sources) = object.get_mut("MediaSources").and_then(Value::as_array_mut) else {
        return;
    };

    for source in media_sources.iter_mut() {
        let Some(source_obj) = source.as_object_mut() else {
            continue;
        };
        ensure_playback_media_source_defaults(source_obj, access_token, default_item_id);
    }
}

fn media_source_is_remote_for_redirect(source_obj: &serde_json::Map<String, Value>) -> bool {
    if let Some(is_remote) = source_obj.get("IsRemote").and_then(Value::as_bool) {
        return is_remote;
    }

    let source_protocol = source_obj
        .get("Protocol")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let source_path = source_obj
        .get("Path")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let path_is_strm = source_path
        .rsplit('.')
        .next()
        .map(|ext| ext.eq_ignore_ascii_case("strm"))
        .unwrap_or(false);
    source_protocol.eq_ignore_ascii_case("Http")
        || source_protocol.eq_ignore_ascii_case("Https")
        || source_path.starts_with("http://")
        || source_path.starts_with("https://")
        || path_is_strm
}

fn apply_playback_original_stream_target(
    source_obj: &mut serde_json::Map<String, Value>,
    target: String,
) {
    source_obj.insert("DirectStreamUrl".to_string(), Value::String(target.clone()));
    source_obj.insert("Path".to_string(), Value::String(target));
    source_obj.insert("AddApiKeyToDirectStreamUrl".to_string(), Value::Bool(false));
    // The stream URL now points to LumenStream's own endpoint, so from the client's
    // perspective this is a local file served by the server, not a remote source.
    source_obj.insert("IsRemote".to_string(), Value::Bool(false));
    source_obj.insert("Protocol".to_string(), Value::String("File".to_string()));
    // Sync Protocol on individual MediaStreams — they were populated before
    // this rewrite and still carry the original "Http" value.
    if let Some(streams) = source_obj.get_mut("MediaStreams").and_then(Value::as_array_mut) {
        for stream in streams.iter_mut() {
            if let Some(obj) = stream.as_object_mut() {
                obj.insert("Protocol".to_string(), Value::String("File".to_string()));
            }
        }
    }
}

fn normalize_original_container_extension(raw: Option<&str>) -> Option<String> {
    let value = raw?.trim();
    if value.is_empty() {
        return None;
    }
    let value = value
        .split(',')
        .find_map(|segment| {
            let candidate = segment.trim().trim_start_matches('.');
            (!candidate.is_empty()).then_some(candidate)
        })?
        .to_ascii_lowercase();
    Some(value)
}

fn build_playback_original_stream_url(
    raw_item_id: &str,
    container: &str,
    media_source_id: Option<&str>,
    play_session_id: Option<&str>,
    access_token: &str,
    device_id: Option<&str>,
) -> String {
    let mut query = Vec::new();
    if let Some(value) = device_id.map(str::trim).filter(|value| !value.is_empty()) {
        query.push(format!("DeviceId={}", urlencoding::encode(value)));
    }
    if let Some(value) = media_source_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        query.push(format!("MediaSourceId={}", urlencoding::encode(value)));
    }
    if let Some(value) = play_session_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        query.push(format!("PlaySessionId={}", urlencoding::encode(value)));
    }
    query.push(format!(
        "api_key={}",
        urlencoding::encode(access_token.trim())
    ));
    format!(
        "/videos/{}/original.{}?{}",
        raw_item_id.trim(),
        container.trim(),
        query.join("&")
    )
}

fn ensure_playback_info_original_stream_urls(
    payload: &mut Value,
    raw_item_id: &str,
    access_token: Option<&str>,
    device_id: Option<&str>,
) {
    let Some(access_token) = access_token.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };

    let Some(object) = payload.as_object_mut() else {
        return;
    };
    let play_session_id = object
        .get("PlaySessionId")
        .and_then(Value::as_str)
        .map(str::to_string);
    let Some(media_sources) = object.get_mut("MediaSources").and_then(Value::as_array_mut) else {
        return;
    };

    for source in media_sources.iter_mut() {
        let Some(source_obj) = source.as_object_mut() else {
            continue;
        };
        if !media_source_is_remote_for_redirect(source_obj) {
            continue;
        }
        let media_source_id = source_obj
            .get("Id")
            .and_then(Value::as_str)
            .map(str::to_string);
        let container = normalize_original_container_extension(
            source_obj.get("Container").and_then(Value::as_str),
        )
        .unwrap_or_else(|| "mp4".to_string());
        let target = build_playback_original_stream_url(
            raw_item_id,
            container.as_str(),
            media_source_id.as_deref(),
            play_session_id.as_deref(),
            access_token,
            device_id,
        );
        apply_playback_original_stream_target(source_obj, target);
    }
}

fn image_tag_header_value(raw_tag: Option<&str>) -> Option<header::HeaderValue> {
    let tag = raw_tag?.trim().trim_matches('"');
    if tag.is_empty() {
        return None;
    }

    header::HeaderValue::from_str(&format!("\"{tag}\"")).ok()
}

fn infer_image_extension_from_content_type(content_type: Option<&str>) -> &'static str {
    match content_type
        .map(str::trim)
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        "image/gif" => "gif",
        _ => "jpg",
    }
}

fn is_supported_library_image_type(image_type: &str) -> bool {
    matches!(
        image_type.trim().to_ascii_lowercase().as_str(),
        "primary"
            | "poster"
            | "thumb"
            | "thumbnail"
            | "logo"
            | "art"
            | "banner"
            | "backdrop"
            | "fanart"
    )
}

fn normalize_media_source_id_candidate(
    raw_media_source_id: Option<&str>,
    resolved_media_source_id: Option<Uuid>,
) -> Option<String> {
    let raw = raw_media_source_id
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    resolved_media_source_id
        .map(|value| value.to_string())
        .or_else(|| Some(raw.to_string()))
}

async fn normalize_media_source_id_for_lookup(
    state: &ApiContext,
    raw_media_source_id: Option<&str>,
) -> Result<Option<String>, Response> {
    let raw = raw_media_source_id
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let Some(raw) = raw else {
        return Ok(None);
    };
    let resolved = match state.infra.resolve_uuid_by_any_item_id(raw).await {
        Ok(value) => value,
        Err(err) => {
            return Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to resolve media source id: {err}"),
            ));
        }
    };
    Ok(normalize_media_source_id_candidate(Some(raw), resolved))
}

async fn get_playback_info(
    State(state): State<ApiContext>,
    AxPath(raw_item_id): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<PlaybackInfoCompatQuery>,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(user) => user,
        Err(resp) => return resp,
    };

    let uid = parse_compat_uuid(query.user_id.as_deref()).or_else(|| Uuid::parse_str(&auth_user.id).ok());
    let media_source_id =
        match normalize_media_source_id_for_lookup(&state, query.media_source_id.as_deref()).await {
            Ok(value) => value,
            Err(resp) => return resp,
        };
    let max_streaming_bitrate = parse_compat_i64(query._max_streaming_bitrate.as_deref());
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };

    match state
        .infra
        .playback_info(item_id, uid, media_source_id.as_deref(), max_streaming_bitrate)
        .await
    {
        Ok(Some(info)) => {
            let mut payload = serde_json::to_value(info).unwrap_or_else(|_| json!({}));
            let token = extract_token(&headers, &uri);
            let item_id_value = raw_item_id.trim();
            let item_id_fallback = item_id.to_string();
            ensure_playback_info_compat_defaults(
                &mut payload,
                token.as_deref(),
                Some(if item_id_value.is_empty() {
                    item_id_fallback.as_str()
                } else {
                    item_id_value
                }),
            );
            let device_id = extract_emby_authorization_param_from_headers(&headers, "DeviceId")
                .or_else(|| query._device_id.clone());
            ensure_playback_info_original_stream_urls(
                &mut payload,
                raw_item_id.as_str(),
                token.as_deref(),
                device_id.as_deref(),
            )
            ;
            Json(payload).into_response()
        }
        Ok(None) => error_response(StatusCode::NOT_FOUND, "item not found"),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to build playback info: {err}"),
        ),
    }
}

async fn post_playback_info(
    State(state): State<ApiContext>,
    AxPath(raw_item_id): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<PlaybackInfoCompatQuery>,
    Json(request_payload): Json<PlaybackInfoCompatRequest>,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(user) => user,
        Err(resp) => return resp,
    };

    let uid = parse_compat_uuid(request_payload.user_id.as_deref())
        .or_else(|| parse_compat_uuid(query.user_id.as_deref()))
        .or_else(|| Uuid::parse_str(&auth_user.id).ok());
    let media_source_id = match normalize_media_source_id_for_lookup(
        &state,
        request_payload
            .media_source_id
            .as_deref()
            .or(query.media_source_id.as_deref()),
    )
    .await
    {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    let max_streaming_bitrate = request_payload
        ._max_streaming_bitrate
        .as_ref()
        .and_then(playback_value_to_i64)
        .or_else(|| parse_compat_i64(query._max_streaming_bitrate.as_deref()));
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };

    match state
        .infra
        .playback_info(item_id, uid, media_source_id.as_deref(), max_streaming_bitrate)
        .await
    {
        Ok(Some(info)) => {
            let mut payload = serde_json::to_value(info).unwrap_or_else(|_| json!({}));
            let token = extract_token(&headers, &uri);
            let item_id_value = raw_item_id.trim();
            let item_id_fallback = item_id.to_string();
            ensure_playback_info_compat_defaults(
                &mut payload,
                token.as_deref(),
                Some(if item_id_value.is_empty() {
                    item_id_fallback.as_str()
                } else {
                    item_id_value
                }),
            );
            let device_id = extract_emby_authorization_param_from_headers(&headers, "DeviceId")
                .or_else(|| request_payload._device_id.clone())
                .or_else(|| query._device_id.clone());
            ensure_playback_info_original_stream_urls(
                &mut payload,
                raw_item_id.as_str(),
                token.as_deref(),
                device_id.as_deref(),
            )
            ;
            Json(payload).into_response()
        }
        Ok(None) => error_response(StatusCode::NOT_FOUND, "item not found"),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to build playback info: {err}"),
        ),
    }
}

async fn stream_video(
    State(state): State<ApiContext>,
    AxPath(raw_item_id): AxPath<String>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<StreamVideoCompatQuery>,
) -> Response {
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    stream_video_inner(state, item_id, query.media_source_id, headers, uri).await
}

/// GET /Videos/{itemId}/stream.{container} - Stream video with container suffix
/// The container suffix is for client convenience; actual format is determined by query params
async fn stream_video_with_container(
    State(state): State<ApiContext>,
    AxPath((raw_item_id, _container)): AxPath<(String, String)>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<StreamVideoCompatQuery>,
) -> Response {
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    // Container suffix is informational only - reuse the same streaming logic
    stream_video_inner(state, item_id, query.media_source_id, headers, uri).await
}

/// GET /Videos/{itemId}/original.{container} - Emby-style original stream endpoint
/// SenPlayer relies on this path shape from PlaybackInfo DirectStreamUrl.
async fn stream_video_original_with_container(
    State(state): State<ApiContext>,
    AxPath((raw_item_id, _container)): AxPath<(String, String)>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<StreamVideoCompatQuery>,
) -> Response {
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    stream_video_inner(state, item_id, query.media_source_id, headers, uri).await
}

async fn stream_video_inner(
    state: ApiContext,
    item_id: Uuid,
    media_source_id: Option<String>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let auth_user = match require_auth(&state, &headers, &uri).await {
        Ok(user) => user,
        Err(resp) => return resp,
    };

    let user_id = match Uuid::parse_str(&auth_user.id) {
        Ok(v) => v,
        Err(_) => return error_response(StatusCode::UNAUTHORIZED, "invalid user id"),
    };

    if let Err(err) = state.infra.check_stream_admission(user_id).await {
        if let Some(response) = map_stream_admission_error(&err) {
            state
                .metrics
                .stream_failures_total
                .fetch_add(1, Ordering::Relaxed);
            return response;
        }

        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to check stream admission: {err}"),
        );
    }

    state
        .metrics
        .stream_attempts_total
        .fetch_add(1, Ordering::Relaxed);

    let access_token = match extract_token(&headers, &uri) {
        Some(v) => v,
        None => {
            state
                .metrics
                .stream_failures_total
                .fetch_add(1, Ordering::Relaxed);
            return error_response(StatusCode::UNAUTHORIZED, "missing access token");
        }
    };
    let media_source_id =
        match normalize_media_source_id_for_lookup(&state, media_source_id.as_deref()).await {
            Ok(value) => value,
            Err(resp) => return resp,
        };

    let target = match state
        .infra
        .resolve_stream_redirect_target(
            item_id,
            media_source_id.as_deref(),
            user_id,
            access_token.as_str(),
        )
        .await
    {
        Ok(v) => v,
        Err(err) => {
            state
                .metrics
                .stream_failures_total
                .fetch_add(1, Ordering::Relaxed);
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to resolve redirect target: {err}"),
            );
        }
    };

    let Some(target) = target else {
        state
            .metrics
            .stream_failures_total
            .fetch_add(1, Ordering::Relaxed);
        return error_response(StatusCode::NOT_FOUND, "stream url not found");
    };

    state
        .metrics
        .stream_success_total
        .fetch_add(1, Ordering::Relaxed);
    state
        .metrics
        .stream_upstream_total
        .fetch_add(1, Ordering::Relaxed);
    HttpResponse::Found()
        .insert_header((header::LOCATION, target))
        .finish()
}

async fn get_item_subtitles(
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

    match state.infra.list_subtitle_tracks(item_id).await {
        Ok(items) => Json(items).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to query subtitles: {err}"),
        ),
    }
}

async fn stream_item_subtitle(
    State(state): State<ApiContext>,
    AxPath((raw_item_id, subtitle_index)): AxPath<(String, i32)>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<SubtitleStreamCompatQuery>,
) -> Response {
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    stream_subtitle_inner(
        state,
        item_id,
        subtitle_index,
        None,
        None,
        query,
        headers,
        uri,
    )
    .await
}

async fn stream_item_subtitle_with_format(
    State(state): State<ApiContext>,
    AxPath((raw_item_id, subtitle_index, format)): AxPath<(String, i32, String)>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<SubtitleStreamCompatQuery>,
) -> Response {
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    stream_subtitle_inner(
        state,
        item_id,
        subtitle_index,
        None,
        Some(format),
        query,
        headers,
        uri,
    )
    .await
}

async fn stream_item_subtitle_with_media_source(
    State(state): State<ApiContext>,
    AxPath((raw_item_id, media_source_id, subtitle_index)): AxPath<(String, String, i32)>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<SubtitleStreamCompatQuery>,
) -> Response {
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    stream_subtitle_inner(
        state,
        item_id,
        subtitle_index,
        Some(media_source_id),
        None,
        query,
        headers,
        uri,
    )
    .await
}

async fn stream_item_subtitle_with_media_source_and_format(
    State(state): State<ApiContext>,
    AxPath((raw_item_id, media_source_id, subtitle_index, format)): AxPath<(String, String, i32, String)>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<SubtitleStreamCompatQuery>,
) -> Response {
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    stream_subtitle_inner(
        state,
        item_id,
        subtitle_index,
        Some(media_source_id),
        Some(format),
        query,
        headers,
        uri,
    )
    .await
}

async fn stream_video_subtitle(
    State(state): State<ApiContext>,
    AxPath((raw_item_id, subtitle_index)): AxPath<(String, i32)>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<SubtitleStreamCompatQuery>,
) -> Response {
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    stream_subtitle_inner(
        state,
        item_id,
        subtitle_index,
        None,
        None,
        query,
        headers,
        uri,
    )
    .await
}

async fn stream_video_subtitle_with_format(
    State(state): State<ApiContext>,
    AxPath((raw_item_id, subtitle_index, format)): AxPath<(String, i32, String)>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<SubtitleStreamCompatQuery>,
) -> Response {
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    stream_subtitle_inner(
        state,
        item_id,
        subtitle_index,
        None,
        Some(format),
        query,
        headers,
        uri,
    )
    .await
}

async fn stream_video_subtitle_with_media_source(
    State(state): State<ApiContext>,
    AxPath((raw_item_id, media_source_id, subtitle_index)): AxPath<(String, String, i32)>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<SubtitleStreamCompatQuery>,
) -> Response {
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    stream_subtitle_inner(
        state,
        item_id,
        subtitle_index,
        Some(media_source_id),
        None,
        query,
        headers,
        uri,
    )
    .await
}

async fn stream_video_subtitle_with_media_source_and_format(
    State(state): State<ApiContext>,
    AxPath((raw_item_id, media_source_id, subtitle_index, format)): AxPath<(String, String, i32, String)>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<SubtitleStreamCompatQuery>,
) -> Response {
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    stream_subtitle_inner(
        state,
        item_id,
        subtitle_index,
        Some(media_source_id),
        Some(format),
        query,
        headers,
        uri,
    )
    .await
}

async fn stream_subtitle_inner(
    state: ApiContext,
    item_id: Uuid,
    subtitle_index: i32,
    path_media_source_id: Option<String>,
    format_hint: Option<String>,
    query: SubtitleStreamCompatQuery,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }

    let subtitle_item_id = match path_media_source_id
        .as_deref()
        .or(query._media_source_id.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(raw_media_source_id) => match state.infra.resolve_uuid_by_any_item_id(raw_media_source_id).await {
            Ok(Some(media_source_item_id)) => media_source_item_id,
            Ok(None) => item_id,
            Err(err) => {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("failed to resolve media source id: {err}"),
                );
            }
        },
        None => item_id,
    };

    let subtitle_path = match state
        .infra
        .subtitle_path_by_index(subtitle_item_id, subtitle_index)
        .await
    {
        Ok(Some(path)) => path,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "subtitle not found"),
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to resolve subtitle path: {err}"),
            );
        }
    };

    let bytes = match tokio::fs::read(&subtitle_path).await {
        Ok(data) => data,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "subtitle file not found"),
    };

    let codec = format_hint.unwrap_or_else(|| subtitle_codec_from_path(&subtitle_path));
    let content_type = subtitle_content_type(&codec);

    HttpResponse::Ok()
        .insert_header((header::CONTENT_TYPE, content_type))
        .insert_header((header::CACHE_CONTROL, "public, max-age=60"))
        .body(bytes)
}

async fn stream_item_image(
    State(state): State<ApiContext>,
    AxPath((raw_item_id, image_type)): AxPath<(String, String)>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<ImageRequestCompatQuery>,
) -> Response {
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    stream_image_inner(state, item_id, image_type, 0, query, headers, uri).await
}

async fn upload_item_image(
    State(state): State<ApiContext>,
    AxPath((raw_item_id, image_type)): AxPath<(String, String)>,
    headers: HeaderMap,
    uri: Uri,
    body: web::Bytes,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }
    if body.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "empty image body");
    }
    if !is_supported_library_image_type(&image_type) {
        return error_response(StatusCode::BAD_REQUEST, "unsupported image type");
    }
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };

    let extension = infer_image_extension_from_content_type(
        headers.get(header::CONTENT_TYPE).and_then(|v| v.to_str().ok()),
    );

    match state
        .infra
        .save_library_image(item_id, &image_type, &body, extension)
        .await
    {
        Ok(Some(_)) => StatusCode::NO_CONTENT.into_response(),
        Ok(None) => match state
            .infra
            .save_media_item_image(item_id, &image_type, &body, extension)
            .await
        {
            Ok(Some(_)) => StatusCode::NO_CONTENT.into_response(),
            Ok(None) => error_response(StatusCode::NOT_FOUND, "item not found"),
            Err(err) => error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to save item image: {err}"),
            ),
        },
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to save library image: {err}"),
        ),
    }
}

async fn delete_item_image(
    State(state): State<ApiContext>,
    AxPath((raw_item_id, image_type)): AxPath<(String, String)>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers, &uri).await {
        return resp;
    }
    if !is_supported_library_image_type(&image_type) {
        return error_response(StatusCode::BAD_REQUEST, "unsupported image type");
    }
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };

    match state.infra.delete_library_image(item_id, &image_type).await {
        Ok(Some(())) => StatusCode::NO_CONTENT.into_response(),
        Ok(None) => match state.infra.delete_media_item_image(item_id, &image_type).await {
            Ok(Some(())) => StatusCode::NO_CONTENT.into_response(),
            Ok(None) => error_response(StatusCode::NOT_FOUND, "item not found"),
            Err(err) => error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to delete item image: {err}"),
            ),
        },
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to delete library image: {err}"),
        ),
    }
}

async fn stream_item_image_with_index(
    State(state): State<ApiContext>,
    AxPath((raw_item_id, image_type, image_index)): AxPath<(String, String, i32)>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<ImageRequestCompatQuery>,
) -> Response {
    let item_id = match resolve_item_uuid_or_bad_request(&state, &raw_item_id).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    stream_image_inner(
        state,
        item_id,
        image_type,
        image_index,
        query,
        headers,
        uri,
    )
    .await
}

async fn serve_image_with_optional_resize(
    state: &ApiContext,
    source_path: &str,
    resize_request: Option<&ls_infra::ImageResizeRequest>,
    etag: Option<header::HeaderValue>,
    headers: &HeaderMap,
) -> Response {
    let resolved_path = if let Some(request) = resize_request {
        match state.infra.ensure_resized_image(source_path, request).await {
            Ok(path) => path,
            Err(err) => {
                warn!(
                    source_path,
                    error = %err,
                    "failed to materialize resized image, fallback to source image"
                );
                source_path.to_string()
            }
        }
    } else {
        source_path.to_string()
    };

    serve_image_file(&resolved_path, etag, headers).await
}

async fn serve_image_file(
    image_path: &str,
    etag: Option<header::HeaderValue>,
    headers: &HeaderMap,
) -> Response {
    let content_type = image_content_type(image_path);
    let cache_control = if etag.is_some() {
        "public, max-age=31536000, immutable"
    } else {
        "public, max-age=300"
    };

    if let Some(ref etag_val) = etag {
        if let Some(inm) = headers.get(header::IF_NONE_MATCH) {
            if inm.as_bytes() == etag_val.as_bytes() {
                let mut resp = HttpResponse::NotModified();
                resp.insert_header((header::CACHE_CONTROL, cache_control));
                resp.insert_header((header::ETAG, etag_val.clone()));
                return resp.finish();
            }
        }
    }

    let file = match tokio::fs::File::open(image_path).await {
        Ok(f) => f,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "image file not found"),
    };
    let file_size = match file.metadata().await {
        Ok(m) => m.len(),
        Err(_) => return error_response(StatusCode::NOT_FOUND, "image file not found"),
    };

    let stream = ReaderStream::new(file);
    let body = SizedStream::new(file_size, stream);

    let mut resp = HttpResponse::Ok();
    resp.insert_header((header::CONTENT_TYPE, content_type));
    resp.insert_header((header::CACHE_CONTROL, cache_control));
    if let Some(value) = etag {
        resp.insert_header((header::ETAG, value));
    }
    resp.body(body)
}

async fn stream_image_inner(
    state: ApiContext,
    item_id: Uuid,
    image_type: String,
    image_index: i32,
    query: ImageRequestCompatQuery,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if extract_token(&headers, &uri).is_some() {
        if let Err(resp) = require_auth(&state, &headers, &uri).await {
            return resp;
        }
    }

    let image_path = match state
        .infra
        .image_path_for_item(item_id, &image_type, image_index)
        .await
    {
        Ok(Some(path)) => path,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "image not found"),
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to resolve image path: {err}"),
            );
        }
    };

    let resize_request = query.resize_request();
    let etag = image_tag_header_value(query.tag.as_deref());
    serve_image_with_optional_resize(
        &state,
        &image_path,
        resize_request.as_ref(),
        etag,
        &headers,
    )
    .await
}

async fn stream_person_image(
    State(state): State<ApiContext>,
    AxPath((person_id, image_type)): AxPath<(Uuid, String)>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<ImageRequestCompatQuery>,
) -> Response {
    if extract_token(&headers, &uri).is_some() {
        if let Err(resp) = require_auth(&state, &headers, &uri).await {
            return resp;
        }
    }

    let image_path = match state
        .infra
        .image_path_for_person(person_id, &image_type)
        .await
    {
        Ok(Some(path)) => path,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "image not found"),
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to resolve person image path: {err}"),
            );
        }
    };

    let resize_request = query.resize_request();
    let etag = image_tag_header_value(query.tag.as_deref());
    serve_image_with_optional_resize(
        &state,
        &image_path,
        resize_request.as_ref(),
        etag,
        &headers,
    )
    .await
}

async fn logout_session(State(state): State<ApiContext>, headers: HeaderMap, uri: Uri) -> Response {
    if let Err(resp) = require_auth(&state, &headers, &uri).await {
        return resp;
    }

    let Some(token) = extract_token(&headers, &uri) else {
        return error_response(StatusCode::UNAUTHORIZED, "missing access token");
    };

    match state.infra.revoke_access_token(&token).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to revoke token: {err}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct BillingCreateRechargeOrderRequest {
    amount: Decimal,
    channel: Option<String>,
    subject: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AdminBillingPlansQuery {
    include_disabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct AdminBillingPermissionGroupsQuery {
    include_disabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct AdminUpsertBillingPlanRequest {
    id: Option<Uuid>,
    code: String,
    name: String,
    price: Decimal,
    duration_days: i32,
    traffic_quota_bytes: i64,
    traffic_window_days: i32,
    permission_group_id: Option<Uuid>,
    enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct AdminUpsertBillingPermissionGroupRequest {
    id: Option<Uuid>,
    code: String,
    name: String,
    domain_ids: Vec<Uuid>,
    enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct AdminBillingRechargeOrdersQuery {
    user_id: Option<Uuid>,
    status: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct AdminWalletLedgerQuery {
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct AdminUserSubscriptionsQuery {
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct AdminAdjustWalletBalanceRequest {
    amount: Decimal,
    note: Option<String>,
}
