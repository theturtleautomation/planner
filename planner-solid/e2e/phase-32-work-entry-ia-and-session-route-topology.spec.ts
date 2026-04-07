// @ts-nocheck
import { expect, test } from "@playwright/test";

function project(overrides = {}) {
  return {
    id: "project-1",
    slug: "personal-calendar",
    name: "Personal Calendar",
    description: "A local-first calendar app with task planning.",
    owner_user_id: "dev|local",
    team_label: null,
    created_at: "2026-03-26T00:00:00Z",
    updated_at: "2026-03-26T03:00:00Z",
    archived_at: null,
    legacy_scope_keys: [],
    ...overrides,
  };
}

function session(overrides = {}) {
  return {
    id: "session-1",
    title: "Calendar intake",
    archived: false,
    created_at: "2026-03-26T00:00:00Z",
    last_activity_at: "2026-03-26T03:30:00Z",
    pipeline_running: false,
    intake_phase: "complete",
    project_description: "A local-first calendar app with task planning.",
    project_id: "project-1",
    project_slug: "personal-calendar",
    project_name: "Personal Calendar",
    current_step: null,
    error_message: null,
    can_resume_live: false,
    can_resume_checkpoint: false,
    can_restart_from_description: false,
    can_retry_pipeline: false,
    has_checkpoint: false,
    resume_status: "complete",
    workspace_status: {
      state: "complete",
      label: "Plan complete",
      detail: "The session is ready for review.",
      tone: "success",
    },
    ...overrides,
  };
}

async function mockWorkEntry(page) {
  await page.route("**/api/projects", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({ projects: [project()] }),
    });
  });

  await page.route("**/api/sessions", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({ sessions: [session()] }),
    });
  });
}

test("phase 32 keeps project-first entry as the only visible new-work path", async ({ page }) => {
  await mockWorkEntry(page);

  await page.goto("/");
  await expect(page.locator(`a.btn.btn-primary[href="/projects/personal-calendar"]`)).toBeVisible();
  await expect(page.getByRole("link", { name: "Direct session" }).first()).toHaveCount(0);

  await page.goto("/sessions");
  await expect(page.getByRole("heading", { name: "Current work queue" })).toBeVisible();
  await expect(page.getByText(/new work should always start from a project/i)).toBeVisible();
  await expect(page.getByRole("link", { name: "New project" })).toBeVisible();
  await expect(page.getByRole("link", { name: "Direct session" })).toHaveCount(0);
});

test("phase 32 redirects /sessions/new into project creation", async ({ page }) => {
  await page.goto("/sessions/new");

  await expect(page).toHaveURL("/projects/new");
  await expect(page.getByText(/projects are required for all new work/i)).toHaveCount(0);
});

test("phase 32 no longer offers projectless direct-session creation", async ({ page }) => {
  await page.goto("/sessions/new");
  await expect(page).toHaveURL("/projects/new");
  await expect(page.getByRole("button", { name: /create project/i })).toBeVisible();
});
