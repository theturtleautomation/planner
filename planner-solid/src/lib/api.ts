import type {
  BlueprintResponse,
  CreateProjectRequest,
  CreateSessionResponse,
  GetSessionResponse,
  ListProjectsResponse,
  ListSessionsResponse,
  RunListResponse,
  PromptBankResponse,
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
