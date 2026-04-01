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

  await page.getByRole("link", { name: "Calendar intake" }).first().click();
  await expect(page).toHaveURL(/\/sessions\/session-1(\?mockScenario=default)?$/);
  await expect(page.getByRole("heading", { name: "Calendar intake", exact: true })).toBeVisible();
  await expect(page.getByText("Every banked question is available from the start.")).toHaveCount(0);
  await expect(page.locator(".session-question-progress-line")).toContainText("0 of 0 answers committed");
  await expect(page.locator(".session-question-status-row")).toBeVisible();

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

test("phase 37 session command rail keeps thread switching local in frontend mock mode", async ({ page }) => {
  await page.goto("/sessions/session-11?mockScenario=session-workspace");

  await expect(page.getByRole("heading", { name: "Session workspace mock", exact: true })).toBeVisible();
  await expect(page.getByText("0 of 4 answers committed")).toBeVisible();
  await expect(page.getByRole("button", { name: /Workflow/ })).toBeVisible();
  await expect(page.getByRole("button", { name: /Scope/ })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Workflow", exact: true })).toBeVisible();
  await expect(page.getByText("Confirm the main workflow shape.")).toBeVisible();

  await page.getByRole("button", { name: /Scope/ }).click();
  await expect(page).toHaveURL(/\/sessions\/session-11\?mockScenario=session-workspace$/);
  await expect(page.getByRole("heading", { name: "Scope", exact: true })).toBeVisible();
  await expect(page.getByText("Define the release boundaries before delivery handoff.")).toBeVisible();
  await expect(page.getByText("Which planning output needs to feel complete in v1?")).toBeVisible();
});

test("phase 37.1 session command rail collapses into a thread sheet below desktop widths", async ({ page }) => {
  await page.setViewportSize({ width: 840, height: 900 });
  await page.goto("/sessions/session-11?mockScenario=session-workspace");

  await expect(page.getByRole("button", { name: /Threads.*Workflow/ })).toBeVisible();
  await expect(page.locator(".session-question-shell > .session-question-rail")).toHaveCount(0);

  await page.getByRole("button", { name: /Threads.*Workflow/ }).click();
  await expect(page.getByRole("dialog", { name: "Session threads" })).toBeVisible();
  await expect(page.getByRole("button", { name: /Scope/ })).toBeVisible();

  await page.getByRole("button", { name: /Scope/ }).click();
  await expect(page.getByRole("dialog", { name: "Session threads" })).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Scope", exact: true })).toBeFocused();
  await expect(page.getByText("Define the release boundaries before delivery handoff.")).toBeVisible();

  await page.getByRole("button", { name: /Threads.*Scope/ }).click();
  await page.getByRole("button", { name: "Close" }).click();
  await expect(page.getByRole("dialog", { name: "Session threads" })).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Scope", exact: true })).toBeVisible();
});
