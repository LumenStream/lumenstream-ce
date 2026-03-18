/**
 * @vitest-environment jsdom
 */

import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { AdminScraperPanel } from "./AdminScraperPanel";

const getSystemFlagsMock = vi.fn();
const getSystemSummaryMock = vi.fn();
const getScraperSettingsMock = vi.fn();
const listScraperProvidersMock = vi.fn();
const getScraperCacheStatsMock = vi.fn();
const listScraperFailuresMock = vi.fn();
const upsertScraperSettingsMock = vi.fn();

vi.mock("@/lib/auth/use-auth-session", () => ({
  useAuthSession: () => ({ ready: true }),
}));

vi.mock("@/lib/api/admin", () => ({
  clearScraperCache: vi.fn(),
  clearScraperFailures: vi.fn(),
  getScraperCacheStats: (...args: unknown[]) => getScraperCacheStatsMock(...args),
  getScraperSettings: (...args: unknown[]) => getScraperSettingsMock(...args),
  getSystemFlags: (...args: unknown[]) => getSystemFlagsMock(...args),
  getSystemSummary: (...args: unknown[]) => getSystemSummaryMock(...args),
  listScraperFailures: (...args: unknown[]) => listScraperFailuresMock(...args),
  listScraperProviders: (...args: unknown[]) => listScraperProvidersMock(...args),
  runTaskNow: vi.fn(),
  testScraperProvider: vi.fn(),
  updateSystemFlags: vi.fn(),
  upsertScraperSettings: (...args: unknown[]) => upsertScraperSettingsMock(...args),
}));

vi.mock("@/components/domain/DataState", () => ({
  ErrorState: ({ title }: { title: string }) => React.createElement("div", null, title),
  LoadingState: ({ title }: { title?: string }) => React.createElement("div", null, title || ""),
}));

vi.mock("@/lib/notifications/toast-store", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

async function flushEffects() {
  await Promise.resolve();
  await Promise.resolve();
}

describe("AdminScraperPanel", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    (
      globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
    ).IS_REACT_ACT_ENVIRONMENT = true;
    container = document.createElement("div");
    document.body.appendChild(container);
    root = createRoot(container);

    getSystemFlagsMock.mockResolvedValue({
      strm_only_streaming: true,
      transcoding_enabled: false,
      scraper_enabled: true,
      tmdb_enabled: true,
      lumenbackend_enabled: false,
      prefer_segment_gateway: false,
      metrics_enabled: true,
    });
    getSystemSummaryMock.mockResolvedValue({
      generated_at_utc: "2026-03-12T00:00:00Z",
      server_id: "mock-server",
      transcoding_enabled: false,
      libraries_total: 2,
      libraries_enabled: 2,
      media_items_total: 128,
      users_total: 6,
      users_disabled: 0,
      active_playback_sessions: 1,
      active_auth_sessions: 2,
      jobs_by_status: {},
      infra_metrics: {
        scraper_http_requests_total: 12,
        scraper_cache_hits_total: 9,
        scraper_cache_misses_total: 3,
        scraper_hit_rate: 0.75,
        scraper_success_total: 10,
        scraper_failure_total: 1,
      },
    });
    getScraperSettingsMock.mockResolvedValue({
      settings: {
        server: { host: "0.0.0.0", port: 8096, base_url: "", cors_allow_origins: [] },
        auth: {
          token_ttl_hours: 24,
          bootstrap_admin_user: "admin",
          bootstrap_admin_password: "***",
          admin_api_key_prefix: "lsadm",
          max_failed_attempts: 5,
          risk_window_seconds: 300,
          risk_block_seconds: 600,
          invite: {
            force_on_register: false,
            invitee_bonus_enabled: false,
            invitee_bonus_amount: "0.00",
            inviter_rebate_enabled: false,
            inviter_rebate_rate: "0.0000",
          },
        },
        scan: {},
        storage: {},
        tmdb: {
          enabled: true,
          api_key: "***",
          language: "zh-CN",
          timeout_seconds: 10,
          request_interval_ms: 350,
          cache_ttl_seconds: 86400,
          retry_attempts: 3,
          retry_backoff_ms: 2000,
        },
        scraper: {
          enabled: true,
          default_strategy: "primary_with_fallback",
          providers: ["tmdb", "tvdb", "bangumi"],
          default_routes: {
            movie: ["tmdb", "tvdb"],
            series: ["tmdb", "tvdb"],
            image: ["tmdb", "tvdb"],
          },
          tvdb: {
            enabled: true,
            base_url: "https://api4.thetvdb.com/v4",
            api_key: "***",
            pin: "***",
            timeout_seconds: 15,
          },
          bangumi: {
            enabled: false,
            base_url: "https://api.bgm.tv",
            access_token: "",
            timeout_seconds: 15,
            user_agent: "lumenstream/0.1",
          },
        },
        security: {
          admin_allow_ips: [],
          trust_x_forwarded_for: false,
          redact_sensitive_logs: true,
        },
        observability: {},
        jobs: {},
        scheduler: {},
        billing: {},
        agent: {
          enabled: false,
          auto_mode: "automatic",
          missing_scan_enabled: false,
          missing_scan_cron: "0 */30 * * * *",
          auto_close_on_library_hit: true,
          review_required_on_parse_ambiguity: true,
          feedback_auto_route: true,
          moviepilot: {
            enabled: false,
            base_url: "",
            username: "",
            password: "",
            timeout_seconds: 20,
            search_download_enabled: true,
            subscribe_fallback_enabled: true,
            filter: {
              min_seeders: 5,
              max_movie_size_gb: 35,
              max_episode_size_gb: 5,
              preferred_resource_pix: [],
              preferred_video_encode: [],
              preferred_resource_type: [],
              preferred_labels: [],
              excluded_keywords: [],
            },
          },
        },
      },
      libraries: [
        {
          id: "lib-001",
          name: "Movies",
          root_path: "/media/movies",
          paths: ["/media/movies"],
          library_type: "Movie",
          enabled: true,
          scraper_policy: {},
          created_at: "2026-03-12T00:00:00Z",
        },
        {
          id: "lib-002",
          name: "Shows",
          root_path: "/media/shows",
          paths: ["/media/shows", "/media/anime"],
          library_type: "Series",
          enabled: true,
          scraper_policy: {
            movie: ["tmdb", "tvdb"],
            series: ["bangumi", "tvdb", "tmdb"],
            image: ["bangumi", "tvdb", "tmdb"],
          },
          created_at: "2026-03-12T00:00:00Z",
        },
      ],
    });
    listScraperProvidersMock.mockResolvedValue([
      {
        provider_id: "tmdb",
        display_name: "TMDB",
        provider_kind: "metadata",
        enabled: true,
        configured: true,
        healthy: true,
        capabilities: ["search", "details", "images", "people", "external_ids"],
        scenarios: ["movie_metadata", "series_metadata"],
        message: "ready",
        checked_at: "2026-03-12T00:00:00Z",
      },
      {
        provider_id: "tvdb",
        display_name: "TVDB",
        provider_kind: "metadata",
        enabled: true,
        configured: true,
        healthy: true,
        capabilities: ["search", "details", "images", "people", "external_ids"],
        scenarios: ["series_metadata", "episode_metadata"],
        message: "ready",
        checked_at: "2026-03-12T00:00:00Z",
      },
      {
        provider_id: "bangumi",
        display_name: "Bangumi",
        provider_kind: "metadata",
        enabled: false,
        configured: false,
        healthy: false,
        capabilities: ["search", "details", "images", "external_ids"],
        scenarios: ["series_metadata", "episode_metadata"],
        message: "ready",
        checked_at: "2026-03-12T00:00:00Z",
      },
    ]);
    getScraperCacheStatsMock.mockResolvedValue({
      total_entries: 12,
      entries_with_result: 10,
      expired_entries: 2,
      total_hits: 24,
    });
    listScraperFailuresMock.mockResolvedValue([]);
    upsertScraperSettingsMock.mockImplementation(
      async (payload: { settings: Record<string, unknown> }) => ({
        settings: payload.settings,
        libraries: [],
      })
    );
  });

  afterEach(() => {
    act(() => {
      root.unmount();
    });
    container.remove();
    (
      globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
    ).IS_REACT_ACT_ENVIRONMENT = undefined;
    vi.clearAllMocks();
  });

  it("renders providers, metrics, and navigation hint to libraries", async () => {
    await act(async () => {
      root.render(<AdminScraperPanel />);
      await flushEffects();
    });

    expect(container.textContent).toContain("刮削框架配置");
    expect(container.textContent).toContain("Provider 健康状态");
    expect(container.textContent).toContain("TMDB");
    expect(container.textContent).toContain("TVDB");
    expect(container.textContent).toContain("Bangumi");
    expect(container.textContent).toContain("总缓存条目");
    expect(container.textContent).toContain("管理媒体库");
    expect(container.textContent).toContain("媒体库级别的刮削配置已迁移到媒体库管理页面");
    expect(container.textContent).toContain("电影");
    expect(container.textContent).toContain("电视剧");
    expect(container.textContent).toContain("图像");
  });
});
