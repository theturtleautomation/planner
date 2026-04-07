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
    intake_phase: "waiting",
    project_description: "A local-first calendar app with task planning.",
    project_id: "project-1",
    project_slug: "personal-calendar",
    project_name: "Personal Calendar",
    current_step: null,
    error_message: null,
    can_resume_live: false,
    can_resume_checkpoint: false,
    can_restart_from_description: true,
    can_retry_pipeline: false,
    has_checkpoint: false,
    resume_status: "ready_to_start",
    workspace_status: {
      state: "ready_to_start",
      label: "Ready to start analysis",
      detail: "Waiting for the session workspace to begin from the saved brief.",
      tone: "neutral",
    },
    ...overrides,
  };
}

function promptBank(sessionId) {
  return {
    session_id: sessionId,
    active_thread_id: "workflow",
    banked_threads: [
      {
        category_id: "workflow",
        title: "Workflow",
        summary: "Confirm the main flow",
        question_count: 1,
        prompt: {
          prompt_id: "prompt-1",
          title: "Workflow",
          kind: "question_batch",
          items: [],
          allow_partial_submit: true,
        },
      },
    ],
    queued_threads: [],
    build_ready: false,
    build_readiness_message: null,
    initial_bank_complete: true,
  };
}

async function mockWorkEntry(page, sessions) {
  const projects = [project()];

  await page.route("**/api/projects", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({ projects }),
    });
  });

  await page.route("**/api/sessions", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({ sessions }),
    });
  });
}

async function mockProjectWorkspace(page, sessions) {
  const currentProject = project();

  await mockWorkEntry(page, sessions);

  await page.route("**/api/projects/personal-calendar", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({ project: currentProject }),
    });
  });

  await page.route("**/api/blueprint?*", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        total_nodes: 2,
        total_edges: 1,
        counts: {
          project: 1,
          decision: 1,
        },
        nodes: [
          {
            id: "project-1",
            name: "Personal Calendar",
            node_type: "project",
            status: "active",
            scope_class: "project",
            scope_visibility: "project_local",
            is_shared: false,
            lifecycle: "active",
            project_id: "project-1",
            project_name: "Personal Calendar",
            secondary_scope: {},
            linked_project_ids: [],
            tags: [],
            has_documentation: true,
            updated_at: "2026-03-26T03:00:00Z",
          },
          {
            id: "decision-1",
            name: "Prompt-bank first",
            node_type: "decision",
            status: "accepted",
            scope_class: "project",
            scope_visibility: "project_local",
            is_shared: false,
            lifecycle: "active",
            project_id: "project-1",
            project_name: "Personal Calendar",
            secondary_scope: {},
            linked_project_ids: [],
            tags: [],
            has_documentation: true,
            updated_at: "2026-03-26T03:00:00Z",
          },
        ],
        edges: [{ source: "project-1", target: "decision-1", edge_type: "contains" }],
      }),
    });
  });

  await page.route("**/api/projects/personal-calendar/import-state", async route => {
    await route.fulfill({ status: 404, contentType: "application/json", body: "null" });
  });

  await page.route("**/api/projects/personal-calendar/import-review", async route => {
    await route.fulfill({ status: 404, contentType: "application/json", body: "null" });
  });

  await page.route("**/api/sessions/session-1/prompt-bank", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(promptBank("session-1")),
    });
  });

  await page.route("**/api/sessions/session-1/runs", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({ runs: [] }),
    });
  });

  await page.route("**/api/sessions/session-1/events?*", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        session_id: "session-1",
        events: [],
        count: 0,
      }),
    });
  });

  await page.route("**/api/blueprint/export-history?*", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        entries: [],
        total: 0,
      }),
    });
  });
}

test("phase 29 keeps project-first entry as the only visible new-work path", async ({ page }) => {
  await mockWorkEntry(page, [
    session(),
  ]);

  await page.goto("/");
  await expect(page.getByRole("link", { name: "Open project" })).toBeVisible();
  await expect(page.getByRole("link", { name: /direct session/i }).first()).toHaveCount(0);

  await page.goto("/sessions");
  await expect(page.getByRole("link", { name: "New project" })).toBeVisible();
  await expect(page.getByRole("link", { name: "Direct session" })).toHaveCount(0);
  const firstRow = page.locator(".queue-row").first();
  await expect(firstRow).toContainText("Ready to start");
  await expect(firstRow).not.toContainText("waiting");
});

test("phase 29 removes raw phase language from project session summaries and activity copy", async ({ page }) => {
  await mockProjectWorkspace(page, [
    session({
      intake_phase: "interviewing",
      has_checkpoint: true,
      resume_status: "interview_checkpoint_resumable",
      workspace_status: {
        state: "awaiting_response",
        label: "Waiting for your response",
        detail: "A saved prompt checkpoint is ready to resume.",
        tone: "neutral",
      },
    }),
  ]);

  await page.goto("/projects/personal-calendar");
  const sessionRow = page.locator("a").filter({ hasText: "Calendar intake" }).first();
  await expect(sessionRow).toContainText("Waiting for response");
  await expect(sessionRow).not.toContainText("interviewing");

  await page.getByRole("button", { name: "Project review, readiness, and advanced surfaces" }).click();
  await page.getByRole("tab", { name: "Activity" }).click();
  await expect(page.getByText(/Waiting for response ·/)).toBeVisible();
  await expect(page.getByText(/interviewing ·/)).toHaveCount(0);
});

test("phase 29 gives standalone sessions an explicit return path", async ({ page }) => {
  await page.route("**/api/sessions/session-standalone", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        session: session({
          id: "session-standalone",
          title: "Solo session",
          project_id: null,
          project_slug: null,
          project_name: null,
          project_description: null,
          intake_phase: "complete",
          can_restart_from_description: false,
          workspace_status: {
            state: "complete",
            label: "Plan complete",
            detail: "The session is complete and ready for review.",
            tone: "success",
          },
        }),
      }),
    });
  });

  await page.route("**/api/sessions/session-standalone/prompt-bank", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        session_id: "session-standalone",
        active_thread_id: null,
        banked_threads: [],
        queued_threads: [],
        build_ready: false,
        build_readiness_message: null,
        initial_bank_complete: false,
      }),
    });
  });

  await page.goto("/sessions/session-standalone");
  await expect(page.getByRole("link", { name: "Back to sessions" })).toBeVisible();
  await expect(page.getByRole("link", { name: "Back to project" })).toHaveCount(0);
});

test("phase 29 keeps the return flow continuous from home into project work and back out", async ({ page, request }) => {
  const uniqueName = `Phase 29 Ops Console ${Date.now()}`;
  const createProjectResponse = await request.post("/api/projects", {
    data: {
      name: uniqueName,
      description: "Track service health, alerts, and deployment posture.",
    },
  });
  expect(createProjectResponse.ok()).toBeTruthy();
  const { project } = await createProjectResponse.json();

  await page.goto("/");
  await page.getByRole("link", { name: new RegExp(uniqueName) }).click();
  await expect(page).toHaveURL(`/projects/${project.slug}`);

  await page.getByRole("button", { name: "Start analysis" }).click();
  await expect(page).toHaveURL(/\/sessions\/[^/]+$/);
  await expect(page.locator(".session-interview-question")).toBeVisible({ timeout: 20_000 });

  await page.getByRole("link", { name: "Back to project" }).click();
  await expect(page).toHaveURL(`/projects/${project.slug}`);
});
