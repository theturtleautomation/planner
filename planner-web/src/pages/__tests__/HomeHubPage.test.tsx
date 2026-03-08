import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import HomeHubPage from '../HomeHubPage';

const mockListProjects = vi.fn();
const mockCreateProject = vi.fn();

vi.mock('../../api/client.ts', () => ({
  createApiClient: vi.fn(() => ({
    listProjects: mockListProjects,
    createProject: mockCreateProject,
  })),
}));

vi.mock('../../auth/useAuthenticatedFetch.ts', () => ({
  useGetAccessToken: vi.fn(() => vi.fn().mockResolvedValue('mock-token')),
}));

function renderHome() {
  render(
    <MemoryRouter initialEntries={['/']}>
      <Routes>
        <Route path="/" element={<HomeHubPage />} />
        <Route path="/projects" element={<div>Projects Route</div>} />
        <Route path="/sessions" element={<div>Sessions Route</div>} />
        <Route path="/knowledge" element={<div>Knowledge Route</div>} />
        <Route path="/admin" element={<div>Admin Route</div>} />
        <Route path="/projects/:projectSlug/sessions" element={<div>Project Sessions Route</div>} />
      </Routes>
    </MemoryRouter>,
  );
}

describe('HomeHubPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockCreateProject.mockResolvedValue({
      project: {
        id: 'p-created',
        slug: 'created-project',
        name: 'Created Project',
        description: null,
        owner_user_id: 'dev|local',
        created_at: '2026-03-07T00:00:00Z',
        updated_at: '2026-03-07T00:00:00Z',
        archived_at: null,
        legacy_scope_keys: [],
      },
    });
  });

  it('renders quick actions and recent projects', async () => {
    mockListProjects.mockResolvedValue({
      projects: [
        {
          id: 'p1',
          slug: 'alpha-project',
          name: 'Alpha Project',
          description: 'Core migration work',
          owner_user_id: 'dev|local',
          created_at: '2026-03-07T00:00:00Z',
          updated_at: '2026-03-07T02:00:00Z',
          archived_at: null,
          legacy_scope_keys: [],
        },
      ],
    });

    renderHome();

    await waitFor(() => {
      expect(mockListProjects).toHaveBeenCalledTimes(1);
    });

    expect(screen.getByRole('heading', { name: 'Home' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /open projects/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /new project/i })).toBeInTheDocument();
    expect(screen.getByText('Alpha Project')).toBeInTheDocument();
  });

  it('routes prompt intent to projects', async () => {
    const user = userEvent.setup();
    mockListProjects.mockResolvedValue({ projects: [] });

    renderHome();

    await waitFor(() => {
      expect(mockListProjects).toHaveBeenCalledTimes(1);
    });

    await user.type(screen.getByRole('textbox', { name: /home intent prompt/i }), 'open projects');
    await user.click(screen.getByRole('button', { name: 'Go' }));

    expect(await screen.findByText('Projects Route')).toBeInTheDocument();
  });

  it('routes matching project names to project sessions', async () => {
    const user = userEvent.setup();
    mockListProjects.mockResolvedValue({
      projects: [
        {
          id: 'p1',
          slug: 'alpha-project',
          name: 'Alpha Project',
          description: null,
          owner_user_id: 'dev|local',
          created_at: '2026-03-07T00:00:00Z',
          updated_at: '2026-03-07T02:00:00Z',
          archived_at: null,
          legacy_scope_keys: [],
        },
      ],
    });

    renderHome();

    await waitFor(() => {
      expect(mockListProjects).toHaveBeenCalledTimes(1);
    });

    await user.type(screen.getByRole('textbox', { name: /home intent prompt/i }), 'alpha project');
    await user.click(screen.getByRole('button', { name: 'Go' }));

    expect(await screen.findByText('Project Sessions Route')).toBeInTheDocument();
  });
});
