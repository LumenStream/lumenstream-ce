# Codebase Concerns

**Analysis Date:** 2026-03-20

## Tech Debt

**Monolithic API and infra composition:**
- Issue: request routing and infrastructure behavior are split across very large files and then flattened back together with `include!`, which keeps compile-time coupling high and makes targeted changes risky.
- Files: `crates/ls-api/src/api/router.rs` (945 lines), `crates/ls-infra/src/lib.rs:8`, `crates/ls-infra/src/infra/app_agent_requests.rs` (4416 lines), `crates/ls-infra/src/scanner.rs` (2950 lines)
- Impact: small changes in routing, agent logic, or scanning are harder to review, isolate, and regression-test; merge conflicts and accidental cross-feature regressions are more likely.
- Fix approach: replace `include!` flattening with real modules, break `router.rs` into scoped builders, and split `app_agent_requests.rs` / `scanner.rs` by workflow boundary instead of by helper accumulation.

**Oversized shared test modules:**
- Issue: backend tests are concentrated into a few giant files instead of feature-local test modules.
- Files: `crates/ls-api/src/api/tests.rs` (5509 lines), `crates/ls-infra/src/infra/tests.rs` (3498 lines)
- Impact: failures are harder to localize, setup reuse is opaque, and new contributors will tend to append more cases to already-unwieldy files.
- Fix approach: move tests next to the route/helper/job module they cover and reserve shared fixtures for common setup only.

**Agent workflow mixes policy, transport, persistence, and fallback heuristics in one place:**
- Issue: request creation, runtime state persistence, MoviePilot search, LLM loop decisions, and manual fallback ranking all live in `app_agent_requests.rs`.
- Files: `crates/ls-infra/src/infra/app_agent_requests.rs:215`, `crates/ls-infra/src/infra/app_agent_requests.rs:991`, `crates/ls-infra/src/infra/app_agent_requests.rs:1926`, `crates/ls-infra/src/infra/app_agent_requests.rs:3995`
- Impact: changing one provider or one state transition can unintentionally change user-visible request lifecycle behavior.
- Fix approach: split agent persistence, provider orchestration, loop policy, and user/public projections into separate modules with fixture-driven tests around each boundary.

## Known Bugs

**Several compatibility endpoints return success without performing the advertised action:**
- Symptoms: clients receive `204` or placeholder payloads even though no real restart, shutdown, config write, log listing, or activity-log retrieval happened.
- Files: `crates/ls-api/src/api/routes_system_self.rs:537`, `crates/ls-api/src/api/routes_system_self.rs:558`, `crates/ls-api/src/api/routes_system_self.rs:578`, `crates/ls-api/src/api/routes_system_self.rs:626`, `crates/ls-api/src/api/routes_system_self.rs:664`
- Trigger: Jellyfin/Emby-compatible admin clients call `/System/ActivityLog/Entries`, `/System/Configuration`, `/System/Logs`, `/System/Restart`, or `/System/Shutdown`.
- Workaround: use the real admin endpoints under `/admin/...`; do not rely on these compat endpoints for operational actions.

**User configuration update is acknowledged but not persisted:**
- Symptoms: `/Users/{user_id}/Configuration` returns success and then settings revert on next read/session.
- Files: `crates/ls-api/src/api/routes_users_jellyfin.rs:238`
- Trigger: any client posting a user configuration payload through the Jellyfin-compatible endpoint.
- Workaround: treat the endpoint as compatibility-only until persistence is implemented.

**Playlist reordering is accepted but ignored:**
- Symptoms: move/reorder requests succeed with `204`, but playlist order does not change.
- Files: `crates/ls-api/src/api/routes_compat_playlists_collections.rs:185`
- Trigger: clients calling the Emby/Jellyfin playlist move endpoint.
- Workaround: avoid exposing playlist ordering controls to users who expect stable server-side order.

## Security Considerations

**`/metrics` is unauthenticated and enabled by default:**
- Risk: runtime counters, error rate, cache hit rate, and infra metrics are exposed to any caller when observability is on.
- Files: `crates/ls-api/src/api/router.rs:26`, `crates/ls-api/src/api/routes_system_self.rs:5`, `crates/ls-config/src/lib.rs:468`, `crates/ls-config/src/lib.rs:1756`
- Current mitigation: operators can disable metrics with config.
- Recommendations: require admin or bind `/metrics` behind a separate listener/reverse-proxy policy; default `metrics_enabled` to false for internet-facing deployments.

**Compatibility path rewrite can panic on malformed rewritten URIs:**
- Risk: middleware rebuilds the request URI with `parse().unwrap()`, so an unexpected normalized path or query string can crash the worker instead of returning a controlled error.
- Files: `crates/ls-api/src/api/middleware.rs:226`
- Current mitigation: upstream path normalization tries to keep transformations narrow.
- Recommendations: replace `unwrap()` with fallible parsing and return `400 Bad Request` on invalid rewritten URIs.

## Performance Bottlenecks

**Library scanning is still dominated by full tree walks and repeated filesystem probes:**
- Problem: scans traverse every file with `WalkDir`, then perform per-item `.exists()`, `.read_to_string()`, `read_dir`, and JSON parsing work even for sidecars and images.
- Files: `crates/ls-infra/src/scanner.rs:387`, `crates/ls-infra/src/scanner.rs:464`, `crates/ls-infra/src/scanner.rs:500`, `crates/ls-infra/src/scanner.rs:2110`, `crates/ls-infra/src/scanner.rs:2147`
- Cause: scanning remains file-system driven instead of change-feed driven, and sidecar/image discovery uses many small synchronous checks.
- Improvement path: persist file fingerprints, batch sidecar existence checks per directory, move more logic into incremental indexes, and reduce sync filesystem calls inside hot loops.

**Agent request list endpoints fetch whole rows including large JSON payloads:**
- Problem: admin and user list queries use `SELECT *` and return provider/public/runtime-heavy rows ordered by `created_at`.
- Files: `crates/ls-infra/src/infra/app_agent_requests.rs:215`, `crates/ls-infra/src/infra/app_agent_requests.rs:242`
- Cause: list views share the same row model used by detail views.
- Improvement path: introduce lean list projections that exclude `content`, `provider_payload`, `provider_result`, `public_state`, and future large runtime blobs unless a detail view explicitly asks for them.

## Fragile Areas

**NFO parsing relies on regex over XML-like content and swallows parse failures:**
- Files: `crates/ls-infra/src/scanner.rs:464`, `crates/ls-infra/src/scanner.rs:509`, `crates/ls-infra/src/scanner.rs:1665`, `crates/ls-infra/src/scanner.rs:1703`
- Why fragile: malformed sidecars, nested tags, or edge-case XML formatting degrade silently into empty metadata instead of producing actionable errors.
- Safe modification: keep behavior-compatible fixtures for malformed NFO and mediainfo payloads before swapping parsing strategy.
- Test coverage: scanner tests exist, but they are concentrated in `crates/ls-infra/src/scanner.rs` and do not cover a wide variety of malformed real-world sidecars.

**In-process metrics can panic after mutex poisoning and add contention on hot paths:**
- Files: `crates/ls-api/src/api/prelude.rs:87`
- Why fragile: every latency write and snapshot read locks the same `Mutex<VecDeque<u64>>`, and poison leads to `expect()` panics.
- Safe modification: move to lock-free counters/histograms or tolerate poison with recovery semantics.
- Test coverage: only snapshot math is covered in `crates/ls-api/src/api/tests.rs`; concurrency and poison scenarios are not exercised.

**Agent fallback selection logic is policy-heavy and string-driven:**
- Files: `crates/ls-infra/src/infra/app_agent_requests.rs:3995`, `crates/ls-agent/src/workflow.rs:91`, `crates/ls-agent/src/llm.rs:98`
- Why fragile: behavior depends on prompt text, string enums, and source-name matching; subtle provider payload or prompt changes can reroute requests into manual review.
- Safe modification: lock down prompt/output fixtures and provider normalization rules before changing heuristics.
- Test coverage: `crates/ls-agent/src/workflow.rs` has local tests, but `crates/ls-agent/src/llm.rs` has no direct tests for request parsing or schema handling.

## Scaling Limits

**Only one running job per job kind is allowed across the whole system:**
- Current capacity: one `running` row per `jobs.kind`, regardless of library, user, or host.
- Limit: concurrent `scan_library`, `agent_request_process`, `search_reindex`, or other same-kind jobs serialize globally.
- Scaling path: scope the running-job uniqueness rule by resource (`library_id`, request id, tenant) instead of `kind` alone, or move to queue workers with explicit concurrency controls.
- Files: `crates/ls-infra/src/infra/app_jobs_queue.rs:13`, `crates/ls-infra/src/infra/app_jobs_queue.rs:750`, `crates/ls-infra/src/infra/app_jobs_queue.rs:771`

**Scheduler throughput is bounded by polling and sequential dispatch:**
- Current capacity: the scheduler wakes every 30 seconds and queued dispatch iterates jobs one by one.
- Limit: bursty workloads wait for the next poll and then compete for a single sequential dispatch cycle.
- Scaling path: use row-locking dequeue patterns with immediate notifications, or multiple worker tasks with bounded concurrency.
- Files: `crates/ls-app/src/main.rs:24`, `crates/ls-infra/src/scheduler/mod.rs:47`, `crates/ls-infra/src/scheduler/queued_dispatch.rs:12`, `crates/ls-infra/src/scheduler/queued_dispatch.rs:58`

## Dependencies at Risk

**MoviePilot / LLM upstream API contracts are weakly verified:**
- Risk: provider payloads are deserialized directly from remote responses, and drift in response shape can quietly reduce search results or break loop decisions.
- Impact: request automation falls back to manual review or no-result flows without strong diagnostics.
- Migration plan: add recorded fixtures for `MoviePilotResponse` and LLM tool-call payloads, version response adapters, and avoid binding internal workflow logic directly to raw upstream response shape.
- Files: `crates/ls-agent/src/moviepilot.rs:12`, `crates/ls-agent/src/moviepilot.rs:668`, `crates/ls-agent/src/llm.rs:98`

## Missing Critical Features

**Operational compat endpoints are not backed by real server-management capabilities:**
- Problem: restart, shutdown, log access, and config compatibility routes mainly exist as placeholders.
- Blocks: full Jellyfin admin-client compatibility for server management and diagnostics.
- Files: `crates/ls-api/src/api/routes_system_self.rs:537`, `crates/ls-api/src/api/routes_system_self.rs:626`, `crates/ls-api/src/api/routes_system_self.rs:664`

**Persistent user preference support is incomplete on compat APIs:**
- Problem: user configuration updates and playlist reorder requests are not persisted.
- Blocks: reliable round-trip behavior for Jellyfin/Emby clients that expect server-side preferences and ordering.
- Files: `crates/ls-api/src/api/routes_users_jellyfin.rs:238`, `crates/ls-api/src/api/routes_compat_playlists_collections.rs:185`

## Test Coverage Gaps

**LLM integration behavior is effectively untested at unit/contract level:**
- What's not tested: `LlmProvider` request building, schema enforcement, and failure handling against representative provider payloads.
- Files: `crates/ls-agent/src/llm.rs`
- Risk: prompt/schema edits can break parsing or loop execution without compile-time or fixture-level feedback.
- Priority: High

**Contract replay tooling exists but is not enforced in pre-commit or CI execution:**
- What's not tested: actual API contract and replay callchains; CI only syntax-checks the shell scripts and never runs the contract/replay targets.
- Files: `justfile:47`, `prek.toml:6`, `.github/workflows/ci-ghcr.yml:53`
- Risk: Jellyfin/Emby compatibility regressions can land even when Rust/unit/frontend tests are green.
- Priority: High

**Frontend automated tests do not cover browser hydration or visual regressions:**
- What's not tested: Astro page hydration, browser-only interactions, and visual/UI regressions.
- Files: `web/package.json`, `web/vitest.config.ts`
- Risk: node-environment unit tests can pass while real browser behavior breaks.
- Priority: Medium

---

*Concerns audit: 2026-03-20*
