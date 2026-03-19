# Technology Stack

**Analysis Date:** 2026-03-20

## Languages

**Primary:**
- Rust (workspace `edition = "2024"`) - backend API, domain, infra, config, logging, scraper, and agent crates in `Cargo.toml`, `crates/ls-app/Cargo.toml`, `crates/ls-api/Cargo.toml`, `crates/ls-infra/Cargo.toml`, `crates/ls-config/Cargo.toml`, `crates/ls-domain/Cargo.toml`, `crates/ls-logging/Cargo.toml`, `crates/ls-scraper/Cargo.toml`, and `crates/ls-agent/Cargo.toml`.

**Secondary:**
- TypeScript - frontend application and tests in `web/package.json`, `web/src/`, and `web/tsconfig.json`.
- JavaScript (ESM) - Astro/Vite config in `web/astro.config.mjs`.
- Shell - container entrypoints, runtime config injection, and repo automation in `docker/fullstack-entrypoint.sh`, `docker/web-entrypoint.sh`, `docker/write-web-runtime-config.sh`, and `scripts/`.
- YAML - bootstrap config and deployment manifests in `config.example.yaml`, `docker-compose.fullstack.yml`, `docker-compose.fullstack.localbackend.yml`, and `.github/workflows/ci-ghcr.yml`.

## Runtime

**Environment:**
- Rust backend runs as `ls-app` from `crates/ls-app/src/main.rs`.
- Actix uses Tokio async runtime via `#[actix_web::main]` in `crates/ls-app/src/main.rs`.
- Frontend development/runtime uses Bun 1.2.2 from `web/package.json`, `Dockerfile.fullstack`, `docker/Dockerfile.web`, and `.github/workflows/ci-ghcr.yml`.
- Container runtime is Debian-based in `Dockerfile` and `Dockerfile.fullstack`; the fullstack image also runs Nginx for the web UI from `docker/nginx.conf`.

**Package Manager:**
- Rust: Cargo workspace defined in `Cargo.toml`.
- Frontend: Bun via `web/package.json`.
- Lockfiles: `Cargo.lock` present, `web/bun.lock` present.

## Frameworks

**Core:**
- Actix Web 4.11 - HTTP API server and middleware in `Cargo.toml`, `crates/ls-app/src/main.rs`, and `crates/ls-api/src/lib.rs`.
- SQLx 0.8.6 - PostgreSQL access and migrations in `Cargo.toml`, `crates/ls-infra/src/db.rs`, and `migrations/`.
- Astro 5.17.2 - static web app shell in `web/package.json` and `web/astro.config.mjs`.
- React 19 - Astro islands-based UI in `web/package.json` and `web/src/islands/`.
- Tailwind CSS 4 - frontend styling through Vite in `web/package.json` and `web/astro.config.mjs`.

**Testing:**
- Rust unit/integration tests via `cargo test --workspace` from `justfile` and `.github/workflows/ci-ghcr.yml`.
- Vitest 4 for frontend tests from `web/package.json`.

**Build/Dev:**
- Bun + Vite build pipeline for `web/` in `web/package.json` and `web/astro.config.mjs`.
- Docker multi-stage images in `Dockerfile`, `Dockerfile.fullstack`, and `docker/Dockerfile.web`.
- Just command runner in `justfile`.
- Prek pre-commit orchestration in `prek.toml`.
- GitHub Actions CI/CD in `.github/workflows/ci-ghcr.yml`.

## Key Dependencies

**Critical:**
- `actix-web`, `actix-cors`, `actix-ws` - REST API, CORS, and WebSocket support in `Cargo.toml` and `crates/ls-api/Cargo.toml`.
- `sqlx` - PostgreSQL pool and embedded migrations in `Cargo.toml` and `crates/ls-infra/src/db.rs`.
- `reqwest` - outbound HTTP for metadata, agent, and provider calls in `Cargo.toml`, `crates/ls-infra/src/infra/prelude.rs`, `crates/ls-agent/src/moviepilot.rs`, `crates/ls-agent/src/llm.rs`, `crates/ls-scraper/src/tvdb.rs`, and `crates/ls-scraper/src/bangumi.rs`.
- `meilisearch-sdk` - search index client in `Cargo.toml`, `crates/ls-infra/Cargo.toml`, and `crates/ls-infra/src/infra/app_core.rs`.
- `tracing` and `tracing-subscriber` - structured logging in `Cargo.toml` and `crates/ls-logging/src/lib.rs`.

**Infrastructure:**
- `argon2`, `hmac`, `sha2`, `base64`, `md5` - auth, signing, and token/hash helpers in `Cargo.toml` and `crates/ls-infra/src/infra/prelude.rs`.
- `cron` - scheduler definitions in `Cargo.toml` and `crates/ls-infra/src/infra/utils_tasks_lumenbackend.rs`.
- `image` and system `libvips-tools` - image handling and media asset processing in `Cargo.toml`, `crates/ls-infra/Cargo.toml`, `Dockerfile`, and `Dockerfile.fullstack`.
- `pinyin` and `unicode-normalization` - multilingual search key generation in `Cargo.toml` and `crates/ls-infra/src/search.rs`.
- `@astrojs/react`, `react`, `react-dom`, `tailwindcss`, `@tailwindcss/vite`, `framer-motion`, `@react-three/fiber`, and `three` - frontend rendering and interaction stack in `web/package.json`.
- `eslint`, `prettier`, `typescript`, and `vitest` - frontend quality tooling in `web/package.json`.

## Workspace Crates

**Runtime crates:**
- `crates/ls-app` - executable entrypoint that loads config, initializes logging/infra, spawns scheduler, and serves HTTP.
- `crates/ls-api` - route registration, middleware, compatibility endpoints, billing routes, and metrics endpoint.
- `crates/ls-infra` - database access, search, scheduling, playback routing, billing logic, scraper orchestration, and agent workflows.

**Support crates:**
- `crates/ls-config` - config schema, defaults, and environment overrides.
- `crates/ls-domain` - shared domain models.
- `crates/ls-logging` - runtime-configurable tracing setup.
- `crates/ls-scraper` - TVDB/Bangumi scraper clients and provider descriptors.
- `crates/ls-agent` - LLM and MoviePilot integrations for request automation.

## Configuration

**Environment:**
- Bootstrap file config is intentionally narrow: only `database` is loaded from YAML in `crates/ls-config/src/lib.rs`; the sample lives in `config.example.yaml`.
- Runtime config is primarily env-driven through `AppConfig::load_default` and `AppConfig::load_from_path` in `crates/ls-config/src/lib.rs`.
- Important env entrypoints include `LS_CONFIG_PATH`, `LS_DATABASE_URL`, `LS_DATABASE_MAX_CONNECTIONS`, `LS_BOOTSTRAP_ADMIN_USER`, `LS_BOOTSTRAP_ADMIN_PASSWORD`, `LS_CORS_ALLOW_ORIGINS`, `MEILI_MASTER_KEY`, `LS_TMDB_API_KEY`, `LS_TVDB_*`, `LS_BANGUMI_*`, `LS_AGENT_*`, `LS_BILLING_*`, and `LS_LUMENBACKEND_*` in `crates/ls-config/src/lib.rs`.
- Frontend build-time API config uses `PUBLIC_LS_API_BASE_URL`, `PUBLIC_LS_ENABLE_MOCK`, and `PUBLIC_LS_MOCK_MODE` in `web/src/env.d.ts`, `web/astro.config.mjs`, and `web/src/lib/api/client.ts`.
- Frontend runtime API override is injected to `window.__LS_CONFIG__` by `docker/write-web-runtime-config.sh`.

**Build:**
- Backend workspace manifest: `Cargo.toml`.
- Frontend manifest and build config: `web/package.json`, `web/astro.config.mjs`, `web/tsconfig.json`.
- Container builds: `Dockerfile`, `Dockerfile.fullstack`, `docker/Dockerfile.web`.
- Deployment manifests: `docker-compose.fullstack.yml`, `docker-compose.fullstack.localbackend.yml`.
- CI and pre-commit: `.github/workflows/ci-ghcr.yml`, `prek.toml`, `justfile`.

## Platform Requirements

**Development:**
- Rust toolchain and Cargo workspace from `Cargo.toml`; GitHub Actions uses `dtolnay/rust-toolchain@stable` in `.github/workflows/ci-ghcr.yml`.
- Bun 1.2.2 for the frontend in `web/package.json` and `.github/workflows/ci-ghcr.yml`.
- PostgreSQL and Meilisearch are required for normal backend development per `README.md`, `crates/ls-infra/src/db.rs`, and `crates/ls-infra/src/infra/app_core.rs`.
- Media helpers expected by containers include `ffmpeg` or `ffprobe` and `libvips-tools` from `Dockerfile` and `Dockerfile.fullstack`.

**Production:**
- Default shipping target is a Dockerized fullstack image built from `Dockerfile.fullstack` and published by `.github/workflows/ci-ghcr.yml`.
- `docker-compose.fullstack.yml` runs PostgreSQL, Meilisearch, and the `ghcr.io/lumenstream/lumenstream-ce-fullstack` image.
- `docker-compose.fullstack.localbackend.yml` adds `lumenlocalbackend` and `stream-gateway` for unified local-media streaming.

---

*Stack analysis: 2026-03-20*
