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

async function answerActiveQuestion(page: Page, answer: string) {
  const textbox = page.getByRole("textbox", { name: "Type your answer" });
  await expect(textbox).toBeVisible();
  await textbox.fill(answer);
  await page.getByRole("button", { name: "Commit and next" }).click();
}

async function ensureMultipleLiveThreads(page: Page) {
  const railButtons = page.locator(".session-thread-rail-button");
  const seedAnswers = [
    "Create a timer, edit intervals, and run a guided workout end to end.",
    "Success means one workout can be configured and completed without confusion or missed prompts.",
    "Analytics, collaboration, and third-party integrations should stay out of the first version.",
  ];

  for (const answer of seedAnswers) {
    if ((await railButtons.count()) > 1) return;
    await answerActiveQuestion(page, answer);
  }

  await expect.poll(async () => await railButtons.count(), {
    message: "expected the canonical runtime to expose multiple live threads after progressing the initial interview",
  }).toBeGreaterThan(1);
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

test("phase 37.2 canonical runtime keeps the command rail truthful as queued work becomes live", async ({ page, request }) => {
  const { sessionId, sessionTitle } = await createWaitingSession(
    request,
    "Build a CLI workout timer that guides one person through interval training.",
  );
  const failures = trackClientBootstrapFailures(page);

  await page.setViewportSize({ width: 1280, height: 960 });
  await page.goto(`/sessions/${sessionId}`);

  await expect(page.locator(".app-shell")).toBeVisible();
  await expect(page.getByRole("heading", { name: sessionTitle })).toBeVisible();
  await expect(page.locator(".session-question-header")).toBeVisible();
  await expect(page.locator(".session-question-summary-strip")).toHaveCount(0);
  await expect(page.locator(".session-question-jumpbar")).toHaveCount(0);
  await expect(page.locator(".session-question-shell > .session-question-rail")).toHaveCount(1);
  await expect(page.locator(".session-thread-workspace")).toHaveCount(1);
  await expect(page.locator(".session-queued-panel")).toHaveCount(0);
  if ((await page.locator(".session-question-queued-disclosure").count()) > 0) {
    await expect(page.locator(".session-question-queued-disclosure")).toBeVisible();
  }

  await ensureMultipleLiveThreads(page);

  const railButtons = page.locator(".session-thread-rail-button");
  const initialUrl = page.url();
  const initialHeading = (await page.locator(".session-thread-section-title").textContent())?.trim();
  const nextThreadTitle = (await railButtons.nth(1).locator(".session-thread-rail-title").textContent())?.trim();

  expect(initialHeading).toBeTruthy();
  expect(nextThreadTitle).toBeTruthy();
  expect(nextThreadTitle).not.toBe(initialHeading);

  await railButtons.nth(1).click();
  await expect(page).toHaveURL(initialUrl);
  await expect(page.locator(".session-thread-section-title")).toHaveText(nextThreadTitle ?? "");
  await expect(page.getByRole("textbox", { name: "Type your answer" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Commit and next" })).toBeVisible();
  await expect(page.locator(".session-thread-section-title")).toHaveText(nextThreadTitle ?? "");

  expect(failures.pageErrors).toEqual([]);
  expect(failures.consoleErrors).toEqual([]);
});
