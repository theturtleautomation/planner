import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import AdminPage from '../AdminPage';

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

describe('AdminPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockAdminStatus.mockResolvedValue({
      status: 'ok',
      version: '0.1.0',
      uptime_secs: 90,
      sessions: { active: 1, total_events: 1 },
      providers: [],
    });
  });

  it('opens project-scoped knowledge from admin event rows when project identity is available', async () => {
    const user = userEvent.setup();
    mockAdminEvents.mockResolvedValue({
      total: 1,
      events: [
        {
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
        },
      ],
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
});
