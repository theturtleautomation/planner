import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import SessionPage from '../SessionPage';
import type { Session } from '../../types';
import { useSocraticWebSocket } from '../../hooks/useSocraticWebSocket';
import { resetSocraticDocumentGraph } from '../../stores/socraticDocumentStore.ts';
import { useSocraticDraftStore } from '../../stores/useSocraticDraftStore.ts';

const mockCreateSession = vi.fn();
const mockGetSession = vi.fn();
const mockStartSocratic = vi.fn();
const mockGetSessionEvents = vi.fn();
const mockRestartFromDescription = vi.fn();
const mockRetryPipeline = vi.fn();
const mockUpdateSession = vi.fn();
const mockDuplicateSession = vi.fn();
const mockExportSession = vi.fn();
const mockAttach = vi.fn();
const mockSendDescription = vi.fn();
const mockEnterCategory = vi.fn();
const mockBackToCategories = vi.fn();
const mockGetAccessToken = vi.fn().mockResolvedValue('mock-token');

vi.mock('../../api/client.ts', () => ({
  createApiClient: vi.fn(() => ({
    createSession: mockCreateSession,
    getSession: mockGetSession,
    getSessionEvents: mockGetSessionEvents,
    startSocratic: mockStartSocratic,
    restartFromDescription: mockRestartFromDescription,
    retryPipeline: mockRetryPipeline,
    updateSession: mockUpdateSession,
    duplicateSession: mockDuplicateSession,
    exportSession: mockExportSession,
  })),
}));

vi.mock('../../auth/useAuthenticatedFetch.ts', () => ({
  useGetAccessToken: vi.fn(() => mockGetAccessToken),
}));

vi.mock('../../hooks/useSocraticWebSocket.ts', () => ({
  useSocraticWebSocket: vi.fn(),
}));

function makeSession(overrides: Partial<Session>): Session {
  return {
    id: 'abc',
    title: null,
    archived: false,
    archived_at: null,
    messages: [
      {
        id: 'm1',
        role: 'system',
        content: 'welcome',
        timestamp: new Date().toISOString(),
      },
    ],
    stages: [
      { name: 'Intake', status: 'running' },
      { name: 'Chunk', status: 'pending' },
      { name: 'Compile', status: 'pending' },
      { name: 'Lint', status: 'pending' },
      { name: 'AR Review', status: 'pending' },
      { name: 'Refine', status: 'pending' },
      { name: 'Scenarios', status: 'pending' },
      { name: 'Ralph', status: 'pending' },
      { name: 'Graph', status: 'pending' },
      { name: 'Factory', status: 'pending' },
      { name: 'Validate', status: 'pending' },
      { name: 'Git', status: 'pending' },
    ],
    pipeline_running: false,
    intake_phase: 'waiting',
    interview_live_attached: false,
    can_resume_live: false,
    can_resume_checkpoint: false,
    can_restart_from_description: false,
    can_retry_pipeline: false,
    has_checkpoint: false,
    resume_status: 'ready_to_start',
    socratic_run_id: null,
    checkpoint: null,
    ...overrides,
  };
}

function LocationSnapshot() {
  const location = useLocation();
  return (
    <div>
      <div data-testid="location-path">{location.pathname}</div>
      <div data-testid="location-search">{location.search}</div>
    </div>
  );
}

function renderSessionPage(path = '/session/abc') {
  render(
    <MemoryRouter initialEntries={[path]}>
      <Routes>
        <Route path="/session/:id" element={<SessionPage />} />
        <Route path="/session/new" element={<SessionPage />} />
        <Route path="/knowledge/projects/:projectId" element={<LocationSnapshot />} />
      </Routes>
    </MemoryRouter>,
  );
}

function makeMockSocraticState(
  overrides: Partial<ReturnType<typeof useSocraticWebSocket>> = {},
): ReturnType<typeof useSocraticWebSocket> {
  return {
    isConnected: false,
    reconnectFailed: false,
    intakePhase: 'waiting',
    messages: [],
    classification: null,
    beliefState: null,
    convergencePct: 0,
    currentCategorySnapshot: null,
    currentWorkspace: null,
    pendingCategoryId: null,
    workspaceNotice: null,
    currentPrompt: null,
    speculativeDraft: null,
    confirmedSections: new Set(),
    contradictions: [],
    stages: [],
    pipelineComplete: false,
    pipelineSummary: null,
    events: [],
    currentStep: null,
    attach: mockAttach,
    sendDescription: mockSendDescription,
    submitPromptAnswers: vi.fn(),
    enterCategory: mockEnterCategory,
    backToCategories: mockBackToCategories,
    sendDone: vi.fn(),
    sendDimensionEdit: vi.fn(),
    ...overrides,
  };
}

describe('SessionPage resume behavior', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useSocraticDraftStore.setState((state) => ({ ...state, prompts: {} }));
    resetSocraticDocumentGraph();
    mockGetAccessToken.mockResolvedValue('mock-token');
    vi.spyOn(window, 'prompt').mockReturnValue(null);
    vi.spyOn(window, 'confirm').mockReturnValue(true);
    vi.spyOn(window.URL, 'createObjectURL').mockReturnValue('blob:session-export');
    vi.spyOn(window.URL, 'revokeObjectURL').mockImplementation(() => undefined);
    vi.mocked(useSocraticWebSocket).mockReturnValue(makeMockSocraticState());
    mockGetSessionEvents.mockResolvedValue({ session_id: 'abc', events: [], count: 0 });
  });

  it('attaches for existing pipeline_running sessions without restarting', async () => {
    const session = makeSession({
      intake_phase: 'pipeline_running',
      pipeline_running: true,
      project_description: 'Build timer',
      can_resume_live: true,
      resume_status: 'live_attach_available',
    });
    mockGetSession.mockResolvedValue({ session });

    renderSessionPage('/session/abc');

    await waitFor(() => {
      expect(mockGetSession).toHaveBeenCalledWith('abc');
    });
    await waitFor(() => {
      expect(mockAttach).toHaveBeenCalledTimes(1);
    });

    expect(mockSendDescription).not.toHaveBeenCalled();
    expect(mockStartSocratic).not.toHaveBeenCalled();

    const lastHookCall = vi.mocked(useSocraticWebSocket).mock.calls.at(-1)?.[0];
    expect(lastHookCall?.sessionId).toBe('abc');
    expect(lastHookCall?.initialSession?.id).toBe('abc');
    expect(lastHookCall?.initialSession?.intake_phase).toBe('pipeline_running');
  });

  it('does not attach when phase is pipeline_running but capability says no live resume', async () => {
    const session = makeSession({
      intake_phase: 'pipeline_running',
      pipeline_running: true,
      can_resume_live: false,
      resume_status: 'interview_resume_unknown',
    });
    mockGetSession.mockResolvedValue({ session });

    renderSessionPage('/session/abc');

    await waitFor(() => {
      expect(mockGetSession).toHaveBeenCalledWith('abc');
    });

    expect(mockAttach).not.toHaveBeenCalled();
  });

  it('opens project-scoped knowledge from the session header action', async () => {
    const user = userEvent.setup();
    const session = makeSession({
      project_id: 'proj-task-tracker',
      project_slug: 'task-tracker',
      project_name: 'Task Tracker',
    });
    mockGetSession.mockResolvedValue({ session });

    renderSessionPage('/session/abc');

    await waitFor(() => {
      expect(mockGetSession).toHaveBeenCalledWith('abc');
    });

    await user.click(screen.getByRole('button', { name: 'Knowledge' }));

    expect(await screen.findByTestId('location-path')).toHaveTextContent('/knowledge/projects/proj-task-tracker');
    const params = new URLSearchParams(screen.getByTestId('location-search').textContent ?? '');
    expect(params.get('project')).toBe('proj-task-tracker');
    expect(params.get('from')).toBe('/session/abc');
    expect(params.get('from_label')).toBe('Session');
  });

  it('prefills the planning brief textarea for seeded import sessions', async () => {
    const session = makeSession({
      intake_phase: 'waiting',
      project_description: 'Imported planning brief for Task Tracker.\n\nRepository brief: Track work across teams.',
      project_id: 'proj-task-tracker',
      project_slug: 'task-tracker',
      project_name: 'Task Tracker',
    });
    mockGetSession.mockResolvedValue({ session });

    renderSessionPage('/session/abc');

    await waitFor(() => {
      expect(mockGetSession).toHaveBeenCalledWith('abc');
    });

    expect(screen.getByRole('textbox', { name: 'Planning brief' })).toHaveValue(
      'Imported planning brief for Task Tracker.\n\nRepository brief: Track work across teams.',
    );
  });

  it('keeps existing sessions in a loading state instead of flashing the planning brief before hydration', async () => {
    let resolveSession: ((value: { session: Session }) => void) | null = null;
    mockGetSession.mockReturnValue(
      new Promise<{ session: Session }>((resolve) => {
        resolveSession = resolve;
      }),
    );

    renderSessionPage('/session/abc');

    expect(screen.getByText(/loading session/i)).toBeInTheDocument();
    expect(screen.queryByRole('textbox', { name: 'Planning brief' })).not.toBeInTheDocument();

    resolveSession?.({ session: makeSession({ intake_phase: 'waiting' }) });

    await waitFor(() => {
      expect(screen.getByRole('textbox', { name: 'Planning brief' })).toBeInTheDocument();
    });
  });

  it('uses tighter planning-brief copy on waiting sessions', async () => {
    const session = makeSession({
      intake_phase: 'waiting',
      project_description: 'Build a field service scheduler.',
    });
    mockGetSession.mockResolvedValue({ session });

    renderSessionPage('/session/abc');

    await waitFor(() => {
      expect(mockGetSession).toHaveBeenCalledWith('abc');
    });

    expect(screen.getByRole('heading', { name: /start with the planning brief/i })).toBeInTheDocument();
    expect(screen.getByText(/planner will move straight into the next question from there/i)).toBeInTheDocument();
    expect(screen.queryByText(/we'll ask focused questions to fill in the details/i)).not.toBeInTheDocument();
  });

  it('shows the first-reveal preload state after starting the interview', async () => {
    const user = userEvent.setup();
    const session = makeSession({
      intake_phase: 'waiting',
      project_description: 'Build a field service scheduler.',
    });
    mockGetSession.mockResolvedValue({ session });
    mockStartSocratic.mockResolvedValue({
      session_id: 'abc',
      ws_url: '/api/sessions/abc/socratic/ws',
    });

    renderSessionPage('/session/abc');

    await waitFor(() => {
      expect(mockGetSession).toHaveBeenCalledWith('abc');
    });

    await user.click(screen.getByRole('button', { name: /start session/i }));

    await waitFor(() => {
      expect(mockStartSocratic).toHaveBeenCalledWith('abc', 'Build a field service scheduler.');
    });

    expect(mockSendDescription).toHaveBeenCalledWith('Build a field service scheduler.');
    expect(screen.getByRole('heading', { name: /planner is preparing the first working set of questions/i })).toBeInTheDocument();
    expect(screen.getByText(/0\/8 locally known question items ready for the first reveal/i)).toBeInTheDocument();
    expect(screen.getByRole('region', { name: /interview progress/i })).toBeInTheDocument();
    expect(screen.queryByRole('textbox', { name: 'Planning brief' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /belief state/i })).not.toBeInTheDocument();
    expect(screen.queryByLabelText('Thread index')).not.toBeInTheDocument();
  });

  it('reveals the lobby immediately when the first loaded batch already meets the preload target', async () => {
    const user = userEvent.setup();
    const session = makeSession({
      intake_phase: 'waiting',
      project_description: 'Build a field service scheduler.',
    });
    let preloadReady = false;
    mockGetSession.mockResolvedValue({ session });
    mockStartSocratic.mockImplementation(async () => {
      preloadReady = true;
      return {
        session_id: 'abc',
        ws_url: '/api/sessions/abc/socratic/ws',
      };
    });
    vi.mocked(useSocraticWebSocket).mockImplementation(() => (
      preloadReady
        ? makeMockSocraticState({
            currentCategorySnapshot: {
        revision: 'category-preload-1',
        root_category_ids: ['root-discovery'],
        nodes: [
          {
            category_id: 'root-discovery',
            parent_category_id: null,
            title: 'Explore missing areas',
            summary: 'Answer the first loaded batch.',
            status: 'active',
            depth: 0,
            mapped_dimensions: [],
            has_children: false,
            has_prompt_ready: true,
            item_count_hint: 8,
          },
        ],
        active_category_path: [
          { category_id: 'root-discovery', title: 'Explore missing areas' },
        ],
        newly_available_category_ids: [],
        build_ready: false,
        build_readiness_message: 'Build is blocked until the first discovery batch is answered.',
      },
      currentWorkspace: {
        revision: 'workspace-preload-1',
        focused_category_id: 'root-discovery',
        branch_notice: null,
        category_snapshot: {
          revision: 'category-preload-1',
          root_category_ids: ['root-discovery'],
          nodes: [
            {
              category_id: 'root-discovery',
              parent_category_id: null,
              title: 'Explore missing areas',
              summary: 'Answer the first loaded batch.',
              status: 'active',
              depth: 0,
              mapped_dimensions: [],
              has_children: false,
              has_prompt_ready: true,
              item_count_hint: 8,
            },
          ],
          active_category_path: [
            { category_id: 'root-discovery', title: 'Explore missing areas' },
          ],
          newly_available_category_ids: [],
          build_ready: false,
          build_readiness_message: 'Build is blocked until the first discovery batch is answered.',
        },
        groups: [
          {
            category_id: 'root-discovery',
            title: 'Explore missing areas',
            summary: 'Answer the first loaded batch.',
            status: 'active',
            question_count: 8,
            is_focused: true,
            is_new: false,
            preview_items: [],
          },
        ],
      },
      currentPrompt: {
        prompt_id: 'prompt-preload-1',
        kind: 'question_batch',
        title: 'Clarify the first batch',
        instructions: 'Answer the first loaded batch.',
        origin_category_id: 'root-discovery',
        category_path: [
          { category_id: 'root-discovery', title: 'Explore missing areas' },
        ],
        items: Array.from({ length: 8 }, (_, index) => ({
          item_id: `item-preload-${index + 1}`,
          kind: 'discovery' as const,
          target_dimension: 'Scope',
          section_ref: null,
          text: `Question ${index + 1}?`,
          options: [],
          response_mode: 'single_select_with_custom_text' as const,
          required: false,
          priority: 100 - index,
          dependency_item_ids: [],
        })),
        draft_snapshot: null,
        required_item_ids: [],
        allow_partial_submit: true,
        ui_hints: {
          preferred_layout: 'cards',
          show_draft_sidebar: false,
        },
        based_on_turn: 1,
        created_at: '2026-03-24T00:00:00Z',
      },
          })
        : makeMockSocraticState()
    ));

    renderSessionPage('/session/abc');

    await waitFor(() => {
      expect(mockGetSession).toHaveBeenCalledWith('abc');
    });

    await user.click(screen.getByRole('button', { name: /start session/i }));

    await waitFor(() => {
      expect(screen.getByLabelText('Thread index')).toBeInTheDocument();
    });

    expect(screen.queryByRole('heading', { name: /planner is preparing the first working set of questions/i })).not.toBeInTheDocument();
    expect(screen.getAllByText(/question 1\?/i).length).toBeGreaterThan(0);
  });

  it('reveals the best known partial lobby after the hard preload timeout', async () => {
    const session = makeSession({
      intake_phase: 'waiting',
      project_description: 'Build a field service scheduler.',
    });
    let partialPreloadReady = false;
    mockGetSession.mockResolvedValue({ session });
    mockStartSocratic.mockImplementation(async () => {
      partialPreloadReady = true;
      return {
        session_id: 'abc',
        ws_url: '/api/sessions/abc/socratic/ws',
      };
    });
    vi.mocked(useSocraticWebSocket).mockImplementation(() => (
      partialPreloadReady
        ? makeMockSocraticState({
            currentCategorySnapshot: {
        revision: 'category-partial-1',
        root_category_ids: ['root-discovery'],
        nodes: [
          {
            category_id: 'root-discovery',
            parent_category_id: null,
            title: 'Explore missing areas',
            summary: 'Only one preview is known so far.',
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
        build_readiness_message: 'Build is blocked until more discovery questions arrive.',
      },
      currentWorkspace: {
        revision: 'workspace-partial-1',
        focused_category_id: null,
        branch_notice: null,
        category_snapshot: {
          revision: 'category-partial-1',
          root_category_ids: ['root-discovery'],
          nodes: [
            {
              category_id: 'root-discovery',
              parent_category_id: null,
              title: 'Explore missing areas',
              summary: 'Only one preview is known so far.',
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
          build_readiness_message: 'Build is blocked until more discovery questions arrive.',
        },
        groups: [
          {
            category_id: 'root-discovery',
            title: 'Explore missing areas',
            summary: 'Only one preview is known so far.',
            status: 'ready',
            question_count: 1,
            is_focused: false,
            is_new: false,
            preview_items: [
              {
                item_id: 'root-discovery::preview::0',
                kind: 'discovery',
                text: 'Clarify the remaining area.',
              },
            ],
          },
        ],
      },
      currentPrompt: null,
          })
        : makeMockSocraticState()
    ));

    renderSessionPage('/session/abc');

    await waitFor(() => {
      expect(mockGetSession).toHaveBeenCalledWith('abc');
    });

    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: /start session/i }));
    });

    await act(async () => {
      await new Promise((resolve) => window.setTimeout(resolve, 8_250));
    });

    await waitFor(() => {
      expect(screen.getByLabelText('Thread index')).toBeInTheDocument();
    }, { timeout: 2_000 });

    expect(screen.getByText(/opened the desk with a partial initial set/i)).toBeInTheDocument();
    expect(screen.getByText(/clarify the remaining area/i)).toBeInTheDocument();
  }, 12_000);

  it('attaches for detached interviewing sessions when a live runtime is available', async () => {
    const session = makeSession({
      intake_phase: 'interviewing',
      pipeline_running: false,
      can_resume_live: true,
      resume_status: 'live_attach_available',
      project_description: 'Build timer',
    });
    mockGetSession.mockResolvedValue({ session });

    renderSessionPage('/session/abc');

    await waitFor(() => {
      expect(mockGetSession).toHaveBeenCalledWith('abc');
    });
    await waitFor(() => {
      expect(mockAttach).toHaveBeenCalledTimes(1);
    });
  });

  it('shows restart-only warning for detached interviewing sessions', async () => {
    const session = makeSession({
      intake_phase: 'interviewing',
      pipeline_running: false,
      project_description: 'Build timer',
      can_restart_from_description: true,
      resume_status: 'interview_restart_only',
    });
    mockGetSession.mockResolvedValue({ session });

    renderSessionPage('/session/abc');

    await waitFor(() => {
      expect(mockGetSession).toHaveBeenCalledWith('abc');
    });

    expect(mockAttach).not.toHaveBeenCalled();
    expect(mockSendDescription).not.toHaveBeenCalled();
    expect(screen.getByText(/live interview resume is not supported yet/i)).toBeInTheDocument();
  });

  it('shows unknown resume-state warning for interviewing sessions with unknown status', async () => {
    const session = makeSession({
      intake_phase: 'interviewing',
      pipeline_running: false,
      can_restart_from_description: false,
      resume_status: 'interview_resume_unknown',
    });
    mockGetSession.mockResolvedValue({ session });

    renderSessionPage('/session/abc');

    await waitFor(() => {
      expect(mockGetSession).toHaveBeenCalledWith('abc');
    });

    expect(screen.getByText(/interview resume state is unknown/i)).toBeInTheDocument();
  });

  it('renders saved checkpoint details for detached interviews', async () => {
    const session = makeSession({
      intake_phase: 'interviewing',
      has_checkpoint: true,
      can_resume_checkpoint: true,
      resume_status: 'interview_checkpoint_resumable',
      checkpoint: {
        socratic_run_id: '11111111-1111-1111-1111-111111111111',
        classification: null,
        belief_state: null,
        current_prompt: {
          prompt_id: 'prompt-1',
          kind: 'draft_review',
          title: 'Review draft',
          instructions: null,
          category_path: [],
          items: [{
            item_id: 'item-1',
            kind: 'draft_section',
            target_dimension: 'Stakeholders',
            section_ref: 'Goal',
            text: 'What are the core user roles?',
            options: [],
            response_mode: 'single_select_with_custom_text',
            required: false,
            priority: 100,
            dependency_item_ids: [],
          }],
          draft_snapshot: {
            sections: [{ heading: 'Goal', content: 'Draft goal section' }],
            assumptions: [],
            not_discussed: [],
          },
          required_item_ids: [],
          allow_partial_submit: true,
          ui_hints: {
            preferred_layout: 'review',
            show_draft_sidebar: true,
          },
          based_on_turn: 2,
          created_at: '2026-03-06T12:00:00Z',
        },
        contradictions: [],
        stale_turns: 1,
        draft_shown_at_turn: 2,
        last_checkpoint_at: '2026-03-06T12:00:00Z',
      },
    });
    mockGetSession.mockResolvedValue({ session });

    renderSessionPage('/session/abc');

    await waitFor(() => {
      expect(mockGetSession).toHaveBeenCalledWith('abc');
    });

    expect(screen.getByText(/saved interview checkpoint/i)).toBeInTheDocument();
    expect(screen.getByText(/target dimension: stakeholders/i)).toBeInTheDocument();
    expect(screen.getByText(/pending draft review: goal/i)).toBeInTheDocument();
  });

  it('renders workflow actions from backend capabilities', async () => {
    const session = makeSession({
      intake_phase: 'interviewing',
      project_description: 'Build timer',
      can_restart_from_description: true,
      can_retry_pipeline: true,
      resume_status: 'interview_restart_only',
    });
    mockGetSession.mockResolvedValue({ session });

    renderSessionPage('/session/abc');

    await waitFor(() => {
      expect(mockGetSession).toHaveBeenCalledWith('abc');
    });

    expect(screen.getByRole('button', { name: /restart from description/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /retry pipeline/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /back to sessions/i })).toBeInTheDocument();
  });

  it('restarts from the saved description via the explicit workflow endpoint', async () => {
    const user = userEvent.setup();
    const session = makeSession({
      intake_phase: 'interviewing',
      project_description: 'Build timer',
      can_restart_from_description: true,
      resume_status: 'interview_restart_only',
    });
    mockGetSession.mockResolvedValue({ session });
    mockRestartFromDescription.mockResolvedValue({
      session: makeSession({
        intake_phase: 'interviewing',
        project_description: 'Build timer',
        can_restart_from_description: true,
        resume_status: 'interview_restart_only',
      }),
    });

    renderSessionPage('/session/abc');

    await waitFor(() => {
      expect(mockGetSession).toHaveBeenCalledWith('abc');
    });

    await user.click(screen.getByRole('button', { name: /restart from description/i }));

    await waitFor(() => {
      expect(mockRestartFromDescription).toHaveBeenCalledWith('abc');
    });
    expect(mockSendDescription).toHaveBeenCalledWith('Build timer');
  });

  it('retries a failed pipeline and reattaches to live progress updates', async () => {
    const user = userEvent.setup();
    const session = makeSession({
      intake_phase: 'error',
      project_description: 'Build timer',
      can_retry_pipeline: true,
      error_message: 'Pipeline failed',
    });
    mockGetSession.mockResolvedValue({ session });
    mockRetryPipeline.mockResolvedValue({
      session: makeSession({
        intake_phase: 'pipeline_running',
        pipeline_running: true,
        project_description: 'Build timer',
        can_resume_live: true,
        resume_status: 'live_attach_available',
      }),
    });

    renderSessionPage('/session/abc');

    await waitFor(() => {
      expect(mockGetSession).toHaveBeenCalledWith('abc');
    });

    await user.click(screen.getByRole('button', { name: /retry pipeline/i }));

    await waitFor(() => {
      expect(mockRetryPipeline).toHaveBeenCalledWith('abc');
    });
    await waitFor(() => {
      expect(mockAttach).toHaveBeenCalledTimes(1);
    });
  });

  it('renames the session from the action bar', async () => {
    const user = userEvent.setup();
    vi.mocked(window.prompt).mockReturnValue('Renamed session');
    const session = makeSession({
      title: 'Original session',
      intake_phase: 'waiting',
    });
    mockGetSession.mockResolvedValue({ session });
    mockUpdateSession.mockResolvedValue({
      session: makeSession({
        ...session,
        title: 'Renamed session',
      }),
    });

    renderSessionPage('/session/abc');

    await waitFor(() => {
      expect(mockGetSession).toHaveBeenCalledWith('abc');
    });

    await user.click(screen.getByRole('button', { name: /rename/i }));

    await waitFor(() => {
      expect(mockUpdateSession).toHaveBeenCalledWith('abc', { title: 'Renamed session' });
    });
  });

  it('duplicates the session and navigates to the new copy', async () => {
    const user = userEvent.setup();
    vi.mocked(window.prompt).mockReturnValue('Copied session');
    const session = makeSession({
      title: 'Original session',
      intake_phase: 'waiting',
    });
    mockGetSession.mockResolvedValue({ session });
    mockDuplicateSession.mockResolvedValue({
      session: makeSession({
        id: 'copy-123',
        title: 'Copied session',
        intake_phase: 'waiting',
      }),
    });

    renderSessionPage('/session/abc');

    await waitFor(() => {
      expect(mockGetSession).toHaveBeenCalledWith('abc');
    });

    await user.click(screen.getByRole('button', { name: /duplicate/i }));

    await waitFor(() => {
      expect(mockDuplicateSession).toHaveBeenCalledWith('abc', { title: 'Copied session' });
    });
  });

  it('archives the session from the action bar', async () => {
    const user = userEvent.setup();
    const session = makeSession({
      title: 'Archive me',
      intake_phase: 'complete',
    });
    mockGetSession.mockResolvedValue({ session });
    mockUpdateSession.mockResolvedValue({
      session: makeSession({
        ...session,
        archived: true,
        archived_at: '2026-03-06T12:00:00Z',
      }),
    });

    renderSessionPage('/session/abc');

    await waitFor(() => {
      expect(mockGetSession).toHaveBeenCalledWith('abc');
    });

    await user.click(screen.getByRole('button', { name: /archive/i }));

    await waitFor(() => {
      expect(mockUpdateSession).toHaveBeenCalledWith('abc', { archived: true });
    });
    expect(screen.getByText(/archived/i)).toBeInTheDocument();
  });

  it('exports the session transcript and event log', async () => {
    const user = userEvent.setup();
    const appendSpy = vi.spyOn(document.body, 'appendChild');
    const removeSpy = vi.spyOn(HTMLAnchorElement.prototype, 'remove').mockImplementation(() => undefined);
    const clickSpy = vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => undefined);
    const session = makeSession({
      title: 'Exportable session',
      intake_phase: 'complete',
      events: [{ id: 'e1', timestamp: new Date().toISOString(), level: 'info', source: 'system', message: 'done', metadata: {} }],
    });
    mockGetSession.mockResolvedValue({ session });
    mockExportSession.mockResolvedValue({
      exported_at: '2026-03-06T12:30:00Z',
      session,
    });

    renderSessionPage('/session/abc');

    await waitFor(() => {
      expect(mockGetSession).toHaveBeenCalledWith('abc');
    });

    await user.click(screen.getByRole('button', { name: /export/i }));

    await waitFor(() => {
      expect(mockExportSession).toHaveBeenCalledWith('abc');
    });
    expect(window.URL.createObjectURL).toHaveBeenCalledTimes(1);
    expect(appendSpy).toHaveBeenCalled();
    expect(clickSpy).toHaveBeenCalled();
    expect(removeSpy).toHaveBeenCalled();
  });

  it('renders Events as a first-class right-pane tab and opens it from the header action', async () => {
    const user = userEvent.setup();
    const session = makeSession({
      intake_phase: 'pipeline_running',
      pipeline_running: true,
      can_resume_live: true,
      resume_status: 'live_attach_available',
    });
    mockGetSession.mockResolvedValue({ session });
    vi.mocked(useSocraticWebSocket).mockReturnValue(makeMockSocraticState({
      isConnected: true,
      reconnectFailed: false,
      intakePhase: 'pipeline_running',
      messages: [
        { id: 'planner-1', role: 'planner', content: 'Working…', timestamp: new Date().toISOString() },
      ],
      classification: null,
      beliefState: null,
      convergencePct: 0.8,
      currentCategorySnapshot: null,
      currentPrompt: null,
      speculativeDraft: null,
      confirmedSections: new Set(),
      contradictions: [],
      stages: [],
      pipelineComplete: false,
      pipelineSummary: null,
      events: [
        {
          id: 'evt-1',
          timestamp: new Date().toISOString(),
          level: 'warn',
          source: 'pipeline',
          message: 'Pipeline waiting for retry',
          metadata: {},
        },
      ],
      currentStep: 'pipeline.wait',
      attach: mockAttach,
      sendDescription: mockSendDescription,
    }));

    renderSessionPage('/session/abc');

    await waitFor(() => {
      expect(mockGetSession).toHaveBeenCalledWith('abc');
    });

    expect(screen.getByRole('button', { name: /belief state/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /^draft/i })).toBeInTheDocument();
    expect(screen.getAllByRole('button', { name: /events/i }).length).toBeGreaterThan(0);

    await user.click(screen.getByRole('button', { name: /events 1/i }));
    expect(screen.getByText(/pipeline waiting for retry/i)).toBeInTheDocument();
  });

  it('auto-foregrounds the Events feed when pipeline execution is active', async () => {
    const session = makeSession({
      intake_phase: 'pipeline_running',
      pipeline_running: true,
      can_resume_live: true,
      resume_status: 'live_attach_available',
    });
    mockGetSession.mockResolvedValue({ session });
    vi.mocked(useSocraticWebSocket).mockReturnValue(makeMockSocraticState({
      isConnected: true,
      reconnectFailed: false,
      intakePhase: 'pipeline_running',
      messages: [],
      classification: null,
      beliefState: null,
      convergencePct: 0.8,
      currentCategorySnapshot: null,
      currentPrompt: null,
      speculativeDraft: null,
      confirmedSections: new Set(),
      contradictions: [],
      stages: [],
      pipelineComplete: false,
      pipelineSummary: null,
      events: [
        {
          id: 'evt-live',
          timestamp: new Date().toISOString(),
          level: 'info',
          source: 'pipeline',
          message: 'Factory stage started',
          metadata: { stage: 'Factory' },
        },
      ],
      currentStep: 'pipeline.stage.started',
      attach: mockAttach,
      sendDescription: mockSendDescription,
    }));

    renderSessionPage('/session/abc');

    await waitFor(() => {
      expect(mockGetSession).toHaveBeenCalledWith('abc');
    });

    expect(screen.getByText(/factory stage started/i)).toBeInTheDocument();
  });

  it('shows first-class retry and artifact summaries in the Events pane', async () => {
    const session = makeSession({
      intake_phase: 'pipeline_running',
      pipeline_running: true,
      can_resume_live: true,
      resume_status: 'live_attach_available',
    });
    mockGetSession.mockResolvedValue({ session });
    vi.mocked(useSocraticWebSocket).mockReturnValue(makeMockSocraticState({
      isConnected: true,
      reconnectFailed: false,
      intakePhase: 'pipeline_running',
      messages: [],
      classification: null,
      beliefState: null,
      convergencePct: 0.8,
      currentCategorySnapshot: null,
      currentPrompt: null,
      speculativeDraft: null,
      confirmedSections: new Set(),
      contradictions: [],
      stages: [],
      pipelineComplete: false,
      pipelineSummary: null,
      events: [
        {
          id: 'evt-retry',
          timestamp: new Date().toISOString(),
          level: 'warn',
          source: 'pipeline',
          step: 'pipeline.retry.feedback',
          message: 'Retry feedback prepared',
          metadata: {
            feedback_count: 2,
            details: {
              attempt: 2,
              categories: { runtime: 1, contract: 1 },
              severities: { High: 1, Medium: 1 },
            },
          },
        },
        {
          id: 'evt-artifact-1',
          timestamp: new Date().toISOString(),
          level: 'info',
          source: 'pipeline',
          step: 'pipeline.artifact.persisted',
          message: 'Persisted artifact',
          metadata: { type_id: 'nlspec-v1' },
        },
        {
          id: 'evt-artifact-2',
          timestamp: new Date().toISOString(),
          level: 'info',
          source: 'pipeline',
          step: 'pipeline.artifact.persisted',
          message: 'Persisted artifact',
          metadata: { type_id: 'satisfaction' },
        },
      ],
      currentStep: 'pipeline.retry.feedback',
      attach: mockAttach,
      sendDescription: mockSendDescription,
    }));

    renderSessionPage('/session/abc');

    await waitFor(() => {
      expect(mockGetSession).toHaveBeenCalledWith('abc');
    });

    expect(screen.getByLabelText(/retry feedback summary/i)).toBeInTheDocument();
    expect(screen.getByText(/2 categorized items \(attempt 2\)/i)).toBeInTheDocument();
    expect(screen.getByText(/runtime: 1/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/artifact persistence summary/i)).toBeInTheDocument();
    expect(screen.getByText(/2 artifacts persisted/i)).toBeInTheDocument();
  });

  it('shows explicit interview progress while waiting for the first prompt batch', async () => {
    const session = makeSession({
      intake_phase: 'interviewing',
      pipeline_running: false,
      can_resume_live: false,
      resume_status: 'interview_attached',
    });
    mockGetSession.mockResolvedValue({ session });
    vi.mocked(useSocraticWebSocket).mockReturnValue(makeMockSocraticState({
      isConnected: true,
      reconnectFailed: false,
      intakePhase: 'interviewing',
      messages: [
        { id: 'planner-1', role: 'planner', content: 'Analyzing your project description...', timestamp: new Date().toISOString() },
      ],
      classification: null,
      beliefState: null,
      convergencePct: 0.15,
      currentCategorySnapshot: null,
      currentPrompt: null,
      speculativeDraft: null,
      confirmedSections: new Set(),
      contradictions: [],
      stages: [],
      pipelineComplete: false,
      pipelineSummary: null,
      events: [
        {
          id: 'evt-1',
          timestamp: new Date().toISOString(),
          level: 'info',
          source: 'socratic_engine',
          step: 'socratic.response.adjudicated',
          message: 'Prompt response adjudicated at 15% convergence',
          metadata: {},
        },
      ],
      currentStep: 'socratic.response.adjudicated',
      attach: mockAttach,
      sendDescription: mockSendDescription,
    }));

    renderSessionPage('/session/abc');

    await waitFor(() => {
      expect(mockGetSession).toHaveBeenCalledWith('abc');
    });

    expect(screen.getByRole('region', { name: /interview progress/i })).toBeInTheDocument();
    expect(screen.getByText(/generating your next questions/i)).toBeInTheDocument();
    expect(screen.getAllByText(/planning the next question batch/i).length).toBeGreaterThan(0);
    expect(screen.getByText(/prompt response adjudicated at 15% convergence/i)).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /belief state/i })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /show all threads/i })).not.toBeInTheDocument();
  });

  it('renders the category navigator during category-driven intake', async () => {
    const user = userEvent.setup();
    const session = makeSession({
      intake_phase: 'interviewing',
      pipeline_running: false,
      can_resume_live: false,
      resume_status: 'interview_attached',
    });
    mockGetSession.mockResolvedValue({ session });
    vi.mocked(useSocraticWebSocket).mockReturnValue(makeMockSocraticState({
      isConnected: true,
      reconnectFailed: false,
      intakePhase: 'interviewing',
      messages: [],
      classification: null,
      beliefState: null,
      convergencePct: 0.4,
      currentCategorySnapshot: {
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
      currentWorkspace: {
        revision: 'workspace-1',
        focused_category_id: null,
        branch_notice: null,
        category_snapshot: {
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
        groups: [
          {
            category_id: 'root-discovery',
            title: 'Explore missing areas',
            summary: '1 area still needs discovery.',
            status: 'ready',
            question_count: 1,
            is_focused: false,
            is_new: false,
            preview_items: [
              {
                item_id: 'root-discovery::preview::0',
                kind: 'discovery',
                text: 'Clarify the remaining area.',
              },
            ],
          },
        ],
      },
      currentPrompt: null,
      speculativeDraft: null,
      confirmedSections: new Set(),
      contradictions: [],
      stages: [],
      pipelineComplete: false,
      pipelineSummary: null,
      events: [],
      currentStep: 'socratic.category_state.generated',
      attach: mockAttach,
      sendDescription: mockSendDescription,
    }));

    renderSessionPage('/session/abc');

    await waitFor(() => {
      expect(mockGetSession).toHaveBeenCalledWith('abc');
    });

    expect(screen.getByLabelText('Thread index')).toBeInTheDocument();
    expect(screen.getByLabelText('Consultant desk')).toBeInTheDocument();
    expect(screen.getByText(/clarify the remaining area/i)).toBeInTheDocument();
  });

  it('routes checkpoint-backed question state straight into the active question view without the old intro form', async () => {
    const session = makeSession({
      intake_phase: 'waiting',
      pipeline_running: false,
      can_resume_checkpoint: true,
      has_checkpoint: true,
      resume_status: 'interview_checkpoint_resumable',
      checkpoint: {
        socratic_run_id: '22222222-2222-2222-2222-222222222222',
        classification: null,
        belief_state: null,
        current_prompt: {
          prompt_id: 'prompt-hydrated-1',
          kind: 'question_batch',
          title: 'Clarify deployment',
          instructions: 'Answer the next deployment question.',
          origin_category_id: 'root-deployment',
          category_path: [
            { category_id: 'root-deployment', title: 'Deployment model' },
          ],
          items: [
            {
              item_id: 'item-hydrated-1',
              kind: 'discovery',
              target_dimension: 'Deployment',
              section_ref: null,
              text: 'Where will the application run?',
              options: [],
              response_mode: 'single_select_with_custom_text',
              required: false,
              priority: 100,
              dependency_item_ids: [],
            },
          ],
          draft_snapshot: null,
          required_item_ids: [],
          allow_partial_submit: true,
          ui_hints: {
            preferred_layout: 'cards',
            show_draft_sidebar: false,
          },
          based_on_turn: 3,
          created_at: '2026-03-22T00:00:00Z',
        },
        current_category_snapshot: {
          revision: 'category-hydrated-1',
          root_category_ids: ['root-deployment'],
          nodes: [
            {
              category_id: 'root-deployment',
              parent_category_id: null,
              title: 'Deployment model',
              summary: 'Clarify the runtime environment.',
              status: 'ready',
              depth: 0,
              mapped_dimensions: [],
              has_children: false,
              has_prompt_ready: true,
              item_count_hint: 1,
            },
          ],
          active_category_path: [
            { category_id: 'root-deployment', title: 'Deployment model' },
          ],
          newly_available_category_ids: [],
          build_ready: false,
          build_readiness_message: 'Build is blocked until deployment is clarified.',
        },
        contradictions: [],
        stale_turns: 0,
        draft_shown_at_turn: null,
        last_checkpoint_at: '2026-03-22T00:00:00Z',
      },
    });
    mockGetSession.mockResolvedValue({ session });
    vi.mocked(useSocraticWebSocket).mockReturnValue(makeMockSocraticState({
      isConnected: true,
      reconnectFailed: false,
      intakePhase: 'waiting',
      currentCategorySnapshot: {
        revision: 'category-hydrated-1',
        root_category_ids: ['root-deployment'],
        nodes: [
          {
            category_id: 'root-deployment',
            parent_category_id: null,
            title: 'Deployment model',
            summary: 'Clarify the runtime environment.',
            status: 'ready',
            depth: 0,
            mapped_dimensions: [],
            has_children: false,
            has_prompt_ready: true,
            item_count_hint: 1,
          },
        ],
        active_category_path: [
          { category_id: 'root-deployment', title: 'Deployment model' },
        ],
        newly_available_category_ids: [],
        build_ready: false,
        build_readiness_message: 'Build is blocked until deployment is clarified.',
      },
      currentWorkspace: null,
      currentPrompt: {
        prompt_id: 'prompt-hydrated-1',
        kind: 'question_batch',
        title: 'Clarify deployment',
        instructions: 'Answer the next deployment question.',
        origin_category_id: 'root-deployment',
        category_path: [
          { category_id: 'root-deployment', title: 'Deployment model' },
        ],
        items: [
          {
            item_id: 'item-hydrated-1',
            kind: 'discovery',
            target_dimension: 'Deployment',
            section_ref: null,
            text: 'Where will the application run?',
            options: [],
            response_mode: 'single_select_with_custom_text',
            required: false,
            priority: 100,
            dependency_item_ids: [],
          },
        ],
        draft_snapshot: null,
        required_item_ids: [],
        allow_partial_submit: true,
        ui_hints: {
          preferred_layout: 'cards',
          show_draft_sidebar: false,
        },
        based_on_turn: 3,
        created_at: '2026-03-22T00:00:00Z',
      },
      events: [],
      currentStep: 'socratic.question.focused',
    }));

    renderSessionPage('/session/abc');

    await waitFor(() => {
      expect(mockGetSession).toHaveBeenCalledWith('abc');
    });

    expect(screen.queryByRole('textbox', { name: 'Planning brief' })).not.toBeInTheDocument();
    expect(screen.getByLabelText('Thread index')).toBeInTheDocument();
    expect(screen.getAllByText(/where will the application run/i).length).toBeGreaterThan(0);
    expect(screen.getByRole('button', { name: /submit prompt answers/i })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /question map/i })).not.toBeInTheDocument();
  });

  it('renders the thread of thought as ancestor actions that can return to earlier categories', async () => {
    const user = userEvent.setup();
    const session = makeSession({
      intake_phase: 'interviewing',
      pipeline_running: false,
      can_resume_live: false,
      resume_status: 'interview_attached',
    });
    mockGetSession.mockResolvedValue({ session });
    vi.mocked(useSocraticWebSocket).mockReturnValue(makeMockSocraticState({
      isConnected: true,
      reconnectFailed: false,
      intakePhase: 'interviewing',
      currentWorkspace: {
        revision: 'workspace-thread-1',
        focused_category_id: 'root-security-auth',
        branch_notice: null,
        category_snapshot: {
          revision: 'category-thread-1',
          root_category_ids: ['root-discovery'],
          nodes: [
            {
              category_id: 'root-discovery',
              parent_category_id: null,
              title: 'Explore missing areas',
              summary: 'Broaden the discovery map.',
              status: 'ready',
              depth: 0,
              mapped_dimensions: [],
              has_children: true,
              has_prompt_ready: false,
              item_count_hint: 1,
            },
            {
              category_id: 'root-security',
              parent_category_id: 'root-discovery',
              title: 'Security',
              summary: 'Clarify security requirements.',
              status: 'ready',
              depth: 1,
              mapped_dimensions: [],
              has_children: true,
              has_prompt_ready: false,
              item_count_hint: 1,
            },
            {
              category_id: 'root-security-auth',
              parent_category_id: 'root-security',
              title: 'Authentication model',
              summary: 'Clarify the authentication flow.',
              status: 'active',
              depth: 2,
              mapped_dimensions: [],
              has_children: false,
              has_prompt_ready: true,
              item_count_hint: 1,
            },
          ],
          active_category_path: [
            { category_id: 'root-discovery', title: 'Explore missing areas' },
            { category_id: 'root-security', title: 'Security' },
            { category_id: 'root-security-auth', title: 'Authentication model' },
          ],
          newly_available_category_ids: [],
          build_ready: false,
          build_readiness_message: 'Build is blocked until security is clarified.',
        },
        groups: [
          {
            category_id: 'root-security-auth',
            title: 'Authentication model',
            summary: 'Clarify the authentication flow.',
            status: 'active',
            question_count: 1,
            is_focused: true,
            is_new: false,
            preview_items: [],
          },
        ],
      },
      currentPrompt: {
        prompt_id: 'prompt-thread-1',
        kind: 'question_batch',
        title: 'Clarify authentication',
        instructions: 'Answer the focused security question.',
        origin_category_id: 'root-security-auth',
        category_path: [
          { category_id: 'root-discovery', title: 'Explore missing areas' },
          { category_id: 'root-security', title: 'Security' },
          { category_id: 'root-security-auth', title: 'Authentication model' },
        ],
        items: [
          {
            item_id: 'item-thread-1',
            kind: 'discovery',
            target_dimension: 'Security',
            section_ref: null,
            text: 'How should authentication work?',
            options: [],
            response_mode: 'single_select_with_custom_text',
            required: false,
            priority: 100,
            dependency_item_ids: [],
          },
        ],
        draft_snapshot: null,
        required_item_ids: [],
        allow_partial_submit: true,
        ui_hints: {
          preferred_layout: 'cards',
          show_draft_sidebar: false,
        },
        based_on_turn: 5,
        created_at: '2026-03-22T00:00:00Z',
      },
      speculativeDraft: null,
      confirmedSections: new Set(),
      contradictions: [],
      stages: [],
      pipelineComplete: false,
      pipelineSummary: null,
      events: [],
      currentStep: 'socratic.question.focused',
      attach: mockAttach,
      sendDescription: mockSendDescription,
    }));

    renderSessionPage('/session/abc');

    await waitFor(() => {
      expect(mockGetSession).toHaveBeenCalledWith('abc');
    });

    expect(screen.getByLabelText('Thread index')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Security' }));
    expect(mockEnterCategory).toHaveBeenCalledWith('root-security', 'category-thread-1');
  });

  it('keeps category-only interview state inside the focused lobby instead of the legacy split pane', async () => {
    const session = makeSession({
      intake_phase: 'interviewing',
      pipeline_running: false,
      can_resume_live: false,
      resume_status: 'interview_attached',
    });
    mockGetSession.mockResolvedValue({ session });
    vi.mocked(useSocraticWebSocket).mockReturnValue(makeMockSocraticState({
      isConnected: true,
      reconnectFailed: false,
      intakePhase: 'interviewing',
      messages: [],
      classification: null,
      beliefState: null,
      convergencePct: 0.4,
      currentCategorySnapshot: {
        revision: 'category-only-1',
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
          {
            category_id: 'root-discovery::security',
            parent_category_id: 'root-discovery',
            title: 'Security',
            summary: 'Authentication still needs definition.',
            status: 'ready',
            depth: 1,
            mapped_dimensions: [],
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
        build_readiness_message: 'Build is blocked until 1 remaining area is explored.',
      },
      currentWorkspace: null,
      currentPrompt: null,
      speculativeDraft: null,
      confirmedSections: new Set(),
      contradictions: [],
      stages: [],
      pipelineComplete: false,
      pipelineSummary: null,
      events: [],
      currentStep: 'socratic.category_state.generated',
      attach: mockAttach,
      sendDescription: mockSendDescription,
    }));

    renderSessionPage('/session/abc');

    await waitFor(() => {
      expect(mockGetSession).toHaveBeenCalledWith('abc');
    });

    expect(screen.getByLabelText('Thread index')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /context/i })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Belief State' })).not.toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Explore missing areas' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Security' })).not.toBeInTheDocument();
  });

  it('keeps belief state, draft, transcript, and events hidden until the context shelf is opened', async () => {
    const user = userEvent.setup();
    const session = makeSession({
      intake_phase: 'interviewing',
      pipeline_running: false,
      can_resume_live: false,
      resume_status: 'interview_attached',
    });
    mockGetSession.mockResolvedValue({ session });
    vi.mocked(useSocraticWebSocket).mockReturnValue(makeMockSocraticState({
      isConnected: true,
      reconnectFailed: false,
      intakePhase: 'interviewing',
      messages: [
        { id: 'planner-1', role: 'planner', content: 'welcome', timestamp: new Date().toISOString() },
      ],
      classification: {
        project_type: 'Web App',
        complexity: 'medium',
      },
      beliefState: {
        filled: {
          stack: { value: 'React', confidence: 1 },
        },
        uncertain: {},
        missing: [],
        out_of_scope: [],
        convergence_pct: 0.45,
      },
      convergencePct: 0.45,
      currentCategorySnapshot: null,
      currentWorkspace: {
        revision: 'workspace-context-1',
        focused_category_id: 'root-discovery',
        branch_notice: null,
        category_snapshot: {
          revision: 'category-context-1',
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
              has_prompt_ready: true,
              item_count_hint: 1,
            },
          ],
          active_category_path: [],
          newly_available_category_ids: [],
          build_ready: false,
          build_readiness_message: 'Build is blocked until 1 remaining area is explored.',
        },
        groups: [
          {
            category_id: 'root-discovery',
            title: 'Explore missing areas',
            summary: 'Clarify the remaining area.',
            status: 'active',
            question_count: 1,
            is_focused: true,
            is_new: false,
            preview_items: [
              {
                item_id: 'root-discovery::preview::0',
                kind: 'discovery',
                text: 'Clarify the remaining area.',
              },
            ],
          },
        ],
      },
      currentPrompt: {
        prompt_id: 'prompt-context-1',
        kind: 'question_batch',
        title: 'Clarify the remaining area',
        instructions: 'Answer the focused question.',
        origin_category_id: 'root-discovery',
        category_path: [
          { category_id: 'root-discovery', title: 'Explore missing areas' },
        ],
        items: [
          {
            item_id: 'item-context-1',
            kind: 'discovery',
            target_dimension: 'Scope',
            section_ref: null,
            text: 'What is still missing?',
            options: [],
            response_mode: 'single_select_with_custom_text',
            required: false,
            priority: 100,
            dependency_item_ids: [],
          },
        ],
        draft_snapshot: null,
        required_item_ids: [],
        allow_partial_submit: true,
        ui_hints: {
          preferred_layout: 'cards',
          show_draft_sidebar: false,
        },
        based_on_turn: 2,
        created_at: '2026-03-22T00:00:00Z',
      },
      speculativeDraft: {
        sections: [{ heading: 'Goal', content: 'Draft goal section' }],
        not_discussed: [],
      },
      confirmedSections: new Set(),
      contradictions: [],
      stages: [],
      pipelineComplete: false,
      pipelineSummary: null,
      events: [
        {
          id: 'evt-context-1',
          timestamp: new Date().toISOString(),
          level: 'info',
          source: 'socratic_engine',
          step: 'socratic.question.focused',
          message: 'Focused branch is ready for another answer.',
          metadata: {},
        },
      ],
      currentStep: 'socratic.question.focused',
      attach: mockAttach,
      sendDescription: mockSendDescription,
    }));

    renderSessionPage('/session/abc');

    await waitFor(() => {
      expect(mockGetSession).toHaveBeenCalledWith('abc');
    });

    expect(screen.queryByLabelText('Context shelf')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Belief State' })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: /context/i })).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: /context/i }));

    const contextShelf = screen.getByLabelText('Context shelf');
    expect(contextShelf).toBeInTheDocument();
    expect(screen.getByText(/open belief state, draft, transcript, or events without leaving the active question flow/i)).toBeInTheDocument();
    expect(within(contextShelf).getByRole('button', { name: 'Belief State' })).toBeInTheDocument();
    expect(within(contextShelf).getByRole('button', { name: 'Draft' })).toBeInTheDocument();
    expect(within(contextShelf).getByRole('button', { name: 'Transcript' })).toBeInTheDocument();
    expect(within(contextShelf).getByRole('button', { name: /^Events/ })).toBeInTheDocument();
    expect(screen.getByText(/draft spec/i)).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Belief State' }));
    expect(screen.getByText('React')).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Transcript' }));
    expect(screen.getAllByText('welcome').length).toBeGreaterThan(0);

    await user.click(within(contextShelf).getByRole('button', { name: /^Events/ }));
    expect(screen.getAllByText(/focused branch is ready for another answer/i).length).toBeGreaterThan(0);
  });

  it('explains branch transitions inline and exposes visible controls to refocus or follow server focus', async () => {
    const user = userEvent.setup();
    const session = makeSession({
      intake_phase: 'interviewing',
      pipeline_running: false,
      can_resume_live: false,
      resume_status: 'interview_attached',
    });
    mockGetSession.mockResolvedValue({ session });
    vi.mocked(useSocraticWebSocket).mockReturnValue(makeMockSocraticState({
      isConnected: true,
      reconnectFailed: false,
      intakePhase: 'interviewing',
      messages: [],
      classification: null,
      beliefState: null,
      convergencePct: 0.5,
      currentCategorySnapshot: null,
      currentWorkspace: {
        revision: 'workspace-transition-1',
        focused_category_id: 'branch-auth',
        branch_notice: 'Planner moved active work to Security follow-up while this branch remains reviewable.',
        category_snapshot: {
          revision: 'category-transition-1',
          root_category_ids: ['branch-auth'],
          nodes: [
            {
              category_id: 'branch-auth',
              parent_category_id: null,
              title: 'Authentication model',
              summary: 'Follow-up work moved to another branch.',
              status: 'ready',
              depth: 0,
              mapped_dimensions: [],
              has_children: false,
              has_prompt_ready: false,
              item_count_hint: 1,
            },
          ],
          active_category_path: [],
          newly_available_category_ids: ['branch-security'],
          build_ready: false,
          build_readiness_message: 'Build is blocked until active branches are reviewed.',
        },
        groups: [
          {
            category_id: 'branch-auth',
            title: 'Authentication model',
            summary: 'Follow-up work moved to another branch.',
            status: 'ready',
            question_count: 1,
            is_focused: true,
            is_new: false,
            preview_items: [
              {
                item_id: 'branch-auth::preview::0',
                kind: 'discovery',
                text: 'Review the authentication branch transition.',
              },
            ],
          },
        ],
      },
      currentPrompt: null,
      speculativeDraft: null,
      confirmedSections: new Set(),
      contradictions: [],
      stages: [],
      pipelineComplete: false,
      pipelineSummary: null,
      events: [],
      currentStep: 'socratic.branch.transitioned',
      attach: mockAttach,
      sendDescription: mockSendDescription,
    }));

    renderSessionPage('/session/abc');

    await waitFor(() => {
      expect(mockGetSession).toHaveBeenCalledWith('abc');
    });

    expect(screen.getAllByText(/planner moved active work to security follow-up while this branch remains reviewable/i).length).toBeGreaterThan(0);

    await user.click(screen.getByRole('button', { name: /go to live question/i }));
    expect(mockBackToCategories).toHaveBeenCalledTimes(1);
  });

  it('does not expose a build completion button while a scoped prompt is active', async () => {
    const session = makeSession({
      intake_phase: 'interviewing',
      pipeline_running: false,
      can_resume_live: false,
      resume_status: 'interview_attached',
    });
    mockGetSession.mockResolvedValue({ session });
    vi.mocked(useSocraticWebSocket).mockReturnValue(makeMockSocraticState({
      isConnected: true,
      reconnectFailed: false,
      intakePhase: 'interviewing',
      messages: [],
      classification: null,
      beliefState: null,
      convergencePct: 0.4,
      currentCategorySnapshot: null,
      currentPrompt: {
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
            section_ref: null,
            text: 'How should authentication work?',
            options: [],
            response_mode: 'single_select_with_custom_text',
            required: false,
            priority: 100,
            dependency_item_ids: [],
          },
        ],
        draft_snapshot: null,
        required_item_ids: [],
        allow_partial_submit: true,
        ui_hints: {
          preferred_layout: 'cards',
          show_draft_sidebar: false,
        },
        based_on_turn: 2,
        created_at: '2026-03-21T00:00:00Z',
      },
      speculativeDraft: null,
      confirmedSections: new Set(),
      contradictions: [],
      stages: [],
      pipelineComplete: false,
      pipelineSummary: null,
      events: [],
      currentStep: null,
      attach: mockAttach,
      sendDescription: mockSendDescription,
    }));

    renderSessionPage('/session/abc');

    await waitFor(() => {
      expect(mockGetSession).toHaveBeenCalledWith('abc');
    });

    expect(screen.getByText(/clarify security/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /submit prompt answers/i })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /done with interview/i })).not.toBeInTheDocument();
  });

  it('keeps legacy event-role chat messages out of the conversation pane', async () => {
    const session = makeSession({
      intake_phase: 'pipeline_running',
      pipeline_running: true,
    });
    mockGetSession.mockResolvedValue({ session });
    vi.mocked(useSocraticWebSocket).mockReturnValue(makeMockSocraticState({
      isConnected: true,
      reconnectFailed: false,
      intakePhase: 'pipeline_running',
      messages: [
        { id: 'legacy-event', role: 'event', content: 'legacy event payload', timestamp: new Date().toISOString() },
        { id: 'planner-1', role: 'planner', content: 'Planner visible message', timestamp: new Date().toISOString() },
      ],
      classification: null,
      beliefState: null,
      convergencePct: 0.8,
      currentCategorySnapshot: null,
      currentPrompt: null,
      speculativeDraft: null,
      confirmedSections: new Set(),
      contradictions: [],
      stages: [],
      pipelineComplete: false,
      pipelineSummary: null,
      events: [],
      currentStep: 'pipeline.wait',
      attach: mockAttach,
      sendDescription: mockSendDescription,
    }));

    renderSessionPage('/session/abc');

    await waitFor(() => {
      expect(mockGetSession).toHaveBeenCalledWith('abc');
    });

    expect(screen.queryByText('legacy event payload')).not.toBeInTheDocument();
    expect(screen.getByText('Planner visible message')).toBeInTheDocument();
  });
});
