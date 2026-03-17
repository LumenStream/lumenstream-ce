# lumenstream-web

Astro + Tailwind + React Islands + shadcn/ui 的 LumenStream Web 前端（Dark-only）。

## 技术约束

- 仅深色模式（Neutral + Rose，Primary: `#be123c`）
- 不内置视频播放，不做 DRM
- 点击播放仅做第三方播放器 deeplink + 复制链接回退
- 登录态为纯 Bearer Token（`sessionStorage`）

## 首页与演示模式

- `/` 是落地页，不会强制要求登录。
- 默认策略：`bun run dev` 启用 Mock，`bun run build`（正式部署）关闭 Mock。
- 可通过 `PUBLIC_LS_ENABLE_MOCK=true/false` 覆盖默认策略。
- 仅在 Mock 功能启用时显示 Mock 演示入口；可通过 `PUBLIC_LS_MOCK_MODE=true` 强制全局 mock。
- 仅在启用 Mock 功能时，后端不可达会自动回退到 Mock 数据。

## 页面设计要点

- 首页（`/app/home`）采用海报行（horizontal rows）+ Top10 轮播 Hero，接近流媒体信息页浏览体验。
- 个人页（`/app/profile`）采用仪表盘布局：欢迎区、账户信息、订阅信息、流量进度、Emby 兼容链接、邀请码。
- 个人页（`/app/profile`）新增侧边栏导航，统一提供“进入管理端（`/admin/overview`）”“返回首页（`/app/home`）”“返回落地页（`/`）”入口。
- 搜索与库浏览复用海报卡组件，支持电影/剧集/演职员混合检索与“演职员”单独筛选，并可在无后端场景下用 Mock 数据预览视觉效果。
- 剧集改为两级目录：`Series` 先进入目录页（`/app/library/:id`），先选季再点具体集进入详情页（`/app/item/:episodeId`）。
- 用户端不展示媒体库路径与 `.strm` 文件路径，仅展示业务元信息。

## Wave 1 导航与搜索快捷入口

- 顶部统一头像入口在用户端与管理端头部均复用 `src/islands/navigation/HeaderAccountEntry.tsx`。
- 用户端头部补充“管理后台（`/admin/overview`）”快捷链接；管理端头部补充“返回用户端（`/app/home`）”快捷链接。
- 头像展示规则集中在 `src/lib/navigation/header-account-entry.ts`：优先读取头像字段，缺失时回退为用户名首字母或“访客/访”。
- 搜索页“常用组件/入口”快捷建议集中在 `src/lib/search/frequent-shortcuts.ts`，由 `SearchCenter` 统一消费，包含直达链接与快速回填搜索词两类动作。

## 测试说明（导航与搜索）

- 侧边栏导航数据源测试位于 `src/lib/profile/sidebar-navigation.test.ts`。
- 顶部头像入口模型测试位于 `src/lib/navigation/header-account-entry.test.ts`。
- 搜索常用快捷建议测试位于 `src/lib/search/frequent-shortcuts.test.ts`。
- 核心断言聚焦稳定行为：导航项存在、目标链接正确、快捷建议来源集中，避免耦合易变布局结构。

## Recovery Wave 1R3 范围说明

- 本波次不更新 worktree 外的 `/Volumes/AppleSoft/media/TODO.md`；相关进展统一同步在仓库内文档（如本 `web/README.md`）中。

## 本地开发

```bash
cd lumenstream/web
bun install
bun run dev
```

默认会请求 `http://127.0.0.1:8096`，也可以设置：

```bash
PUBLIC_LS_API_BASE_URL=http://127.0.0.1:8096 bun run dev
```

开发态默认启用 Mock；若需强制关闭：

```bash
PUBLIC_LS_ENABLE_MOCK=false bun run dev
```

## 校验与构建

```bash
bun run check
bun run test
bun run build
```

## 部署（跨域）

构建后使用 Astro Node 入口运行前端服务：

```bash
node dist/server/entry.mjs
```

客户端静态资源位于 `dist/client`。

容器部署时可通过运行时环境变量覆盖 API 地址（无需重建镜像）：

```bash
LS_API_BASE_URL=https://api.example.com node dist/server/entry.mjs
```

## 目录（核心）

- `src/pages/`: Astro 页面路由
- `src/islands/`: React islands（用户端 + 管理端）
- `src/components/ui/`: shadcn 风格基础组件
- `src/components/domain/`: 业务域组件（DataState/MediaCard/PosterItemCard/Modal）
- `src/lib/api/`: API 客户端与 endpoint 封装
- `src/lib/mock/`: Mock 数据与演示模式逻辑
- `src/lib/auth/`: token/session 管理与权限判断
- `src/lib/player/`: 第三方播放器 deeplink
- `src/lib/types/`: 后端 DTO 对应 TS 类型
- `src/styles/globals.css`: Dark-only 主题 token
