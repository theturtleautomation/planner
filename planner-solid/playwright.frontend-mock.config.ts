import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  testMatch: /phase-(08-events|12-discovery|35-frontend-mock)\.spec\.ts/,
  workers: 1,
  use: {
    baseURL: "http://127.0.0.1:3000",
    headless: true,
  },
  webServer: {
    command: "npm run dev -- --host 127.0.0.1 --port 3000 --strictPort",
    env: {
      VITE_PLANNER_FRONTEND_MOCK: "1",
    },
    port: 3000,
    reuseExistingServer: false,
    timeout: 120_000,
  },
});
