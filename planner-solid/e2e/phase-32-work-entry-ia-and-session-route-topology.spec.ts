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

test("phase 32 keeps project-first entry primary while framing direct sessions as a detour", async ({ page }) => {
  await mockWorkEntry(page);

  await page.goto("/");
  await expect(page.locator(`a.btn.btn-primary[href="/projects/personal-calendar"]`)).toBeVisible();
  await expect(page.getByRole("link", { name: "Direct session" }).first()).toBeVisible();
  await expect(page.getByText(/focused detour/i)).toBeVisible();

  await page.goto("/sessions");
  await expect(page.getByRole("heading", { name: "Current work queue" })).toBeVisible();
  await expect(page.getByText(/primary container for ongoing work/i)).toBeVisible();
  await expect(page.getByRole("link", { name: "New project" })).toBeVisible();
  await expect(page.getByRole("link", { name: "Direct session" })).toBeVisible();
});

test("phase 32 frames /sessions/new as direct-session entry and preserves a project-first escape hatch", async ({ page }) => {
  await page.goto("/sessions/new");

  await expect(page.getByRole("heading", { name: "Start a focused direct session" })).toBeVisible();
  await expect(page.getByText(/Projects remain the primary home for ongoing work/i)).toBeVisible();
  await expect(page.getByRole("button", { name: "Start direct session" })).toBeVisible();
  await expect(page.getByRole("link", { name: "Start with a project instead" })).toBeVisible();

  await page.getByRole("link", { name: "Start with a project instead" }).click();
  await expect(page).toHaveURL("/projects/new");
  await expect(page.getByRole("heading", { name: "Create the primary container for the next analysis." })).toBeVisible();
});

test("phase 32 keeps direct-session entry usable without changing the retained route set", async ({ page }) => {
  await page.route("**/api/sessions", async route => {
    if (route.request().method() !== "POST") {
      await route.fallback();
      return;
    }

    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        session: session({
          id: "session-direct",
          title: "Direct session",
          project_id: null,
          project_slug: null,
          project_name: null,
          project_description: "One-off planning brief",
        }),
      }),
    });
  });

  await page.route("**/api/sessions/session-direct", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        session: session({
          id: "session-direct",
          title: "Direct session",
          project_id: null,
          project_slug: null,
          project_name: null,
          project_description: "One-off planning brief",
        }),
      }),
    });
  });

  await page.route("**/api/sessions/session-direct/prompt-bank", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        session_id: "session-direct",
        active_thread_id: null,
        banked_threads: [],
        queued_threads: [],
        build_ready: false,
        build_readiness_message: null,
        initial_bank_complete: false,
      }),
    });
  });

  await page.goto("/sessions/new");
  await page.locator("textarea").fill("One-off planning brief");
  await page.getByRole("button", { name: "Start direct session" }).click();

  await expect(page).toHaveURL("/sessions/session-direct");
  await expect(page.getByRole("link", { name: "Back to sessions" })).toBeVisible();
});
