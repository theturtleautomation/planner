import type { APIRequestContext, Page } from "@playwright/test";
import { expect, test } from "@playwright/test";

function trackClientBootstrapFailures(page: Page) {
  const pageErrors: string[] = [];
  const consoleErrors: string[] = [];

  page.on("pageerror", (error: Error) => {
    pageErrors.push(String(error));
  });

  page.on("console", (message) => {
    if (message.type() === "error") {
      consoleErrors.push(message.text());
    }
  });

  return { pageErrors, consoleErrors };
}

async function createWaitingSession(request: APIRequestContext, description: string) {
  const nonce = Date.now();
  const projectResponse = await request.post("/api/projects", {
    data: {
      name: `Phase 37.3 Static Runtime ${nonce}`,
    },
  });
  expect(projectResponse.ok()).toBeTruthy();
  const projectPayload = await projectResponse.json();
  const projectRef = projectPayload.project.slug ?? projectPayload.project.id;

  const sessionResponse = await request.post(`/api/projects/${projectRef}/sessions`, {
    data: {
      description,
    },
  });
  expect(sessionResponse.ok()).toBeTruthy();
  const sessionPayload = await sessionResponse.json();
  return {
    sessionId: sessionPayload.session.id as string,
    sessionTitle: sessionPayload.session.title as string,
  };
}

test("phase 37.3 canonical static runtime mounts the home route without hydration crash", async ({ page }) => {
  const failures = trackClientBootstrapFailures(page);

  await page.goto("/");
  await expect(page.locator(".app-shell")).toBeVisible();
  await expect(page.getByRole("button", { name: "Create project" })).toBeVisible();

  expect(failures.pageErrors).toEqual([]);
  expect(failures.consoleErrors).toEqual([]);
});

test("phase 37.3 canonical static runtime mounts a session route without hydration crash", async ({ page, request }) => {
  const { sessionId, sessionTitle } = await createWaitingSession(
    request,
    "Verify the server-backed static runtime mounts the session route before any live answering work begins.",
  );
  const failures = trackClientBootstrapFailures(page);

  await page.goto(`/sessions/${sessionId}`);
  await expect(page.locator(".app-shell")).toBeVisible();
  await expect(page.getByRole("heading", { name: sessionTitle })).toBeVisible();
  await expect(page.getByText("Question-bank workspace")).toBeVisible();
  await expect(page.getByText("Session actions")).toBeVisible();

  expect(failures.pageErrors).toEqual([]);
  expect(failures.consoleErrors).toEqual([]);
});
