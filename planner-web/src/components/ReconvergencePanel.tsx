import { useState, useCallback } from 'react';
import type { ReconvergenceResult, ReconvergenceStep } from '../types/blueprint.ts';

interface ReconvergencePanelProps {
  result: ReconvergenceResult | null;
  loading: boolean;
  onClose: () => void;
  /** Called when user approves a pending (deep severity) step. */
  onApproveStep?: (stepId: string) => void;
  /** Called when user skips a pending step. */
  onSkipStep?: (stepId: string) => void;
}

// ─── Status icons ──────────────────────────────────────────────────────────

function StepStatusIcon({ status }: { status: string }) {
  switch (status) {
    case 'done':
      return (
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--color-success)" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
          <polyline points="20 6 9 17 4 12"/>
        </svg>
      );
    case 'running':
      return (
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--color-primary)" strokeWidth="2" strokeLinecap="round">
          <circle cx="12" cy="12" r="10" strokeDasharray="31 31" strokeDashoffset="0">
            <animateTransform attributeName="transform" type="rotate" from="0 12 12" to="360 12 12" dur="1s" repeatCount="indefinite"/>
          </circle>
        </svg>
      );
    case 'pending':
      return (
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--color-warning)" strokeWidth="2" strokeLinecap="round">
          <circle cx="12" cy="12" r="10"/>
          <path d="M12 8v4M12 16h.01"/>
        </svg>
      );
    case 'skipped':
      return (
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--color-text-faint)" strokeWidth="2" strokeLinecap="round">
          <circle cx="12" cy="12" r="10"/>
          <path d="M9 9l6 6M15 9l-6 6"/>
        </svg>
      );
    case 'error':
      return (
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--color-error)" strokeWidth="2" strokeLinecap="round">
          <circle cx="12" cy="12" r="10"/>
          <path d="M12 8v4M12 16h.01"/>
        </svg>
      );
    default:
      return (
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--color-text-faint)" strokeWidth="2">
          <circle cx="12" cy="12" r="4"/>
        </svg>
      );
  }
}

// ─── Severity badge ────────────────────────────────────────────────────────

function SeverityBadge({ severity }: { severity: string }) {
  const color =
    severity === 'deep' ? 'var(--color-error)' :
    severity === 'medium' ? 'var(--color-warning)' :
    'var(--color-text-faint)';

  return (
    <span style={{
      fontSize: '0.6rem',
      fontWeight: 700,
      textTransform: 'uppercase',
      letterSpacing: '0.05em',
      color,
      border: `1px solid ${color}`,
      borderRadius: 'var(--radius-sm)',
      padding: '1px 4px',
    }}>
      {severity}
    </span>
  );
}

// ─── Step row ──────────────────────────────────────────────────────────────

function StepRow({
  step,
  onApprove,
  onSkip,
}: {
  step: ReconvergenceStep;
  onApprove?: (stepId: string) => void;
  onSkip?: (stepId: string) => void;
}) {
  const isPending = step.status === 'pending';
  const isDeep = step.severity === 'deep';

  return (
    <div className={`recon-step recon-step--${step.status}`}>
      <div className="recon-step-icon">
        <StepStatusIcon status={step.status} />
      </div>
      <div className="recon-step-body">
        <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)', flexWrap: 'wrap' }}>
          <span style={{ fontWeight: 600, fontSize: 'var(--text-sm)' }}>
            {step.node_name}
          </span>
          <span style={{
            fontSize: '0.625rem', color: 'var(--color-text-faint)',
            textTransform: 'uppercase', letterSpacing: '0.04em',
          }}>
            {step.node_type}
          </span>
          <span style={{
            fontSize: '0.65rem', fontWeight: 600,
            color: step.action === 'reconverge' ? 'var(--color-gold)' :
                   step.action === 'remove' || step.action === 'invalidate' ? 'var(--color-error)' :
                   'var(--color-primary)',
          }}>
            [{step.action.toUpperCase()}]
          </span>
          <SeverityBadge severity={step.severity} />
        </div>
        <div style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-muted)', marginTop: '2px' }}>
          {step.description}
        </div>
        {step.error && (
          <div style={{ fontSize: 'var(--text-xs)', color: 'var(--color-error)', marginTop: '4px' }}>
            Error: {step.error}
          </div>
        )}
        {/* Approval controls for pending deep-severity steps */}
        {isPending && isDeep && (onApprove || onSkip) && (
          <div style={{ display: 'flex', gap: 'var(--space-2)', marginTop: 'var(--space-2)' }}>
            {onApprove && (
              <button
                className="btn btn-primary"
                style={{ fontSize: '0.65rem', padding: '2px 8px' }}
                onClick={() => onApprove(step.step_id)}
              >
                Approve
              </button>
            )}
            {onSkip && (
              <button
                className="btn btn-ghost"
                style={{ fontSize: '0.65rem', padding: '2px 8px' }}
                onClick={() => onSkip(step.step_id)}
              >
                Skip
              </button>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

// ─── Main component ────────────────────────────────────────────────────────

export default function ReconvergencePanel({
  result,
  loading,
  onClose,
  onApproveStep,
  onSkipStep,
}: ReconvergencePanelProps) {
  // Local state for optimistic step status updates
  const [localOverrides, setLocalOverrides] = useState<Record<string, string>>({});

  const handleApprove = useCallback((stepId: string) => {
    setLocalOverrides(prev => ({ ...prev, [stepId]: 'done' }));
    onApproveStep?.(stepId);
  }, [onApproveStep]);

  const handleSkip = useCallback((stepId: string) => {
    setLocalOverrides(prev => ({ ...prev, [stepId]: 'skipped' }));
    onSkipStep?.(stepId);
  }, [onSkipStep]);

  // Merge server result with local overrides
  const steps: ReconvergenceStep[] = result
    ? result.steps.map(s => ({
        ...s,
        status: (localOverrides[s.step_id] ?? s.status) as ReconvergenceStep['status'],
      }))
    : [];

  // Recompute summary from (possibly overridden) steps
  const summary = {
    total: steps.length,
    applied: steps.filter(s => s.status === 'done').length,
    skipped: steps.filter(s => s.status === 'skipped').length,
    errors: steps.filter(s => s.status === 'error').length,
    needs_review: steps.filter(s => s.status === 'pending').length,
  };

  const allResolved = summary.needs_review === 0 && !loading;

  return (
    <div className="recon-panel">
      {/* Header */}
      <div style={{
        display: 'flex', alignItems: 'center', justifyContent: 'space-between',
        padding: 'var(--space-3) var(--space-4)',
        borderBottom: '1px solid var(--color-divider)',
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)' }}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--color-gold)" strokeWidth="2" strokeLinecap="round">
            <path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"/>
          </svg>
          <span style={{ fontWeight: 700, fontSize: 'var(--text-sm)' }}>Reconvergence</span>
          {loading && <span style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-faint)' }}>running...</span>}
        </div>
        <button
          className="drawer-close"
          onClick={onClose}
          aria-label="Close reconvergence panel"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
            <path d="M18 6L6 18M6 6l12 12"/>
          </svg>
        </button>
      </div>

      {/* Summary bar */}
      {result && (
        <div className="recon-summary">
          <div className="recon-summary-stat">
            <span style={{ color: 'var(--color-text-faint)' }}>Total</span>
            <span style={{ fontWeight: 700 }}>{summary.total}</span>
          </div>
          <div className="recon-summary-stat">
            <span style={{ color: 'var(--color-success)' }}>Applied</span>
            <span style={{ fontWeight: 700, color: 'var(--color-success)' }}>{summary.applied}</span>
          </div>
          <div className="recon-summary-stat">
            <span style={{ color: 'var(--color-warning)' }}>Review</span>
            <span style={{ fontWeight: 700, color: 'var(--color-warning)' }}>{summary.needs_review}</span>
          </div>
          <div className="recon-summary-stat">
            <span style={{ color: 'var(--color-text-faint)' }}>Skipped</span>
            <span style={{ fontWeight: 700 }}>{summary.skipped}</span>
          </div>
          {summary.errors > 0 && (
            <div className="recon-summary-stat">
              <span style={{ color: 'var(--color-error)' }}>Errors</span>
              <span style={{ fontWeight: 700, color: 'var(--color-error)' }}>{summary.errors}</span>
            </div>
          )}
        </div>
      )}

      {/* Loading spinner */}
      {loading && !result && (
        <div style={{
          display: 'flex', alignItems: 'center', justifyContent: 'center',
          padding: 'var(--space-6)',
        }}>
          <div className="skeleton-pulse" />
        </div>
      )}

      {/* Step list */}
      <div style={{ flex: 1, overflowY: 'auto', padding: 'var(--space-3) var(--space-4)' }}>
        {steps.map(step => (
          <StepRow
            key={step.step_id}
            step={step}
            onApprove={handleApprove}
            onSkip={handleSkip}
          />
        ))}
      </div>

      {/* Footer */}
      {allResolved && (
        <div style={{
          padding: 'var(--space-3) var(--space-4)',
          borderTop: '1px solid var(--color-divider)',
          display: 'flex', alignItems: 'center', justifyContent: 'space-between',
        }}>
          <span style={{ fontSize: 'var(--text-xs)', color: 'var(--color-success)' }}>
            All steps resolved
          </span>
          <button className="btn btn-primary" onClick={onClose} style={{ fontSize: 'var(--text-xs)' }}>
            Done
          </button>
        </div>
      )}
    </div>
  );
}
