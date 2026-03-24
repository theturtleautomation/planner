// @ts-nocheck
import { expect, test } from "@playwright/test";

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

test("phase 15 keeps import history, comparison, and restore project-local", async ({ page }) => {
  let currentStateStatus = "applied";

  const currentStateResponse = () => ({
    ...projectPayload,
    import_job: {
      id: currentStateStatus === "review_pending" ? "job-restored-review" : "job-current",
      project_id: "project-1",
      provider: "local",
      requested_ref: "/tmp/personal-calendar",
      status: currentStateStatus,
      restored_from_job_id: currentStateStatus === "review_pending" ? "job-old" : null,
      seed_session_id: "session-1",
      analysis_summary:
        currentStateStatus === "review_pending"
          ? "Historical review draft restored from import job-old."
          : "Latest applied import remains attached to this project.",
      progress_message:
        currentStateStatus === "review_pending"
          ? "Historical review draft restored from import job-old. Review and apply when ready."
          : "Import draft applied and reconciled against the canonical project blueprint.",
      error_message: null,
      created_at: "2026-03-24T04:00:00Z",
      updated_at: "2026-03-24T05:30:00Z",
    },
    source_binding: {
      project_id: "project-1",
      provider: "local",
      canonical_ref: "/tmp/personal-calendar",
      default_branch: null,
      head_revision: "deadbeef",
      local_root: "/tmp/personal-calendar",
      managed_checkout: false,
      created_at: "2026-03-24T04:00:00Z",
      updated_at: "2026-03-24T05:30:00Z",
    },
    import_draft:
      currentStateStatus === "review_pending"
        ? {
            job_id: "job-restored-review",
            project_id: "project-1",
            analysis_summary: "Historical review draft restored from import job-old.",
            source_metadata: {
              provider: "local",
              canonical_ref: "/tmp/personal-calendar",
              local_root: "/tmp/personal-calendar",
              default_branch: null,
              head_revision: "cafebabe",
            },
            discovered_nodes: [{ id: "node-restore-1" }],
            created_at: "2026-03-24T04:00:00Z",
            updated_at: "2026-03-24T05:30:00Z",
          }
        : null,
    import_review_selection:
      currentStateStatus === "review_pending"
        ? {
            job_id: "job-restored-review",
            excluded_node_ids: [],
            included_node_count: 1,
            excluded_node_count: 0,
          }
        : null,
    review_nodes:
      currentStateStatus === "review_pending"
        ? [{ node_id: "node-restore-1", node_name: "Task Service", node_type: "component", included: true }]
        : null,
  });

  const historyResponse = () => ({
    project: projectPayload.project,
    source_binding: currentStateResponse().source_binding,
    history: [
      {
        import_job: currentStateResponse().import_job,
        source_metadata: {
          provider: "local",
          canonical_ref: "/tmp/personal-calendar",
          local_root: "/tmp/personal-calendar",
          default_branch: null,
          head_revision: "deadbeef",
        },
        discovered_node_count: currentStateStatus === "review_pending" ? 1 : 2,
        effective_included_node_count: currentStateStatus === "review_pending" ? 1 : 1,
        effective_excluded_node_count: currentStateStatus === "review_pending" ? 0 : 1,
      },
      {
        import_job: {
          id: "job-old",
          project_id: "project-1",
          provider: "local",
          requested_ref: "/tmp/personal-calendar",
          status: "applied",
          restored_from_job_id: null,
          seed_session_id: "session-0",
          analysis_summary: "Older applied import with an earlier reminder model.",
          progress_message: "Import draft applied and reconciled against the canonical project blueprint.",
          error_message: null,
          created_at: "2026-03-23T22:00:00Z",
          updated_at: "2026-03-23T22:10:00Z",
        },
        source_metadata: {
          provider: "local",
          canonical_ref: "/tmp/personal-calendar",
          local_root: "/tmp/personal-calendar",
          default_branch: null,
          head_revision: "cafebabe",
        },
        discovered_node_count: 1,
        effective_included_node_count: 1,
        effective_excluded_node_count: 0,
      },
      {
        import_job: {
          id: "job-old-review",
          project_id: "project-1",
          provider: "local",
          requested_ref: "/tmp/personal-calendar",
          status: "review_pending",
          restored_from_job_id: null,
          seed_session_id: "session-old",
          analysis_summary: "Older review draft that never got applied.",
          progress_message: "Import draft ready. Review imported context in the seeded session.",
          error_message: null,
          created_at: "2026-03-23T20:00:00Z",
          updated_at: "2026-03-23T20:10:00Z",
        },
        source_metadata: {
          provider: "local",
          canonical_ref: "/tmp/personal-calendar",
          local_root: "/tmp/personal-calendar",
          default_branch: null,
          head_revision: "baddcafe",
        },
        discovered_node_count: 2,
        effective_included_node_count: 2,
        effective_excluded_node_count: 0,
      },
    ],
    diff_summary: {
      current_job_id: currentStateResponse().import_job.id,
      compared_to_job_id: "job-old",
      added_nodes: [{ node_id: "node-new", node_name: "Reminder Engine", node_type: "component" }],
      removed_nodes: [],
      added_node_types: [{ node_type: "component", count: 1 }],
      removed_node_types: [],
      current_head_revision: "deadbeef",
      compared_head_revision: "cafebabe",
    },
  });

  await page.route("**/api/projects/personal-calendar", async route => {
    await route.fulfill({ contentType: "application/json", body: JSON.stringify(projectPayload) });
  });

  await page.route("**/api/projects/personal-calendar/import-state", async route => {
    await route.fulfill({ contentType: "application/json", body: JSON.stringify(currentStateResponse()) });
  });

  await page.route("**/api/projects/personal-calendar/import-review", async route => {
    if (currentStateStatus !== "review_pending") {
      await route.fulfill({ status: 404, contentType: "application/json", body: JSON.stringify({ error: "not found" }) });
      return;
    }
    await route.fulfill({ contentType: "application/json", body: JSON.stringify(currentStateResponse()) });
  });

  await page.route("**/api/projects/personal-calendar/import-history", async route => {
    await route.fulfill({ contentType: "application/json", body: JSON.stringify(historyResponse()) });
  });

  await page.route("**/api/projects/personal-calendar/import-history/job-old/compare", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        project: projectPayload.project,
        source_binding: currentStateResponse().source_binding,
        selected_entry: historyResponse().history[1],
        current_import_job: currentStateResponse().import_job,
        selected_entry_uses_selection_filter: false,
        current_import_job_uses_selection_filter: true,
        diff_summary: {
          current_job_id: currentStateResponse().import_job.id,
          compared_to_job_id: "job-old",
          added_nodes: [{ node_id: "node-new", node_name: "Reminder Engine", node_type: "component" }],
          removed_nodes: [],
          added_node_types: [{ node_type: "component", count: 1 }],
          removed_node_types: [],
          current_head_revision: "deadbeef",
          compared_head_revision: "cafebabe",
        },
      }),
    });
  });

  await page.route("**/api/projects/personal-calendar/import-history/job-current/compare/job-old", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        project: projectPayload.project,
        source_binding: currentStateResponse().source_binding,
        baseline_entry: historyResponse().history[0],
        compared_entry: historyResponse().history[1],
        baseline_entry_uses_selection_filter: true,
        compared_entry_uses_selection_filter: false,
        diff_summary: {
          current_job_id: "job-old",
          compared_to_job_id: "job-current",
          added_nodes: [],
          removed_nodes: [{ node_id: "node-new", node_name: "Reminder Engine", node_type: "component" }],
          added_node_types: [],
          removed_node_types: [{ node_type: "component", count: 1 }],
          current_head_revision: "cafebabe",
          compared_head_revision: "deadbeef",
        },
      }),
    });
  });

  await page.route("**/api/projects/personal-calendar/import-history/job-old/restore-for-review", async route => {
    currentStateStatus = "review_pending";
    await route.fulfill({ contentType: "application/json", body: JSON.stringify(currentStateResponse()) });
  });

  await page.goto("/projects/personal-calendar/import");

  await expect(page.getByRole("heading", { name: "Import review", exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Historical restore and comparison" })).toBeVisible();
  await expect(page.getByText("Saved exclusions affect this job's effective apply footprint.")).toBeVisible();
  await expect(page.getByRole("button", { name: "Restore for review" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Restore draft for review" })).toBeVisible();

  await page.getByRole("button", { name: "Compare to current" }).first().click();
  await expect(page.getByText("Selected history entry vs current import")).toBeVisible();
  await expect(page.getByText("Current import comparison uses selected nodes from saved merge controls.")).toBeVisible();

  await page.getByRole("button", { name: "Use as baseline" }).first().click();
  await expect(page.getByRole("button", { name: "Baseline Selected" })).toBeVisible();
  await page.getByRole("button", { name: "Compare to selected" }).first().click();
  await expect(page.getByText("Selected history entries compared")).toBeVisible();
  await expect(page.getByText("Baseline history entry comparison uses selected nodes from saved merge controls.")).toBeVisible();

  await page.getByRole("button", { name: "Restore for review" }).click();
  await expect(page.locator(".success-copy")).toContainText(/Historical review draft restored from import job-old/i);
  await expect(page.locator("#import-history .advanced-metric-text").first()).toHaveText("Review pending");
});
