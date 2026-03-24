// @ts-nocheck
import { expect, test } from "@playwright/test";

const statusPayload = {
  status: "ok",
  version: "0.1.0-test",
  uptime_secs: 5432,
  sessions: {
    active: 3,
    total_events: 42,
  },
  providers: [
    { name: "openai", binary: "codex", available: true },
    { name: "anthropic", binary: "claude", available: false },
  ],
};

const eventsPayload = {
  total: 2,
  events: [
    {
      id: "evt-1",
      timestamp: "2026-03-24T06:30:00Z",
      level: "error",
      source: "pipeline",
      session_id: "session-1",
      project_id: "project-1",
      project_name: "Personal Calendar",
      step: "pipeline.compile",
      message: "Pipeline compile failed for the latest run",
      duration_ms: 2300,
      metadata: {},
    },
    {
      id: "evt-2",
      timestamp: "2026-03-24T06:28:00Z",
      level: "info",
      source: "system",
      session_id: null,
      project_id: null,
      project_name: null,
      step: null,
      message: "Background cleanup completed",
      duration_ms: null,
      metadata: {},
    },
  ],
};

test.beforeEach(async ({ page }) => {
  await page.route("**/api/admin/status", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(statusPayload),
    });
  });

  await page.route("**/api/admin/events?**", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(eventsPayload),
    });
  });
});

test("phase 09 keeps admin anchored on health first and events second", async ({ page }) => {
  await page.goto("/admin");

  await expect(page.getByRole("heading", { name: "Admin", exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Runtime posture", exact: true })).toBeVisible();
  await expect(page.getByText("openai")).toBeVisible();
  await expect(page.getByText("Unavailable", { exact: true })).toBeVisible();

  await expect(page.getByRole("heading", { name: "Recent operator-visible events" })).toBeVisible();
  await expect(page.getByText("Pipeline compile failed for the latest run")).toBeVisible();

  await page.getByRole("button", { name: "Errors" }).click();
  await expect(page.getByText("Pipeline compile failed for the latest run")).toBeVisible();
});
