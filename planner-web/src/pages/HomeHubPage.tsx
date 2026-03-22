import { useCallback, useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import Layout from '../components/Layout.tsx';
import CreateProjectModal from '../components/CreateProjectModal.tsx';
import { createApiClient } from '../api/client.ts';
import { useGetAccessToken } from '../auth/useAuthenticatedFetch.ts';
import { AUTH0_ENABLED } from '../config.ts';
import type { Project } from '../types.ts';

function formatRelativeTime(iso?: string | null): string {
  if (!iso) return 'No recent activity';
  const parsed = new Date(iso);
  if (Number.isNaN(parsed.getTime())) return 'No recent activity';

  const diffMs = Date.now() - parsed.getTime();
  if (diffMs < 60_000) return 'Updated just now';

  const minutes = Math.floor(diffMs / 60_000);
  if (minutes < 60) return `Updated ${minutes}m ago`;

  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `Updated ${hours}h ago`;

  const days = Math.floor(hours / 24);
  return `Updated ${days}d ago`;
}

function projectSessionsPath(slug: string): string {
  return `/projects/${encodeURIComponent(slug)}/sessions`;
}

export default function HomeHubPage() {
  const navigate = useNavigate();
  const getToken = useGetAccessToken();
  const api = useMemo(() => createApiClient(getToken), [getToken]);

  const [projects, setProjects] = useState<Project[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [prompt, setPrompt] = useState('');
  const [createModalOpen, setCreateModalOpen] = useState(false);

  const loadProjects = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const response = await api.listProjects();
      const sorted = [...response.projects].sort((left, right) => (
        new Date(right.updated_at).getTime() - new Date(left.updated_at).getTime()
      ));
      setProjects(sorted);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [api]);

  useEffect(() => {
    void loadProjects();
  }, [loadProjects]);

  const recentProjects = useMemo(() => projects.slice(0, 6), [projects]);
  const activeProjectCount = useMemo(
    () => projects.filter((project) => !project.archived_at).length,
    [projects],
  );
  const latestProjectUpdate = recentProjects[0]?.updated_at ?? null;

  const handleCreateProject = useCallback(async (name: string, description?: string) => {
    try {
      const response = await api.createProject({
        name,
        description,
      });
      await loadProjects();
      void navigate(projectSessionsPath(response.project.slug));
    } catch (err) {
      throw err; // Let the modal handle it
    }
  }, [api, loadProjects, navigate]);

  const routePrompt = useCallback(() => {
    const raw = prompt.trim();
    if (!raw) {
      void navigate('/projects');
      return;
    }

    const normalized = raw.toLowerCase();
    const exactIntentRoutes: Record<string, string> = {
      'open projects': '/projects',
      projects: '/projects',
      'new project': '/projects',
      'start planning': '/projects',
      knowledge: '/knowledge',
      events: '/events',
      admin: '/admin',
      sessions: '/sessions',
      'open sessions': '/sessions',
      discovery: '/discovery',
      blueprint: '/blueprint',
    };

    const directRoute = exactIntentRoutes[normalized];
    if (directRoute) {
      void navigate(directRoute);
      return;
    }

    const matchingProject = projects.find((project) => {
      const slug = project.slug.toLowerCase();
      const name = project.name.toLowerCase();
      return slug === normalized || name === normalized || name.includes(normalized);
    });

    if (matchingProject) {
      void navigate(projectSessionsPath(matchingProject.slug));
      return;
    }

    void navigate(`/projects?query=${encodeURIComponent(raw)}`);
  }, [navigate, projects, prompt]);

  const quickActions: Array<{ label: string; onClick: () => void; variant?: 'primary' | 'outline' | 'default' }> = [
    { label: 'New Project', onClick: () => { setCreateModalOpen(true); }, variant: 'primary' },
    { label: 'Open Projects', onClick: () => { void navigate('/projects'); }, variant: 'outline' },
    { label: 'Knowledge Library', onClick: () => { void navigate('/knowledge'); }, variant: 'outline' },
    { label: 'Events', onClick: () => { void navigate('/events'); }, variant: 'default' },
    { label: 'Admin', onClick: () => { void navigate('/admin'); }, variant: 'default' },
    { label: 'Open Sessions', onClick: () => { void navigate('/sessions'); } },
    { label: 'Blueprint', onClick: () => { void navigate('/blueprint'); } },
    { label: 'Discovery', onClick: () => { void navigate('/discovery'); } },
  ];

  return (
    <Layout>
      <div className="command-page">
        <section className="command-hero-grid">
          <div className="command-surface-strong">
            <div className="command-surface-header">
              <div className="command-surface-copy" style={{ maxWidth: '38rem' }}>
                <span className="page-kicker">Command center</span>
                <h1 className="display-heading" style={{ margin: 0 }}>Home</h1>
                <p className="section-copy" style={{ margin: 0 }}>
                  Route quickly to active projects, planning sessions, and the core operating surfaces around them.
                </p>
              </div>
              {!AUTH0_ENABLED && (
                <span className="directory-row-highlight" data-tone="warning">
                  dev mode
                </span>
              )}
            </div>

            <div className="command-input-row">
              <input
                value={prompt}
                onChange={(event) => setPrompt(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === 'Enter') {
                    event.preventDefault();
                    routePrompt();
                  }
                }}
                placeholder="Try: open projects, new project, knowledge, or a project name"
                aria-label="Home intent prompt"
                className="command-input"
              />
              <button className="btn btn-primary" onClick={routePrompt}>
                Go
              </button>
            </div>

            <div className="command-pill-matrix">
              {quickActions.map((action) => (
                <button
                  key={action.label}
                  className={`btn${action.variant === 'primary' ? ' btn-primary' : action.variant === 'outline' ? ' btn-outline' : ''}`}
                  onClick={action.onClick}
                >
                  {action.label}
                </button>
              ))}
            </div>
          </div>

          <aside className="command-surface-soft">
            <div className="command-surface-copy">
              <span className="page-kicker">Operating picture</span>
              <h2 className="section-heading" style={{ margin: 0 }}>Project-first routing stays central.</h2>
              <p className="section-copy" style={{ margin: 0 }}>
                Start from project context, then branch into sessions, knowledge, blueprint, and events from a bounded workspace.
              </p>
            </div>
            <div className="command-info-grid">
              <div className="command-info-cell">
                <span className="command-info-label">Active projects</span>
                <span className="command-info-value">{activeProjectCount}</span>
                <span className="command-info-copy">Available project spaces in the working directory.</span>
              </div>
              <div className="command-info-cell">
                <span className="command-info-label">Recent update</span>
                <span className="command-info-value" style={{ fontSize: '1.15rem', lineHeight: 1.15 }}>
                  {formatRelativeTime(latestProjectUpdate)}
                </span>
                <span className="command-info-copy">The freshest project update in the current workspace.</span>
              </div>
            </div>
            <div className="utility-note" style={{ margin: 0 }}>
              Use the prompt for direct routing and the project list below for the active working directory.
            </div>
          </aside>
        </section>

        <section className="command-surface-soft">
          <div className="command-surface-header">
            <div className="command-surface-copy">
              <h2 className="section-heading" style={{ margin: 0 }}>Recent Projects</h2>
              <p className="section-copy" style={{ margin: 0 }}>
                Project sessions, blueprint, knowledge, and events all start here.
              </p>
            </div>
            <button className="btn btn-outline" onClick={() => { void navigate('/projects'); }}>
              Open Directory
            </button>
          </div>

          {loading && <div style={{ color: 'var(--color-text-muted)', fontSize: '13px' }}>Loading projects…</div>}

          {!loading && error && (
            <div style={{ color: 'var(--color-error)', fontSize: '13px' }}>
              Failed to load projects: {error}
            </div>
          )}

          {!loading && !error && recentProjects.length === 0 && (
            <div className="empty-state-card">
              <span className="empty-state-kicker">Start here</span>
              <span className="empty-state-title">Create the first project shell.</span>
              <span className="empty-state-body">
                Sessions now live inside projects. Start a project space, then branch into sessions, blueprint, knowledge, and events from there.
              </span>
              <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
                <button className="btn btn-primary" onClick={() => { setCreateModalOpen(true); }}>
                  Create your first project
                </button>
                <button className="btn btn-outline" onClick={() => { void navigate('/knowledge'); }}>
                  Open Knowledge Library
                </button>
              </div>
            </div>
          )}

          {!loading && !error && recentProjects.length > 0 && (
            <div className="directory-list">
              {recentProjects.map((project) => (
                <article key={project.id} className="directory-row">
                  <div className="directory-row-main">
                    <div className="directory-row-heading">
                      <div style={{ minWidth: 0 }}>
                        <div className="directory-row-title">{project.name}</div>
                        <div className="directory-row-code">{project.slug}</div>
                      </div>
                      <span className="directory-row-highlight" data-tone="primary">
                        {formatRelativeTime(project.updated_at)}
                      </span>
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
                      Updated route anchor for project-scoped planning work.
                    </div>
                  </div>
                  <div className="directory-row-actions">
                    <button className="btn btn-outline" onClick={() => { void navigate(projectSessionsPath(project.slug)); }}>
                      Open
                    </button>
                  </div>
                </article>
              ))}
            </div>
          )}
        </section>
      </div>
      <CreateProjectModal
        isOpen={createModalOpen}
        onClose={() => setCreateModalOpen(false)}
        onCreate={handleCreateProject}
      />
    </Layout>
  );
}
