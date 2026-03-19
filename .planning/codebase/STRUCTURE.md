# Codebase Structure

**Analysis Date:** 2026-03-20

## Directory Layout

```text
lumenstream-ce/
├── crates/                 # Rust workspace crates for backend runtime, API, config, domain, infra, logging, scraper, agent
├── migrations/             # PostgreSQL schema migrations used by `sqlx::migrate!`
├── web/                    # Astro + React frontend
├── docker/                 # Web/runtime entrypoint helpers and nginx config fragments
├── docs/                   # Repository and product documentation assets
├── scripts/                # Repository split/export automation scripts
├── .planning/codebase/     # Generated architecture/reference documents for GSD
├── Cargo.toml              # Workspace definition and shared Rust dependencies
├── justfile                # Common backend/frontend/dev commands
├── prek.toml               # Pre-commit hook configuration
├── config.example.yaml     # Bootstrap config example
├── Dockerfile              # Backend runtime image
├── Dockerfile.fullstack    # Combined backend + frontend image
└── docker-compose.fullstack.yml  # Local fullstack deployment topology
```

## Directory Purposes

**`crates/ls-app/`:**
- Purpose: backend process entrypoint.
- Contains: `src/main.rs` only.
- Key files: `crates/ls-app/src/main.rs`

**`crates/ls-api/`:**
- Purpose: HTTP layer and Jellyfin/Emby-compatible API surface.
- Contains: `src/lib.rs` plus `src/api/*.rs` route, middleware, helper, and test files.
- Key files: `crates/ls-api/src/lib.rs`, `crates/ls-api/src/api/router.rs`, `crates/ls-api/src/api/middleware.rs`

**`crates/ls-config/`:**
- Purpose: runtime/bootstrap configuration and edition capability logic.
- Contains: `AppConfig`, env parsing, web-config projection.
- Key files: `crates/ls-config/src/lib.rs`

**`crates/ls-domain/`:**
- Purpose: shared structs between API and infra.
- Contains: persisted model structs and Jellyfin/Emby DTOs.
- Key files: `crates/ls-domain/src/model.rs`, `crates/ls-domain/src/jellyfin.rs`

**`crates/ls-infra/`:**
- Purpose: business logic, persistence, scheduler, search, and integrations.
- Contains: `src/lib.rs`, `src/db.rs`, `src/search.rs`, `src/scanner.rs`, `src/scheduler/*.rs`, `src/infra/*.rs`
- Key files: `crates/ls-infra/src/lib.rs`, `crates/ls-infra/src/infra/app_core.rs`, `crates/ls-infra/src/scheduler/mod.rs`

**`crates/ls-logging/`:**
- Purpose: centralized tracing configuration and runtime log-level control.
- Contains: logging init and reload handle code.
- Key files: `crates/ls-logging/src/lib.rs`

**`crates/ls-scraper/`:**
- Purpose: external metadata scraping providers and NFO helpers.
- Contains: TMDB/TVDB/Bangumi clients, provider traits, NFO helpers.
- Key files: `crates/ls-scraper/src/lib.rs`, `crates/ls-scraper/src/tmdb.rs`, `crates/ls-scraper/src/tvdb.rs`, `crates/ls-scraper/src/bangumi.rs`

**`crates/ls-agent/`:**
- Purpose: request-agent workflows and provider abstractions.
- Contains: workflow modeling, LLM/MoviePilot provider interfaces and helpers.
- Key files: `crates/ls-agent/src/lib.rs`, `crates/ls-agent/src/workflow.rs`, `crates/ls-agent/src/moviepilot.rs`

**`migrations/`:**
- Purpose: ordered SQL migrations applied on startup.
- Contains: numbered migration files from `0001_squashed_baseline.sql` through `0023_agent_loop_runtime.sql`.
- Key files: `migrations/0001_squashed_baseline.sql`, `migrations/0020_agent_requests.sql`, `migrations/0023_agent_loop_runtime.sql`

**`web/src/pages/`:**
- Purpose: Astro route files for landing, auth, user app, and admin app.
- Contains: `.astro` pages under `admin/` and `app/`.
- Key files: `web/src/pages/index.astro`, `web/src/pages/app/home.astro`, `web/src/pages/admin/overview.astro`

**`web/src/layouts/`:**
- Purpose: shared HTML and shell layouts.
- Contains: base, user-app, and admin layouts.
- Key files: `web/src/layouts/BaseLayout.astro`, `web/src/layouts/AppLayout.astro`, `web/src/layouts/AdminLayout.astro`

**`web/src/islands/`:**
- Purpose: client-side React features mounted by Astro.
- Contains: admin panels, media screens, auth forms, navigation widgets, landing sections, billing flows.
- Key files: `web/src/islands/media/HomeDashboard.tsx`, `web/src/islands/admin/AdminOverviewPanel.tsx`, `web/src/islands/navigation/HeaderAccountEntry.tsx`

**`web/src/components/`:**
- Purpose: reusable presentation primitives.
- Contains: `ui/`, `domain/`, `navigation/`, `brand/`, `effects/`, `edition/`.
- Key files: `web/src/components/domain/PosterItemCard.tsx`, `web/src/components/ui/button.tsx`, `web/src/components/navigation/FloatingDock.tsx`

**`web/src/lib/`:**
- Purpose: client-side data access and reusable frontend logic.
- Contains: API wrappers, auth/session utilities, hooks, types, mock mode, notifications, navigation helpers.
- Key files: `web/src/lib/api/client.ts`, `web/src/lib/api/items.ts`, `web/src/lib/auth/use-auth-session.ts`, `web/src/lib/types/jellyfin.ts`

**`docker/`:**
- Purpose: deployment helper scripts and config snippets for bundled images.
- Contains: entrypoint scripts referenced by dev tooling and compose files.
- Key files: `docker/Dockerfile.web`, `docker/nginx.stream-gateway.localbackend.conf`

**`docs/`:**
- Purpose: user-facing repository docs and media assets.
- Contains: generated repository notes, edition notes, screenshots, logo/banner assets.
- Key files: `docs/editions.md`, `docs/generated-repository.md`

**`scripts/`:**
- Purpose: repository management and CE/commercial split automation.
- Contains: export, init, sync, cut-split shell scripts.
- Key files: `scripts/export_ce_upstream.sh`, `scripts/init_commercial_downstream.sh`, `scripts/sync_from_ce_upstream.sh`

## Key File Locations

**Entry Points:**
- `crates/ls-app/src/main.rs`: backend executable entrypoint.
- `crates/ls-api/src/api/router.rs`: all HTTP route registration.
- `crates/ls-infra/src/scheduler/mod.rs`: background scheduler entrypoint.
- `web/src/pages/index.astro`: public landing route.
- `web/src/pages/app/home.astro`: user app entry route.
- `web/src/pages/admin/overview.astro`: admin app entry route.

**Configuration:**
- `Cargo.toml`: workspace members and shared Rust dependencies.
- `config.example.yaml`: bootstrap config example for DB and env-driven runtime setup.
- `crates/ls-config/src/lib.rs`: actual config parsing and normalization.
- `web/astro.config.mjs`: Astro/Vite config and alias/mock switching.
- `web/package.json`: frontend scripts and dependencies.
- `prek.toml`: repository-wide pre-commit checks.

**Core Logic:**
- `crates/ls-infra/src/infra/app_core.rs`: infra initialization and seeding.
- `crates/ls-infra/src/infra/app_media_root_search.rs`: browsing and search workflows.
- `crates/ls-infra/src/infra/app_media_playback_stream.rs`: playback and stream routing.
- `crates/ls-infra/src/infra/app_libraries_storage_domains.rs`: libraries, storage config, playback domains.
- `crates/ls-infra/src/infra/app_jobs_queue.rs`: durable job queue and dispatch.
- `crates/ls-infra/src/infra/app_agent_requests.rs`: request-agent orchestration.

**Testing:**
- `crates/ls-api/src/api/tests.rs`: API-level backend tests.
- `crates/ls-infra/src/infra/tests.rs`: infra/service tests.
- `web/src/**/*.test.tsx`: component and island tests next to implementation files.
- `web/src/**/*.test.ts`: frontend utility tests next to implementation files.

## Naming Conventions

**Files:**
- Rust workspace crates use the `ls-*` prefix: `crates/ls-api`, `crates/ls-infra`, `crates/ls-agent`.
- Backend API split files use role prefixes: `crates/ls-api/src/api/routes_*.rs`, `crates/ls-api/src/api/helpers_*.rs`.
- Backend infra split files use domain prefixes: `crates/ls-infra/src/infra/app_*.rs`, `crates/ls-infra/src/infra/db_rows_*.rs`, `crates/ls-infra/src/infra/mappers_*.rs`, `crates/ls-infra/src/infra/utils_*.rs`.
- Scheduler files are grouped by job class: `crates/ls-infra/src/scheduler/library_scan.rs`, `crates/ls-infra/src/scheduler/job_retry.rs`.
- Astro route files are lowercase path-oriented names: `web/src/pages/app/home.astro`, `web/src/pages/admin/traffic.astro`.
- React component and island files use PascalCase: `web/src/islands/media/HomeDashboard.tsx`, `web/src/components/domain/AddToPlaylistModal.tsx`.
- Tests are co-located and named `*.test.ts` or `*.test.tsx`.

**Directories:**
- Backend code is grouped by crate first, then by responsibility inside `src/`.
- Frontend code is grouped by runtime role: pages in `web/src/pages/`, layouts in `web/src/layouts/`, interactive features in `web/src/islands/`, reusable code in `web/src/lib/`.

## Where to Add New Code

**New Backend Feature:**
- Primary code: add the use-case implementation to the closest `crates/ls-infra/src/infra/app_*.rs` file; create a new `app_*.rs` file only when the concern is large enough to justify another split unit.
- HTTP surface: add or extend the matching file in `crates/ls-api/src/api/routes_*.rs`.
- Registration step: update `crates/ls-api/src/lib.rs` or `crates/ls-infra/src/lib.rs` with a new `include!` line if you create a new split file.
- Shared DTO/model changes: put persisted structs in `crates/ls-domain/src/model.rs`; put Jellyfin/Emby payload structs in `crates/ls-domain/src/jellyfin.rs`.
- Tests: backend route tests belong in `crates/ls-api/src/api/tests.rs`; service/integration-style tests belong in `crates/ls-infra/src/infra/tests.rs`.

**New Database-Backed Capability:**
- Schema change: add a new numbered SQL file under `migrations/`.
- Query row adapters: add or extend row structs in `crates/ls-infra/src/infra/db_rows_user_playlist.rs` or `crates/ls-infra/src/infra/db_rows_system.rs`, depending on domain fit.
- Mapping logic: keep API-shaping mappers in `crates/ls-infra/src/infra/mappers_user_media.rs` or a new mapper split file registered from `crates/ls-infra/src/lib.rs`.

**New Background Job or Scheduled Task:**
- Queue/execution logic: `crates/ls-infra/src/infra/app_jobs_queue.rs` or a specialized `crates/ls-infra/src/infra/app_jobs_*.rs`.
- Periodic dispatch policy: `crates/ls-infra/src/scheduler/*.rs` or `crates/ls-infra/src/scheduler/mod.rs`.
- API trigger endpoints: `crates/ls-api/src/api/routes_admin_libraries_tasks.rs` or the nearest admin/user route module.

**New Frontend Screen:**
- User/admin route wrapper: add a new `.astro` file under `web/src/pages/app/` or `web/src/pages/admin/`.
- Interactive screen logic: add a new island in `web/src/islands/` and mount it from the page file.
- Data fetching: add endpoint-specific helpers in `web/src/lib/api/*.ts`.
- Shared frontend types: add them in `web/src/lib/types/*.ts`.
- Tests: co-locate `*.test.tsx` beside the new island/component and `*.test.ts` beside new helpers.

**New Component/Module:**
- Implementation: `web/src/components/ui/` for primitives, `web/src/components/domain/` for media/business widgets, `web/src/components/navigation/` for nav shells, `web/src/components/effects/` for visual effects.

**Utilities:**
- Backend shared helpers: `crates/ls-infra/src/infra/utils_*.rs`, `crates/ls-api/src/api/helpers_*.rs`, or `crates/ls-infra/src/search.rs` / `crates/ls-infra/src/scanner.rs` when the helper is tightly tied to that subsystem.
- Frontend shared helpers: `web/src/lib/`.

## Special Directories

**`.planning/codebase/`:**
- Purpose: generated architecture/reference docs used by GSD planning commands.
- Generated: Yes.
- Committed: Yes.

**`target/`:**
- Purpose: Rust build output.
- Generated: Yes.
- Committed: No.

**`web/.astro/`:**
- Purpose: Astro build/cache metadata.
- Generated: Yes.
- Committed: No.

**`web/node_modules/`:**
- Purpose: frontend dependency install tree.
- Generated: Yes.
- Committed: No.

**`migrations/`:**
- Purpose: canonical schema history consumed at startup by `crates/ls-infra/src/db.rs`.
- Generated: No.
- Committed: Yes.

---

*Structure analysis: 2026-03-20*
