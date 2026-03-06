import { API_BASE } from '../config.ts';
import { ApiError } from '../types.ts';
import type {
  HealthResponse,
  CreateSessionResponse,
  GetSessionResponse,
  SendMessageResponse,
  StartSocraticResponse,
  BeliefStateResponse,
  ListModelsResponse,
  Session,
  AdminStatusResponse,
  AdminEventsResponse,
} from '../types.ts';
import type {
  BlueprintResponse,
  NodeListResponse,
  BlueprintNode,
  ImpactReport,
  BlueprintEventsResponse,
  ReconvergenceRequest,
  ReconvergenceResult,
} from '../types/blueprint.ts';

export { ApiError };

export function isAuthError(e: Error): boolean {
  return e instanceof ApiError
    ? e.status === 401 || e.status === 403
    : e.message.includes('401') || e.message.includes('403');
}

type GetTokenFn = () => Promise<string>;

async function apiFetch<T>(
  getToken: GetTokenFn,
  path: string,
  init: RequestInit = {},
): Promise<T> {
  const token = await getToken();
  const headers = new Headers(init.headers);
  if (token) headers.set('Authorization', `Bearer ${token}`);
  headers.set('Content-Type', 'application/json');

  const url = `${API_BASE}${path}`;
  const response = await fetch(url, { ...init, headers });

  if (!response.ok) {
    const text = await response.text().catch(() => response.statusText);
    throw new ApiError(
      `API ${init.method ?? 'GET'} ${path} → ${response.status}: ${text}`,
      response.status,
    );
  }

  return response.json() as Promise<T>;
}

// ─── Factory ─────────────────────────────────────────────────────────────────

export interface ListSessionsResponse {
  sessions: Session[];
}

export function createApiClient(getToken: GetTokenFn) {
  return {
    health(): Promise<HealthResponse> {
      return apiFetch<HealthResponse>(getToken, '/health');
    },

    createSession(): Promise<CreateSessionResponse> {
      return apiFetch<CreateSessionResponse>(getToken, '/sessions', {
        method: 'POST',
        body: '{}',
      });
    },

    getSession(id: string): Promise<GetSessionResponse> {
      return apiFetch<GetSessionResponse>(getToken, `/sessions/${id}`);
    },

    listSessions(): Promise<ListSessionsResponse> {
      return apiFetch<ListSessionsResponse>(getToken, '/sessions');
    },

    sendMessage(id: string, content: string): Promise<SendMessageResponse> {
      return apiFetch<SendMessageResponse>(getToken, `/sessions/${id}/message`, {
        method: 'POST',
        body: JSON.stringify({ content }),
      });
    },

    listModels(): Promise<ListModelsResponse> {
      return apiFetch<ListModelsResponse>(getToken, '/models');
    },

    startSocratic(id: string, description: string): Promise<StartSocraticResponse> {
      return apiFetch<StartSocraticResponse>(getToken, `/sessions/${id}/socratic`, {
        method: 'POST',
        body: JSON.stringify({ description }),
      });
    },

    getBeliefState(id: string): Promise<BeliefStateResponse> {
      return apiFetch<BeliefStateResponse>(getToken, `/sessions/${id}/belief-state`);
    },

    adminStatus(): Promise<AdminStatusResponse> {
      return fetch(`${API_BASE}/admin/status`).then(async (res) => {
        if (!res.ok) {
          const text = await res.text().catch(() => res.statusText);
          throw new ApiError(`API GET /admin/status → ${res.status}: ${text}`, res.status);
        }
        return res.json() as Promise<AdminStatusResponse>;
      });
    },

    adminEvents(params?: { limit?: number; level?: string }): Promise<AdminEventsResponse> {
      const qs = new URLSearchParams();
      if (params?.limit !== undefined) qs.set('limit', String(params.limit));
      if (params?.level !== undefined) qs.set('level', params.level);
      const query = qs.toString() ? `?${qs.toString()}` : '';
      return fetch(`${API_BASE}/admin/events${query}`).then(async (res) => {
        if (!res.ok) {
          const text = await res.text().catch(() => res.statusText);
          throw new ApiError(`API GET /admin/events → ${res.status}: ${text}`, res.status);
        }
        return res.json() as Promise<AdminEventsResponse>;
      });
    },

    // ─── Blueprint ──────────────────────────────────────────────────────

    /** GET /blueprint — Full graph snapshot (summaries + edges + counts). */
    getBlueprint(): Promise<BlueprintResponse> {
      return apiFetch<BlueprintResponse>(getToken, '/blueprint');
    },

    /** GET /blueprint/nodes?type=<type> — List nodes, optionally by type. */
    listBlueprintNodes(nodeType?: string): Promise<NodeListResponse> {
      const qs = nodeType ? `?type=${encodeURIComponent(nodeType)}` : '';
      return apiFetch<NodeListResponse>(getToken, `/blueprint/nodes${qs}`);
    },

    /** POST /blueprint/nodes — Create a new node. */
    createBlueprintNode(node: BlueprintNode): Promise<{ id: string; message: string }> {
      return apiFetch<{ id: string; message: string }>(getToken, '/blueprint/nodes', {
        method: 'POST',
        body: JSON.stringify(node),
      });
    },

    /** GET /blueprint/nodes/:id — Get a single node (full data). */
    getBlueprintNode(nodeId: string): Promise<BlueprintNode> {
      return apiFetch<BlueprintNode>(getToken, `/blueprint/nodes/${encodeURIComponent(nodeId)}`);
    },

    /** PATCH /blueprint/nodes/:id — Replace a node. */
    updateBlueprintNode(nodeId: string, node: BlueprintNode): Promise<{ id: string; message: string }> {
      return apiFetch<{ id: string; message: string }>(getToken, `/blueprint/nodes/${encodeURIComponent(nodeId)}`, {
        method: 'PATCH',
        body: JSON.stringify(node),
      });
    },

    /** DELETE /blueprint/nodes/:id — Remove a node and its edges. */
    deleteBlueprintNode(nodeId: string): Promise<{ message: string }> {
      return apiFetch<{ message: string }>(getToken, `/blueprint/nodes/${encodeURIComponent(nodeId)}`, {
        method: 'DELETE',
      });
    },

    /** POST /blueprint/edges — Create a directed edge. */
    createBlueprintEdge(edge: { source: string; target: string; edge_type: string; metadata?: string }): Promise<{ message: string }> {
      return apiFetch<{ message: string }>(getToken, '/blueprint/edges', {
        method: 'POST',
        body: JSON.stringify(edge),
      });
    },

    /** DELETE /blueprint/edges — Remove a directed edge by source+target+edge_type. */
    deleteBlueprintEdge(edge: { source: string; target: string; edge_type: string }): Promise<void> {
      return apiFetch<void>(getToken, '/blueprint/edges', {
        method: 'DELETE',
        body: JSON.stringify(edge),
      });
    },

    /** GET /blueprint/history — List history snapshots. */
    listBlueprintHistory(): Promise<{ snapshots: { timestamp: string; filename: string }[] }> {
      return apiFetch<{ snapshots: { timestamp: string; filename: string }[] }>(getToken, '/blueprint/history');
    },

    /** POST /blueprint/history — Create a named snapshot. */
    createBlueprintSnapshot(label?: string): Promise<{ timestamp: string; filename: string }> {
      return apiFetch<{ timestamp: string; filename: string }>(getToken, '/blueprint/history', {
        method: 'POST',
        body: JSON.stringify({ label: label || undefined }),
      });
    },

    /** GET /blueprint/events — List event log, optionally filtered by node. */
    listBlueprintEvents(params?: { nodeId?: string; limit?: number }): Promise<BlueprintEventsResponse> {
      const qs = new URLSearchParams();
      if (params?.nodeId) qs.set('node_id', params.nodeId);
      if (params?.limit !== undefined) qs.set('limit', String(params.limit));
      const query = qs.toString() ? `?${qs.toString()}` : '';
      return apiFetch<BlueprintEventsResponse>(getToken, `/blueprint/events${query}`);
    },

    /** POST /blueprint/impact-preview — Analyze impact of changing a node. */
    impactPreview(nodeId: string, changeDescription: string): Promise<ImpactReport> {
      return apiFetch<ImpactReport>(getToken, '/blueprint/impact-preview', {
        method: 'POST',
        body: JSON.stringify({ node_id: nodeId, change_description: changeDescription }),
      });
    },

    /** POST /blueprint/reconverge — Execute reconvergence from an impact report. */
    reconvergeBlueprint(req: ReconvergenceRequest): Promise<ReconvergenceResult> {
      return apiFetch<ReconvergenceResult>(getToken, '/blueprint/reconverge', {
        method: 'POST',
        body: JSON.stringify(req),
      });
    },
  };
}

export type ApiClient = ReturnType<typeof createApiClient>;
