export interface UserPolicy {
  IsAdministrator: boolean;
  IsDisabled: boolean;
  Role?: string;
}

export interface User {
  Id: string;
  Name: string;
  HasPassword: boolean;
  ServerId: string;
  Policy: UserPolicy;
}

export interface SessionInfo {
  Id: string;
  UserId: string;
  UserName: string;
  Client: string;
  DeviceName: string;
  DeviceId: string;
}

export interface AuthResult {
  User: User;
  SessionInfo: SessionInfo;
  AccessToken: string;
  ServerId: string;
}

export interface UserData {
  Played: boolean;
  PlaybackPositionTicks: number;
  IsFavorite?: boolean;
}

export interface UserItemData {
  PlaybackPositionTicks: number;
  PlayCount: number;
  IsFavorite: boolean;
  Played: boolean;
  LastPlayedDate?: string | null;
  ItemId: string;
}

export interface MediaStream {
  Index: number;
  Type: string;
  Language?: string | null;
  IsExternal: boolean;
  Path?: string | null;
  Codec?: string | null;
  DisplayTitle?: string | null;
  Channels?: number | null;
  BitRate?: number | null;
  IsDefault?: boolean | null;
}

export interface MediaSource {
  Id: string;
  Path?: string | null;
  Protocol: string;
  Container?: string | null;
  RunTimeTicks?: number | null;
  Bitrate?: number | null;
  SupportsDirectPlay: boolean;
  SupportsDirectStream: boolean;
  SupportsTranscoding: boolean;
  MediaStreams: MediaStream[];
}

export interface BaseItem {
  Id: string;
  Name: string;
  Type: string;
  Path: string;
  RunTimeTicks?: number | null;
  Bitrate?: number | null;
  ProductionYear?: number | null;
  CommunityRating?: number | null;
  OfficialRating?: string | null;
  ImagePrimaryUrl?: string | null;
  ImageTags?: Record<string, string> | null;
  BackdropImageTags?: string[] | null;
  Overview?: string | null;
  PremiereDate?: string | null;
  Genres?: string[] | null;
  ProviderIds?: Record<string, string> | null;
  Studios?: NameGuidPair[] | null;
  People?: BaseItemPerson[] | null;
  DateCreated?: string | null;
  ChildCount?: number | null;
  PlayAccess?: string | null;
  MediaSources?: MediaSource[] | null;
  UserData?: UserData | null;
  SeriesId?: string | null;
  SeriesName?: string | null;
  SeasonId?: string | null;
  SeasonName?: string | null;
  IndexNumber?: number | null;
  ParentIndexNumber?: number | null;
}

export interface NameGuidPair {
  Name: string;
  Id?: string | null;
}

export interface BaseItemPerson {
  Name: string;
  Id?: string | null;
  Role?: string | null;
  Type?: string | null;
  PrimaryImageTag?: string | null;
}

export interface Season {
  Id: string;
  Name: string;
  SeriesId: string;
  IndexNumber: number;
  ImagePrimaryUrl?: string | null;
}

export interface QueryResult<T> {
  Items: T[];
  TotalRecordCount: number;
  StartIndex: number;
}

export interface PlaybackInfo {
  MediaSources: MediaSource[];
  PlaySessionId: string;
}

export interface SubtitleTrack {
  Index: number;
  Codec: string;
  Language?: string | null;
  DisplayTitle: string;
  IsExternal: boolean;
  IsDefault: boolean;
}

export interface ItemCounts {
  MovieCount: number;
  SeriesCount: number;
  EpisodeCount: number;
  SongCount: number;
  AlbumCount: number;
  ArtistCount: number;
  ProgramCount: number;
  TrailerCount: number;
}

export interface TopPlayedItem {
  Id: string;
  Name: string;
  Type: string;
  RunTimeTicks?: number | null;
  Bitrate?: number | null;
  ProductionYear?: number | null;
  CommunityRating?: number | null;
  Overview?: string | null;
  PlayCount: number;
  UniqueUsers: number;
}

export interface TopPlayedSummary {
  StatDate: string;
  WindowDays: number;
  Items: TopPlayedItem[];
}
