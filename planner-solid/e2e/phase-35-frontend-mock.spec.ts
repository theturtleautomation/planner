import { expect, test } from "@playwright/test";

test("phase 35 frontend mock runtime drives Builder-targeted shell navigation and route continuity", async ({ page }) => {
  await page.goto("/?mockScenario=default&name=Canonical%20home%20fallback&description=Home%20route%20owns%20fallback%20creation.");
  await expect(page).toHaveURL(/\/projects\/canonical-home-fallback(\?mockScenario=default)?$/);
  await expect(page.getByRole("button", { name: "Start analysis" })).toBeVisible();

  await page.goto("/");
  await expect(page.locator(".app-shell")).toHaveCount(1);
  await expect(page.locator('nav[aria-label="Primary"]')).toHaveCount(1);

  await expect(page.getByTestId("frontend-mock-badge").first()).toHaveText("Frontend mock · default");

  const primaryNav = page.getByRole("navigation", { name: "Primary" });

  await expect(primaryNav.getByRole("link", { name: "Projects" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Create project" })).toBeVisible();
  await expect(page.getByRole("link", { name: /direct session/i })).toHaveCount(0);
  await page.getByLabel("Project title").fill("Home directory pilot");
  await page.getByLabel("Description").fill("Moved the project directory onto the home route.");
  await page.getByRole("button", { name: "Create project" }).click();
  await expect(page).toHaveURL(/\/projects\/home-directory-pilot(\?mockScenario=default)?$/);

  await expect(page.getByRole("button", { name: "Start analysis" })).toBeVisible();

  await page.goto("/projects/personal-calendar/import");
  await expect(page.getByRole("heading", { name: "Import review", exact: true })).toBeVisible();

  await primaryNav.getByRole("link", { name: "Sessions" }).first().click();
  await expect(page).toHaveURL(/\/sessions(\?mockScenario=default)?$/);
  await expect(page.getByRole("heading", { name: "Current work queue", exact: true })).toBeVisible();
  await expect(page.getByRole("link", { name: /direct session/i })).toHaveCount(0);

  await page.getByRole("link", { name: "Calendar intake" }).first().click();
  await expect(page).toHaveURL(/\/sessions\/session-1(\?mockScenario=default)?$/);
  await expect(page.getByRole("heading", { name: "Calendar intake", exact: true })).toBeVisible();
  await expect(page.locator(".session-question-status-row")).toBeVisible();
  await expect(page.locator(".session-question-actions-trigger")).toHaveText("Actions");

  await primaryNav.getByRole("link", { name: "Knowledge" }).first().click();
  await expect(page).toHaveURL(/\/knowledge(\?mockScenario=default)?$/);
  await expect(page.getByRole("heading", { name: "Nodes", exact: true })).toBeVisible();

  await primaryNav.getByRole("link", { name: "Blueprint" }).first().click();
  await expect(page).toHaveURL(/\/blueprint(\?mockScenario=default)?$/);
  await expect(page.getByRole("heading", { name: "Blueprint", exact: true })).toBeVisible();

  await primaryNav.getByRole("link", { name: "Events" }).first().click();
  await expect(page).toHaveURL(/\/events(\?mockScenario=default)?$/);
  await expect(page.getByRole("heading", { name: "Events", exact: true })).toBeVisible();
  await expect(page.getByText("Recorded a blueprint checkpoint export").first()).toBeVisible();

  await primaryNav.getByRole("link", { name: "Discovery" }).first().click();
  await expect(page).toHaveURL(/\/discovery(\?mockScenario=default)?$/);
  await expect(page.getByRole("heading", { name: "Discovery", exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Selected proposal" })).toBeVisible();

  await primaryNav.getByRole("link", { name: "Admin" }).first().click();
  await expect(page).toHaveURL(/\/admin(\?mockScenario=default)?$/);
  await expect(page.getByRole("heading", { name: "Admin", exact: true })).toBeVisible();

  await primaryNav.getByRole("link", { name: "Home" }).first().click();
  await expect(page).toHaveURL(/\/(\?mockScenario=default)?$/);
  await expect(page.getByText("Home directory pilot").first()).toBeVisible();
  await expect(page.getByText("Personal Calendar").first()).toBeVisible();

  await page.goto("/projects");
  await expect(page).toHaveURL(/\/(\?mockScenario=default)?$/);
  await expect(page.locator(".app-shell")).toHaveCount(1);
  await expect(page.getByText("Redirecting to home…")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Create project" })).toBeVisible();
});

test("phase 37 session workspace keeps the project picture primary while area shaping stays local in frontend mock mode", async ({ page }) => {
  await page.goto("/sessions/session-11?mockScenario=session-workspace");
  const areaCards = page.locator(".session-project-area-card");
  const areaWorkspace = page.locator(".session-area-workspace");

  await expect(page.getByRole("heading", { name: "Session workspace mock", exact: true })).toBeVisible();
  await expect(page.getByText("Current project shape")).toBeVisible();
  await expect(areaCards).toHaveCount(5);
  await expect(page.getByRole("heading", { name: "Transformation", exact: true })).toBeVisible();
  await expect(page.locator(".session-area-preview-dominant")).toBeVisible();
  await expect(page.locator(".session-area-preview-dominant .session-area-pressure-summary")).toHaveText("Confirm the main workflow shape.");
  await expect(areaWorkspace.getByRole("button", { name: "Go deeper in Transformation" })).toBeVisible();
  await expect(page.getByRole("textbox", { name: "Type your answer" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Commit and next" })).toHaveCount(0);
  await expect(page.locator(".session-area-preview-secondary")).toHaveCount(1);
  await expect(page.locator(".session-area-preview-secondary-title")).toHaveText("Scope");
});

test("phase 37.1 project picture remains primary below desktop widths", async ({ page }) => {
  await page.setViewportSize({ width: 840, height: 900 });
  await page.goto("/sessions/session-11?mockScenario=session-workspace");

  await expect(page.getByText("Current project shape")).toBeVisible();
  await expect(page.locator(".session-area-preview")).toBeVisible();
  await expect(page.locator(".session-area-preview-dominant .session-area-pressure-summary")).toHaveText("Confirm the main workflow shape.");
});


test("phase 38.3 session workspace keeps the project picture primary on ultra-wide viewports", async ({ page }) => {
  await page.setViewportSize({ width: 1680, height: 1050 });
  await page.goto("/sessions/session-11?mockScenario=session-workspace");

  await expect(page.locator(".session-project-shell")).toHaveCount(1);
  await expect(page.locator(".session-project-primary")).toHaveCount(1);
  await expect(page.locator(".session-project-support")).toHaveCount(1);
  await expect(page.getByText("Current project shape")).toBeVisible();
  await expect(page.getByText("Next move")).toBeVisible();
  await expect(page.locator(".session-area-preview")).toHaveCount(1);
  await expect(page.getByRole("textbox", { name: "Type your answer" })).toHaveCount(0);

  await page.setViewportSize({ width: 1280, height: 960 });
  await expect(page.locator(".session-project-support")).toBeVisible();
  await expect(page.locator(".session-area-workspace")).toHaveCount(1);
});


test("phase 39 commit and next preserves workspace continuity as prompt-bank updates land", async ({ page }) => {
  await page.goto("/sessions/session-11?mockScenario=session-workspace");

  const initialUrl = page.url();
  const seedText = "Maybe habit streaks matter more than total volume.";
  await page.locator(".session-area-workspace").getByRole("button", { name: "Go deeper in Transformation" }).click();
  await expect(page.locator(".session-area-shaping")).toBeVisible();
  await expect(page.locator(".session-area-shaping-object")).toHaveCount(3);
  await expect(page.locator(".session-area-pressure-point.is-dominant")).toBeVisible();
  await expect(page.locator(".session-area-shaping .session-area-pressure-point, .session-area-shaping .session-area-preview-secondary")).toHaveCount(2);
  await expect(page.getByText("Pending revisions")).toBeVisible();
  await expect(page.locator(".session-area-revision-kind")).toHaveText("North-star revision");
  await expect(page.getByText("conflict", { exact: true })).toBeVisible();
  const areaCapture = page.getByPlaceholder("Capture something local to Transformation");
  await areaCapture.fill(seedText);
  await page.getByRole("button", { name: "Save as seed" }).click();
  await expect(areaCapture).toHaveValue("");
  await expect(page.getByText("1 seed resting quietly for later.")).toBeVisible();
  await expect(page.getByText("Next move")).toHaveCount(0);
  await expect(page.locator(".session-project-support")).toHaveCount(0);
  await expect(page.getByRole("textbox", { name: "Type your answer" })).toHaveCount(0);
  await page.getByRole("button", { name: "Discuss in composer" }).click();
  await expect(page.getByText("Resurfaced seed")).toBeVisible();
  await expect(page.getByText(seedText)).toBeVisible();
  await expect(page.getByRole("button", { name: "Promote into active work" })).toBeVisible();
  await page.getByRole("button", { name: "Dismiss for now" }).click();
  await expect(page.getByText(seedText)).toHaveCount(0);
  await page.getByRole("button", { name: "Back to shaping" }).click();
  await expect(page.getByText("Resurfaced seed")).toBeVisible();
  await page.getByRole("button", { name: "Promote into active work" }).click();
  await expect(areaCapture).toHaveValue(seedText);
  await expect(page.getByText("Resurfaced seed")).toHaveCount(0);
  await page.getByRole("button", { name: "Discuss in composer" }).click();
  await expect(page.getByText("Pending revisions still in context")).toBeVisible();
  await expect(page.getByText("Area context")).toBeVisible();
  await expect(page.getByRole("button", { name: "Open composer" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Back to shaping" })).toBeVisible();
  await expect(page.getByRole("textbox", { name: "Type your answer" })).toHaveCount(0);
  await expect(page.locator(".session-project-support")).toHaveCount(0);
  await page.getByRole("button", { name: "Open composer" }).click();
  await expect(page.getByRole("textbox", { name: "Type your answer" })).toBeVisible();
  await page.getByRole("textbox", { name: "Type your answer" }).fill("Weekly planning should feel effortless.");
  await page.getByRole("button", { name: "Commit and next" }).click();

  await expect(page).toHaveURL(initialUrl);
  await expect(page.locator(".session-project-picture")).toHaveCount(1);
  await expect(page.locator(".session-area-workspace")).toHaveCount(1);
  await expect(page.locator(".session-area-preview")).toHaveCount(0);
  await expect(page.locator(".session-area-shaping")).toHaveCount(0);
  await expect(page.locator(".session-question-loading")).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Transformation", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Back to shaping" })).toBeVisible();
  await expect(page.locator(".session-project-area-card")).toHaveCount(5);
  await expect(page.getByText(/prompt draft save rejected because the prompt is no longer current/i)).toHaveCount(0);
});
