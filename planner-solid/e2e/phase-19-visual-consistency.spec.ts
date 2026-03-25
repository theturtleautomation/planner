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
      can_resume_live: false,
      can_resume_checkpoint: true,
      can_restart_from_description: true,
      can_retry_pipeline: false,
      has_checkpoint: true,
      resume_status: "interview_checkpoint_resumable",
    },
  ],
};

const blueprintPayload = {
  total_nodes: 4,
  total_edges: 2,
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

const eventsPayload = {
  events: [
    {
      event_type: "node_updated",
      timestamp: "2026-03-24T10:20:00Z",
      summary: "Decision node updated",
      data: { node_id: "decision-1", field: "status" },
    },
    {
      event_type: "edge_created",
      timestamp: "2026-03-24T09:10:00Z",
      summary: "Blueprint edge created",
      data: { source: "decision-1", target: "component-1" },
    },
  ],
};

async function fontSize(locator) {
  return locator.evaluate(element => Number.parseFloat(getComputedStyle(element).fontSize));
}

async function bounds(locator) {
  const box = await locator.boundingBox();
  if (!box) throw new Error("expected locator to have a bounding box");
  return box;
}

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

  await page.route("**/api/blueprint/events**", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(eventsPayload),
    });
  });

  await page.route("**/api/blueprint?**", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(blueprintPayload),
    });
  });
});

test("phase 19 desktop hierarchy distinguishes page, section, group, and row roles", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 1100 });

  await page.goto("/sessions");
  await expect(page.getByRole("heading", { name: "Current work queue" })).toBeVisible();

  const sessionsPageTitle = page.locator("h1.page-title");
  const sessionsSectionTitle = page.getByRole("heading", { name: "All sessions" });
  expect(await fontSize(sessionsPageTitle)).toBeGreaterThan(await fontSize(sessionsSectionTitle));

  const pageTitleBox = await bounds(sessionsPageTitle);
  const sectionTitleBox = await bounds(sessionsSectionTitle);
  expect(Math.abs(pageTitleBox.x - sectionTitleBox.x)).toBeLessThan(3);

  await page.goto("/projects");
  await expect(page.getByRole("heading", { name: "Active work directory" })).toBeVisible();
  await expect(page.locator(".project-row-link .project-row-facts").first()).toBeVisible();
  await expect(page.locator(".project-row-actions button").first()).toBeVisible();

  const projectRowGrammar = await page.locator(".project-row").first().evaluate(element => {
    const link = element.querySelector(".project-row-link");
    const facts = element.querySelector(".project-row-link .project-row-facts");
    const actions = element.querySelector(".project-row-actions");
    return Boolean(link && facts && actions && link.contains(facts) && !link.contains(actions));
  });
  expect(projectRowGrammar).toBe(true);

  await page.goto("/events");
  await expect(page.getByRole("heading", { name: "Events", exact: true })).toBeVisible();
  await expect(page.locator(".timeline-group-title").first()).toBeVisible();

  const eventsSectionTitle = page.getByRole("heading", { name: "Timeline workspace" });
  const groupTitle = page.locator(".timeline-group-title").first();
  expect(await fontSize(eventsSectionTitle)).toBeGreaterThan(await fontSize(groupTitle));
});

test("phase 19 mobile collapse keeps the shell and knowledge workspace readable", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });

  await page.goto("/knowledge");
  await expect(page.getByRole("heading", { name: "Knowledge", exact: true })).toBeVisible();

  const navBehavior = await page.locator(".app-nav").evaluate(element => {
    const style = getComputedStyle(element);
    return {
      overflowX: style.overflowX,
      flexWrap: style.flexWrap,
    };
  });
  expect(["auto", "scroll"]).toContain(navBehavior.overflowX);
  expect(navBehavior.flexWrap).toBe("nowrap");

  const toolbarLabels = page.locator(".knowledge-toolbar .timeline-limit-field");
  const firstToolbarBox = await bounds(toolbarLabels.nth(0));
  const secondToolbarBox = await bounds(toolbarLabels.nth(1));
  const thirdToolbarBox = await bounds(toolbarLabels.nth(2));
  expect(Math.abs(firstToolbarBox.x - secondToolbarBox.x)).toBeLessThan(3);
  expect(Math.abs(firstToolbarBox.x - thirdToolbarBox.x)).toBeLessThan(3);
  expect(secondToolbarBox.y).toBeGreaterThan(firstToolbarBox.y);
  expect(thirdToolbarBox.y).toBeGreaterThan(secondToolbarBox.y);

  const knowledgeListBox = await bounds(page.locator(".knowledge-list-panel"));
  const knowledgeDetailBox = await bounds(page.locator(".knowledge-detail-panel"));
  expect(Math.abs(knowledgeListBox.x - knowledgeDetailBox.x)).toBeLessThan(3);
  expect(knowledgeDetailBox.y).toBeGreaterThan(knowledgeListBox.y);
});

test("phase 19 mobile blueprint keeps the graph workspace stacked without breaking the canvas", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });

  await page.goto("/blueprint");
  await expect(page.getByRole("heading", { name: "Blueprint", exact: true })).toBeVisible();

  const canvasBox = await bounds(page.locator(".blueprint-canvas-panel"));
  const inspectorBox = await bounds(page.locator(".blueprint-inspector-panel"));
  expect(Math.abs(canvasBox.x - inspectorBox.x)).toBeLessThan(3);
  expect(inspectorBox.y).toBeGreaterThan(canvasBox.y);

  const svgMinWidth = await page.locator(".blueprint-svg").evaluate(element => getComputedStyle(element).minWidth);
  expect(svgMinWidth).toBe("640px");
});
