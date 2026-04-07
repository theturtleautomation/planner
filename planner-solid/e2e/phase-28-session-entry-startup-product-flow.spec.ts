// @ts-nocheck
import { expect, test } from "@playwright/test";

test("phase 28 /sessions/new now redirects to project creation", async ({ page }) => {
  await page.goto("/sessions/new");

  await expect(page).toHaveURL("/projects/new");
  await expect(page.getByRole("button", { name: /create project/i })).toBeVisible();
});

test("phase 28 project-scoped saved-brief entry reaches the same startup truth", async ({ page, request }) => {
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
  await expect(page.locator(".session-interview-question, .session-thread-workspace")).toHaveCount(1, { timeout: 20000 });
  await expect.poll(async () =>
    page.evaluate(() =>
      window.__socketMessages.filter((message) => message?.type === "start_socratic").length,
    ),
  ).toBe(1);
});
