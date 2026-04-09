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
  const constraintArea = page.locator(".session-project-area-card").filter({ hasText: "Constraints" }).first();
  const seedAnswers = [
    "Create a timer, edit intervals, and run a guided workout end to end.",
    "Success means one workout can be configured and completed without confusion or missed prompts.",
    "Analytics, collaboration, and third-party integrations should stay out of the first version.",
  ];

  for (const answer of seedAnswers) {
    if (await constraintArea.isVisible()) return;
    await answerActiveQuestion(page, answer);
  }

  await expect(constraintArea).toBeVisible();
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
  await expect(page.locator("h1.session-question-title")).toHaveText(sessionTitle);
  await expect(page.getByText("Current project shape")).toBeVisible();
  await expect(page.locator(".session-question-actions-trigger")).toHaveText("Actions");

  expect(failures.pageErrors).toEqual([]);
  expect(failures.consoleErrors).toEqual([]);
});

test("phase 37.2 canonical runtime keeps the project picture primary as additional work becomes live", async ({ page, request }) => {
  const { sessionId, sessionTitle } = await createWaitingSession(
    request,
    "Build a CLI workout timer that guides one person through interval training.",
  );
  const failures = trackClientBootstrapFailures(page);
  const seedText = "Maybe guided cooldowns should adapt to how hard the last interval felt.";

  await page.setViewportSize({ width: 1280, height: 960 });
  await page.goto(`/sessions/${sessionId}`);

  await expect(page.locator(".app-shell")).toBeVisible();
  await expect(page.locator("h1.session-question-title")).toHaveText(sessionTitle);
  await expect(page.locator(".session-question-header")).toBeVisible();
  await expect(page.locator(".session-project-picture")).toHaveCount(1);
  await expect(page.locator(".session-area-workspace")).toHaveCount(1);
  await expect(page.locator(".session-area-preview")).toHaveCount(1);
  await expect(page.getByText("Current project shape")).toBeVisible();
  await expect(page.getByRole("textbox", { name: "Type your answer" })).toHaveCount(0);

  await ensureMultipleLiveThreads(page);

  await expect(page.locator(".session-project-area-card")).toHaveCount(5);
  await expect(page.locator(".session-project-area-card").filter({ hasText: "Constraints" }).first()).toBeVisible();
  const transformationCard = page.locator(".session-project-area-card").filter({ hasText: "Transformation" }).first();
  await expect(transformationCard.locator(".session-project-area-freshness")).toBeVisible();
  await expect(transformationCard.locator(".session-project-area-summary")).toBeVisible();
  await expect(transformationCard.locator(".session-project-area-relation")).toHaveCount(1);
  await expect(page.locator(".session-area-workspace")).toHaveCount(1);
  await page.locator(".session-area-workspace").getByRole("button", { name: /Go deeper in/i }).click();
  await expect(page.locator(".session-area-shaping")).toBeVisible();
  await expect(page.locator(".session-area-shaping-object")).toHaveCount(3);
  const revisionCards = page.locator(".session-area-revision-card");
  if ((await revisionCards.count()) > 0) {
    await expect(revisionCards.first()).toContainText(/revision/i);
    await expect(page.getByText(/pending revision|conflict/i).first()).toBeVisible();
  }
  const areaCapture = page.getByPlaceholder(/Capture something local to/);
  await areaCapture.fill(seedText);
  await page.getByRole("button", { name: "Save as seed" }).click();
  await expect(areaCapture).toHaveValue("");
  await expect(page.getByText("1 seed resting quietly for later.")).toBeVisible();
  await expect(page.getByRole("textbox", { name: "Type your answer" })).toHaveCount(0);
  await page.getByRole("button", { name: "Discuss in composer" }).click();
  await expect(page.getByText("Resurfaced seed")).toBeVisible();
  await expect(page.getByText(seedText)).toBeVisible();
  await expect(page.getByRole("button", { name: "Promote into active work" })).toBeVisible();
  await expect(page.getByText("Area context")).toBeVisible();
  await expect(page.locator(".session-project-support")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Open composer" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Back to shaping" })).toBeVisible();
  await expect(page.getByRole("textbox", { name: "Type your answer" })).toHaveCount(0);
  await page.getByRole("button", { name: "Open composer" }).click();
  await expect(page.getByRole("textbox", { name: "Type your answer" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Commit and next" })).toBeVisible();

  expect(failures.pageErrors).toEqual([]);
  expect(failures.consoleErrors).toEqual([]);
});
