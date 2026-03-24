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
  total_nodes: 3,
  total_edges: 2,
  counts: {
    project: 1,
    decision: 1,
    component: 1,
  },
  edges: [
    { source: "project-1", target: "decision-1", edge_type: "contains" },
    { source: "decision-1", target: "component-1", edge_type: "decided_by" },
  ],
  nodes: [
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
      updated_at: "2026-03-24T04:00:00Z",
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

test("phase 10 keeps knowledge inventory primary with attached node detail", async ({ page }) => {
  await page.goto("/knowledge");

  await expect(page.getByRole("heading", { name: "Knowledge", exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Nodes" })).toBeVisible();
  await expect(page.getByRole("button", { name: /Web first/ })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Selected node" })).toBeVisible();

  await page.getByRole("button", { name: /Task Service/ }).click();
  await expect(page.getByText("Task Service").last()).toBeVisible();
  await page.getByPlaceholder("Find by name or type").fill("decision");
  await expect(page.getByText("Task Service").first()).not.toBeVisible();
});
