import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import ProjectsPage from '../ProjectsPage';

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

function renderProjects(initialPath = '/projects') {
  render(
    <MemoryRouter initialEntries={[initialPath]}>
      <Routes>
        <Route path="/projects" element={<ProjectsPage />} />
        <Route path="/projects/:projectSlug/sessions" element={<div>Project Sessions Route</div>} />
      </Routes>
    </MemoryRouter>,
  );
}

describe('ProjectsPage', () => {
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

  it('renders projects from the projects API', async () => {
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

    renderProjects();

    await waitFor(() => {
      expect(mockListProjects).toHaveBeenCalledTimes(1);
    });

    expect(screen.getByRole('heading', { name: 'Projects' })).toBeInTheDocument();
    expect(screen.getByText('Alpha Project')).toBeInTheDocument();
    expect(screen.getByText('alpha-project')).toBeInTheDocument();
  });

  it('applies query filtering from URL params', async () => {
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
        {
          id: 'p2',
          slug: 'beta-platform',
          name: 'Beta Platform',
          description: 'Analytics surface',
          owner_user_id: 'dev|local',
          created_at: '2026-03-07T00:00:00Z',
          updated_at: '2026-03-07T01:00:00Z',
          archived_at: null,
          legacy_scope_keys: [],
        },
      ],
    });

    renderProjects('/projects?query=beta');

    await waitFor(() => {
      expect(mockListProjects).toHaveBeenCalledTimes(1);
    });

    expect(screen.queryByText('Alpha Project')).not.toBeInTheDocument();
    expect(screen.getByText('Beta Platform')).toBeInTheDocument();
  });

  it('navigates into project sessions from the Open action', async () => {
    const user = userEvent.setup();
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

    renderProjects();

    await waitFor(() => {
      expect(mockListProjects).toHaveBeenCalledTimes(1);
    });

    await user.click(screen.getByRole('button', { name: 'Open' }));

    expect(await screen.findByText('Project Sessions Route')).toBeInTheDocument();
  });
});
