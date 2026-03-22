import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import SessionPage from '../SessionPage';
import type { Session } from '../../types';
import { useSocraticWebSocket } from '../../hooks/useSocraticWebSocket';

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

describe('SessionPage resume behavior', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockGetAccessToken.mockResolvedValue('mock-token');
    vi.spyOn(window, 'prompt').mockReturnValue(null);
    vi.spyOn(window, 'confirm').mockReturnValue(true);
    vi.spyOn(window.URL, 'createObjectURL').mockReturnValue('blob:session-export');
    vi.spyOn(window.URL, 'revokeObjectURL').mockImplementation(() => undefined);
    vi.mocked(useSocraticWebSocket).mockReturnValue({
      isConnected: false,
      reconnectFailed: false,
      intakePhase: 'waiting',
      messages: [],
      classification: null,
      beliefState: null,
      convergencePct: 0,
      currentCategorySnapshot: null,
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
    });
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
    vi.mocked(useSocraticWebSocket).mockReturnValue({
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
      submitPromptAnswers: vi.fn(),
      enterCategory: mockEnterCategory,
      backToCategories: mockBackToCategories,
      sendDone: vi.fn(),
      sendDimensionEdit: vi.fn(),
    });

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
    vi.mocked(useSocraticWebSocket).mockReturnValue({
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
      submitPromptAnswers: vi.fn(),
      enterCategory: mockEnterCategory,
      backToCategories: mockBackToCategories,
      sendDone: vi.fn(),
      sendDimensionEdit: vi.fn(),
    });

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
    vi.mocked(useSocraticWebSocket).mockReturnValue({
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
      submitPromptAnswers: vi.fn(),
      enterCategory: mockEnterCategory,
      backToCategories: mockBackToCategories,
      sendDone: vi.fn(),
      sendDimensionEdit: vi.fn(),
    });

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
    vi.mocked(useSocraticWebSocket).mockReturnValue({
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
      submitPromptAnswers: vi.fn(),
      enterCategory: mockEnterCategory,
      backToCategories: mockBackToCategories,
      sendDone: vi.fn(),
      sendDimensionEdit: vi.fn(),
    });

    renderSessionPage('/session/abc');

    await waitFor(() => {
      expect(mockGetSession).toHaveBeenCalledWith('abc');
    });

    expect(screen.getByRole('region', { name: /interview progress/i })).toBeInTheDocument();
    expect(screen.getByText(/generating your next questions/i)).toBeInTheDocument();
    expect(screen.getAllByText(/planning the next question batch/i).length).toBeGreaterThan(0);
    expect(screen.getByText(/prompt response adjudicated at 15% convergence/i)).toBeInTheDocument();
  });

  it('renders the category navigator during category-driven intake', async () => {
    const session = makeSession({
      intake_phase: 'interviewing',
      pipeline_running: false,
      can_resume_live: false,
      resume_status: 'interview_attached',
    });
    mockGetSession.mockResolvedValue({ session });
    vi.mocked(useSocraticWebSocket).mockReturnValue({
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
      submitPromptAnswers: vi.fn(),
      enterCategory: mockEnterCategory,
      backToCategories: mockBackToCategories,
      sendDone: vi.fn(),
      sendDimensionEdit: vi.fn(),
    });

    renderSessionPage('/session/abc');

    await waitFor(() => {
      expect(mockGetSession).toHaveBeenCalledWith('abc');
    });

    expect(screen.getByRole('region', { name: /interview categories/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /explore missing areas/i })).toBeInTheDocument();
  });

  it('does not expose a build completion button while a scoped prompt is active', async () => {
    const session = makeSession({
      intake_phase: 'interviewing',
      pipeline_running: false,
      can_resume_live: false,
      resume_status: 'interview_attached',
    });
    mockGetSession.mockResolvedValue({ session });
    vi.mocked(useSocraticWebSocket).mockReturnValue({
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
      submitPromptAnswers: vi.fn(),
      enterCategory: mockEnterCategory,
      backToCategories: mockBackToCategories,
      sendDone: vi.fn(),
      sendDimensionEdit: vi.fn(),
    });

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
    vi.mocked(useSocraticWebSocket).mockReturnValue({
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
      submitPromptAnswers: vi.fn(),
      enterCategory: mockEnterCategory,
      backToCategories: mockBackToCategories,
      sendDone: vi.fn(),
      sendDimensionEdit: vi.fn(),
    });

    renderSessionPage('/session/abc');

    await waitFor(() => {
      expect(mockGetSession).toHaveBeenCalledWith('abc');
    });

    expect(screen.queryByText('legacy event payload')).not.toBeInTheDocument();
    expect(screen.getByText('Planner visible message')).toBeInTheDocument();
  });
});
