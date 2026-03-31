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
  await expect(page.getByText("Every banked question is available from the start.")).toBeVisible();

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
