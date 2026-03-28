import { configDefaults, defineConfig } from "vitest/config";
import { nitroV2Plugin as nitro } from "@solidjs/vite-plugin-nitro-2";

import { solidStart } from "@solidjs/start/config";

export default defineConfig({
  plugins: [solidStart(),
    nitro()
  ],
  server: {
    proxy: {
      "/api": {
        target: "http://localhost:3100",
        changeOrigin: true,
        ws: true,
      },
    },
  },
  test: {
    environment: "jsdom",
    setupFiles: "./src/test/setup.ts",
    globals: true,
    exclude: [...configDefaults.exclude, "e2e/**"],
  },
});
