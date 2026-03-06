import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { createApiClient, isAuthError, ApiError } from '../client';

// Mock the config module so API_BASE is a known value
vi.mock('../../config', () => ({
  API_BASE: '/api',
  AUTH0_ENABLED: false,
  AUTH0_DOMAIN: '',
  AUTH0_CLIENT_ID: '',
  AUTH0_AUDIENCE: '',
  WS_PROTOCOL: 'ws:',
}));

const mockGetToken = vi.fn().mockResolvedValue('mock-token');

describe('createApiClient', () => {
  let fetchSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    fetchSpy = vi.spyOn(global, 'fetch');
  });

  afterEach(() => {
    fetchSpy.mockRestore();
  });

  function makeFetchResponse(data: unknown, status = 200): Response {
    return {
      ok: status >= 200 && status < 300,
      status,
      statusText: status === 200 ? 'OK' : 'Error',
      json: vi.fn().mockResolvedValue(data),
      text: vi.fn().mockResolvedValue(JSON.stringify(data)),
    } as unknown as Response;
  }

  it('getSession makes GET request to correct endpoint', async () => {
    const sessionData = { session: { id: 'abc', messages: [], stages: [], pipeline_running: false, intake_phase: 'waiting' } };
    fetchSpy.mockResolvedValue(makeFetchResponse(sessionData));
    const api = createApiClient(mockGetToken);
    const result = await api.getSession('abc');
    expect(fetchSpy).toHaveBeenCalledWith(
      '/api/sessions/abc',
      expect.objectContaining({ headers: expect.any(Headers) }),
    );
    expect(result).toEqual(sessionData);
  });

  it('createSession makes POST request to /api/sessions', async () => {
    const sessionData = { session: { id: 'new-id', messages: [], stages: [], pipeline_running: false, intake_phase: 'waiting' } };
    fetchSpy.mockResolvedValue(makeFetchResponse(sessionData));
    const api = createApiClient(mockGetToken);
    await api.createSession();
    expect(fetchSpy).toHaveBeenCalledWith(
      '/api/sessions',
      expect.objectContaining({ method: 'POST' }),
    );
  });

  it('createSession sends {} as body', async () => {
    const sessionData = { session: { id: 'new-id', messages: [], stages: [], pipeline_running: false, intake_phase: 'waiting' } };
    fetchSpy.mockResolvedValue(makeFetchResponse(sessionData));
    const api = createApiClient(mockGetToken);
    await api.createSession();
    const callArgs = fetchSpy.mock.calls[0][1] as RequestInit;
    expect(callArgs.body).toBe('{}');
  });

  it('listSessions makes GET request to /api/sessions', async () => {
    fetchSpy.mockResolvedValue(makeFetchResponse({ sessions: [] }));
    const api = createApiClient(mockGetToken);
    await api.listSessions();
    expect(fetchSpy).toHaveBeenCalledWith(
      '/api/sessions',
      expect.objectContaining({ headers: expect.any(Headers) }),
    );
  });

  it('sendMessage makes POST request with correct payload', async () => {
    const msgData = {
      user_message: { id: '1', role: 'user', content: 'Hi', timestamp: '' },
      planner_message: { id: '2', role: 'planner', content: 'Hello', timestamp: '' },
      session: { id: 'sess-1', messages: [], stages: [], pipeline_running: false, intake_phase: 'waiting' },
    };
    fetchSpy.mockResolvedValue(makeFetchResponse(msgData));
    const api = createApiClient(mockGetToken);
    await api.sendMessage('sess-1', 'Hi there');
    expect(fetchSpy).toHaveBeenCalledWith(
      '/api/sessions/sess-1/message',
      expect.objectContaining({ method: 'POST' }),
    );
    const callArgs = fetchSpy.mock.calls[0][1] as RequestInit;
    expect(JSON.parse(callArgs.body as string)).toEqual({ content: 'Hi there' });
  });

  it('sets Authorization Bearer token in request headers', async () => {
    fetchSpy.mockResolvedValue(makeFetchResponse({ sessions: [] }));
    const api = createApiClient(mockGetToken);
    await api.listSessions();
    const callArgs = fetchSpy.mock.calls[0][1] as RequestInit;
    const headers = callArgs.headers as Headers;
    expect(headers.get('Authorization')).toBe('Bearer mock-token');
  });

  it('sets Content-Type to application/json', async () => {
    fetchSpy.mockResolvedValue(makeFetchResponse({ sessions: [] }));
    const api = createApiClient(mockGetToken);
    await api.listSessions();
    const callArgs = fetchSpy.mock.calls[0][1] as RequestInit;
    const headers = callArgs.headers as Headers;
    expect(headers.get('Content-Type')).toBe('application/json');
  });

  it('throws ApiError on non-OK response with status code', async () => {
    fetchSpy.mockResolvedValue(makeFetchResponse({ detail: 'Not found' }, 404));
    const api = createApiClient(mockGetToken);
    await expect(api.getSession('missing')).rejects.toBeInstanceOf(ApiError);
  });

  it('ApiError has the correct status code', async () => {
    fetchSpy.mockResolvedValue(makeFetchResponse({ detail: 'Not found' }, 404));
    const api = createApiClient(mockGetToken);
    try {
      await api.getSession('missing');
    } catch (e) {
      expect(e).toBeInstanceOf(ApiError);
      expect((e as ApiError).status).toBe(404);
    }
  });

  it('throws ApiError with 500 status for server errors', async () => {
    fetchSpy.mockResolvedValue(makeFetchResponse('Internal Server Error', 500));
    const api = createApiClient(mockGetToken);
    try {
      await api.listSessions();
    } catch (e) {
      expect(e).toBeInstanceOf(ApiError);
      expect((e as ApiError).status).toBe(500);
    }
  });

  it('health makes GET request to /api/health', async () => {
    fetchSpy.mockResolvedValue(makeFetchResponse({ status: 'ok', version: '1.0', sessions_active: 0 }));
    const api = createApiClient(mockGetToken);
    await api.health();
    expect(fetchSpy).toHaveBeenCalledWith('/api/health', expect.any(Object));
  });

  it('startSocratic makes POST request to /sessions/:id/socratic', async () => {
    const responseData = { session_id: 'sess-1', ws_url: 'ws://localhost/ws/sess-1' };
    fetchSpy.mockResolvedValue(makeFetchResponse(responseData));
    const api = createApiClient(mockGetToken);
    await api.startSocratic('sess-1', 'Build a todo app');
    expect(fetchSpy).toHaveBeenCalledWith(
      '/api/sessions/sess-1/socratic',
      expect.objectContaining({ method: 'POST' }),
    );
  });

  it('startSocratic sends description in request body', async () => {
    const responseData = { session_id: 'sess-1', ws_url: 'ws://localhost/ws/sess-1' };
    fetchSpy.mockResolvedValue(makeFetchResponse(responseData));
    const api = createApiClient(mockGetToken);
    await api.startSocratic('sess-1', 'Build a todo app');
    const callArgs = fetchSpy.mock.calls[0][1] as RequestInit;
    expect(JSON.parse(callArgs.body as string)).toEqual({ description: 'Build a todo app' });
  });

  it('startSocratic returns session_id and ws_url', async () => {
    const responseData = { session_id: 'sess-1', ws_url: 'ws://localhost/ws/sess-1' };
    fetchSpy.mockResolvedValue(makeFetchResponse(responseData));
    const api = createApiClient(mockGetToken);
    const result = await api.startSocratic('sess-1', 'Build a todo app');
    expect(result).toEqual(responseData);
  });

  it('restartFromDescription makes POST request to /sessions/:id/restart-from-description', async () => {
    const responseData = {
      session: { id: 'sess-1', messages: [], stages: [], pipeline_running: false, intake_phase: 'interviewing' },
    };
    fetchSpy.mockResolvedValue(makeFetchResponse(responseData));
    const api = createApiClient(mockGetToken);
    await api.restartFromDescription('sess-1');
    expect(fetchSpy).toHaveBeenCalledWith(
      '/api/sessions/sess-1/restart-from-description',
      expect.objectContaining({ method: 'POST' }),
    );
  });

  it('retryPipeline makes POST request to /sessions/:id/retry-pipeline', async () => {
    const responseData = {
      session: { id: 'sess-1', messages: [], stages: [], pipeline_running: true, intake_phase: 'pipeline_running' },
    };
    fetchSpy.mockResolvedValue(makeFetchResponse(responseData));
    const api = createApiClient(mockGetToken);
    await api.retryPipeline('sess-1');
    expect(fetchSpy).toHaveBeenCalledWith(
      '/api/sessions/sess-1/retry-pipeline',
      expect.objectContaining({ method: 'POST' }),
    );
  });

  it('getBeliefState makes GET request to /sessions/:id/belief-state', async () => {
    const responseData = {
      session_id: 'sess-1',
      intake_phase: 'interviewing',
      belief_state: null,
    };
    fetchSpy.mockResolvedValue(makeFetchResponse(responseData));
    const api = createApiClient(mockGetToken);
    await api.getBeliefState('sess-1');
    expect(fetchSpy).toHaveBeenCalledWith(
      '/api/sessions/sess-1/belief-state',
      expect.objectContaining({ headers: expect.any(Headers) }),
    );
  });

  it('getBeliefState returns belief state response', async () => {
    const responseData = {
      session_id: 'sess-1',
      intake_phase: 'interviewing',
      belief_state: {
        filled: { stack: { value: 'React', confidence: 0.9 } },
        uncertain: {},
        missing: ['deployment'],
        out_of_scope: [],
        convergence_pct: 45,
      },
    };
    fetchSpy.mockResolvedValue(makeFetchResponse(responseData));
    const api = createApiClient(mockGetToken);
    const result = await api.getBeliefState('sess-1');
    expect(result).toEqual(responseData);
  });

  it('getBeliefState uses GET method (no body)', async () => {
    const responseData = { session_id: 'sess-2', intake_phase: 'waiting', belief_state: null };
    fetchSpy.mockResolvedValue(makeFetchResponse(responseData));
    const api = createApiClient(mockGetToken);
    await api.getBeliefState('sess-2');
    const callArgs = fetchSpy.mock.calls[0][1] as RequestInit;
    // Default GET requests do not set a method or body
    expect(callArgs.method).toBeUndefined();
    expect(callArgs.body).toBeUndefined();
  });
});

describe('isAuthError', () => {
  it('returns true for ApiError with status 401', () => {
    const err = new ApiError('Unauthorized', 401);
    expect(isAuthError(err)).toBe(true);
  });

  it('returns true for ApiError with status 403', () => {
    const err = new ApiError('Forbidden', 403);
    expect(isAuthError(err)).toBe(true);
  });

  it('returns false for ApiError with status 404', () => {
    const err = new ApiError('Not Found', 404);
    expect(isAuthError(err)).toBe(false);
  });

  it('returns false for ApiError with status 500', () => {
    const err = new ApiError('Server Error', 500);
    expect(isAuthError(err)).toBe(false);
  });

  it('returns true for generic Error with 401 in message', () => {
    const err = new Error('Request failed with status 401');
    expect(isAuthError(err)).toBe(true);
  });

  it('returns true for generic Error with 403 in message', () => {
    const err = new Error('Access denied: 403 Forbidden');
    expect(isAuthError(err)).toBe(true);
  });

  it('returns false for generic Error without auth codes', () => {
    const err = new Error('Network error');
    expect(isAuthError(err)).toBe(false);
  });
});

describe('ApiError', () => {
  it('has name "ApiError"', () => {
    const err = new ApiError('test', 400);
    expect(err.name).toBe('ApiError');
  });

  it('is an instance of Error', () => {
    const err = new ApiError('test', 400);
    expect(err).toBeInstanceOf(Error);
  });

  it('stores status property', () => {
    const err = new ApiError('test', 422);
    expect(err.status).toBe(422);
  });

  it('stores message', () => {
    const err = new ApiError('Something went wrong', 400);
    expect(err.message).toBe('Something went wrong');
  });
});
