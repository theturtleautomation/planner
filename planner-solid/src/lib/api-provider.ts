import {
  acceptMockEdgeProposal,
  acceptMockProposal,
  applyMockProjectImportReview,
  createMockBlueprintSnapshot,
  createMockProject,
  createMockProjectSession,
  createMockSession,
  deleteMockProject,
  duplicateMockSession,
  exportMockSession,
  getMockAdminEvents,
  getMockAdminStatus,
  getMockProject,
  getMockProjectBlueprint,
  getMockProjectImportHistory,
  getMockProjectImportHistoryComparison,
  getMockProjectImportHistoryPairComparison,
  getMockProjectImportReview,
  getMockProjectImportState,
  getMockPromptBank,
  getMockSession,
  getMockSessionEvents,
  getMockSessionRuns,
  listMockBlueprintEvents,
  listMockBlueprintExportHistory,
  listMockBlueprintHistory,
  listMockProjects,
  listMockProjectSessions,
  listMockProposedEdges,
  listMockProposedNodes,
  listMockSessions,
  reimportMockProject,
  rejectMockEdgeProposal,
  rejectMockProposal,
  restartMockSessionFromDescription,
  restoreMockProjectImportHistoryEntry,
  restoreMockProjectImportHistoryEntryForReview,
  restoreMockProjectImportReviewDraft,
  retryMockSessionPipeline,
  runMockDiscoveryScan,
  saveMockPromptDrafts,
  updateMockProjectImportReviewSelection,
} from "./mock/store";
import { isFrontendMockEnabled } from "./mock/runtime";

const API_BASE = typeof window === "undefined" ? "http://127.0.0.1:3100/api" : "/api";
const PENDING_MOCK_PROJECT_STORAGE_KEY = "planner.frontend-mock.pending-project";

function jsonResponse(payload: unknown, status = 200): Response {
  return new Response(JSON.stringify(payload), {
    status,
    headers: {
      "Content-Type": "application/json",
    },
  });
}

function textResponse(message: string, status: number): Response {
  return new Response(message, {
    status,
    headers: {
      "Content-Type": "text/plain",
    },
  });
}

function parseJsonBody<T>(init?: RequestInit): T {
  if (!init?.body || typeof init.body !== "string") {
    return {} as T;
  }
  return JSON.parse(init.body) as T;
}

function syncPendingBrowserMockProject(): void {
  if (typeof window === "undefined" || !isFrontendMockEnabled()) {
    return;
  }

  const payload = window.sessionStorage.getItem(PENDING_MOCK_PROJECT_STORAGE_KEY);
  if (!payload) {
    return;
  }

  window.sessionStorage.removeItem(PENDING_MOCK_PROJECT_STORAGE_KEY);

  try {
    const parsed = JSON.parse(payload) as {
      name?: string;
      description?: string | null;
      slug?: string;
    };
    const slug = parsed.slug?.trim();
    const name = parsed.name?.trim();
    if (!slug || !name) {
      return;
    }

    try {
      getMockProject(slug);
      return;
    } catch {
      createMockProject({
        name,
        description: parsed.description?.trim() || null,
        slug,
      });
    }
  } catch {
    // Ignore malformed seed state and continue with the normal mock provider path.
  }
}

async function handleMockRequest(path: string, init?: RequestInit): Promise<Response> {
  syncPendingBrowserMockProject();
  const method = (init?.method ?? "GET").toUpperCase();

  if (path === "/projects" && method === "GET") {
    return jsonResponse(listMockProjects());
  }

  if (path === "/projects" && method === "POST") {
    return jsonResponse(createMockProject(parseJsonBody(init)));
  }

  if (path === "/sessions" && method === "GET") {
    return jsonResponse(listMockSessions());
  }

  if (path === "/sessions" && method === "POST") {
    const payload = parseJsonBody<{ project_ref?: string | null; description?: string | null }>(init);
    return jsonResponse(
      createMockSession({
        projectRef: payload.project_ref ?? null,
        description: payload.description ?? null,
      }),
    );
  }

  if (path === "/blueprint/history" && method === "GET") {
    return jsonResponse(listMockBlueprintHistory());
  }

  if (path === "/blueprint/history" && method === "POST") {
    return jsonResponse(createMockBlueprintSnapshot());
  }

  const blueprintEventsUrl = new URL(path, "http://planner.local");
  if (blueprintEventsUrl.pathname === "/blueprint/events" && method === "GET") {
    return jsonResponse(
      listMockBlueprintEvents({
        nodeId: blueprintEventsUrl.searchParams.get("node_id") ?? undefined,
        limit: blueprintEventsUrl.searchParams.get("limit")
          ? Number(blueprintEventsUrl.searchParams.get("limit"))
          : undefined,
      }),
    );
  }

  if (blueprintEventsUrl.pathname === "/blueprint/export-history" && method === "GET") {
    return jsonResponse(
      listMockBlueprintExportHistory({
        projectId: blueprintEventsUrl.searchParams.get("project_id") ?? undefined,
        limit: blueprintEventsUrl.searchParams.get("limit")
          ? Number(blueprintEventsUrl.searchParams.get("limit"))
          : undefined,
      }),
    );
  }

  const blueprintUrl = new URL(path, "http://planner.local");
  if (blueprintUrl.pathname === "/blueprint" && method === "GET") {
    const projectRef = blueprintUrl.searchParams.get("projectId");
    if (!projectRef) {
      return textResponse("Mock blueprint requests require projectId.", 400);
    }
    return jsonResponse(getMockProjectBlueprint(projectRef));
  }

  if (path === "/blueprint/discovery/scan" && method === "POST") {
    return jsonResponse(runMockDiscoveryScan());
  }

  const proposedNodesUrl = new URL(path, "http://planner.local");
  if (proposedNodesUrl.pathname === "/blueprint/discovery/proposals" && method === "GET") {
    return jsonResponse(listMockProposedNodes(proposedNodesUrl.searchParams.get("status") ?? undefined));
  }

  const proposedEdgesUrl = new URL(path, "http://planner.local");
  if (proposedEdgesUrl.pathname === "/blueprint/discovery/edge-proposals" && method === "GET") {
    return jsonResponse(listMockProposedEdges(proposedEdgesUrl.searchParams.get("status") ?? undefined));
  }

  const proposalAcceptMatch = path.match(/^\/blueprint\/discovery\/proposals\/([^/]+)\/accept$/);
  if (proposalAcceptMatch && method === "POST") {
    return jsonResponse(acceptMockProposal(decodeURIComponent(proposalAcceptMatch[1]!)));
  }

  const proposalRejectMatch = path.match(/^\/blueprint\/discovery\/proposals\/([^/]+)\/reject$/);
  if (proposalRejectMatch && method === "POST") {
    return jsonResponse(rejectMockProposal(decodeURIComponent(proposalRejectMatch[1]!)));
  }

  const edgeAcceptMatch = path.match(/^\/blueprint\/discovery\/edge-proposals\/([^/]+)\/accept$/);
  if (edgeAcceptMatch && method === "POST") {
    return jsonResponse(acceptMockEdgeProposal(decodeURIComponent(edgeAcceptMatch[1]!)));
  }

  const edgeRejectMatch = path.match(/^\/blueprint\/discovery\/edge-proposals\/([^/]+)\/reject$/);
  if (edgeRejectMatch && method === "POST") {
    return jsonResponse(rejectMockEdgeProposal(decodeURIComponent(edgeRejectMatch[1]!)));
  }

  if (path === "/admin/status" && method === "GET") {
    return jsonResponse(getMockAdminStatus());
  }

  const adminEventsUrl = new URL(path, "http://planner.local");
  if (adminEventsUrl.pathname === "/admin/events" && method === "GET") {
    return jsonResponse(
      getMockAdminEvents({
        level: adminEventsUrl.searchParams.get("level") ?? undefined,
        sessionId: adminEventsUrl.searchParams.get("session_id") ?? undefined,
        limit: adminEventsUrl.searchParams.get("limit")
          ? Number(adminEventsUrl.searchParams.get("limit"))
          : undefined,
      }),
    );
  }

  const projectMatch = path.match(/^\/projects\/([^/]+)$/);
  if (projectMatch && method === "GET") {
    return jsonResponse(getMockProject(decodeURIComponent(projectMatch[1]!)));
  }
  if (projectMatch && method === "DELETE") {
    return jsonResponse(deleteMockProject(decodeURIComponent(projectMatch[1]!)));
  }

  const projectSessionsMatch = path.match(/^\/projects\/([^/]+)\/sessions$/);
  if (projectSessionsMatch && method === "GET") {
    return jsonResponse(listMockProjectSessions(decodeURIComponent(projectSessionsMatch[1]!)));
  }
  if (projectSessionsMatch && method === "POST") {
    const payload = parseJsonBody<{ title?: string | null; description?: string | null }>(init);
    return jsonResponse(
      createMockProjectSession(decodeURIComponent(projectSessionsMatch[1]!), {
        title: payload.title ?? null,
        description: payload.description ?? null,
      }),
    );
  }

  const importReviewMatch = path.match(/^\/projects\/([^/]+)\/import-review$/);
  if (importReviewMatch && method === "GET") {
    const response = getMockProjectImportReview(decodeURIComponent(importReviewMatch[1]!));
    return response ? jsonResponse(response) : textResponse("Not found", 404);
  }
  if (importReviewMatch && method === "POST") {
    return jsonResponse(applyMockProjectImportReview(decodeURIComponent(importReviewMatch[1]!)));
  }

  const importSelectionMatch = path.match(/^\/projects\/([^/]+)\/import-review-selection$/);
  if (importSelectionMatch && method === "POST") {
    const payload = parseJsonBody<{ node_id: string; included: boolean }>(init);
    return jsonResponse(
      updateMockProjectImportReviewSelection(decodeURIComponent(importSelectionMatch[1]!), {
        nodeId: payload.node_id,
        included: payload.included,
      }),
    );
  }

  const importStateMatch = path.match(/^\/projects\/([^/]+)\/import-state$/);
  if (importStateMatch && method === "GET") {
    const response = getMockProjectImportState(decodeURIComponent(importStateMatch[1]!));
    return response ? jsonResponse(response) : textResponse("Not found", 404);
  }

  const importHistoryMatch = path.match(/^\/projects\/([^/]+)\/import-history$/);
  if (importHistoryMatch && method === "GET") {
    const response = getMockProjectImportHistory(decodeURIComponent(importHistoryMatch[1]!));
    return response ? jsonResponse(response) : textResponse("Not found", 404);
  }

  const importComparisonMatch = path.match(/^\/projects\/([^/]+)\/import-history\/([^/]+)\/compare$/);
  if (importComparisonMatch && method === "GET") {
    return jsonResponse(
      getMockProjectImportHistoryComparison(
        decodeURIComponent(importComparisonMatch[1]!),
        decodeURIComponent(importComparisonMatch[2]!),
      ),
    );
  }

  const importPairComparisonMatch = path.match(/^\/projects\/([^/]+)\/import-history\/([^/]+)\/compare\/([^/]+)$/);
  if (importPairComparisonMatch && method === "GET") {
    return jsonResponse(
      getMockProjectImportHistoryPairComparison(
        decodeURIComponent(importPairComparisonMatch[1]!),
        decodeURIComponent(importPairComparisonMatch[2]!),
        decodeURIComponent(importPairComparisonMatch[3]!),
      ),
    );
  }

  const importRestoreMatch = path.match(/^\/projects\/([^/]+)\/import-history\/([^/]+)\/restore$/);
  if (importRestoreMatch && method === "POST") {
    return jsonResponse(
      restoreMockProjectImportHistoryEntry(
        decodeURIComponent(importRestoreMatch[1]!),
        decodeURIComponent(importRestoreMatch[2]!),
      ),
    );
  }

  const importRestoreReviewMatch = path.match(/^\/projects\/([^/]+)\/import-history\/([^/]+)\/restore-for-review$/);
  if (importRestoreReviewMatch && method === "POST") {
    return jsonResponse(
      restoreMockProjectImportHistoryEntryForReview(
        decodeURIComponent(importRestoreReviewMatch[1]!),
        decodeURIComponent(importRestoreReviewMatch[2]!),
      ),
    );
  }

  const importRestoreDraftMatch = path.match(/^\/projects\/([^/]+)\/import-history\/([^/]+)\/restore-review-draft$/);
  if (importRestoreDraftMatch && method === "POST") {
    return jsonResponse(
      restoreMockProjectImportReviewDraft(
        decodeURIComponent(importRestoreDraftMatch[1]!),
        decodeURIComponent(importRestoreDraftMatch[2]!),
      ),
    );
  }

  const reimportMatch = path.match(/^\/projects\/([^/]+)\/reimport$/);
  if (reimportMatch && method === "POST") {
    return jsonResponse(reimportMockProject(decodeURIComponent(reimportMatch[1]!)));
  }

  const sessionDuplicateMatch = path.match(/^\/sessions\/([^/]+)\/duplicate$/);
  if (sessionDuplicateMatch && method === "POST") {
    const payload = parseJsonBody<{ title?: string | null }>(init);
    return jsonResponse(
      duplicateMockSession(decodeURIComponent(sessionDuplicateMatch[1]!), {
        title: payload.title ?? null,
      }),
    );
  }

  const sessionExportMatch = path.match(/^\/sessions\/([^/]+)\/export$/);
  if (sessionExportMatch && method === "GET") {
    return jsonResponse(exportMockSession(decodeURIComponent(sessionExportMatch[1]!)));
  }

  const sessionRestartMatch = path.match(/^\/sessions\/([^/]+)\/restart-from-description$/);
  if (sessionRestartMatch && method === "POST") {
    return jsonResponse(restartMockSessionFromDescription(decodeURIComponent(sessionRestartMatch[1]!)));
  }

  const sessionRetryMatch = path.match(/^\/sessions\/([^/]+)\/retry-pipeline$/);
  if (sessionRetryMatch && method === "POST") {
    return jsonResponse(retryMockSessionPipeline(decodeURIComponent(sessionRetryMatch[1]!)));
  }

  const sessionRunsMatch = path.match(/^\/sessions\/([^/]+)\/runs$/);
  if (sessionRunsMatch && method === "GET") {
    return jsonResponse(getMockSessionRuns(decodeURIComponent(sessionRunsMatch[1]!)));
  }

  const sessionEventsUrl = new URL(path, "http://planner.local");
  const sessionEventsMatch = sessionEventsUrl.pathname.match(/^\/sessions\/([^/]+)\/events$/);
  if (sessionEventsMatch && method === "GET") {
    return jsonResponse(getMockSessionEvents(decodeURIComponent(sessionEventsMatch[1]!)));
  }

  const sessionMatch = path.match(/^\/sessions\/([^/]+)$/);
  if (sessionMatch && method === "GET") {
    return jsonResponse(getMockSession(decodeURIComponent(sessionMatch[1]!)));
  }

  const promptBankMatch = path.match(/^\/sessions\/([^/]+)\/prompt-bank$/);
  if (promptBankMatch && method === "GET") {
    return jsonResponse(getMockPromptBank(decodeURIComponent(promptBankMatch[1]!)));
  }

  const promptDraftsMatch = path.match(/^\/sessions\/([^/]+)\/prompt-drafts$/);
  if (promptDraftsMatch && method === "POST") {
    const payload = parseJsonBody<{ prompt_id: string; answers: Array<Record<string, unknown>> }>(init);
    return jsonResponse(
      saveMockPromptDrafts(decodeURIComponent(promptDraftsMatch[1]!), {
        promptId: payload.prompt_id,
        answers: payload.answers as never,
      }),
    );
  }

  return textResponse(`Mock API route not implemented: ${method} ${path}`, 404);
}

export async function apiRequest(path: string, init?: RequestInit): Promise<Response> {
  if (isFrontendMockEnabled()) {
    return handleMockRequest(path, init);
  }

  return fetch(`${API_BASE}${path}`, {
    ...init,
    headers: {
      "Content-Type": "application/json",
      ...(init?.headers ?? {}),
    },
  });
}
