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
  };
}

export type ApiClient = ReturnType<typeof createApiClient>;
