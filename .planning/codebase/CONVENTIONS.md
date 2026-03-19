# Coding Conventions

**Analysis Date:** 2026-03-20

## Naming Patterns

**Files:**
- Use `snake_case` for Rust source files and modules, especially route and scheduler units such as `crates/ls-api/src/api/routes_admin_libraries_tasks.rs`, `crates/ls-api/src/api/helpers_request_ip.rs`, and `crates/ls-infra/src/scheduler/job_retry.rs`.
- Use `PascalCase.tsx` for React component files such as `web/src/components/domain/Modal.tsx`, `web/src/islands/admin/AdminJobsPanel.tsx`, and `web/src/islands/media/ItemDetail.tsx`.
- Use `kebab-case.ts` for frontend utility and hook files such as `web/src/lib/media/episode-label.ts`, `web/src/lib/player/deeplink.ts`, and `web/src/lib/hooks/use-image-glow.ts`.
- Name test files next to the source they verify using `*.test.ts` or `*.test.tsx`, for example `web/src/lib/api/items.test.ts` and `web/src/components/domain/Modal.test.tsx`.

**Functions:**
- Use `snake_case` for Rust functions, helpers, and route builders, for example `build_api_router` in `crates/ls-api/src/api/router.rs`, `normalize_scan_library_path` in `crates/ls-config/src/lib.rs`, and `find_due_time` in `crates/ls-infra/src/scheduler/mod.rs`.
- Use `camelCase` for frontend functions and handlers, for example `listMyPlaylists` in `web/src/lib/api/playlists.ts`, `getApiBaseUrl` in `web/src/lib/api/client.ts`, and `onSubmit` in `web/src/islands/auth/LoginForm.tsx`.
- Prefix React hooks with `use`, as shown by `useImageGlow` in `web/src/lib/hooks/use-image-glow.ts`.

**Variables:**
- Use `snake_case` for Rust locals and parameters such as `window_start`, `scheduled_for`, and `bind_addr` in `crates/ls-infra/src/scheduler/mod.rs` and `crates/ls-app/src/main.rs`.
- Use `camelCase` for frontend locals and state such as `mockFeatureEnabled`, `rememberMe`, and `portalTarget` in `web/src/islands/auth/LoginForm.tsx` and `web/src/components/domain/Modal.tsx`.
- Reserve all-caps for constants and fixtures, for example `MIGRATOR` in `crates/ls-infra/src/db.rs` and `DEMO_USER` in `web/src/lib/auth/token.test.ts`.

**Types:**
- Use `PascalCase` for Rust structs/enums like `LogError`, `MoviePilotClient`, and `SearchKeys` in `crates/ls-logging/src/lib.rs`, `crates/ls-agent/src/moviepilot.rs`, and `crates/ls-infra/src/search.rs`.
- Use `PascalCase` for TypeScript interfaces and types like `ModalProps`, `Playlist`, and `AdminTaskRun` in `web/src/components/domain/Modal.tsx`, `web/src/lib/types/playlist.ts`, and `web/src/lib/types/admin.ts`.

## Code Style

**Formatting:**
- Format Rust with `cargo fmt --all`, wired through `just fmt` and `just fmt-check` in `justfile`.
- No repository-level `rustfmt.toml`, `clippy.toml`, or `.editorconfig` is detected at `/Volumes/AppleSoft/media/lumen/lumenstream-ce`.
- Format frontend code with Prettier from `web/.prettierrc`.
- Keep frontend formatting aligned with `web/.prettierrc`: semicolons enabled, double quotes, `tabWidth: 2`, `printWidth: 100`, `trailingComma: "es5"`, plus `prettier-plugin-astro` and `prettier-plugin-tailwindcss`.

**Linting:**
- Run Rust linting through `just lint`, which executes `cargo clippy --workspace --all-targets --all-features -- -D warnings` from `justfile`.
- Run frontend linting through `just web-lint`, which executes `bun run lint` inside `web/`.
- Follow the ESLint rules in `web/eslint.config.js`: React and React Hooks recommended rules are enabled, `react/react-in-jsx-scope` and `react/prop-types` are disabled, and unused args/vars may be intentionally prefixed with `_`.

## Import Organization

**Order:**
1. Rust files group standard library imports first, then third-party crates, then workspace crates, then `crate::` imports. `crates/ls-agent/src/moviepilot.rs` and `crates/ls-config/src/lib.rs` are representative.
2. Frontend files import platform/framework modules first, alias imports from `@/` second, and local relative imports last. `web/src/islands/auth/LoginForm.tsx` and `web/src/lib/api/items.test.ts` follow this consistently.
3. Type-only imports are split with `type`, especially in frontend files such as `web/src/lib/api/playlists.ts` and `web/src/components/domain/Modal.tsx`.

**Path Aliases:**
- Use the `@` alias for `web/src`, configured in `web/vitest.config.ts`.
- Prefer direct module imports like `@/lib/api/client` and `@/components/ui/button` instead of frontend barrel files.

## Error Handling

**Patterns:**
- Wrap fallible Rust operations with `anyhow::Context` or `with_context`, as shown in `crates/ls-app/src/main.rs`, `crates/ls-infra/src/db.rs`, `crates/ls-scraper/src/nfo.rs`, and `crates/ls-agent/src/moviepilot.rs`.
- Model reusable Rust error domains with `thiserror::Error`, as in `crates/ls-logging/src/lib.rs` and `crates/ls-config/src/lib.rs`.
- Prefer early returns for validation and branch exits, for example `validate_bootstrap_credentials` in `crates/ls-config/src/lib.rs` and `dispatch_due_tasks` in `crates/ls-infra/src/scheduler/mod.rs`.
- Convert backend failures to HTTP responses close to the handler boundary, as in `crates/ls-api/src/api/routes_admin_sessions_system.rs` and the mapper helpers included from `crates/ls-api/src/api/error_map.rs`.
- In frontend event handlers, catch API failures and surface user-facing toasts instead of silent failure. `web/src/islands/auth/LoginForm.tsx` is the clearest pattern.

## Logging

**Framework:** `tracing` on the Rust side, toast/notification stores on the frontend.

**Patterns:**
- Use structured Rust logs with fields instead of interpolated strings, for example `tracing::info!(address = %bind_addr, ...)` in `crates/ls-app/src/main.rs` and `warn!(error = %err, run_id = %run_id, ...)` in `crates/ls-infra/src/scheduler/mod.rs`.
- Initialize backend logging centrally through `init_logging` in `crates/ls-logging/src/lib.rs`.
- Use frontend toast notifications for UX-visible failures and success states, for example `toast.error(...)` in `web/src/islands/auth/LoginForm.tsx` and mocked `toast.success(...)` flows in `web/src/components/domain/AddToPlaylistModal.test.tsx`.
- Avoid `console.*`; no repository-wide frontend console logging pattern is established in sampled code under `web/src/`.

## Comments

**When to Comment:**
- Use Rust module docs (`//!`) or short targeted comments only when structure or lifecycle is non-obvious, as in `crates/ls-logging/src/lib.rs`, `crates/ls-infra/src/scheduler/mod.rs`, and `crates/ls-api/src/lib.rs`.
- Use inline comments to explain exceptions or framework constraints, not routine assignments. `web/eslint.config.js` includes this style for the Three.js ESLint exception.
- Keep most business logic self-descriptive through names and types instead of prose.

**JSDoc/TSDoc:**
- Not a common frontend pattern. `web/src/` relies on TypeScript signatures, props interfaces, and descriptive names instead of JSDoc blocks.
- Rust public APIs use doc comments selectively, not exhaustively. `crates/ls-logging/src/lib.rs` is representative of when docs are added.

## Function Design

**Size:** Prefer small normalization, parsing, and mapping helpers even inside large files. Good examples include `crates/ls-infra/src/search.rs`, `crates/ls-config/src/lib.rs`, and `web/src/lib/media/episode-label.ts`.

**Parameters:** 
- Use typed DTO structs/interfaces for complex payloads rather than loose maps, for example `MoviePilotExactSearchQuery` in `crates/ls-agent/src/moviepilot.rs`, `ModalProps` in `web/src/components/domain/Modal.tsx`, and request/response types in `web/src/lib/types/admin.ts`.
- Pass explicit primitives for narrow helpers. `buildPersonImageUrl(personId, token)` in `web/src/lib/api/items.ts` and `find_due_time(schedule, window_start, now)` in `crates/ls-infra/src/scheduler/mod.rs` are representative.

**Return Values:** 
- Return `anyhow::Result<T>` or domain-specific `Result<T, E>` for backend fallible work.
- Return typed `Promise<T>` for frontend API wrappers, as in `web/src/lib/api/playlists.ts`.
- Use `Option<T>` in Rust and nullable browser values in TypeScript only for genuinely optional data paths.

## Module Design

**Exports:** 
- Rust crate roots act as the main export surface. `crates/ls-scraper/src/lib.rs` uses `pub mod` plus `pub use`, while `crates/ls-api/src/lib.rs` and `crates/ls-infra/src/lib.rs` aggregate many focused files with `include!`.
- Frontend modules export concrete functions and components directly from the file that owns the behavior, for example `web/src/lib/api/playlists.ts` and `web/src/components/domain/Modal.tsx`.

**Barrel Files:** 
- Rust uses crate root barrel behavior heavily through `pub use` and `include!` in `crates/ls-scraper/src/lib.rs`, `crates/ls-api/src/lib.rs`, and `crates/ls-infra/src/lib.rs`.
- Broad frontend barrel files are not a repository norm. Import from the concrete module path under `web/src/lib/`, `web/src/components/`, or `web/src/islands/`.

---

*Convention analysis: 2026-03-20*
