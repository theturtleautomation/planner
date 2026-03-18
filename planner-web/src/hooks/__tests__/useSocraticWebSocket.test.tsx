import { act, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useSocraticWebSocket } from '../useSocraticWebSocket.ts';
import type { ServerWsMessage } from '../../types.ts';

class MockWebSocket {
  static CONNECTING = 0;
  static OPEN = 1;
  static CLOSING = 2;
  static CLOSED = 3;

  readyState = MockWebSocket.CONNECTING;
  sent: string[] = [];
  onopen: ((event: Event) => void) | null = null;
  onclose: ((event: CloseEvent) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;
  onmessage: ((event: MessageEvent) => void) | null = null;

  constructor(public readonly url: string) {
    mockSockets.push(this);
  }

  send(data: string): void {
    this.sent.push(data);
  }

  close(): void {
    this.readyState = MockWebSocket.CLOSED;
    this.onclose?.(new CloseEvent('close'));
  }

  open(): void {
    this.readyState = MockWebSocket.OPEN;
    this.onopen?.(new Event('open'));
  }

  emit(message: ServerWsMessage): void {
    this.onmessage?.(
      new MessageEvent('message', {
        data: JSON.stringify(message),
      }),
    );
  }
}

const mockSockets: MockWebSocket[] = [];
const originalWebSocket = globalThis.WebSocket;

describe('useSocraticWebSocket', () => {
  beforeEach(() => {
    mockSockets.length = 0;
    globalThis.WebSocket = MockWebSocket as unknown as typeof WebSocket;
  });

  afterEach(() => {
    globalThis.WebSocket = originalWebSocket;
  });

  it('sendDone only sends protocol message and does not append optimistic transcript text', async () => {
    const getToken = vi.fn().mockResolvedValue('token');
    const { result } = renderHook(() => useSocraticWebSocket({
      sessionId: 'session-1',
      getToken,
      initialSession: null,
    }));

    act(() => {
      result.current.attach();
    });

    await waitFor(() => {
      expect(mockSockets.length).toBe(1);
    });

    const socket = mockSockets[0];
    act(() => {
      socket.open();
    });

    const messageCountBefore = result.current.messages.length;

    act(() => {
      result.current.sendDone();
    });

    expect(
      socket.sent.some((payload) => JSON.parse(payload).type === 'done'),
    ).toBe(true);
    expect(result.current.messages).toHaveLength(messageCountBefore);
  });

  it('updates pipeline stage state from planner_event metadata', async () => {
    const getToken = vi.fn().mockResolvedValue('token');
    const { result } = renderHook(() => useSocraticWebSocket({
      sessionId: 'session-2',
      getToken,
      initialSession: null,
    }));

    act(() => {
      result.current.attach();
    });

    await waitFor(() => {
      expect(mockSockets.length).toBe(1);
    });

    const socket = mockSockets[0];
    act(() => {
      socket.open();
    });

    act(() => {
      socket.emit({
        type: 'planner_event',
        id: 'evt-stage-start',
        timestamp: new Date().toISOString(),
        level: 'info',
        source: 'pipeline',
        step: 'pipeline.stage.started',
        message: 'Factory stage started',
        metadata: { stage: 'Factory' },
      });
    });

    expect(
      result.current.stages.find((stage) => stage.name === 'Factory')?.status,
    ).toBe('running');

    act(() => {
      socket.emit({
        type: 'planner_event',
        id: 'evt-validation',
        timestamp: new Date().toISOString(),
        level: 'info',
        source: 'pipeline',
        step: 'pipeline.validation.completed',
        message: 'Validation failed',
        metadata: { stage: 'Validate', gates_passed: false },
      });
    });

    expect(
      result.current.stages.find((stage) => stage.name === 'Validate')?.status,
    ).toBe('failed');
  });

  it('derives pipeline stage from planner_event message when stage metadata is missing', async () => {
    const getToken = vi.fn().mockResolvedValue('token');
    const { result } = renderHook(() => useSocraticWebSocket({
      sessionId: 'session-3',
      getToken,
      initialSession: null,
    }));

    act(() => {
      result.current.attach();
    });

    await waitFor(() => {
      expect(mockSockets.length).toBe(1);
    });

    const socket = mockSockets[0];
    act(() => {
      socket.open();
    });

    act(() => {
      socket.emit({
        type: 'planner_event',
        id: 'evt-stage-message',
        timestamp: new Date().toISOString(),
        level: 'info',
        source: 'pipeline',
        step: 'pipeline.stage.started',
        message: 'Graph compilation stage started',
        metadata: {},
      });
    });

    expect(
      result.current.stages.find((stage) => stage.name === 'Graph')?.status,
    ).toBe('running');
  });

  it('falls back to the sole running stage when completion metadata omits stage', async () => {
    const getToken = vi.fn().mockResolvedValue('token');
    const { result } = renderHook(() => useSocraticWebSocket({
      sessionId: 'session-4',
      getToken,
      initialSession: null,
    }));

    act(() => {
      result.current.attach();
    });

    await waitFor(() => {
      expect(mockSockets.length).toBe(1);
    });

    const socket = mockSockets[0];
    act(() => {
      socket.open();
    });

    act(() => {
      socket.emit({
        type: 'planner_event',
        id: 'evt-stage-start-compile',
        timestamp: new Date().toISOString(),
        level: 'info',
        source: 'pipeline',
        step: 'pipeline.stage.started',
        message: 'Compile stage started',
        metadata: { stage: 'Compile' },
      });
    });

    act(() => {
      socket.emit({
        type: 'planner_event',
        id: 'evt-stage-complete-missing-stage',
        timestamp: new Date().toISOString(),
        level: 'info',
        source: 'pipeline',
        step: 'pipeline.stage.completed',
        message: 'Stage completed',
        metadata: {},
      });
    });

    expect(
      result.current.stages.find((stage) => stage.name === 'Compile')?.status,
    ).toBe('complete');
  });

  it('handles retry-heavy planner_event sequences without losing stage coherence', async () => {
    const getToken = vi.fn().mockResolvedValue('token');
    const { result } = renderHook(() => useSocraticWebSocket({
      sessionId: 'session-5',
      getToken,
      initialSession: null,
    }));

    act(() => {
      result.current.attach();
    });

    await waitFor(() => {
      expect(mockSockets.length).toBe(1);
    });

    const socket = mockSockets[0];
    act(() => {
      socket.open();
    });

    act(() => {
      socket.emit({
        type: 'planner_event',
        id: 'evt-factory-start',
        timestamp: new Date().toISOString(),
        level: 'info',
        source: 'pipeline',
        step: 'pipeline.stage.started',
        message: 'Factory stage started (attempt 1/3)',
        metadata: { stage: 'Factory' },
      });
    });
    expect(
      result.current.stages.find((stage) => stage.name === 'Factory')?.status,
    ).toBe('running');

    act(() => {
      socket.emit({
        type: 'planner_event',
        id: 'evt-validate-failed',
        timestamp: new Date().toISOString(),
        level: 'info',
        source: 'pipeline',
        step: 'pipeline.validation.completed',
        message: 'Validation attempt 1 failed',
        metadata: { stage: 'Validate', gates_passed: false },
      });
      socket.emit({
        type: 'planner_event',
        id: 'evt-retry-started',
        timestamp: new Date().toISOString(),
        level: 'warn',
        source: 'pipeline',
        step: 'pipeline.retry.started',
        message: 'Retrying pipeline validation loop',
        metadata: { details: { stage: 'Factory' } },
      });
      socket.emit({
        type: 'planner_event',
        id: 'evt-validate-passed',
        timestamp: new Date().toISOString(),
        level: 'info',
        source: 'pipeline',
        step: 'pipeline.validation.completed',
        message: 'Validation attempt 2 passed',
        metadata: { stage: 'Validate', gates_passed: true },
      });
    });

    expect(
      result.current.stages.find((stage) => stage.name === 'Factory')?.status,
    ).toBe('running');
    expect(
      result.current.stages.find((stage) => stage.name === 'Validate')?.status,
    ).toBe('complete');
  });
});
