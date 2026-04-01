import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  use: {
    baseURL: "http://127.0.0.1:4177",
    headless: true,
  },
  webServer: {
    command: "cargo run --manifest-path ../Cargo.toml --bin planner-server -- --port 4177 --static-dir ./dist/static",
    env: {
      PLANNER_E2E_LLM_MOCK: "phase26_live",
      PLANNER_DATA_DIR: "../target/playwright-data-static",
      PLANNER_RATE_LIMIT_MAX_REQUESTS: "1000",
    },
    port: 4177,
    reuseExistingServer: false,
    timeout: 120_000,
  },
});
