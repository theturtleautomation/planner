import { render, screen, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import SessionPage from '../SessionPage';
import type { Session } from '../../types';
import { useSocraticWebSocket } from '../../hooks/useSocraticWebSocket';

const mockCreateSession = vi.fn();
const mockGetSession = vi.fn();
const mockStartSocratic = vi.fn();
const mockAttach = vi.fn();
const mockSendDescription = vi.fn();

vi.mock('../../api/client.ts', () => ({
  createApiClient: vi.fn(() => ({
    createSession: mockCreateSession,
    getSession: mockGetSession,
    startSocratic: mockStartSocratic,
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
});
