import { useCallback, useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import Layout from '../components/Layout.tsx';
import { createApiClient } from '../api/client.ts';
import { useGetAccessToken } from '../auth/useAuthenticatedFetch.ts';
import { buildKnowledgeDeepLink } from '../lib/knowledgeDeepLinks.ts';
import type { IntakePhase, SessionSummary } from '../types.ts';

type BadgeTone = 'default' | 'primary' | 'success' | 'warning' | 'error';

interface BadgeDescriptor {
  label: string;
  tone: BadgeTone;
}

const ACTIVE_PHASES = new Set<IntakePhase>(['interviewing', 'pipeline_running']);

const PHASE_CONFIG: Record<
  IntakePhase,
  { label: string; color: string; bg: string; borderColor: string }
> = {
  waiting: {
    label: 'waiting',
    color: 'var(--color-text-muted)',
    bg: 'rgba(136,136,160,0.12)',
    borderColor: 'rgba(136,136,160,0.3)',
  },
  interviewing: {
    label: 'interviewing',
    color: 'var(--color-primary)',
    bg: 'rgba(0,212,255,0.08)',
    borderColor: 'var(--color-primary)',
  },
  pipeline_running: {
    label: 'building',
    color: 'var(--color-gold)',
    bg: 'rgba(255,215,0,0.08)',
    borderColor: 'rgba(255,215,0,0.5)',
  },
  complete: {
    label: 'complete',
    color: 'var(--color-success)',
    bg: 'rgba(0,255,136,0.08)',
    borderColor: 'rgba(0,255,136,0.4)',
  },
  error: {
    label: 'error',
    color: 'var(--color-error)',
    bg: 'rgba(255,68,68,0.10)',
    borderColor: 'var(--color-error)',
  },
};

function formatDate(iso: string): string {
  const parsed = new Date(iso);
  if (Number.isNaN(parsed.getTime())) return iso;

  return parsed.toLocaleString([], {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  });
}

function formatRelativeTime(iso: string): string {
  const parsed = new Date(iso);
  if (Number.isNaN(parsed.getTime())) return 'time unavailable';

  const diffMs = Date.now() - parsed.getTime();
  if (diffMs < 60_000) return 'just now';

  const minutes = Math.floor(diffMs / 60_000);
  if (minutes < 60) return `${minutes}m ago`;

  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;

  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

function getSessionTitle(
  session: Pick<SessionSummary, 'title' | 'project_description' | 'id'>,
): string {
  const explicit = session.title?.trim();
  if (explicit) return explicit;

  const description = session.project_description?.trim();
  if (description) {
    const singleLine = description.replace(/\s+/g, ' ').trim();
    return singleLine.length > 72 ? `${singleLine.slice(0, 72)}…` : singleLine;
  }

  return `Session ${session.id.slice(0, 8)}`;
}

function getDescriptionSnippet(description?: string | null): string {
  if (!description?.trim()) return 'No planning brief saved yet.';
  return description.length > 120 ? `${description.slice(0, 120)}…` : description;
}

function formatWorkflowStep(step?: string | null): string | null {
  if (!step?.trim()) return null;
  return step
    .split('.')
    .map((part) => part.replace(/_/g, ' '))
    .join(' / ');
}

function getPrimaryActionLabel(session: SessionSummary): string {
  if (session.can_resume_checkpoint) return 'Resume Interview';
  if (session.can_resume_live) return session.intake_phase === 'pipeline_running' ? 'Reconnect Build' : 'Reconnect';
  if (session.can_retry_pipeline) return 'Retry Pipeline';
  if (session.can_restart_from_description) return 'Restart Interview';
  if (session.resume_status === 'ready_to_start') return 'Start Interview';
  return 'Open Session';
}

function getPrimaryActionTone(session: SessionSummary): BadgeTone {
  if (session.can_retry_pipeline) return 'error';
  if (session.can_restart_from_description && session.intake_phase === 'interviewing') return 'warning';
  if (session.can_resume_checkpoint || session.can_resume_live || session.resume_status === 'ready_to_start') {
    return 'primary';
  }
  return 'default';
}

function isActionable(session: SessionSummary): boolean {
  return session.can_resume_checkpoint
    || session.can_resume_live
    || session.can_retry_pipeline
    || session.can_restart_from_description
    || session.resume_status === 'ready_to_start';
}

function needsAttention(session: SessionSummary): boolean {
  return session.intake_phase === 'error'
    || Boolean(session.error_message)
    || session.error_count > 0
    || session.warning_count > 0
    || session.can_retry_pipeline
    || session.resume_status === 'interview_restart_only'
    || session.resume_status === 'interview_resume_unknown';
}

function getPriorityBucket(session: SessionSummary): number {
  if (needsAttention(session)) return 0;
  if (ACTIVE_PHASES.has(session.intake_phase) && isActionable(session)) return 1;
  if (isActionable(session)) return 2;
  if (ACTIVE_PHASES.has(session.intake_phase)) return 3;
  if (session.intake_phase === 'waiting') return 4;
  if (session.intake_phase === 'complete') return 5;
  return 6;
}

function compareSessions(left: SessionSummary, right: SessionSummary): number {
  if (left.archived !== right.archived) {
    return left.archived ? 1 : -1;
  }

  const priorityDiff = getPriorityBucket(left) - getPriorityBucket(right);
  if (priorityDiff !== 0) return priorityDiff;

  const activityDiff = new Date(right.last_activity_at).getTime() - new Date(left.last_activity_at).getTime();
  if (activityDiff !== 0) return activityDiff;

  return right.created_at.localeCompare(left.created_at);
}

function getStateBadge(session: SessionSummary): BadgeDescriptor {
  if (session.archived) {
    return { label: 'archived', tone: 'default' };
  }

  switch (session.resume_status) {
    case 'interview_attached':
      return { label: 'live attached', tone: 'success' };
    case 'live_attach_available':
      if (session.intake_phase === 'pipeline_running') return { label: 'live build', tone: 'primary' };
      if (session.intake_phase === 'complete') return { label: 'results ready', tone: 'success' };
      if (session.intake_phase === 'error') return { label: 'failed run', tone: 'error' };
      return { label: 'live resume', tone: 'primary' };
    case 'interview_checkpoint_resumable':
      return { label: 'checkpoint resume', tone: 'primary' };
    case 'interview_restart_only':
      return { label: 'restart only', tone: 'warning' };
    case 'interview_resume_unknown':
      return { label: 'resume unknown', tone: 'warning' };
    case 'ready_to_start':
    default:
      return { label: 'ready to start', tone: 'default' };
  }
}

function getAttentionBadges(session: SessionSummary): BadgeDescriptor[] {
  const badges: BadgeDescriptor[] = [];

  if (session.can_retry_pipeline) {
    badges.push({ label: 'needs retry', tone: 'error' });
  }
  if (session.resume_status === 'interview_restart_only') {
    badges.push({ label: 'needs restart', tone: 'warning' });
  }
  if (session.resume_status === 'interview_resume_unknown') {
    badges.push({ label: 'resume unknown', tone: 'warning' });
  }
  if (session.warning_count > 0) {
    badges.push({
      label: `${session.warning_count} warning${session.warning_count === 1 ? '' : 's'}`,
      tone: 'warning',
    });
  }
  if (session.error_count > 0 || session.intake_phase === 'error') {
    const count = Math.max(session.error_count, session.intake_phase === 'error' ? 1 : 0);
    badges.push({ label: `${count} error${count === 1 ? '' : 's'}`, tone: 'error' });
  }

  return badges;
}

function getWorkflowSummary(session: SessionSummary): string {
  if (session.archived) {
    return 'Archived session. Unarchive it to return it to the main working surface.';
  }

  const step = formatWorkflowStep(session.current_step);
  if (step) return `Step: ${step}`;

  switch (session.intake_phase) {
    case 'waiting':
      return 'Awaiting the initial planning brief.';
    case 'interviewing':
      if (session.can_resume_checkpoint) return 'Checkpoint is saved and ready to resume.';
      if (session.can_resume_live) return 'Live interview runtime is available for reattach.';
      if (session.interview_live_attached) return 'Interview is currently attached.';
      if (session.can_restart_from_description) return 'Interview needs a fresh restart from the saved brief.';
      return 'Interview is detached and waiting for intervention.';
    case 'pipeline_running':
      return 'Pipeline is actively processing this session.';
    case 'complete':
      return 'Pipeline finished; outputs are ready for review.';
    case 'error':
      return session.can_retry_pipeline
        ? 'Pipeline failed and can be retried from the saved brief.'
        : 'Pipeline failed; inspect the session for details.';
    default:
      return 'Workflow status is available in the session detail view.';
  }
}

function getBadgeStyle(tone: BadgeTone) {
  switch (tone) {
    case 'primary':
      return {
        background: 'rgba(0,212,255,0.10)',
        border: '1px solid rgba(0,212,255,0.45)',
        color: 'var(--color-primary)',
      };
    case 'success':
      return {
        background: 'rgba(0,255,136,0.10)',
        border: '1px solid rgba(0,255,136,0.35)',
        color: 'var(--color-success)',
      };
    case 'warning':
      return {
        background: 'rgba(255,215,0,0.10)',
        border: '1px solid rgba(255,215,0,0.38)',
        color: 'var(--color-gold)',
      };
    case 'error':
      return {
        background: 'rgba(255,68,68,0.10)',
        border: '1px solid rgba(255,68,68,0.42)',
        color: 'var(--color-error)',
      };
    case 'default':
    default:
      return {
        background: 'rgba(136,136,160,0.12)',
        border: '1px solid rgba(136,136,160,0.25)',
        color: 'var(--color-text-muted)',
      };
  }
}

function Badge({ label, tone }: BadgeDescriptor) {
  return (
    <span
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: '5px',
        padding: '4px 8px',
        borderRadius: '999px',
        fontSize: '10px',
        fontWeight: 700,
        letterSpacing: '0.04em',
        textTransform: 'uppercase',
        whiteSpace: 'nowrap',
        ...getBadgeStyle(tone),
      }}
    >
      {label}
    </span>
  );
}

function MetadataPill({ children }: { children: string }) {
  return (
    <span
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        padding: '4px 8px',
        borderRadius: '999px',
        background: 'color-mix(in srgb, var(--color-surface-offset) 78%, transparent)',
        color: 'var(--color-text-muted)',
        fontSize: '11px',
        lineHeight: 1.4,
      }}
    >
      {children}
    </span>
  );
}

type SessionCardActionKind = 'rename' | 'duplicate' | 'archive';

function CardActionButton(
  {
    label,
    ariaLabel,
    title,
    onClick,
    disabled = false,
  }: {
    label: string;
    ariaLabel: string;
    title?: string;
    onClick: () => void;
    disabled?: boolean;
  },
) {
  return (
    <button
      type="button"
      aria-label={ariaLabel}
      title={title}
      disabled={disabled}
      onClick={(event) => {
        event.stopPropagation();
        onClick();
      }}
      style={{
        background: 'color-mix(in srgb, var(--color-surface-2) 88%, transparent)',
        border: 'none',
        boxShadow: 'inset 0 0 0 1px var(--color-ghost-border)',
        color: disabled ? 'var(--color-text-muted)' : 'var(--color-text-muted)',
        padding: '5px 10px',
        borderRadius: '999px',
        fontSize: '10px',
        fontWeight: 700,
        letterSpacing: '0.05em',
        textTransform: 'uppercase',
        cursor: disabled ? 'not-allowed' : 'pointer',
        opacity: disabled ? 0.55 : 0.9,
      }}
    >
      {label}
    </button>
  );
}

interface SessionCardProps {
  session: SessionSummary;
  onClick: () => void;
  onOpenKnowledge: () => void;
  onRename: () => void;
  onDuplicate: () => void;
  onArchiveToggle: () => void;
  actionBusy: SessionCardActionKind | null;
}

function SessionCard({
  session,
  onClick,
  onOpenKnowledge,
  onRename,
  onDuplicate,
  onArchiveToggle,
  actionBusy,
}: SessionCardProps) {
  const phaseConfig = PHASE_CONFIG[session.intake_phase];
  const primaryAction = getPrimaryActionLabel(session);
  const primaryActionTone = getPrimaryActionTone(session);
  const stateBadge = getStateBadge(session);
  const attentionBadges = getAttentionBadges(session);
  const title = getSessionTitle(session);
  const needsAlertTone = attentionBadges.some((badge) => badge.tone === 'error')
    ? 'error'
    : attentionBadges.length > 0
      ? 'warning'
      : session.can_resume_checkpoint || session.can_resume_live
        ? 'primary'
        : 'default';
  const lastActivity = `${formatRelativeTime(session.last_activity_at)} · ${formatDate(session.last_activity_at)}`;
  const description = getDescriptionSnippet(session.project_description);
  const workflowSummary = getWorkflowSummary(session);
  const convergencePct = session.convergence_pct != null
    ? `${Math.round(session.convergence_pct * 100)}% converged`
    : null;
  const classification = session.classification
    ? `${session.classification.project_type} · ${session.classification.complexity}`
    : null;
  const checkpointSaved = session.checkpoint_last_saved_at
    ? `Checkpoint ${formatRelativeTime(session.checkpoint_last_saved_at)}`
    : null;
  const projectLabel = session.project_name?.trim() || session.project_slug?.trim() || null;
  const knowledgeAvailable = Boolean(session.project_id?.trim());
  const alertMessage = session.error_message
    ? session.error_message
    : session.resume_status === 'interview_resume_unknown'
      ? 'Resume path is unclear; inspect the session before continuing.'
      : session.resume_status === 'interview_restart_only'
        ? 'The live interview is detached; restart from the saved brief to continue.'
        : null;

  return (
    <div
      role="button"
      tabIndex={0}
      aria-label={`Open session ${session.id}`}
      data-testid={`session-card-${session.id}`}
      onClick={onClick}
      onKeyDown={(event) => {
        if (event.key === 'Enter' || event.key === ' ') onClick();
      }}
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: '12px',
        padding: '18px 20px',
        background: `linear-gradient(180deg, color-mix(in srgb, ${phaseConfig.bg} 62%, var(--color-surface-offset)), color-mix(in srgb, var(--color-surface) 94%, transparent))`,
        borderRadius: '24px',
        boxShadow: 'var(--shadow-sm)',
        cursor: 'pointer',
        transition: 'transform 0.18s ease, background 0.18s ease, box-shadow 0.18s ease',
      }}
      onMouseEnter={(event) => {
        const element = event.currentTarget as HTMLDivElement;
        element.style.transform = 'translateY(-1px)';
        element.style.boxShadow = 'var(--shadow-md)';
      }}
      onMouseLeave={(event) => {
        const element = event.currentTarget as HTMLDivElement;
        element.style.transform = 'translateY(0)';
        element.style.boxShadow = 'var(--shadow-sm)';
      }}
    >
      <div style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1.35fr) minmax(220px, 0.95fr)', gap: '14px 18px', alignItems: 'start' }}>
        <div style={{ display: 'flex', flexDirection: 'column', gap: '6px', minWidth: 0 }}>
          <span style={{ color: 'var(--color-text)', fontSize: '15px', fontWeight: 700 }}>
            {title}
          </span>
          <span
            style={{
              color: 'var(--color-primary)',
              fontSize: '11px',
              fontWeight: 700,
              letterSpacing: '0.05em',
              fontFamily: 'monospace',
            }}
          >
            {session.id.slice(0, 8)}…
          </span>
          <span style={{ color: 'var(--color-text-muted)', fontSize: '11px' }}>
            Last activity {lastActivity}
          </span>
          <div style={{ color: 'var(--color-text)', fontSize: '13px', lineHeight: 1.55 }}>
            {description}
          </div>
          <div style={{ color: 'var(--color-text-muted)', fontSize: '12px', lineHeight: 1.6 }}>
            {workflowSummary}
          </div>
        </div>

        <div style={{ display: 'flex', flexDirection: 'column', gap: '10px', minWidth: 0 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px', flexWrap: 'wrap', justifyContent: 'flex-start' }}>
            <span
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                padding: '4px 9px',
                borderRadius: '999px',
                background: phaseConfig.bg,
                color: phaseConfig.color,
                fontSize: '10px',
                fontWeight: 700,
                letterSpacing: '0.05em',
                textTransform: 'uppercase',
                whiteSpace: 'nowrap',
              }}
            >
              {phaseConfig.label}
            </span>
            <Badge label={primaryAction} tone={primaryActionTone} />
            <Badge label={stateBadge.label} tone={stateBadge.tone} />
          </div>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: '8px' }}>
            {projectLabel && <MetadataPill>{`Project: ${projectLabel}`}</MetadataPill>}
            <MetadataPill>{`${session.message_count} ${session.message_count === 1 ? 'message' : 'messages'}`}</MetadataPill>
            <MetadataPill>{`${session.event_count} ${session.event_count === 1 ? 'event' : 'events'}`}</MetadataPill>
            {classification && <MetadataPill>{classification}</MetadataPill>}
            {convergencePct && <MetadataPill>{convergencePct}</MetadataPill>}
            {checkpointSaved && <MetadataPill>{checkpointSaved}</MetadataPill>}
          </div>
        </div>
      </div>

      {attentionBadges.length > 0 && (
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: '8px' }}>
          {attentionBadges.map((badge) => (
            <Badge key={`${session.id}-${badge.label}`} label={badge.label} tone={badge.tone} />
          ))}
        </div>
      )}

      <div style={{ display: 'flex', flexWrap: 'wrap', gap: '8px' }}>
        <CardActionButton
          label="Knowledge"
          ariaLabel={knowledgeAvailable ? `Open knowledge for session ${session.id}` : `Knowledge unavailable for session ${session.id}`}
          title={knowledgeAvailable
            ? projectLabel
              ? `Open Knowledge for ${projectLabel}`
              : 'Open project-scoped Knowledge'
            : 'Knowledge unavailable: session is not attached to a project yet'}
          onClick={onOpenKnowledge}
          disabled={!knowledgeAvailable || actionBusy !== null}
        />
        <CardActionButton
          label={actionBusy === 'rename' ? 'Renaming…' : 'Rename'}
          ariaLabel={`Rename session ${session.id}`}
          onClick={onRename}
          disabled={actionBusy !== null}
        />
        <CardActionButton
          label={actionBusy === 'duplicate' ? 'Duplicating…' : 'Duplicate'}
          ariaLabel={`Duplicate session ${session.id}`}
          onClick={onDuplicate}
          disabled={actionBusy !== null}
        />
        <CardActionButton
          label={session.archived
            ? actionBusy === 'archive'
              ? 'Restoring…'
              : 'Unarchive'
            : actionBusy === 'archive'
              ? 'Archiving…'
              : 'Archive'}
          ariaLabel={`${session.archived ? 'Unarchive' : 'Archive'} session ${session.id}`}
          onClick={onArchiveToggle}
          disabled={actionBusy !== null || (!session.archived && (session.intake_phase === 'interviewing' || session.pipeline_running))}
        />
      </div>

      {alertMessage && (
        <div
          style={{
            padding: '10px 12px',
            borderRadius: '14px',
            background: attentionBadges.some((badge) => badge.tone === 'error')
              ? 'rgba(255,68,68,0.08)'
              : 'rgba(255,215,0,0.08)',
            color: attentionBadges.some((badge) => badge.tone === 'error')
              ? 'var(--color-error)'
              : 'var(--color-gold)',
            fontSize: '12px',
            lineHeight: 1.5,
          }}
        >
          {alertMessage}
        </div>
      )}
    </div>
  );
}

export default function Dashboard() {
  const navigate = useNavigate();
  const getToken = useGetAccessToken();
  const api = useMemo(() => createApiClient(getToken), [getToken]);

  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [fetchError, setFetchError] = useState<string | null>(null);
  const [showArchived, setShowArchived] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  const [activeAction, setActiveAction] = useState<{
    sessionId: string;
    kind: SessionCardActionKind;
  } | null>(null);

  const loadSessions = useCallback(async (): Promise<void> => {
    setLoading(true);
    setFetchError(null);
    try {
      const response = await api.listSessions({ includeArchived: showArchived });
      setSessions(response.sessions);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setFetchError(message);
    } finally {
      setLoading(false);
    }
  }, [api, showArchived]);

  useEffect(() => {
    void loadSessions();
  }, [loadSessions]);

  const handleRename = useCallback(async (session: SessionSummary): Promise<void> => {
    const currentTitle = getSessionTitle(session);
    const nextTitle = window.prompt('Rename session', currentTitle);
    if (nextTitle === null) return;
    const trimmed = nextTitle.trim();
    if (!trimmed || trimmed === currentTitle) return;

    setActionError(null);
    setActiveAction({ sessionId: session.id, kind: 'rename' });
    try {
      await api.updateSession(session.id, { title: trimmed });
      await loadSessions();
    } catch (error) {
      setActionError(error instanceof Error ? error.message : String(error));
    } finally {
      setActiveAction(null);
    }
  }, [api, loadSessions]);

  const handleDuplicate = useCallback(async (session: SessionSummary): Promise<void> => {
    const suggestedTitle = `${getSessionTitle(session)} (Copy)`;
    const requestedTitle = window.prompt('Name the duplicate session', suggestedTitle);
    if (requestedTitle === null) return;
    const trimmed = requestedTitle.trim();

    setActionError(null);
    setActiveAction({ sessionId: session.id, kind: 'duplicate' });
    try {
      const response = await api.duplicateSession(
        session.id,
        trimmed ? { title: trimmed } : undefined,
      );
      await loadSessions();
      void navigate(`/session/${response.session.id}`);
    } catch (error) {
      setActionError(error instanceof Error ? error.message : String(error));
    } finally {
      setActiveAction(null);
    }
  }, [api, loadSessions, navigate]);

  const handleArchiveToggle = useCallback(async (session: SessionSummary): Promise<void> => {
    if (!session.archived) {
      const confirmed = window.confirm(`Archive "${getSessionTitle(session)}"?`);
      if (!confirmed) return;
    }

    setActionError(null);
    setActiveAction({ sessionId: session.id, kind: 'archive' });
    try {
      await api.updateSession(session.id, { archived: !session.archived });
      await loadSessions();
    } catch (error) {
      setActionError(error instanceof Error ? error.message : String(error));
    } finally {
      setActiveAction(null);
    }
  }, [api, loadSessions]);

  const sortedSessions = useMemo(() => [...sessions].sort(compareSessions), [sessions]);
  const actionableCount = sortedSessions.filter(isActionable).length;
  const attentionCount = sortedSessions.filter(needsAttention).length;
  const activeCount = sortedSessions.filter((session) => ACTIVE_PHASES.has(session.intake_phase)).length;
  const archivedCount = sortedSessions.filter((session) => session.archived).length;

  return (
    <Layout>
      <div className="command-page" style={{ maxWidth: '980px' }}>
        <section className="command-hero-grid">
          <div className="command-surface-strong">
            <div className="command-surface-header">
              <div className="command-surface-copy">
                <span className="page-kicker">Operational queue</span>
                <h1 className="display-heading" style={{ margin: 0 }}>Sessions</h1>
                <p className="section-copy" style={{ margin: 0 }}>
                  Watch resumability, interventions, and recent activity across the global session queue while new planning work starts from projects.
                </p>
              </div>
              <div className="command-pill-matrix">
                <a href="/admin" className="command-link">
                  Admin
                </a>
                <button
                  type="button"
                  className="btn btn-outline"
                  aria-pressed={showArchived}
                  onClick={() => setShowArchived((value) => !value)}
                  style={showArchived ? { color: 'var(--color-primary)' } : undefined}
                >
                  {showArchived ? 'hide archived' : 'show archived'}
                </button>
                <button
                  type="button"
                  className="btn btn-primary"
                  onClick={() => void navigate('/projects')}
                >
                  start from project
                </button>
              </div>
            </div>
            <div className="utility-note" style={{ margin: 0 }}>
              Attention and resumability sort first, then recent activity. The queue is for intervention and continuity, not project creation.
            </div>
          </div>

          <aside className="command-surface-soft">
            <div className="command-info-grid">
              <div className="command-info-cell">
                <span className="command-info-label">Actionable</span>
                <span className="command-info-value">{actionableCount}</span>
                <span className="command-info-copy">Sessions ready to resume, restart, retry, or open now.</span>
              </div>
              <div className="command-info-cell">
                <span className="command-info-label">Attention needed</span>
                <span className="command-info-value">{attentionCount}</span>
                <span className="command-info-copy">Warnings, errors, and blocked flows that need intervention.</span>
              </div>
              <div className="command-info-cell">
                <span className="command-info-label">Live or building</span>
                <span className="command-info-value">{activeCount}</span>
                <span className="command-info-copy">Interviewing and pipeline-running work currently on the board.</span>
              </div>
              {showArchived && (
                <div className="command-info-cell">
                  <span className="command-info-label">Archived in view</span>
                  <span className="command-info-value">{archivedCount}</span>
                  <span className="command-info-copy">Archived sessions are visible without overtaking the active queue.</span>
                </div>
              )}
            </div>
          </aside>
        </section>

        {loading && (
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              padding: '40px 24px',
              color: 'var(--color-text-muted)',
              fontSize: '13px',
            }}
          >
            loading sessions…
          </div>
        )}

        {!loading && fetchError && (
          <div
            className="utility-card"
            style={{
              background: 'rgba(255,68,68,0.06)',
              color: 'var(--color-error)',
              fontSize: '13px',
            }}
          >
            <span style={{ fontWeight: 600 }}>Error loading sessions: </span>
            {fetchError}
          </div>
        )}

        {!loading && !fetchError && actionError && (
          <div
            className="utility-card"
            style={{
              background: 'rgba(255,68,68,0.06)',
              color: 'var(--color-error)',
              fontSize: '12px',
            }}
          >
            {actionError}
          </div>
        )}

        {!loading && !fetchError && sessions.length === 0 && (
          <div className="empty-state-card" style={{ alignItems: 'flex-start', paddingBlock: '40px' }}>
            <span className="empty-state-kicker">Global queue</span>
            <h2 className="empty-state-title" style={{ maxWidth: '28rem' }}>
              {showArchived ? 'No sessions match this view.' : 'No sessions yet.'}
            </h2>
            <p className="empty-state-body">
              {showArchived
                ? 'toggle archived sessions off or open project sessions to continue'
                : 'sessions are now project-scoped. open projects to start planning'}
            </p>
            <button
              type="button"
              className="btn btn-primary"
              onClick={() => void navigate('/projects')}
              style={{ marginTop: '4px' }}
            >
              open projects →
            </button>
          </div>
        )}

        {!loading && !fetchError && sessions.length > 0 && (
          <div className="directory-list">
            {sortedSessions.map((session) => (
              <SessionCard
                key={session.id}
                session={session}
                onClick={() => void navigate(`/session/${session.id}`)}
                onOpenKnowledge={() => {
                  if (!session.project_id?.trim()) return;
                  void navigate(
                    buildKnowledgeDeepLink({
                      projectId: session.project_id,
                      originPath: '/sessions',
                      originLabel: 'Sessions',
                    }),
                  );
                }}
                onRename={() => { void handleRename(session); }}
                onDuplicate={() => { void handleDuplicate(session); }}
                onArchiveToggle={() => { void handleArchiveToggle(session); }}
                actionBusy={
                  activeAction?.sessionId === session.id
                    ? activeAction.kind
                    : null
                }
              />
            ))}
          </div>
        )}

        <div className="utility-note">
          <span style={{ color: 'var(--color-primary)', fontWeight: 600 }}>TIP</span>
          {' '}Sessions remain a global operational queue, while new planning work starts from projects.
        </div>
      </div>
    </Layout>
  );
}
