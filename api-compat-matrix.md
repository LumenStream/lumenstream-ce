# API Compatibility Matrix (LumenStream vs Emby)

- 生成时间: 2026-02-25
- 对比工具: `python3 scripts/compare_endpoint.py` + 结构化补充脚本
- 完整机器可读差异: `docs/emby-compat-diff-2026-02-25.json`
- 说明: `EXTRA in LumenStream` 多数为无害扩展；本表重点关注 `MISSING in LumenStream` 与 `TYPE MISMATCH`。

## 1. 汇总

| Endpoint | LumenStream/Emby Status | Missing in LumenStream | Type Mismatch | 优先级 |
|---|---:|---:|---:|---|
| `/System/Info/Public` | 200/200 | 0 | 0 | - |
| `/System/Info` | 200/200 | 0 | 3 | P2 |
| `/Sessions` | 200/200 | 12 | 1 | P0 |
| `/Users/{id}/Views` | 200/200 | 13 | 0 | P1 |
| `/Users/{id}/Items?Limit=1&Recursive=true` | 200/200 | 1 | 0 | P2 |
| `/Users/{id}/Items/Latest?Limit=1` | 200/200 | 14 | 1 | P1 |
| `/Users/{id}/Items/Resume?Limit=1` | 200/200 | 0 | 1 | P1 |
| `/Users/{id}/Items/{itemId}/HideFromResume?Hide=true` (POST) | 200/200 | 0 | 0 | P1 |
| `/Users/{id}/Items/{itemId}` | 200/200 | 85 | 11 | P0 |
| `/Items/{itemId}/PlaybackInfo?UserId={id}` | 200/200 | 50 | 4 | P0 |
| `/Users/Public` | 200/200 | 0 | 1 | P0 |

## 2. 逐项问题与修复判定

### /System/Info/Public

- 状态: LumenStream `200` / Emby `200`
- 缺失字段数: `0`
- 类型差异数: `0`
- 问题描述: 待补充。
- 缺失字段明细:
  - 无
- 类型差异明细:
  - 无

### /System/Info

- 状态: LumenStream `200` / Emby `200`
- 缺失字段数: `0`
- 类型差异数: `3`
- 问题描述: 能力标志位与 Emby 实际值不一致（布尔值差异）。
- 根因定位: 系统信息字段在路由中硬编码。
- 可直接复用现有能力修复:
  - 将 CanSelfRestart / HasUpdateAvailable / HardwareAccelerationRequiresPremiere 改为基于配置与运行态计算。
  - 保留已存在字段，不需要改动数据库结构。
- 需要新增功能/数据链路: 无（可通过现有能力完成）
- 缺失字段明细:
  - 无
- 类型差异明细:
  - `CanSelfRestart` (LumenStream=`bool=false` / Emby=`bool=true`)
  - `HardwareAccelerationRequiresPremiere` (LumenStream=`bool=false` / Emby=`bool=true`)
  - `HasUpdateAvailable` (LumenStream=`bool=false` / Emby=`bool=true`)

### /Sessions

- 状态: LumenStream `200` / Emby `200`
- 缺失字段数: `12`
- 类型差异数: `1`
- 问题描述: 会话结构缺失 PlayState/AdditionalUsers/InternalDeviceId，导致第三方客户端无法正确识别播放态。
- 根因定位: 会话查询接口固定返回 play_state=None/additional_users=None/internal_device_id=None。
- 本地修复进展（2026-02-25）:
  - 会话返回已补齐 `PlayState/AdditionalUsers/InternalDeviceId` 基础结构，避免客户端解析空值失败。
  - 会话返回已接入活跃 `playback_sessions` 数据，`PlayState.PlayMethod/PositionTicks` 可回填真实播放态。
  - `/Sessions` 已增加“最近活跃会话”过滤（30 分钟窗口），用于减少历史会话导致的数量级偏差。
  - `/Sessions` 已按 `user_id + device_id` 去重，优先保留最近活跃会话，降低同设备多 token 的重复记录噪声。
  - 尚未部署前，远端对比结果仍会显示旧差异。
- 可直接复用现有能力修复:
  - InternalDeviceId 可复用现有派生算法（认证流程已实现）。
  - AdditionalUsers 可先返回空数组。
  - PlayState 的基础字段可复用登录返回中的默认结构。
- 需要新增功能/数据链路:
  - 若要返回真实 PlayState（暂停/静音/播放方式/可 seek），需要把 /Sessions/Playing* 上报数据持久化到会话状态（新增会话播放态功能）。
- 缺失字段明细:
  - `[].AdditionalUsers`
  - `[].InternalDeviceId`
  - `[].PlayState`
  - `[].PlayState.CanSeek`
  - `[].PlayState.IsMuted`
  - `[].PlayState.IsPaused`
  - `[].PlayState.PlayMethod`
  - `[].PlayState.PlaybackRate`
  - `[].PlayState.RepeatMode`
  - `[].PlayState.Shuffle`
  - `[].PlayState.SleepTimerMode`
  - `[].PlayState.SubtitleOffset`
- 类型差异明细:
  - `(root)` (LumenStream=`array[189]` / Emby=`array[5]`)

### /Users/{id}/Views

- 状态: LumenStream `200` / Emby `200`
- 缺失字段数: `13`
- 类型差异数: `0`
- 问题描述: 视图项缺失 Emby 客户端常用标识字段（Guid/Etag/DisplayPreferencesId 等）。
- 根因定位: Root/View 列表复用 BaseItemDto 子集，未补齐视图端字段。
- 本地修复进展（2026-02-25）:
  - 已在通用兼容整形阶段补齐 `DateModified/DisplayPreferencesId/PresentationUniqueKey/Guid/Etag/ForcedSortName`。
  - 已补齐 `ExternalUrls/RemoteTrailers/Taglines/LockedFields/LockData` 缺省结构。
  - 已补齐 `ImageTags.Primary` 与 folder `PrimaryImageAspectRatio` 默认值。
  - 尚未部署前，远端对比结果仍会显示旧差异。
- 可直接复用现有能力修复:
  - DisplayPreferencesId/PresentationUniqueKey/Guid 可由现有 item id 或兼容 id 稳定派生。
  - ForcedSortName 可复用 SortName。
  - ExternalUrls/Taglines/RemoteTrailers/LockedFields 可先返回空数组，LockData 返回 false。
- 需要新增功能/数据链路:
  - 若要求 Etag/DateModified 与媒体变更严格一致，需要引入稳定 Etag 生成策略与更新时间来源（可能需要补充索引/更新时间字段使用策略）。
- 缺失字段明细:
  - `DateModified`
  - `DisplayPreferencesId`
  - `Etag`
  - `ExternalUrls`
  - `ForcedSortName`
  - `Guid`
  - `ImageTags.Primary`
  - `LockData`
  - `LockedFields`
  - `PresentationUniqueKey`
  - `PrimaryImageAspectRatio`
  - `RemoteTrailers`
  - `Taglines`
- 类型差异明细:
  - 无

### /Users/{id}/Items?Limit=1&Recursive=true

- 状态: LumenStream `200` / Emby `200`
- 缺失字段数: `1`
- 类型差异数: `0`
- 问题描述: 首项样本缺 BackdropImageTags（数据依赖差异）。
- 根因定位: 两端样本数据不同，且 LumenStream 项目中封面/背景图覆盖率不一致。
- 可直接复用现有能力修复:
  - 优先复用现有 metadata/image_tags 生成 BackdropImageTags。
- 需要新增功能/数据链路:
  - 若 metadata 没有背景图，需要在扫描/刮削阶段补充背景图获取逻辑。
- 缺失字段明细:
  - `BackdropImageTags`
- 类型差异明细:
  - 无

### /Users/{id}/Items/Latest?Limit=1

- 状态: LumenStream `200` / Emby `200`
- 缺失字段数: `14`
- 类型差异数: `1`
- 问题描述: Latest 列表缺 RunTimeTicks、状态字段、UserData 子结构。
- 根因定位: Latest 接口直接返回 items.items，未做更完整 Emby 视图整形。
- 本地修复进展（2026-02-25）:
  - Latest 已改为复用兼容整形链路，不再直接返回原始 items。
  - 已补齐 `ServerId/UserData/RunTimeTicks/Status/AirDays` 等核心结构字段默认值。
  - 尚未部署前，远端对比结果仍会显示旧差异。
- 可直接复用现有能力修复:
  - 可复用 compat_items_query_result_json 与现有 UserData 映射逻辑。
  - 可复用现有 metadata 字段填充 Status/EndDate/Logo 等。
- 需要新增功能/数据链路:
  - 若要保证 UnplayedItemCount 等动态字段准确，需要新增聚合计算或缓存策略。
- 缺失字段明细:
  - `[].AirDays`
  - `[].BackdropImageTags`
  - `[].BackdropImageTags[]`
  - `[].EndDate`
  - `[].ImageTags.Logo`
  - `[].RunTimeTicks`
  - `[].ServerId`
  - `[].Status`
  - `[].UserData`
  - `[].UserData.IsFavorite`
  - `[].UserData.PlayCount`
  - `[].UserData.PlaybackPositionTicks`
  - `[].UserData.Played`
  - `[].UserData.UnplayedItemCount`
- 类型差异明细:
  - `[].IsFolder` (LumenStream=`bool=false` / Emby=`bool=true`)

### /Users/{id}/Items/Resume?Limit=1

- 状态: LumenStream `200` / Emby `200`
- 缺失字段数: `0`
- 类型差异数: `1`
- 问题描述: Resume 的 Items 内容类型差异明显（当前样本中 LumenStream 有数据、Emby 为空）。
- 根因定位: 主要受媒体库内容与播放记录差异影响；但 LumenStream 额外字段较多。
- 可直接复用现有能力修复:
  - 可复用现有 filter/fields 裁剪逻辑，让返回更接近 Emby。
- 需要新增功能/数据链路:
  - 若要严格对齐 Emby 的“可恢复项目判定”，需要新增更细粒度播放进度判定规则。
- 缺失字段明细:
  - 无
- 类型差异明细:
  - `Items` (LumenStream=`array[1]` / Emby=`array[0]`)

### /Users/{id}/Items/{itemId}/HideFromResume?Hide=true (POST)

- 状态: LumenStream `404 -> 200` / Emby `200`
- 缺失字段数: `0`
- 类型差异数: `0`
- 问题描述: Emby 客户端调用该接口移除“继续观看”时，LumenStream 未注册路由导致 `404`。
- 根因定位: `router.rs` 未挂载 `/Users/{user_id}/Items/{item_id}/HideFromResume` 的 POST 路由。
- 本地修复进展（2026-03-07）:
  - 已新增 `post_user_item_hide_from_resume` 处理器并在 router 挂载路由。
  - `Hide=true` 时执行真实写入：将 `playback_position_ticks` 归零（保留 `played/play_count` 不变）。
  - `Hide=false` 时返回当前 `UserItemData`（不改写状态）。
- 可直接复用现有能力修复:
  - 复用 `update_user_item_data` 与 `get_user_item_data`，无需新增存储字段。
- 需要新增功能/数据链路:
  - 若未来要 1:1 对齐 Emby 的“隐藏但保留进度”语义，需要新增独立 `hide_from_resume` 持久化字段。
- 缺失字段明细:
  - 无
- 类型差异明细:
  - 无

### /Users/{id}/PlayedItems/{itemId}/Delete (POST)

- 状态: LumenStream `404 -> 200` / Emby `200`
- 缺失字段数: `0`
- 类型差异数: `0`
- 问题描述: 部分 Emby 客户端会调用 legacy `/PlayedItems/{itemId}/Delete` 取消已播放，LumenStream 之前仅支持 `DELETE /PlayedItems/{itemId}`，因此返回 `404`。
- 根因定位: `router.rs` 未挂载 `/Users/{user_id}/PlayedItems/{item_id}/{action}` 的 legacy POST 路由。
- 本地修复进展（2026-03-09）:
  - 已新增 `post_item_played_legacy_action` 并支持 `Add/Delete`。
  - `Delete` 复用 `mark_unplayed`，真实清除 `played/play_count/last_played_at/playback_position_ticks`。
  - `Add` 复用 `mark_played`，避免 legacy 客户端后续出现同类兼容缺口。
- 可直接复用现有能力修复:
  - 复用现有 `mark_played` / `mark_unplayed` 数据链路，无需新增存储字段。
- 需要新增功能/数据链路:
  - 无
- 缺失字段明细:
  - 无
- 类型差异明细:
  - 无

### /Users/{id}/Items/{itemId}

- 状态: LumenStream `200` / Emby `200`
- 缺失字段数: `85`
- 类型差异数: `11`
- 问题描述: 详情页缺失大量 Emby 关键字段（85 项）且多处类型不一致（11 项）。
- 根因定位: BaseItemDto 与 MediaSource/MediaStreams 字段覆盖不完整；部分字段未从 metadata/mediainfo 映射。
- 本地修复进展（2026-02-25）:
  - `MediaSources[].RunTimeTicks` 已在兼容整形阶段补齐默认值（优先 item RunTimeTicks，缺失回落 `0`）。
  - 详情页 MediaStreams 兼容层已补齐多项缺省字段（如 `DisplayLanguage/AspectRatio/ExtendedVideo*/RefFrames/TimeBase` 等）。
  - 详情页已接入 `mediainfo` 章节解析，`Chapters` 与 `MediaSources[].Chapters` 可返回真实 `StartPositionTicks/Name/MarkerType/ChapterIndex`。
  - 详情页 `ExternalUrls` 已支持基于 `ProviderIds(Imdb/Tmdb/Tvdb)` 推导 `Name/Url`，不再仅返回空数组占位。
  - 尚未部署前，远端对比结果仍会显示旧差异。
- 可直接复用现有能力修复:
  - Overview/ProductionYear/Genres/Studios/People 等可优先复用现有 metadata 入库数据。
  - SortName/PremiereDate/ProviderIds 等可通过现有字段映射补齐。
- 需要新增功能/数据链路:
  - Chapters、深层 MediaStreams 参数、远端流能力判断等需要新增更完整 mediainfo 采集与持久化能力。
  - 若要完全匹配 Emby 的播放能力字段，需要扩展转码/直播放能力判定模块。
- 缺失字段明细:
  - `BackdropImageTags[]`
  - `Chapters[].ChapterIndex`
  - `Chapters[].MarkerType`
  - `Chapters[].Name`
  - `Chapters[].StartPositionTicks`
  - `CommunityRating`
  - `ExternalUrls[].Name`（本地代码已修复，待部署复测）
  - `ExternalUrls[].Url`（本地代码已修复，待部署复测）
  - `GenreItems`
  - `GenreItems[].Id`
  - `GenreItems[].Name`
  - `Genres`
  - `Genres[]`
  - `Height`
  - `ImageTags.Logo`
  - `MediaSources[].Chapters[].ChapterIndex`
  - `MediaSources[].Chapters[].MarkerType`
  - `MediaSources[].Chapters[].Name`
  - `MediaSources[].Chapters[].StartPositionTicks`
  - `MediaSources[].DefaultAudioStreamIndex`
  - `MediaSources[].MediaStreams[].AspectRatio`
  - `MediaSources[].MediaStreams[].AverageFrameRate`
  - `MediaSources[].MediaStreams[].BitDepth`
  - `MediaSources[].MediaStreams[].BitRate`
  - `MediaSources[].MediaStreams[].Codec`
  - `MediaSources[].MediaStreams[].DisplayLanguage`
  - `MediaSources[].MediaStreams[].DisplayTitle`
  - `MediaSources[].MediaStreams[].ExtendedVideoSubType`
  - `MediaSources[].MediaStreams[].ExtendedVideoSubTypeDescription`
  - `MediaSources[].MediaStreams[].ExtendedVideoType`
  - `MediaSources[].MediaStreams[].Height`
  - `MediaSources[].MediaStreams[].IsDefault`
  - `MediaSources[].MediaStreams[].IsForced`
  - `MediaSources[].MediaStreams[].Language`
  - `MediaSources[].MediaStreams[].Level`
  - `MediaSources[].MediaStreams[].PixelFormat`
  - `MediaSources[].MediaStreams[].Profile`
  - `MediaSources[].MediaStreams[].RealFrameRate`
  - `MediaSources[].MediaStreams[].RefFrames`
  - `MediaSources[].MediaStreams[].TimeBase`
  - `MediaSources[].MediaStreams[].VideoRange`
  - `MediaSources[].MediaStreams[].Width`
  - `MediaStreams[].AspectRatio`
  - `MediaStreams[].AverageFrameRate`
  - `MediaStreams[].BitDepth`
  - `MediaStreams[].BitRate`
  - `MediaStreams[].Codec`
  - `MediaStreams[].DisplayLanguage`
  - `MediaStreams[].DisplayTitle`
  - `MediaStreams[].ExtendedVideoSubType`
  - `MediaStreams[].ExtendedVideoSubTypeDescription`
  - `MediaStreams[].ExtendedVideoType`
  - `MediaStreams[].Height`
  - `MediaStreams[].IsDefault`
  - `MediaStreams[].IsForced`
  - `MediaStreams[].Language`
  - `MediaStreams[].Level`
  - `MediaStreams[].PixelFormat`
  - `MediaStreams[].Profile`
  - `MediaStreams[].RealFrameRate`
  - `MediaStreams[].RefFrames`
  - `MediaStreams[].TimeBase`
  - `MediaStreams[].VideoRange`
  - `MediaStreams[].Width`
  - `Overview`
  - `People`
  - `People[].Id`
  - `People[].Name`
  - `People[].PrimaryImageTag`
  - `People[].Role`
  - `People[].Type`
  - `PremiereDate`
  - `ProductionLocations[]`
  - `ProductionYear`
  - `ProviderIds`
  - `ProviderIds.Imdb`
  - `ProviderIds.Tmdb`
  - `ProviderIds.Tvdb`
  - `RemoteTrailers[].Url`
  - `RunTimeTicks`
  - `SortName`
  - `Studios`
  - `Studios[].Id`
  - `Studios[].Name`
  - `Width`
- 类型差异明细:
  - `BackdropImageTags` (LumenStream=`array[0]` / Emby=`array[1]`)
  - `Chapters` (LumenStream=`array[0]` / Emby=`array[23]`)
  - `ExternalUrls` (LumenStream=`array[0]` / Emby=`array[4]`)
  - `MediaSources[].Chapters` (LumenStream=`array[0]` / Emby=`array[23]`)
  - `MediaSources[].IsRemote` (LumenStream=`bool=true` / Emby=`bool=false`)
  - `MediaSources[].MediaStreams` (LumenStream=`array[1]` / Emby=`array[3]`)
  - `MediaSources[].RunTimeTicks` (LumenStream=`NoneType=null` / Emby=`int`)（本地代码已修复，待部署复测）
  - `MediaSources[].SupportsTranscoding` (LumenStream=`bool=false` / Emby=`bool=true`)
  - `MediaStreams` (LumenStream=`array[1]` / Emby=`array[3]`)
  - `ProductionLocations` (LumenStream=`array[0]` / Emby=`array[1]`)
  - `RemoteTrailers` (LumenStream=`array[0]` / Emby=`array[1]`)

### /Items/{itemId}/PlaybackInfo?UserId={id}

- 状态: LumenStream `200` / Emby `200`
- 缺失字段数: `50`
- 类型差异数: `4`
- 问题描述: PlaybackInfo 缺失 50 项 MediaSource 子字段并有 4 处类型差异。
- 根因定位: PlaybackInfoResponseDto/MediaSourceInfoDto 字段集合本身偏小，构造逻辑未包含章节/格式/默认轨等。
- 本地修复进展（2026-02-25）:
  - `MediaSources[].RunTimeTicks` 与 `MediaSources[].Bitrate` 已做数值归一化（缺失回落 `0`）。
  - 媒体项映射阶段同样补齐了 MediaSource 数值字段缺省值，减少 `null` 类型差异外溢到列表/详情接口。
  - API 层已补齐 PlaybackInfo 的 MediaSource/MediaStream 兼容缺省字段（Type/ItemId/Chapters/Formats/RequiredHttpHeaders/SupportsProbing、以及多项 MediaStream 缺省属性）。
  - `MediaSources[].IsRemote` 已改为基于 `Protocol/Path` 推导（HTTP 源为 `true`，本地文件为 `false`），避免远端流误判。
  - PlaybackInfo 已接入 `mediainfo` 章节解析，MediaSource 章节从“空数组占位”升级为优先返回真实章节数据。
  - `MediaSources[].MediaStreams` 在缺失探测信息时不再强制注入空 `Video` 占位轨，避免客户端将占位轨误判为真实流信息（2026-02-27）。
  - PlaybackInfo 请求阶段新增按需 `ffprobe` 回填：当 `mediainfo` 缺失且目标为 `http/https` 远程源时，自动探测并回写 `metadata.mediainfo`（含 `RunTimeTicks/Bitrate/MediaStreams/Chapters`），后续请求复用缓存结果（2026-02-27）。
  - 尚未部署前，远端对比结果仍会显示旧差异。
- 可直接复用现有能力修复:
  - ErrorCode 可直接加到响应 DTO（无须新数据源）。
  - Bitrate/RunTimeTicks/媒体流基础字段可复用现有 mediainfo 解析结果。
- 需要新增功能/数据链路:
  - Chapters、DefaultAudioStreamIndex、Formats、HasMixedProtocols 等需要新增解析+存储+映射链路。
- 缺失字段明细:
  - `MediaSources[].AddApiKeyToDirectStreamUrl`
  - `MediaSources[].Chapters`
  - `MediaSources[].Chapters[].ChapterIndex`
  - `MediaSources[].Chapters[].MarkerType`
  - `MediaSources[].Chapters[].Name`
  - `MediaSources[].Chapters[].StartPositionTicks`
  - `MediaSources[].DefaultAudioStreamIndex`
  - `MediaSources[].Formats`
  - `MediaSources[].HasMixedProtocols`
  - `MediaSources[].IsInfiniteStream`
  - `MediaSources[].IsRemote`
  - `MediaSources[].ItemId`
  - `MediaSources[].MediaStreams[].AspectRatio`
  - `MediaSources[].MediaStreams[].AttachmentSize`
  - `MediaSources[].MediaStreams[].AverageFrameRate`
  - `MediaSources[].MediaStreams[].BitDepth`
  - `MediaSources[].MediaStreams[].BitRate`
  - `MediaSources[].MediaStreams[].Codec`
  - `MediaSources[].MediaStreams[].DisplayLanguage`
  - `MediaSources[].MediaStreams[].DisplayTitle`
  - `MediaSources[].MediaStreams[].ExtendedVideoSubType`
  - `MediaSources[].MediaStreams[].ExtendedVideoSubTypeDescription`
  - `MediaSources[].MediaStreams[].ExtendedVideoType`
  - `MediaSources[].MediaStreams[].Height`
  - `MediaSources[].MediaStreams[].IsAnamorphic`
  - `MediaSources[].MediaStreams[].IsDefault`
  - `MediaSources[].MediaStreams[].IsForced`
  - `MediaSources[].MediaStreams[].IsHearingImpaired`
  - `MediaSources[].MediaStreams[].IsInterlaced`
  - `MediaSources[].MediaStreams[].IsTextSubtitleStream`
  - `MediaSources[].MediaStreams[].Language`
  - `MediaSources[].MediaStreams[].Level`
  - `MediaSources[].MediaStreams[].PixelFormat`
  - `MediaSources[].MediaStreams[].Profile`
  - `MediaSources[].MediaStreams[].Protocol`
  - `MediaSources[].MediaStreams[].RealFrameRate`
  - `MediaSources[].MediaStreams[].RefFrames`
  - `MediaSources[].MediaStreams[].SupportsExternalStream`
  - `MediaSources[].MediaStreams[].TimeBase`
  - `MediaSources[].MediaStreams[].VideoRange`
  - `MediaSources[].MediaStreams[].Width`
  - `MediaSources[].Name`
  - `MediaSources[].ReadAtNativeFramerate`
  - `MediaSources[].RequiredHttpHeaders`
  - `MediaSources[].RequiresClosing`
  - `MediaSources[].RequiresLooping`
  - `MediaSources[].RequiresOpening`
  - `MediaSources[].Size`
  - `MediaSources[].SupportsProbing`
  - `MediaSources[].Type`
- 类型差异明细:
  - `MediaSources[].Bitrate` (LumenStream=`NoneType=null` / Emby=`int`)（本地代码已修复，待部署复测）
  - `MediaSources[].MediaStreams` (LumenStream=`array[1]` / Emby=`array[3]`)
  - `MediaSources[].RunTimeTicks` (LumenStream=`NoneType=null` / Emby=`int`)（本地代码已修复，待部署复测）
  - `MediaSources[].SupportsTranscoding` (LumenStream=`bool=false` / Emby=`bool=true`)

### /Users/Public

- 状态: LumenStream `200` / Emby `200`
- 缺失字段数: `0`
- 类型差异数: `1`
- 问题描述: 行为差异：Emby 返回空数组，LumenStream 返回完整用户列表（且字段非常多）。
- 根因定位: LumenStream 直接暴露 list_public_users 结果。
- 本地修复进展（2026-02-25）:
  - `/Users/Public` 已改为返回空数组，和 Emby 行为对齐。
  - 尚未部署前，远端对比结果仍会显示旧差异。
- 可直接复用现有能力修复:
  - 可通过配置开关快速切换为最小公开形态（例如默认 []）。
- 需要新增功能/数据链路:
  - 若要兼容多客户端期望，需要新增“公开用户可见性策略”与权限模型（按客户端/场景控制）。
- 缺失字段明细:
  - 无
- 类型差异明细:
  - `(root)` (LumenStream=`array[2]` / Emby=`array[0]`)

## 附录：ParentId 非递归语义专项（2026-02-26）

- 问题背景: `ParentId=剧集库` 的 `/Users/{id}/Items`、`/Items` 在 `Recursive=false` 时会混入 `Season/Episode`。
- 根因定位:
  - `crates/ls-infra/src/infra/app_media_root_search.rs` 中 `recursive=true/false` 使用相同过滤条件。
  - `crates/ls-infra/src/infra/app_media_filters_items.rs` 的 `ParentId` 过滤同样未区分递归语义。
- 本地修复进展（2026-02-26）:
  - 新增 `ParentQueryScope` 统一父级语义：`CollectionFolder`、`Series`、`Season`、Fallback。
  - `/Users/{id}/Items`、`/Items` 已按父级类型 + `Recursive` 执行分层过滤。
  - `/Genres`、`/Studios`、`/Years`、`/OfficialRatings`、`/Tags` 已透传 `Recursive` 并复用同一父级范围规则。
  - `/Items/Filters` 维持非递归语义（固定 `Recursive=false`）避免层级穿透。

## 3. 代码定位（根因相关）

- 路径兼容层与 header 注入: `crates/ls-api/src/api/middleware.rs`
- 会话返回字段当前置空: `crates/ls-api/src/api/routes_sessions_report.rs`
- 登录返回默认 PlayState/InternalDeviceId: `crates/ls-infra/src/infra/app_auth_tokens.rs`
- 详情/列表兼容整形: `crates/ls-api/src/api/routes_items_browse.rs`
- PlaybackInfo 构造: `crates/ls-infra/src/infra/app_media_playback_stream.rs`
- Playback DTO 字段定义: `crates/ls-domain/src/jellyfin.rs`
- 用户公开列表接口: `crates/ls-api/src/api/routes_users_jellyfin.rs`
- 系统信息字段硬编码: `crates/ls-api/src/api/routes_system_self.rs`

## 4. 建议修复顺序

1. `P0`: `/Sessions`、`/Items/{itemId}/PlaybackInfo`、`/Users/{id}/Items/{itemId}`、`/Users/Public`
2. `P1`: `/Users/{id}/Views`、`/Users/{id}/Items/Latest`、`/Users/{id}/Items/Resume`
3. `P2`: `/System/Info` 能力布尔值与少量样本字段差异
