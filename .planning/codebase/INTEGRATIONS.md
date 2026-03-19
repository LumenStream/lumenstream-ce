# External Integrations

**Analysis Date:** 2026-03-20

## APIs & External Services

**Metadata providers:**
- TMDB - movie/series metadata lookup, enrichment, rate limiting, retries, and cache-backed scraping in `crates/ls-infra/src/infra/app_jobs_scan_tmdb_core.rs`, `crates/ls-infra/src/infra/app_jobs_tmdb_fill.rs`, and `crates/ls-infra/src/infra/prelude.rs`.
  - SDK/Client: raw `reqwest` HTTP client from `crates/ls-infra/src/infra/prelude.rs`.
  - Auth: `LS_TMDB_API_KEY` from `crates/ls-config/src/lib.rs`.
- TVDB - search/login/detail scraping against `https://api4.thetvdb.com/v4` in `crates/ls-scraper/src/tvdb.rs`.
  - SDK/Client: raw `reqwest` HTTP client from `crates/ls-scraper/src/tvdb.rs`.
  - Auth: `LS_TVDB_API_KEY`, `LS_TVDB_PIN`, optional `LS_TVDB_BASE_URL`, `LS_TVDB_TIMEOUT_SECONDS` from `crates/ls-config/src/lib.rs`.
- Bangumi - anime/series metadata search and subject/episode lookup in `crates/ls-scraper/src/bangumi.rs`.
  - SDK/Client: raw `reqwest` HTTP client from `crates/ls-scraper/src/bangumi.rs`.
  - Auth: `LS_BANGUMI_ACCESS_TOKEN`, optional `LS_BANGUMI_BASE_URL`, `LS_BANGUMI_TIMEOUT_SECONDS`, `LS_BANGUMI_USER_AGENT` from `crates/ls-config/src/lib.rs`.

**Search and discovery:**
- Meilisearch - full-text media/person index initialization and querying in `crates/ls-infra/src/infra/app_core.rs`, `crates/ls-infra/src/infra/app_media_root_search.rs`, and `crates/ls-infra/src/infra/app_media_filters_items.rs`.
  - SDK/Client: `meilisearch-sdk` from `crates/ls-infra/Cargo.toml`.
  - Auth: `MEILI_MASTER_KEY` read by `crates/ls-infra/src/infra/prelude.rs`.

**Automation providers:**
- MoviePilot - agent-side authentication, TMDB-keyed search, and subscription/download workflow support in `crates/ls-agent/src/moviepilot.rs`.
  - SDK/Client: raw `reqwest` HTTP client from `crates/ls-agent/src/moviepilot.rs`.
  - Auth: `LS_AGENT_MOVIEPILOT_BASE_URL`, `LS_AGENT_MOVIEPILOT_USERNAME`, `LS_AGENT_MOVIEPILOT_PASSWORD`, plus `LS_AGENT_MOVIEPILOT_ENABLED` in `crates/ls-config/src/lib.rs`.
- LLM provider - function-calling style request parsing and agent planning via `POST {base_url}/chat/completions` in `crates/ls-agent/src/llm.rs`.
  - SDK/Client: raw `reqwest` HTTP client from `crates/ls-agent/src/llm.rs`.
  - Auth: `LS_AGENT_ENABLED` gates the feature set; `AgentLlmConfig` in `crates/ls-config/src/lib.rs` stores `base_url`, `api_key`, and `model`. `default_agent_llm_base_url()` points to `https://api.openai.com/v1`, so the implementation is OpenAI-compatible by default.

**Payments and billing:**
- ePay-compatible gateway - recharge order checkout URL generation and callback verification in `crates/ls-infra/src/billing.rs`.
  - SDK/Client: no SDK; signed form/query submission URLs are assembled directly in `crates/ls-infra/src/billing.rs`.
  - Auth: `LS_BILLING_EPAY_GATEWAY_URL`, `LS_BILLING_EPAY_PID`, `LS_BILLING_EPAY_KEY`, `LS_BILLING_EPAY_NOTIFY_URL`, `LS_BILLING_EPAY_RETURN_URL`, `LS_BILLING_EPAY_SITENAME` from `crates/ls-config/src/lib.rs`.

**Streaming and media routing:**
- LumenBackend nodes - distributed playback target generation and signed stream token creation in `crates/ls-infra/src/infra/app_media_playback_stream.rs`, `crates/ls-infra/src/infra/app_playlists_redirect.rs`, and `crates/ls-infra/src/infra/utils_tasks_lumenbackend.rs`.
  - SDK/Client: no SDK; HTTP URLs are built directly from configured node bases.
  - Auth: `LS_LUMENBACKEND_ENABLED`, `LS_LUMENBACKEND_NODES`, `LS_LUMENBACKEND_ROUTE`, `LS_LOCAL_STREAM_ROUTE`, `LS_LUMENBACKEND_STREAM_SIGNING_KEY`, `LS_LUMENBACKEND_STREAM_TOKEN_TTL_SECONDS` from `crates/ls-config/src/lib.rs`.
- Segment gateway - optional head/tail segment URL generation for playback in `crates/ls-infra/src/infra/app_media_playback_stream.rs`.
  - SDK/Client: no SDK; path URLs are assembled against `storage.segment_gateway_base_url`.
  - Auth: no dedicated auth env detected; base URL is stored in runtime web settings handled by `crates/ls-config/src/lib.rs` and `crates/ls-api/src/api/routes_admin_sessions_system.rs`.

## Data Storage

**Databases:**
- PostgreSQL
  - Connection: `LS_DATABASE_URL` and `LS_DATABASE_MAX_CONNECTIONS` in `crates/ls-config/src/lib.rs`.
  - Client: SQLx `PgPool` with embedded migrations in `crates/ls-infra/src/db.rs`.
  - Schema source: `migrations/`.

**Search index:**
- Meilisearch
  - Connection: hard-coded `http://127.0.0.1:7700` plus optional `MEILI_MASTER_KEY` in `crates/ls-infra/src/infra/prelude.rs`.
  - Client: `meilisearch-sdk` in `crates/ls-infra/src/infra/app_core.rs`.

**File Storage:**
- Local filesystem and mounted media roots are first-class inputs for scanning and playback in `crates/ls-config/src/lib.rs`, `crates/ls-infra/src/infra/app_media_playback_stream.rs`, and `docker-compose.fullstack.localbackend.yml`.
- Google Drive style links are supported through `gdrive://` URL resolution in `crates/ls-infra/src/infra/app_media_playback_stream.rs`.
  - If `storage.gdrive_accounts` is configured, the service rewrites to account-specific `/drive/{file_id}` HTTP bases.
  - Otherwise it falls back to `https://drive.google.com/uc?id=...&export=download`.
- S3 style links are supported through `s3://bucket/key` URL resolution in `crates/ls-infra/src/infra/app_media_playback_stream.rs`.
  - The endpoint is loaded from the `storage_configs` table, not from an AWS SDK config.
- Web assets are served statically by Nginx from `/usr/share/nginx/html` in `Dockerfile.fullstack` and `docker/Dockerfile.web`.

**Caching:**
- Local disk cache under `./cache` style paths from `crates/ls-config/src/lib.rs`.
  - `default_mediainfo_cache_dir()` -> `./cache/mediainfo`
  - `default_s3_cache_dir()` -> `./cache/segments`
  - `default_tmdb_person_image_cache_dir()` -> `./cache`
- Docker Compose mounts `/app/cache` as `ls_cache` in `docker-compose.fullstack.yml` and `docker-compose.fullstack.localbackend.yml`.
- TMDB response caching and cleanup are handled in backend infra and scheduler code in `crates/ls-infra/src/infra/app_jobs_scan_tmdb_core.rs` and `crates/ls-infra/src/scheduler/cleanup.rs`.

## Authentication & Identity

**Auth Provider:**
- Custom
  - Implementation: bootstrap admin credentials and runtime auth config live in `crates/ls-config/src/lib.rs`; password hashing/signing helpers come from `argon2`, `hmac`, and `sha2` in `Cargo.toml` and `crates/ls-infra/src/infra/prelude.rs`.
  - API/session routes are implemented in `crates/ls-api/src/api/routes_system_self.rs`, `crates/ls-api/src/api/routes_auth_invite.rs`, and `crates/ls-api/src/api/helpers_auth.rs`.
  - Frontend stores bearer tokens in session storage according to `web/README.md` and uses `web/src/lib/auth/token` through `web/src/lib/api/client.ts`.

## Monitoring & Observability

**Error Tracking:**
- No external SaaS error tracker detected.

**Logs:**
- Structured application logging via `tracing` with stdout/file/both outputs in `crates/ls-logging/src/lib.rs`.
- Audit logs are stored in PostgreSQL through `crates/ls-infra/src/infra/app_sessions_audit.rs`.

**Metrics:**
- Internal metrics endpoint at `/metrics` in `crates/ls-api/src/api/router.rs` and `crates/ls-api/src/api/routes_system_self.rs`.
- API and infra metric snapshots include latency, status counts, playback success, TMDB/scraper stats, and cleanup stats in `crates/ls-api/src/api/prelude.rs` and `crates/ls-infra/src/infra/app_core.rs`.

## CI/CD & Deployment

**Hosting:**
- Container-first deployment. The primary published artifact is `ghcr.io/lumenstream/lumenstream-ce-fullstack` referenced by `docker-compose.fullstack.yml`, `docker-compose.fullstack.localbackend.yml`, and `README.md`.
- Fullstack runtime combines backend + Nginx in `Dockerfile.fullstack`.
- Separate backend-only image exists in `Dockerfile`; separate web-only image exists in `docker/Dockerfile.web`.

**CI Pipeline:**
- GitHub Actions workflow in `.github/workflows/ci-ghcr.yml`.
  - Validate job runs Rust format/tests, shell syntax checks, frontend type checks/tests/lint/format.
  - Publish job builds `ls-app`, builds `Dockerfile.fullstack`, and pushes tags to GHCR.

## Environment Configuration

**Required env vars:**
- Core backend: `LS_DATABASE_URL`, `LS_DATABASE_MAX_CONNECTIONS`, `LS_BOOTSTRAP_ADMIN_USER`, `LS_BOOTSTRAP_ADMIN_PASSWORD` from `crates/ls-config/src/lib.rs` and `config.example.yaml`.
- Search: `MEILI_MASTER_KEY` from `crates/ls-infra/src/infra/prelude.rs` and both compose files.
- Frontend/API wiring: `LS_API_BASE_URL` for runtime injection and `PUBLIC_LS_API_BASE_URL` for build-time frontend config from `docker/write-web-runtime-config.sh`, `Dockerfile.fullstack`, and `docker/Dockerfile.web`.
- Optional external providers: `LS_TMDB_API_KEY`, `LS_TVDB_*`, `LS_BANGUMI_*`, `LS_AGENT_MOVIEPILOT_*`, `LS_BILLING_EPAY_*`, and `LS_LUMENBACKEND_*` from `crates/ls-config/src/lib.rs`.

**Secrets location:**
- Local deployments are expected to provide secrets via environment variables and `.env` files referenced by `README.md` and the compose files.
- CI registry publishing uses `${{ secrets.GITHUB_TOKEN }}` in `.github/workflows/ci-ghcr.yml`.
- No dedicated secret manager integration is detected in repository code.

## Webhooks & Callbacks

**Incoming:**
- `POST /billing/epay/notify` handled in `crates/ls-api/src/api/routes_billing_user.rs`.
- `GET /billing/epay/return` handled in `crates/ls-api/src/api/routes_billing_admin_ops.rs`.
- Billing recharge order status also has an internal WebSocket callback channel at `/billing/recharge/orders/{order_id}/ws` in `crates/ls-api/src/api/router_commercial.rs` and `crates/ls-api/src/api/routes_billing_user.rs`.

**Outgoing:**
- ePay checkout redirect URLs are generated against the configured gateway in `crates/ls-infra/src/billing.rs`.
- Outbound HTTP requests are issued to TMDB, TVDB, Bangumi, MoviePilot, the LLM endpoint, and Meilisearch from `crates/ls-infra/src/infra/app_jobs_scan_tmdb_core.rs`, `crates/ls-scraper/src/tvdb.rs`, `crates/ls-scraper/src/bangumi.rs`, `crates/ls-agent/src/moviepilot.rs`, `crates/ls-agent/src/llm.rs`, and `crates/ls-infra/src/infra/app_core.rs`.

---

*Integration audit: 2026-03-20*
