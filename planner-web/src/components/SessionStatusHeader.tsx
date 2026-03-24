import { useEffect, useRef, useState } from 'react';
import type { PlannerEvent } from '../types.ts';
import { formatWorkflowStep } from '../lib/workflowStatus.ts';

export type SessionHeaderActionTone = 'default' | 'primary' | 'danger';

export interface SessionHeaderAction {
  key: string;
  label: string;
  onClick: () => void;
  disabled?: boolean;
  title?: string;
  tone?: SessionHeaderActionTone;
  isPrimary?: boolean;
}

export interface SessionEventSummary {
  total: number;
  warnings: number;
  errors: number;
  unread: number;
}

export interface SessionStatusHeaderProps {
  sessionTitle?: string | null;
  sessionId?: string | null;
  isArchived?: boolean;
  currentStep: string | null;
  events: PlannerEvent[];
  isError: boolean;
  errorMessage?: string | null;
  actions?: SessionHeaderAction[];
  eventSummary?: SessionEventSummary;
  onOpenEvents?: () => void;
}

function statusDotColor(events: PlannerEvent[], isError: boolean): string {
  if (isError) return 'var(--color-error)';
  const hasError = events.some((e) => e.level === 'error');
  if (hasError) return 'var(--color-error)';
  const hasWarn = events.some((e) => e.level === 'warn');
  if (hasWarn) return 'var(--color-gold)';
  return 'var(--color-success)';
}

function formatElapsed(ms: number): string {
  if (ms < 1000) return '0s';
  const s = Math.floor(ms / 1000);
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  const rem = s % 60;
  return `${m}m ${rem}s`;
}

export default function SessionStatusHeader({
  sessionTitle,
  isArchived,
  currentStep,
  events,
  isError,
  errorMessage,
  actions = [],
  eventSummary,
  onOpenEvents,
}: SessionStatusHeaderProps) {
  const stepStartRef = useRef<number>(Date.now());
  const prevStepRef = useRef<string | null>(null);
  const [elapsed, setElapsed] = useState(0);

  useEffect(() => {
    if (currentStep !== prevStepRef.current) {
      prevStepRef.current = currentStep;
      stepStartRef.current = Date.now();
      setElapsed(0);
    }
  }, [currentStep]);

  useEffect(() => {
    const id = setInterval(() => {
      setElapsed(Date.now() - stepStartRef.current);
    }, 1000);
    return () => clearInterval(id);
  }, []);

  const dotColor = statusDotColor(events, isError);
  const readableStep = formatWorkflowStep(currentStep);

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '12px 24px',
        background: 'rgba(20, 20, 22, 0.65)',
        backdropFilter: 'blur(20px)',
        WebkitBackdropFilter: 'blur(20px)',
        borderBottom: '1px solid rgba(255, 255, 255, 0.08)',
        boxShadow: '0 4px 24px rgba(0,0,0,0.2)',
        position: 'sticky',
        top: 0,
        zIndex: 50,
        gap: '16px',
        flexWrap: 'wrap',
      }}
    >
      {/* LEFT: Title & Status */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '16px', flex: 1, minWidth: 0 }}>
        {sessionTitle && (
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <span style={{ color: 'var(--color-text)', fontSize: '15px', fontWeight: 600, letterSpacing: '0.02em', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis', maxWidth: '300px' }}>
              {sessionTitle}
            </span>
            {isArchived && (
              <span style={{ background: 'rgba(255,255,255,0.1)', color: 'var(--color-text-muted)', fontSize: '10px', fontWeight: 700, padding: '3px 8px', borderRadius: '999px', textTransform: 'uppercase', letterSpacing: '0.05em' }}>
                Archived
              </span>
            )}
          </div>
        )}

        <div style={{ width: '1px', height: '16px', background: 'rgba(255,255,255,0.1)' }} />

        <div style={{ display: 'flex', alignItems: 'center', gap: '8px', overflow: 'hidden' }}>
          <span
            style={{
              width: '8px',
              height: '8px',
              borderRadius: '50%',
              background: dotColor,
              boxShadow: `0 0 8px ${dotColor}`,
              flexShrink: 0,
            }}
          />
          {isError && errorMessage ? (
            <span style={{ fontSize: '12px', color: 'var(--color-error)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
              {errorMessage}
            </span>
          ) : (
            <span style={{ fontSize: '12px', color: 'var(--color-text-muted)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
              <span style={{ color: 'var(--color-text)' }}>{readableStep || 'Idle'}</span>
              {readableStep && elapsed > 0 && <span style={{ opacity: 0.6, marginLeft: '6px' }}>{formatElapsed(elapsed)}</span>}
            </span>
          )}
        </div>
      </div>

      {/* RIGHT: Actions */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
        {onOpenEvents && (
          <button
            type="button"
            onClick={onOpenEvents}
            title="Open Events tab"
            style={{
              background: 'transparent',
              border: '1px solid rgba(255,255,255,0.1)',
              color: 'var(--color-text-muted)',
              padding: '6px 14px',
              borderRadius: '999px',
              fontSize: '11px',
              fontWeight: 600,
              cursor: 'pointer',
              display: 'inline-flex',
              alignItems: 'center',
              gap: '6px',
            }}
          >
            <span>Events {eventSummary?.total ?? events.length}</span>
            {(eventSummary?.unread ?? 0) > 0 && (
              <span style={{ color: 'var(--color-primary)' }}>{(eventSummary?.unread ?? 0)}</span>
            )}
          </button>
        )}
        {actions.map((action) => {
          const isPrimary = action.isPrimary || action.tone === 'primary';
          const isDanger = action.tone === 'danger';
          const bg = isPrimary ? 'var(--color-primary)' : isDanger ? 'rgba(255,68,68,0.15)' : 'rgba(255,255,255,0.05)';
          const text = isPrimary ? 'var(--color-bg)' : isDanger ? 'var(--color-error)' : 'var(--color-text)';
          return (
            <button
              key={action.key}
              onClick={action.onClick}
              disabled={action.disabled}
              title={action.title}
              style={{
                background: bg,
                color: text,
                border: isPrimary || isDanger ? 'none' : '1px solid rgba(255,255,255,0.1)',
                padding: '6px 14px',
                borderRadius: '999px',
                fontSize: '11px',
                fontWeight: 600,
                cursor: action.disabled ? 'not-allowed' : 'pointer',
                opacity: action.disabled ? 0.5 : 1,
                transition: 'all 0.2s',
              }}
              onMouseOver={(e) => {
                if (!action.disabled) {
                  e.currentTarget.style.transform = 'translateY(-1px)';
                  if (!isPrimary && !isDanger) e.currentTarget.style.background = 'rgba(255,255,255,0.1)';
                }
              }}
              onMouseOut={(e) => {
                if (!action.disabled) {
                  e.currentTarget.style.transform = 'translateY(0)';
                  if (!isPrimary && !isDanger) e.currentTarget.style.background = bg;
                }
              }}
            >
              {action.label}
            </button>
          );
        })}
      </div>
    </div>
  );
}
