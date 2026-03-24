// @ts-nocheck
import { expect, test } from "@playwright/test";

const nodeProposals = [
  {
    id: "proposal-1",
    node: {
      id: "component-1",
      node_type: "component",
      name: "Task Service",
      scope: {
        project: { project_id: "project-1", project_name: "Personal Calendar" },
        secondary: { component: "Task Service" },
      },
    },
    source: "directory_scan",
    reason: "Detected a task-focused service layer in the repo structure.",
    status: "pending",
    proposed_at: "2026-03-24T05:00:00Z",
    confidence: 0.84,
    source_artifact: "src/task-service.ts",
  },
  {
    id: "proposal-2",
    node: {
      id: "pattern-1",
      node_type: "pattern",
      name: "Reminder cadence",
      scope: {
        project: { project_id: "project-1", project_name: "Personal Calendar" },
        secondary: {},
      },
    },
    source: "pipeline_run",
    reason: "Reminder scheduling shows up as a recurring workflow concern.",
    status: "accepted",
    proposed_at: "2026-03-24T04:00:00Z",
    reviewed_at: "2026-03-24T04:20:00Z",
    confidence: 0.65,
    source_artifact: "planner pipeline",
  },
];

const edgeProposals = [
  {
    id: "edge-proposal-1",
    edge: {
      source: "decision-1",
      target: "component-1",
      edge_type: "decided_by",
    },
    source: "code_graph_context",
    reason: "The delivery decision appears to shape the task service boundary.",
    status: "pending",
    proposed_at: "2026-03-24T05:10:00Z",
    confidence: 0.72,
    source_artifact: "graph.json",
  },
];

test.beforeEach(async ({ page }) => {
  await page.route("**/api/blueprint/discovery/proposals*", async route => {
    const url = new URL(route.request().url());
    const status = url.searchParams.get("status");
    const proposals = status ? nodeProposals.filter(proposal => proposal.status === status) : nodeProposals;
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({ proposals, total: proposals.length }),
    });
  });

  await page.route("**/api/blueprint/discovery/edge-proposals*", async route => {
    const url = new URL(route.request().url());
    const status = url.searchParams.get("status");
    const proposals = status ? edgeProposals.filter(proposal => proposal.status === status) : edgeProposals;
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({ proposals, total: proposals.length }),
    });
  });

  await page.route("**/api/blueprint/discovery/scan", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        results: [
          {
            scanner: "all",
            proposed_count: 1,
            skipped_count: 0,
            proposed_edge_count: 1,
            skipped_edge_count: 0,
            errors: [],
            duration_ms: 1200,
          },
        ],
        total_proposed: 1,
        total_edge_proposed: 1,
      }),
    });
  });

  await page.route("**/api/blueprint/discovery/proposals/*/accept", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({ node_id: "component-1", message: "accepted" }),
    });
  });

  await page.route("**/api/blueprint/discovery/proposals/*/reject", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({ message: "rejected" }),
    });
  });

  await page.route("**/api/blueprint/discovery/edge-proposals/*/accept", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({ edge: {}, message: "accepted" }),
    });
  });

  await page.route("**/api/blueprint/discovery/edge-proposals/*/reject", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({ message: "rejected" }),
    });
  });
});

test("phase 12 keeps discovery proposal triage primary while controls and context stay secondary", async ({ page }) => {
  await page.goto("/discovery");

  await expect(page.getByRole("heading", { name: "Discovery", exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Proposal queue" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Selected proposal" })).toBeVisible();
  await expect(page.getByText("Task Service").first()).toBeVisible();

  await page.getByRole("button", { name: "Run scan" }).click();
  await expect(page.getByText(/Scan complete/)).toBeVisible();

  await page.getByRole("button", { name: "All" }).click();
  await expect(page.getByRole("heading", { name: "Pending review", exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Reviewed", exact: true })).toBeVisible();

  await page.getByRole("tab", { name: "Edge proposals" }).click();
  await expect(page.getByRole("button", { name: /decision-1 → component-1/ })).toBeVisible();
});
