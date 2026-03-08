import { useCallback, useEffect, useMemo, useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import Layout from '../components/Layout.tsx';
import { createApiClient } from '../api/client.ts';
import { useGetAccessToken } from '../auth/useAuthenticatedFetch.ts';
import type { Project } from '../types.ts';

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

export default function ProjectsPage() {
  const navigate = useNavigate();
  const getToken = useGetAccessToken();
  const api = useMemo(() => createApiClient(getToken), [getToken]);

  const [searchParams, setSearchParams] = useSearchParams();
  const [projects, setProjects] = useState<Project[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);

  const query = searchParams.get('query') ?? '';

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

  const filtered = useMemo(() => {
    const normalized = query.trim().toLowerCase();
    if (!normalized) return projects;

    return projects.filter((project) => {
      const name = project.name.toLowerCase();
      const slug = project.slug.toLowerCase();
      const description = project.description?.toLowerCase() ?? '';
      return name.includes(normalized) || slug.includes(normalized) || description.includes(normalized);
    });
  }, [projects, query]);

  const handleCreateProject = useCallback(async () => {
    const name = window.prompt('Project name');
    if (name === null) return;
    const trimmed = name.trim();
    if (!trimmed) return;

    const description = window.prompt('Optional short description') ?? undefined;
    setCreating(true);
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
      setCreating(false);
    }
  }, [api, loadProjects, navigate]);

  return (
    <Layout>
      <div
        style={{
          flex: 1,
          overflow: 'auto',
          padding: '30px 24px',
          maxWidth: '1100px',
          margin: '0 auto',
          width: '100%',
          display: 'flex',
          flexDirection: 'column',
          gap: '16px',
        }}
      >
        <header
          style={{
            display: 'flex',
            alignItems: 'flex-end',
            justifyContent: 'space-between',
            gap: '12px',
            flexWrap: 'wrap',
            borderBottom: '1px solid var(--color-border)',
            paddingBottom: '12px',
          }}
        >
          <div>
            <h1 style={{ margin: 0, color: 'var(--color-text)', fontSize: '24px' }}>Projects</h1>
            <p style={{ margin: '6px 0 0', color: 'var(--color-text-muted)', fontSize: '13px' }}>
              Project directory for sessions, blueprint, knowledge, and events.
            </p>
          </div>

          <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
            <button className="btn btn-outline" onClick={() => { void navigate('/sessions'); }}>
              Open Sessions
            </button>
            <button className="btn btn-primary" onClick={() => { void handleCreateProject(); }} disabled={creating}>
              {creating ? 'Creating…' : 'New Project'}
            </button>
          </div>
        </header>

        <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
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
            style={{
              flex: '1 1 280px',
              minWidth: '220px',
              background: 'var(--color-bg)',
              border: '1px solid var(--color-border)',
              borderRadius: '6px',
              color: 'var(--color-text)',
              padding: '10px 12px',
              fontSize: '13px',
            }}
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

        {loading && <div style={{ color: 'var(--color-text-muted)' }}>Loading projects…</div>}

        {!loading && error && (
          <div style={{ color: 'var(--color-error)', fontSize: '13px' }}>
            Failed to load projects: {error}
          </div>
        )}

        {!loading && !error && filtered.length === 0 && (
          <div
            style={{
              border: '1px dashed var(--color-border)',
              borderRadius: '8px',
              padding: '18px',
              color: 'var(--color-text-muted)',
              fontSize: '13px',
              display: 'flex',
              flexDirection: 'column',
              gap: '8px',
            }}
          >
            <span style={{ color: 'var(--color-text)', fontWeight: 600 }}>
              {projects.length === 0 ? 'No projects yet' : 'No projects match this query'}
            </span>
            <span>
              {projects.length === 0
                ? 'Create a project to start project-scoped sessions and planning work.'
                : 'Try a broader search or clear the current query.'}
            </span>
            <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
              <button className="btn btn-primary" onClick={() => { void handleCreateProject(); }} disabled={creating}>
                {creating ? 'Creating…' : 'Create Project'}
              </button>
              {query && (
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
              )}
            </div>
          </div>
        )}

        {!loading && !error && filtered.length > 0 && (
          <div style={{ display: 'grid', gap: '12px', gridTemplateColumns: 'repeat(auto-fit, minmax(260px, 1fr))' }}>
            {filtered.map((project) => (
              <article
                key={project.id}
                style={{
                  border: '1px solid var(--color-border)',
                  borderRadius: '10px',
                  padding: '14px',
                  background: 'var(--color-surface)',
                  display: 'flex',
                  flexDirection: 'column',
                  gap: '10px',
                }}
              >
                <div style={{ display: 'flex', justifyContent: 'space-between', gap: '8px' }}>
                  <div style={{ minWidth: 0 }}>
                    <div style={{ color: 'var(--color-text)', fontSize: '15px', fontWeight: 700 }}>{project.name}</div>
                    <div style={{ color: 'var(--color-primary)', fontSize: '11px', fontFamily: 'monospace' }}>
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
                  Updated {formatDate(project.updated_at)}
                </div>

                <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
                  <button className="btn" onClick={() => { void navigate(`/projects/${encodeURIComponent(project.slug)}/knowledge`); }}>
                    Knowledge
                  </button>
                  <button className="btn" onClick={() => { void navigate(`/projects/${encodeURIComponent(project.slug)}/blueprint`); }}>
                    Blueprint
                  </button>
                  <button className="btn" onClick={() => { void navigate(`/projects/${encodeURIComponent(project.slug)}/events`); }}>
                    Events
                  </button>
                </div>
              </article>
            ))}
          </div>
        )}
      </div>
    </Layout>
  );
}
