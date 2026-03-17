# LumenStream Media Server (lumenstream)

Rust 实现的 Jellyfin 客户端兼容流媒体服务。

## 已落地能力

- Jellyfin 兼容 API（P0 + P1 子集）
  - 登录、用户、媒体浏览、详情、播放信息、播放状态上报
  - `PlaybackInfo` 返回媒体流轨细节（Audio/Subtitle 的 codec、声道、码率、默认轨等字段）
  - Root/Resume、字幕列表与字幕流、图片读取、Items 过滤参数兼容
  - 人物元数据与人物主图：`/Persons`、`/Persons/{id}`、`/Persons/{id}/Images/Primary`
  - `SearchTerm` 中文模糊检索（Meilisearch + 汉字/拼音全拼/首字母）
- 播放链路
  - Direct Play
  - `lumenstream` 鉴权 + 域名策略决策，`/Videos/{id}/stream` 默认返回 302 到 lumenbackend 数据面
  - 用户可在面板按账号选择播放域名（跨设备持久化）
  - `lumenbackend-rs` 存储协同（`gdrive://` / `lumenbackend://`）
  - lumenstream 与 lumenbackend 控制面双向通信（register/heartbeat/runtime-config/traffic-report）
- 认证与安全
  - Access Token 生命周期
  - `user_sessions` 会话追踪
  - 邀请码注册（支持全局强制邀请码开关）
  - 用户邀请码面板（查看/重置）与邀请关系追踪
  - 管理 API Key（创建/列表/删除）
  - RBAC（Admin/Viewer）
  - 基础风控（失败登录事件、限流、管理端 IP 白名单）
- 媒体扫描与元数据
  - 增量扫描（基于上次扫描时间窗口）
  - 支持按路径子树扫描（full/incremental/path）
  - STRM + NFO + `*-mediainfo.json` + 外挂字幕
  - `scan_library` 仅处理“新增 path”媒体（已存在 path 默认跳过，不重复回填历史旧条目）
  - `scan_library` 默认仅入库（STRM/NFO/字幕/图片/去重），不在扫描阶段阻塞执行 `ffprobe`
  - 如需扫描阶段生成 `mediainfo`，可在任务 payload 中设置 `probe_mediainfo=true`
  - 播放请求会异步触发按需 `ffprobe` 回填，不阻塞首播
  - 扫描完成后自动触发刮削补齐任务（`scraper_fill`，按媒体库/场景路由到 Provider 链；当前内置 TMDB / TVDB / Bangumi，未配置库默认保持 TMDB 行为）
  - 补全过程会通过统一刮削框架写回 DB + NFO（movie / tvshow / season / episode），并下载缺失图片到本地路径
  - 演职员信息（Top 20 cast + director/writer）入库到 `people` / `media_item_people`，用于 Jellyfin 客户端人物拉取
  - 人物主图统一写入 `tmdb.person_image_cache_dir`（默认 `./cache`），不再回退读取历史按媒体目录分散图片
  - 去重标记（同 stream_url）
- 管理端 API
  - 库管理、用户管理、播放会话、鉴权会话、审计日志
  - 任务中心：scan / metadata repair / subtitle sync / scraper fill / search reindex / retry
  - 存储配置管理与缓存清理/失效
  - 求片 Agent：用户求片/反馈中心、管理员审核、缺集/漏季自动发现、MoviePilot 搜索/下载/订阅回退
- 运维可观测
  - request-id、结构化请求日志
  - `/metrics` 指标快照（QPS、错误率、P95/P99、播放成功率、缓存命中率、Scraper 命中率、任务失败率）
- Web 前端（Astro）
  - Dark-only（Neutral + Rose，Primary `#be123c`）
  - 用户端 + 管理端界面（React Islands）
  - 媒体详情页采用横幅背景 + 竖版海报 + 演职员区块 + 技术信息区块布局
  - 演员/导演/编剧可点击跳转人物作品海报墙（默认电影+剧集）
  - 详情页支持播放、收藏、音频轨/字幕轨、时长与码率展示
  - 不内置播放器，点击播放仅引导第三方播放器（Infuse/VLC/PotPlayer）
  - 首页为落地页，支持 Mock 演示模式（无需后端）

## 版本切割

- 默认发行版为 **CE（Community Edition）**
- CE 保留：Jellyfin 兼容核心、媒体库/扫描/刮削/搜索、Agent、播放域名与 LumenBackend 推流能力
- 商业版扩展：账单/钱包/套餐订阅、高级用户流控（并发/配额/Top 用量/重置）、邀请返利、审计 CSV 导出
- 运行时可通过环境变量切换版本：

```bash
export LS_EDITION=ce   # 默认
# 或
export LS_EDITION=ee
```

- 若需细粒度覆盖，可使用：
  - `LS_EDITION_BILLING_ENABLED`
  - `LS_EDITION_ADVANCED_TRAFFIC_CONTROLS_ENABLED`
  - `LS_EDITION_INVITE_REWARDS_ENABLED`
  - `LS_EDITION_AUDIT_LOG_EXPORT_ENABLED`
  - `LS_EDITION_REQUEST_AGENT_ENABLED`
  - `LS_EDITION_PLAYBACK_ROUTING_ENABLED`

## 数据库迁移

- `0001_squashed_baseline.sql`：预上线阶段合并后的基线迁移（整合原 `0001`~`0017` 的表结构、索引与数据修复逻辑）
- `0002_people_metadata.sql`：新增人物元数据表（`people`、`media_item_people`）用于人物 API 与图片分发
- `0003_task_center_cron.sql`：任务中心调度配置持久化（cron/default payload/启停）
- `0004_user_profiles.sql`：用户资料字段扩展（email/display_name/avatar）
- `0005_single_admin_role.sql`：角色模型统一到单一 Admin 语义
- `0006_invite_system.sql`：邀请码体系（邀请码、邀请关系、首充返利记录）
- `0013_library_multi_paths.sql`：媒体库多路径模型（`library_paths`），并将历史 `libraries.root_path` 一次性迁移
- `0014_media_items_multiversion.sql`：媒体多版本字段（`media_items.version_group_id/version_rank`）与分组索引
- `0017_retire_cache_prewarm_task.sql`：下线 `cache_prewarm` 任务并清理非终态历史任务（通过增量迁移执行，避免修改已应用基线）
- `0018_scan_library_default_cron_3min.sql`：将 `scan_library` 默认 cron 从 30 分钟调整为 3 分钟（保留用户自定义 cron）
- `0020_agent_requests.sql`：求片 Agent 工单主表与事件时间线

> 注意：基线迁移文件一旦在任意环境执行，后续不可直接修改内容；所有变更必须通过新的递增 migration 文件落地。

## 契约测试与调用链回放

```bash
# 调用链回放（Web + TV 典型链路）
bash ../scripts/replay_client_callchains.sh

# API 契约测试（含失败样例）
bash ../scripts/contract_test_api.sh
```

## 快速启动

1. 准备 PostgreSQL
2. 复制配置（仅数据库连接）

```bash
cp config.example.yaml config.yaml
```

3. 启动 Meilisearch（搜索默认依赖，无需用户配置）

```bash
docker run --rm -p 7700:7700 getmeili/meilisearch:v1.15
```

> 项目内置固定地址：`http://127.0.0.1:7700`，固定索引：`lumenstream_media_items`。

4. 首次启动前，显式设置安全 bootstrap 管理员凭据

```bash
export LS_BOOTSTRAP_ADMIN_USER="replace-with-admin-user"
export LS_BOOTSTRAP_ADMIN_PASSWORD="replace-with-strong-password"
```

5. 启动服务

```bash
cargo run -p ls-app
```

默认地址：`http://127.0.0.1:8096`

首次启用后建议执行一次 `POST /admin/task-center/tasks/search_reindex/run`，把历史媒体写入 Meilisearch 索引。
常规流程下，库扫描与刮削补齐任务结束后会自动执行增量 Meilisearch 索引更新。

> 安全提示：当数据库中尚无管理员账号时，未提供 `LS_BOOTSTRAP_ADMIN_USER` / `LS_BOOTSTRAP_ADMIN_PASSWORD` 会导致启动失败；不再提供默认 `admin/admin123`。

## CI 与 GHCR 镜像

- CI workflow：`.github/workflows/ci-ghcr.yml`
- 每次 `push` / `pull_request` 会自动执行：
  - `cargo fmt --all -- --check`
  - `cargo test --workspace`
  - `cargo build --release -p ls-app`
  - 上传 `ls-app` Linux 二进制 artifact（便于下载调试）
- 当触发 `main` 分支 push、`v*` tag push、或手动 `workflow_dispatch` 时，会额外执行：
  - 基于 Debian 的 `Dockerfile` 多架构构建并推送后端镜像（`ghcr.io/<owner>/lumenstream`）
  - 基于 `Dockerfile.fullstack` 多架构构建并推送前后端一体镜像（`ghcr.io/<owner>/lumenstream-fullstack`）
  - 基于 `docker/Dockerfile.web` 多架构构建并推送前端镜像（`ghcr.io/<owner>/lumenstream-web`）

示例：

```bash
docker pull ghcr.io/<owner>/lumenstream:latest
docker run --rm -p 8096:8096 ghcr.io/<owner>/lumenstream:latest
```

```bash
docker pull ghcr.io/<owner>/lumenstream-fullstack:latest
docker run --rm -p 8096:8096 -p 4321:4321 \
  -e LS_API_BASE_URL=https://api.example.com \
  ghcr.io/<owner>/lumenstream-fullstack:latest
```

```bash
docker pull ghcr.io/<owner>/lumenstream-web:latest
docker run --rm -p 4321:4321 \
  -e LS_API_BASE_URL=https://api.example.com \
  ghcr.io/<owner>/lumenstream-web:latest
```

## 一键部署（前后端 + PostgreSQL + Meilisearch）

仓库内置 `docker-compose.fullstack.yml`，默认使用 GHCR 一体镜像，包含：

- `lumenstream`：后端 `ls-app` + Astro 前端（端口 `8096` + `4321`）
- `postgres`：主数据库
- `meilisearch`：搜索服务

启动前设置 bootstrap 管理员环境变量（首次启动必需）：

```bash
cp .env.fullstack.example .env
# 编辑 .env 中的 LS_BOOTSTRAP_ADMIN_USER / LS_BOOTSTRAP_ADMIN_PASSWORD
```

一键启动：

```bash
docker compose -f docker-compose.fullstack.yml up -d
```

> 该 compose 使用 `lumenstream` 与 `meilisearch` 共享网络命名空间，保证后端固定 `127.0.0.1:7700` 的搜索连接可用。

访问：

- Web 前端：`http://127.0.0.1:4321`
- 后端 API：`http://127.0.0.1:8096`

## 配置管理（Jellyfin 风格）

- 配置文件/环境变量只负责数据库启动参数：
  - `database.url` / `LS_DATABASE_URL`
  - `database.max_connections` / `LS_DATABASE_MAX_CONNECTIONS`
- 首次安全引导可通过环境变量提供管理员凭据：
  - `LS_BOOTSTRAP_ADMIN_USER`
  - `LS_BOOTSTRAP_ADMIN_PASSWORD`
- 其余配置项（server/auth/scan/storage/tmdb/scraper/security/observability/jobs/billing）通过 Web 管理端维护：
  - `GET /admin/settings`
  - `POST /admin/settings`
- Scraper / Agent 独立设置接口：
  - `GET /admin/scraper/settings`
  - `POST /admin/scraper/settings`
  - `GET /admin/scraper/providers`
  - `POST /admin/scraper/providers/{provider_id}/test`
  - `scraper` 支持全局 provider 列表、默认场景链，以及 TVDB / Bangumi 官方 API 凭据
  - 每个媒体库可通过 `libraries.scraper_policy` 覆盖场景链，例如动画库配置 `bangumi -> tvdb -> tmdb`
  - `GET /admin/agent/settings`
  - `POST /admin/agent/settings`
  - `POST /admin/agent/moviepilot/test`
- `POST /admin/settings` 返回 `restart_required: true`，配置将在重启后生效。
- `scan` 支持默认库多路径（示例）：

```yaml
scan:
  default_library_name: "Default Library"
  default_library_paths:
    - "/mnt/media/movies"
    - "/mnt/media/tvshows"
```

- 兼容升级：历史 `scan.default_library_path` 会在读取设置时自动一次性迁移为 `scan.default_library_paths`。
- `storage` 中可配置 lumenbackend 分布式节点（`POST /admin/settings` payload 片段）：

```yaml
storage:
  lumenbackend_enabled: true
  lumenbackend_route: "v1/streams/gdrive"
  lumenbackend_nodes:
    - "https://lumenbackend-us.example.com"
    - "https://lumenbackend-eu.example.com"
  lumenbackend_stream_signing_key: "replace-with-strong-random-key"
  lumenbackend_stream_token_ttl_seconds: 86400

security:
  rate_limit_per_minute: 240
  admin_allow_ips: []
  trust_x_forwarded_for: false
  trusted_proxies: []
  default_user_max_concurrent_streams: 2
  default_user_traffic_quota_bytes: 536870912000
  default_user_traffic_window_days: 30

billing:
  enabled: true
  min_recharge_amount: 1.00
  max_recharge_amount: 2000.00
  order_expire_minutes: 30
  channels: ["alipay", "wxpay"]
  epay:
    gateway_url: "https://epay.example.com"
    pid: "10001"
    key: "replace-with-epay-key"
    notify_url: "https://lumenstream.example.com/billing/epay/notify"
    return_url: "https://lumenstream.example.com/billing/epay/return"
    sitename: "LumenStream"

agent:
  enabled: true
  auto_mode: "automatic"
  missing_scan_enabled: true
  missing_scan_cron: "0 */30 * * * *"
  auto_close_on_library_hit: true
  review_required_on_parse_ambiguity: true
  feedback_auto_route: true
  moviepilot:
    enabled: true
    base_url: "https://moviepilot.example.com"
    username: "admin"
    password: "***"
    timeout_seconds: 20
    search_download_enabled: true
    subscribe_fallback_enabled: true
```

- Agent 关键环境变量覆盖（适合首启或容器部署）：
  - `LS_AGENT_ENABLED`
  - `LS_AGENT_MISSING_SCAN_ENABLED`
  - `LS_AGENT_MOVIEPILOT_ENABLED`
  - `LS_AGENT_MOVIEPILOT_BASE_URL`
  - `LS_AGENT_MOVIEPILOT_USERNAME`
  - `LS_AGENT_MOVIEPILOT_PASSWORD`

## Agent APIs

- 用户侧：
  - `GET /api/requests`
  - `POST /api/requests`
  - `GET /api/requests/{request_id}`
  - `POST /api/requests/{request_id}/resubmit`
- 管理侧：
  - `GET /admin/requests`
  - `GET /admin/requests/{request_id}`
  - `POST /admin/requests/{request_id}/review`
  - `POST /admin/requests/{request_id}/retry`
  - `GET /admin/agent/providers`

> `GET /admin/requests/{request_id}` 除请求与事件本体外，还会返回 `workflow_kind`、`workflow_steps`、`required_capabilities`、`manual_actions`，用于展示 Agent 工作流上下文与人工接管建议。

- 启动期可通过环境变量覆盖上述默认值：
  - `LS_LUMENBACKEND_STREAM_SIGNING_KEY`
  - `LS_LUMENBACKEND_STREAM_TOKEN_TTL_SECONDS`
  - `LS_DEFAULT_USER_MAX_CONCURRENT_STREAMS`
  - `LS_DEFAULT_USER_TRAFFIC_QUOTA_BYTES`
  - `LS_DEFAULT_USER_TRAFFIC_WINDOW_DAYS`
  - `LS_TRUST_X_FORWARDED_FOR`
  - `LS_TRUSTED_PROXIES`（逗号分隔，支持 IP 或 CIDR）
  - `LS_BILLING_ENABLED` / `LS_BILLING_MIN_RECHARGE_AMOUNT` / `LS_BILLING_MAX_RECHARGE_AMOUNT`
  - `LS_BILLING_ORDER_EXPIRE_MINUTES` / `LS_BILLING_CHANNELS`
  - `LS_BILLING_EPAY_GATEWAY_URL` / `LS_BILLING_EPAY_PID` / `LS_BILLING_EPAY_KEY`
  - `LS_BILLING_EPAY_NOTIFY_URL` / `LS_BILLING_EPAY_RETURN_URL` / `LS_BILLING_EPAY_SITENAME`
  - `LS_INVITE_FORCE_ON_REGISTER`
  - `LS_INVITEE_BONUS_ENABLED` / `LS_INVITEE_BONUS_AMOUNT`
  - `LS_INVITER_REBATE_ENABLED` / `LS_INVITER_REBATE_RATE`

> 若部署在反向代理后，请仅在 `trust_x_forwarded_for=true` 且 `trusted_proxies` 已配置为受信代理 IP/CIDR 时使用 `X-Forwarded-For`，否则系统将回退到直连源地址。

## Billing APIs

- 用户侧：
  - `GET /billing/wallet`
  - `GET /billing/plans`
  - `POST /billing/recharge/orders`
  - `GET /billing/recharge/orders/{order_id}`
  - `GET /billing/recharge/orders/{order_id}/ws?token=...`（充值订单状态实时推送，WebSocket）
  - `POST /billing/plans/{plan_id}/purchase`
- 支付回调：
  - `POST /billing/epay/notify`
  - `GET /billing/epay/return`
- 管理侧：
  - `GET|POST /admin/billing/plans`
  - `GET /admin/billing/recharge-orders`
  - `GET /admin/billing/users/{user_id}/wallet`
  - `GET /admin/billing/users/{user_id}/ledger`
  - `GET /admin/billing/users/{user_id}/subscriptions`
  - `POST /admin/billing/users/{user_id}/adjust-balance`

## Notifications APIs

- `GET /api/notifications`
- `POST /api/notifications`
- `PATCH /api/notifications/read-all`
- `PATCH /api/notifications/{notification_id}/read`
- `GET /api/notifications/ws?token=...`（通知实时推送，WebSocket）

## Invite APIs

- 注册与用户侧：
  - `POST /api/auth/register`（`username/password/invite_code`，注册后自动登录）
  - `GET /api/invite/me`
  - `POST /api/invite/me/reset`
- 管理侧：
  - `GET /admin/invite/settings`
  - `POST /admin/invite/settings`
  - `GET /admin/invite/relations`
  - `GET /admin/invite/rebates`

## LumenBackend 控制面接口（内部）

- 管理侧（节点运行配置）：
  - `GET /admin/lumenbackend/nodes`
  - `POST /admin/lumenbackend/nodes`（手动创建节点）
  - `PATCH /admin/lumenbackend/nodes/{node_id}`
  - `DELETE /admin/lumenbackend/nodes/{node_id}`
  - `GET /admin/lumenbackend/nodes/{node_id}/schema`（读取节点上报的动态表单 schema）
  - `GET /admin/lumenbackend/nodes/{node_id}/config`
  - `POST /admin/lumenbackend/nodes/{node_id}/config`（按 schema 校验后保存）
- `POST /internal/lumenbackend/register`
- `POST /internal/lumenbackend/heartbeat`
- `GET /internal/lumenbackend/runtime-config`
- `POST /internal/lumenbackend/traffic/report`

> 以上接口要求 `X-Api-Key`（使用 LumenStream 管理 API Key）。
> lumenbackend 本地 `config.toml` 仅保留 `lumenstream.base_url/api_key/node_id`，其余运行参数由 LumenStream 节点配置下发。
> `register` 不再自动建档节点：必须先在 `/admin/playback` 手动创建 node，随后由 LumenBackend 上报 runtime schema，管理端按 schema 动态渲染配置表单。

## 目录

- `crates/ls-app`：程序入口
- `crates/ls-agent`：可复用的 Agent 核心模型、workflow/provider 抽象、MoviePilot 客户端与筛选策略
- `crates/ls-api`：HTTP 路由与 middleware
- `crates/ls-scraper`：可扩展的刮削核心模型、Provider 抽象、场景路由与通用 NFO IO（当前包含 TMDB / TVDB / Bangumi）
- `crates/ls-config`：配置加载
- `crates/ls-domain`：DTO 与通用领域模型
- `crates/ls-infra`：DB/鉴权/扫描/任务/存储逻辑
- `migrations/`：PostgreSQL migration
- `web/`：Astro + Tailwind + React Islands 前端工程

## 关联文档（仓库根目录）

- `docs/editions.md`
- `docs/upstream-downstream.md`
- `docs/repository-split.md`
- `CONTRIBUTING.md`
- `CLA.md`

## CE / Commercial 双仓切割

如需在本地直接切出 CE upstream 与 commercial downstream：

```bash
bash scripts/cut_split_repositories.sh --force /Volumes/AppleSoft/media
```

执行后会生成：

- `/Volumes/AppleSoft/media/lumenstream-ce`
- `/Volumes/AppleSoft/media/lumenstream-commercial`

## Web Frontend（Astro）

前端工程位于 `web/`，采用 Astro + Tailwind + React Islands + shadcn/ui。

```bash
cd web
bun install
PUBLIC_LS_API_BASE_URL=http://127.0.0.1:8096 bun run dev
```

> 默认策略：`bun run dev` 启用 Mock，`bun run build`（正式部署）禁用 Mock。可用 `PUBLIC_LS_ENABLE_MOCK=true/false` 覆盖。

构建并运行：

```bash
cd web
bun run build
node dist/server/entry.mjs
```

> 跨域部署时，请在后端 `server.cors_allow_origins` 中加入前端域名。
> 容器运行时可通过 `LS_API_BASE_URL` 动态覆盖 API 地址（无需重建镜像）。
