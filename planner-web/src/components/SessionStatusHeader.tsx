/**
 * SessionStatusHeader — thin 28px status bar that sits above ConvergenceBar/PipelineBar.
 *
 * Shows:
 * - Left: status dot (green/yellow/red) + current step + elapsed time since step started
 * - Middle: LLM call count
 * - Right: [Logs] toggle button
 * - Error state: shows error message in red
 */

import { useEffect, useRef, useState } from 'react';
import type { PlannerEvent } from '../types.ts';

export interface SessionStatusHeaderProps {
  currentStep: string | null;
  events: PlannerEvent[];
  isError: boolean;
  errorMessage?: string | null;
  onToggleLogs: () => void;
  showLogs: boolean;
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
  if (isError) return 'var(--accent-red)';
  const hasError = events.some((e) => e.level === 'error');
  if (hasError) return 'var(--accent-red)';
  const hasWarn = events.some((e) => e.level === 'warn');
  if (hasWarn) return 'var(--accent-yellow)';
  return 'var(--accent-green)';
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

export default function SessionStatusHeader({
  currentStep,
  events,
  isError,
  errorMessage,
  onToggleLogs,
  showLogs,
}: SessionStatusHeaderProps) {
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

  return (
    <div
      style={{
        height: '28px',
        background: 'var(--bg-secondary)',
        borderBottom: '1px solid var(--border)',
        display: 'flex',
        alignItems: 'center',
        flexShrink: 0,
        padding: '0 12px',
        gap: '10px',
        overflow: 'hidden',
      }}
    >
      {/* ── Left: status dot + step + elapsed ── */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '7px',
          flex: 1,
          minWidth: 0,
          overflow: 'hidden',
        }}
      >
        {/* Status dot */}
        <span
          title={isError ? 'Error' : dotColor === 'var(--accent-yellow)' ? 'Warning' : 'OK'}
          style={{
            width: '6px',
            height: '6px',
            borderRadius: '50%',
            background: dotColor,
            flexShrink: 0,
            boxShadow: `0 0 4px ${dotColor}`,
            transition: 'background 0.3s',
          }}
        />

        {/* Error message (if error state) */}
        {isError && errorMessage ? (
          <span
            style={{
              fontSize: '11px',
              color: 'var(--accent-red)',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            }}
            title={errorMessage}
          >
            {errorMessage}
          </span>
        ) : (
          <>
            {/* Current step */}
            {currentStep ? (
              <span
                style={{
                  fontSize: '11px',
                  color: 'var(--text-secondary)',
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                  whiteSpace: 'nowrap',
                }}
                title={currentStep}
              >
                <span
                  style={{
                    fontWeight: 700,
                    color: 'var(--accent-cyan)',
                    marginRight: '3px',
                  }}
                >
                  {currentStep}
                </span>
              </span>
            ) : (
              <span
                style={{
                  fontSize: '11px',
                  color: 'var(--text-secondary)',
                  fontStyle: 'italic',
                  opacity: 0.6,
                }}
              >
                idle
              </span>
            )}

            {/* Elapsed */}
            {currentStep && elapsed > 0 && (
              <span
                style={{
                  fontSize: '10px',
                  color: 'var(--text-secondary)',
                  opacity: 0.55,
                  flexShrink: 0,
                }}
              >
                {formatElapsed(elapsed)}
              </span>
            )}
          </>
        )}
      </div>

      {/* ── Middle: LLM call counter ── */}
      <div
        style={{
          flexShrink: 0,
          display: 'flex',
          alignItems: 'center',
          gap: '4px',
        }}
      >
        <span
          style={{
            fontSize: '10px',
            color: llmCalls > 0 ? 'var(--text-secondary)' : 'var(--text-secondary)',
            opacity: llmCalls > 0 ? 0.9 : 0.4,
          }}
        >
          <span
            style={{
              fontWeight: 700,
              color: llmCalls > 0 ? 'var(--accent-cyan)' : 'var(--text-secondary)',
              marginRight: '2px',
            }}
          >
            {llmCalls}
          </span>
          LLM call{llmCalls !== 1 ? 's' : ''}
        </span>
      </div>

      {/* ── Right: Logs toggle ── */}
      <button
        onClick={onToggleLogs}
        title={showLogs ? 'Hide logs' : 'Show logs'}
        style={{
          flexShrink: 0,
          background: showLogs ? 'var(--accent-cyan)' : 'transparent',
          border: `1px solid ${showLogs ? 'var(--accent-cyan)' : 'var(--border)'}`,
          borderRadius: '2px',
          color: showLogs ? 'var(--bg-primary)' : 'var(--text-secondary)',
          fontSize: '10px',
          fontWeight: showLogs ? 700 : 400,
          fontFamily: 'inherit',
          padding: '2px 8px',
          cursor: 'pointer',
          letterSpacing: '0.04em',
          transition: 'background 0.15s, border-color 0.15s, color 0.15s',
          height: '18px',
          display: 'flex',
          alignItems: 'center',
        }}
      >
        Logs
        {events.length > 0 && (
          <span
            style={{
              marginLeft: '4px',
              fontSize: '9px',
              opacity: 0.75,
            }}
          >
            {events.length}
          </span>
        )}
      </button>
    </div>
  );
}
