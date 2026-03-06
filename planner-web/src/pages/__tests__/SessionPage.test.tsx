import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import SessionPage from '../SessionPage';
import type { Session } from '../../types';
import { useSocraticWebSocket } from '../../hooks/useSocraticWebSocket';

const mockCreateSession = vi.fn();
const mockGetSession = vi.fn();
const mockStartSocratic = vi.fn();
const mockRestartFromDescription = vi.fn();
const mockRetryPipeline = vi.fn();
const mockUpdateSession = vi.fn();
const mockDuplicateSession = vi.fn();
const mockExportSession = vi.fn();
const mockAttach = vi.fn();
const mockSendDescription = vi.fn();

vi.mock('../../api/client.ts', () => ({
  createApiClient: vi.fn(() => ({
    createSession: mockCreateSession,
    getSession: mockGetSession,
    startSocratic: mockStartSocratic,
    restartFromDescription: mockRestartFromDescription,
    retryPipeline: mockRetryPipeline,
    updateSession: mockUpdateSession,
    duplicateSession: mockDuplicateSession,
    exportSession: mockExportSession,
  })),
}));

vi.mock('../../auth/useAuthenticatedFetch.ts', () => ({
  useGetAccessToken: vi.fn(() => vi.fn().mockResolvedValue('mock-token')),
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

function renderSessionPage(path = '/session/abc') {
  render(
    <MemoryRouter initialEntries={[path]}>
      <Routes>
        <Route path="/session/:id" element={<SessionPage />} />
        <Route path="/session/new" element={<SessionPage />} />
      </Routes>
    </MemoryRouter>,
  );
}

describe('SessionPage resume behavior', () => {
  beforeEach(() => {
    vi.clearAllMocks();
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
      currentQuestion: null,
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
      sendResponse: vi.fn(),
      skipQuestion: vi.fn(),
      sendDone: vi.fn(),
      sendDraftReaction: vi.fn(),
      sendDimensionEdit: vi.fn(),
    });
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
        current_question: {
          question: 'What are the core user roles?',
          target_dimension: 'Stakeholders',
          quick_options: [],
          allow_skip: true,
        },
        pending_draft: {
          sections: [{ heading: 'Goal', content: 'Draft goal section' }],
          assumptions: [],
          not_discussed: [],
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
    expect(screen.getByText(/current question: what are the core user roles\?/i)).toBeInTheDocument();
    expect(screen.getByText(/pending draft: goal/i)).toBeInTheDocument();
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
    expect(screen.getByRole('button', { name: /back to dashboard/i })).toBeInTheDocument();
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
});
