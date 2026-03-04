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
  };
}

export type ApiClient = ReturnType<typeof createApiClient>;
