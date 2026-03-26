// @ts-nocheck
import { expect, test } from "@playwright/test";

const baseSession = {
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
  can_resume_live: false,
  can_resume_checkpoint: true,
  can_restart_from_description: true,
  can_retry_pipeline: true,
  has_checkpoint: true,
  resume_status: "interview_checkpoint_resumable",
};

const promptBank = {
  session_id: "session-1",
  active_thread_id: "verify-platform",
  banked_threads: [
    {
      category_id: "verify-platform",
      title: "Verify Platform",
      summary: "Confirm the delivery surface.",
      question_count: 1,
      prompt: {
        prompt_id: "prompt-1",
        title: "Verify Platform",
        kind: "verification_batch",
        origin_category_id: "verify-platform",
        items: [
          {
            item_id: "item-1",
            kind: "verification",
            text: "Should this ship as a web app first?",
            options: [{ option_id: "web", label: "Web app", semantic_value: "Web app" }],
            required: true,
          },
        ],
        allow_partial_submit: true,
      },
    },
  ],
  queued_threads: [],
  build_ready: false,
  build_readiness_message: null,
  initial_bank_complete: true,
};

const projectPayload = {
  project: {
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
};

const importState = {
  project: projectPayload.project,
  import_job: {
    id: "import-job-1",
    project_id: "project-1",
    provider: "local",
    requested_ref: "/tmp/personal-calendar",
    status: "applied",
    restored_from_job_id: null,
    seed_session_id: "session-1",
    analysis_summary: "Latest imported structure is available for review.",
    progress_message: null,
    error_message: null,
    created_at: "2026-03-24T04:00:00Z",
    updated_at: "2026-03-24T05:30:00Z",
  },
  source_binding: {
    project_id: "project-1",
    provider: "local",
    canonical_ref: "/tmp/personal-calendar",
    default_branch: null,
    head_revision: null,
    local_root: "/tmp/personal-calendar",
    managed_checkout: false,
    created_at: "2026-03-24T04:00:00Z",
    updated_at: "2026-03-24T05:30:00Z",
  },
  import_draft: {
    job_id: "import-job-1",
    project_id: "project-1",
    analysis_summary: "Latest imported structure is available for review.",
    source_metadata: {
      provider: "local",
      canonical_ref: "/tmp/personal-calendar",
      local_root: "/tmp/personal-calendar",
      default_branch: null,
      head_revision: null,
    },
    discovered_nodes: [{ id: "node-1" }],
    created_at: "2026-03-24T04:00:00Z",
    updated_at: "2026-03-24T05:30:00Z",
  },
  import_review_selection: {
    job_id: "import-job-1",
    excluded_node_ids: [],
    included_node_count: 1,
    excluded_node_count: 0,
  },
  review_nodes: [],
};

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    class MockWebSocket {
      static OPEN = 1;
      readyState = 1;
      onopen = null;
      onclose = null;
      onerror = null;
      onmessage = null;

      constructor(_url) {
        setTimeout(() => this.onopen?.(), 0);
      }

      send(_payload) {}

      close() {
        this.onclose?.();
      }
    }

    window.WebSocket = MockWebSocket;
  });

  await page.route("**/api/sessions/session-1", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({ session: baseSession }),
    });
  });

  await page.route("**/api/sessions/session-1/prompt-bank", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(promptBank),
    });
  });

  await page.route("**/api/sessions/session-1/duplicate", async route => {
    await route.fulfill({
      status: 201,
      contentType: "application/json",
      body: JSON.stringify({
        session: {
          ...baseSession,
          id: "session-copy",
          title: "Calendar intake copy",
        },
      }),
    });
  });

  await page.route("**/api/sessions/session-copy", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        session: {
          ...baseSession,
          id: "session-copy",
          title: "Calendar intake copy",
        },
      }),
    });
  });

  await page.route("**/api/sessions/session-copy/prompt-bank", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        ...promptBank,
        session_id: "session-copy",
      }),
    });
  });

  await page.route("**/api/sessions/session-1/export", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        exported_at: "2026-03-24T06:00:00Z",
        session: {
          ...baseSession,
          messages: [{ id: "m-1", role: "planner", content: "Exported" }],
        },
      }),
    });
  });

  await page.route("**/api/sessions/session-1/restart-from-description", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        session: {
          ...baseSession,
          current_step: "socratic.workspace.generated",
        },
      }),
    });
  });

  await page.route("**/api/sessions/session-1/retry-pipeline", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        session: {
          ...baseSession,
          intake_phase: "pipeline_running",
          pipeline_running: true,
          current_step: "pipeline.retry.started",
        },
      }),
    });
  });

  await page.route("**/api/projects/personal-calendar", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(projectPayload),
    });
  });

  await page.route("**/api/projects/personal-calendar/import-state", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(importState),
    });
  });

  await page.route("**/api/projects/personal-calendar/import-review", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(importState),
    });
  });

  await page.route("**/api/projects/personal-calendar/import-history", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        project: projectPayload.project,
        current_import_job: importState.import_job,
        source_binding: importState.source_binding,
        history: [
          {
            import_job: {
              ...importState.import_job,
              id: "import-job-0",
              status: "applied",
            },
            source_metadata: importState.import_draft.source_metadata,
            discovered_node_count: 1,
            effective_included_node_count: 1,
            effective_excluded_node_count: 0,
          },
        ],
      }),
    });
  });

  await page.route("**/api/projects/personal-calendar/reimport", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        ...importState,
        import_job: {
          ...importState.import_job,
          id: "import-job-2",
          status: "review_pending",
          progress_message: "Re-import request queued",
        },
      }),
    });
  });
});

test("phase 17 closes the workflow loop and keeps Solid as the active surface", async ({ page }) => {
  await page.goto("/sessions/session-1");

  await expect(page.getByRole("button", { name: "Duplicate" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Export" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Restart" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Retry pipeline" })).toBeVisible();

  await page.getByRole("button", { name: "Export" }).click();
  await expect(page.getByText(/Exported .*\.json/)).toBeVisible();

  await page.getByRole("button", { name: "Restart" }).click();
  await expect(page.getByText("Session reset to the original description.")).toBeVisible();

  await page.getByRole("button", { name: "Retry pipeline" }).click();
  await expect(page.getByText("Pipeline retry started.")).toBeVisible();

  await page.getByRole("button", { name: "Duplicate" }).click();
  await expect(page).toHaveURL(/\/sessions\/session-copy$/);
  await expect(page.getByRole("heading", { name: "Calendar intake copy" })).toBeVisible();

  await page.getByRole("link", { name: "Back to project" }).click();
  await expect(page).toHaveURL(/\/projects\/personal-calendar$/);

  await page.getByRole("button", { name: "Project review, readiness, and advanced surfaces" }).click();
  await expect(page).toHaveURL(/\/projects\/personal-calendar\?tab=review$/);
  await page.getByRole("tab", { name: "Review" }).click();
  await page.getByRole("button", { name: "Start re-import" }).click();
  await expect(page.getByRole("link", { name: "Open import review" })).toBeVisible();

  await page.getByRole("link", { name: "Open import review" }).click();
  await expect(page).toHaveURL(/\/projects\/personal-calendar\/import$/);
  await page.getByRole("button", { name: "Start re-import" }).click();
  await expect(page.getByText("Re-import request queued")).toBeVisible();
});
