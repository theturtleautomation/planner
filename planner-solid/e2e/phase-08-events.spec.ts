// @ts-nocheck
import { expect, test } from "@playwright/test";

const frontendMockOnly = process.env.VITE_PLANNER_FRONTEND_MOCK === "1";

test.skip(!frontendMockOnly, "phase 08 events route now runs against the frontend mock runtime");

test("phase 08 keeps the event stream primary while snapshots stay secondary", async ({ page }) => {
  await page.goto("/events?mockScenario=ops-history");

  await expect(page.getByRole("heading", { name: "Events", exact: true }).first()).toBeVisible();
  await expect(page.getByText("Recorded project export for Personal Calendar.").first()).toBeVisible();
  await expect(page.getByRole("button", { name: "Exports" }).first()).toBeVisible();

  await page.getByRole("button", { name: "Exports" }).first().click();
  await expect(page.getByText("Recorded project export for Personal Calendar.").first()).toBeVisible();

  await page.locator('[aria-label="Events route sections"]').last().getByRole("tab", { name: "Snapshots" }).click();
  await expect(page.locator(".page-summary-row .pill").last()).toHaveText("Snapshot history");
  await expect(page.getByRole("button", { name: "Create snapshot" }).last()).toBeVisible();
  await page.getByRole("button", { name: "Create snapshot" }).last().click();
  await expect(page.locator(".page-summary-note").last()).toContainText("4 saved snapshots");
});
