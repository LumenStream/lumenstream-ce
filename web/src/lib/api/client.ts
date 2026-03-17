import { clearAuthSession, getAccessToken } from "@/lib/auth/token";

export interface ApiError {
  status: number;
  message: string;
  requestId?: string;
}

export interface RequestOptions extends RequestInit {
  auth?: boolean;
  query?: Record<string, string | number | boolean | undefined | null>;
}

const DEFAULT_BASE_URL = "http://127.0.0.1:8096";

function getRuntimeApiBaseUrl(): string | undefined {
  if (typeof window === "undefined") {
    return undefined;
  }

  const runtimeValue = window.__LS_CONFIG__?.apiBaseUrl;
  if (typeof runtimeValue !== "string") {
    return undefined;
  }

  const normalized = runtimeValue.trim().replace(/\/$/, "");
  return normalized.length > 0 ? normalized : undefined;
}

export function getApiBaseUrl(): string {
  const runtimeValue = getRuntimeApiBaseUrl();
  if (runtimeValue) {
    return runtimeValue;
  }

  const configured = import.meta.env.PUBLIC_LS_API_BASE_URL;
  if (!configured) {
    return DEFAULT_BASE_URL;
  }

  const normalized = configured.trim();
  if (normalized.length === 0 || normalized === "undefined" || normalized === "null") {
    return DEFAULT_BASE_URL;
  }

  return normalized.replace(/\/$/, "");
}

function buildUrl(path: string, query?: RequestOptions["query"]): string {
  const base = getApiBaseUrl();
  const normalizedPath = path.startsWith("/") ? path : `/${path}`;
  const url = new URL(`${base}${normalizedPath}`);

  if (query) {
    Object.entries(query).forEach(([key, value]) => {
      if (value === undefined || value === null || value === "") {
        return;
      }
      url.searchParams.set(key, String(value));
    });
  }

  return url.toString();
}

function normalizeErrorPayload(payload: unknown): string {
  if (payload && typeof payload === "object" && "error" in payload) {
    const maybe = (payload as { error?: unknown }).error;
    if (typeof maybe === "string") {
      return maybe;
    }
  }

  return "请求失败";
}

export async function apiRequest<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const { auth = true, headers, query, ...rest } = options;
  const mergedHeaders = new Headers(headers || {});

  if (!mergedHeaders.has("Content-Type") && rest.body) {
    mergedHeaders.set("Content-Type", "application/json");
  }

  if (auth) {
    const token = getAccessToken();
    if (!token) {
      throw {
        status: 401,
        message: "缺少登录态，请重新登录",
      } satisfies ApiError;
    }
    mergedHeaders.set("Authorization", `Bearer ${token}`);
  }

  const response = await fetch(buildUrl(path, query), {
    ...rest,
    headers: mergedHeaders,
  });

  const requestId = response.headers.get("x-request-id") || undefined;
  const contentType = response.headers.get("content-type") || "";

  if (!response.ok) {
    let message = `${response.status} ${response.statusText}`;
    if (contentType.includes("application/json")) {
      try {
        const payload = await response.json();
        message = normalizeErrorPayload(payload);
      } catch {
        // Keep default message on malformed payload.
      }
    } else {
      try {
        const text = await response.text();
        if (text.trim().length > 0) {
          message = text;
        }
      } catch {
        // Keep default message.
      }
    }

    if (response.status === 401) {
      clearAuthSession();
    }

    throw {
      status: response.status,
      message,
      requestId,
    } satisfies ApiError;
  }

  if (response.status === 204) {
    return undefined as T;
  }

  if (contentType.includes("application/json")) {
    return (await response.json()) as T;
  }

  return (await response.text()) as T;
}
