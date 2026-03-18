import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import ProjectsPage from '../ProjectsPage';
import { createApiClient } from '../../api/client.ts';

const {
  mockListProjects,
  mockCreateProject,
  mockUpdateProject,
  mockDeleteProject,
  mockGetToken,
} = vi.hoisted(() => ({
  mockListProjects: vi.fn(),
  mockCreateProject: vi.fn(),
  mockUpdateProject: vi.fn(),
  mockDeleteProject: vi.fn(),
  mockGetToken: vi.fn().mockResolvedValue('mock-token'),
}));

vi.mock('../../api/client.ts', () => ({
  createApiClient: vi.fn(() => ({
    listProjects: mockListProjects,
    createProject: mockCreateProject,
    updateProject: mockUpdateProject,
    deleteProject: mockDeleteProject,
  })),
}));

vi.mock('../../auth/useAuthenticatedFetch.ts', () => ({
  useGetAccessToken: vi.fn(() => mockGetToken),
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
    vi.mocked(createApiClient).mockImplementation(() => ({
      listProjects: mockListProjects,
      createProject: mockCreateProject,
      updateProject: mockUpdateProject,
      deleteProject: mockDeleteProject,
    }));
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
    mockUpdateProject.mockResolvedValue({
      project: {
        id: 'p1',
        slug: 'alpha-project',
        name: 'Alpha Project',
        description: 'Core migration work',
        owner_user_id: 'dev|local',
        created_at: '2026-03-07T00:00:00Z',
        updated_at: '2026-03-07T03:00:00Z',
        archived_at: null,
        legacy_scope_keys: [],
      },
    });
    mockDeleteProject.mockResolvedValue({
      project_id: 'p1',
      project_name: 'Alpha Project',
      stopped_live_sessions: 0,
      stopped_pipeline_sessions: 0,
      deleted_sessions: 0,
      deleted_session_event_files: 0,
      deleted_cxdb_runs: 0,
      deleted_blueprint_nodes: 0,
      unlinked_shared_blueprint_nodes: 0,
      deleted_project_record: true,
      blueprint_events_pruned: 0,
      blueprint_history_snapshots_pruned: 0,
    });
    mockGetToken.mockResolvedValue('mock-token');
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

  it('hides archived projects by default', async () => {
    mockListProjects.mockResolvedValue({
      projects: [
        {
          id: 'p1',
          slug: 'active-project',
          name: 'Active Project',
          description: 'Visible project',
          owner_user_id: 'dev|local',
          created_at: '2026-03-07T00:00:00Z',
          updated_at: '2026-03-07T02:00:00Z',
          archived_at: null,
          legacy_scope_keys: [],
        },
        {
          id: 'p2',
          slug: 'archived-project',
          name: 'Archived Project',
          description: 'Hidden by default',
          owner_user_id: 'dev|local',
          created_at: '2026-03-07T00:00:00Z',
          updated_at: '2026-03-07T01:00:00Z',
          archived_at: '2026-03-08T01:00:00Z',
          legacy_scope_keys: [],
        },
      ],
    });

    renderProjects();

    await waitFor(() => {
      expect(mockListProjects).toHaveBeenCalledWith({ includeArchived: false });
    });

    expect(screen.getByText('Active Project')).toBeInTheDocument();
    expect(screen.queryByText('Archived Project')).not.toBeInTheDocument();
  });

  it('shows archived projects when filter is enabled', async () => {
    const user = userEvent.setup();
    mockListProjects.mockResolvedValue({
      projects: [
        {
          id: 'p2',
          slug: 'archived-project',
          name: 'Archived Project',
          description: 'Should become visible',
          owner_user_id: 'dev|local',
          created_at: '2026-03-07T00:00:00Z',
          updated_at: '2026-03-07T01:00:00Z',
          archived_at: '2026-03-08T01:00:00Z',
          legacy_scope_keys: [],
        },
      ],
    });

    renderProjects();

    await waitFor(() => {
      expect(mockListProjects).toHaveBeenCalledWith({ includeArchived: false });
    });

    await user.click(screen.getByRole('checkbox', { name: 'Show archived' }));

    await waitFor(() => {
      expect(mockListProjects).toHaveBeenLastCalledWith({ includeArchived: true });
    });

    expect(screen.getByText('Archived Project')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Unarchive' })).toBeInTheDocument();
  });

  it('archives a project and reloads the list', async () => {
    const user = userEvent.setup();
    let resolveArchive: ((value: unknown) => void) | null = null;

    mockListProjects
      .mockResolvedValueOnce({
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
      })
      .mockResolvedValueOnce({
        projects: [
          {
            id: 'p1',
            slug: 'alpha-project',
            name: 'Alpha Project',
            description: 'Core migration work',
            owner_user_id: 'dev|local',
            created_at: '2026-03-07T00:00:00Z',
            updated_at: '2026-03-07T03:00:00Z',
            archived_at: '2026-03-08T01:00:00Z',
            legacy_scope_keys: [],
          },
        ],
      });
    mockUpdateProject.mockImplementation(
      () => new Promise((resolve) => {
        resolveArchive = resolve;
      }),
    );

    renderProjects();

    await waitFor(() => {
      expect(mockListProjects).toHaveBeenCalledTimes(1);
    });

    const archiveButton = screen.getByRole('button', { name: 'Archive' });
    await user.click(archiveButton);

    expect(mockUpdateProject).toHaveBeenCalledWith('alpha-project', { archived: true });
    expect(screen.getByRole('button', { name: 'Archiving…' })).toBeDisabled();

    resolveArchive?.({
      project: {
        id: 'p1',
        slug: 'alpha-project',
        name: 'Alpha Project',
        archived_at: '2026-03-08T01:00:00Z',
      },
    });

    await waitFor(() => {
      expect(mockListProjects).toHaveBeenCalledTimes(2);
    });
  });

  it('archive failure renders error and leaves project visible', async () => {
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
    mockUpdateProject.mockRejectedValue(new Error('Archive failed'));

    renderProjects();

    await waitFor(() => {
      expect(mockListProjects).toHaveBeenCalledTimes(1);
    });

    await user.click(screen.getByRole('button', { name: 'Archive' }));

    expect(mockUpdateProject).toHaveBeenCalledWith('alpha-project', { archived: true });
    expect(await screen.findByText(/Failed to load projects: Archive failed/)).toBeInTheDocument();
    expect(screen.getByText('Alpha Project')).toBeInTheDocument();
    expect(mockListProjects).toHaveBeenCalledTimes(1);
  });

  it('unarchives a project and reloads the list', async () => {
    const user = userEvent.setup();
    mockListProjects
      .mockResolvedValueOnce({
        projects: [
          {
            id: 'p2',
            slug: 'archived-project',
            name: 'Archived Project',
            description: 'Hidden by default',
            owner_user_id: 'dev|local',
            created_at: '2026-03-07T00:00:00Z',
            updated_at: '2026-03-07T01:00:00Z',
            archived_at: '2026-03-08T01:00:00Z',
            legacy_scope_keys: [],
          },
        ],
      })
      .mockResolvedValueOnce({
        projects: [
          {
            id: 'p2',
            slug: 'archived-project',
            name: 'Archived Project',
            description: 'Restored',
            owner_user_id: 'dev|local',
            created_at: '2026-03-07T00:00:00Z',
            updated_at: '2026-03-07T04:00:00Z',
            archived_at: null,
            legacy_scope_keys: [],
          },
        ],
      });
    mockUpdateProject.mockResolvedValue({ project: { id: 'p2', archived_at: null } });

    renderProjects('/projects?show_archived=true');

    await waitFor(() => {
      expect(mockListProjects).toHaveBeenCalledWith({ includeArchived: true });
    });

    const unarchiveButton = screen.getByRole('button', { name: 'Unarchive' });
    await user.click(unarchiveButton);

    expect(mockUpdateProject).toHaveBeenCalledWith('archived-project', { archived: false });
    await waitFor(() => {
      expect(mockListProjects).toHaveBeenCalledTimes(2);
    });
  });

  it('delete confirmation warns that sessions will be stopped and removed', async () => {
    const user = userEvent.setup();
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(false);
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

    await user.click(screen.getByRole('button', { name: 'Delete' }));

    expect(confirmSpy).toHaveBeenCalledTimes(1);
    const [message] = confirmSpy.mock.calls[0]!;
    expect(message).toContain('Alpha Project');
    expect(message).toContain('permanently');
    expect(message).toContain('stop any active sessions');
    expect(message).toContain('preserve shared knowledge');
    expect(message).toContain('unlinking it from this project');
    expect(message).toContain('cannot be undone');
    confirmSpy.mockRestore();
  });

  it('cancelled delete does not call API', async () => {
    const user = userEvent.setup();
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(false);
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

    await user.click(screen.getByRole('button', { name: 'Delete' }));
    expect(mockDeleteProject).not.toHaveBeenCalled();
    confirmSpy.mockRestore();
  });

  it('confirmed delete calls deleteProject and reloads list', async () => {
    const user = userEvent.setup();
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);
    mockListProjects
      .mockResolvedValueOnce({
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
      })
      .mockResolvedValueOnce({ projects: [] });

    renderProjects();

    await waitFor(() => {
      expect(mockListProjects).toHaveBeenCalledTimes(1);
    });

    await user.click(screen.getByRole('button', { name: 'Delete' }));

    expect(mockDeleteProject).toHaveBeenCalledWith('alpha-project');
    await waitFor(() => {
      expect(mockListProjects).toHaveBeenCalledTimes(2);
    });
    expect(screen.queryByText('Alpha Project')).not.toBeInTheDocument();
    expect(screen.queryByText('Project Sessions Route')).not.toBeInTheDocument();
    confirmSpy.mockRestore();
  });

  it('delete failure renders error and leaves project visible', async () => {
    const user = userEvent.setup();
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);
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
    mockDeleteProject.mockRejectedValue(new Error('Delete failed'));

    renderProjects();

    await waitFor(() => {
      expect(mockListProjects).toHaveBeenCalledTimes(1);
    });

    await user.click(screen.getByRole('button', { name: 'Delete' }));

    expect(await screen.findByText(/Failed to load projects: Delete failed/)).toBeInTheDocument();
    expect(screen.getByText('Alpha Project')).toBeInTheDocument();
    confirmSpy.mockRestore();
  });

  it('delete action is disabled while request is in flight', async () => {
    const user = userEvent.setup();
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);
    let resolveDelete: ((value: unknown) => void) | null = null;
    mockDeleteProject.mockImplementation(
      () => new Promise((resolve) => {
        resolveDelete = resolve;
      }),
    );
    mockListProjects
      .mockResolvedValueOnce({
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
      })
      .mockResolvedValueOnce({ projects: [] });

    renderProjects();

    await waitFor(() => {
      expect(mockListProjects).toHaveBeenCalledTimes(1);
    });

    await user.click(screen.getByRole('button', { name: 'Delete' }));
    expect(screen.getByRole('button', { name: 'Deleting…' })).toBeDisabled();

    resolveDelete?.({
      project_id: 'p1',
      project_name: 'Alpha Project',
      stopped_live_sessions: 0,
      stopped_pipeline_sessions: 0,
      deleted_sessions: 0,
      deleted_session_event_files: 0,
      deleted_cxdb_runs: 0,
      deleted_blueprint_nodes: 0,
      unlinked_shared_blueprint_nodes: 0,
      deleted_project_record: true,
      blueprint_events_pruned: 0,
      blueprint_history_snapshots_pruned: 0,
    });

    await waitFor(() => {
      expect(mockListProjects).toHaveBeenCalledTimes(2);
    });
    confirmSpy.mockRestore();
  });
});
