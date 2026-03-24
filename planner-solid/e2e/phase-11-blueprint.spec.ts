// @ts-nocheck
import { expect, test } from "@playwright/test";

const projectsPayload = {
  projects: [
    {
      id: "project-1",
      slug: "personal-calendar",
      name: "Personal Calendar",
      description: "Calendar app",
      owner_user_id: "dev|local",
      team_label: null,
      created_at: "2026-03-24T00:00:00Z",
      updated_at: "2026-03-24T03:12:00Z",
      archived_at: null,
      legacy_scope_keys: [],
    },
  ],
};

const blueprintPayload = {
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
    { source: "decision-1", target: "pattern-1", edge_type: "uses" },
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
      tags: ["anchor"],
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
      tags: ["frontend"],
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
      updated_at: "2026-03-24T04:30:00Z",
    },
    {
      id: "pattern-1",
      name: "Reminder cadence",
      node_type: "pattern",
      status: "candidate",
      scope_class: "project",
      scope_visibility: "project_local",
      is_shared: false,
      lifecycle: "active",
      project_id: "project-1",
      project_name: "Personal Calendar",
      secondary_scope: {},
      linked_project_ids: [],
      tags: [],
      has_documentation: false,
      updated_at: "2026-03-24T03:00:00Z",
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

  await page.route("**/api/blueprint?**", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(blueprintPayload),
    });
  });
});

test("phase 11 keeps the blueprint graph primary with attached node inspection", async ({ page }) => {
  await page.goto("/blueprint");

  await expect(page.getByRole("heading", { name: "Blueprint", exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Graph canvas" })).toBeVisible();
  await expect(page.getByTestId("blueprint-graph-canvas")).toBeVisible();
  await expect(page.getByRole("heading", { name: "Selected node" })).toBeVisible();

  await page.locator('[data-node-id="component-1"]').click();
  await expect(page.getByText("Task Service").last()).toBeVisible();
  await expect(page.getByText("Missing")).toBeVisible();

  await page.getByLabel("Node type").selectOption("decision");
  await expect(page.locator('[data-node-id="decision-1"]')).toBeVisible();
  await expect(page.locator('[data-node-id="component-1"]')).toHaveCount(0);
});
