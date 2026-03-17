/**
 * @vitest-environment jsdom
 */

import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  cancelTaskRun,
  getTaskRunsWebSocketUrl,
  listTaskDefinitions,
  listTaskRuns,
  patchTaskDefinition,
  runTaskNow,
} from "@/lib/api/admin";

import { AdminJobsPanel } from "./AdminJobsPanel";

const mockAuthState = {
  ready: true,
};

vi.mock("@/lib/auth/use-auth-session", () => ({
  useAuthSession: () => mockAuthState,
}));

vi.mock("@/components/domain/DataState", () => ({
  LoadingState: () => React.createElement("div", null, "loading"),
  ErrorState: () => React.createElement("div", null, "error"),
}));

vi.mock("@/lib/auth/token", () => ({
  getAccessToken: () => null,
}));

vi.mock("@/lib/api/admin", () => ({
  cancelTaskRun: vi.fn(),
  getTaskRunsWebSocketUrl: vi.fn(),
  listTaskDefinitions: vi.fn(),
  listTaskRuns: vi.fn(),
  patchTaskDefinition: vi.fn(),
  runTaskNow: vi.fn(),
}));

const mockGetTaskRunsWebSocketUrl = vi.mocked(getTaskRunsWebSocketUrl);
const mockCancelTaskRun = vi.mocked(cancelTaskRun);
const mockListTaskDefinitions = vi.mocked(listTaskDefinitions);
const mockListTaskRuns = vi.mocked(listTaskRuns);
const mockPatchTaskDefinition = vi.mocked(patchTaskDefinition);
const mockRunTaskNow = vi.mocked(runTaskNow);

async function flushEffects() {
  await Promise.resolve();
  await Promise.resolve();
}

function setTextareaValue(element: HTMLTextAreaElement, value: string) {
  const descriptor = Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value");
  descriptor?.set?.call(element, value);
  element.dispatchEvent(new Event("input", { bubbles: true }));
}

describe("AdminJobsPanel", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    (
      globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
    ).IS_REACT_ACT_ENVIRONMENT = true;
    container = document.createElement("div");
    document.body.appendChild(container);
    root = createRoot(container);

    mockListTaskDefinitions.mockResolvedValue([
      {
        task_key: "sync.metadata",
        display_name: "Metadata Sync",
        enabled: true,
        cron_expr: "0 30 3 * * *",
        default_payload: {},
        max_attempts: 3,
        created_at: "2026-02-16T00:00:00Z",
        updated_at: "2026-02-16T00:00:00Z",
      },
    ]);
    mockGetTaskRunsWebSocketUrl.mockReturnValue(
      "ws://127.0.0.1:8096/admin/task-center/ws?token=test"
    );
    mockListTaskRuns.mockResolvedValue([]);
    mockPatchTaskDefinition.mockResolvedValue({
      task_key: "sync.metadata",
      display_name: "Metadata Sync",
      enabled: true,
      cron_expr: "0 30 3 * * *",
      default_payload: {},
      max_attempts: 3,
      created_at: "2026-02-16T00:00:00Z",
      updated_at: "2026-02-16T00:00:00Z",
    });
    mockRunTaskNow.mockResolvedValue({
      id: "run-001",
      kind: "sync.metadata",
      status: "queued",
      payload: {},
      attempts: 0,
      max_attempts: 3,
      dead_letter: false,
      created_at: "2026-02-16T00:00:00Z",
    });
    mockCancelTaskRun.mockResolvedValue({
      id: "run-001",
      kind: "sync.metadata",
      status: "cancelled",
      payload: {},
      progress: {
        phase: "cancelled",
        total: 1,
        completed: 1,
        percent: 100,
        message: "任务已取消（排队中）",
      },
      attempts: 0,
      max_attempts: 3,
      dead_letter: false,
      created_at: "2026-02-16T00:00:00Z",
      finished_at: "2026-02-16T00:01:00Z",
    });
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

  it("shows inline JSON errors and blocks save/run when override payload is invalid", async () => {
    await act(async () => {
      root.render(<AdminJobsPanel />);
      await flushEffects();
    });

    const configButton = Array.from(container.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "配置"
    );
    expect(configButton).not.toBeUndefined();

    await act(async () => {
      configButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });

    const dialog = document.body.querySelector("[role='dialog']") as HTMLElement | null;
    expect(dialog).not.toBeNull();

    const textareas = Array.from(dialog?.querySelectorAll("textarea") ?? []);
    expect(textareas.length).toBeGreaterThanOrEqual(2);
    const runPayloadTextarea = textareas[1] as HTMLTextAreaElement | undefined;
    expect(runPayloadTextarea).toBeDefined();

    await act(async () => {
      if (runPayloadTextarea) {
        setTextareaValue(runPayloadTextarea, "{");
      }
      await flushEffects();
    });

    const saveButton = Array.from(dialog?.querySelectorAll("button") ?? []).find(
      (button) => button.textContent?.trim() === "保存配置"
    ) as HTMLButtonElement | undefined;
    const runButton = Array.from(dialog?.querySelectorAll("button") ?? []).find(
      (button) => button.textContent?.trim() === "立即执行"
    ) as HTMLButtonElement | undefined;

    expect(saveButton).toBeDefined();
    expect(runButton).toBeDefined();
    expect(saveButton?.disabled).toBe(true);
    expect(runButton?.disabled).toBe(true);

    await act(async () => {
      saveButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      runButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });

    expect(mockPatchTaskDefinition).not.toHaveBeenCalled();
    expect(mockRunTaskNow).not.toHaveBeenCalled();
  });

  it("cancels a queued run via compact progress action", async () => {
    mockListTaskRuns.mockResolvedValueOnce([
      {
        id: "run-001",
        kind: "sync.metadata",
        status: "queued",
        payload: {},
        progress: {
          phase: "queued",
          total: 0,
          completed: 0,
          percent: 0,
          message: "任务已排队",
        },
        attempts: 0,
        max_attempts: 3,
        dead_letter: false,
        created_at: "2026-02-16T00:00:00Z",
      },
    ]);

    await act(async () => {
      root.render(<AdminJobsPanel />);
      await flushEffects();
    });

    const cancelButton = container.querySelector(
      'button[aria-label="取消任务 run-001"]'
    ) as HTMLButtonElement | null;
    expect(cancelButton).not.toBeNull();

    await act(async () => {
      cancelButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });

    expect(mockCancelTaskRun).toHaveBeenCalledWith("run-001");
  });
});
