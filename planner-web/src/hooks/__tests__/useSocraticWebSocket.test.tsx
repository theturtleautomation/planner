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
    vi.useRealTimers();
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

  it('hydrates category state and sends category navigation actions', async () => {
    const getToken = vi.fn().mockResolvedValue('token');
    const { result } = renderHook(() => useSocraticWebSocket({
      sessionId: 'session-category',
      getToken,
      initialSession: null,
    }));

    act(() => {
      result.current.attach();
    });

    await waitFor(() => {
      expect(mockSockets.length).toBeGreaterThan(0);
    });

    const socket = mockSockets.at(-1)!;
    act(() => {
      socket.open();
      socket.emit({
        type: 'category_state',
        snapshot: {
          revision: 'category-1',
          root_category_ids: ['root-discovery'],
          nodes: [
            {
              category_id: 'root-discovery',
              parent_category_id: null,
              title: 'Explore missing areas',
              summary: '1 area still needs discovery.',
              status: 'ready',
              depth: 0,
              mapped_dimensions: [],
              has_children: true,
              has_prompt_ready: false,
              item_count_hint: 1,
            },
          ],
          active_category_path: [],
          newly_available_category_ids: [],
          build_ready: false,
          build_readiness_message: 'Build is blocked until 1 remaining area is explored.',
        },
      });
    });

    expect(result.current.currentCategorySnapshot?.revision).toBe('category-1');

    act(() => {
      result.current.enterCategory('root-discovery', 'category-1');
      result.current.backToCategories();
    });

    expect(
      socket.sent.some((payload) => {
        const parsed = JSON.parse(payload);
        return parsed.type === 'enter_category' && parsed.category_id === 'root-discovery';
      }),
    ).toBe(true);
    expect(
      socket.sent.some((payload) => JSON.parse(payload).type === 'back_to_categories'),
    ).toBe(true);
  });

  it('keeps category focus pending until workspace state or prompt resolves it', async () => {
    const getToken = vi.fn().mockResolvedValue('token');
    const { result } = renderHook(() => useSocraticWebSocket({
      sessionId: 'session-workspace',
      getToken,
      initialSession: null,
    }));

    act(() => {
      result.current.attach();
    });

    await waitFor(() => {
      expect(mockSockets.length).toBeGreaterThan(0);
    });

    const socket = mockSockets.at(-1)!;
    act(() => {
      socket.open();
    });

    act(() => {
      result.current.enterCategory('root-discovery::dimension::security', 'category-2');
    });

    expect(result.current.pendingCategoryId).toBe('root-discovery::dimension::security');

    act(() => {
      socket.emit({
        type: 'category_state',
        snapshot: {
          revision: 'category-2',
          root_category_ids: ['root-discovery'],
          nodes: [
            {
              category_id: 'root-discovery',
              parent_category_id: null,
              title: 'Explore missing areas',
              summary: '1 area still needs discovery.',
              status: 'active',
              depth: 0,
              mapped_dimensions: [],
              has_children: true,
              has_prompt_ready: false,
              item_count_hint: 1,
            },
            {
              category_id: 'root-discovery::dimension::security',
              parent_category_id: 'root-discovery',
              title: 'Security',
              summary: 'Authentication still needs definition.',
              status: 'ready',
              depth: 1,
              mapped_dimensions: ['Security'],
              has_children: false,
              has_prompt_ready: true,
              item_count_hint: 2,
            },
          ],
          active_category_path: [
            { category_id: 'root-discovery', title: 'Explore missing areas' },
          ],
          newly_available_category_ids: [],
          build_ready: false,
          build_readiness_message: 'Build is blocked until security is clarified.',
        },
      });
    });

    expect(result.current.pendingCategoryId).toBe('root-discovery::dimension::security');

    act(() => {
      socket.emit({
        type: 'workspace_state',
        workspace: {
          revision: 'workspace-2',
          focused_category_id: 'root-discovery::dimension::security',
          branch_notice: null,
          category_snapshot: {
            revision: 'category-2',
            root_category_ids: ['root-discovery'],
            nodes: [
              {
                category_id: 'root-discovery',
                parent_category_id: null,
                title: 'Explore missing areas',
                summary: '1 area still needs discovery.',
                status: 'active',
                depth: 0,
                mapped_dimensions: [],
                has_children: true,
                has_prompt_ready: false,
                item_count_hint: 1,
              },
              {
                category_id: 'root-discovery::dimension::security',
                parent_category_id: 'root-discovery',
                title: 'Security',
                summary: 'Authentication still needs definition.',
                status: 'ready',
                depth: 1,
                mapped_dimensions: ['Security'],
                has_children: false,
                has_prompt_ready: true,
                item_count_hint: 2,
              },
            ],
            active_category_path: [
              { category_id: 'root-discovery', title: 'Explore missing areas' },
              { category_id: 'root-discovery::dimension::security', title: 'Security' },
            ],
            newly_available_category_ids: ['root-discovery::dimension::security'],
            build_ready: false,
            build_readiness_message: 'Build is blocked until security is clarified.',
          },
          groups: [
            {
              category_id: 'root-discovery::dimension::security',
              title: 'Security',
              summary: 'Authentication still needs definition.',
              status: 'ready',
              question_count: 2,
              is_focused: true,
              is_new: true,
              preview_items: [
                {
                  item_id: 'root-discovery::dimension::security::preview::0',
                  kind: 'discovery',
                  text: 'How should authentication work?',
                },
              ],
            },
          ],
        },
      });
    });

    expect(result.current.pendingCategoryId).toBeNull();
    expect(result.current.currentWorkspace?.focused_category_id).toBe('root-discovery::dimension::security');
  });

  it('stores workspace state and branch notices from websocket updates', async () => {
    const getToken = vi.fn().mockResolvedValue('token');
    const { result } = renderHook(() => useSocraticWebSocket({
      sessionId: 'session-workspace-branch',
      getToken,
      initialSession: null,
    }));

    act(() => {
      result.current.attach();
    });

    await waitFor(() => {
      expect(mockSockets.length).toBeGreaterThan(0);
    });

    const socket = mockSockets.at(-1)!;
    act(() => {
      socket.open();
      socket.emit({
        type: 'workspace_state',
        workspace: {
          revision: 'workspace-branch-1',
          focused_category_id: null,
          branch_notice: '"Authentication model" no longer has active questions. Review the updated workspace for the remaining work.',
          category_snapshot: {
            revision: 'category-branch-1',
            root_category_ids: ['root-discovery'],
            nodes: [
              {
                category_id: 'root-discovery',
                parent_category_id: null,
                title: 'Explore missing areas',
                summary: 'Review the remaining branches.',
                status: 'active',
                depth: 0,
                mapped_dimensions: [],
                has_children: true,
                has_prompt_ready: false,
                item_count_hint: 1,
              },
            ],
            active_category_path: [],
            newly_available_category_ids: [],
            build_ready: false,
            build_readiness_message: 'Build is blocked until the remaining branch is reviewed.',
          },
          groups: [
            {
              category_id: 'root-discovery',
              title: 'Explore missing areas',
              summary: 'Review the remaining branches.',
              status: 'active',
              question_count: 1,
              is_focused: false,
              is_new: false,
              preview_items: [
                {
                  item_id: 'root-discovery::preview::0',
                  kind: 'discovery',
                  text: 'Clarify the remaining branch.',
                },
              ],
            },
          ],
        },
      });
    });

    expect(result.current.currentWorkspace?.revision).toBe('workspace-branch-1');
    expect(result.current.currentCategorySnapshot?.revision).toBe('category-branch-1');
    expect(result.current.workspaceNotice).toMatch(/no longer has active questions/i);
  });

  it('reconnect replay restores deep category state without reviving a stale prompt', async () => {
    const getToken = vi.fn().mockResolvedValue('token');
    const { result } = renderHook(() => useSocraticWebSocket({
      sessionId: 'session-category-replay',
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
      socket.emit({
        type: 'category_state',
        snapshot: {
          revision: 'category-deep-1',
          root_category_ids: ['root-discovery'],
          nodes: [
            {
              category_id: 'root-discovery',
              parent_category_id: null,
              title: 'Explore missing areas',
              summary: '1 area still needs discovery.',
              status: 'active',
              depth: 0,
              mapped_dimensions: [],
              has_children: true,
              has_prompt_ready: false,
              item_count_hint: 1,
            },
            {
              category_id: 'root-discovery::dimension::security',
              parent_category_id: 'root-discovery',
              title: 'Security',
              summary: 'Authentication model still needs definition.',
              status: 'ready',
              depth: 1,
              mapped_dimensions: ['Security'],
              has_children: false,
              has_prompt_ready: true,
              item_count_hint: 1,
            },
          ],
          active_category_path: [
            { category_id: 'root-discovery', title: 'Explore missing areas' },
          ],
          newly_available_category_ids: [],
          build_ready: false,
          build_readiness_message: 'Build is blocked until the remaining category is explored.',
        },
      });
    });

    expect(result.current.currentCategorySnapshot?.revision).toBe('category-deep-1');
    expect(result.current.currentPrompt).toBeNull();

    act(() => {
      socket.close();
    });

    await new Promise((resolve) => {
      setTimeout(resolve, 1200);
    });
    expect(mockSockets.length).toBe(2);

    const replaySocket = mockSockets[1];
    act(() => {
      replaySocket.open();
      replaySocket.emit({
        type: 'category_state',
        snapshot: {
          revision: 'category-deep-1',
          root_category_ids: ['root-discovery'],
          nodes: [
            {
              category_id: 'root-discovery',
              parent_category_id: null,
              title: 'Explore missing areas',
              summary: '1 area still needs discovery.',
              status: 'active',
              depth: 0,
              mapped_dimensions: [],
              has_children: true,
              has_prompt_ready: false,
              item_count_hint: 1,
            },
            {
              category_id: 'root-discovery::dimension::security',
              parent_category_id: 'root-discovery',
              title: 'Security',
              summary: 'Authentication model still needs definition.',
              status: 'ready',
              depth: 1,
              mapped_dimensions: ['Security'],
              has_children: false,
              has_prompt_ready: true,
              item_count_hint: 1,
            },
          ],
          active_category_path: [
            { category_id: 'root-discovery', title: 'Explore missing areas' },
          ],
          newly_available_category_ids: [],
          build_ready: false,
          build_readiness_message: 'Build is blocked until the remaining category is explored.',
        },
      });
    });

    expect(result.current.currentCategorySnapshot?.active_category_path).toEqual([
      { category_id: 'root-discovery', title: 'Explore missing areas' },
    ]);
    expect(result.current.currentPrompt).toBeNull();
  });

  it('reconnect replay restores prompt state with deep category breadcrumbs', async () => {
    const getToken = vi.fn().mockResolvedValue('token');
    const { result } = renderHook(() => useSocraticWebSocket({
      sessionId: 'session-prompt-replay',
      getToken,
      initialSession: null,
    }));

    act(() => {
      result.current.attach();
    });

    await waitFor(() => {
      expect(mockSockets.length).toBeGreaterThan(0);
    });

    const socket = mockSockets[0];
    act(() => {
      socket.open();
      socket.emit({
        type: 'prompt',
        prompt: {
          prompt_id: 'prompt-deep-1',
          kind: 'question_batch',
          title: 'Clarify security',
          instructions: 'Answer the scoped security question.',
          origin_category_id: 'root-discovery::dimension::security::auth',
          category_path: [
            { category_id: 'root-discovery', title: 'Explore missing areas' },
            { category_id: 'root-discovery::dimension::security', title: 'Security' },
            { category_id: 'root-discovery::dimension::security::auth', title: 'Authentication model' },
          ],
          items: [
            {
              item_id: 'item-security',
              kind: 'discovery',
              target_dimension: 'Security',
              text: 'How should authentication work?',
              options: [],
              response_mode: 'single_select_with_custom_text',
              required: false,
              priority: 100,
              dependency_item_ids: [],
            },
          ],
          required_item_ids: [],
          allow_partial_submit: true,
          ui_hints: {
            preferred_layout: 'cards',
            show_draft_sidebar: false,
          },
          based_on_turn: 2,
          created_at: '2026-03-21T00:00:00Z',
        },
      });
    });

    expect(result.current.currentPrompt?.prompt_id).toBe('prompt-deep-1');
    expect(result.current.currentPrompt?.category_path).toHaveLength(3);

    act(() => {
      socket.close();
    });

    await new Promise((resolve) => {
      setTimeout(resolve, 1200);
    });
    expect(mockSockets.length).toBe(2);

    const replaySocket = mockSockets[1];
    act(() => {
      replaySocket.open();
      replaySocket.emit({
        type: 'prompt',
        prompt: {
          prompt_id: 'prompt-deep-1',
          kind: 'question_batch',
          title: 'Clarify security',
          instructions: 'Answer the scoped security question.',
          origin_category_id: 'root-discovery::dimension::security::auth',
          category_path: [
            { category_id: 'root-discovery', title: 'Explore missing areas' },
            { category_id: 'root-discovery::dimension::security', title: 'Security' },
            { category_id: 'root-discovery::dimension::security::auth', title: 'Authentication model' },
          ],
          items: [
            {
              item_id: 'item-security',
              kind: 'discovery',
              target_dimension: 'Security',
              text: 'How should authentication work?',
              options: [],
              response_mode: 'single_select_with_custom_text',
              required: false,
              priority: 100,
              dependency_item_ids: [],
            },
          ],
          required_item_ids: [],
          allow_partial_submit: true,
          ui_hints: {
            preferred_layout: 'cards',
            show_draft_sidebar: false,
          },
          based_on_turn: 2,
          created_at: '2026-03-21T00:00:00Z',
        },
      });
    });

    expect(result.current.currentPrompt?.category_path).toEqual([
      { category_id: 'root-discovery', title: 'Explore missing areas' },
      { category_id: 'root-discovery::dimension::security', title: 'Security' },
      { category_id: 'root-discovery::dimension::security::auth', title: 'Authentication model' },
    ]);
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

  it('does not reconnect after a terminal pipeline error closes the socket', async () => {
    const getToken = vi.fn().mockResolvedValue('token');
    const { result } = renderHook(() => useSocraticWebSocket({
      sessionId: 'session-error',
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
    vi.useFakeTimers();

    act(() => {
      socket.open();
      socket.emit({
        type: 'planner_event',
        id: 'evt-pipeline-start',
        timestamp: new Date().toISOString(),
        level: 'info',
        source: 'pipeline',
        step: 'pipeline.stage.started',
        message: 'Chunk stage started',
        metadata: { stage: 'Chunk' },
      });
      socket.emit({
        type: 'error',
        message: 'Pipeline failed: LLM call failed: Not logged in',
      });
      socket.close();
    });

    expect(result.current.intakePhase).toBe('error');

    act(() => {
      vi.advanceTimersByTime(10_000);
    });

    expect(mockSockets).toHaveLength(1);
    expect(result.current.reconnectFailed).toBe(false);
  });
});
