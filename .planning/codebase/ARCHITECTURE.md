# Architecture

**Analysis Date:** 2026-03-20

## Pattern Overview

**Overall:** Modular monolith in a Rust Cargo workspace, with a separate Astro + React web client in `web/`.

**Key Characteristics:**
- `crates/ls-app/src/main.rs` is a thin bootstrap binary that wires config, logging, infrastructure, scheduler, and the Actix HTTP server.
- `crates/ls-api/src/lib.rs` is the HTTP boundary and Jellyfin/Emby compatibility layer; it exposes routes, middleware, request parsing, and response shaping.
- `crates/ls-infra/src/lib.rs` is the main application core. Most business rules, SQL access, background jobs, search integration, and playback routing live here.
- `crates/ls-domain/src/model.rs` and `crates/ls-domain/src/jellyfin.rs` centralize domain structs and compatibility DTOs so API and infra share the same contracts.
- `web/src/pages/` defines page routes and shells, while `web/src/islands/` contains the interactive React application loaded inside Astro pages.
- Both `crates/ls-api/src/lib.rs` and `crates/ls-infra/src/lib.rs` use `include!` to split very large modules into focused files while keeping one crate-level namespace.

## Layers

**Bootstrap / Process Layer:**
- Purpose: Start the backend process and compose runtime dependencies.
- Location: `crates/ls-app/src/main.rs`
- Contains: config loading, logging initialization, `AppInfra::init`, `ApiContext` creation, scheduler startup, `HttpServer`.
- Depends on: `crates/ls-config/src/lib.rs`, `crates/ls-logging/src/lib.rs`, `crates/ls-infra/src/lib.rs`, `crates/ls-api/src/lib.rs`.
- Used by: runtime containers and local `cargo run -p ls-app`.

**Configuration Layer:**
- Purpose: Load bootstrap config and runtime capability flags.
- Location: `crates/ls-config/src/lib.rs`, `config.example.yaml`
- Contains: `AppConfig`, edition gating, env overrides, web-config projection, runtime config merge helpers.
- Depends on: file/env input plus serde types.
- Used by: `crates/ls-app/src/main.rs`, `crates/ls-infra/src/infra/app_core.rs`, `crates/ls-api/src/api/router.rs`, `crates/ls-logging/src/lib.rs`.

**HTTP / Compatibility Layer:**
- Purpose: Expose REST endpoints and normalize Jellyfin/Emby-compatible behavior.
- Location: `crates/ls-api/src/lib.rs`, `crates/ls-api/src/api/router.rs`, `crates/ls-api/src/api/middleware.rs`, `crates/ls-api/src/api/routes_*.rs`
- Contains: route registration, auth guards, request parsing, compatibility path normalization, DTO serialization, error-to-response mapping.
- Depends on: `ApiContext`, `ls-domain` DTOs, `ls-infra` methods.
- Used by: Actix server created in `crates/ls-app/src/main.rs`.

**Application / Service Layer:**
- Purpose: Hold application state and implement business workflows.
- Location: `crates/ls-infra/src/lib.rs`, especially `crates/ls-infra/src/infra/app_*.rs`
- Contains: `AppInfra`, user auth/session flows, library management, media browse/search/playback, playlists, notifications, billing, scraper orchestration, agent requests, task center.
- Depends on: `sqlx` pool from `crates/ls-infra/src/db.rs`, scraper/agent crates, config, search helpers, HTTP client.
- Used by: every route handler through `state.infra`.

**Persistence / Mapping Layer:**
- Purpose: Isolate DB connection, migrations, query row structs, and conversion into domain/API types.
- Location: `crates/ls-infra/src/db.rs`, `crates/ls-infra/src/infra/db_rows_user_playlist.rs`, `crates/ls-infra/src/infra/db_rows_system.rs`, `crates/ls-infra/src/infra/mappers_user_media.rs`, `migrations/*.sql`
- Contains: `sqlx::migrate!`, row structs, `FromRow` adapters, mapping helpers for `UserDto`, `BaseItemDto`, playlists, sessions, playback domains, audit logs.
- Depends on: PostgreSQL schema plus `ls-domain` and `ls-domain::jellyfin`.
- Used by: `AppInfra` methods.

**Background Job Layer:**
- Purpose: Run scheduled and queued work outside request/response flow.
- Location: `crates/ls-infra/src/scheduler/mod.rs`, `crates/ls-infra/src/scheduler/*.rs`, `crates/ls-infra/src/infra/app_jobs_*.rs`, `crates/ls-infra/src/infra/app_agent_requests.rs`
- Contains: cron dispatch, queued job dispatch, scan/tmdb/scraper/cleanup/retry/billing/agent job execution.
- Depends on: `AppInfra`, jobs table, task definitions, Tokio tasks.
- Used by: startup flow in `crates/ls-app/src/main.rs` and APIs that enqueue jobs.

**Shared Domain Layer:**
- Purpose: Provide common Rust data contracts.
- Location: `crates/ls-domain/src/model.rs`, `crates/ls-domain/src/jellyfin.rs`
- Contains: persisted entities, admin-facing structs, Jellyfin/Emby request and response DTOs.
- Depends on: serde/uuid/chrono only.
- Used by: `ls-api`, `ls-infra`, sometimes tests.

**Integration Layer:**
- Purpose: Encapsulate external platform behavior.
- Location: `crates/ls-scraper/src/*.rs`, `crates/ls-agent/src/*.rs`, `crates/ls-infra/src/search.rs`
- Contains: TMDB/TVDB/Bangumi scraper clients, MoviePilot and LLM agent abstractions, Meilisearch key generation helpers.
- Depends on: external HTTP APIs and Meilisearch.
- Used by: `AppInfra` job flows and browse/search methods.

**Web UI Layer:**
- Purpose: Serve landing page, user app, and admin app.
- Location: `web/src/pages/`, `web/src/layouts/`, `web/src/islands/`, `web/src/lib/`
- Contains: Astro routes/layouts, React islands, API client wrappers, auth/session utilities, UI/domain components, mock mode.
- Depends on: backend HTTP API from `crates/ls-api`, runtime config script, browser session storage.
- Used by: static frontend build from `web/`.

## Data Flow

**Backend Request Flow:**

1. `crates/ls-app/src/main.rs` loads `AppConfig`, initializes logging, creates `AppInfra`, wraps it in `ApiContext`, and starts Actix.
2. `crates/ls-api/src/api/router.rs` builds one root scope and attaches middleware from `crates/ls-api/src/api/middleware.rs`.
3. Middleware normalizes `/emby` and `/jellyfin` prefixes, rewrites compatibility path casing, injects missing JSON content types, and records request metrics.
4. Route handlers in `crates/ls-api/src/api/routes_*.rs` authenticate the request, parse query/body data, and call methods on `state.infra`.
5. `crates/ls-infra/src/infra/app_*.rs` runs the actual workflow, usually using SQL queries, row mappers, DTO mapping helpers, and optional external integrations.
6. `crates/ls-api/src/api/error_map.rs` and helper functions turn infra results into HTTP status codes and compatibility-shaped JSON payloads.

**Startup / Runtime Config Flow:**

1. `crates/ls-config/src/lib.rs` loads `config.yaml` or `LS_CONFIG_PATH`, but only seeds bootstrap file fields such as database settings from disk.
2. The same config loader applies extensive `LS_*` environment variable overrides.
3. `crates/ls-infra/src/infra/app_core.rs` connects to PostgreSQL, runs `migrations/*.sql`, then loads web-managed settings from the database and applies them back into the in-memory config.
4. The resulting `AppInfra` exposes `config_snapshot()` so HTTP and scheduler logic always read the current runtime config.

**Background Job Flow:**

1. `crates/ls-infra/src/scheduler/mod.rs` wakes every 30 seconds after startup.
2. It reads task definitions from the database, evaluates cron schedules, enqueues due jobs, and dispatches queued jobs.
3. Job execution is implemented in `crates/ls-infra/src/infra/app_jobs_queue.rs` and related `app_jobs_*.rs` files, using the `jobs` table as the durable queue.
4. APIs such as `crates/ls-api/src/api/routes_requests_agent.rs` can also enqueue a job immediately and spawn `process_job()` in the background.

**Frontend Page Flow:**

1. Astro page files in `web/src/pages/` choose a layout and mount one or more React islands with `client:load`.
2. Layouts in `web/src/layouts/BaseLayout.astro`, `web/src/layouts/AppLayout.astro`, and `web/src/layouts/AdminLayout.astro` define chrome, theme bootstrapping, and shared navigation.
3. Islands such as `web/src/islands/media/HomeDashboard.tsx` or `web/src/islands/admin/AdminOverviewPanel.tsx` use helpers from `web/src/lib/api/*.ts` to call backend endpoints.
4. `web/src/lib/api/client.ts` builds URLs from runtime config and attaches the bearer token from `web/src/lib/auth/token.ts`.
5. Page state is held locally in React hooks; there is no global client-side state container.

**State Management:**
- Backend shared state is centralized in `ApiContext` (`crates/ls-api/src/api/prelude.rs`) and `AppInfra` (`crates/ls-infra/src/infra/prelude.rs`).
- Runtime mutable config is stored inside `AppInfra.config` as `Arc<RwLock<AppConfig>>`.
- Realtime notifications use Tokio `broadcast` channels inside `AppInfra`.
- Frontend auth state is session-storage based in `web/src/lib/auth/token.ts`; view state is local component state and hooks.

## Key Abstractions

**ApiContext:**
- Purpose: Request-time container for infrastructure, metrics, and optional log handle.
- Examples: `crates/ls-api/src/api/prelude.rs`, `crates/ls-api/src/api/router.rs`
- Pattern: lightweight Actix application state passed into every route.

**AppInfra:**
- Purpose: Long-lived application service object that owns DB pool, config snapshot, HTTP client, metrics, search backend, and realtime channels.
- Examples: `crates/ls-infra/src/infra/prelude.rs`, `crates/ls-infra/src/infra/app_core.rs`
- Pattern: service facade over a modular monolith; most use cases are methods on `AppInfra`.

**Compatibility DTO Layer:**
- Purpose: Decouple internal persistence shape from Jellyfin/Emby-compatible payloads.
- Examples: `crates/ls-domain/src/jellyfin.rs`, `crates/ls-infra/src/infra/mappers_user_media.rs`
- Pattern: map DB/domain models into client-facing DTOs at the edge.

**Jobs and Task Definitions:**
- Purpose: Represent durable background work and cron-managed recurring tasks.
- Examples: `crates/ls-domain/src/model.rs`, `crates/ls-infra/src/infra/app_jobs_queue.rs`, `crates/ls-infra/src/scheduler/mod.rs`
- Pattern: database-backed queue with in-process dispatch.

**Include-Based Module Partitioning:**
- Purpose: Keep `ls-api` and `ls-infra` in a single namespace while splitting the monolith into focused files.
- Examples: `crates/ls-api/src/lib.rs`, `crates/ls-infra/src/lib.rs`
- Pattern: add a new `api/routes_*.rs` or `infra/app_*.rs` file, then register it with an `include!` entry.

## Entry Points

**Backend Binary:**
- Location: `crates/ls-app/src/main.rs`
- Triggers: `cargo run -p ls-app`, container `ENTRYPOINT` in `Dockerfile`
- Responsibilities: load config, init logging, init infra, spawn scheduler, start HTTP server.

**API Router:**
- Location: `crates/ls-api/src/api/router.rs`
- Triggers: mounted by `crates/ls-app/src/main.rs`
- Responsibilities: register every backend endpoint and attach compatibility/auth/metrics middleware.

**Scheduler Loop:**
- Location: `crates/ls-infra/src/scheduler/mod.rs`
- Triggers: `scheduler::spawn_scheduler(state.infra.clone())` in `crates/ls-app/src/main.rs`
- Responsibilities: cron evaluation, due-task enqueue, queued-job dispatch.

**Frontend Page Routes:**
- Location: `web/src/pages/index.astro`, `web/src/pages/login.astro`, `web/src/pages/app/*.astro`, `web/src/pages/admin/*.astro`
- Triggers: Astro static routing.
- Responsibilities: choose layout, mount the correct island, define user vs admin shell.

**Fullstack Image Build:**
- Location: `Dockerfile.fullstack`, `docker-compose.fullstack.yml`
- Triggers: fullstack container builds and local compose deployments.
- Responsibilities: build `web/`, package the backend runtime, and expose backend/API/web ports for deployment.

## Error Handling

**Strategy:** Backend code uses typed infra errors plus `anyhow` context internally, then converts failures to HTTP responses at the API edge.

**Patterns:**
- `crates/ls-app/src/main.rs` wraps startup failures with `anyhow::Context` so process-level errors are explicit.
- `crates/ls-infra/src/db.rs` and many `AppInfra` methods return `anyhow::Result<_>` for internal flows.
- `crates/ls-infra/src/infra/prelude.rs` defines `InfraError` for user-facing business failures such as billing disabled or stream access denied.
- Route files such as `crates/ls-api/src/api/routes_items_browse.rs` and `crates/ls-api/src/api/routes_admin_libraries_tasks.rs` convert errors into `StatusCode` plus JSON/text responses.
- `crates/ls-api/src/api/error_map.rs` centralizes some error-to-status translation so compat endpoints stay consistent.

## Cross-Cutting Concerns

**Logging:** `crates/ls-logging/src/lib.rs` initializes tracing with runtime-adjustable levels. API middleware also attaches `middleware::Logger`.

**Validation:** Input validation is mostly manual and local to route handlers or infra helpers, for example `normalize_library_type`, `resolve_create_library_paths`, and config normalization in `crates/ls-config/src/lib.rs`.

**Authentication:** `crates/ls-api/src/api/helpers_auth.rs` provides request guards; backend sessions and tokens are managed by `crates/ls-infra/src/infra/app_auth_tokens.rs`; frontend tokens live in `web/src/lib/auth/token.ts`.

**Compatibility:** `crates/ls-api/src/api/middleware.rs` and `crates/ls-domain/src/jellyfin.rs` are the main compatibility seam for Jellyfin/Emby clients.

**Observability:** API metrics are tracked in `crates/ls-api/src/api/prelude.rs`; infra/job/scraper metrics live in `crates/ls-infra/src/infra/prelude.rs`; `/metrics` is exposed from `crates/ls-api/src/api/router.rs`.

**Edition Gating:** `crates/ls-config/src/lib.rs` computes capability flags and normalizes both backend and web-facing config, while `crates/ls-api/src/api/router_commercial.rs` and `crates/ls-api/src/api/helpers_edition_masks.rs` participate in route-level gating.

---

*Architecture analysis: 2026-03-20*
