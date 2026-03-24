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

test("phase 14 keeps project import review local and decision-first", async ({ page }) => {
  let included = true;
  let importStatus = "review_pending";

  const reviewResponse = () => ({
    ...projectPayload,
    import_job: {
      id: "import-job-1",
      project_id: "project-1",
      provider: "local",
      requested_ref: "/tmp/personal-calendar",
      status: importStatus,
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
    import_review_selection: {
      job_id: "import-job-1",
      excluded_node_ids: included ? [] : ["node-1"],
      included_node_count: included ? 1 : 0,
      excluded_node_count: included ? 0 : 1,
    },
    review_nodes:
      importStatus === "review_pending"
        ? [
            {
              node_id: "node-1",
              node_name: "Task Service",
              node_type: "component",
              included,
            },
          ]
        : [],
  });

  await page.route("**/api/projects/personal-calendar", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(projectPayload),
    });
  });

  await page.route("**/api/projects/personal-calendar/import-review-selection", async route => {
    included = !included;
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(reviewResponse()),
    });
  });

  await page.route("**/api/projects/personal-calendar/import-review", async route => {
    if (route.request().method() === "POST") {
      importStatus = "applied";
    }
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(reviewResponse()),
    });
  });

  await page.route("**/api/projects/personal-calendar/import-state", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(reviewResponse()),
    });
  });

  await page.goto("/projects/personal-calendar/import");

  await expect(page.getByRole("heading", { name: "Import review", exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Imported nodes" })).toBeVisible();
  await expect(page.getByText("Task Service")).toBeVisible();

  await page.getByRole("button", { name: "Exclude" }).click();
  await expect(page.getByRole("button", { name: "Include" })).toBeVisible();

  await page.getByRole("button", { name: "Apply import review" }).click();
  await expect(page.getByText(/No pending import node decisions remain/)).toBeVisible();
});
