// @ts-nocheck
import { expect, test } from "@playwright/test";

const sessionList = {
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
    {
      id: "session-2",
      title: "Finance ideas",
      archived: false,
      created_at: "2026-03-24T00:00:00Z",
      last_activity_at: "2026-03-24T02:00:00Z",
      pipeline_running: false,
      intake_phase: "complete",
      project_description: "Household finance dashboard",
      project_id: "project-2",
      project_slug: "household-finance",
      project_name: "Household Finance",
      current_step: null,
      error_message: null,
    },
  ],
};

const projectsList = {
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
    {
      id: "project-2",
      slug: "household-finance",
      name: "Household Finance",
      description: "Shared family finance planner",
      owner_user_id: "dev|local",
      team_label: null,
      created_at: "2026-03-24T00:00:00Z",
      updated_at: "2026-03-24T02:00:00Z",
      archived_at: null,
      legacy_scope_keys: ["finance-app"],
    },
  ],
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
};

const projectBlueprint = {
  total_nodes: 4,
  total_edges: 3,
  counts: {
    project: 1,
    decision: 1,
    component: 1,
    pattern: 1,
  },
  edges: [
    { source: "project-1", target: "decision-1", edge_type: "contains" },
    { source: "decision-1", target: "component-1", edge_type: "decided_by" },
  ],
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
      updated_at: "2026-03-24T04:00:00Z",
    },
    {
      id: "decision-1",
      name: "Web first",
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
      updated_at: "2026-03-24T05:00:00Z",
    },
    {
      id: "component-1",
      name: "Task Service",
      node_type: "component",
      status: "planned",
      scope_class: "project_contextual",
      scope_visibility: "shared",
      is_shared: true,
      lifecycle: "active",
      project_id: "project-1",
      project_name: "Personal Calendar",
      secondary_scope: { component: "Task Service" },
      linked_project_ids: [],
      tags: [],
      has_documentation: false,
      updated_at: "2026-02-20T05:00:00Z",
    },
  ],
};

const importReview = {
  project: projectsList.projects[0],
  import_job: {
    id: "import-job-1",
    project_id: "project-1",
    provider: "local",
    requested_ref: "/tmp/personal-calendar",
    status: "review_pending",
    restored_from_job_id: null,
    seed_session_id: "session-1",
    analysis_summary: "Import draft captured new task and reminder entities.",
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
    analysis_summary: "Import draft captured new task and reminder entities.",
    source_metadata: {
      provider: "local",
      canonical_ref: "/tmp/personal-calendar",
      local_root: "/tmp/personal-calendar",
      default_branch: null,
      head_revision: null,
    },
    discovered_nodes: [{ id: "node-1" }, { id: "node-2" }],
    created_at: "2026-03-24T04:00:00Z",
    updated_at: "2026-03-24T05:30:00Z",
  },
  import_review_selection: {
    job_id: "import-job-1",
    excluded_node_ids: ["node-2"],
    included_node_count: 1,
    excluded_node_count: 1,
  },
  review_nodes: [
    { node_id: "node-1", node_name: "Task Service", node_type: "component", included: true },
    { node_id: "node-2", node_name: "Reminder Engine", node_type: "component", included: false },
  ],
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

  await page.route("**/api/projects", async route => {
    if (route.request().method() === "POST") {
      await route.fulfill({
        status: 201,
        contentType: "application/json",
        body: JSON.stringify({
          project: {
            id: "project-3",
            slug: "new-idea",
            name: "New Idea",
            description: "Fresh automation concept",
            owner_user_id: "dev|local",
            team_label: null,
            created_at: "2026-03-24T04:00:00Z",
            updated_at: "2026-03-24T04:00:00Z",
            archived_at: null,
            legacy_scope_keys: [],
          },
        }),
      });
      return;
    }

    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(projectsList),
    });
  });

  await page.route("**/api/projects/personal-calendar", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({ project: projectsList.projects[0] }),
    });
  });

  await page.route("**/api/projects/new-idea", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        project: {
          id: "project-3",
          slug: "new-idea",
          name: "New Idea",
          description: "Fresh automation concept",
          owner_user_id: "dev|local",
          team_label: null,
          created_at: "2026-03-24T04:00:00Z",
          updated_at: "2026-03-24T04:00:00Z",
          archived_at: null,
          legacy_scope_keys: [],
        },
      }),
    });
  });

  await page.route("**/api/projects/personal-calendar/sessions", async route => {
    if (route.request().method() === "POST") {
      await route.fulfill({
        status: 201,
        contentType: "application/json",
        body: JSON.stringify({
          session: {
            ...sessionList.sessions[0],
            id: "session-3",
            title: "Personal Calendar analysis",
          },
        }),
      });
      return;
    }

    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({ sessions: [sessionList.sessions[0]] }),
    });
  });

  await page.route("**/api/projects/new-idea/sessions", async route => {
    if (route.request().method() === "POST") {
      await route.fulfill({
        status: 201,
        contentType: "application/json",
        body: JSON.stringify({
          session: {
            ...sessionList.sessions[0],
            id: "session-4",
            title: "New Idea analysis",
            project_id: "project-3",
            project_slug: "new-idea",
            project_name: "New Idea",
            project_description: "Fresh automation concept",
          },
        }),
      });
      return;
    }

    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({ sessions: [] }),
    });
  });

  await page.route("**/api/sessions", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(sessionList),
    });
  });

  await page.route("**/api/sessions/session-1", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({ session: sessionList.sessions[0] }),
    });
  });

  await page.route("**/api/sessions/session-1/prompt-bank", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(promptBank),
    });
  });

  await page.route("**/api/sessions/session-1/runs", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        runs: ["run-12345678", "run-previous"],
      }),
    });
  });

  await page.route("**/api/sessions/session-1/events?**", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        session_id: "session-1",
        count: 2,
        events: [
          {
            id: "event-2",
            timestamp: "2026-03-24T05:10:00Z",
            level: "info",
            source: "pipeline",
            session_id: "session-1",
            step: "pipeline.compile",
            message: "Compiled project blueprint",
            metadata: {},
          },
          {
            id: "event-1",
            timestamp: "2026-03-24T05:06:00Z",
            level: "warn",
            source: "pipeline",
            session_id: "session-1",
            step: "pipeline.retry.started",
            message: "Retrying validation loop",
            metadata: {},
          },
        ],
      }),
    });
  });

  await page.route("**/api/blueprint?**", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(projectBlueprint),
    });
  });

  await page.route("**/api/projects/personal-calendar/import-review", async route => {
    if (route.request().method() === "POST") {
      await route.fulfill({
        contentType: "application/json",
        body: JSON.stringify({
          ...importReview,
          import_job: {
            ...importReview.import_job,
            status: "applied",
          },
        }),
      });
      return;
    }

    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(importReview),
    });
  });

  await page.route("**/api/projects/personal-calendar/import-review-selection", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(importReview),
    });
  });

  await page.route("**/api/projects/personal-calendar/import-state", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(importReview),
    });
  });

  await page.route("**/api/projects/new-idea/import-review", async route => {
    await route.fulfill({ status: 404, contentType: "application/json", body: JSON.stringify({ error: "not found" }) });
  });

  await page.route("**/api/projects/new-idea/import-state", async route => {
    await route.fulfill({ status: 404, contentType: "application/json", body: JSON.stringify({ error: "not found" }) });
  });
});

test("phase 01 centers guided work entry and project-first navigation", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByRole("heading", { name: /Open the project, continue the analysis/i })).toBeVisible();
  await expect(page.locator(".hero-focus-label")).toHaveText("Active Socratic analysis");

  await page.getByRole("link", { name: "Continue analysis" }).first().click();
  await expect(page.getByRole("heading", { name: "Personal Calendar" })).toBeVisible();
  await expect(page.getByRole("link", { name: "Continue analysis" })).toBeVisible();

  await page.getByRole("link", { name: "Continue analysis" }).click();
  await expect(page).toHaveURL(/\/sessions\/session-1$/);
  await expect(page.getByRole("heading", { name: "Verify Platform" })).toBeVisible();
  await expect(page.getByText("Should this ship as a web app first?")).toBeVisible();
});

test("phase 02 keeps advanced items hidden while attached knowledge and blueprint stay local", async ({ page }) => {
  await page.goto("/projects");
  await expect(page.getByRole("heading", { name: "Active work directory" })).toBeVisible();
  await expect(page.getByRole("link", { name: "Personal Calendar" })).toBeVisible();

  await page.getByRole("link", { name: "New project" }).click();
  await page.getByLabel("Project name").fill("New Idea");
  await page.getByLabel("What are you shaping?").fill("Fresh automation concept");
  await page.getByRole("button", { name: "Create project" }).click();

  await expect(page.getByRole("heading", { name: "New Idea" })).toBeVisible();
  await expect(page.getByText("No active analysis yet")).toBeVisible();
  await expect(page.getByText("Project review, readiness, and advanced surfaces")).toBeVisible();
  await expect(page.getByRole("tab", { name: "Knowledge" })).not.toBeVisible();

  await page.getByText("Project review, readiness, and advanced surfaces").click();
  await expect(page.getByRole("tab", { name: "Review" })).toBeVisible();
  await page.getByRole("tab", { name: "Review" }).click();
  await expect(page.getByText("No active review queue")).toBeVisible();

  await page.getByRole("tab", { name: "Knowledge" }).click();
  await expect(page.getByRole("tab", { name: "Knowledge" })).toBeVisible();
  await expect(page.getByText("Knowledge records")).toBeVisible();
  await expect(page.getByText("Task Service")).toBeVisible();

  await page.getByRole("tab", { name: "Blueprint" }).click();
  await expect(page.getByText("Edges")).toBeVisible();
  await expect(page.getByText("Web first")).toBeVisible();
});

test("phase 03 keeps review and build readiness attached to the project workspace", async ({ page }) => {
  await page.goto("/projects/personal-calendar");

  await expect(page.getByRole("heading", { name: "Personal Calendar" })).toBeVisible();
  await expect(page.getByText("Project review, readiness, and advanced surfaces")).toBeVisible();
  await expect(page.getByRole("tab", { name: "Review" })).not.toBeVisible();

  await page.getByText("Project review, readiness, and advanced surfaces").click();
  await expect(page.getByRole("tab", { name: "Review" })).toBeVisible();
  await page.getByRole("tab", { name: "Review" }).click();
  await expect(page.getByRole("heading", { name: "Import draft needs review" })).toBeVisible();
  await expect(page.getByText("Pending review")).toBeVisible();
  await expect(page.getByText("Reminder Engine")).toBeVisible();
  await expect(page.getByRole("button", { name: "Apply import review" })).toBeVisible();

  await page.getByRole("tab", { name: "Build readiness" }).click();
  await expect(page.getByRole("heading", { name: "A review gate is still blocking build readiness" })).toBeVisible();
  await expect(page.getByText("Needs review").first()).toBeVisible();
  await expect(page.getByText(/Import draft still needs merge review/i)).toBeVisible();

  await page.getByRole("tab", { name: "Build path" }).click();
  await expect(page.getByRole("heading", { name: "Build handoff is blocked by unresolved review work" })).toBeVisible();
  await expect(
    page.getByText(/Resolve the project review queue before handing this project to the automated build path/i).first(),
  ).toBeVisible();

  await page.getByRole("tab", { name: "Activity" }).click();
  await expect(page.getByRole("heading", { name: "Recent project activity" })).toBeVisible();
  await expect(page.getByText("Calendar intake").first()).toBeVisible();

  await page.getByRole("tab", { name: "Build execution" }).click();
  await expect(page.getByRole("heading", { name: "The latest build-facing run is no longer active" })).toBeVisible();
  await expect(page.getByText("run-1234")).toBeVisible();
  await expect(page.getByText("Compiled project blueprint")).toBeVisible();
});
