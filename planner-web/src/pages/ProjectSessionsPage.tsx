import { useCallback, useEffect, useMemo, useState } from 'react';
import { NavLink, useNavigate, useParams } from 'react-router-dom';
import Layout from '../components/Layout.tsx';
import { createApiClient } from '../api/client.ts';
import { useGetAccessToken } from '../auth/useAuthenticatedFetch.ts';
import type { Project, SessionSummary } from '../types.ts';

function formatRelativeTime(iso: string): string {
  const parsed = new Date(iso);
  if (Number.isNaN(parsed.getTime())) return iso;

  const diffMs = Date.now() - parsed.getTime();
  if (diffMs < 60_000) return 'just now';

  const minutes = Math.floor(diffMs / 60_000);
  if (minutes < 60) return `${minutes}m ago`;

  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;

  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

function sessionTitle(session: SessionSummary): string {
  const title = session.title?.trim();
  if (title) return title;

  const brief = session.project_description?.trim();
  if (brief) {
    const line = brief.replace(/\s+/g, ' ').trim();
    return line.length > 72 ? `${line.slice(0, 72)}…` : line;
  }

  return `Session ${session.id.slice(0, 8)}`;
}

function phaseLabel(phase: SessionSummary['intake_phase']): string {
  if (phase === 'pipeline_running') return 'building';
  return phase;
}

export default function ProjectSessionsPage() {
  const navigate = useNavigate();
  const params = useParams<{ projectSlug: string }>();
  const projectSlug = params.projectSlug ?? '';

  const getToken = useGetAccessToken();
  const api = useMemo(() => createApiClient(getToken), [getToken]);

  const [project, setProject] = useState<Project | null>(null);
  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadData = useCallback(async () => {
    if (!projectSlug) {
      setError('Missing project slug.');
      return;
    }

    setLoading(true);
    setError(null);
    try {
      const [projectResponse, sessionsResponse] = await Promise.all([
        api.getProject(projectSlug),
        api.listProjectSessions(projectSlug),
      ]);
      setProject(projectResponse.project);
      setSessions([...sessionsResponse.sessions].sort((left, right) => (
        new Date(right.last_activity_at).getTime() - new Date(left.last_activity_at).getTime()
      )));
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [api, projectSlug]);

  useEffect(() => {
    void loadData();
  }, [loadData]);

  const projectPath = `/projects/${encodeURIComponent(projectSlug)}`;

  const tabs = [
    { label: 'Sessions', to: `${projectPath}/sessions` },
    { label: 'Blueprint', to: `${projectPath}/blueprint` },
    { label: 'Knowledge', to: `${projectPath}/knowledge` },
    { label: 'Events', to: `${projectPath}/events` },
  ];

  return (
    <Layout>
      <div
        style={{
          flex: 1,
          overflow: 'auto',
          padding: '30px 24px',
          maxWidth: '1040px',
          margin: '0 auto',
          width: '100%',
          display: 'flex',
          flexDirection: 'column',
          gap: '14px',
        }}
      >
        <header
          style={{
            borderBottom: '1px solid var(--color-border)',
            paddingBottom: '12px',
            display: 'flex',
            alignItems: 'flex-end',
            justifyContent: 'space-between',
            gap: '12px',
            flexWrap: 'wrap',
          }}
        >
          <div>
            <h1 style={{ margin: 0, fontSize: '22px', color: 'var(--color-text)' }}>
              {project?.name ?? 'Project Sessions'}
            </h1>
            <p style={{ margin: '6px 0 0', color: 'var(--color-text-muted)', fontSize: '13px' }}>
              {project?.description?.trim() || 'Project-local sessions and planning workflow.'}
            </p>
          </div>
          <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
            <button className="btn btn-outline" onClick={() => { void navigate('/projects'); }}>
              Back to Projects
            </button>
            <button
              className="btn btn-primary"
              onClick={() => { void navigate(`/session/new?project=${encodeURIComponent(projectSlug)}`); }}
            >
              New Project Session
            </button>
          </div>
        </header>

        <nav style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }} aria-label="Project sections">
          {tabs.map((tab) => (
            <NavLink
              key={tab.to}
              to={tab.to}
              className={({ isActive }) => `btn${isActive ? ' btn-outline' : ''}`}
              style={({ isActive }) => ({
                textDecoration: 'none',
                borderColor: isActive ? 'var(--color-primary)' : undefined,
                color: isActive ? 'var(--color-primary)' : undefined,
              })}
            >
              {tab.label}
            </NavLink>
          ))}
        </nav>

        {loading && <div style={{ color: 'var(--color-text-muted)' }}>Loading project sessions…</div>}

        {!loading && error && (
          <div style={{ color: 'var(--color-error)', fontSize: '13px' }}>
            Failed to load project sessions: {error}
          </div>
        )}

        {!loading && !error && sessions.length === 0 && (
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
            <span style={{ color: 'var(--color-text)', fontWeight: 600 }}>No sessions in this project yet</span>
            <span>Create the first project session to begin intake and pipeline work.</span>
            <div>
              <button
                className="btn btn-primary"
                onClick={() => { void navigate(`/session/new?project=${encodeURIComponent(projectSlug)}`); }}
              >
                Start Project Session
              </button>
            </div>
          </div>
        )}

        {!loading && !error && sessions.length > 0 && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
            {sessions.map((session) => (
              <article
                key={session.id}
                style={{
                  border: '1px solid var(--color-border)',
                  borderRadius: '8px',
                  background: 'var(--color-surface)',
                  padding: '12px',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  gap: '10px',
                  flexWrap: 'wrap',
                }}
              >
                <div style={{ minWidth: 0, display: 'flex', flexDirection: 'column', gap: '4px' }}>
                  <span style={{ color: 'var(--color-text)', fontWeight: 700 }}>{sessionTitle(session)}</span>
                  <span style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>
                    {phaseLabel(session.intake_phase)} · {formatRelativeTime(session.last_activity_at)}
                  </span>
                  {session.project_description?.trim() && (
                    <span style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>
                      {session.project_description.length > 120
                        ? `${session.project_description.slice(0, 120)}…`
                        : session.project_description}
                    </span>
                  )}
                </div>
                <button
                  className="btn btn-outline"
                  onClick={() => { void navigate(`/session/${session.id}`); }}
                >
                  Open Session
                </button>
              </article>
            ))}
          </div>
        )}
      </div>
    </Layout>
  );
}
