import { useEffect, useRef, useState } from 'react';
import type { PlannerEvent, SocraticWorkspaceSnapshot } from '../types.ts';
import { formatWorkflowStep } from '../lib/workflowStatus.ts';

export interface SessionPulseBarProps {
  sessionTitle: string | null;
  currentStep: string | null;
  events: PlannerEvent[];
  isError: boolean;
  errorMessage?: string | null;
  workspace: SocraticWorkspaceSnapshot | null;
  unreadEventCount: number;
  hasDraft: boolean;
  isContextShelfOpen: boolean;
  onToggleContextShelf: () => void;
}

/** Number of LLM call completions — events from llm_router with step starting with "llm.call.complete" */
function countLlmCalls(events: PlannerEvent[]): number {
  return events.filter(
    (e) => e.source === 'llm_router' && (e.step?.startsWith('llm.call.complete') ?? false),
  ).length;
}

/** Determine status dot color:
 * - red if isError or any error event
 * - yellow if any warn event
 * - green otherwise
 */
function statusDotColor(
  events: PlannerEvent[],
  isError: boolean,
): string {
  if (isError) return 'var(--color-error)';
  const hasError = events.some((e) => e.level === 'error');
  if (hasError) return 'var(--color-error)';
  const hasWarn = events.some((e) => e.level === 'warn');
  if (hasWarn) return 'var(--color-gold)';
  return 'var(--color-success)';
}

/** Format elapsed milliseconds as a short human string: "0s", "12s", "2m 5s" */
function formatElapsed(ms: number): string {
  if (ms < 1000) return '0s';
  const s = Math.floor(ms / 1000);
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  const rem = s % 60;
  return `${m}m ${rem}s`;
}


export default function SessionPulseBar({
  sessionTitle,
  currentStep,
  events,
  isError,
  errorMessage,
  workspace,
  unreadEventCount,
  hasDraft,
  isContextShelfOpen,
  onToggleContextShelf,
}: SessionPulseBarProps) {
  // Track when the current step last changed so we can show elapsed time
  const stepStartRef = useRef<number>(Date.now());
  const prevStepRef = useRef<string | null>(null);
  const [elapsed, setElapsed] = useState(0);

  // Reset timer when step changes
  useEffect(() => {
    if (currentStep !== prevStepRef.current) {
      prevStepRef.current = currentStep;
      stepStartRef.current = Date.now();
      setElapsed(0);
    }
  }, [currentStep]);

  // Tick elapsed time every second
  useEffect(() => {
    const id = setInterval(() => {
      setElapsed(Date.now() - stepStartRef.current);
    }, 1000);
    return () => clearInterval(id);
  }, []);

  const dotColor = statusDotColor(events, isError);
  const llmCalls = countLlmCalls(events);
  const readableStep = formatWorkflowStep(currentStep);

  const readyQuestionGroups = workspace?.groups.filter(group => group.status === 'ready' || group.status === 'active').length || 0;
  const preparingQuestionGroups = workspace?.groups.filter(group => group.status === 'pending').length || 0;
  const isWorkspaceSelectionState = currentStep === 'socratic.workspace.generated'
    && !workspace?.focused_category_id;

  // Derive pulse label and main message
  const pulseLabel = isError
    ? 'Error'
    : workspace?.category_snapshot.build_ready
      ? 'Build ready'
      : isWorkspaceSelectionState
        ? 'Ready'
      : preparingQuestionGroups > 0
        ? 'Preparing'
        : readyQuestionGroups > 0
          ? 'Ready'
          : 'Waiting';
  
  const mainMessage = errorMessage
    ? errorMessage
    : readableStep
      ? `${readableStep} ${elapsed > 0 ? `(${formatElapsed(elapsed)})` : ''}`
      : workspace?.category_snapshot.build_readiness_message
        ?? 'Session idle';

  return (
    <div
      className="session-pulse-bar"
      style={{
        minHeight: '52px',
        background: 'color-mix(in srgb, var(--color-surface) 68%, transparent)',
        borderBottom: '1px solid color-mix(in srgb, var(--color-divider) 70%, transparent)',
        display: 'flex',
        alignItems: 'center',
        flexShrink: 0,
        justifyContent: 'space-between',
        padding: '10px 24px',
        gap: '16px',
        overflow: 'hidden',
        flexWrap: 'wrap',
        backdropFilter: 'blur(14px)',
        WebkitBackdropFilter: 'blur(14px)',
      }}
    >
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '12px',
          flex: 1,
          minWidth: 0,
          overflow: 'hidden',
          flexWrap: 'wrap',
        }}
      >
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '10px',
            minWidth: 0,
            flex: '1 1 320px',
          }}
        >
          <span
            title={pulseLabel}
            style={{
              width: '8px',
              height: '8px',
              borderRadius: '50%',
              background: dotColor,
              flexShrink: 0,
              boxShadow: `0 0 8px ${dotColor}`,
              transition: 'background 0.3s, box-shadow 0.3s',
            }}
          />
          <span
            style={{
              fontSize: '10px',
              fontWeight: 700,
              letterSpacing: '0.12em',
              textTransform: 'uppercase',
              color: 'var(--color-text-faint)',
              flexShrink: 0,
            }}
          >
            {pulseLabel}
          </span>
          <span
            style={{
              fontSize: '13px',
              color: 'var(--color-text)',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            }}
            title={mainMessage}
          >
            {mainMessage}
            {sessionTitle && <span style={{ color: 'var(--color-text-muted)', marginLeft: '8px' }}>({sessionTitle})</span>}
          </span>
        </div>

        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '8px',
            flexWrap: 'wrap',
          }}
        >
          <span className="session-pulse-pill">
            {readyQuestionGroups} active thread{readyQuestionGroups === 1 ? '' : 's'}
          </span>
          {preparingQuestionGroups > 0 && (
            <span className="session-pulse-pill session-pulse-pill-warm">
              {preparingQuestionGroups} preparing
            </span>
          )}
          {workspace?.branch_notice && (
            <span className="session-pulse-pill session-pulse-pill-muted">
              Branch moved
            </span>
          )}
        </div>
      </div>

      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '10px',
          flexShrink: 0,
          flexWrap: 'wrap',
        }}
      >
        <button
          type="button"
          onClick={onToggleContextShelf}
          aria-expanded={isContextShelfOpen}
          aria-label="Toggle Context Shelf"
          style={{
            background: isContextShelfOpen
              ? 'color-mix(in srgb, var(--color-primary-highlight) 88%, transparent)'
              : 'color-mix(in srgb, var(--color-surface-2) 84%, transparent)',
            boxShadow: 'inset 0 0 0 1px color-mix(in srgb, var(--color-ghost-border) 72%, transparent)',
            borderRadius: '999px',
            color: isContextShelfOpen ? 'var(--color-primary)' : 'var(--color-text-muted)',
            fontSize: '11px',
            fontWeight: 700,
            letterSpacing: '0.08em',
            textTransform: 'uppercase',
            padding: '7px 14px',
            cursor: 'pointer',
            position: 'relative',
            border: 'none',
          }}
        >
          Context
          {(unreadEventCount > 0 || hasDraft) && !isContextShelfOpen && (
            <span
              style={{
                position: 'absolute',
                top: '-4px',
                right: '-4px',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                minWidth: '16px',
                height: '16px',
                borderRadius: '50%',
                background: unreadEventCount > 0 ? 'var(--color-error)' : 'var(--color-primary)',
                color: 'var(--color-bg)',
                fontSize: '9px',
                fontWeight: 700,
                lineHeight: 1,
                padding: '0 4px',
              }}
              title={unreadEventCount > 0 ? `${unreadEventCount} unread events` : 'New draft available'}
            >
              {unreadEventCount > 0 ? unreadEventCount : '•'}
            </span>
          )}
        </button>

        <span
          style={{
            fontSize: '11px',
            color: 'var(--color-text-muted)',
            opacity: llmCalls > 0 ? 0.9 : 0.6,
            marginLeft: '4px',
          }}
        >
          <span
            style={{
              fontWeight: 700,
              color: llmCalls > 0 ? 'var(--color-primary)' : 'var(--color-text-muted)',
              marginRight: '2px',
            }}
          >
            {llmCalls}
          </span>
          LLM call{llmCalls !== 1 ? 's' : ''}
        </span>
      </div>
    </div>
  );
}
