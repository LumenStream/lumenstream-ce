/// <reference path="../.astro/types.d.ts" />

declare namespace App {
  interface Locals {}
}

interface ImportMetaEnv {
  readonly PUBLIC_LS_API_BASE_URL?: string;
  readonly PUBLIC_APP_TITLE?: string;
  readonly PUBLIC_LS_ENABLE_MOCK?: string;
  readonly PUBLIC_LS_MOCK_MODE?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}

interface Window {
  __LS_CONFIG__?: {
    apiBaseUrl?: string;
  };
}
