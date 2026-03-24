import type {
  AdminEventsResponse,
  AdminStatusResponse,
  BlueprintResponse,
  BlueprintEventsResponse,
  BlueprintExportHistoryResponse,
  CreateProjectRequest,
  DiscoveryRunResponse,
  CreateSessionResponse,
  GetSessionResponse,
  HistoryListResponse,
  ListProjectsResponse,
  ListSessionsResponse,
  RunListResponse,
  PromptBankResponse,
  ProposedEdgesResponse,
  ProposedNodesResponse,
  ProjectResponse,
  ProjectImportResponse,
  SessionEventsResponse,
  StartSocraticResponse,
} from "./types";

const API_BASE = "/api";

async function apiFetch<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE}${path}`, {
    ...init,
    headers: {
      "Content-Type": "application/json",
      ...(init?.headers ?? {}),
    },
  });

  if (!response.ok) {
    const text = await response.text().catch(() => response.statusText);
    throw new Error(text || `Request failed: ${response.status}`);
  }

  return response.json() as Promise<T>;
}

async function apiFetchOptional<T>(path: string, init?: RequestInit): Promise<T | null> {
  const response = await fetch(`${API_BASE}${path}`, {
    ...init,
    headers: {
      "Content-Type": "application/json",
      ...(init?.headers ?? {}),
    },
  });

  if (response.status === 404) {
    return null;
  }

  if (!response.ok) {
    const text = await response.text().catch(() => response.statusText);
    throw new Error(text || `Request failed: ${response.status}`);
  }

  return response.json() as Promise<T>;
}

const getCache = new Map<string, Promise<unknown>>();

function cachedGet<T>(path: string): Promise<T> {
  const existing = getCache.get(path);
  if (existing) return existing as Promise<T>;
  const request = apiFetch<T>(path).catch(error => {
    getCache.delete(path);
    throw error;
  });
  getCache.set(path, request);
  return request;
}

function invalidateCache(paths: string[]) {
  for (const path of paths) {
    getCache.delete(path);
  }
}

export function listSessions(): Promise<ListSessionsResponse> {
  return cachedGet("/sessions");
}

export function getAdminStatus(): Promise<AdminStatusResponse> {
  return apiFetch("/admin/status");
}

export function getAdminEvents(params?: { limit?: number; level?: string; sessionId?: string }): Promise<AdminEventsResponse> {
  const qs = new URLSearchParams();
  if (params?.limit !== undefined) qs.set("limit", String(params.limit));
  if (params?.level) qs.set("level", params.level);
  if (params?.sessionId) qs.set("session_id", params.sessionId);
  const query = qs.toString();
  return apiFetch(`/admin/events${query ? `?${query}` : ""}`);
}

export function createSession(): Promise<CreateSessionResponse> {
  return apiFetch("/sessions", {
    method: "POST",
    body: "{}",
  });
}

export function getSession(sessionId: string): Promise<GetSessionResponse> {
  return apiFetch(`/sessions/${encodeURIComponent(sessionId)}`);
}

export function getSessionRuns(sessionId: string): Promise<RunListResponse> {
  return apiFetch(`/sessions/${encodeURIComponent(sessionId)}/runs`);
}

export function getSessionEvents(
  sessionId: string,
  params?: {
    level?: "info" | "warn" | "error";
    source?: "socratic_engine" | "llm_router" | "pipeline" | "factory" | "system";
    limit?: number;
    offset?: number;
  },
): Promise<SessionEventsResponse> {
  const qs = new URLSearchParams();
  if (params?.level) qs.set("level", params.level);
  if (params?.source) qs.set("source", params.source);
  if (params?.limit !== undefined) qs.set("limit", String(params.limit));
  if (params?.offset !== undefined) qs.set("offset", String(params.offset));
  const query = qs.toString() ? `?${qs.toString()}` : "";
  return apiFetch(`/sessions/${encodeURIComponent(sessionId)}/events${query}`);
}

export function startSocratic(sessionId: string, description: string): Promise<StartSocraticResponse> {
  return apiFetch(`/sessions/${encodeURIComponent(sessionId)}/socratic`, {
    method: "POST",
    body: JSON.stringify({ description }),
  });
}

export function getPromptBank(sessionId: string): Promise<PromptBankResponse> {
  return apiFetch(`/sessions/${encodeURIComponent(sessionId)}/prompt-bank`);
}

export function listProjects(): Promise<ListProjectsResponse> {
  return cachedGet("/projects");
}

export function createProject(request: CreateProjectRequest): Promise<ProjectResponse> {
  return apiFetch("/projects", {
    method: "POST",
    body: JSON.stringify(request),
  }).then(response => {
    invalidateCache(["/projects"]);
    return response as ProjectResponse;
  });
}

export function getProject(projectRef: string): Promise<ProjectResponse> {
  return cachedGet(`/projects/${encodeURIComponent(projectRef)}`);
}

export function listProjectSessions(projectRef: string): Promise<ListSessionsResponse> {
  return cachedGet(`/projects/${encodeURIComponent(projectRef)}/sessions`);
}

export function getProjectBlueprint(
  projectRef: string,
  options?: { includeShared?: boolean; includeGlobal?: boolean },
): Promise<BlueprintResponse> {
  const params = new URLSearchParams();
  params.set("projectId", projectRef);
  if (options?.includeShared !== undefined) {
    params.set("includeShared", String(options.includeShared));
  }
  if (options?.includeGlobal !== undefined) {
    params.set("includeGlobal", String(options.includeGlobal));
  }
  const query = params.toString();
  return cachedGet(`/blueprint${query ? `?${query}` : ""}`);
}

export function listBlueprintHistory(): Promise<HistoryListResponse> {
  return apiFetch("/blueprint/history");
}

export function createBlueprintSnapshot(label?: string): Promise<{ timestamp: string; filename: string }> {
  return apiFetch("/blueprint/history", {
    method: "POST",
    body: JSON.stringify({ label: label || undefined }),
  });
}

export function listBlueprintEvents(params?: { nodeId?: string; limit?: number }): Promise<BlueprintEventsResponse> {
  const qs = new URLSearchParams();
  if (params?.nodeId) qs.set("node_id", params.nodeId);
  if (params?.limit !== undefined) qs.set("limit", String(params.limit));
  const query = qs.toString();
  return apiFetch(`/blueprint/events${query ? `?${query}` : ""}`);
}

export function listBlueprintExportHistory(params?: {
  projectId?: string;
  scopeClass?: string;
  feature?: string;
  widget?: string;
  artifact?: string;
  component?: string;
  limit?: number;
}): Promise<BlueprintExportHistoryResponse> {
  const qs = new URLSearchParams();
  if (params?.projectId) qs.set("project_id", params.projectId);
  if (params?.scopeClass) qs.set("scope_class", params.scopeClass);
  if (params?.feature) qs.set("feature", params.feature);
  if (params?.widget) qs.set("widget", params.widget);
  if (params?.artifact) qs.set("artifact", params.artifact);
  if (params?.component) qs.set("component", params.component);
  if (params?.limit !== undefined) qs.set("limit", String(params.limit));
  const query = qs.toString();
  return apiFetch(`/blueprint/export-history${query ? `?${query}` : ""}`);
}

export function runDiscoveryScan(): Promise<DiscoveryRunResponse> {
  return apiFetch("/blueprint/discovery/scan", {
    method: "POST",
    body: JSON.stringify({ scanners: ["all"] }),
  });
}

export function listProposedNodes(status?: string): Promise<ProposedNodesResponse> {
  const qs = status ? `?status=${encodeURIComponent(status)}` : "";
  return apiFetch(`/blueprint/discovery/proposals${qs}`);
}

export function listProposedEdges(status?: string): Promise<ProposedEdgesResponse> {
  const qs = status ? `?status=${encodeURIComponent(status)}` : "";
  return apiFetch(`/blueprint/discovery/edge-proposals${qs}`);
}

export function acceptProposal(
  proposalId: string,
  payload?: { node_patch?: Record<string, unknown> },
): Promise<{ node_id: string; message: string }> {
  return apiFetch(`/blueprint/discovery/proposals/${encodeURIComponent(proposalId)}/accept`, {
    method: "POST",
    body: JSON.stringify(payload ?? {}),
  });
}

export function rejectProposal(proposalId: string, reason?: string): Promise<{ message: string }> {
  return apiFetch(`/blueprint/discovery/proposals/${encodeURIComponent(proposalId)}/reject`, {
    method: "POST",
    body: JSON.stringify({ reason }),
  });
}

export function acceptEdgeProposal(proposalId: string): Promise<{ edge: unknown; message: string }> {
  return apiFetch(`/blueprint/discovery/edge-proposals/${encodeURIComponent(proposalId)}/accept`, {
    method: "POST",
  });
}

export function rejectEdgeProposal(proposalId: string, reason?: string): Promise<{ message: string }> {
  return apiFetch(`/blueprint/discovery/edge-proposals/${encodeURIComponent(proposalId)}/reject`, {
    method: "POST",
    body: JSON.stringify({ reason }),
  });
}

export function getProjectImportState(projectRef: string): Promise<ProjectImportResponse | null> {
  return apiFetchOptional(`/projects/${encodeURIComponent(projectRef)}/import-state`);
}

export function getProjectImportReview(projectRef: string): Promise<ProjectImportResponse | null> {
  return apiFetchOptional(`/projects/${encodeURIComponent(projectRef)}/import-review`);
}

export function updateProjectImportReviewSelection(
  projectRef: string,
  payload: { nodeId: string; included: boolean },
): Promise<ProjectImportResponse> {
  return apiFetch(`/projects/${encodeURIComponent(projectRef)}/import-review-selection`, {
    method: "POST",
    body: JSON.stringify({
      node_id: payload.nodeId,
      included: payload.included,
    }),
  });
}

export function applyProjectImportReview(projectRef: string): Promise<ProjectImportResponse> {
  return apiFetch(`/projects/${encodeURIComponent(projectRef)}/import-review`, {
    method: "POST",
    body: "{}",
  });
}

export function createProjectSession(
  projectRef: string,
  payload?: { title?: string | null; description?: string | null },
): Promise<CreateSessionResponse> {
  return apiFetch(`/projects/${encodeURIComponent(projectRef)}/sessions`, {
    method: "POST",
    body: JSON.stringify(payload ?? {}),
  }).then(response => {
    invalidateCache([
      "/sessions",
      `/projects/${encodeURIComponent(projectRef)}`,
      `/projects/${encodeURIComponent(projectRef)}/sessions`,
    ]);
    return response as CreateSessionResponse;
  });
}

export function buildSocraticWebSocketUrl(sessionId: string): string {
  const url = new URL(window.location.origin);
  url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
  url.pathname = `/api/sessions/${encodeURIComponent(sessionId)}/socratic/ws`;
  return url.toString();
}
