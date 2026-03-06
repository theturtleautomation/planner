import { render, screen, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { MemoryRouter } from 'react-router-dom';
import Dashboard from '../Dashboard';
import type { SessionSummary } from '../../types';

const mockListSessions = vi.fn();

vi.mock('../../api/client.ts', () => ({
  createApiClient: vi.fn(() => ({
    listSessions: mockListSessions,
  })),
}));

vi.mock('../../auth/useAuthenticatedFetch.ts', () => ({
  useGetAccessToken: vi.fn(() => vi.fn().mockResolvedValue('mock-token')),
}));

function makeSessionSummary(overrides: Partial<SessionSummary>): SessionSummary {
  return {
    id: 'sess-default',
    user_id: 'dev|local',
    created_at: '2026-03-06T12:00:00Z',
    last_accessed: '2026-03-06T12:00:00Z',
    last_activity_at: '2026-03-06T12:00:00Z',
    pipeline_running: false,
    intake_phase: 'waiting',
    interview_live_attached: false,
    project_description: 'Build something useful',
    message_count: 1,
    event_count: 0,
    warning_count: 0,
    error_count: 0,
    current_step: null,
    error_message: null,
    can_resume_live: false,
    can_resume_checkpoint: false,
    can_restart_from_description: false,
    can_retry_pipeline: false,
    has_checkpoint: false,
    resume_status: 'ready_to_start',
    classification: null,
    convergence_pct: null,
    checkpoint_last_saved_at: null,
    ...overrides,
  };
}

function renderDashboard() {
  render(
    <MemoryRouter>
      <Dashboard />
    </MemoryRouter>,
  );
}

describe('Dashboard workflow visibility', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.spyOn(Date, 'now').mockReturnValue(new Date('2026-03-06T15:00:00Z').getTime());
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders activity, resumability, and workflow metadata from session summaries', async () => {
    mockListSessions.mockResolvedValue({
      sessions: [
        makeSessionSummary({
          id: 'sess-checkpoint',
          intake_phase: 'interviewing',
          can_resume_checkpoint: true,
          has_checkpoint: true,
          resume_status: 'interview_checkpoint_resumable',
          current_step: 'draft.generate',
          last_activity_at: '2026-03-06T14:30:00Z',
          checkpoint_last_saved_at: '2026-03-06T14:25:00Z',
          message_count: 5,
          event_count: 8,
          classification: {
            project_type: 'Web App',
            complexity: 'standard',
          },
          convergence_pct: 0.67,
        }),
      ],
    });

    renderDashboard();

    await waitFor(() => {
      expect(mockListSessions).toHaveBeenCalledTimes(1);
    });

    expect(screen.getByText(/last activity 30m ago/i)).toBeInTheDocument();
    expect(screen.getByText(/step: draft \/ generate/i)).toBeInTheDocument();
    expect(screen.getByText('Resume Interview')).toBeInTheDocument();
    expect(screen.getByText(/checkpoint 35m ago/i)).toBeInTheDocument();
    expect(screen.getByText(/67% converged/i)).toBeInTheDocument();
    expect(screen.getByText(/Web App · standard/i)).toBeInTheDocument();
  });

  it('sorts attention and live-resume sessions ahead of lower-priority completed work', async () => {
    mockListSessions.mockResolvedValue({
      sessions: [
        makeSessionSummary({
          id: 'sess-complete',
          intake_phase: 'complete',
          can_resume_live: true,
          resume_status: 'live_attach_available',
          last_activity_at: '2026-03-06T14:59:00Z',
        }),
        makeSessionSummary({
          id: 'sess-live',
          intake_phase: 'interviewing',
          can_resume_live: true,
          resume_status: 'live_attach_available',
          last_activity_at: '2026-03-06T14:55:00Z',
        }),
        makeSessionSummary({
          id: 'sess-error',
          intake_phase: 'error',
          can_retry_pipeline: true,
          error_count: 2,
          error_message: 'Pipeline failed',
          last_activity_at: '2026-03-06T14:50:00Z',
        }),
      ],
    });

    renderDashboard();

    await waitFor(() => {
      expect(mockListSessions).toHaveBeenCalledTimes(1);
    });

    const cards = screen.getAllByRole('button', { name: /open session sess-/i });
    expect(cards.map((card) => card.getAttribute('aria-label'))).toEqual([
      'Open session sess-error',
      'Open session sess-live',
      'Open session sess-complete',
    ]);
  });

  it('shows intervention badges for blocked interviews and warnings', async () => {
    mockListSessions.mockResolvedValue({
      sessions: [
        makeSessionSummary({
          id: 'sess-restart',
          intake_phase: 'interviewing',
          can_restart_from_description: true,
          warning_count: 2,
          resume_status: 'interview_restart_only',
          last_activity_at: '2026-03-06T14:10:00Z',
        }),
      ],
    });

    renderDashboard();

    await waitFor(() => {
      expect(mockListSessions).toHaveBeenCalledTimes(1);
    });

    expect(screen.getByText(/needs restart/i)).toBeInTheDocument();
    expect(screen.getByText(/2 warnings/i)).toBeInTheDocument();
    expect(
      screen.getByText(/the live interview is detached; restart from the saved description to continue\./i),
    ).toBeInTheDocument();
  });
});
