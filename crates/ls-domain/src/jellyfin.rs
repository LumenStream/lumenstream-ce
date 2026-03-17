use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthenticateByNameRequest {
    #[serde(
        rename = "Username",
        alias = "username",
        alias = "UserName",
        alias = "userName",
        alias = "Name",
        alias = "name"
    )]
    pub username: String,
    #[serde(rename = "Pw", alias = "pw", default)]
    pub pw: Option<String>,
    #[serde(rename = "Password", alias = "password", default)]
    pub password: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UserDto {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "HasPassword")]
    pub has_password: bool,
    #[serde(rename = "HasConfiguredPassword")]
    pub has_configured_password: bool,
    #[serde(rename = "HasConfiguredEasyPassword")]
    pub has_configured_easy_password: bool,
    #[serde(rename = "EnableAutoLogin")]
    pub enable_auto_login: bool,
    #[serde(rename = "ServerId")]
    pub server_id: String,
    #[serde(rename = "ServerName", skip_serializing_if = "Option::is_none")]
    pub server_name: Option<String>,
    #[serde(rename = "ConnectUserName", skip_serializing_if = "Option::is_none")]
    pub connect_user_name: Option<String>,
    #[serde(rename = "ConnectLinkType", skip_serializing_if = "Option::is_none")]
    pub connect_link_type: Option<String>,
    #[serde(rename = "PrimaryImageTag", skip_serializing_if = "Option::is_none")]
    pub primary_image_tag: Option<String>,
    #[serde(rename = "LastLoginDate", skip_serializing_if = "Option::is_none")]
    pub last_login_date: Option<String>,
    #[serde(rename = "LastActivityDate", skip_serializing_if = "Option::is_none")]
    pub last_activity_date: Option<String>,
    #[serde(rename = "Configuration", skip_serializing_if = "Option::is_none")]
    pub configuration: Option<Value>,
    #[serde(
        rename = "PrimaryImageAspectRatio",
        skip_serializing_if = "Option::is_none"
    )]
    pub primary_image_aspect_ratio: Option<f64>,
    #[serde(rename = "Policy")]
    pub policy: UserPolicyDto,
}

#[derive(Debug, Serialize)]
pub struct UserPolicyDto {
    #[serde(rename = "IsAdministrator")]
    pub is_administrator: bool,
    #[serde(rename = "IsHidden")]
    pub is_hidden: bool,
    #[serde(rename = "IsHiddenRemotely")]
    pub is_hidden_remotely: bool,
    #[serde(rename = "IsHiddenFromUnusedDevices")]
    pub is_hidden_from_unused_devices: bool,
    #[serde(rename = "IsDisabled")]
    pub is_disabled: bool,
    #[serde(rename = "LockedOutDate")]
    pub locked_out_date: i64,
    #[serde(rename = "AllowTagOrRating")]
    pub allow_tag_or_rating: bool,
    #[serde(rename = "BlockedTags")]
    pub blocked_tags: Vec<String>,
    #[serde(rename = "IsTagBlockingModeInclusive")]
    pub is_tag_blocking_mode_inclusive: bool,
    #[serde(rename = "IncludeTags")]
    pub include_tags: Vec<String>,
    #[serde(rename = "EnableUserPreferenceAccess")]
    pub enable_user_preference_access: bool,
    #[serde(rename = "AccessSchedules")]
    pub access_schedules: Vec<Value>,
    #[serde(rename = "BlockUnratedItems")]
    pub block_unrated_items: Vec<String>,
    #[serde(rename = "EnableRemoteControlOfOtherUsers")]
    pub enable_remote_control_of_other_users: bool,
    #[serde(rename = "EnableSharedDeviceControl")]
    pub enable_shared_device_control: bool,
    #[serde(rename = "EnableRemoteAccess")]
    pub enable_remote_access: bool,
    #[serde(rename = "EnableLiveTvManagement")]
    pub enable_live_tv_management: bool,
    #[serde(rename = "EnableLiveTvAccess")]
    pub enable_live_tv_access: bool,
    #[serde(rename = "EnableMediaPlayback")]
    pub enable_media_playback: bool,
    #[serde(rename = "EnableAudioPlaybackTranscoding")]
    pub enable_audio_playback_transcoding: bool,
    #[serde(rename = "EnableVideoPlaybackTranscoding")]
    pub enable_video_playback_transcoding: bool,
    #[serde(rename = "EnablePlaybackRemuxing")]
    pub enable_playback_remuxing: bool,
    #[serde(rename = "EnableContentDeletion")]
    pub enable_content_deletion: bool,
    #[serde(rename = "RestrictedFeatures")]
    pub restricted_features: Vec<String>,
    #[serde(rename = "EnableContentDeletionFromFolders")]
    pub enable_content_deletion_from_folders: Vec<String>,
    #[serde(rename = "EnableContentDownloading")]
    pub enable_content_downloading: bool,
    #[serde(rename = "EnableSubtitleDownloading")]
    pub enable_subtitle_downloading: bool,
    #[serde(rename = "EnableSubtitleManagement")]
    pub enable_subtitle_management: bool,
    #[serde(rename = "EnableSyncTranscoding")]
    pub enable_sync_transcoding: bool,
    #[serde(rename = "EnableMediaConversion")]
    pub enable_media_conversion: bool,
    #[serde(rename = "EnabledChannels")]
    pub enabled_channels: Vec<String>,
    #[serde(rename = "EnableAllChannels")]
    pub enable_all_channels: bool,
    #[serde(rename = "EnabledFolders")]
    pub enabled_folders: Vec<String>,
    #[serde(rename = "EnableAllFolders")]
    pub enable_all_folders: bool,
    #[serde(rename = "InvalidLoginAttemptCount")]
    pub invalid_login_attempt_count: i32,
    #[serde(rename = "EnablePublicSharing")]
    pub enable_public_sharing: bool,
    #[serde(rename = "RemoteClientBitrateLimit")]
    pub remote_client_bitrate_limit: i32,
    #[serde(rename = "AuthenticationProviderId")]
    pub authentication_provider_id: String,
    #[serde(rename = "ExcludedSubFolders")]
    pub excluded_sub_folders: Vec<String>,
    #[serde(rename = "SimultaneousStreamLimit")]
    pub simultaneous_stream_limit: i32,
    #[serde(rename = "EnabledDevices")]
    pub enabled_devices: Vec<String>,
    #[serde(rename = "EnableAllDevices")]
    pub enable_all_devices: bool,
    #[serde(rename = "AllowCameraUpload")]
    pub allow_camera_upload: bool,
    #[serde(rename = "AllowSharingPersonalItems")]
    pub allow_sharing_personal_items: bool,
    #[serde(rename = "Role", skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SessionInfoDto {
    #[serde(rename = "PlayState", skip_serializing_if = "Option::is_none")]
    pub play_state: Option<Value>,
    #[serde(rename = "AdditionalUsers", skip_serializing_if = "Option::is_none")]
    pub additional_users: Option<Vec<Value>>,
    #[serde(rename = "RemoteEndPoint", skip_serializing_if = "Option::is_none")]
    pub remote_end_point: Option<String>,
    #[serde(rename = "Protocol", skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    #[serde(rename = "PlayableMediaTypes", skip_serializing_if = "Option::is_none")]
    pub playable_media_types: Option<Vec<String>>,
    #[serde(rename = "PlaylistIndex", skip_serializing_if = "Option::is_none")]
    pub playlist_index: Option<i32>,
    #[serde(rename = "PlaylistLength", skip_serializing_if = "Option::is_none")]
    pub playlist_length: Option<i32>,
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "UserId")]
    pub user_id: String,
    #[serde(rename = "UserName")]
    pub user_name: String,
    #[serde(rename = "Client")]
    pub client: String,
    #[serde(rename = "DeviceName")]
    pub device_name: String,
    #[serde(rename = "DeviceId")]
    pub device_id: String,
    #[serde(rename = "ServerId", skip_serializing_if = "Option::is_none")]
    pub server_id: Option<String>,
    #[serde(rename = "LastActivityDate", skip_serializing_if = "Option::is_none")]
    pub last_activity_date: Option<String>,
    #[serde(rename = "ApplicationVersion", skip_serializing_if = "Option::is_none")]
    pub application_version: Option<String>,
    #[serde(rename = "DeviceType", skip_serializing_if = "Option::is_none")]
    pub device_type: Option<String>,
    #[serde(rename = "SupportedCommands", skip_serializing_if = "Option::is_none")]
    pub supported_commands: Option<Vec<String>>,
    #[serde(rename = "InternalDeviceId", skip_serializing_if = "Option::is_none")]
    pub internal_device_id: Option<i64>,
    #[serde(
        rename = "SupportsRemoteControl",
        skip_serializing_if = "Option::is_none"
    )]
    pub supports_remote_control: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct AuthenticationResultDto {
    #[serde(rename = "User")]
    pub user: UserDto,
    #[serde(rename = "SessionInfo")]
    pub session_info: SessionInfoDto,
    #[serde(rename = "AccessToken")]
    pub access_token: String,
    #[serde(rename = "ServerId")]
    pub server_id: String,
}

#[derive(Debug, Serialize)]
pub struct QueryResultDto<T> {
    #[serde(rename = "Items")]
    pub items: Vec<T>,
    #[serde(rename = "TotalRecordCount")]
    pub total_record_count: i32,
    #[serde(rename = "StartIndex")]
    pub start_index: i32,
}

#[derive(Debug, Serialize)]
pub struct BaseItemDto {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Type")]
    pub item_type: String,
    #[serde(rename = "Path")]
    pub path: String,
    #[serde(rename = "IsFolder", skip_serializing_if = "Option::is_none")]
    pub is_folder: Option<bool>,
    #[serde(rename = "MediaType", skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    #[serde(rename = "Container", skip_serializing_if = "Option::is_none")]
    pub container: Option<String>,
    #[serde(rename = "LocationType", skip_serializing_if = "Option::is_none")]
    pub location_type: Option<String>,
    #[serde(rename = "CanDelete", skip_serializing_if = "Option::is_none")]
    pub can_delete: Option<bool>,
    #[serde(rename = "CanDownload", skip_serializing_if = "Option::is_none")]
    pub can_download: Option<bool>,
    #[serde(rename = "CollectionType", skip_serializing_if = "Option::is_none")]
    pub collection_type: Option<String>,
    #[serde(rename = "RunTimeTicks", skip_serializing_if = "Option::is_none")]
    pub runtime_ticks: Option<i64>,
    #[serde(rename = "Bitrate", skip_serializing_if = "Option::is_none")]
    pub bitrate: Option<i32>,
    #[serde(rename = "MediaSources", skip_serializing_if = "Option::is_none")]
    pub media_sources: Option<Vec<MediaSourceInfoDto>>,
    #[serde(rename = "UserData", skip_serializing_if = "Option::is_none")]
    pub user_data: Option<UserDataDto>,

    // P0 Fields
    #[serde(rename = "Overview", skip_serializing_if = "Option::is_none")]
    pub overview: Option<String>,
    #[serde(rename = "PremiereDate", skip_serializing_if = "Option::is_none")]
    pub premiere_date: Option<String>,
    #[serde(rename = "EndDate", skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,
    #[serde(rename = "ProductionYear", skip_serializing_if = "Option::is_none")]
    pub production_year: Option<i32>,
    #[serde(rename = "Genres", skip_serializing_if = "Option::is_none")]
    pub genres: Option<Vec<String>>,
    #[serde(rename = "Tags", skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(rename = "ProviderIds", skip_serializing_if = "Option::is_none")]
    pub provider_ids: Option<HashMap<String, String>>,
    #[serde(rename = "ImageTags", skip_serializing_if = "Option::is_none")]
    pub image_tags: Option<HashMap<String, String>>,
    #[serde(rename = "PrimaryImageTag", skip_serializing_if = "Option::is_none")]
    pub primary_image_tag: Option<String>,
    #[serde(rename = "ParentId", skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(rename = "SeriesId", skip_serializing_if = "Option::is_none")]
    pub series_id: Option<String>,
    #[serde(rename = "SeriesName", skip_serializing_if = "Option::is_none")]
    pub series_name: Option<String>,
    #[serde(rename = "SeasonId", skip_serializing_if = "Option::is_none")]
    pub season_id: Option<String>,
    #[serde(rename = "SeasonName", skip_serializing_if = "Option::is_none")]
    pub season_name: Option<String>,
    #[serde(rename = "IndexNumber", skip_serializing_if = "Option::is_none")]
    pub index_number: Option<i32>,
    #[serde(rename = "ParentIndexNumber", skip_serializing_if = "Option::is_none")]
    pub parent_index_number: Option<i32>,

    // P1 Fields
    #[serde(rename = "BackdropImageTags", skip_serializing_if = "Option::is_none")]
    pub backdrop_image_tags: Option<Vec<String>>,
    #[serde(rename = "OfficialRating", skip_serializing_if = "Option::is_none")]
    pub official_rating: Option<String>,
    #[serde(rename = "CommunityRating", skip_serializing_if = "Option::is_none")]
    pub community_rating: Option<f64>,
    #[serde(rename = "Studios", skip_serializing_if = "Option::is_none")]
    pub studios: Option<Vec<NameGuidPairDto>>,
    #[serde(rename = "People", skip_serializing_if = "Option::is_none")]
    pub people: Option<Vec<BaseItemPersonDto>>,
    #[serde(rename = "SortName", skip_serializing_if = "Option::is_none")]
    pub sort_name: Option<String>,
    #[serde(
        rename = "PrimaryImageAspectRatio",
        skip_serializing_if = "Option::is_none"
    )]
    pub primary_image_aspect_ratio: Option<f64>,
    #[serde(rename = "DateCreated", skip_serializing_if = "Option::is_none")]
    pub date_created: Option<String>,
    #[serde(rename = "ChildCount", skip_serializing_if = "Option::is_none")]
    pub child_count: Option<i32>,
    #[serde(rename = "RecursiveItemCount", skip_serializing_if = "Option::is_none")]
    pub recursive_item_count: Option<i32>,
    #[serde(rename = "PlayAccess", skip_serializing_if = "Option::is_none")]
    pub play_access: Option<String>,
}

/// Studio/genre name-guid pair for Jellyfin API
#[derive(Debug, Serialize, Clone)]
pub struct NameGuidPairDto {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Id", skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

/// Person (cast/crew) for Jellyfin API
#[derive(Debug, Serialize, Clone)]
pub struct BaseItemPersonDto {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Id", skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "Role", skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(rename = "Type", skip_serializing_if = "Option::is_none")]
    pub person_type: Option<String>,
    #[serde(rename = "PrimaryImageTag", skip_serializing_if = "Option::is_none")]
    pub primary_image_tag: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UserDataDto {
    #[serde(rename = "Played")]
    pub played: bool,
    #[serde(rename = "PlaybackPositionTicks")]
    pub playback_position_ticks: i64,
    #[serde(rename = "IsFavorite", skip_serializing_if = "Option::is_none")]
    pub is_favorite: Option<bool>,
}

#[derive(Debug, Serialize, Clone)]
pub struct MediaStreamDto {
    #[serde(rename = "Index")]
    pub index: i32,
    #[serde(rename = "Type")]
    pub stream_type: String,
    #[serde(rename = "Language", skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(rename = "IsExternal")]
    pub is_external: bool,
    #[serde(rename = "Path", skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(rename = "Codec", skip_serializing_if = "Option::is_none")]
    pub codec: Option<String>,
    #[serde(rename = "DisplayTitle", skip_serializing_if = "Option::is_none")]
    pub display_title: Option<String>,
    #[serde(rename = "Width", skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,
    #[serde(rename = "Height", skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,
    #[serde(rename = "AverageFrameRate", skip_serializing_if = "Option::is_none")]
    pub average_frame_rate: Option<f64>,
    #[serde(rename = "RealFrameRate", skip_serializing_if = "Option::is_none")]
    pub real_frame_rate: Option<f64>,
    #[serde(rename = "Profile", skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    #[serde(rename = "Level", skip_serializing_if = "Option::is_none")]
    pub level: Option<i32>,
    #[serde(rename = "Channels", skip_serializing_if = "Option::is_none")]
    pub channels: Option<i32>,
    #[serde(rename = "SampleRate", skip_serializing_if = "Option::is_none")]
    pub sample_rate: Option<i32>,
    #[serde(rename = "ChannelLayout", skip_serializing_if = "Option::is_none")]
    pub channel_layout: Option<String>,
    #[serde(rename = "BitRate", skip_serializing_if = "Option::is_none")]
    pub bit_rate: Option<i32>,
    // Video HDR / color metadata (best-effort; sourced from Jellyfin-style mediainfo JSON or ffprobe)
    #[serde(rename = "ColorRange", skip_serializing_if = "Option::is_none")]
    pub color_range: Option<String>,
    #[serde(rename = "ColorSpace", skip_serializing_if = "Option::is_none")]
    pub color_space: Option<String>,
    #[serde(rename = "ColorTransfer", skip_serializing_if = "Option::is_none")]
    pub color_transfer: Option<String>,
    #[serde(rename = "ColorPrimaries", skip_serializing_if = "Option::is_none")]
    pub color_primaries: Option<String>,
    #[serde(rename = "BitDepth", skip_serializing_if = "Option::is_none")]
    pub bit_depth: Option<i32>,
    #[serde(rename = "VideoRange", skip_serializing_if = "Option::is_none")]
    pub video_range: Option<String>,
    #[serde(rename = "VideoRangeType", skip_serializing_if = "Option::is_none")]
    pub video_range_type: Option<String>,
    #[serde(
        rename = "Hdr10PlusPresentFlag",
        skip_serializing_if = "Option::is_none"
    )]
    pub hdr10_plus_present_flag: Option<bool>,
    #[serde(rename = "DvVersionMajor", skip_serializing_if = "Option::is_none")]
    pub dv_version_major: Option<i32>,
    #[serde(rename = "DvVersionMinor", skip_serializing_if = "Option::is_none")]
    pub dv_version_minor: Option<i32>,
    #[serde(rename = "DvProfile", skip_serializing_if = "Option::is_none")]
    pub dv_profile: Option<i32>,
    #[serde(rename = "DvLevel", skip_serializing_if = "Option::is_none")]
    pub dv_level: Option<i32>,
    #[serde(rename = "RpuPresentFlag", skip_serializing_if = "Option::is_none")]
    pub rpu_present_flag: Option<bool>,
    #[serde(rename = "ElPresentFlag", skip_serializing_if = "Option::is_none")]
    pub el_present_flag: Option<bool>,
    #[serde(rename = "BlPresentFlag", skip_serializing_if = "Option::is_none")]
    pub bl_present_flag: Option<bool>,
    #[serde(
        rename = "DvBlSignalCompatibilityId",
        skip_serializing_if = "Option::is_none"
    )]
    pub dv_bl_signal_compatibility_id: Option<i32>,
    #[serde(rename = "IsDefault", skip_serializing_if = "Option::is_none")]
    pub is_default: Option<bool>,
    #[serde(rename = "IsForced", skip_serializing_if = "Option::is_none")]
    pub is_forced: Option<bool>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ChapterInfoDto {
    #[serde(rename = "StartPositionTicks")]
    pub start_position_ticks: i64,
    #[serde(rename = "Name", skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "ImageTag", skip_serializing_if = "Option::is_none")]
    pub image_tag: Option<String>,
    #[serde(rename = "MarkerType", skip_serializing_if = "Option::is_none")]
    pub marker_type: Option<String>,
    #[serde(rename = "ChapterIndex", skip_serializing_if = "Option::is_none")]
    pub chapter_index: Option<i32>,
}

#[derive(Debug, Serialize, Clone)]
pub struct MediaSourceInfoDto {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "Name", skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "Path")]
    pub path: Option<String>,
    #[serde(rename = "Protocol")]
    pub protocol: String,
    #[serde(rename = "Container")]
    pub container: Option<String>,
    #[serde(rename = "RunTimeTicks")]
    pub runtime_ticks: Option<i64>,
    #[serde(rename = "Bitrate")]
    pub bitrate: Option<i32>,
    #[serde(rename = "SupportsDirectPlay")]
    pub supports_direct_play: bool,
    #[serde(rename = "SupportsDirectStream")]
    pub supports_direct_stream: bool,
    #[serde(rename = "SupportsTranscoding")]
    pub supports_transcoding: bool,
    #[serde(rename = "Chapters", skip_serializing_if = "Vec::is_empty", default)]
    pub chapters: Vec<ChapterInfoDto>,
    #[serde(rename = "MediaStreams")]
    pub media_streams: Vec<MediaStreamDto>,
}

#[derive(Debug, Serialize)]
pub struct PlaybackInfoResponseDto {
    #[serde(rename = "MediaSources")]
    pub media_sources: Vec<MediaSourceInfoDto>,
    #[serde(rename = "PlaySessionId")]
    pub play_session_id: String,
}

#[derive(Debug, Serialize)]
pub struct SubtitleTrackDto {
    #[serde(rename = "Index")]
    pub index: i32,
    #[serde(rename = "Codec")]
    pub codec: String,
    #[serde(rename = "Language")]
    pub language: Option<String>,
    #[serde(rename = "DisplayTitle")]
    pub display_title: String,
    #[serde(rename = "IsExternal")]
    pub is_external: bool,
    #[serde(rename = "IsDefault")]
    pub is_default: bool,
}

#[derive(Debug, Serialize)]
pub struct ItemCountsDto {
    #[serde(rename = "MovieCount")]
    pub movie_count: i64,
    #[serde(rename = "SeriesCount")]
    pub series_count: i64,
    #[serde(rename = "EpisodeCount")]
    pub episode_count: i64,
    #[serde(rename = "SongCount")]
    pub song_count: i64,
    #[serde(rename = "AlbumCount")]
    pub album_count: i64,
    #[serde(rename = "ArtistCount")]
    pub artist_count: i64,
    #[serde(rename = "MusicVideoCount")]
    pub music_video_count: i64,
    #[serde(rename = "BoxSetCount")]
    pub box_set_count: i64,
    #[serde(rename = "BookCount")]
    pub book_count: i64,
    #[serde(rename = "GameCount")]
    pub game_count: i64,
    #[serde(rename = "GameSystemCount")]
    pub game_system_count: i64,
    #[serde(rename = "ItemCount")]
    pub item_count: i64,
    #[serde(rename = "ProgramCount")]
    pub program_count: i64,
    #[serde(rename = "TrailerCount")]
    pub trailer_count: i64,
}

#[derive(Debug, Deserialize)]
pub struct PlaybackInfoRequestDto {
    #[serde(rename = "UserId")]
    pub user_id: Option<Uuid>,
    #[serde(rename = "MediaSourceId")]
    pub media_source_id: Option<String>,
    #[serde(rename = "MaxStreamingBitrate")]
    pub max_streaming_bitrate: Option<i32>,
}

/// Public system info returned by /System/Info/Public (no auth required)
#[derive(Debug, Serialize)]
pub struct PublicSystemInfoDto {
    #[serde(rename = "LocalAddress", skip_serializing_if = "Option::is_none")]
    pub local_address: Option<String>,
    #[serde(rename = "WanAddress", skip_serializing_if = "Option::is_none")]
    pub wan_address: Option<String>,
    #[serde(rename = "ServerName")]
    pub server_name: String,
    #[serde(rename = "Version")]
    pub version: String,
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "LocalAddresses")]
    pub local_addresses: Vec<String>,
    #[serde(rename = "RemoteAddresses")]
    pub remote_addresses: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct WakeOnLanInfoDto {
    #[serde(rename = "BroadcastAddress")]
    pub broadcast_address: String,
    #[serde(rename = "MacAddress")]
    pub mac_address: String,
    #[serde(rename = "Port")]
    pub port: i32,
}

/// Full system info returned by /System/Info (auth required)
#[derive(Debug, Serialize)]
pub struct SystemInfoDto {
    #[serde(rename = "SystemUpdateLevel", skip_serializing_if = "Option::is_none")]
    pub system_update_level: Option<String>,
    #[serde(
        rename = "OperatingSystemDisplayName",
        skip_serializing_if = "Option::is_none"
    )]
    pub operating_system_display_name: Option<String>,
    #[serde(rename = "PackageName", skip_serializing_if = "Option::is_none")]
    pub package_name: Option<String>,
    #[serde(rename = "SupportsLibraryMonitor")]
    pub supports_library_monitor: bool,
    #[serde(rename = "WebSocketPortNumber")]
    pub web_socket_port_number: i32,
    #[serde(
        rename = "CompletedInstallations",
        skip_serializing_if = "Option::is_none"
    )]
    pub completed_installations: Option<Vec<Value>>,
    #[serde(rename = "HasImageEnhancers")]
    pub has_image_enhancers: bool,
    #[serde(rename = "SupportsLocalPortConfiguration")]
    pub supports_local_port_configuration: bool,
    #[serde(rename = "SupportsWakeServer")]
    pub supports_wake_server: bool,
    #[serde(rename = "WakeOnLanInfo")]
    pub wake_on_lan_info: Vec<WakeOnLanInfoDto>,
    #[serde(rename = "IsInMaintenanceMode")]
    pub is_in_maintenance_mode: bool,
    #[serde(rename = "ServerName")]
    pub server_name: String,
    #[serde(rename = "Version")]
    pub version: String,
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "LocalAddress")]
    pub local_address: String,
    #[serde(rename = "LocalAddresses")]
    pub local_addresses: Vec<String>,
    #[serde(rename = "StartupWizardCompleted")]
    pub startup_wizard_completed: bool,
    #[serde(rename = "ProductName")]
    pub product_name: String,
    #[serde(rename = "OperatingSystem")]
    pub operating_system: String,
    #[serde(rename = "ProgramDataPath")]
    pub program_data_path: String,
    #[serde(rename = "ItemsByNamePath")]
    pub items_by_name_path: String,
    #[serde(rename = "CachePath")]
    pub cache_path: String,
    #[serde(rename = "LogPath")]
    pub log_path: String,
    #[serde(rename = "InternalMetadataPath")]
    pub internal_metadata_path: String,
    #[serde(rename = "TranscodingTempPath")]
    pub transcoding_temp_path: String,
    #[serde(rename = "HttpServerPortNumber")]
    pub http_server_port_number: i32,
    #[serde(rename = "SupportsHttps")]
    pub supports_https: bool,
    #[serde(rename = "HttpsPortNumber")]
    pub https_port_number: i32,
    #[serde(rename = "HasPendingRestart")]
    pub has_pending_restart: bool,
    #[serde(rename = "IsShuttingDown")]
    pub is_shutting_down: bool,
    #[serde(rename = "CanSelfRestart")]
    pub can_self_restart: bool,
    #[serde(rename = "CanSelfUpdate")]
    pub can_self_update: bool,
    #[serde(rename = "CanLaunchWebBrowser")]
    pub can_launch_web_browser: bool,
    #[serde(rename = "HasUpdateAvailable")]
    pub has_update_available: bool,
    #[serde(rename = "SupportsAutoRunAtStartup")]
    pub supports_auto_run_at_startup: bool,
    #[serde(rename = "HardwareAccelerationRequiresPremiere")]
    pub hardware_acceleration_requires_premiere: bool,
    #[serde(rename = "WanAddress", skip_serializing_if = "Option::is_none")]
    pub wan_address: Option<String>,
    #[serde(rename = "RemoteAddresses")]
    pub remote_addresses: Vec<String>,
}

/// User item data returned by playstate endpoints (mark played, favorites, etc.)
#[derive(Debug, Serialize)]
pub struct UserItemDataDto {
    #[serde(rename = "PlaybackPositionTicks")]
    pub playback_position_ticks: i64,
    #[serde(rename = "PlayCount")]
    pub play_count: i32,
    #[serde(rename = "IsFavorite")]
    pub is_favorite: bool,
    #[serde(rename = "Played")]
    pub played: bool,
    #[serde(rename = "LastPlayedDate", skip_serializing_if = "Option::is_none")]
    pub last_played_date: Option<String>,
    #[serde(rename = "ItemId")]
    pub item_id: String,
}

#[derive(Debug, Deserialize)]
pub struct PlaybackProgressDto {
    #[serde(rename = "PlaySessionId", alias = "playSessionId")]
    pub play_session_id: Option<String>,
    #[serde(
        rename = "ItemId",
        alias = "itemId",
        deserialize_with = "deserialize_optional_string_or_number",
        default
    )]
    pub item_id: Option<String>,
    #[serde(
        rename = "PositionTicks",
        alias = "positionTicks",
        alias = "PlaybackPositionTicks",
        alias = "playbackPositionTicks"
    )]
    pub position_ticks: Option<i64>,
    #[serde(rename = "PlayMethod", alias = "playMethod")]
    pub play_method: Option<String>,
    #[serde(rename = "DeviceName", alias = "deviceName")]
    pub device_name: Option<String>,
    #[serde(rename = "Client", alias = "client")]
    pub client: Option<String>,
    #[serde(flatten)]
    pub extra: Value,
}

fn deserialize_optional_string_or_number<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    let parsed = match value {
        None | Some(Value::Null) => None,
        Some(Value::String(raw)) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        Some(Value::Number(raw)) => Some(raw.to_string()),
        Some(other) => {
            return Err(serde::de::Error::custom(format!(
                "expected string or number for item id, got {other}"
            )));
        }
    };
    Ok(parsed)
}

/// Search hint result for /Search/Hints endpoint
#[derive(Debug, Serialize)]
pub struct SearchHintResult {
    #[serde(rename = "SearchHints")]
    pub search_hints: Vec<SearchHint>,
    #[serde(rename = "TotalRecordCount")]
    pub total_record_count: i32,
}

/// Individual search hint item
#[derive(Debug, Serialize)]
pub struct SearchHint {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Type")]
    pub item_type: String,
    #[serde(rename = "ProductionYear", skip_serializing_if = "Option::is_none")]
    pub production_year: Option<i32>,
    #[serde(rename = "PrimaryImageTag", skip_serializing_if = "Option::is_none")]
    pub primary_image_tag: Option<String>,
    #[serde(rename = "ThumbImageTag", skip_serializing_if = "Option::is_none")]
    pub thumb_image_tag: Option<String>,
}

/// Query filters for /Items/Filters endpoint
#[derive(Debug, Serialize)]
pub struct QueryFilters {
    #[serde(rename = "Genres")]
    pub genres: Vec<String>,
    #[serde(rename = "Years")]
    pub years: Vec<i32>,
    #[serde(rename = "OfficialRatings")]
    pub official_ratings: Vec<String>,
    #[serde(rename = "Tags")]
    pub tags: Vec<String>,
}

/// Request body for POST /Users/New
#[derive(Debug, Deserialize)]
pub struct CreateUserByName {
    #[serde(
        rename = "Name",
        alias = "name",
        alias = "UserName",
        alias = "userName",
        alias = "Username",
        alias = "username"
    )]
    pub name: String,
    #[serde(rename = "Password", alias = "password", alias = "Pw", alias = "pw")]
    pub password: Option<String>,
}

fn deserialize_optional_bool_compat<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    Ok(match value {
        None | Some(Value::Null) => None,
        Some(Value::Bool(v)) => Some(v),
        Some(Value::Number(v)) => match v.as_i64() {
            Some(0) => Some(false),
            Some(1) => Some(true),
            _ => None,
        },
        Some(Value::String(raw)) => {
            let normalized = raw.trim().to_ascii_lowercase();
            if normalized.is_empty() {
                None
            } else {
                match normalized.as_str() {
                    "1" | "true" | "yes" | "on" => Some(true),
                    "0" | "false" | "no" | "off" => Some(false),
                    _ => None,
                }
            }
        }
        _ => None,
    })
}

fn deserialize_optional_i32_compat<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    Ok(match value {
        None | Some(Value::Null) => None,
        Some(Value::Number(v)) => v.as_i64().and_then(|n| i32::try_from(n).ok()),
        Some(Value::String(raw)) => raw.trim().parse::<i32>().ok(),
        _ => None,
    })
}

/// Request body for POST /Users/{userId}/Password
#[derive(Debug, Deserialize)]
pub struct UpdateUserPassword {
    #[serde(
        rename = "CurrentPw",
        alias = "currentPw",
        alias = "CurrentPassword",
        alias = "currentPassword"
    )]
    pub current_pw: Option<String>,
    #[serde(
        rename = "NewPw",
        alias = "newPw",
        alias = "NewPassword",
        alias = "newPassword"
    )]
    pub new_pw: Option<String>,
    #[serde(
        rename = "ResetPassword",
        alias = "resetPassword",
        default,
        deserialize_with = "deserialize_optional_bool_compat"
    )]
    pub reset_password: Option<bool>,
}

/// Request body for POST /Users/{userId}/Configuration
#[derive(Debug, Deserialize)]
pub struct UserConfiguration {
    #[serde(rename = "AudioLanguagePreference", alias = "audioLanguagePreference")]
    pub audio_language_preference: Option<String>,
    #[serde(
        rename = "SubtitleLanguagePreference",
        alias = "subtitleLanguagePreference"
    )]
    pub subtitle_language_preference: Option<String>,
    #[serde(
        rename = "PlayDefaultAudioTrack",
        alias = "playDefaultAudioTrack",
        default,
        deserialize_with = "deserialize_optional_bool_compat"
    )]
    pub play_default_audio_track: Option<bool>,
    #[serde(
        rename = "RememberAudioSelections",
        alias = "rememberAudioSelections",
        default,
        deserialize_with = "deserialize_optional_bool_compat"
    )]
    pub remember_audio_selections: Option<bool>,
    #[serde(
        rename = "RememberSubtitleSelections",
        alias = "rememberSubtitleSelections",
        default,
        deserialize_with = "deserialize_optional_bool_compat"
    )]
    pub remember_subtitle_selections: Option<bool>,
    #[serde(rename = "SubtitleMode", alias = "subtitleMode")]
    pub subtitle_mode: Option<String>,
    #[serde(
        rename = "EnableNextEpisodeAutoPlay",
        alias = "enableNextEpisodeAutoPlay",
        default,
        deserialize_with = "deserialize_optional_bool_compat"
    )]
    pub enable_next_episode_auto_play: Option<bool>,
    #[serde(
        rename = "DisplayMissingEpisodes",
        alias = "displayMissingEpisodes",
        default,
        deserialize_with = "deserialize_optional_bool_compat"
    )]
    pub display_missing_episodes: Option<bool>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// Request body for POST /Users/{userId}/Policy
#[derive(Debug, Deserialize)]
pub struct UserPolicyUpdate {
    #[serde(
        rename = "IsAdministrator",
        alias = "isAdministrator",
        default,
        deserialize_with = "deserialize_optional_bool_compat"
    )]
    pub is_administrator: Option<bool>,
    #[serde(
        rename = "IsDisabled",
        alias = "isDisabled",
        default,
        deserialize_with = "deserialize_optional_bool_compat"
    )]
    pub is_disabled: Option<bool>,
    #[serde(
        rename = "EnableAllFolders",
        alias = "enableAllFolders",
        default,
        deserialize_with = "deserialize_optional_bool_compat"
    )]
    pub enable_all_folders: Option<bool>,
    #[serde(
        rename = "EnableMediaPlayback",
        alias = "enableMediaPlayback",
        default,
        deserialize_with = "deserialize_optional_bool_compat"
    )]
    pub enable_media_playback: Option<bool>,
    #[serde(
        rename = "EnableContentDeletion",
        alias = "enableContentDeletion",
        default,
        deserialize_with = "deserialize_optional_bool_compat"
    )]
    pub enable_content_deletion: Option<bool>,
    #[serde(
        rename = "EnableContentDownloading",
        alias = "enableContentDownloading",
        default,
        deserialize_with = "deserialize_optional_bool_compat"
    )]
    pub enable_content_downloading: Option<bool>,
    #[serde(
        rename = "EnableRemoteAccess",
        alias = "enableRemoteAccess",
        default,
        deserialize_with = "deserialize_optional_bool_compat"
    )]
    pub enable_remote_access: Option<bool>,
    #[serde(
        rename = "EnableLiveTvAccess",
        alias = "enableLiveTvAccess",
        default,
        deserialize_with = "deserialize_optional_bool_compat"
    )]
    pub enable_live_tv_access: Option<bool>,
    #[serde(
        rename = "EnableLiveTvManagement",
        alias = "enableLiveTvManagement",
        default,
        deserialize_with = "deserialize_optional_bool_compat"
    )]
    pub enable_live_tv_management: Option<bool>,
    #[serde(
        rename = "EnableSyncTranscoding",
        alias = "enableSyncTranscoding",
        default,
        deserialize_with = "deserialize_optional_bool_compat"
    )]
    pub enable_sync_transcoding: Option<bool>,
    #[serde(
        rename = "EnablePublicSharing",
        alias = "enablePublicSharing",
        default,
        deserialize_with = "deserialize_optional_bool_compat"
    )]
    pub enable_public_sharing: Option<bool>,
    #[serde(rename = "BlockedTags", alias = "blockedTags")]
    pub blocked_tags: Option<Vec<String>>,
    #[serde(rename = "EnabledFolders", alias = "enabledFolders")]
    pub enabled_folders: Option<Vec<String>>,
    #[serde(rename = "BlockedChannels", alias = "blockedChannels")]
    pub blocked_channels: Option<Vec<String>>,
    #[serde(
        rename = "RemoteClientBitrateLimit",
        alias = "remoteClientBitrateLimit",
        default,
        deserialize_with = "deserialize_optional_i32_compat"
    )]
    pub remote_client_bitrate_limit: Option<i32>,
    #[serde(
        rename = "AuthenticationProviderId",
        alias = "authenticationProviderId"
    )]
    pub authentication_provider_id: Option<String>,
    #[serde(
        rename = "InvalidLoginAttemptCount",
        alias = "invalidLoginAttemptCount",
        default,
        deserialize_with = "deserialize_optional_i32_compat"
    )]
    pub invalid_login_attempt_count: Option<i32>,
    #[serde(
        rename = "LoginAttemptsBeforeLockout",
        alias = "loginAttemptsBeforeLockout",
        default,
        deserialize_with = "deserialize_optional_i32_compat"
    )]
    pub login_attempts_before_lockout: Option<i32>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}
