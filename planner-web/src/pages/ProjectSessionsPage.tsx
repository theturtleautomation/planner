import { useCallback, useEffect, useMemo, useState } from 'react';
import { NavLink, useNavigate, useParams } from 'react-router-dom';
import Layout from '../components/Layout.tsx';
import { ApiError, createApiClient } from '../api/client.ts';
import { useGetAccessToken } from '../auth/useAuthenticatedFetch.ts';
import type {
  Project,
  ProjectImportHistoryComparisonResponse,
  ProjectImportHistoryPairComparisonResponse,
  ProjectImportHistoryResponse,
  ProjectImportResponse,
  SessionSummary,
} from '../types.ts';

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

function importStatusLabel(status: string): string {
  switch (status) {
    case 'review_pending':
      return 'review pending';
    default:
      return status.replace(/_/g, ' ');
  }
}

export default function ProjectSessionsPage() {
  const navigate = useNavigate();
  const params = useParams<{ projectSlug: string }>();
  const projectSlug = params.projectSlug ?? '';

  const getToken = useGetAccessToken();
  const api = useMemo(() => createApiClient(getToken), [getToken]);

  const [project, setProject] = useState<Project | null>(null);
  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  const [importState, setImportState] = useState<ProjectImportResponse | null>(null);
  const [importReview, setImportReview] = useState<ProjectImportResponse | null>(null);
  const [importHistory, setImportHistory] = useState<ProjectImportHistoryResponse | null>(null);
  const [importComparison, setImportComparison] = useState<ProjectImportHistoryComparisonResponse | null>(null);
  const [importHistoryBaselineJobId, setImportHistoryBaselineJobId] = useState<string | null>(null);
  const [importPairComparison, setImportPairComparison] = useState<ProjectImportHistoryPairComparisonResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [applyPending, setApplyPending] = useState(false);
  const [applyError, setApplyError] = useState<string | null>(null);
  const [selectionPendingNodeId, setSelectionPendingNodeId] = useState<string | null>(null);
  const [reimportPending, setReimportPending] = useState(false);
  const [reimportError, setReimportError] = useState<string | null>(null);
  const [restorePendingJobId, setRestorePendingJobId] = useState<string | null>(null);
  const [restoreError, setRestoreError] = useState<string | null>(null);

  const loadImportHistory = useCallback(async () => {
    if (!projectSlug) return null;
    try {
      return await api.getProjectImportHistory(projectSlug);
    } catch (err) {
      if (err instanceof ApiError && err.status === 404) {
        return null;
      }
      throw err;
    }
  }, [api, projectSlug]);

  const loadData = useCallback(async () => {
    if (!projectSlug) {
      setError('Missing project slug.');
      setLoading(false);
      return;
    }

    setLoading(true);
    setError(null);
    try {
      const [
        projectResponse,
        sessionsResponse,
        importStateResponse,
        importReviewResponse,
        importHistoryResponse,
      ] = await Promise.all([
        api.getProject(projectSlug),
        api.listProjectSessions(projectSlug),
        api.getProjectImportState(projectSlug).catch((err: unknown) => {
          if (err instanceof ApiError && err.status === 404) {
            return null;
          }
          throw err;
        }),
        api.getProjectImportReview(projectSlug).catch((err: unknown) => {
          if (err instanceof ApiError && err.status === 404) {
            return null;
          }
          throw err;
        }),
        loadImportHistory(),
      ]);
      setProject(projectResponse.project);
      setSessions([...sessionsResponse.sessions].sort((left, right) => (
        new Date(right.last_activity_at).getTime() - new Date(left.last_activity_at).getTime()
      )));
      setImportState(importStateResponse);
      setImportReview(importReviewResponse);
      setImportHistory(importHistoryResponse);
      setImportComparison(null);
      setImportHistoryBaselineJobId(null);
      setImportPairComparison(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [api, loadImportHistory, projectSlug]);

  useEffect(() => {
    void loadData();
  }, [loadData]);

  useEffect(() => {
    if (!importState) return undefined;
    if (
      importState.import_job.status === 'review_pending'
      || importState.import_job.status === 'applied'
      || importState.import_job.status === 'failed'
    ) {
      return undefined;
    }

    let cancelled = false;
    let timer: number | undefined;

    const refresh = async () => {
      try {
        const response = await api.getProjectImportState(projectSlug);
        if (cancelled) return;
        setImportState(response);
        if (response.import_job.status === 'review_pending' || response.import_job.status === 'applied') {
          setImportReview(response);
        }
        if (
          response.import_job.status === 'review_pending'
          || response.import_job.status === 'applied'
          || response.import_job.status === 'failed'
        ) {
          const historyResponse = await loadImportHistory();
          if (!cancelled) {
            setImportHistory(historyResponse);
          }
        }
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
  }, [api, importState, loadImportHistory, projectSlug]);

  const projectPath = `/projects/${encodeURIComponent(projectSlug)}`;
  const blueprintPath = `${projectPath}/blueprint`;

  const tabs = [
    { label: 'Sessions', to: `${projectPath}/sessions` },
    { label: 'Blueprint', to: `${projectPath}/blueprint` },
    { label: 'Knowledge', to: `${projectPath}/knowledge` },
    { label: 'Events', to: `${projectPath}/events` },
  ];

  const handleApplyImportDraft = useCallback(async () => {
    if (!projectSlug) return;
    setApplyPending(true);
    setApplyError(null);
    try {
      const response = await api.applyProjectImportReview(projectSlug);
      setImportState(response);
      setImportReview(response);
      setImportHistory(await loadImportHistory());
      setImportComparison(null);
      setImportHistoryBaselineJobId(null);
      setImportPairComparison(null);
    } catch (err) {
      setApplyError(err instanceof Error ? err.message : String(err));
    } finally {
      setApplyPending(false);
    }
  }, [api, loadImportHistory, projectSlug]);

  const handleSetImportNodeIncluded = useCallback(async (nodeId: string, included: boolean) => {
    if (!projectSlug) return;
    setSelectionPendingNodeId(nodeId);
    setApplyError(null);
    try {
      const response = await api.updateProjectImportReviewSelection(projectSlug, { nodeId, included });
      setImportState(response);
      setImportReview(response);
      setImportComparison(null);
      setImportPairComparison(null);
    } catch (err) {
      setApplyError(err instanceof Error ? err.message : String(err));
    } finally {
      setSelectionPendingNodeId(null);
    }
  }, [api, projectSlug]);

  const handleCompareImportHistoryEntry = useCallback(async (jobId: string) => {
    if (!projectSlug) return;
    setRestoreError(null);
    try {
      const response = await api.getProjectImportHistoryComparison(projectSlug, jobId);
      setImportComparison(response);
      setImportHistoryBaselineJobId(null);
      setImportPairComparison(null);
    } catch (err) {
      setRestoreError(err instanceof Error ? err.message : String(err));
    }
  }, [api, projectSlug]);

  const handleSelectImportHistoryBaseline = useCallback((jobId: string) => {
    setRestoreError(null);
    setImportHistoryBaselineJobId(jobId);
    setImportComparison(null);
    setImportPairComparison(null);
  }, []);

  const handleCompareImportHistoryToBaseline = useCallback(async (jobId: string) => {
    if (!projectSlug || !importHistoryBaselineJobId) return;
    setRestoreError(null);
    try {
      const response = await api.getProjectImportHistoryPairComparison(
        projectSlug,
        importHistoryBaselineJobId,
        jobId,
      );
      setImportPairComparison(response);
      setImportComparison(null);
    } catch (err) {
      setRestoreError(err instanceof Error ? err.message : String(err));
    }
  }, [api, importHistoryBaselineJobId, projectSlug]);

  const handleReimport = useCallback(async () => {
    if (!projectSlug) return;
    setReimportPending(true);
    setReimportError(null);
    try {
      const response = await api.reimportProject(projectSlug);
      setImportState(response);
      setImportHistory(await loadImportHistory());
      setImportComparison(null);
      setImportHistoryBaselineJobId(null);
      setImportPairComparison(null);
    } catch (err) {
      setReimportError(err instanceof Error ? err.message : String(err));
    } finally {
      setReimportPending(false);
    }
  }, [api, loadImportHistory, projectSlug]);

  const handleRestoreImportHistoryEntry = useCallback(async (jobId: string) => {
    if (!projectSlug) return;
    setRestorePendingJobId(jobId);
    setRestoreError(null);
    try {
      const response = await api.restoreProjectImportHistoryEntry(projectSlug, jobId);
      setImportState(response);
      setImportReview(response);
      setImportHistory(await loadImportHistory());
      setImportComparison(null);
      setImportHistoryBaselineJobId(null);
      setImportPairComparison(null);
    } catch (err) {
      setRestoreError(err instanceof Error ? err.message : String(err));
    } finally {
      setRestorePendingJobId(null);
    }
  }, [api, loadImportHistory, projectSlug]);

  const handleRestoreImportHistoryEntryForReview = useCallback(async (jobId: string) => {
    if (!projectSlug) return;
    setRestorePendingJobId(jobId);
    setRestoreError(null);
    try {
      const response = await api.restoreProjectImportHistoryEntryForReview(projectSlug, jobId);
      setImportState(response);
      setImportReview(response);
      setImportHistory(await loadImportHistory());
      setImportComparison(null);
      setImportHistoryBaselineJobId(null);
      setImportPairComparison(null);
    } catch (err) {
      setRestoreError(err instanceof Error ? err.message : String(err));
    } finally {
      setRestorePendingJobId(null);
    }
  }, [api, loadImportHistory, projectSlug]);

  const handleRestoreImportReviewDraft = useCallback(async (jobId: string) => {
    if (!projectSlug) return;
    setRestorePendingJobId(jobId);
    setRestoreError(null);
    try {
      const response = await api.restoreProjectImportReviewDraft(projectSlug, jobId);
      setImportState(response);
      setImportReview(response);
      setImportHistory(await loadImportHistory());
      setImportComparison(null);
      setImportHistoryBaselineJobId(null);
      setImportPairComparison(null);
    } catch (err) {
      setRestoreError(err instanceof Error ? err.message : String(err));
    } finally {
      setRestorePendingJobId(null);
    }
  }, [api, loadImportHistory, projectSlug]);

  const importDraftCount = importReview?.import_draft?.discovered_nodes.length ?? 0;
  const importSource = importReview?.import_draft?.source_metadata ?? null;
  const importStatus = importReview?.import_job.status ?? null;
  const importReviewSelection = importReview?.import_review_selection ?? null;
  const importReviewNodes = importReview?.review_nodes ?? [];
  const includedDraftCount = importReviewSelection?.included_node_count ?? importDraftCount;
  const excludedDraftCount = importReviewSelection?.excluded_node_count ?? 0;
  const importHeadline = importStatus === 'applied'
    ? (importReview?.import_job.restored_from_job_id
      ? 'Historical import restored to canonical blueprint'
      : 'Import draft applied and reconciled to canonical blueprint')
    : importReview?.import_job.restored_from_job_id
      ? 'Historical draft restored for review'
      : 'Import draft ready for project review';
  const importDetails = importReview?.import_job.analysis_summary
    ?? importReview?.import_job.progress_message
    ?? null;
  const importReviewNote = importStatus === 'review_pending'
    ? (importReview?.import_job.restored_from_job_id
      ? 'This historical draft was restored for review. Applying it later will reconcile import-owned project blueprint state.'
      : 'Applying this draft will reconcile import-owned project blueprint state with the latest import result.')
    : null;
  const importStateStatus = importState?.import_job.status ?? null;
  const importStateSource = importState?.source_binding ?? null;
  const importStateHeadline = importStateStatus === 'failed'
    ? 'Latest import attempt failed'
    : importStateStatus === 'review_pending'
      ? 'Latest import draft is ready for review'
      : importStateStatus === 'applied'
        ? (importState?.import_job.restored_from_job_id
          ? 'Historical import was restored'
          : 'Latest import draft was applied')
        : 'Imported source is attached to this project';
  const importStateDetails = importState?.import_job.analysis_summary
    ?? importState?.import_job.progress_message
    ?? importState?.import_job.error_message
    ?? null;
  const importStateBusy = importStateStatus === 'queued'
    || importStateStatus === 'cloning'
    || importStateStatus === 'analyzing';
  const importHistoryEntries = importHistory?.history ?? [];
  const importDiffSummary = importHistory?.diff_summary ?? null;
  const selectedHistoryComparison = importComparison?.diff_summary ?? null;
  const selectedPairComparison = importPairComparison?.diff_summary ?? null;
  const restoreBlockedByPendingReview = importStateStatus === 'review_pending';
  const selectedHistoryComparisonNotes = [
    importComparison?.current_import_job_uses_selection_filter
      ? 'Current import comparison uses selected nodes from saved merge controls.'
      : null,
    importComparison?.selected_entry_uses_selection_filter
      ? 'Historical entry comparison uses selected nodes from saved merge controls.'
      : null,
  ].filter(Boolean) as string[];
  const selectedPairComparisonNotes = [
    importPairComparison?.baseline_entry_uses_selection_filter
      ? 'Baseline entry comparison uses selected nodes from saved merge controls.'
      : null,
    importPairComparison?.compared_entry_uses_selection_filter
      ? 'Compared entry comparison uses selected nodes from saved merge controls.'
      : null,
  ].filter(Boolean) as string[];

  return (
    <Layout>
      <div
        style={{
          flex: 1,
          overflow: 'auto',
          padding: '48px 24px 72px',
          maxWidth: '1040px',
          margin: '0 auto',
          width: '100%',
          display: 'flex',
          flexDirection: 'column',
          gap: '36px',
        }}
      >
        <header
          style={{
            background: 'var(--color-surface-offset)',
            borderRadius: '18px',
            padding: '32px',
            display: 'flex',
            alignItems: 'flex-end',
            justifyContent: 'space-between',
            gap: '12px',
            flexWrap: 'wrap',
            boxShadow: 'var(--shadow-md)',
          }}
        >
          <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', maxWidth: '38rem' }}>
            <span className="page-kicker">Project sessions</span>
            <h1 className="display-heading" style={{ margin: 0 }}>
              {project?.name ?? 'Project Sessions'}
            </h1>
            <p className="section-copy" style={{ margin: 0 }}>
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
                color: isActive ? 'var(--color-primary)' : undefined,
                background: isActive ? 'var(--color-surface-2)' : undefined,
                boxShadow: isActive ? 'inset 0 0 0 1px var(--color-ghost-border)' : undefined,
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

        {!loading && !error && importReview && (
          <>
            {importState && (
              <section
                style={{
                  borderRadius: '16px',
                  background: 'var(--color-surface)',
                  padding: '18px',
                  display: 'flex',
                  flexDirection: 'column',
                  gap: '10px',
                  boxShadow: 'var(--shadow-md)',
                }}
              >
                <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                  <span style={{ color: 'var(--color-text)', fontWeight: 700 }}>{importStateHeadline}</span>
                  {importStateDetails && (
                    <span style={{ color: 'var(--color-text-muted)', fontSize: '13px' }}>
                      {importStateDetails}
                    </span>
                  )}
                </div>

                {importStateSource && (
                  <div style={{ color: 'var(--color-text-muted)', fontSize: '12px', display: 'flex', gap: '12px', flexWrap: 'wrap' }}>
                    <span>{importStateSource.provider.toUpperCase()} source: {importStateSource.canonical_ref}</span>
                    {importStateSource.default_branch && <span>Branch: {importStateSource.default_branch}</span>}
                    {importStateSource.head_revision && <span>Revision: {importStateSource.head_revision.slice(0, 8)}</span>}
                  </div>
                )}

                {reimportError && (
                  <div style={{ color: 'var(--color-error)', fontSize: '12px' }}>
                    Failed to re-import project: {reimportError}
                  </div>
                )}

                {restoreError && (
                  <div style={{ color: 'var(--color-error)', fontSize: '12px' }}>
                    Failed to restore import history: {restoreError}
                  </div>
                )}

                <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
                  <button
                    className="btn btn-outline"
                    onClick={() => { void handleReimport(); }}
                    disabled={reimportPending || importStateBusy}
                  >
                    {reimportPending || importStateBusy ? 'Re-importing…' : 'Re-import'}
                  </button>
                  {importState.import_job.seed_session_id && (
                    <button
                      className="btn btn-outline"
                      onClick={() => { void navigate(`/session/${encodeURIComponent(importState.import_job.seed_session_id!)}`); }}
                    >
                      Open Latest Seeded Session
                    </button>
                  )}
                </div>
              </section>
            )}

          <section
            style={{
              borderRadius: '16px',
              background: 'var(--color-surface)',
              padding: '18px',
              display: 'flex',
              flexDirection: 'column',
              gap: '10px',
              boxShadow: 'var(--shadow-md)',
            }}
          >
            <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
              <span style={{ color: 'var(--color-text)', fontWeight: 700 }}>{importHeadline}</span>
              {importDetails && (
                <span style={{ color: 'var(--color-text-muted)', fontSize: '13px' }}>
                  {importDetails}
                </span>
              )}
              {importReviewNote && (
                <span style={{ color: 'var(--color-text-muted)', fontSize: '13px' }}>
                  {importReviewNote}
                </span>
              )}
            </div>

            {importSource && (
              <div style={{ color: 'var(--color-text-muted)', fontSize: '12px', display: 'flex', gap: '12px', flexWrap: 'wrap' }}>
                <span>{importSource.provider.toUpperCase()} source: {importSource.canonical_ref}</span>
                {importSource.default_branch && <span>Branch: {importSource.default_branch}</span>}
                {importSource.head_revision && <span>Revision: {importSource.head_revision.slice(0, 8)}</span>}
                <span>Draft records: {importDraftCount}</span>
                <span>Included: {includedDraftCount}</span>
                {excludedDraftCount > 0 && <span>Excluded: {excludedDraftCount}</span>}
              </div>
            )}

            {applyError && (
              <div style={{ color: 'var(--color-error)', fontSize: '12px' }}>
                Failed to apply import draft: {applyError}
              </div>
            )}

            <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
              {importReview.import_job.seed_session_id && (
                <button
                  className="btn btn-outline"
                  onClick={() => { void navigate(`/session/${encodeURIComponent(importReview.import_job.seed_session_id!)}`); }}
                >
                  Open Seeded Session
                </button>
              )}
              {importStatus === 'review_pending' && (
                <button
                  className="btn btn-primary"
                  onClick={() => { void handleApplyImportDraft(); }}
                  disabled={applyPending}
                >
                  {applyPending ? 'Applying Import Draft…' : 'Apply Import Draft'}
                </button>
              )}
              {importStatus === 'applied' && (
                <button
                  className="btn btn-primary"
                  onClick={() => { void navigate(blueprintPath); }}
                >
                  Open Blueprint
                </button>
              )}
            </div>

            {importStatus === 'review_pending' && importReviewNodes.length > 0 && (
              <div
                style={{
                  background: 'var(--color-surface-offset)',
                  borderRadius: '14px',
                  padding: '14px',
                  display: 'flex',
                  flexDirection: 'column',
                  gap: '8px',
                }}
              >
                <span style={{ color: 'var(--color-text)', fontWeight: 600 }}>
                  Merge Controls
                </span>
                <span style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>
                  Exclude discovered nodes you do not want promoted when this import draft is applied.
                </span>
                <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                  {importReviewNodes.map((node) => (
                    <div
                      key={node.node_id}
                      style={{
                        background: 'var(--color-surface-2)',
                        borderRadius: '12px',
                        padding: '12px',
                        display: 'flex',
                        justifyContent: 'space-between',
                        gap: '10px',
                        alignItems: 'center',
                        flexWrap: 'wrap',
                      }}
                    >
                      <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                        <span style={{ color: 'var(--color-text)', fontWeight: 600 }}>
                          {node.node_name}
                        </span>
                        <span style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>
                          {node.node_type} · {node.included ? 'included' : 'excluded'}
                        </span>
                      </div>
                      <button
                        className="btn btn-outline"
                        onClick={() => { void handleSetImportNodeIncluded(node.node_id, !node.included); }}
                        disabled={selectionPendingNodeId === node.node_id}
                      >
                        {selectionPendingNodeId === node.node_id
                          ? (node.included ? 'Excluding…' : 'Including…')
                          : (node.included ? 'Exclude From Apply' : 'Include In Apply')}
                      </button>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </section>
          </>
        )}

        {!loading && !error && !importReview && importState && (
          <section
            style={{
              borderRadius: '16px',
              background: 'var(--color-surface)',
              padding: '18px',
              display: 'flex',
              flexDirection: 'column',
              gap: '10px',
              boxShadow: 'var(--shadow-md)',
            }}
          >
            <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
              <span style={{ color: 'var(--color-text)', fontWeight: 700 }}>{importStateHeadline}</span>
              {importStateDetails && (
                <span style={{ color: 'var(--color-text-muted)', fontSize: '13px' }}>
                  {importStateDetails}
                </span>
              )}
            </div>

            {importStateSource && (
              <div style={{ color: 'var(--color-text-muted)', fontSize: '12px', display: 'flex', gap: '12px', flexWrap: 'wrap' }}>
                <span>{importStateSource.provider.toUpperCase()} source: {importStateSource.canonical_ref}</span>
                {importStateSource.default_branch && <span>Branch: {importStateSource.default_branch}</span>}
                {importStateSource.head_revision && <span>Revision: {importStateSource.head_revision.slice(0, 8)}</span>}
              </div>
            )}

            {reimportError && (
              <div style={{ color: 'var(--color-error)', fontSize: '12px' }}>
                Failed to re-import project: {reimportError}
              </div>
            )}

            {restoreError && (
              <div style={{ color: 'var(--color-error)', fontSize: '12px' }}>
                Failed to restore import history: {restoreError}
              </div>
            )}

            <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
              <button
                className="btn btn-outline"
                onClick={() => { void handleReimport(); }}
                disabled={reimportPending || importStateBusy}
              >
                {reimportPending || importStateBusy ? 'Re-importing…' : 'Re-import'}
              </button>
              {importState.import_job.seed_session_id && (
                <button
                  className="btn btn-outline"
                  onClick={() => { void navigate(`/session/${encodeURIComponent(importState.import_job.seed_session_id!)}`); }}
                >
                  Open Latest Seeded Session
                </button>
              )}
            </div>
          </section>
        )}

        {!loading && !error && importHistory && importHistoryEntries.length > 0 && (
          <section
            style={{
              borderRadius: '16px',
              background: 'var(--color-surface)',
              padding: '18px',
              display: 'flex',
              flexDirection: 'column',
              gap: '12px',
              boxShadow: 'var(--shadow-md)',
            }}
          >
            <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
              <span style={{ color: 'var(--color-text)', fontWeight: 700 }}>Import History</span>
              <span style={{ color: 'var(--color-text-muted)', fontSize: '13px' }}>
                Recent project-scoped import attempts for this source binding.
              </span>
              {restoreBlockedByPendingReview && (
                <span style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>
                  Resolve the pending import review before restoring an older applied import.
                </span>
              )}
            </div>

            {importDiffSummary && (
              <div
                style={{
                  background: 'var(--color-surface-offset)',
                  borderRadius: '14px',
                  padding: '14px',
                  display: 'flex',
                  flexDirection: 'column',
                  gap: '6px',
                }}
              >
                <span style={{ color: 'var(--color-text)', fontWeight: 600 }}>
                  Changes Since Last Applied Import
                </span>
                <span style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>
                  {importDiffSummary.added_nodes.length} added, {importDiffSummary.removed_nodes.length} removed
                </span>
                <div style={{ color: 'var(--color-text-muted)', fontSize: '12px', display: 'flex', gap: '12px', flexWrap: 'wrap' }}>
                  {importDiffSummary.current_head_revision && (
                    <span>Current revision: {importDiffSummary.current_head_revision.slice(0, 8)}</span>
                  )}
                  {importDiffSummary.compared_head_revision && (
                    <span>Previous revision: {importDiffSummary.compared_head_revision.slice(0, 8)}</span>
                  )}
                </div>
                {importDiffSummary.added_node_types.length > 0 && (
                  <span style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>
                    Added types: {importDiffSummary.added_node_types.map((entry) => `${entry.node_type} (${entry.count})`).join(', ')}
                  </span>
                )}
                {importDiffSummary.removed_node_types.length > 0 && (
                  <span style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>
                    Removed types: {importDiffSummary.removed_node_types.map((entry) => `${entry.node_type} (${entry.count})`).join(', ')}
                  </span>
                )}
                {importDiffSummary.added_nodes.length > 0 && (
                  <span style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>
                    Added nodes: {importDiffSummary.added_nodes.map((node) => node.node_name).join(', ')}
                  </span>
                )}
                {importDiffSummary.removed_nodes.length > 0 && (
                  <span style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>
                    Removed nodes: {importDiffSummary.removed_nodes.map((node) => node.node_name).join(', ')}
                  </span>
                )}
              </div>
            )}

            {selectedHistoryComparison && (
              <div
                style={{
                  background: 'var(--color-surface-offset)',
                  borderRadius: '14px',
                  padding: '14px',
                  display: 'flex',
                  flexDirection: 'column',
                  gap: '6px',
                }}
              >
                <span style={{ color: 'var(--color-text)', fontWeight: 600 }}>
                  Selected Historical Entry Compared To Current
                </span>
                <span style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>
                  Comparing import {importComparison?.selected_entry.import_job.id.slice(0, 8)} to current import {importComparison?.current_import_job.id.slice(0, 8)}.
                </span>
                {selectedHistoryComparisonNotes.map((note) => (
                  <span key={note} style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>
                    {note}
                  </span>
                ))}
                <span style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>
                  {selectedHistoryComparison.added_nodes.length} added, {selectedHistoryComparison.removed_nodes.length} removed
                </span>
                <div style={{ color: 'var(--color-text-muted)', fontSize: '12px', display: 'flex', gap: '12px', flexWrap: 'wrap' }}>
                  {selectedHistoryComparison.current_head_revision && (
                    <span>Current revision: {selectedHistoryComparison.current_head_revision.slice(0, 8)}</span>
                  )}
                  {selectedHistoryComparison.compared_head_revision && (
                    <span>Historical revision: {selectedHistoryComparison.compared_head_revision.slice(0, 8)}</span>
                  )}
                </div>
                {selectedHistoryComparison.added_node_types.length > 0 && (
                  <span style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>
                    Added types: {selectedHistoryComparison.added_node_types.map((entry) => `${entry.node_type} (${entry.count})`).join(', ')}
                  </span>
                )}
                {selectedHistoryComparison.removed_node_types.length > 0 && (
                  <span style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>
                    Removed types: {selectedHistoryComparison.removed_node_types.map((entry) => `${entry.node_type} (${entry.count})`).join(', ')}
                  </span>
                )}
                {selectedHistoryComparison.added_nodes.length > 0 && (
                  <span style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>
                    Added nodes: {selectedHistoryComparison.added_nodes.map((node) => node.node_name).join(', ')}
                  </span>
                )}
                {selectedHistoryComparison.removed_nodes.length > 0 && (
                  <span style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>
                    Removed nodes: {selectedHistoryComparison.removed_nodes.map((node) => node.node_name).join(', ')}
                  </span>
                )}
              </div>
            )}

            {selectedPairComparison && (
              <div
                style={{
                  background: 'var(--color-surface-offset)',
                  borderRadius: '14px',
                  padding: '14px',
                  display: 'flex',
                  flexDirection: 'column',
                  gap: '6px',
                }}
              >
                <span style={{ color: 'var(--color-text)', fontWeight: 600 }}>
                  Selected History Entries Compared
                </span>
                <span style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>
                  Comparing baseline import {importPairComparison?.baseline_entry.import_job.id.slice(0, 8)} to import {importPairComparison?.compared_entry.import_job.id.slice(0, 8)}.
                </span>
                {selectedPairComparisonNotes.map((note) => (
                  <span key={note} style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>
                    {note}
                  </span>
                ))}
                <span style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>
                  {selectedPairComparison.added_nodes.length} added, {selectedPairComparison.removed_nodes.length} removed
                </span>
                <div style={{ color: 'var(--color-text-muted)', fontSize: '12px', display: 'flex', gap: '12px', flexWrap: 'wrap' }}>
                  {selectedPairComparison.current_head_revision && (
                    <span>Compared revision: {selectedPairComparison.current_head_revision.slice(0, 8)}</span>
                  )}
                  {selectedPairComparison.compared_head_revision && (
                    <span>Baseline revision: {selectedPairComparison.compared_head_revision.slice(0, 8)}</span>
                  )}
                </div>
                {selectedPairComparison.added_node_types.length > 0 && (
                  <span style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>
                    Added types: {selectedPairComparison.added_node_types.map((entry) => `${entry.node_type} (${entry.count})`).join(', ')}
                  </span>
                )}
                {selectedPairComparison.removed_node_types.length > 0 && (
                  <span style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>
                    Removed types: {selectedPairComparison.removed_node_types.map((entry) => `${entry.node_type} (${entry.count})`).join(', ')}
                  </span>
                )}
                {selectedPairComparison.added_nodes.length > 0 && (
                  <span style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>
                    Added nodes: {selectedPairComparison.added_nodes.map((node) => node.node_name).join(', ')}
                  </span>
                )}
                {selectedPairComparison.removed_nodes.length > 0 && (
                  <span style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>
                    Removed nodes: {selectedPairComparison.removed_nodes.map((node) => node.node_name).join(', ')}
                  </span>
                )}
              </div>
            )}

            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
              {importHistoryEntries.map((entry) => (
                <article
                  key={entry.import_job.id}
                  style={{
                    background: 'var(--color-surface-offset)',
                    borderRadius: '14px',
                    padding: '14px',
                    display: 'flex',
                    flexDirection: 'column',
                    gap: '6px',
                  }}
                >
                  <div style={{ display: 'flex', justifyContent: 'space-between', gap: '12px', flexWrap: 'wrap' }}>
                    <span style={{ color: 'var(--color-text)', fontWeight: 600 }}>
                      {entry.import_job.provider.toUpperCase()} · {importStatusLabel(entry.import_job.status)}
                    </span>
                    <span style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>
                      {formatRelativeTime(entry.import_job.updated_at)}
                    </span>
                  </div>
                  <span style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>
                    Source: {entry.import_job.requested_ref}
                  </span>
                  {entry.import_job.restored_from_job_id && (
                    <span style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>
                      Restored from import {entry.import_job.restored_from_job_id.slice(0, 8)}
                    </span>
                  )}
                  <div style={{ color: 'var(--color-text-muted)', fontSize: '12px', display: 'flex', gap: '12px', flexWrap: 'wrap' }}>
                    {entry.source_metadata?.head_revision && (
                      <span>Revision: {entry.source_metadata.head_revision.slice(0, 8)}</span>
                    )}
                    {entry.discovered_node_count !== null && entry.discovered_node_count !== undefined && (
                      <span>Draft nodes: {entry.discovered_node_count}</span>
                    )}
                    {entry.effective_included_node_count !== null && entry.effective_included_node_count !== undefined && (
                      <span>
                        Effective selection: {entry.effective_included_node_count} included
                        {entry.effective_excluded_node_count !== null && entry.effective_excluded_node_count !== undefined
                          ? `, ${entry.effective_excluded_node_count} excluded`
                          : ''}
                      </span>
                    )}
                  </div>
                  {entry.effective_excluded_node_count !== null
                    && entry.effective_excluded_node_count !== undefined
                    && entry.effective_excluded_node_count > 0 && (
                    <span style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>
                      Saved exclusions affect this job&apos;s effective apply footprint.
                    </span>
                  )}
                  {entry.discovered_node_count !== null && entry.discovered_node_count !== undefined && (
                    <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
                      <button
                        className="btn btn-outline"
                        onClick={() => { handleSelectImportHistoryBaseline(entry.import_job.id); }}
                      >
                        {importHistoryBaselineJobId === entry.import_job.id ? 'Baseline Selected' : 'Use As Baseline'}
                      </button>
                      {importHistoryBaselineJobId && importHistoryBaselineJobId !== entry.import_job.id && (
                        <button
                          className="btn btn-outline"
                          onClick={() => { void handleCompareImportHistoryToBaseline(entry.import_job.id); }}
                        >
                          Compare To Selected
                        </button>
                      )}
                      {entry.import_job.id !== importState?.import_job.id && (
                        <button
                          className="btn btn-outline"
                          onClick={() => { void handleCompareImportHistoryEntry(entry.import_job.id); }}
                        >
                          Compare To Current
                        </button>
                      )}
                    </div>
                  )}
                  {entry.import_job.status === 'review_pending'
                    && !restoreBlockedByPendingReview
                    && entry.import_job.id !== importState?.import_job.id && (
                    <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
                      <button
                        className="btn btn-outline"
                        onClick={() => { void handleRestoreImportReviewDraft(entry.import_job.id); }}
                        disabled={restorePendingJobId === entry.import_job.id}
                      >
                        {restorePendingJobId === entry.import_job.id ? 'Restoring Draft…' : 'Restore Draft For Review'}
                      </button>
                    </div>
                  )}
                  {entry.import_job.status === 'applied'
                    && !restoreBlockedByPendingReview
                    && entry.import_job.id !== importState?.import_job.id && (
                    <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
                      <button
                        className="btn btn-outline"
                        onClick={() => { void handleRestoreImportHistoryEntryForReview(entry.import_job.id); }}
                        disabled={restorePendingJobId === entry.import_job.id}
                      >
                        {restorePendingJobId === entry.import_job.id ? 'Restoring For Review…' : 'Restore For Review'}
                      </button>
                      <button
                        className="btn btn-outline"
                        onClick={() => { void handleRestoreImportHistoryEntry(entry.import_job.id); }}
                        disabled={restorePendingJobId === entry.import_job.id}
                      >
                        {restorePendingJobId === entry.import_job.id ? 'Restoring…' : 'Restore This Import'}
                      </button>
                    </div>
                  )}
                </article>
              ))}
            </div>
          </section>
        )}

        {!loading && !error && sessions.length === 0 && (
          <div className="empty-state-card">
            <span className="empty-state-kicker">Session intake</span>
            <span className="empty-state-title">No sessions in this project yet.</span>
            <span className="empty-state-body">
              Start the first project session to open intake, build the planning brief, and move into the pipeline from project context.
            </span>
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
                  borderRadius: '14px',
                  background: 'var(--color-surface)',
                  padding: '14px 16px',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  gap: '10px',
                  flexWrap: 'wrap',
                  boxShadow: 'var(--shadow-md)',
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
