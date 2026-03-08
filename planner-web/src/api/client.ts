import { API_BASE, WS_PROTOCOL } from '../config.ts';
import { ApiError } from '../types.ts';
import type {
  HealthResponse,
  CreateSessionResponse,
  GetSessionResponse,
  SessionEventsResponse,
  ListSessionsResponse,
  SendMessageResponse,
  SessionExportResponse,
  StartSocraticResponse,
  BeliefStateResponse,
  ListModelsResponse,
  AdminStatusResponse,
  AdminEventsResponse,
  ProjectResponse,
  ListProjectsResponse,
} from '../types.ts';
import type {
  BlueprintResponse,
  NodeListResponse,
  BlueprintNode,
  ImpactReport,
  BlueprintEventsResponse,
  ReconvergenceRequest,
  ReconvergenceResult,
  DiscoveryScanRequest,
  DiscoveryRunResponse,
  ProposedNodesResponse,
  ScopeClass,
  ScopeVisibility,
  NodeLifecycle,
  BlueprintExportKind,
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

function buildWebSocketUrl(path: string, token: string): string {
  const url = new URL(API_BASE, window.location.origin);
  url.protocol = WS_PROTOCOL;
  url.pathname = `${url.pathname.replace(/\/$/, '')}${path}`;
  if (token) {
    url.searchParams.set('token', token);
  }
  return url.toString();
}

export function createApiClient(getToken: GetTokenFn) {
  const buildNodeQuery = (params?: {
    nodeType?: string;
    scopeClass?: ScopeClass;
    scopeVisibility?: ScopeVisibility;
    lifecycle?: NodeLifecycle;
    projectId?: string;
    feature?: string;
    widget?: string;
    artifact?: string;
    component?: string;
    includeShared?: boolean;
    includeGlobal?: boolean;
  }): string => {
    if (!params) return '';
    const qs = new URLSearchParams();
    if (params.nodeType) qs.set('type', params.nodeType);
    if (params.scopeClass) qs.set('scope_class', params.scopeClass);
    if (params.scopeVisibility) qs.set('scope_visibility', params.scopeVisibility);
    if (params.lifecycle) qs.set('lifecycle', params.lifecycle);
    if (params.projectId) qs.set('project_id', params.projectId);
    if (params.feature) qs.set('feature', params.feature);
    if (params.widget) qs.set('widget', params.widget);
    if (params.artifact) qs.set('artifact', params.artifact);
    if (params.component) qs.set('component', params.component);
    if (params.includeShared !== undefined) qs.set('include_shared', String(params.includeShared));
    if (params.includeGlobal !== undefined) qs.set('include_global', String(params.includeGlobal));
    const raw = qs.toString();
    return raw ? `?${raw}` : '';
  };

  return {
    health(): Promise<HealthResponse> {
      return apiFetch<HealthResponse>(getToken, '/health');
    },

    createSession(payload?: { projectRef?: string }): Promise<CreateSessionResponse> {
      const body = payload?.projectRef
        ? JSON.stringify({ project_ref: payload.projectRef })
        : '{}';
      return apiFetch<CreateSessionResponse>(getToken, '/sessions', {
        method: 'POST',
        body,
      });
    },

    listProjects(): Promise<ListProjectsResponse> {
      return apiFetch<ListProjectsResponse>(getToken, '/projects');
    },

    createProject(payload: {
      name: string;
      slug?: string;
      description?: string;
      teamLabel?: string;
      legacyScopeKeys?: string[];
    }): Promise<ProjectResponse> {
      return apiFetch<ProjectResponse>(getToken, '/projects', {
        method: 'POST',
        body: JSON.stringify({
          name: payload.name,
          slug: payload.slug,
          description: payload.description,
          team_label: payload.teamLabel,
          legacy_scope_keys: payload.legacyScopeKeys ?? [],
        }),
      });
    },

    getProject(projectRef: string): Promise<ProjectResponse> {
      return apiFetch<ProjectResponse>(getToken, `/projects/${encodeURIComponent(projectRef)}`);
    },

    updateProject(
      projectRef: string,
      patch: {
        name?: string;
        slug?: string;
        description?: string;
        teamLabel?: string;
        legacyScopeKeys?: string[];
      },
    ): Promise<ProjectResponse> {
      return apiFetch<ProjectResponse>(getToken, `/projects/${encodeURIComponent(projectRef)}`, {
        method: 'PATCH',
        body: JSON.stringify({
          name: patch.name,
          slug: patch.slug,
          description: patch.description,
          team_label: patch.teamLabel,
          legacy_scope_keys: patch.legacyScopeKeys,
        }),
      });
    },

    listProjectSessions(
      projectRef: string,
      params?: { includeArchived?: boolean },
    ): Promise<ListSessionsResponse> {
      const qs = new URLSearchParams();
      if (params?.includeArchived) qs.set('include_archived', 'true');
      const query = qs.toString() ? `?${qs.toString()}` : '';
      return apiFetch<ListSessionsResponse>(
        getToken,
        `/projects/${encodeURIComponent(projectRef)}/sessions${query}`,
      );
    },

    createProjectSession(
      projectRef: string,
      body?: { title?: string; description?: string },
    ): Promise<CreateSessionResponse> {
      return apiFetch<CreateSessionResponse>(
        getToken,
        `/projects/${encodeURIComponent(projectRef)}/sessions`,
        {
          method: 'POST',
          body: JSON.stringify({
            title: body?.title,
            description: body?.description,
          }),
        },
      );
    },

    getSession(id: string): Promise<GetSessionResponse> {
      return apiFetch<GetSessionResponse>(getToken, `/sessions/${id}`);
    },

    getSessionEvents(
      id: string,
      params?: {
        level?: 'info' | 'warn' | 'error';
        source?: 'socratic_engine' | 'llm_router' | 'pipeline' | 'factory' | 'system';
        limit?: number;
        offset?: number;
      },
    ): Promise<SessionEventsResponse> {
      const qs = new URLSearchParams();
      if (params?.level) qs.set('level', params.level);
      if (params?.source) qs.set('source', params.source);
      if (params?.limit !== undefined) qs.set('limit', String(params.limit));
      if (params?.offset !== undefined) qs.set('offset', String(params.offset));
      const query = qs.toString() ? `?${qs.toString()}` : '';
      return apiFetch<SessionEventsResponse>(getToken, `/sessions/${id}/events${query}`);
    },

    listSessions(params?: { includeArchived?: boolean }): Promise<ListSessionsResponse> {
      const qs = new URLSearchParams();
      if (params?.includeArchived) qs.set('include_archived', 'true');
      const query = qs.toString() ? `?${qs.toString()}` : '';
      return apiFetch<ListSessionsResponse>(getToken, `/sessions${query}`);
    },

    updateSession(id: string, patch: { title?: string; archived?: boolean }): Promise<GetSessionResponse> {
      return apiFetch<GetSessionResponse>(getToken, `/sessions/${id}`, {
        method: 'PATCH',
        body: JSON.stringify(patch),
      });
    },

    duplicateSession(id: string, body?: { title?: string }): Promise<GetSessionResponse> {
      return apiFetch<GetSessionResponse>(getToken, `/sessions/${id}/duplicate`, {
        method: 'POST',
        body: JSON.stringify(body ?? {}),
      });
    },

    exportSession(id: string): Promise<SessionExportResponse> {
      return apiFetch<SessionExportResponse>(getToken, `/sessions/${id}/export`);
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

    startSocratic(id: string, description: string, projectRef?: string): Promise<StartSocraticResponse> {
      return apiFetch<StartSocraticResponse>(getToken, `/sessions/${id}/socratic`, {
        method: 'POST',
        body: JSON.stringify({ description, project_ref: projectRef }),
      });
    },

    restartFromDescription(id: string): Promise<GetSessionResponse> {
      return apiFetch<GetSessionResponse>(getToken, `/sessions/${id}/restart-from-description`, {
        method: 'POST',
        body: '{}',
      });
    },

    retryPipeline(id: string): Promise<GetSessionResponse> {
      return apiFetch<GetSessionResponse>(getToken, `/sessions/${id}/retry-pipeline`, {
        method: 'POST',
        body: '{}',
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
    getBlueprint(params?: {
      nodeType?: string;
      scopeClass?: ScopeClass;
      scopeVisibility?: ScopeVisibility;
      lifecycle?: NodeLifecycle;
      projectId?: string;
      feature?: string;
      widget?: string;
      artifact?: string;
      component?: string;
      includeShared?: boolean;
      includeGlobal?: boolean;
    }): Promise<BlueprintResponse> {
      return apiFetch<BlueprintResponse>(getToken, `/blueprint${buildNodeQuery(params)}`);
    },

    /** GET /blueprint/nodes?type=<type> — List nodes, optionally by type. */
    listBlueprintNodes(params?: {
      nodeType?: string;
      scopeClass?: ScopeClass;
      scopeVisibility?: ScopeVisibility;
      lifecycle?: NodeLifecycle;
      projectId?: string;
      feature?: string;
      widget?: string;
      artifact?: string;
      component?: string;
      includeShared?: boolean;
      includeGlobal?: boolean;
    }): Promise<NodeListResponse> {
      return apiFetch<NodeListResponse>(getToken, `/blueprint/nodes${buildNodeQuery(params)}`);
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

    /** PATCH /blueprint/nodes/:id — Apply a JSON Merge Patch or send a full node. */
    updateBlueprintNode(nodeId: string, node: Partial<BlueprintNode> | BlueprintNode): Promise<BlueprintNode> {
      return apiFetch<BlueprintNode>(getToken, `/blueprint/nodes/${encodeURIComponent(nodeId)}`, {
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

    /** POST /blueprint/exports — Record a durable export event. */
    recordBlueprintExport(payload: {
      kind: BlueprintExportKind;
      nodeId?: string;
      nodeCount: number;
      edgeCount?: number;
      projectId?: string;
      projectName?: string;
      scopeSnapshot?: Record<string, unknown>;
    }): Promise<{ export_id: string; recorded_at: string }> {
      return apiFetch<{ export_id: string; recorded_at: string }>(getToken, '/blueprint/exports', {
        method: 'POST',
        body: JSON.stringify({
          kind: payload.kind,
          node_id: payload.nodeId,
          node_count: payload.nodeCount,
          edge_count: payload.edgeCount ?? 0,
          project_id: payload.projectId,
          project_name: payload.projectName,
          scope_snapshot: payload.scopeSnapshot,
        }),
      });
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

    async reconvergeBlueprintWs(
      req: ReconvergenceRequest,
      callbacks: {
        onStep: (step: ReconvergenceResult['steps'][number]) => void;
        onComplete: (summary: ReconvergenceResult['summary']) => void;
        onError: (message: string) => void;
      },
    ): Promise<WebSocket> {
      const token = await getToken();
      const ws = new WebSocket(buildWebSocketUrl('/blueprint/reconverge/ws', token));

      ws.addEventListener('open', () => {
        ws.send(JSON.stringify(req));
      });

      ws.addEventListener('message', event => {
        try {
          const payload = JSON.parse(event.data as string) as
            | { type: 'step'; step_id: string; node_id: string; node_name: string; node_type: string; action: string; severity: string; description: string; status: string; error?: string }
            | { type: 'summary'; total: number; applied: number; skipped: number; errors: number; needs_review: number }
            | { type: 'error'; message: string };

          if (payload.type === 'step') {
            callbacks.onStep(payload as ReconvergenceResult['steps'][number]);
            return;
          }
          if (payload.type === 'summary') {
            callbacks.onComplete(payload);
            return;
          }
          callbacks.onError(payload.message);
        } catch (error) {
          callbacks.onError(error instanceof Error ? error.message : 'Failed to parse reconvergence stream');
        }
      });

      ws.addEventListener('error', () => {
        callbacks.onError('WebSocket connection failed');
      });

      return ws;
    },

    // ─── Discovery ────────────────────────────────────────────────────────

    /** POST /blueprint/discovery/scan — Trigger automated scanner(s). */
    runDiscoveryScan(req: DiscoveryScanRequest): Promise<DiscoveryRunResponse> {
      return apiFetch<DiscoveryRunResponse>(getToken, '/blueprint/discovery/scan', {
        method: 'POST',
        body: JSON.stringify(req),
      });
    },

    /** GET /blueprint/discovery/proposals — List proposed nodes from scanners. */
    listProposedNodes(status?: string): Promise<ProposedNodesResponse> {
      const qs = status ? `?status=${encodeURIComponent(status)}` : '';
      return apiFetch<ProposedNodesResponse>(getToken, `/blueprint/discovery/proposals${qs}`);
    },

    /** POST /blueprint/discovery/proposals/:id/accept — Accept a proposed node into blueprint. */
    acceptProposal(
      proposalId: string,
      payload?: { node_patch?: Record<string, unknown> },
    ): Promise<{ node_id: string; message: string }> {
      return apiFetch<{ node_id: string; message: string }>(getToken, `/blueprint/discovery/proposals/${encodeURIComponent(proposalId)}/accept`, {
        method: 'POST',
        body: JSON.stringify(payload ?? {}),
      });
    },

    /** POST /blueprint/discovery/proposals/:id/reject — Reject a proposed node. */
    rejectProposal(proposalId: string, reason?: string): Promise<{ message: string }> {
      return apiFetch<{ message: string }>(getToken, `/blueprint/discovery/proposals/${encodeURIComponent(proposalId)}/reject`, {
        method: 'POST',
        body: JSON.stringify({ reason }),
      });
    },
  };
}

export type ApiClient = ReturnType<typeof createApiClient>;
