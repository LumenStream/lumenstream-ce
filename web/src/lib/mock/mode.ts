import { isBrowser } from "@/lib/utils";

const MOCK_MODE_KEY = "ls.mock_mode";

function envMockFeatureOverride(): boolean | null {
  const value = import.meta.env.PUBLIC_LS_ENABLE_MOCK;
  if (value === "true") {
    return true;
  }
  if (value === "false") {
    return false;
  }
  return null;
}

export function isMockFeatureEnabled(): boolean {
  const override = envMockFeatureOverride();
  if (override !== null) {
    return override;
  }
  return import.meta.env.DEV;
}

function envMockModeEnabled(): boolean {
  return import.meta.env.PUBLIC_LS_MOCK_MODE === "true";
}

export function isMockMode(): boolean {
  if (!isMockFeatureEnabled()) {
    return false;
  }

  if (!isBrowser()) {
    return envMockModeEnabled();
  }

  return window.sessionStorage.getItem(MOCK_MODE_KEY) === "1" || envMockModeEnabled();
}

export function setMockMode(enabled: boolean): void {
  if (!isBrowser()) {
    return;
  }

  if (!isMockFeatureEnabled()) {
    window.sessionStorage.removeItem(MOCK_MODE_KEY);
    return;
  }

  if (enabled) {
    window.sessionStorage.setItem(MOCK_MODE_KEY, "1");
  } else {
    window.sessionStorage.removeItem(MOCK_MODE_KEY);
  }
}

function shouldFallbackToMock(error: unknown): boolean {
  if (error instanceof TypeError) {
    return true;
  }

  if (typeof error === "object" && error !== null && "status" in error) {
    return false;
  }

  return true;
}

export async function runWithMock<T>(
  mockFn: () => Promise<T> | T,
  realFn: () => Promise<T>
): Promise<T> {
  if (!isMockFeatureEnabled()) {
    return await realFn();
  }

  if (isMockMode()) {
    return await Promise.resolve(mockFn());
  }

  try {
    return await realFn();
  } catch (error) {
    if (!shouldFallbackToMock(error)) {
      throw error;
    }

    setMockMode(true);
    return await Promise.resolve(mockFn());
  }
}
