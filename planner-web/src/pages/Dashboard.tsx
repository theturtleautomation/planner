import { useCallback, useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import Layout from '../components/Layout.tsx';
import { createApiClient } from '../api/client.ts';
import { useGetAccessToken } from '../auth/useAuthenticatedFetch.ts';
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
        background: 'rgba(136,136,160,0.08)',
        border: '1px solid rgba(136,136,160,0.18)',
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
    onClick,
    disabled = false,
  }: {
    label: string;
    ariaLabel: string;
    onClick: () => void;
    disabled?: boolean;
  },
) {
  return (
    <button
      type="button"
      aria-label={ariaLabel}
      disabled={disabled}
      onClick={(event) => {
        event.stopPropagation();
        onClick();
      }}
      style={{
        background: 'transparent',
        border: '1px solid rgba(136,136,160,0.25)',
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
  onRename: () => void;
  onDuplicate: () => void;
  onArchiveToggle: () => void;
  actionBusy: SessionCardActionKind | null;
}

function SessionCard({
  session,
  onClick,
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
        padding: '16px 18px',
        background: 'var(--color-surface)',
        border: `1px solid ${phaseConfig.borderColor}`,
        borderLeft: `4px solid ${getBadgeStyle(needsAlertTone).color}`,
        borderRadius: '8px',
        cursor: 'pointer',
        transition: 'transform 0.18s ease, border-color 0.18s ease, background 0.18s ease',
      }}
      onMouseEnter={(event) => {
        const element = event.currentTarget as HTMLDivElement;
        element.style.transform = 'translateY(-1px)';
        element.style.background = 'var(--color-surface-2)';
      }}
      onMouseLeave={(event) => {
        const element = event.currentTarget as HTMLDivElement;
        element.style.transform = 'translateY(0)';
        element.style.background = 'var(--color-surface)';
      }}
    >
      <div style={{ display: 'flex', justifyContent: 'space-between', gap: '16px', flexWrap: 'wrap' }}>
        <div style={{ display: 'flex', flexDirection: 'column', gap: '4px', minWidth: 0 }}>
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
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: '8px', flexWrap: 'wrap', justifyContent: 'flex-end' }}>
          <span
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              padding: '4px 9px',
              borderRadius: '999px',
              border: `1px solid ${phaseConfig.borderColor}`,
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
        </div>
      </div>

      <div style={{ color: 'var(--color-text)', fontSize: '13px', lineHeight: 1.55 }}>
        {description}
      </div>

      <div style={{ color: 'var(--color-text-muted)', fontSize: '12px', lineHeight: 1.6 }}>
        {workflowSummary}
      </div>

        <div style={{ display: 'flex', flexWrap: 'wrap', gap: '8px' }}>
          <Badge label={stateBadge.label} tone={stateBadge.tone} />
          {projectLabel && <MetadataPill>{`Project: ${projectLabel}`}</MetadataPill>}
          <MetadataPill>{`${session.message_count} ${session.message_count === 1 ? 'message' : 'messages'}`}</MetadataPill>
          <MetadataPill>{`${session.event_count} ${session.event_count === 1 ? 'event' : 'events'}`}</MetadataPill>
        {classification && <MetadataPill>{classification}</MetadataPill>}
        {convergencePct && <MetadataPill>{convergencePct}</MetadataPill>}
        {checkpointSaved && <MetadataPill>{checkpointSaved}</MetadataPill>}
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
            borderRadius: '6px',
            background: attentionBadges.some((badge) => badge.tone === 'error')
              ? 'rgba(255,68,68,0.08)'
              : 'rgba(255,215,0,0.08)',
            border: attentionBadges.some((badge) => badge.tone === 'error')
              ? '1px solid rgba(255,68,68,0.28)'
              : '1px solid rgba(255,215,0,0.28)',
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

  return (
    <Layout>
      <div
        style={{
          flex: 1,
          overflow: 'auto',
          padding: '32px 24px',
          display: 'flex',
          flexDirection: 'column',
          gap: '24px',
          maxWidth: '980px',
          margin: '0 auto',
          width: '100%',
        }}
      >
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            borderBottom: '1px solid var(--color-border)',
            paddingBottom: '12px',
            gap: '12px',
            flexWrap: 'wrap',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: '10px', flexWrap: 'wrap' }}>
            <span style={{ color: 'var(--color-text)', fontSize: '14px', fontWeight: 600 }}>
              sessions
            </span>
            <a
              href="/admin"
              style={{
                color: 'var(--color-text-muted)',
                fontSize: '11px',
                textDecoration: 'none',
                opacity: 0.6,
                transition: 'opacity 0.18s',
                fontFamily: 'monospace',
              }}
              onMouseEnter={(event) => { (event.currentTarget as HTMLAnchorElement).style.opacity = '1'; }}
              onMouseLeave={(event) => { (event.currentTarget as HTMLAnchorElement).style.opacity = '0.6'; }}
            >
              admin →
            </a>
            <button
              type="button"
              onClick={() => setShowArchived((value) => !value)}
              style={{
                background: 'transparent',
                border: '1px solid rgba(136,136,160,0.25)',
                color: showArchived ? 'var(--color-primary)' : 'var(--color-text-muted)',
                fontSize: '11px',
                cursor: 'pointer',
                letterSpacing: '0.04em',
                borderRadius: '999px',
                fontFamily: 'inherit',
                padding: '4px 10px',
              }}
            >
              {showArchived ? 'hide archived' : 'show archived'}
            </button>
          </div>

          <button
            onClick={() => void navigate('/projects')}
            style={{
              background: 'var(--color-primary)',
              border: 'none',
              color: 'var(--color-bg)',
              padding: '8px 18px',
              fontSize: '12px',
              fontWeight: 700,
              cursor: 'pointer',
              letterSpacing: '0.05em',
              textTransform: 'uppercase',
              borderRadius: '4px',
              fontFamily: 'inherit',
              transition: 'opacity 0.18s',
            }}
            onMouseEnter={(event) => { (event.currentTarget as HTMLButtonElement).style.opacity = '0.85'; }}
            onMouseLeave={(event) => { (event.currentTarget as HTMLButtonElement).style.opacity = '1'; }}
          >
            start from project
          </button>
        </div>

        {!loading && !fetchError && sessions.length > 0 && (
          <div
            style={{
              display: 'grid',
              gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))',
              gap: '12px',
            }}
          >
            <div
              style={{
                padding: '12px 14px',
                borderRadius: '8px',
                border: '1px solid rgba(0,212,255,0.22)',
                background: 'rgba(0,212,255,0.05)',
              }}
            >
              <div style={{ color: 'var(--color-primary)', fontSize: '10px', fontWeight: 700, letterSpacing: '0.05em', textTransform: 'uppercase' }}>
                actionable
              </div>
              <div style={{ color: 'var(--color-text)', fontSize: '22px', fontWeight: 700, marginTop: '4px' }}>
                {actionableCount}
              </div>
            </div>
            <div
              style={{
                padding: '12px 14px',
                borderRadius: '8px',
                border: '1px solid rgba(255,215,0,0.22)',
                background: 'rgba(255,215,0,0.05)',
              }}
            >
              <div style={{ color: 'var(--color-gold)', fontSize: '10px', fontWeight: 700, letterSpacing: '0.05em', textTransform: 'uppercase' }}>
                attention needed
              </div>
              <div style={{ color: 'var(--color-text)', fontSize: '22px', fontWeight: 700, marginTop: '4px' }}>
                {attentionCount}
              </div>
            </div>
            <div
              style={{
                padding: '12px 14px',
                borderRadius: '8px',
                border: '1px solid rgba(136,136,160,0.22)',
                background: 'rgba(136,136,160,0.05)',
              }}
            >
              <div style={{ color: 'var(--color-text-muted)', fontSize: '10px', fontWeight: 700, letterSpacing: '0.05em', textTransform: 'uppercase' }}>
                sort order
              </div>
              <div style={{ color: 'var(--color-text)', fontSize: '13px', lineHeight: 1.5, marginTop: '6px' }}>
                Attention and resumability first, then recent activity.
              </div>
            </div>
          </div>
        )}

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
            style={{
              padding: '16px',
              border: '1px solid var(--color-error)',
              borderRadius: '8px',
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
            style={{
              padding: '12px 14px',
              border: '1px solid rgba(255,68,68,0.28)',
              borderRadius: '8px',
              background: 'rgba(255,68,68,0.06)',
              color: 'var(--color-error)',
              fontSize: '12px',
            }}
          >
            {actionError}
          </div>
        )}

        {!loading && !fetchError && sessions.length === 0 && (
          <div
            style={{
              display: 'flex',
              flexDirection: 'column',
              alignItems: 'center',
              justifyContent: 'center',
              padding: '60px 24px',
              border: '1px dashed var(--color-border)',
              borderRadius: '8px',
              gap: '12px',
            }}
          >
            <span style={{ color: 'var(--color-text-muted)', fontSize: '13px' }}>
              {showArchived ? 'no sessions match this view' : 'no sessions yet'}
            </span>
            <span style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>
              {showArchived
                ? 'toggle archived sessions off or open project sessions to continue'
                : 'sessions are now project-scoped. open projects to start planning'}
            </span>
            <button
              onClick={() => void navigate('/projects')}
              style={{
                marginTop: '8px',
                background: 'transparent',
                border: '1px solid var(--color-primary)',
                color: 'var(--color-primary)',
                padding: '8px 20px',
                fontSize: '12px',
                cursor: 'pointer',
                borderRadius: '4px',
                fontFamily: 'inherit',
                transition: 'background 0.18s',
              }}
              onMouseEnter={(event) => {
                (event.currentTarget as HTMLButtonElement).style.background = 'rgba(0,212,255,0.08)';
              }}
              onMouseLeave={(event) => {
                (event.currentTarget as HTMLButtonElement).style.background = 'transparent';
              }}
            >
              open projects →
            </button>
          </div>
        )}

        {!loading && !fetchError && sessions.length > 0 && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
            {sortedSessions.map((session) => (
              <SessionCard
                key={session.id}
                session={session}
                onClick={() => void navigate(`/session/${session.id}`)}
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

        <div
          style={{
            padding: '14px 16px',
            background: 'var(--color-surface)',
            border: '1px solid var(--color-border)',
            borderRadius: '8px',
            fontSize: '12px',
            color: 'var(--color-text-muted)',
            lineHeight: 1.7,
          }}
        >
          <span style={{ color: 'var(--color-primary)', fontWeight: 600 }}>TIP</span>
          {' '}Sessions remain a global operational queue, while new planning work starts from projects.
        </div>
      </div>
    </Layout>
  );
}
