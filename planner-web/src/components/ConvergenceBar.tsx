import type { Classification } from '../types.ts';

interface ConvergenceBarProps {
  convergencePct: number;
  classification: Classification | null;
}

export default function ConvergenceBar({ convergencePct, classification }: ConvergenceBarProps) {
  const pct = Math.max(0, Math.min(100, convergencePct));

  const fillColor =
    pct >= 80
      ? 'var(--accent-green)'
      : pct >= 50
      ? 'var(--accent-yellow)'
      : 'var(--text-secondary)';

  const rightText = classification
    ? `${pct}% · ${classification.project_type} · ${classification.complexity}`
    : null;

  return (
    <div
      className={pct >= 80 ? 'convergence-high' : undefined}
      style={{
        height: '36px',
        background: 'var(--bg-secondary)',
        borderBottom: '1px solid var(--border)',
        display: 'flex',
        alignItems: 'center',
        flexShrink: 0,
        position: 'relative',
        overflow: 'hidden',
      }}
    >
      {/* Progress fill strip at very bottom */}
      <div
        style={{
          position: 'absolute',
          bottom: 0,
          left: 0,
          height: '4px',
          width: `${pct}%`,
          background: fillColor,
          transition: 'width 0.4s ease, background 0.3s ease',
        }}
      />

      {/* Content row */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          width: '100%',
          padding: '0 16px',
          paddingBottom: '4px', // offset for the 4px bar at bottom
        }}
      >
        {/* Left: convergence label */}
        <span
          style={{
            fontSize: '10px',
            letterSpacing: '0.08em',
            textTransform: 'uppercase',
            color: 'var(--text-secondary)',
            fontWeight: 700,
          }}
        >
          convergence
        </span>

        {/* Right: summary text */}
        {classification ? (
          <span
            style={{
              fontSize: '11px',
              color: fillColor,
              letterSpacing: '0.04em',
              fontWeight: pct >= 80 ? 700 : 400,
              transition: 'color 0.3s ease',
            }}
          >
            {rightText}
          </span>
        ) : (
          <span
            style={{
              fontSize: '11px',
              color: 'var(--text-secondary)',
              fontStyle: 'italic',
            }}
          >
            Analyzing project…
          </span>
        )}
      </div>
    </div>
  );
}
