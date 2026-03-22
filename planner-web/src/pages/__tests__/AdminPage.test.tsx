import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import AdminPage from '../AdminPage';
import type { AdminEventEntry, AdminStatusResponse } from '../../types.ts';

const mockAdminStatus = vi.fn();
const mockAdminEvents = vi.fn();

vi.mock('../../api/client.ts', () => ({
  createApiClient: vi.fn(() => ({
    adminStatus: mockAdminStatus,
    adminEvents: mockAdminEvents,
  })),
}));

vi.mock('../../auth/useAuthenticatedFetch.ts', () => ({
  useGetAccessToken: vi.fn(() => vi.fn().mockResolvedValue('mock-token')),
}));

vi.mock('../../components/Layout.tsx', () => ({
  default: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
}));

function LocationSnapshot() {
  const location = useLocation();
  return (
    <div>
      <div data-testid="location-path">{location.pathname}</div>
      <div data-testid="location-search">{location.search}</div>
    </div>
  );
}

function renderAdminPage() {
  render(
    <MemoryRouter initialEntries={['/admin']}>
      <Routes>
        <Route path="/admin" element={<AdminPage />} />
        <Route path="/knowledge/projects/:projectId" element={<LocationSnapshot />} />
      </Routes>
    </MemoryRouter>,
  );
}

function makeStatus(overrides: Partial<AdminStatusResponse> = {}): AdminStatusResponse {
  return {
    status: 'ok',
    version: '0.1.0',
    uptime_secs: 90,
    sessions: { active: 1, total_events: 1 },
    providers: [],
    ...overrides,
  };
}

function makeEvent(overrides: Partial<AdminEventEntry> = {}): AdminEventEntry {
  return {
    id: 'evt-1',
    timestamp: '2026-03-18T12:00:00Z',
    level: 'info',
    source: 'pipeline',
    session_id: 'abc12345-def6-7890-abcd-ef1234567890',
    project_id: 'proj-admin-knowledge',
    project_name: 'Admin Knowledge Project',
    step: 'pipeline.compile',
    message: 'Compiled project blueprint',
    metadata: {},
    ...overrides,
  };
}

describe('AdminPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockAdminStatus.mockResolvedValue(makeStatus());
  });

  it('renders the command-center admin header and major operational zones', async () => {
    mockAdminEvents.mockResolvedValue({
      total: 0,
      events: [],
    });

    renderAdminPage();

    await waitFor(() => {
      expect(mockAdminStatus).toHaveBeenCalledTimes(1);
      expect(mockAdminEvents).toHaveBeenCalledWith({ limit: 200 });
    });

    expect(screen.getByText('Operations')).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Admin' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Runtime status' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Event log' })).toBeInTheDocument();
  });

  it('opens project-scoped knowledge from admin event rows when project identity is available', async () => {
    const user = userEvent.setup();
    mockAdminEvents.mockResolvedValue({
      total: 1,
      events: [makeEvent()],
    });

    renderAdminPage();

    await waitFor(() => {
      expect(mockAdminStatus).toHaveBeenCalledTimes(1);
      expect(mockAdminEvents).toHaveBeenCalledWith({ limit: 200 });
    });

    await user.click(screen.getByTitle('Open Knowledge for Admin Knowledge Project'));

    expect(await screen.findByTestId('location-path')).toHaveTextContent('/knowledge/projects/proj-admin-knowledge');
    const params = new URLSearchParams(screen.getByTestId('location-search').textContent ?? '');
    expect(params.get('project')).toBe('proj-admin-knowledge');
    expect(params.get('from')).toBe('/admin');
    expect(params.get('from_label')).toBe('Admin');
  });

  it('shows a healthy operator summary and empty event state when nothing urgent is visible', async () => {
    mockAdminStatus.mockResolvedValue(makeStatus({
      sessions: { active: 0, total_events: 0 },
    }));
    mockAdminEvents.mockResolvedValue({
      total: 0,
      events: [],
    });

    renderAdminPage();

    await waitFor(() => {
      expect(mockAdminEvents).toHaveBeenCalledWith({ limit: 200 });
    });

    expect(await screen.findByText(/runtime is healthy\./i)).toBeInTheDocument();
    expect(screen.getByText(/0 total events/i)).toBeInTheDocument();
  });

  it('shows warning posture and supports level filtering in the event log', async () => {
    const user = userEvent.setup();
    mockAdminEvents.mockResolvedValue({
      total: 2,
      events: [
        makeEvent({
          id: 'evt-warn',
          level: 'warn',
          message: 'Queue latency is rising',
          project_id: 'proj-warning',
          project_name: 'Warning Project',
        }),
        makeEvent({
          id: 'evt-info',
          level: 'info',
          message: 'Heartbeat received',
          project_id: 'proj-info',
          project_name: 'Info Project',
        }),
      ],
    });

    renderAdminPage();

    await waitFor(() => {
      expect(mockAdminEvents).toHaveBeenCalledWith({ limit: 200 });
    });
    expect(await screen.findByText(/runtime is healthy, but warnings need review\./i)).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: /^warn$/i }));

    await waitFor(() => {
      expect(screen.getByTitle('Open Knowledge for Warning Project')).toBeInTheDocument();
      expect(screen.queryByTitle('Open Knowledge for Info Project')).not.toBeInTheDocument();
    });
  });

  it('surfaces operator-attention posture when providers are unavailable', async () => {
    mockAdminStatus.mockResolvedValue(makeStatus({
      providers: [
        {
          name: 'openai',
          binary: 'openai',
          available: false,
        },
      ],
    }));
    mockAdminEvents.mockResolvedValue({
      total: 1,
      events: [makeEvent({ level: 'error', message: 'Provider check failed' })],
    });

    renderAdminPage();

    await waitFor(() => {
      expect(mockAdminEvents).toHaveBeenCalledWith({ limit: 200 });
    });
    expect(await screen.findByText(/operator attention is required\./i)).toBeInTheDocument();
    expect(screen.getByText(/1 visible error event/i)).toBeInTheDocument();
  });

  it('surfaces event-log load failures', async () => {
    mockAdminEvents.mockRejectedValueOnce(new Error('events down'));

    renderAdminPage();

    await waitFor(() => {
      expect(screen.getByText(/events down/i)).toBeInTheDocument();
    });
  });
});
