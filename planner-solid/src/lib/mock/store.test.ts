import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  acceptMockEdgeProposal,
  acceptMockProposal,
  createMockProject,
  createMockSession,
  deleteMockProject,
  getMockPromptBank,
  listMockBlueprintEvents,
  listMockBlueprintHistory,
  listMockProposedEdges,
  listMockProposedNodes,
  listMockProjects,
  listMockSessions,
  resetMockStateForTesting,
  runMockDiscoveryScan,
  saveMockPromptDrafts,
} from "./store";
import { setMockRuntimeLocationSearch } from "./runtime";

describe("frontend mock store", () => {
  beforeEach(() => {
    vi.stubEnv("VITE_PLANNER_FRONTEND_MOCK", "1");
    setMockRuntimeLocationSearch("");
    resetMockStateForTesting();
  });

  afterEach(() => {
    resetMockStateForTesting();
    vi.unstubAllEnvs();
  });

  it("creates projects in the in-memory mock store", () => {
    const before = listMockProjects().projects.length;
    const created = createMockProject({
      name: "Meal Planner",
      description: "Plan weekly meals.",
    });

    expect(created.project.slug).toBe("meal-planner");
    expect(listMockProjects().projects).toHaveLength(before + 1);
  });

  it("creates sessions with an empty prompt bank baseline", () => {
    const created = createMockSession({
      description: "One-off planning brief",
    });

    expect(created.session.project_description).toBe("One-off planning brief");
    expect(getMockPromptBank(created.session.id).initial_bank_complete).toBe(false);
  });

  it("stores saved drafts in the in-memory prompt-bank state", () => {
    const created = createMockSession({
      description: "One-off planning brief",
    });
    const sessionId = created.session.id;

    const response = saveMockPromptDrafts(sessionId, {
      promptId: "prompt-1",
      answers: [
        {
          item_id: "item-1",
          custom_text: "The core workflow is creating and reviewing plans.",
        },
      ],
    });

    expect(response.saved_count).toBe(1);
    expect(getMockPromptBank(sessionId).saved_drafts?.["item-1"]?.custom_text).toBe(
      "The core workflow is creating and reviewing plans.",
    );
  });

  it("deletes a project and its attached sessions from the active scenario", () => {
    const createdProject = createMockProject({
      name: "Travel Planner",
      description: "Trip planning workspace.",
    });
    createMockSession({
      projectRef: createdProject.project.slug,
    });

    const response = deleteMockProject(createdProject.project.slug);

    expect(response.deleted_project_record).toBe(true);
    expect(response.deleted_sessions).toBe(1);
    expect(listMockProjects().projects.map(project => project.slug)).not.toContain(
      createdProject.project.slug,
    );
    expect(listMockSessions().sessions.map(session => session.project_slug)).not.toContain(
      createdProject.project.slug,
    );
  });

  it("reseeds pending discovery work after prior proposals have been reviewed", () => {
    window.history.replaceState({}, "", "/?mockScenario=ops-attention");

    for (const proposal of listMockProposedNodes("pending").proposals) {
      acceptMockProposal(proposal.id);
    }
    for (const proposal of listMockProposedEdges("pending").proposals) {
      acceptMockEdgeProposal(proposal.id);
    }

    expect(listMockProposedNodes("pending").proposals).toHaveLength(0);
    expect(listMockProposedEdges("pending").proposals).toHaveLength(0);

    const response = runMockDiscoveryScan();

    expect(response.total_proposed).toBeGreaterThan(0);
    expect(response.total_edge_proposed).toBeGreaterThan(0);
    expect(listMockProposedNodes("pending").proposals).not.toHaveLength(0);
    expect(listMockProposedEdges("pending").proposals).not.toHaveLength(0);
  });

  it("exposes the richer operational history scenario for events browsing", () => {
    window.history.replaceState({}, "", "/?mockScenario=ops-history");

    expect(listMockBlueprintHistory().snapshots).toHaveLength(3);
    expect(listMockBlueprintEvents().events[0]?.summary).toContain("Recorded project export");
    expect(listMockProposedNodes("pending").proposals).toHaveLength(1);
  });
});
