import { useCallback, useEffect, useMemo, useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import Layout from '../components/Layout.tsx';
import CreateProjectModal from '../components/CreateProjectModal.tsx';
import ImportProjectModal from '../components/ImportProjectModal.tsx';
import { ApiError, createApiClient } from '../api/client.ts';
import { useGetAccessToken } from '../auth/useAuthenticatedFetch.ts';
import type {
  ImportProvider,
  Project,
  ProjectImportConflictResponse,
  ProjectImportResponse,
} from '../types.ts';

function formatDate(iso: string): string {
  const parsed = new Date(iso);
  if (Number.isNaN(parsed.getTime())) return iso;
  return parsed.toLocaleString([], {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

function projectSessionsPath(slug: string): string {
  return `/projects/${encodeURIComponent(slug)}/sessions`;
}

function sessionPath(id: string): string {
  return `/session/${encodeURIComponent(id)}`;
}

export default function ProjectsPage() {
  const navigate = useNavigate();
  const getToken = useGetAccessToken();
  const api = useMemo(() => createApiClient(getToken), [getToken]);

  const [searchParams, setSearchParams] = useSearchParams();
  const [projects, setProjects] = useState<Project[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [createModalOpen, setCreateModalOpen] = useState(false);
  const [importModalOpen, setImportModalOpen] = useState(false);
  const [archiveMutationProjectId, setArchiveMutationProjectId] = useState<string | null>(null);
  const [deleteMutationProjectId, setDeleteMutationProjectId] = useState<string | null>(null);
  const [latestImport, setLatestImport] = useState<ProjectImportResponse | null>(null);

  const query = searchParams.get('query') ?? '';
  const showArchived = searchParams.get('show_archived') === 'true';

  const loadProjects = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const response = await api.listProjects({ includeArchived: showArchived });
      const sorted = [...response.projects].sort((left, right) => (
        new Date(right.updated_at).getTime() - new Date(left.updated_at).getTime()
      ));
      setProjects(sorted);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [api, showArchived]);

  useEffect(() => {
    void loadProjects();
  }, [loadProjects]);

  const filtered = useMemo(() => {
    const visible = showArchived ? projects : projects.filter((project) => !project.archived_at);
    const normalized = query.trim().toLowerCase();
    if (!normalized) return visible;

    return visible.filter((project) => {
      const name = project.name.toLowerCase();
      const slug = project.slug.toLowerCase();
      const description = project.description?.toLowerCase() ?? '';
      return name.includes(normalized) || slug.includes(normalized) || description.includes(normalized);
    });
  }, [projects, query, showArchived]);
  const archivedCount = useMemo(
    () => projects.filter((project) => Boolean(project.archived_at)).length,
    [projects],
  );

  const handleCreateProject = useCallback(async (name: string, description?: string) => {
    try {
      const response = await api.createProject({ name, description });
      await loadProjects();
      void navigate(projectSessionsPath(response.project.slug));
    } catch (err) {
      throw err; // Let the modal handle the error display
    }
  }, [api, loadProjects, navigate]);

  const handleCreateImport = useCallback(async (provider: ImportProvider, sourceRef: string) => {
    setError(null);
    try {
      const response = await api.createProjectImport({ provider, sourceRef });
      setLatestImport(response as ProjectImportResponse);
      await loadProjects();
    } catch (err) {
      if (err instanceof ApiError && err.status === 409) {
        const details = err.details as ProjectImportConflictResponse | undefined;
        if (details?.project?.slug) {
          void navigate(projectSessionsPath(details.project.slug));
          return;
        }
      }
      throw err;
    }
  }, [api, loadProjects, navigate]);

  useEffect(() => {
    if (!latestImport) return undefined;
    if (
      latestImport.import_job.status === 'review_pending'
      || latestImport.import_job.status === 'applied'
      || latestImport.import_job.status === 'failed'
    ) {
      return undefined;
    }

    let cancelled = false;
    let timer: number | undefined;

    const refresh = async () => {
      try {
        const response = await api.getProjectImport(latestImport.import_job.id);
        if (cancelled) return;
        setLatestImport(response);
        if (
          response.import_job.status === 'queued'
          || response.import_job.status === 'cloning'
          || response.import_job.status === 'analyzing'
        ) {
          timer = window.setTimeout(refresh, 400);
        }
      } catch (err) {
        if (cancelled) return;
        setError(err instanceof Error ? err.message : String(err));
      }
    };

    timer = window.setTimeout(refresh, 0);
    return () => {
      cancelled = true;
      if (timer) {
        window.clearTimeout(timer);
      }
    };
  }, [api, latestImport]);

  const handleArchiveToggle = useCallback(async (project: Project, archived: boolean) => {
    setArchiveMutationProjectId(project.id);
    setError(null);
    try {
      await api.updateProject(project.slug, { archived });
      await loadProjects();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setArchiveMutationProjectId(null);
    }
  }, [api, loadProjects]);

  const handleDeleteProject = useCallback(async (project: Project) => {
    const confirmed = window.confirm(
      `Delete "${project.name}" permanently?\n\nThis will stop any active sessions, remove this project's sessions and owned knowledge, and preserve shared knowledge by unlinking it from this project. This action cannot be undone.`,
    );
    if (!confirmed) return;

    setDeleteMutationProjectId(project.id);
    setError(null);
    try {
      await api.deleteProject(project.slug);
      await loadProjects();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setDeleteMutationProjectId(null);
    }
  }, [api, loadProjects]);

  const latestImportMessage = useMemo(() => {
    if (!latestImport) return null;
    const { import_job: job } = latestImport;
    if (job.provider === 'local') {
      switch (job.status) {
        case 'queued':
          return job.progress_message ?? 'Local import is queued.';
        case 'analyzing':
          return job.progress_message ?? 'Local import is analyzing the validated source root and preparing a seeded planning session.';
        case 'review_pending':
          return job.analysis_summary
            ?? job.progress_message
            ?? 'Import draft is ready. Open the seeded session to review imported context.';
        case 'applied':
          return 'Import draft was applied to the canonical project blueprint.';
        case 'failed':
          return job.error_message ?? 'Local import failed.';
        default:
          return null;
      }
    }

    switch (job.status) {
      case 'queued':
        return 'GitHub import is queued.';
      case 'cloning':
        return job.progress_message ?? 'GitHub import is cloning the default branch into managed storage.';
      case 'analyzing':
        return job.progress_message ?? 'GitHub import is analyzing the checkout and preparing a seeded planning session.';
      case 'review_pending':
        return job.analysis_summary
          ?? job.progress_message
          ?? 'Import draft is ready. Open the seeded session to review imported context.';
      case 'applied':
        return 'Import draft was applied to the canonical project blueprint.';
      case 'failed':
        return job.error_message ?? 'GitHub import failed.';
      default:
        return null;
    }
  }, [latestImport]);

  return (
    <Layout>
      <div className="command-page" style={{ maxWidth: '1100px' }}>
        <header className="command-page-header">
          <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', maxWidth: '38rem' }}>
            <span className="page-kicker">Project directory</span>
            <h1 className="display-heading" style={{ margin: 0 }}>Projects</h1>
            <p className="section-copy" style={{ margin: 0 }}>
              Scan projects as working objects, then open the project workspace when you are ready to move into sessions, knowledge, blueprint, or events.
            </p>
          </div>

          <div className="command-page-header-actions">
            <button className="btn btn-outline" onClick={() => setImportModalOpen(true)}>
              Import Existing Project
            </button>
            <button className="btn btn-primary" onClick={() => setCreateModalOpen(true)}>
              New Project
            </button>
          </div>
        </header>

        {latestImport && (
          <div className="command-surface-subtle">
            <div style={{ color: 'var(--color-text)', fontWeight: 700 }}>
              Import queued for {latestImport.project.name}
            </div>
            <div style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>
              {latestImportMessage}
            </div>
            <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
              {latestImport.import_job.status === 'review_pending' && latestImport.import_job.seed_session_id && (
                <button
                  className="btn btn-primary"
                  onClick={() => { void navigate(sessionPath(latestImport.import_job.seed_session_id!)); }}
                >
                  Open Seeded Session
                </button>
              )}
              {(
                latestImport.import_job.status === 'review_pending'
                || latestImport.import_job.status === 'applied'
                || latestImport.import_job.status === 'failed'
              ) && (
                <button
                  className="btn btn-outline"
                  onClick={() => { void navigate(projectSessionsPath(latestImport.project.slug)); }}
                >
                  Open Project
                </button>
              )}
              <button className="btn" onClick={() => setLatestImport(null)}>
                Dismiss
              </button>
            </div>
          </div>
        )}

        <section className="command-surface-subtle">
          <div className="command-surface-header">
            <div className="command-surface-copy">
              <span className="page-kicker">Directory focus</span>
              <h2 className="section-heading" style={{ margin: 0 }}>Scan the project queue from one line of sight.</h2>
              <p className="section-copy" style={{ margin: 0 }}>
                Creation and import stay route-level. The list below carries freshness, archive state, and next move without falling back to card clutter.
              </p>
            </div>
            <div className="directory-row-meta" style={{ alignSelf: 'flex-start' }}>
              <span className="directory-row-highlight" data-tone="primary">
                Visible {filtered.length}
              </span>
              <span className="directory-row-highlight" data-tone={archivedCount > 0 ? 'warning' : 'default'}>
                Archived {archivedCount}
              </span>
              {showArchived && (
                <span className="directory-row-highlight" data-tone="warning">
                  Archived mode
                </span>
              )}
            </div>
          </div>

          <div className="command-input-row">
            <input
              value={query}
              onChange={(event) => {
                const next = event.target.value;
                const nextParams = new URLSearchParams(searchParams);
                if (next.trim()) {
                  nextParams.set('query', next);
                } else {
                  nextParams.delete('query');
                }
                setSearchParams(nextParams, { replace: true });
              }}
              placeholder="Search projects by name, slug, or description"
              aria-label="Search projects"
              className="command-input"
            />
            {query && (
              <button
                className="btn"
                onClick={() => {
                  const nextParams = new URLSearchParams(searchParams);
                  nextParams.delete('query');
                  setSearchParams(nextParams, { replace: true });
                }}
              >
                Clear
              </button>
            )}
          </div>

          <div className="command-pill-matrix" style={{ justifyContent: 'space-between', alignItems: 'center' }}>
            <label className="command-tab active" style={{ cursor: 'pointer' }}>
              <input
                type="checkbox"
                checked={showArchived}
                onChange={(event) => {
                  const nextParams = new URLSearchParams(searchParams);
                  if (event.target.checked) {
                    nextParams.set('show_archived', 'true');
                  } else {
                    nextParams.delete('show_archived');
                  }
                  setSearchParams(nextParams, { replace: true });
                }}
                style={{ marginRight: '8px' }}
              />
              Show archived
            </label>
            <div className="utility-note" style={{ margin: 0 }}>
              Route-level controls stay here so the list can stay focused on the next project move.
            </div>
          </div>
        </section>

        {loading && <div style={{ color: 'var(--color-text-muted)' }}>Loading projects…</div>}

        {!loading && error && (
          <div style={{ color: 'var(--color-error)', fontSize: '13px' }}>
            Failed to load projects: {error}
          </div>
        )}

        {!loading && filtered.length === 0 && (
          <div className="empty-state-card">
            <span className="empty-state-kicker">
              {projects.length === 0 ? 'New workspace' : 'No match'}
            </span>
            <span className="empty-state-title">
              {projects.length === 0 ? 'Open a project workspace.' : 'No projects match this query.'}
            </span>
            <span className="empty-state-body">
              {projects.length === 0
                ? 'Use the New Project button above to start project-scoped sessions and planning work.'
                : 'Try a broader search or clear the current query.'}
            </span>
            {projects.length === 0 && !query && (
              <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
                <button className="btn btn-primary" onClick={() => setCreateModalOpen(true)}>
                  New Project
                </button>
                <button className="btn btn-outline" onClick={() => setImportModalOpen(true)}>
                  Import Repository
                </button>
              </div>
            )}
            {query && (
              <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
                <button
                  className="btn btn-outline"
                  onClick={() => {
                    const nextParams = new URLSearchParams(searchParams);
                    nextParams.delete('query');
                    setSearchParams(nextParams, { replace: true });
                  }}
                >
                  Reset Search
                </button>
              </div>
            )}
          </div>
        )}

        {!loading && filtered.length > 0 && (
          <div className="directory-list">
            {filtered.map((project) => {
              const isArchiving = archiveMutationProjectId === project.id;
              const isDeleting = deleteMutationProjectId === project.id;
              const isMutating = isArchiving || isDeleting;
              return (
                <article key={project.id} className="directory-row">
                  <div className="directory-row-main">
                    <div className="directory-row-heading">
                      <div style={{ minWidth: 0 }}>
                        <div className="directory-row-title">{project.name}</div>
                        <div className="directory-row-code">{project.slug}</div>
                      </div>
                      <div className="directory-row-meta">
                        {project.archived_at && (
                          <span className="directory-row-highlight" data-tone="warning">
                            Archived
                          </span>
                        )}
                        <span className="directory-row-highlight" data-tone="primary">
                          Updated {formatDate(project.updated_at)}
                        </span>
                      </div>
                    </div>
                    <div className="directory-row-copy">
                      {project.description?.trim() || 'No description yet.'}
                    </div>
                  </div>
                  <div className="directory-row-facts">
                    <div className="directory-row-meta">
                      <span className="utility-pill">Sessions</span>
                      <span className="utility-pill">Knowledge</span>
                      <span className="utility-pill">Blueprint</span>
                      <span className="utility-pill">Events</span>
                    </div>
                    <div className="section-copy" style={{ margin: 0 }}>
                      Open the project workspace first, then branch into attached route surfaces from project context.
                    </div>
                  </div>
                  <div className="directory-row-actions">
                    <button className="btn btn-primary" onClick={() => { void navigate(projectSessionsPath(project.slug)); }}>
                      Open
                    </button>
                    {!project.archived_at && (
                      <button
                        className="btn btn-outline"
                        onClick={() => { void handleArchiveToggle(project, true); }}
                        disabled={isMutating}
                      >
                        {isArchiving ? 'Archiving…' : 'Archive'}
                      </button>
                    )}
                    {project.archived_at && (
                      <button
                        className="btn btn-outline"
                        onClick={() => { void handleArchiveToggle(project, false); }}
                        disabled={isMutating}
                      >
                        {isArchiving ? 'Restoring…' : 'Unarchive'}
                      </button>
                    )}
                    <button
                      className="btn btn-outline"
                      onClick={() => { void handleDeleteProject(project); }}
                      disabled={isMutating}
                      style={{
                        color: 'var(--color-error)',
                        background: 'rgba(209, 99, 167, 0.08)',
                        boxShadow: 'inset 0 0 0 1px rgba(209, 99, 167, 0.18)',
                      }}
                    >
                      {isDeleting ? 'Deleting…' : 'Delete'}
                    </button>
                  </div>
                </article>
              );
            })}
          </div>
        )}
      </div>
      <CreateProjectModal
        isOpen={createModalOpen}
        onClose={() => setCreateModalOpen(false)}
        onCreate={handleCreateProject}
      />
      <ImportProjectModal
        isOpen={importModalOpen}
        onClose={() => setImportModalOpen(false)}
        onImport={handleCreateImport}
      />
    </Layout>
  );
}
