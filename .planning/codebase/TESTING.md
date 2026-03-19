# Testing Patterns

**Analysis Date:** 2026-03-20

## Test Framework

**Runner:**
- Rust tests run through `cargo test --workspace`, surfaced as `just test` and `just test-all` in `justfile`.
- Frontend tests run through Vitest 4 via `bun run test` in `web/package.json`.
- Frontend config lives in `web/vitest.config.ts`. Default environment is `node`, and browser-oriented tests opt into JSDOM with a per-file `/** @vitest-environment jsdom */` banner, as seen in `web/src/islands/media/ItemDetail.test.tsx` and `web/src/components/domain/Modal.test.tsx`.

**Assertion Library:**
- Rust uses the standard assertion macros `assert_eq!`, `assert!`, and async test helpers from Actix/Tokio.
- Frontend uses Vitest `expect(...)` plus mock assertions like `toHaveBeenCalledWith` and async assertions like `rejects.toThrow`.

**Run Commands:**
```bash
just test                 # Run Rust workspace tests
just test-all             # Run Rust tests for all targets
just test-verbose         # Run Rust tests with --nocapture
cd web && bun run test    # Run frontend tests once
cd web && bun run test:watch  # Watch frontend tests
```

## Test File Organization

**Location:**
- Rust tests are mostly inline `#[cfg(test)]` modules inside production files such as `crates/ls-config/src/lib.rs`, `crates/ls-infra/src/search.rs`, and `crates/ls-logging/src/lib.rs`.
- Large backend subsystems also maintain aggregate inline suites in dedicated source files included by the crate root, especially `crates/ls-api/src/api/tests.rs` and `crates/ls-infra/src/infra/tests.rs`.
- Frontend tests are co-located beside the source file they verify under `web/src/`, for example `web/src/lib/api/items.test.ts` next to `web/src/lib/api/items.ts` and `web/src/components/domain/Modal.test.tsx` next to `web/src/components/domain/Modal.tsx`.
- No repository-level Rust `tests/` directory or frontend `__tests__` directory is detected under `/Volumes/AppleSoft/media/lumen/lumenstream-ce`.

**Naming:**
- Rust defaults to `mod tests` or a narrow module name such as `middleware_compat_case_tests` in `crates/ls-api/src/api/middleware.rs`.
- Frontend uses `*.test.ts` and `*.test.tsx` consistently across `web/src/lib/`, `web/src/components/`, and `web/src/islands/`.

**Structure:**
```text
crates/<crate>/src/**/*.rs          # Production code with inline #[cfg(test)] modules
crates/ls-api/src/api/tests.rs      # Large API helper/compat regression suite
crates/ls-infra/src/infra/tests.rs  # Large infra helper/runtime suite
web/src/**/<name>.test.ts           # Utility/API tests
web/src/**/<name>.test.tsx          # React component/island tests
```

## Test Structure

**Suite Organization:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_include_tmdb_retry_and_cache_settings() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.tmdb.retry_attempts, 3);
    }
}
```

```typescript
/**
 * @vitest-environment jsdom
 */

describe("AdminJobsPanel", () => {
  beforeEach(() => {
    // createRoot container + default mocks
  });

  it("shows inline JSON errors and blocks save/run when override payload is invalid", async () => {
    // render, mutate DOM, flush effects, assert disabled actions
  });
});
```

**Patterns:**
- Prefer small helper functions inside the test file for setup and DOM mutation, such as `make_test_item` in `crates/ls-api/src/api/tests.rs`, `flushEffects` in `web/src/islands/media/ItemDetail.test.tsx`, and `setTextareaValue` in `web/src/islands/admin/AdminJobsPanel.test.tsx`.
- Reset shared state in teardown. Frontend tests commonly use `vi.clearAllMocks()`, storage cleanup, and `root.unmount()` in `afterEach`, for example `web/src/islands/auth/LoginForm.test.tsx` and `web/src/lib/mock/mode.test.ts`.
- Keep assertions close to the behavior being exercised instead of snapshot-heavy testing. No snapshot test usage is detected in sampled files.

## Mocking

**Framework:** 
- Rust: no dedicated mocking framework is detected.
- Frontend: Vitest mocks with `vi.mock`, `vi.fn`, `vi.mocked`, and `vi.spyOn`.

**Patterns:**
```rust
#[actix_web::test]
async fn compat_prefix_is_case_insensitive() {
    let app = test::init_service(
        App::new()
            .wrap(actix_web::middleware::from_fn(strip_compat_prefix))
            .route("/Videos/{item_id}/stream.{container}", web::get().to(ok)),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/Emby/videos/abc/stream.mkv?api_key=token")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}
```

```typescript
vi.mock("@/lib/api/admin", () => ({
  cancelTaskRun: vi.fn(),
  getTaskRunsWebSocketUrl: vi.fn(),
  listTaskDefinitions: vi.fn(),
  listTaskRuns: vi.fn(),
  patchTaskDefinition: vi.fn(),
  runTaskNow: vi.fn(),
}));

const mockListTaskRuns = vi.mocked(listTaskRuns);
mockListTaskRuns.mockResolvedValue([]);
```

**What to Mock:**
- Mock frontend API clients, auth/session hooks, toast stores, and WebSocket URL builders at module boundaries. `web/src/islands/admin/AdminJobsPanel.test.tsx`, `web/src/islands/media/ItemDetail.test.tsx`, and `web/src/components/domain/AddToPlaylistModal.test.tsx` are the clearest references.
- Mock browser globals when needed, for example `window.__LS_CONFIG__` in `web/src/lib/api/client.test.ts`, `sessionStorage`/`localStorage` in `web/src/lib/auth/token.test.ts`, and `globalThis.Image` in `web/src/lib/image/color-extractor.test.ts`.
- For Rust HTTP middleware, use `actix_web::test::init_service` rather than hand-rolled stubs, as in `crates/ls-api/src/api/middleware.rs`.

**What NOT to Mock:**
- Do not mock pure Rust normalization/parsing helpers; test them directly with real inputs in `crates/ls-infra/src/search.rs`, `crates/ls-logging/src/lib.rs`, and `crates/ls-config/src/lib.rs`.
- Do not mock simple frontend pure helpers such as URL builders and label formatters unless a boundary dependency exists. `web/src/lib/media/episode-label.test.ts` and `web/src/lib/media/item-href.test.ts` call the real functions directly.
- Avoid introducing snapshot abstractions when explicit DOM or value assertions already express the contract.

## Fixtures and Factories

**Test Data:**
```rust
let current = json!({
    "overview": "old",
    "provider_ids": { "Tmdb": "1" }
});

let result = ScrapeResult {
    provider_id: "tvdb".to_string(),
    // ...
    raw: json!({ "id": 100 }),
    complete: true,
};
```

```typescript
const testItem: BaseItem = {
  Id: "item-1",
  Name: "Test Movie",
  Type: "Movie",
  Path: "/media/test/movie.mkv",
};
```

**Location:**
- Keep fixtures local to the test file. Examples include `make_test_item` in `crates/ls-api/src/api/tests.rs`, `DEMO_USER` in `web/src/lib/auth/token.test.ts`, and `testPlaylists` in `web/src/components/domain/AddToPlaylistModal.test.tsx`.
- Build temporary filesystem fixtures inline for Rust file-oriented tests using `std::env::temp_dir()`, `std::fs::write`, and cleanup calls, as seen in `crates/ls-config/src/lib.rs`, `crates/ls-infra/src/infra/tests.rs`, and `crates/ls-infra/src/scanner.rs`.
- Use local environment guards when touching process env. `crates/ls-config/src/lib.rs` defines `EnvVarGuard` plus an `env_lock()` mutex to serialize env-sensitive tests.

## Coverage

**Requirements:** None enforced. `prek.toml` requires passing `just test`, `just web-test`, and related checks, but no coverage threshold or coverage hook is configured.

**View Coverage:**
```bash
# Not detected in `justfile`, `web/package.json`, or `prek.toml`
```

## Test Types

**Unit Tests:**
- Rust unit tests focus heavily on pure helpers, config defaults, parsers, and mappers, for example `crates/ls-config/src/lib.rs`, `crates/ls-infra/src/search.rs`, `crates/ls-logging/src/lib.rs`, and `crates/ls-agent/src/workflow.rs`.
- Frontend unit tests cover utility modules and typed API wrappers in `web/src/lib/`, such as `web/src/lib/api/items.test.ts`, `web/src/lib/player/deeplink.test.ts`, and `web/src/lib/mock/mode.test.ts`.

**Integration Tests:**
- Rust integration-style tests are still inline but exercise framework behavior, especially Actix middleware in `crates/ls-api/src/api/middleware.rs` and async infra flows in `crates/ls-infra/src/infra/tests.rs`.
- Frontend component tests mount real React trees with `createRoot`, drive DOM events manually, and assert visible behavior, as in `web/src/components/domain/Modal.test.tsx`, `web/src/islands/admin/AdminJobsPanel.test.tsx`, and `web/src/islands/media/LibraryBrowser.test.tsx`.

**E2E Tests:**
- Automated E2E coverage is not detected. No Playwright or Cypress config is present.
- Manual frontend verification is still part of repository process via `AGENTS.md`, which requires running `bun run dev` under `web/` and checking behavior with Chrome DevTools MCP.

## Common Patterns

**Async Testing:**
```rust
#[tokio::test]
async fn some_async_helper_test() {
    let result = async_fn().await;
    assert!(result.is_ok());
}
```

```typescript
await act(async () => {
  root.render(<ItemDetail itemId="movie-1" />);
  await Promise.resolve();
  await Promise.resolve();
});
```

- Use `#[tokio::test]` for async backend helpers in `crates/ls-infra/src/infra/tests.rs` and `crates/ls-api/src/api/tests.rs`.
- Use `#[actix_web::test]` for middleware/router behavior in `crates/ls-api/src/api/middleware.rs`.
- In frontend component tests, flush pending promises manually with helper functions like `flushEffects()` instead of adding extra libraries.

**Error Testing:**
```rust
#[test]
fn test_parse_filter_invalid() {
    assert!(parse_filter("not_a_level[invalid").is_err());
}
```

```typescript
await expect(runWithMock(mockFn, realFn)).rejects.toThrow("network down");
expect(mockPatchTaskDefinition).not.toHaveBeenCalled();
```

- Assert invalid inputs explicitly and check the failing branch, rather than only the happy path. Examples include `crates/ls-logging/src/lib.rs`, `crates/ls-config/src/lib.rs`, `web/src/lib/mock/mode.test.ts`, and `web/src/islands/admin/AdminSettingsPanel.test.tsx`.

---

*Testing analysis: 2026-03-20*
