/**
 * EventLogPanel — collapsible event log with filter chips.
 *
 * Defaults COLLAPSED to a single summary bar showing event count and
 * latest event.  Click the bar to expand and see the full scrollable
 * event list with filter chips.
 *
 * Features:
 * - Collapsed mode: thin summary bar (count + last event + error count)
 * - Expanded mode: filter chips + scrollable event rows
 * - Filter chips: All | Errors | LLM | State
 * - Each row: relative timestamp, level badge, source, step, message
 * - Color-coded by level (info=dim, warn=yellow, error=red)
 * - Expandable rows showing metadata + duration
 * - Auto-scroll to bottom on new events (unless user scrolled up)
 * - Empty state when no events
 */

import { useEffect, useRef, useState, useCallback } from 'react';
import type { PlannerEvent } from '../types.ts';

interface EventLogPanelProps {
  events: PlannerEvent[];
}

type FilterKey = 'all' | 'errors' | 'llm' | 'state';

const FILTER_LABELS: Record<FilterKey, string> = {
  all: 'All',
  errors: 'Errors',
  llm: 'LLM',
  state: 'State',
};

function applyFilter(events: PlannerEvent[], filter: FilterKey): PlannerEvent[] {
  switch (filter) {
    case 'errors':
      return events.filter((e) => e.level === 'error');
    case 'llm':
      return events.filter((e) => e.source === 'llm_router');
    case 'state':
      return events.filter((e) => e.source === 'socratic_engine' || e.source === 'pipeline');
    default:
      return events;
  }
}

/** Format a timestamp as relative time: "just now", "5s ago", "2m ago" etc. */
function relativeTime(isoTimestamp: string): string {
  const now = Date.now();
  const then = new Date(isoTimestamp).getTime();
  const diffMs = now - then;

  if (diffMs < 1000) return 'just now';
  if (diffMs < 60_000) return `${Math.floor(diffMs / 1000)}s ago`;
  if (diffMs < 3_600_000) return `${Math.floor(diffMs / 60_000)}m ago`;
  return `${Math.floor(diffMs / 3_600_000)}h ago`;
}

/** Level badge color. */
function levelColor(level: string): string {
  if (level === 'error') return 'var(--color-error)';
  if (level === 'warn') return 'var(--color-gold)';
  return 'var(--color-text-muted)';
}

/** Row text color — info rows are dim, warn are yellow, error are red. */
function rowTextColor(level: string): string {
  if (level === 'error') return 'var(--color-error)';
  if (level === 'warn') return 'var(--color-gold)';
  return 'var(--color-text-muted)';
}

/** Format source label cleanly. */
function formatSource(source: string): string {
  return source.replace(/_/g, ' ');
}

/** Render metadata entries as a simple key: value list. */
function MetadataView({ metadata }: { metadata: Record<string, unknown> }) {
  const entries = Object.entries(metadata);
  if (entries.length === 0) return null;

  return (
    <div
      style={{
        marginTop: '6px',
        padding: '6px 8px',
        background: 'var(--color-bg)',
        border: '1px solid var(--color-border)',
        borderRadius: '2px',
        display: 'flex',
        flexDirection: 'column',
        gap: '2px',
      }}
    >
      {entries.map(([key, val]) => (
        <div key={key} style={{ display: 'flex', gap: '6px', fontSize: '10px', lineHeight: '1.5' }}>
          <span style={{ color: 'var(--color-text-muted)', flexShrink: 0, minWidth: '80px' }}>
            {key}
          </span>
          <span style={{ color: 'var(--color-text)', wordBreak: 'break-all' }}>
            {typeof val === 'string' ? val : JSON.stringify(val)}
          </span>
        </div>
      ))}
    </div>
  );
}

interface EventRowProps {
  event: PlannerEvent;
  /** Timestamp relative string is re-rendered each tick via parent. */
  relTs: string;
}

function EventRow({ event, relTs }: EventRowProps) {
  const [expanded, setExpanded] = useState(false);
  const hasDetails =
    event.duration_ms !== undefined ||
    Object.keys(event.metadata).length > 0;

  const textColor = rowTextColor(event.level);
  const color = levelColor(event.level);

  return (
    <div
      onClick={() => hasDetails && setExpanded((v) => !v)}
      style={{
        padding: '5px 12px',
        borderBottom: '1px solid var(--color-border)',
        cursor: hasDetails ? 'pointer' : 'default',
        background: expanded ? 'rgba(255,255,255,0.02)' : 'transparent',
        transition: 'background 0.15s',
      }}
    >
      {/* Main row */}
      <div
        style={{
          display: 'flex',
          alignItems: 'baseline',
          gap: '8px',
          fontSize: '11px',
          flexWrap: 'nowrap',
          overflow: 'hidden',
        }}
      >
        {/* Relative timestamp */}
        <span
          style={{
            color: 'var(--color-text-muted)',
            flexShrink: 0,
            minWidth: '52px',
            fontSize: '10px',
            opacity: 0.7,
          }}
        >
          {relTs}
        </span>

        {/* Level badge */}
        <span
          style={{
            flexShrink: 0,
            fontSize: '9px',
            fontWeight: 700,
            letterSpacing: '0.06em',
            textTransform: 'uppercase',
            color,
            border: `1px solid ${color}`,
            borderRadius: '2px',
            padding: '0 4px',
            opacity: 0.9,
          }}
        >
          {event.level}
        </span>

        {/* Source */}
        <span
          style={{
            flexShrink: 0,
            color: 'var(--color-text-muted)',
            fontSize: '10px',
            opacity: 0.75,
            minWidth: '80px',
          }}
        >
          {formatSource(event.source)}
        </span>

        {/* Step (optional) */}
        {event.step && (
          <span
            style={{
              flexShrink: 0,
              color: 'var(--color-primary)',
              fontSize: '10px',
              opacity: 0.8,
              maxWidth: '120px',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            }}
            title={event.step}
          >
            {event.step}
          </span>
        )}

        {/* Message */}
        <span
          style={{
            color: textColor,
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            flex: 1,
          }}
          title={event.message}
        >
          {event.message}
        </span>

        {/* Expand indicator */}
        {hasDetails && (
          <span
            style={{
              flexShrink: 0,
              color: 'var(--color-text-muted)',
              fontSize: '9px',
              opacity: 0.5,
            }}
          >
            {expanded ? '▾' : '▸'}
          </span>
        )}
      </div>

      {/* Expanded details */}
      {expanded && (
        <div style={{ marginTop: '4px', paddingLeft: '60px' }}>
          {event.duration_ms !== undefined && (
            <div
              style={{
                fontSize: '10px',
                color: 'var(--color-text-muted)',
                marginBottom: '2px',
              }}
            >
              duration: <span style={{ color: 'var(--color-primary)' }}>{event.duration_ms}ms</span>
            </div>
          )}
          <MetadataView metadata={event.metadata} />
        </div>
      )}
    </div>
  );
}

/** Tick counter hook — re-renders every 10 seconds to refresh relative timestamps. */
function useTick(intervalMs = 10_000): number {
  const [tick, setTick] = useState(0);
  useEffect(() => {
    const id = setInterval(() => setTick((t) => t + 1), intervalMs);
    return () => clearInterval(id);
  }, [intervalMs]);
  return tick;
}

/** Full expanded event log view with filters. */
function ExpandedEventLog({ events }: { events: PlannerEvent[] }) {
  const [filter, setFilter] = useState<FilterKey>('all');
  const scrollRef = useRef<HTMLDivElement>(null);
  const userScrolledRef = useRef(false);
  const tick = useTick();

  const filtered = applyFilter(events, filter);

  // Track user scroll position to decide whether to auto-scroll
  const handleScroll = useCallback(() => {
    const el = scrollRef.current;
    if (!el) return;
    const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 40;
    userScrolledRef.current = !atBottom;
  }, []);

  // Auto-scroll to bottom when new events arrive and user hasn't scrolled up
  useEffect(() => {
    if (userScrolledRef.current) return;
    const el = scrollRef.current;
    if (el) {
      el.scrollTop = el.scrollHeight;
    }
  }, [filtered.length]);

  return (
    <div
      style={{
        flex: 1,
        display: 'flex',
        flexDirection: 'column',
        overflow: 'hidden',
        minHeight: 0,
      }}
    >
      {/* Filter chips */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '6px',
          padding: '5px 12px',
          borderBottom: '1px solid var(--color-border)',
          flexShrink: 0,
          background: 'var(--color-surface)',
        }}
      >
        <span
          style={{
            fontSize: '9px',
            fontWeight: 700,
            letterSpacing: '0.1em',
            textTransform: 'uppercase',
            color: 'var(--color-text-muted)',
            marginRight: '4px',
          }}
        >
          Filter
        </span>
        {(Object.keys(FILTER_LABELS) as FilterKey[]).map((key) => {
          const active = filter === key;
          return (
            <button
              key={key}
              onClick={() => {
                setFilter(key);
                // Reset auto-scroll on filter change
                userScrolledRef.current = false;
              }}
              style={{
                background: active ? 'var(--color-primary)' : 'transparent',
                border: `1px solid ${active ? 'var(--color-primary)' : 'var(--color-border)'}`,
                borderRadius: '2px',
                color: active ? 'var(--color-bg)' : 'var(--color-text-muted)',
                fontSize: '10px',
                fontWeight: active ? 700 : 400,
                fontFamily: 'inherit',
                padding: '2px 8px',
                cursor: 'pointer',
                transition: 'background 0.15s, border-color 0.15s, color 0.15s',
                letterSpacing: '0.03em',
              }}
            >
              {FILTER_LABELS[key]}
              {key !== 'all' && (
                <span
                  style={{
                    marginLeft: '4px',
                    fontSize: '9px',
                    opacity: 0.7,
                  }}
                >
                  ({applyFilter(events, key).length})
                </span>
              )}
            </button>
          );
        })}

        {/* Total count */}
        <span
          style={{
            marginLeft: 'auto',
            fontSize: '10px',
            color: 'var(--color-text-muted)',
            opacity: 0.6,
          }}
        >
          {filtered.length} / {events.length}
        </span>
      </div>

      {/* Event list */}
      <div
        ref={scrollRef}
        onScroll={handleScroll}
        style={{
          flex: 1,
          overflowY: 'auto',
          overscrollBehavior: 'contain',
        }}
      >
        {/* Empty state */}
        {filtered.length === 0 && (
          <div
            style={{
              padding: '32px 16px',
              display: 'flex',
              flexDirection: 'column',
              alignItems: 'center',
              justifyContent: 'center',
              gap: '8px',
            }}
          >
            <span
              style={{
                fontSize: '11px',
                color: 'var(--color-text-muted)',
                fontStyle: 'italic',
                textAlign: 'center',
              }}
            >
              {events.length === 0
                ? 'No events yet — events will appear here during session processing'
                : `No ${filter === 'all' ? '' : FILTER_LABELS[filter] + ' '}events`}
            </span>
          </div>
        )}

        {filtered.map((event) => (
          <EventRow
            key={event.id}
            event={event}
            // tick dependency ensures relative timestamps refresh every 10s
            relTs={relativeTime(event.timestamp) + (tick > -1 ? '' : '')}
          />
        ))}
      </div>
    </div>
  );
}

/**
 * Collapsible event log panel.
 *
 * Collapsed (default): single 28px summary bar.
 * Expanded: full filter + event list, capped at 50% of available height.
 */
export default function EventLogPanel({ events }: EventLogPanelProps) {
  const [expanded, setExpanded] = useState(false);

  const errorCount = events.filter((e) => e.level === 'error').length;
  const warnCount = events.filter((e) => e.level === 'warn').length;
  const lastEvent = events.length > 0 ? events[events.length - 1] : null;

  return (
    <div
      style={{
        borderTop: '1px solid var(--color-border)',
        background: 'var(--color-surface)',
        display: 'flex',
        flexDirection: 'column',
        flexShrink: 0,
        // When expanded, take up to 50% of the right panel
        ...(expanded ? { flex: 1, maxHeight: '50%', minHeight: '120px' } : {}),
        overflow: 'hidden',
        transition: 'max-height 0.2s ease',
      }}
    >
      {/* Summary bar — always visible, acts as toggle */}
      <div
        onClick={() => setExpanded((v) => !v)}
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '8px',
          padding: '0 12px',
          height: '28px',
          flexShrink: 0,
          cursor: 'pointer',
          borderBottom: expanded ? '1px solid var(--color-border)' : 'none',
          transition: 'background 0.15s',
        }}
        onMouseEnter={(e) => {
          (e.currentTarget as HTMLDivElement).style.background = 'rgba(255,255,255,0.02)';
        }}
        onMouseLeave={(e) => {
          (e.currentTarget as HTMLDivElement).style.background = 'transparent';
        }}
        title={expanded ? 'Collapse events' : 'Expand events'}
      >
        {/* Toggle indicator */}
        <span
          style={{
            fontSize: '9px',
            color: 'var(--color-text-muted)',
            opacity: 0.6,
            flexShrink: 0,
            transition: 'transform 0.2s',
            transform: expanded ? 'rotate(90deg)' : 'rotate(0deg)',
          }}
        >
          ▸
        </span>

        {/* Events label + count */}
        <span
          style={{
            fontSize: '10px',
            fontWeight: 700,
            letterSpacing: '0.08em',
            textTransform: 'uppercase',
            color: 'var(--color-text-muted)',
          }}
        >
          Events
        </span>
        <span
          style={{
            fontSize: '10px',
            color: 'var(--color-text-muted)',
            opacity: 0.7,
          }}
        >
          {events.length}
        </span>

        {/* Error/warn badges */}
        {errorCount > 0 && (
          <span
            style={{
              fontSize: '9px',
              fontWeight: 700,
              color: 'var(--color-error)',
              border: '1px solid var(--color-error)',
              borderRadius: '2px',
              padding: '0 4px',
              letterSpacing: '0.04em',
            }}
          >
            {errorCount} err
          </span>
        )}
        {warnCount > 0 && (
          <span
            style={{
              fontSize: '9px',
              fontWeight: 700,
              color: 'var(--color-gold)',
              border: '1px solid var(--color-gold)',
              borderRadius: '2px',
              padding: '0 4px',
              letterSpacing: '0.04em',
            }}
          >
            {warnCount} warn
          </span>
        )}

        {/* Latest event preview (collapsed only) */}
        {!expanded && lastEvent && (
          <span
            style={{
              flex: 1,
              fontSize: '10px',
              color: rowTextColor(lastEvent.level),
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
              opacity: 0.7,
              marginLeft: '4px',
            }}
            title={lastEvent.message}
          >
            {lastEvent.message}
          </span>
        )}
      </div>

      {/* Expanded event list */}
      {expanded && <ExpandedEventLog events={events} />}
    </div>
  );
}
