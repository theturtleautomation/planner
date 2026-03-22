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
      <div
        style={{
          flex: 1,
          overflow: 'auto',
          padding: '40px 24px 56px',
          maxWidth: '1080px',
          margin: '0 auto',
          width: '100%',
          display: 'flex',
          flexDirection: 'column',
          gap: '28px',
        }}
      >
        <section
          style={{
            background: 'var(--color-surface-offset)',
            borderRadius: '18px',
            padding: '28px',
            display: 'flex',
            flexDirection: 'column',
            gap: '18px',
          }}
        >
          <div style={{ display: 'flex', justifyContent: 'space-between', gap: '12px', flexWrap: 'wrap' }}>
            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', maxWidth: '36rem' }}>
              <span className="page-kicker">Command center</span>
              <h1 className="display-heading" style={{ margin: 0 }}>Home</h1>
              <p className="section-copy" style={{ margin: 0 }}>
                Route quickly to active projects, planning sessions, and the core operating surfaces around them.
              </p>
            </div>
            {!AUTH0_ENABLED && (
              <span
                style={{
                  alignSelf: 'flex-start',
                  background: 'rgba(209, 153, 0, 0.12)',
                  color: 'var(--color-gold)',
                  borderRadius: '999px',
                  padding: '5px 10px',
                  fontSize: '10px',
                  fontWeight: 700,
                  letterSpacing: '0.05em',
                  textTransform: 'uppercase',
                }}
              >
                dev mode
              </span>
            )}
          </div>

          <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
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
              style={{
                flex: '1 1 360px',
                minWidth: '220px',
                background: 'var(--color-surface-2)',
                boxShadow: 'inset 0 0 0 1px var(--color-ghost-border)',
                border: 'none',
                borderRadius: '10px',
                color: 'var(--color-text)',
                padding: '12px 14px',
                fontSize: '13px',
              }}
            />
            <button className="btn btn-primary" onClick={routePrompt}>
              Go
            </button>
          </div>

          <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
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
        </section>

        <section
          style={{
            background: 'var(--color-surface)',
            borderRadius: '18px',
            padding: '20px',
            display: 'flex',
            flexDirection: 'column',
            gap: '16px',
            boxShadow: 'var(--shadow-md)',
          }}
        >
          <div style={{ display: 'flex', justifyContent: 'space-between', gap: '12px', flexWrap: 'wrap' }}>
            <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
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
            <div style={{ display: 'grid', gap: '10px', gridTemplateColumns: 'repeat(auto-fit, minmax(240px, 1fr))' }}>
              {recentProjects.map((project) => (
                <article
                  key={project.id}
                  style={{
                    borderRadius: '14px',
                    padding: '14px',
                    background: 'var(--color-surface-offset)',
                    display: 'flex',
                    flexDirection: 'column',
                    gap: '10px',
                  }}
                >
                  <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', gap: '8px' }}>
                    <div style={{ minWidth: 0 }}>
                      <div style={{ color: 'var(--color-text)', fontWeight: 700, fontSize: '14px' }}>{project.name}</div>
                      <div style={{ color: 'var(--color-text-muted)', fontSize: '11px', fontFamily: 'monospace' }}>
                        {project.slug}
                      </div>
                    </div>
                    <button className="btn btn-outline" onClick={() => { void navigate(projectSessionsPath(project.slug)); }}>
                      Open
                    </button>
                  </div>
                  <div style={{ color: 'var(--color-text-muted)', fontSize: '12px', lineHeight: 1.5 }}>
                    {project.description?.trim() || 'No description yet.'}
                  </div>
                  <div style={{ color: 'var(--color-text-muted)', fontSize: '11px' }}>
                    {formatRelativeTime(project.updated_at)}
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
