// @ts-nocheck
import { expect, test } from "@playwright/test";

const projectsPayload = {
  projects: [
    {
      id: "project-1",
      slug: "personal-calendar",
      name: "Personal Calendar",
      description: "Personal calendar app with task tracking",
      owner_user_id: "dev|local",
      team_label: null,
      created_at: "2026-03-24T00:00:00Z",
      updated_at: "2026-03-24T03:12:00Z",
      archived_at: null,
      legacy_scope_keys: [],
    },
  ],
};

const sessionsPayload = {
  sessions: [
    {
      id: "session-1",
      title: "Calendar intake",
      archived: false,
      created_at: "2026-03-24T00:00:00Z",
      last_activity_at: "2026-03-24T03:12:00Z",
      pipeline_running: false,
      intake_phase: "interviewing",
      project_description: "Personal calendar app with task tracking",
      project_id: "project-1",
      project_slug: "personal-calendar",
      project_name: "Personal Calendar",
      current_step: "socratic.question.generated",
      error_message: null,
    },
  ],
};

test.beforeEach(async ({ page }) => {
  await page.route("**/api/projects", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(projectsPayload),
    });
  });

  await page.route("**/api/sessions", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(sessionsPayload),
    });
  });
});

test("phase 13 shell copy and fallback routes reflect the active Solid workspace", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByText("Local-first project analysis and build workspace")).toBeVisible();
  await expect(page.getByRole("link", { name: "Blueprint" })).toBeVisible();
  await expect(page.getByRole("link", { name: "Discovery" })).toBeVisible();

  await page.goto("/missing-route");
  await expect(page.getByRole("heading", { name: "Route not found" })).toBeVisible();
  await expect(page.getByText(/outside the current Planner workspace/i)).toBeVisible();
  await expect(page.getByRole("link", { name: "Open projects" })).toBeVisible();
});
