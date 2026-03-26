// @ts-nocheck
import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    window.__socketMessages = [];
    const NativeWebSocket = window.WebSocket;

    window.WebSocket = class TrackingWebSocket extends NativeWebSocket {
      send(payload) {
        try {
          window.__socketMessages.push(JSON.parse(String(payload)));
        } catch {
          // Ignore non-JSON payloads from browser instrumentation.
        }
        return super.send(payload);
      }
    };
  });
});

async function startSessionFromNewPage(page, description: string) {
  await page.goto("/sessions/new");
  await page.locator("textarea").fill(description);
  const startButton = page.getByRole("button", { name: /start direct session/i });
  await expect(startButton).toBeVisible({ timeout: 20_000 });
  await startButton.click();
  await expect(page).toHaveURL(/\/sessions\/[^/]+$/);
  return page.url().split("/").pop() as string;
}

test("phase 28 /sessions/new creates a startup-ready saved-brief session and reaches first reveal", async ({ page }) => {
  const sessionId = await startSessionFromNewPage(
    page,
    "Build a CLI workout timer that guides one person through interval training.",
  );

  await expect(page.getByRole("heading", { name: "Analysis needs a restart" })).toHaveCount(0);
  await expect(page.locator(".session-interview-question")).toBeVisible({ timeout: 20_000 });
  await expect.poll(async () =>
    page.evaluate(() =>
      window.__socketMessages.filter((message) => message?.type === "start_socratic").length,
    ),
  ).toBe(1);

  expect(sessionId).toBeTruthy();
});

test("phase 28 reload during startup keeps the /sessions/new path in startup truth until first reveal", async ({ page }) => {
  await startSessionFromNewPage(
    page,
    "Build a CLI workout timer [phase28-slow-startup] that guides one person through interval training.",
  );

  await expect(page.getByRole("heading", { name: "Starting analysis" })).toBeVisible();
  await page.reload();

  await expect(page.getByRole("heading", { name: "Analysis needs a restart" })).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Starting analysis" })).toBeVisible();
  await expect(page.locator(".session-interview-question")).toBeVisible({ timeout: 30_000 });
});

test("phase 28 early startup failure preserves the saved brief and retry reaches first reveal", async ({ page }) => {
  const description =
    "Build a CLI workout timer [phase28-fail-once] that guides one person through interval training.";
  const sessionId = await startSessionFromNewPage(page, description);
  const retryStartupButton = page.getByRole("button", { name: "Retry startup" }).first();

  await expect(page.getByRole("heading", { name: "Needs attention" })).toBeVisible({ timeout: 20_000 });
  await expect(retryStartupButton).toBeVisible();
  await expect(page.getByRole("heading", { name: /Build a CLI workout timer/ })).toBeVisible();

  await retryStartupButton.click();
  await expect(page.getByRole("heading", { name: "Analysis needs a restart" })).toHaveCount(0);
  await expect(page.locator(".session-interview-question")).toBeVisible({ timeout: 30_000 });
  await expect.poll(async () =>
    page.evaluate(() =>
      window.__socketMessages.filter((message) => message?.type === "start_socratic").length,
    ),
  ).toBeGreaterThanOrEqual(2);
});

test("phase 28 project-scoped saved-brief entry reaches the same startup truth", async ({ page, request }) => {
  const createProjectResponse = await request.post("/api/projects", {
    data: {
      name: "Ops Console",
      description: "Track service health, alerts, and deployment posture.",
    },
  });
  expect(createProjectResponse.ok()).toBeTruthy();
  const { project } = await createProjectResponse.json();

  await page.goto(`/projects/${project.slug}`);
  await page.getByRole("button", { name: "Start analysis" }).click();
  await expect(page).toHaveURL(/\/sessions\/[^/]+$/);

  await expect(page.getByRole("heading", { name: "Analysis needs a restart" })).toHaveCount(0);
  await expect(page.locator(".session-interview-question")).toBeVisible({ timeout: 20_000 });
  await expect.poll(async () =>
    page.evaluate(() =>
      window.__socketMessages.filter((message) => message?.type === "start_socratic").length,
    ),
  ).toBe(1);
});
