// @ts-nocheck
import { expect, test } from "@playwright/test";

const frontendMockOnly = process.env.VITE_PLANNER_FRONTEND_MOCK === "1";

test.skip(!frontendMockOnly, "phase 12 discovery route now runs against the frontend mock runtime");

test("phase 12 keeps discovery proposal triage primary while controls and context stay secondary", async ({ page }) => {
  await page.goto("/discovery?mockScenario=ops-attention");

  await expect(page.getByRole("heading", { name: "Discovery", exact: true }).first()).toBeVisible();
  await expect(page.getByRole("heading", { name: "Proposal queue" }).first()).toBeVisible();
  await expect(page.getByRole("heading", { name: "Selected proposal" }).first()).toBeVisible();
  await expect(page.getByText("Reminder engine").first()).toBeVisible();

  await page.getByRole("button", { name: "Run scan" }).first().click();
  await expect(page.locator(".page-summary-row .pill").last()).toContainText("pending");

  await page.getByRole("button", { name: "All" }).first().click();
  await expect(page.getByRole("heading", { name: "Pending review", exact: true }).first()).toBeVisible();

  await page.locator('[aria-label="Discovery proposal mode"]').last().getByRole("tab", { name: "Edge proposals" }).click();
  await expect(page.locator(".page-summary-note").last()).toContainText("Edge proposals are active.");
});
