import type { ReactNode } from 'react';

type EntryTone = 'default' | 'warning' | 'error';

interface EntryShellProps {
  badge?: string;
  kicker: string;
  title: string;
  description: string;
  actionLabel?: string;
  onAction?: () => void;
  note?: ReactNode;
  details?: ReactNode;
  tone?: EntryTone;
}

export default function EntryShell({
  badge,
  kicker,
  title,
  description,
  actionLabel,
  onAction,
  note,
  details,
  tone = 'default',
}: EntryShellProps) {
  return (
    <div className="entry-shell">
      <div className={`entry-card${tone !== 'default' ? ` entry-card-${tone}` : ''}`}>
        <div className="entry-brand">
          <div className="entry-brand-mark" aria-hidden="true">
            <svg width="22" height="22" viewBox="0 0 24 24" fill="none">
              <rect x="2" y="2" width="20" height="20" rx="4" stroke="currentColor" strokeWidth="1.5" />
              <path d="M7 8h10M7 12h7M7 16h4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
              <circle cx="18" cy="16" r="2" fill="var(--color-primary)" />
            </svg>
          </div>
          <div className="entry-brand-copy">
            <span className="entry-brand-name">Planner</span>
            {badge ? <span className="entry-badge">{badge}</span> : null}
          </div>
        </div>

        <div className="entry-copy-block">
          <span className="entry-kicker">{kicker}</span>
          <h1 className="entry-title">{title}</h1>
          <p className="entry-description">{description}</p>
        </div>

        {details ? <div className="entry-details">{details}</div> : null}

        {actionLabel && onAction ? (
          <div className="entry-action-row">
            <button className="btn btn-primary entry-action" onClick={onAction}>
              {actionLabel}
            </button>
          </div>
        ) : null}

        {note ? <div className="entry-note">{note}</div> : null}
      </div>
    </div>
  );
}
