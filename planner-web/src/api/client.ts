import { API_BASE } from '../config.ts';
import type {
  HealthResponse,
  CreateSessionResponse,
  GetSessionResponse,
  SendMessageResponse,
  ListModelsResponse,
} from '../types.ts';

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
    throw new Error(`API ${init.method ?? 'GET'} ${path} → ${response.status}: ${text}`);
  }

  return response.json() as Promise<T>;
}

// ─── Factory ─────────────────────────────────────────────────────────────────

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

    sendMessage(id: string, content: string): Promise<SendMessageResponse> {
      return apiFetch<SendMessageResponse>(getToken, `/sessions/${id}/message`, {
        method: 'POST',
        body: JSON.stringify({ content }),
      });
    },

    listModels(): Promise<ListModelsResponse> {
      return apiFetch<ListModelsResponse>(getToken, '/models');
    },
  };
}

export type ApiClient = ReturnType<typeof createApiClient>;
