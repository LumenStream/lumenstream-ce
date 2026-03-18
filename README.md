<div align="center">
  <img src="./docs/assets/banner.svg" alt="LumenStream Logo" width="100%" />
</div>

<p align="center">
  <b>基于 Rust 的高性能、轻量级、兼容 Jellyfin 的流媒体服务器</b>
</p>

<p align="center">
  <a href="https://github.com/LumenStream/lumenstream-ce/actions"><img src="https://img.shields.io/github/actions/workflow/status/LumenStream/lumenstream-ce/ci-ghcr.yml?branch=main&logo=github&style=flat-square" alt="Build Status"></a>
  <a href="https://github.com/orgs/LumenStream/packages/container/package/lumenstream-ce-fullstack"><img src="https://img.shields.io/badge/registry-ghcr.io-2ea44f?logo=github&style=flat-square" alt="GHCR"></a>
  <img src="https://img.shields.io/badge/rust-1.75%2B-orange?logo=rust&style=flat-square" alt="Rust Version">
  <a href="https://github.com/LumenStream/lumenstream-ce/blob/main/LICENSE"><img src="https://img.shields.io/github/license/LumenStream/lumenstream-ce?style=flat-square" alt="License"></a>
</p>

---

## 项目简介

**LumenStream (社区版)** 是一个采用 Rust 从零构建的现代化流媒体服务端。它的设计初衷是解决传统流媒体服务器资源占用过高、响应迟缓的问题，同时保持对现有生态的良好兼容。

核心特性上，它拥有极低的内存占用和极快的响应速度，并且完全兼容绝大多数 Jellyfin 客户端。无论是元数据刮削、增量扫描、海报墙展示，还是复杂的分布式推流场景，LumenStream 都能提供稳定的支持。

此外，项目内置了一个基于 Astro 和 React 开发的现代化、响应式 Web 前端，为管理媒体库和日常观影提供流畅且美观的体验。

## 界面概览

<div align="center">
  <img src="./docs/assets/home-screenshot.webp" alt="LumenStream 首页界面" width="92%">
  <p><em>全新设计的响应式首页</em></p>
</div>

<table>
  <tr>
    <td width="50%" valign="top">
      <img src="./docs/assets/details-screenshot.webp" alt="LumenStream 媒体详情页">
      <p align="center"><em>媒体详情页</em></p>
    </td>
    <td width="50%" valign="top">
      <img src="./docs/assets/setting-screenshot.webp" alt="LumenStream 设置页">
      <p align="center"><em>系统设置页</em></p>
    </td>
  </tr>
</table>

## 核心特性

- **无缝兼容 Jellyfin**：原生支持绝大多数 Jellyfin 官方及第三方客户端的登录、媒体浏览、播放记录同步与状态上报。
- **极致的性能表现**：得益于 Rust 的底层优势，在提供极快 API 响应的同时，保持极低的内存占用。
- **智能扫描与刮削引擎**：
  - 支持全量与增量路径扫描，无缝读取本地 NFO 文件与图片资产。
  - 内置 TMDB、TVDB 与 Bangumi 刮削器，自动补全演职员与剧集元数据。
- **播放与推流控制**：
  - 支持 Direct Play 原画直出。
  - 可与 LumenBackend 分布式节点协同，实现智能域名解析与流量分发。
- **完善的可观测性**：内置 Metrics 指标收集、请求日志、P99/P95 延迟监控与播放成功率统计。
- **现代化前端体验**：内置基于 Astro 构建的深色主题响应式 Web 客户端与管理后台。

## 快速部署

我们推荐使用 Docker Compose 进行一键部署，LumenStream 提供了前后端一体化的镜像，开箱即用。默认镜像由 GitHub Actions 构建并发布到 GitHub 容器注册表（`ghcr.io/lumenstream/lumenstream-ce-fullstack`）。

### 1. 环境准备

请确保部署环境中已安装 [Docker](https://docs.docker.com/get-docker/) 和 [Docker Compose](https://docs.docker.com/compose/install/)。

### 2. 初始化配置

创建一个新的工作目录，并新建 `.env` 文件。首次启动必须设置管理员凭据：

```env
# 数据库连接配置
LS_DATABASE_URL=postgres://lumenstream:lumenstream@postgres:5432/lumenstream
LS_DATABASE_MAX_CONNECTIONS=50

# 初始管理员凭证 (请在启动后妥善保管)
LS_BOOTSTRAP_ADMIN_USER=admin
LS_BOOTSTRAP_ADMIN_PASSWORD=your_secure_password
```

### 3. 启动服务

创建 `docker-compose.yml` 文件：

```yaml
services:
  lumenstream:
    image: ghcr.io/lumenstream/lumenstream-ce-fullstack:latest
    container_name: lumenstream
    network_mode: service:meilisearch # 共享网络以连接搜索引擎
    ports:
      - "8096:8096" # API 与客户端服务端口
      - "4321:4321" # Web 前端访问端口
    env_file:
      - .env
    volumes:
      - ./data/cache:/app/cache
      - /path/to/your/media:/media:ro # 请替换为实际的媒体库路径
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

执行以下命令启动服务：

```bash
docker compose up -d
```

### 4. 访问系统

服务启动后：
- **Web 界面**: 访问 `http://localhost:4321` 体验管理与播放功能。
- **Jellyfin 客户端**: 在服务器地址中填入 `http://<服务器IP>:8096` 即可连接。

---

## 本地开发指南

如果您希望从源码编译或参与项目贡献，请参考以下指南。

### 环境依赖

- [Rust](https://rustup.rs/) (要求 1.75+ 版本)
- [Node.js](https://nodejs.org/) & [Bun](https://bun.sh/)
- PostgreSQL
- Meilisearch

### 1. 启动基础服务

首先需要运行依赖的数据库与搜索引擎：

```bash
# 启动 Meilisearch (默认监听 7700 端口)
docker run --rm -p 7700:7700 getmeili/meilisearch:v1.15

# 请确保本地 PostgreSQL 已启动并创建了相应的数据库
```

### 2. 启动 Rust 后端 API

```bash
cp config.example.yaml config.yaml
# 根据实际情况修改数据库连接等配置

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


```

## 参与贡献

我们非常欢迎以 Issue 或 Pull Request 的形式参与贡献。

在提交代码前，请确保遵循了代码格式规范并通过了所有测试：

```bash
cargo fmt --all
cargo test --workspace
```

更多详细的开发与提交规范，请参阅 [CONTRIBUTING.md](./CONTRIBUTING.md) 和 [AGENTS.md](./AGENTS.md)。

## 许可证说明

本项目采用 [GNU Affero General Public License v3.0](https://www.gnu.org/licenses/agpl-3.0.html)（AGPL-3.0）协议开源，完整条款请参阅代码库中的 [LICENSE](LICENSE) 文件。

**商业授权**

如果您计划将本项目（LumenStream）用于商业二次开发、闭源分发，以及任何其他不适用于 AGPL-3.0 协议的场景，请联系 `admin@sleepstars.net` 获取商业授权许可。
