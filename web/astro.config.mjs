import { fileURLToPath } from "node:url";

import { defineConfig } from "astro/config";
import react from "@astrojs/react";

import tailwindcss from "@tailwindcss/vite";

function detectAstroCommand() {
  const knownCommands = new Set(["dev", "build", "preview"]);
  const matched = process.argv.find((arg) => knownCommands.has(arg));
  return matched || "";
}

function resolveMockFeatureEnabled(command) {
  const override = process.env.PUBLIC_LS_ENABLE_MOCK;
  if (override === "true") {
    return true;
  }
  if (override === "false") {
    return false;
  }
  return command === "dev";
}

const mockFeatureEnabled = resolveMockFeatureEnabled(detectAstroCommand());
const mockAliases = mockFeatureEnabled
  ? {}
  : {
      "@/lib/mock/api": fileURLToPath(new URL("./src/lib/mock/api.disabled.ts", import.meta.url)),
      "@/lib/mock/billing": fileURLToPath(
        new URL("./src/lib/mock/billing.disabled.ts", import.meta.url)
      ),
    };
const resolveAlias = Object.entries({
  ...mockAliases,
  "@": fileURLToPath(new URL("./src", import.meta.url)),
}).map(([find, replacement]) => ({ find, replacement }));

export default defineConfig({
  integrations: [react()],
  output: "static",
  vite: {
    server: {
      port: 4321,
    },

    resolve: {
      alias: resolveAlias,
    },

    plugins: [tailwindcss()],
    build: {
      rollupOptions: {
        output: {
          manualChunks: {
            "three-vendor": ["three", "@react-three/fiber"],
          },
        },
      },
    },
  },
});
