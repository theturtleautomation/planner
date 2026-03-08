import { useCallback, useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import Layout from '../components/Layout.tsx';
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
  const [creatingProject, setCreatingProject] = useState(false);

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

  const createProject = useCallback(async () => {
    const name = window.prompt('Project name');
    if (name === null) return;
    const trimmed = name.trim();
    if (!trimmed) return;

    const description = window.prompt('Optional short description') ?? undefined;
    setCreatingProject(true);
    setError(null);

    try {
      const response = await api.createProject({
        name: trimmed,
        description: description?.trim() || undefined,
      });
      await loadProjects();
      void navigate(projectSessionsPath(response.project.slug));
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setCreatingProject(false);
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
      'legacy blueprint': '/blueprint',
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

  const quickActions: Array<{ label: string; onClick: () => void; primary?: boolean }> = [
    { label: 'Open Projects', onClick: () => { void navigate('/projects'); }, primary: true },
    { label: creatingProject ? 'Creating…' : 'New Project', onClick: () => { void createProject(); }, primary: true },
    { label: 'Knowledge Library', onClick: () => { void navigate('/knowledge'); }, primary: true },
    { label: 'Events', onClick: () => { void navigate('/events'); }, primary: true },
    { label: 'Admin', onClick: () => { void navigate('/admin'); }, primary: true },
    { label: 'Open Sessions', onClick: () => { void navigate('/sessions'); } },
    { label: 'Legacy Blueprint', onClick: () => { void navigate('/blueprint'); } },
    { label: 'Discovery', onClick: () => { void navigate('/discovery'); } },
  ];

  return (
    <Layout>
      <div
        style={{
          flex: 1,
          overflow: 'auto',
          padding: '32px 24px',
          maxWidth: '1080px',
          margin: '0 auto',
          width: '100%',
          display: 'flex',
          flexDirection: 'column',
          gap: '20px',
        }}
      >
        <section
          style={{
            background: 'var(--color-surface)',
            border: '1px solid var(--color-border)',
            borderRadius: '10px',
            padding: '18px 18px 16px',
            display: 'flex',
            flexDirection: 'column',
            gap: '12px',
          }}
        >
          <div style={{ display: 'flex', justifyContent: 'space-between', gap: '12px', flexWrap: 'wrap' }}>
            <div>
              <h1 style={{ margin: 0, fontSize: '22px', color: 'var(--color-text)' }}>Home</h1>
              <p style={{ margin: '6px 0 0', color: 'var(--color-text-muted)', fontSize: '13px' }}>
                Route quickly to projects and core planning surfaces.
              </p>
            </div>
            {!AUTH0_ENABLED && (
              <span
                style={{
                  alignSelf: 'flex-start',
                  border: '1px solid rgba(255,215,0,0.35)',
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
                background: 'var(--color-bg)',
                border: '1px solid var(--color-border)',
                borderRadius: '6px',
                color: 'var(--color-text)',
                padding: '10px 12px',
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
                className={action.primary ? 'btn btn-outline' : 'btn'}
                onClick={action.onClick}
                disabled={creatingProject && action.label.startsWith('New Project')}
                style={{
                  opacity: creatingProject && action.label.startsWith('New Project') ? 0.75 : 1,
                }}
              >
                {action.label}
              </button>
            ))}
          </div>
        </section>

        <section
          style={{
            background: 'var(--color-surface)',
            border: '1px solid var(--color-border)',
            borderRadius: '10px',
            padding: '16px',
            display: 'flex',
            flexDirection: 'column',
            gap: '12px',
          }}
        >
          <div style={{ display: 'flex', justifyContent: 'space-between', gap: '12px', flexWrap: 'wrap' }}>
            <div>
              <h2 style={{ margin: 0, fontSize: '16px', color: 'var(--color-text)' }}>Recent Projects</h2>
              <p style={{ margin: '4px 0 0', color: 'var(--color-text-muted)', fontSize: '12px' }}>
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
            <div
              style={{
                border: '1px dashed var(--color-border)',
                borderRadius: '8px',
                padding: '18px',
                color: 'var(--color-text-muted)',
                display: 'flex',
                flexDirection: 'column',
                gap: '8px',
              }}
            >
              <span style={{ color: 'var(--color-text)', fontWeight: 600 }}>No projects yet</span>
              <span>Sessions now live inside projects. Create a project to begin.</span>
              <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
                <button className="btn btn-primary" onClick={() => { void createProject(); }}>
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
                    border: '1px solid var(--color-border)',
                    borderRadius: '8px',
                    padding: '12px',
                    background: 'var(--color-surface-2)',
                    display: 'flex',
                    flexDirection: 'column',
                    gap: '8px',
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
    </Layout>
  );
}
