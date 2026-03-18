<div align="center">
  <img src="./docs/assets/banner.svg" alt="LumenStream Logo" width="100%" />
  <p>🚀 Rust 实现的高性能、轻量级、Jellyfin 兼容流媒体服务器</p>

  <p>
    <a href="https://github.com/LumenStream/lumenstream-ce/actions"><img src="https://img.shields.io/github/actions/workflow/status/LumenStream/lumenstream-ce/ci-ghcr.yml?branch=main&logo=github&style=flat-square" alt="Build Status"></a>
    <a href="https://github.com/orgs/LumenStream/packages/container/package/lumenstream-ce-fullstack"><img src="https://img.shields.io/badge/registry-ghcr.io-2ea44f?logo=github&style=flat-square" alt="GHCR"></a>
    <img src="https://img.shields.io/badge/rust-1.75%2B-orange?logo=rust&style=flat-square" alt="Rust Version">
    <a href="https://github.com/LumenStream/lumenstream-ce/blob/main/LICENSE"><img src="https://img.shields.io/github/license/LumenStream/lumenstream-ce?style=flat-square" alt="License"></a>
  </p>
</div>

---

## 📖 项目简介

**LumenStream (CE)** 是一个基于 Rust 构建的现代化流媒体服务器，完全兼容大部分 Jellyfin 客户端核心 API。它旨在提供一个更轻量、更高性能的媒体中心体验，支持影视元数据刮削、增量扫描、海报墙展示以及强大的分布式推流控制。

自带现代化、响应式的 Web 界面（基于 Astro + React 构建），无论是管理你的媒体库还是享受观影时光，都能获得极佳的体验。

## 📸 界面预览

> **注：** 以下为界面截图展示位，您可以替换为您实际的运行截图。

<div align="center">
  <img src="https://via.placeholder.com/800x450?text=Home+Page+Screenshot" alt="首页展示" width="80%">
  <p><em>▲ 现代化海报墙与首页布局</em></p>
</div>

<div align="center">
  <img src="https://via.placeholder.com/800x450?text=Media+Detail+Screenshot" alt="媒体详情页展示" width="80%">
  <p><em>▲ 沉浸式媒体详情页（横幅背景 + 演职员信息）</em></p>
</div>

<div align="center">
  <img src="https://via.placeholder.com/800x450?text=Admin+Dashboard+Screenshot" alt="管理后台展示" width="80%">
  <p><em>▲ 强大的后台管理与任务中心</em></p>
</div>

## ✨ 核心特性

- 🎬 **Jellyfin 客户端兼容**：完美支持大多数 Jellyfin 官方及第三方客户端的登录、媒体浏览、播放记录与状态上报。
- ⚡ **极致性能**：基于 Rust 编写，内存占用极低，极速响应。
- 🔍 **智能扫描与刮削**：
  - 支持全量/增量路径扫描，无缝读取 NFO 与本地图片。
  - 内置 TMDB、TVDB、Bangumi 刮削器，自动补全演职员与剧集元数据。
- 🚀 **播放与推流**：
  - Direct Play 支持。
  - 与 LumenBackend 分布式节点协同，实现智能域名解析与流量分发。
- 🛡️ **安全与多用户**：
  - 完善的 RBAC（Admin/Viewer）角色控制。
  - 基于邀请码的注册体系与会话管理。
- 📊 **可观测性**：内置 Metrics、请求日志、P99/P95 延迟监控与播放成功率统计。
- 🎨 **现代化前端**：内置基于 Astro 构建的 Dark-theme 响应式 Web 客户端与管理台。

## 🚀 快速部署

LumenStream CE 推荐使用 Docker 进行一键部署，自带前后端一体镜像。默认镜像由 GitHub Actions 构建并发布到 **LumenStream 组织** 的 GHCR（`ghcr.io/lumenstream/lumenstream-ce-fullstack`）。

### 环境准备

确保您的系统已安装 [Docker](https://docs.docker.com/get-docker/) 和 [Docker Compose](https://docs.docker.com/compose/install/)。

### 一键启动 (Docker Compose)

创建一个新目录，并新建 `docker-compose.yml` 和 `.env` 文件。

**1. 配置 `.env` 文件（首次启动必须设置管理员凭据）**

```env
# 数据库配置
LS_DATABASE_URL=postgres://lumenstream:lumenstream@postgres:5432/lumenstream
LS_DATABASE_MAX_CONNECTIONS=50

# 初始管理员凭证 (启动后请妥善保管)
LS_BOOTSTRAP_ADMIN_USER=admin
LS_BOOTSTRAP_ADMIN_PASSWORD=your_secure_password
```

**2. 启动服务**

获取官方 `docker-compose.fullstack.yml`（或使用以下精简版）：

```yaml
version: '3.8'

services:
  lumenstream:
    image: ghcr.io/lumenstream/lumenstream-ce-fullstack:latest
    container_name: lumenstream
    network_mode: service:meilisearch # 共享网络以连接搜索引擎
    ports:
      - "8096:8096" # 后端 API
      - "4321:4321" # Web 前端
    env_file:
      - .env
    volumes:
      - ./data/cache:/app/cache
      - /path/to/your/media:/media:ro # 挂载您的媒体库目录
    restart: unless-stopped
    depends_on:
      postgres:
        condition: service_healthy

  postgres:
    image: postgres:15-alpine
    container_name: lumenstream_db
    environment:
      POSTGRES_USER: lumenstream
      POSTGRES_PASSWORD: lumenstream
      POSTGRES_DB: lumenstream
    volumes:
      - ./data/pgdata:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U lumenstream"]
      interval: 5s
      timeout: 5s
      retries: 5
    restart: unless-stopped

  meilisearch:
    image: getmeili/meilisearch:v1.15
    container_name: lumenstream_search
    environment:
      - MEILI_NO_ANALYTICS=true
    volumes:
      - ./data/meili_data:/meili_data
    restart: unless-stopped
```

执行启动命令：

```bash
docker compose up -d
```

**3. 访问系统**

- Web 界面: `http://localhost:4321`
- API 服务: `http://localhost:8096`

---

## 🛠️ 本地开发指南

如果您希望从源码编译和贡献代码：

### 依赖要求
- [Rust](https://rustup.rs/) (1.75+)
- [Node.js](https://nodejs.org/) & [Bun](https://bun.sh/)
- PostgreSQL
- Meilisearch

### 1. 启动依赖服务

```bash
# 启动 Meilisearch (默认 7700 端口)
docker run --rm -p 7700:7700 getmeili/meilisearch:v1.15

# 确保本地 PostgreSQL 已启动并创建相应数据库
```

### 2. 启动后端 API

```bash
cp config.example.yaml config.yaml
# 根据需要修改数据库连接等配置

export LS_BOOTSTRAP_ADMIN_USER="admin"
export LS_BOOTSTRAP_ADMIN_PASSWORD="admin_password"

cargo run -p ls-app
```

### 3. 启动 Web 前端

```bash
cd web
bun install
PUBLIC_LS_API_BASE_URL=http://127.0.0.1:8096 bun run dev
```

## 📂 项目结构

```text
lumenstream/
├── crates/
│   ├── ls-app/      # 核心后端入口与 HTTP 启动
│   ├── ls-api/      # REST API 路由控制器与中间件
│   ├── ls-domain/   # 核心领域模型与 DTO 定义
│   ├── ls-infra/    # 数据库仓储、任务系统、存储与鉴权实现
│   ├── ls-scraper/  # 元数据刮削引擎 (TMDB/TVDB/Bangumi)
│   └── ls-agent/    # 自动化任务与求片策略 Agent
├── migrations/      # 数据库表结构迁移脚本
├── web/             # 基于 Astro + React 构建的现代前端
└── docs/            # 详细架构与开发文档
```

## 🤝 参与贡献

欢迎提交 Issue 和 Pull Request！
在提交 PR 之前，请确保您的代码通过了所有测试，并且遵循了项目规范。

```bash
# 代码格式化与测试验证
cargo fmt --all
cargo test --workspace
```
请阅读 [CONTRIBUTING.md](./CONTRIBUTING.md) 和 [AGENTS.md](./AGENTS.md) 了解完整的开发与提交规范。

## 📄 许可证

本项目遵循相应的开源许可证，详情请参阅 [LICENSE](LICENSE) 文件。
