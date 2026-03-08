import { Fragment, useMemo, useState } from 'react';
import type { EventLevel, EventSourceType, PlannerEvent } from '../types.ts';

type LevelFilter = 'all' | 'error' | 'warn';
type SourceFilter = 'all' | EventSourceType;

interface SessionEventsTableProps {
  events: PlannerEvent[];
}

const SOURCE_LABELS: Record<EventSourceType, string> = {
  socratic_engine: 'Socratic',
  llm_router: 'LLM',
  pipeline: 'Pipeline',
  factory: 'Factory',
  system: 'System',
};

function levelBadgeClass(level: EventLevel): string {
  if (level === 'error') return 'session-events-badge level-error';
  if (level === 'warn') return 'session-events-badge level-warn';
  return 'session-events-badge level-info';
}

function formatTimestamp(ts: string): { short: string; full: string } {
  const date = new Date(ts);
  if (Number.isNaN(date.getTime())) {
    return { short: ts, full: ts };
  }
  return {
    short: date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' }),
    full: date.toLocaleString(),
  };
}

function formatDuration(durationMs?: number): string {
  if (durationMs === undefined) return '';
  if (durationMs < 1000) return `${durationMs}ms`;
  const seconds = durationMs / 1000;
  return `${seconds.toFixed(seconds >= 10 ? 0 : 1)}s`;
}

export default function SessionEventsTable({ events }: SessionEventsTableProps) {
  const [levelFilter, setLevelFilter] = useState<LevelFilter>('all');
  const [sourceFilter, setSourceFilter] = useState<SourceFilter>('all');
  const [expandedIds, setExpandedIds] = useState<Set<string>>(new Set());

  const filteredEvents = useMemo(() => {
    const filtered = events.filter((event) => {
      if (levelFilter !== 'all' && event.level !== levelFilter) {
        return false;
      }
      if (sourceFilter !== 'all' && event.source !== sourceFilter) {
        return false;
      }
      return true;
    });

    return [...filtered].sort((left, right) => (
      new Date(right.timestamp).getTime() - new Date(left.timestamp).getTime()
    ));
  }, [events, levelFilter, sourceFilter]);

  const toggleExpanded = (eventId: string): void => {
    setExpandedIds((previous) => {
      const next = new Set(previous);
      if (next.has(eventId)) {
        next.delete(eventId);
      } else {
        next.add(eventId);
      }
      return next;
    });
  };

  return (
    <div className="session-events-table-shell">
      <div className="session-events-toolbar">
        <label className="session-events-filter">
          <span>Level</span>
          <select
            aria-label="Event level filter"
            value={levelFilter}
            onChange={(event) => setLevelFilter(event.target.value as LevelFilter)}
          >
            <option value="all">All</option>
            <option value="error">Errors</option>
            <option value="warn">Warnings</option>
          </select>
        </label>

        <label className="session-events-filter">
          <span>Source</span>
          <select
            aria-label="Event source filter"
            value={sourceFilter}
            onChange={(event) => setSourceFilter(event.target.value as SourceFilter)}
          >
            <option value="all">All</option>
            <option value="socratic_engine">Socratic</option>
            <option value="llm_router">LLM</option>
            <option value="pipeline">Pipeline</option>
            <option value="factory">Factory</option>
            <option value="system">System</option>
          </select>
        </label>
      </div>

      {filteredEvents.length === 0 ? (
        <div className="session-events-empty">No events yet. Live session activity will appear here.</div>
      ) : (
        <div className="session-events-table-wrap">
          <table className="session-events-table">
            <thead>
              <tr>
                <th>Time</th>
                <th>Level</th>
                <th>Source</th>
                <th>Step</th>
                <th>Message</th>
                <th>Duration</th>
              </tr>
            </thead>
            <tbody>
              {filteredEvents.map((event) => {
                const ts = formatTimestamp(event.timestamp);
                const expanded = expandedIds.has(event.id);
                const hasDetails = event.duration_ms !== undefined || Object.keys(event.metadata).length > 0;

                return (
                  <Fragment key={event.id}>
                    <tr
                      className={hasDetails ? 'expandable' : undefined}
                      onClick={hasDetails ? () => toggleExpanded(event.id) : undefined}
                    >
                      <td title={ts.full}>{ts.short}</td>
                      <td>
                        <span className={levelBadgeClass(event.level)}>{event.level}</span>
                      </td>
                      <td>{SOURCE_LABELS[event.source]}</td>
                      <td title={event.step ?? ''}>{event.step ?? '—'}</td>
                      <td title={event.message}>{event.message}</td>
                      <td>{formatDuration(event.duration_ms)}</td>
                    </tr>
                    {expanded && (
                      <tr className="session-events-detail-row">
                        <td colSpan={6}>
                          <div className="session-events-details">
                            <div>
                              <strong>Full message:</strong> {event.message}
                            </div>
                            {event.duration_ms !== undefined && (
                              <div>
                                <strong>Duration:</strong> {event.duration_ms}ms
                              </div>
                            )}
                            {Object.keys(event.metadata).length > 0 && (
                              <pre>{JSON.stringify(event.metadata, null, 2)}</pre>
                            )}
                          </div>
                        </td>
                      </tr>
                    )}
                  </Fragment>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
