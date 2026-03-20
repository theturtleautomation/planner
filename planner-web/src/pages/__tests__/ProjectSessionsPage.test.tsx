import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import ProjectSessionsPage from '../ProjectSessionsPage.tsx';
import { createApiClient } from '../../api/client.ts';

const {
  mockGetProject,
  mockListProjectSessions,
  mockGetProjectImportState,
  mockGetProjectImportHistory,
  mockGetProjectImportReview,
  mockApplyProjectImportReview,
  mockRestoreProjectImportHistoryEntry,
  mockRestoreProjectImportReviewDraft,
  mockReimportProject,
  mockGetToken,
  MockApiError,
} = vi.hoisted(() => ({
  mockGetProject: vi.fn(),
  mockListProjectSessions: vi.fn(),
  mockGetProjectImportState: vi.fn(),
  mockGetProjectImportHistory: vi.fn(),
  mockGetProjectImportReview: vi.fn(),
  mockApplyProjectImportReview: vi.fn(),
  mockRestoreProjectImportHistoryEntry: vi.fn(),
  mockRestoreProjectImportReviewDraft: vi.fn(),
  mockReimportProject: vi.fn(),
  mockGetToken: vi.fn().mockResolvedValue('mock-token'),
  MockApiError: class ApiError extends Error {
    status: number;

    constructor(message: string, status: number) {
      super(message);
      this.name = 'ApiError';
      this.status = status;
    }
  },
}));

vi.mock('../../api/client.ts', () => ({
  ApiError: MockApiError,
  createApiClient: vi.fn(() => ({
    getProject: mockGetProject,
    listProjectSessions: mockListProjectSessions,
    getProjectImportState: mockGetProjectImportState,
    getProjectImportHistory: mockGetProjectImportHistory,
    getProjectImportReview: mockGetProjectImportReview,
    applyProjectImportReview: mockApplyProjectImportReview,
    restoreProjectImportHistoryEntry: mockRestoreProjectImportHistoryEntry,
    restoreProjectImportReviewDraft: mockRestoreProjectImportReviewDraft,
    reimportProject: mockReimportProject,
  })),
}));

vi.mock('../../auth/useAuthenticatedFetch.ts', () => ({
  useGetAccessToken: vi.fn(() => mockGetToken),
}));

function LocationSnapshot() {
  const location = useLocation();
  return (
    <div>
      <div data-testid="location-path">{location.pathname}</div>
    </div>
  );
}

function renderProjectSessions(path = '/projects/task-tracker/sessions') {
  render(
    <MemoryRouter initialEntries={[path]}>
      <Routes>
        <Route path="/projects/:projectSlug/sessions" element={<ProjectSessionsPage />} />
        <Route path="/projects/:projectSlug/blueprint" element={<LocationSnapshot />} />
        <Route path="/session/:id" element={<LocationSnapshot />} />
      </Routes>
    </MemoryRouter>,
  );
}

describe('ProjectSessionsPage import review', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(createApiClient).mockImplementation(() => ({
      getProject: mockGetProject,
      listProjectSessions: mockListProjectSessions,
      getProjectImportState: mockGetProjectImportState,
      getProjectImportHistory: mockGetProjectImportHistory,
      getProjectImportReview: mockGetProjectImportReview,
      applyProjectImportReview: mockApplyProjectImportReview,
      restoreProjectImportHistoryEntry: mockRestoreProjectImportHistoryEntry,
      restoreProjectImportReviewDraft: mockRestoreProjectImportReviewDraft,
      reimportProject: mockReimportProject,
    }));

    mockGetProject.mockResolvedValue({
      project: {
        id: 'proj-1',
        slug: 'task-tracker',
        name: 'Task Tracker',
        description: 'Import review workspace',
        owner_user_id: 'dev|local',
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:00:00Z',
        archived_at: null,
        legacy_scope_keys: [],
      },
    });
    mockListProjectSessions.mockResolvedValue({
      sessions: [
        {
          id: 'seed-1',
          user_id: 'dev|local',
          title: 'Imported planning brief',
          archived: false,
          archived_at: null,
          created_at: '2026-03-20T00:00:00Z',
          last_accessed: '2026-03-20T00:00:00Z',
          last_activity_at: '2026-03-20T00:05:00Z',
          pipeline_running: false,
          intake_phase: 'waiting',
          interview_live_attached: false,
          project_description: 'Imported planning brief for Task Tracker.',
          project_id: 'proj-1',
          project_slug: 'task-tracker',
          project_name: 'Task Tracker',
          message_count: 1,
          event_count: 0,
          warning_count: 0,
          error_count: 0,
          current_step: null,
          error_message: null,
          can_resume_live: false,
          can_resume_checkpoint: false,
          can_restart_from_description: true,
          can_retry_pipeline: false,
          has_checkpoint: false,
          resume_status: 'ready_to_start',
          classification: null,
          convergence_pct: null,
          checkpoint_last_saved_at: null,
        },
      ],
    });
    mockGetProjectImportReview.mockResolvedValue({
      project: {
        id: 'proj-1',
        slug: 'task-tracker',
        name: 'Task Tracker',
        description: 'Import review workspace',
        owner_user_id: 'dev|local',
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:00:00Z',
        archived_at: null,
        legacy_scope_keys: [],
      },
      import_job: {
        id: 'job-1',
        project_id: 'proj-1',
        provider: 'github',
        requested_ref: 'https://github.com/example/task-tracker',
        status: 'review_pending',
        seed_session_id: 'seed-1',
        analysis_summary: 'Imported draft for Task Tracker from GitHub.',
        progress_message: 'Import draft ready. Review imported context in the seeded session.',
        error_message: null,
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:01:00Z',
      },
      source_binding: {
        project_id: 'proj-1',
        provider: 'github',
        canonical_ref: 'https://github.com/example/task-tracker',
        default_branch: 'main',
        head_revision: 'deadbeef',
        local_root: '/tmp/imports/task-tracker',
        managed_checkout: true,
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:01:00Z',
      },
      import_draft: {
        job_id: 'job-1',
        project_id: 'proj-1',
        analysis_summary: 'Imported draft for Task Tracker from GitHub.',
        source_metadata: {
          provider: 'github',
          canonical_ref: 'https://github.com/example/task-tracker',
          local_root: '/tmp/imports/task-tracker',
          default_branch: 'main',
          head_revision: 'deadbeef',
        },
        discovered_nodes: [{ id: 'comp-auth-a1' }, { id: 'tech-rust-a1' }],
        created_at: '2026-03-20T00:01:00Z',
        updated_at: '2026-03-20T00:01:00Z',
      },
    });
    mockGetProjectImportHistory.mockResolvedValue({
      project: {
        id: 'proj-1',
        slug: 'task-tracker',
        name: 'Task Tracker',
        description: 'Import review workspace',
        owner_user_id: 'dev|local',
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:00:00Z',
        archived_at: null,
        legacy_scope_keys: [],
      },
      source_binding: {
        project_id: 'proj-1',
        provider: 'github',
        canonical_ref: 'https://github.com/example/task-tracker',
        default_branch: 'main',
        head_revision: 'deadbeef',
        local_root: '/tmp/imports/task-tracker',
        managed_checkout: true,
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:01:00Z',
      },
      history: [
        {
          import_job: {
            id: 'job-1',
            project_id: 'proj-1',
            provider: 'github',
            requested_ref: 'https://github.com/example/task-tracker',
            status: 'review_pending',
            seed_session_id: 'seed-1',
            analysis_summary: 'Imported draft for Task Tracker from GitHub.',
            progress_message: 'Import draft ready. Review imported context in the seeded session.',
            error_message: null,
            created_at: '2026-03-20T00:00:00Z',
            updated_at: '2026-03-20T00:01:00Z',
          },
          source_metadata: {
            provider: 'github',
            canonical_ref: 'https://github.com/example/task-tracker',
            local_root: '/tmp/imports/task-tracker',
            default_branch: 'main',
            head_revision: 'deadbeef',
          },
          discovered_node_count: 2,
        },
        {
          import_job: {
            id: 'job-0',
            project_id: 'proj-1',
            provider: 'github',
            requested_ref: 'https://github.com/example/task-tracker',
            status: 'applied',
            seed_session_id: 'seed-0',
            analysis_summary: 'Earlier import draft for Task Tracker from GitHub.',
            progress_message: 'Import draft applied and reconciled against the canonical project blueprint.',
            error_message: null,
            created_at: '2026-03-19T23:00:00Z',
            updated_at: '2026-03-19T23:10:00Z',
          },
          source_metadata: {
            provider: 'github',
            canonical_ref: 'https://github.com/example/task-tracker',
            local_root: '/tmp/imports/task-tracker',
            default_branch: 'main',
            head_revision: 'cafebabe',
          },
          discovered_node_count: 1,
        },
      ],
      diff_summary: {
        current_job_id: 'job-1',
        compared_to_job_id: 'job-0',
        added_nodes: [
          { node_id: 'tech-rust-a1', node_name: 'Rust', node_type: 'technology' },
        ],
        removed_nodes: [],
        added_node_types: [{ node_type: 'technology', count: 1 }],
        removed_node_types: [],
        current_head_revision: 'deadbeef',
        compared_head_revision: 'cafebabe',
      },
    });
    mockGetProjectImportState.mockResolvedValue({
      project: {
        id: 'proj-1',
        slug: 'task-tracker',
        name: 'Task Tracker',
        description: 'Import review workspace',
        owner_user_id: 'dev|local',
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:00:00Z',
        archived_at: null,
        legacy_scope_keys: [],
      },
      import_job: {
        id: 'job-1',
        project_id: 'proj-1',
        provider: 'github',
        requested_ref: 'https://github.com/example/task-tracker',
        status: 'review_pending',
        seed_session_id: 'seed-1',
        analysis_summary: 'Imported draft for Task Tracker from GitHub.',
        progress_message: 'Import draft ready. Review imported context in the seeded session.',
        error_message: null,
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:01:00Z',
      },
      source_binding: {
        project_id: 'proj-1',
        provider: 'github',
        canonical_ref: 'https://github.com/example/task-tracker',
        default_branch: 'main',
        head_revision: 'deadbeef',
        local_root: '/tmp/imports/task-tracker',
        managed_checkout: true,
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:01:00Z',
      },
      import_draft: {
        job_id: 'job-1',
        project_id: 'proj-1',
        analysis_summary: 'Imported draft for Task Tracker from GitHub.',
        source_metadata: {
          provider: 'github',
          canonical_ref: 'https://github.com/example/task-tracker',
          local_root: '/tmp/imports/task-tracker',
          default_branch: 'main',
          head_revision: 'deadbeef',
        },
        discovered_nodes: [{ id: 'comp-auth-a1' }, { id: 'tech-rust-a1' }],
        created_at: '2026-03-20T00:01:00Z',
        updated_at: '2026-03-20T00:01:00Z',
      },
    });
    mockApplyProjectImportReview.mockResolvedValue({
      project: {
        id: 'proj-1',
        slug: 'task-tracker',
        name: 'Task Tracker',
        description: 'Import review workspace',
        owner_user_id: 'dev|local',
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:00:00Z',
        archived_at: null,
        legacy_scope_keys: [],
      },
      import_job: {
        id: 'job-1',
        project_id: 'proj-1',
        provider: 'github',
        requested_ref: 'https://github.com/example/task-tracker',
        status: 'applied',
        seed_session_id: 'seed-1',
        analysis_summary: 'Imported draft for Task Tracker from GitHub.',
        progress_message: 'Import draft applied and reconciled against the canonical project blueprint.',
        error_message: null,
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:02:00Z',
      },
      source_binding: {
        project_id: 'proj-1',
        provider: 'github',
        canonical_ref: 'https://github.com/example/task-tracker',
        default_branch: 'main',
        head_revision: 'deadbeef',
        local_root: '/tmp/imports/task-tracker',
        managed_checkout: true,
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:01:00Z',
      },
      import_draft: {
        job_id: 'job-1',
        project_id: 'proj-1',
        analysis_summary: 'Imported draft for Task Tracker from GitHub.',
        source_metadata: {
          provider: 'github',
          canonical_ref: 'https://github.com/example/task-tracker',
          local_root: '/tmp/imports/task-tracker',
          default_branch: 'main',
          head_revision: 'deadbeef',
        },
        discovered_nodes: [{ id: 'comp-auth-a1' }, { id: 'tech-rust-a1' }],
        created_at: '2026-03-20T00:01:00Z',
        updated_at: '2026-03-20T00:02:00Z',
      },
    });
    mockReimportProject.mockResolvedValue({
      project: {
        id: 'proj-1',
        slug: 'task-tracker',
        name: 'Task Tracker',
        description: 'Import review workspace',
        owner_user_id: 'dev|local',
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:00:00Z',
        archived_at: null,
        legacy_scope_keys: [],
      },
      import_job: {
        id: 'job-2',
        project_id: 'proj-1',
        provider: 'github',
        requested_ref: 'https://github.com/example/task-tracker',
        status: 'queued',
        seed_session_id: null,
        analysis_summary: null,
        progress_message: 'Re-import request queued',
        error_message: null,
        created_at: '2026-03-20T00:03:00Z',
        updated_at: '2026-03-20T00:03:00Z',
      },
      source_binding: {
        project_id: 'proj-1',
        provider: 'github',
        canonical_ref: 'https://github.com/example/task-tracker',
        default_branch: 'main',
        head_revision: 'deadbeef',
        local_root: '/tmp/imports/task-tracker',
        managed_checkout: true,
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:03:00Z',
      },
      import_draft: null,
    });
  });

  it('renders the pending import review card on the project sessions page', async () => {
    renderProjectSessions();

    await waitFor(() => {
      expect(mockGetProjectImportReview).toHaveBeenCalledWith('task-tracker');
    });
    expect(mockGetProjectImportState).toHaveBeenCalledWith('task-tracker');
    expect(mockGetProjectImportHistory).toHaveBeenCalledWith('task-tracker');

    expect(screen.getByText('Import draft ready for project review')).toBeInTheDocument();
    expect(screen.getByText(/Draft records: 2/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Re-import' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Open Seeded Session' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Apply Import Draft' })).toBeInTheDocument();
    expect(screen.getByText('Import History')).toBeInTheDocument();
    expect(screen.getByText('Changes Since Last Applied Import')).toBeInTheDocument();
    expect(screen.getByText(/Added nodes: Rust/)).toBeInTheDocument();
    expect(screen.getByText(/Applying this draft will reconcile import-owned project blueprint state/i)).toBeInTheDocument();
    expect(screen.getByText(/Resolve the pending import review before restoring an older applied import/i)).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Restore This Import' })).not.toBeInTheDocument();
  });

  it('applies the import draft and exposes blueprint navigation', async () => {
    const user = userEvent.setup();
    renderProjectSessions();

    await waitFor(() => {
      expect(mockGetProjectImportReview).toHaveBeenCalledWith('task-tracker');
    });

    await user.click(screen.getByRole('button', { name: 'Apply Import Draft' }));

    await waitFor(() => {
      expect(mockApplyProjectImportReview).toHaveBeenCalledWith('task-tracker');
    });

    expect(screen.getByText('Import draft applied and reconciled to canonical blueprint')).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Open Blueprint' }));

    expect(await screen.findByTestId('location-path')).toHaveTextContent('/projects/task-tracker/blueprint');
  });

  it('treats a missing review payload as no review card instead of a page failure', async () => {
    mockGetProjectImportReview.mockRejectedValueOnce(new MockApiError('Not found', 404));

    renderProjectSessions();

    await waitFor(() => {
      expect(mockListProjectSessions).toHaveBeenCalledWith('task-tracker');
    });

    expect(screen.queryByText('Import draft ready for project review')).not.toBeInTheDocument();
    expect(screen.getByText('Latest import draft is ready for review')).toBeInTheDocument();
    expect(screen.getByText('Imported planning brief')).toBeInTheDocument();
  });

  it('reuses the same review banner contract for local imports', async () => {
    mockGetProjectImportHistory.mockResolvedValueOnce({
      project: {
        id: 'proj-1',
        slug: 'task-tracker',
        name: 'Task Tracker',
        description: 'Import review workspace',
        owner_user_id: 'dev|local',
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:00:00Z',
        archived_at: null,
        legacy_scope_keys: [],
      },
      source_binding: {
        project_id: 'proj-1',
        provider: 'local',
        canonical_ref: '/tmp/recipes',
        default_branch: 'main',
        head_revision: 'deadbeef',
        local_root: '/tmp/recipes',
        managed_checkout: false,
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:01:00Z',
      },
      history: [
        {
          import_job: {
            id: 'job-local',
            project_id: 'proj-1',
            provider: 'local',
            requested_ref: '/tmp/recipes',
            status: 'review_pending',
            seed_session_id: 'seed-1',
            analysis_summary: 'Imported draft for Task Tracker from /tmp/recipes.',
            progress_message: 'Import draft ready. Review imported context in the seeded session.',
            error_message: null,
            created_at: '2026-03-20T00:00:00Z',
            updated_at: '2026-03-20T00:01:00Z',
          },
          source_metadata: {
            provider: 'local',
            canonical_ref: '/tmp/recipes',
            local_root: '/tmp/recipes',
            default_branch: 'main',
            head_revision: 'deadbeef',
          },
          discovered_node_count: 1,
        },
      ],
      diff_summary: null,
    });
    mockGetProjectImportState.mockResolvedValueOnce({
      project: {
        id: 'proj-1',
        slug: 'task-tracker',
        name: 'Task Tracker',
        description: 'Import review workspace',
        owner_user_id: 'dev|local',
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:00:00Z',
        archived_at: null,
        legacy_scope_keys: [],
      },
      import_job: {
        id: 'job-local',
        project_id: 'proj-1',
        provider: 'local',
        requested_ref: '/tmp/recipes',
        status: 'review_pending',
        seed_session_id: 'seed-1',
        analysis_summary: 'Imported draft for Task Tracker from /tmp/recipes.',
        progress_message: 'Import draft ready. Review imported context in the seeded session.',
        error_message: null,
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:01:00Z',
      },
      source_binding: {
        project_id: 'proj-1',
        provider: 'local',
        canonical_ref: '/tmp/recipes',
        default_branch: 'main',
        head_revision: 'deadbeef',
        local_root: '/tmp/recipes',
        managed_checkout: false,
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:01:00Z',
      },
      import_draft: null,
    });
    mockGetProjectImportReview.mockResolvedValueOnce({
      project: {
        id: 'proj-1',
        slug: 'task-tracker',
        name: 'Task Tracker',
        description: 'Import review workspace',
        owner_user_id: 'dev|local',
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:00:00Z',
        archived_at: null,
        legacy_scope_keys: [],
      },
      import_job: {
        id: 'job-local',
        project_id: 'proj-1',
        provider: 'local',
        requested_ref: '/tmp/recipes',
        status: 'review_pending',
        seed_session_id: 'seed-1',
        analysis_summary: 'Imported draft for Task Tracker from /tmp/recipes.',
        progress_message: 'Import draft ready. Review imported context in the seeded session.',
        error_message: null,
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:01:00Z',
      },
      source_binding: {
        project_id: 'proj-1',
        provider: 'local',
        canonical_ref: '/tmp/recipes',
        default_branch: 'main',
        head_revision: 'deadbeef',
        local_root: '/tmp/recipes',
        managed_checkout: false,
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:01:00Z',
      },
      import_draft: {
        job_id: 'job-local',
        project_id: 'proj-1',
        analysis_summary: 'Imported draft for Task Tracker from /tmp/recipes.',
        source_metadata: {
          provider: 'local',
          canonical_ref: '/tmp/recipes',
          local_root: '/tmp/recipes',
          default_branch: 'main',
          head_revision: 'deadbeef',
        },
        discovered_nodes: [{ id: 'comp-auth-a1' }],
        created_at: '2026-03-20T00:01:00Z',
        updated_at: '2026-03-20T00:01:00Z',
      },
    });

    renderProjectSessions();

    await waitFor(() => {
      expect(mockGetProjectImportReview).toHaveBeenCalledWith('task-tracker');
    });

    expect(screen.getByText('Import draft ready for project review')).toBeInTheDocument();
    expect(screen.getAllByText(/LOCAL source: \/tmp\/recipes/i).length).toBeGreaterThan(0);
    expect(screen.getByRole('button', { name: 'Apply Import Draft' })).toBeInTheDocument();
  });

  it('starts a project-level re-import and shows the queued state', async () => {
    const user = userEvent.setup();
    mockGetProjectImportState
      .mockResolvedValueOnce({
        project: {
          id: 'proj-1',
          slug: 'task-tracker',
          name: 'Task Tracker',
          description: 'Import review workspace',
          owner_user_id: 'dev|local',
          created_at: '2026-03-20T00:00:00Z',
          updated_at: '2026-03-20T00:00:00Z',
          archived_at: null,
          legacy_scope_keys: [],
        },
        import_job: {
          id: 'job-1',
          project_id: 'proj-1',
          provider: 'github',
          requested_ref: 'https://github.com/example/task-tracker',
          status: 'applied',
          seed_session_id: 'seed-1',
          analysis_summary: 'Imported draft for Task Tracker from GitHub.',
          progress_message: 'Import draft applied and reconciled against the canonical project blueprint.',
          error_message: null,
          created_at: '2026-03-20T00:00:00Z',
          updated_at: '2026-03-20T00:02:00Z',
        },
        source_binding: {
          project_id: 'proj-1',
          provider: 'github',
          canonical_ref: 'https://github.com/example/task-tracker',
          default_branch: 'main',
          head_revision: 'deadbeef',
          local_root: '/tmp/imports/task-tracker',
          managed_checkout: true,
          created_at: '2026-03-20T00:00:00Z',
          updated_at: '2026-03-20T00:02:00Z',
        },
        import_draft: null,
      })
      .mockResolvedValueOnce({
        project: {
          id: 'proj-1',
          slug: 'task-tracker',
          name: 'Task Tracker',
          description: 'Import review workspace',
          owner_user_id: 'dev|local',
          created_at: '2026-03-20T00:00:00Z',
          updated_at: '2026-03-20T00:00:00Z',
          archived_at: null,
          legacy_scope_keys: [],
        },
        import_job: {
          id: 'job-2',
          project_id: 'proj-1',
          provider: 'github',
          requested_ref: 'https://github.com/example/task-tracker',
          status: 'queued',
          seed_session_id: null,
          analysis_summary: null,
          progress_message: 'Re-import request queued',
          error_message: null,
          created_at: '2026-03-20T00:03:00Z',
          updated_at: '2026-03-20T00:03:00Z',
        },
        source_binding: {
          project_id: 'proj-1',
          provider: 'github',
          canonical_ref: 'https://github.com/example/task-tracker',
          default_branch: 'main',
          head_revision: 'deadbeef',
          local_root: '/tmp/imports/task-tracker',
          managed_checkout: true,
          created_at: '2026-03-20T00:00:00Z',
          updated_at: '2026-03-20T00:03:00Z',
        },
        import_draft: null,
      });

    renderProjectSessions();

    await waitFor(() => {
      expect(mockGetProjectImportState).toHaveBeenCalledWith('task-tracker');
    });

    await user.click(screen.getByRole('button', { name: 'Re-import' }));

    await waitFor(() => {
      expect(mockReimportProject).toHaveBeenCalledWith('task-tracker');
    });

    expect(await screen.findByText('Imported source is attached to this project')).toBeInTheDocument();
    expect(screen.getByText('Re-import request queued')).toBeInTheDocument();
  });

  it('restores an older applied import from project history', async () => {
    const user = userEvent.setup();
    mockGetProjectImportState.mockResolvedValueOnce({
      project: {
        id: 'proj-1',
        slug: 'task-tracker',
        name: 'Task Tracker',
        description: 'Import review workspace',
        owner_user_id: 'dev|local',
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:00:00Z',
        archived_at: null,
        legacy_scope_keys: [],
      },
      import_job: {
        id: 'job-1',
        project_id: 'proj-1',
        provider: 'github',
        requested_ref: 'https://github.com/example/task-tracker',
        status: 'applied',
        restored_from_job_id: null,
        seed_session_id: 'seed-1',
        analysis_summary: 'Imported draft for Task Tracker from GitHub.',
        progress_message: 'Import draft applied and reconciled against the canonical project blueprint.',
        error_message: null,
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:02:00Z',
      },
      source_binding: {
        project_id: 'proj-1',
        provider: 'github',
        canonical_ref: 'https://github.com/example/task-tracker',
        default_branch: 'main',
        head_revision: 'deadbeef',
        local_root: '/tmp/imports/task-tracker',
        managed_checkout: true,
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:02:00Z',
      },
      import_draft: null,
    });
    mockGetProjectImportReview.mockResolvedValueOnce({
      project: {
        id: 'proj-1',
        slug: 'task-tracker',
        name: 'Task Tracker',
        description: 'Import review workspace',
        owner_user_id: 'dev|local',
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:00:00Z',
        archived_at: null,
        legacy_scope_keys: [],
      },
      import_job: {
        id: 'job-1',
        project_id: 'proj-1',
        provider: 'github',
        requested_ref: 'https://github.com/example/task-tracker',
        status: 'applied',
        restored_from_job_id: null,
        seed_session_id: 'seed-1',
        analysis_summary: 'Imported draft for Task Tracker from GitHub.',
        progress_message: 'Import draft applied and reconciled against the canonical project blueprint.',
        error_message: null,
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:02:00Z',
      },
      source_binding: {
        project_id: 'proj-1',
        provider: 'github',
        canonical_ref: 'https://github.com/example/task-tracker',
        default_branch: 'main',
        head_revision: 'deadbeef',
        local_root: '/tmp/imports/task-tracker',
        managed_checkout: true,
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:02:00Z',
      },
      import_draft: null,
    });
    mockGetProjectImportHistory
      .mockResolvedValueOnce({
        project: {
          id: 'proj-1',
          slug: 'task-tracker',
          name: 'Task Tracker',
          description: 'Import review workspace',
          owner_user_id: 'dev|local',
          created_at: '2026-03-20T00:00:00Z',
          updated_at: '2026-03-20T00:00:00Z',
          archived_at: null,
          legacy_scope_keys: [],
        },
        source_binding: {
          project_id: 'proj-1',
          provider: 'github',
          canonical_ref: 'https://github.com/example/task-tracker',
          default_branch: 'main',
          head_revision: 'deadbeef',
          local_root: '/tmp/imports/task-tracker',
          managed_checkout: true,
          created_at: '2026-03-20T00:00:00Z',
          updated_at: '2026-03-20T00:02:00Z',
        },
        history: [
          {
            import_job: {
              id: 'job-1',
              project_id: 'proj-1',
              provider: 'github',
              requested_ref: 'https://github.com/example/task-tracker',
              status: 'applied',
              restored_from_job_id: null,
              seed_session_id: 'seed-1',
              analysis_summary: 'Imported draft for Task Tracker from GitHub.',
              progress_message: 'Import draft applied and reconciled against the canonical project blueprint.',
              error_message: null,
              created_at: '2026-03-20T00:00:00Z',
              updated_at: '2026-03-20T00:02:00Z',
            },
            source_metadata: {
              provider: 'github',
              canonical_ref: 'https://github.com/example/task-tracker',
              local_root: '/tmp/imports/task-tracker',
              default_branch: 'main',
              head_revision: 'deadbeef',
            },
            discovered_node_count: 2,
          },
          {
            import_job: {
              id: 'job-0',
              project_id: 'proj-1',
              provider: 'github',
              requested_ref: 'https://github.com/example/task-tracker',
              status: 'applied',
              restored_from_job_id: null,
              seed_session_id: 'seed-0',
              analysis_summary: 'Earlier import draft for Task Tracker from GitHub.',
              progress_message: 'Import draft applied and reconciled against the canonical project blueprint.',
              error_message: null,
              created_at: '2026-03-19T23:00:00Z',
              updated_at: '2026-03-19T23:10:00Z',
            },
            source_metadata: {
              provider: 'github',
              canonical_ref: 'https://github.com/example/task-tracker',
              local_root: '/tmp/imports/task-tracker',
              default_branch: 'main',
              head_revision: 'cafebabe',
            },
            discovered_node_count: 1,
          },
        ],
        diff_summary: null,
      })
      .mockResolvedValueOnce({
        project: {
          id: 'proj-1',
          slug: 'task-tracker',
          name: 'Task Tracker',
          description: 'Import review workspace',
          owner_user_id: 'dev|local',
          created_at: '2026-03-20T00:00:00Z',
          updated_at: '2026-03-20T00:00:00Z',
          archived_at: null,
          legacy_scope_keys: [],
        },
        source_binding: {
          project_id: 'proj-1',
          provider: 'github',
          canonical_ref: 'https://github.com/example/task-tracker',
          default_branch: 'main',
          head_revision: 'cafebabe',
          local_root: '/tmp/imports/task-tracker',
          managed_checkout: true,
          created_at: '2026-03-20T00:00:00Z',
          updated_at: '2026-03-20T00:05:00Z',
        },
        history: [
          {
            import_job: {
              id: 'job-restore',
              project_id: 'proj-1',
              provider: 'github',
              requested_ref: 'https://github.com/example/task-tracker',
              status: 'applied',
              restored_from_job_id: 'job-0',
              seed_session_id: null,
              analysis_summary: 'Earlier import draft for Task Tracker from GitHub.',
              progress_message: 'Historical import restored from job-0 into the canonical project blueprint.',
              error_message: null,
              created_at: '2026-03-20T00:05:00Z',
              updated_at: '2026-03-20T00:05:00Z',
            },
            source_metadata: {
              provider: 'github',
              canonical_ref: 'https://github.com/example/task-tracker',
              local_root: '/tmp/imports/task-tracker',
              default_branch: 'main',
              head_revision: 'cafebabe',
            },
            discovered_node_count: 1,
          },
          {
            import_job: {
              id: 'job-1',
              project_id: 'proj-1',
              provider: 'github',
              requested_ref: 'https://github.com/example/task-tracker',
              status: 'applied',
              restored_from_job_id: null,
              seed_session_id: 'seed-1',
              analysis_summary: 'Imported draft for Task Tracker from GitHub.',
              progress_message: 'Import draft applied and reconciled against the canonical project blueprint.',
              error_message: null,
              created_at: '2026-03-20T00:00:00Z',
              updated_at: '2026-03-20T00:02:00Z',
            },
            source_metadata: {
              provider: 'github',
              canonical_ref: 'https://github.com/example/task-tracker',
              local_root: '/tmp/imports/task-tracker',
              default_branch: 'main',
              head_revision: 'deadbeef',
            },
            discovered_node_count: 2,
          },
        ],
        diff_summary: null,
      });
    mockRestoreProjectImportHistoryEntry.mockResolvedValueOnce({
      project: {
        id: 'proj-1',
        slug: 'task-tracker',
        name: 'Task Tracker',
        description: 'Import review workspace',
        owner_user_id: 'dev|local',
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:00:00Z',
        archived_at: null,
        legacy_scope_keys: [],
      },
      import_job: {
        id: 'job-restore',
        project_id: 'proj-1',
        provider: 'github',
        requested_ref: 'https://github.com/example/task-tracker',
        status: 'applied',
        restored_from_job_id: 'job-0',
        seed_session_id: null,
        analysis_summary: 'Earlier import draft for Task Tracker from GitHub.',
        progress_message: 'Historical import restored from job-0 into the canonical project blueprint.',
        error_message: null,
        created_at: '2026-03-20T00:05:00Z',
        updated_at: '2026-03-20T00:05:00Z',
      },
      source_binding: {
        project_id: 'proj-1',
        provider: 'github',
        canonical_ref: 'https://github.com/example/task-tracker',
        default_branch: 'main',
        head_revision: 'cafebabe',
        local_root: '/tmp/imports/task-tracker',
        managed_checkout: true,
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:05:00Z',
      },
      import_draft: null,
    });

    renderProjectSessions();

    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Restore This Import' })).toBeInTheDocument();
    });

    await user.click(screen.getByRole('button', { name: 'Restore This Import' }));

    await waitFor(() => {
      expect(mockRestoreProjectImportHistoryEntry).toHaveBeenCalledWith('task-tracker', 'job-0');
    });

    expect(screen.getByText('Historical import restored to canonical blueprint')).toBeInTheDocument();
    expect(screen.getByText(/Restored from import job-0/i)).toBeInTheDocument();
  });

  it('reopens an older historical review draft into the current review slot', async () => {
    const user = userEvent.setup();
    mockGetProjectImportState.mockResolvedValueOnce({
      project: {
        id: 'proj-1',
        slug: 'task-tracker',
        name: 'Task Tracker',
        description: 'Import review workspace',
        owner_user_id: 'dev|local',
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:00:00Z',
        archived_at: null,
        legacy_scope_keys: [],
      },
      import_job: {
        id: 'job-1',
        project_id: 'proj-1',
        provider: 'github',
        requested_ref: 'https://github.com/example/task-tracker',
        status: 'applied',
        restored_from_job_id: null,
        seed_session_id: 'seed-1',
        analysis_summary: 'Imported draft for Task Tracker from GitHub.',
        progress_message: 'Import draft applied and reconciled against the canonical project blueprint.',
        error_message: null,
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:02:00Z',
      },
      source_binding: {
        project_id: 'proj-1',
        provider: 'github',
        canonical_ref: 'https://github.com/example/task-tracker',
        default_branch: 'main',
        head_revision: 'deadbeef',
        local_root: '/tmp/imports/task-tracker',
        managed_checkout: true,
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:02:00Z',
      },
      import_draft: null,
    });
    mockGetProjectImportReview.mockResolvedValueOnce({
      project: {
        id: 'proj-1',
        slug: 'task-tracker',
        name: 'Task Tracker',
        description: 'Import review workspace',
        owner_user_id: 'dev|local',
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:00:00Z',
        archived_at: null,
        legacy_scope_keys: [],
      },
      import_job: {
        id: 'job-1',
        project_id: 'proj-1',
        provider: 'github',
        requested_ref: 'https://github.com/example/task-tracker',
        status: 'applied',
        restored_from_job_id: null,
        seed_session_id: 'seed-1',
        analysis_summary: 'Imported draft for Task Tracker from GitHub.',
        progress_message: 'Import draft applied and reconciled against the canonical project blueprint.',
        error_message: null,
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:02:00Z',
      },
      source_binding: {
        project_id: 'proj-1',
        provider: 'github',
        canonical_ref: 'https://github.com/example/task-tracker',
        default_branch: 'main',
        head_revision: 'deadbeef',
        local_root: '/tmp/imports/task-tracker',
        managed_checkout: true,
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:02:00Z',
      },
      import_draft: null,
    });
    mockGetProjectImportHistory
      .mockResolvedValueOnce({
        project: {
          id: 'proj-1',
          slug: 'task-tracker',
          name: 'Task Tracker',
          description: 'Import review workspace',
          owner_user_id: 'dev|local',
          created_at: '2026-03-20T00:00:00Z',
          updated_at: '2026-03-20T00:00:00Z',
          archived_at: null,
          legacy_scope_keys: [],
        },
        source_binding: {
          project_id: 'proj-1',
          provider: 'github',
          canonical_ref: 'https://github.com/example/task-tracker',
          default_branch: 'main',
          head_revision: 'deadbeef',
          local_root: '/tmp/imports/task-tracker',
          managed_checkout: true,
          created_at: '2026-03-20T00:00:00Z',
          updated_at: '2026-03-20T00:02:00Z',
        },
        history: [
          {
            import_job: {
              id: 'job-1',
              project_id: 'proj-1',
              provider: 'github',
              requested_ref: 'https://github.com/example/task-tracker',
              status: 'applied',
              restored_from_job_id: null,
              seed_session_id: 'seed-1',
              analysis_summary: 'Imported draft for Task Tracker from GitHub.',
              progress_message: 'Import draft applied and reconciled against the canonical project blueprint.',
              error_message: null,
              created_at: '2026-03-20T00:00:00Z',
              updated_at: '2026-03-20T00:02:00Z',
            },
            source_metadata: {
              provider: 'github',
              canonical_ref: 'https://github.com/example/task-tracker',
              local_root: '/tmp/imports/task-tracker',
              default_branch: 'main',
              head_revision: 'deadbeef',
            },
            discovered_node_count: 2,
          },
          {
            import_job: {
              id: 'job-old-review',
              project_id: 'proj-1',
              provider: 'github',
              requested_ref: 'https://github.com/example/task-tracker',
              status: 'review_pending',
              restored_from_job_id: null,
              seed_session_id: 'seed-old',
              analysis_summary: 'Older draft for Task Tracker from GitHub.',
              progress_message: 'Import draft ready. Review imported context in the seeded session.',
              error_message: null,
              created_at: '2026-03-19T23:00:00Z',
              updated_at: '2026-03-19T23:10:00Z',
            },
            source_metadata: {
              provider: 'github',
              canonical_ref: 'https://github.com/example/task-tracker',
              local_root: '/tmp/imports/task-tracker',
              default_branch: 'main',
              head_revision: 'cafebabe',
            },
            discovered_node_count: 1,
          },
        ],
        diff_summary: null,
      })
      .mockResolvedValueOnce({
        project: {
          id: 'proj-1',
          slug: 'task-tracker',
          name: 'Task Tracker',
          description: 'Import review workspace',
          owner_user_id: 'dev|local',
          created_at: '2026-03-20T00:00:00Z',
          updated_at: '2026-03-20T00:00:00Z',
          archived_at: null,
          legacy_scope_keys: [],
        },
        source_binding: {
          project_id: 'proj-1',
          provider: 'github',
          canonical_ref: 'https://github.com/example/task-tracker',
          default_branch: 'main',
          head_revision: 'cafebabe',
          local_root: '/tmp/imports/task-tracker',
          managed_checkout: true,
          created_at: '2026-03-20T00:00:00Z',
          updated_at: '2026-03-20T00:05:00Z',
        },
        history: [
          {
            import_job: {
              id: 'job-restored-review',
              project_id: 'proj-1',
              provider: 'github',
              requested_ref: 'https://github.com/example/task-tracker',
              status: 'review_pending',
              restored_from_job_id: 'job-old-review',
              seed_session_id: 'seed-old',
              analysis_summary: 'Older draft for Task Tracker from GitHub.',
              progress_message: 'Historical review draft restored from import job-old-review. Review and apply when ready.',
              error_message: null,
              created_at: '2026-03-20T00:05:00Z',
              updated_at: '2026-03-20T00:05:00Z',
            },
            source_metadata: {
              provider: 'github',
              canonical_ref: 'https://github.com/example/task-tracker',
              local_root: '/tmp/imports/task-tracker',
              default_branch: 'main',
              head_revision: 'cafebabe',
            },
            discovered_node_count: 1,
          },
          {
            import_job: {
              id: 'job-1',
              project_id: 'proj-1',
              provider: 'github',
              requested_ref: 'https://github.com/example/task-tracker',
              status: 'applied',
              restored_from_job_id: null,
              seed_session_id: 'seed-1',
              analysis_summary: 'Imported draft for Task Tracker from GitHub.',
              progress_message: 'Import draft applied and reconciled against the canonical project blueprint.',
              error_message: null,
              created_at: '2026-03-20T00:00:00Z',
              updated_at: '2026-03-20T00:02:00Z',
            },
            source_metadata: {
              provider: 'github',
              canonical_ref: 'https://github.com/example/task-tracker',
              local_root: '/tmp/imports/task-tracker',
              default_branch: 'main',
              head_revision: 'deadbeef',
            },
            discovered_node_count: 2,
          },
        ],
        diff_summary: null,
      });
    mockRestoreProjectImportReviewDraft.mockResolvedValueOnce({
      project: {
        id: 'proj-1',
        slug: 'task-tracker',
        name: 'Task Tracker',
        description: 'Import review workspace',
        owner_user_id: 'dev|local',
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:00:00Z',
        archived_at: null,
        legacy_scope_keys: [],
      },
      import_job: {
        id: 'job-restored-review',
        project_id: 'proj-1',
        provider: 'github',
        requested_ref: 'https://github.com/example/task-tracker',
        status: 'review_pending',
        restored_from_job_id: 'job-old-review',
        seed_session_id: 'seed-old',
        analysis_summary: 'Older draft for Task Tracker from GitHub.',
        progress_message: 'Historical review draft restored from import job-old-review. Review and apply when ready.',
        error_message: null,
        created_at: '2026-03-20T00:05:00Z',
        updated_at: '2026-03-20T00:05:00Z',
      },
      source_binding: {
        project_id: 'proj-1',
        provider: 'github',
        canonical_ref: 'https://github.com/example/task-tracker',
        default_branch: 'main',
        head_revision: 'cafebabe',
        local_root: '/tmp/imports/task-tracker',
        managed_checkout: true,
        created_at: '2026-03-20T00:00:00Z',
        updated_at: '2026-03-20T00:05:00Z',
      },
      import_draft: {
        job_id: 'job-restored-review',
        project_id: 'proj-1',
        analysis_summary: 'Older draft for Task Tracker from GitHub.',
        source_metadata: {
          provider: 'github',
          canonical_ref: 'https://github.com/example/task-tracker',
          local_root: '/tmp/imports/task-tracker',
          default_branch: 'main',
          head_revision: 'cafebabe',
        },
        discovered_nodes: [{ id: 'comp-auth-a1' }],
        created_at: '2026-03-20T00:05:00Z',
        updated_at: '2026-03-20T00:05:00Z',
      },
    });

    renderProjectSessions();

    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Restore Draft For Review' })).toBeInTheDocument();
    });

    await user.click(screen.getByRole('button', { name: 'Restore Draft For Review' }));

    await waitFor(() => {
      expect(mockRestoreProjectImportReviewDraft).toHaveBeenCalledWith('task-tracker', 'job-old-review');
    });

    expect(screen.getByText('Historical draft restored for review')).toBeInTheDocument();
    expect(screen.getByText(/This historical draft was restored for review/i)).toBeInTheDocument();
  });
});
