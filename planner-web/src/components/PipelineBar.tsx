import type { PipelineStage, StageStatus } from '../types.ts';

interface PipelineBarProps {
  stages: PipelineStage[];
}

const STATUS_COLORS: Record<StageStatus, string> = {
  pending: 'var(--text-secondary)',
  running: 'var(--accent-yellow)',
  complete: 'var(--accent-green)',
  failed: 'var(--accent-red)',
};

const STATUS_BG: Record<StageStatus, string> = {
  pending: 'transparent',
  running: 'rgba(255,215,0,0.08)',
  complete: 'rgba(0,255,136,0.06)',
  failed: 'rgba(255,68,68,0.08)',
};

export default function PipelineBar({ stages }: PipelineBarProps) {
  return (
    <div style={{
      display: 'flex',
      alignItems: 'stretch',
      padding: '0 12px',
      background: 'var(--bg-secondary)',
      borderTop: '1px solid var(--border)',
      height: '48px',
      flexShrink: 0,
      overflowX: 'auto',
    }}>
      {stages.map((stage, i) => (
        <div key={stage.name} style={{ display: 'flex', alignItems: 'center' }}>
          <StageChip stage={stage} />
          {i < stages.length - 1 && (
            <span style={{
              color: 'var(--border)',
              fontSize: '12px',
              margin: '0 2px',
              userSelect: 'none',
            }}>
              ›
            </span>
          )}
        </div>
      ))}
    </div>
  );
}

function StageChip({ stage }: { stage: PipelineStage }) {
  const color = STATUS_COLORS[stage.status];
  const bg = STATUS_BG[stage.status];
  const isRunning = stage.status === 'running';

  return (
    <div style={{
      display: 'flex',
      alignItems: 'center',
      gap: '5px',
      padding: '0 6px',
      height: '100%',
      background: bg,
    }}>
      {/* Dot indicator */}
      <span
        className={isRunning ? 'pulse' : undefined}
        style={{
          width: '6px',
          height: '6px',
          borderRadius: '50%',
          background: color,
          flexShrink: 0,
          display: 'inline-block',
        }}
      />
      {/* Stage name */}
      <span style={{
        fontSize: '10px',
        color,
        letterSpacing: '0.04em',
        fontWeight: isRunning ? 700 : 400,
        whiteSpace: 'nowrap',
        textTransform: 'uppercase',
      }}>
        {stage.name}
      </span>
    </div>
  );
}
