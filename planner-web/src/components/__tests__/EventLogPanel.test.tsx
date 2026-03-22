import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import EventLogPanel from '../EventLogPanel.tsx';
import type { PlannerEvent } from '../../types.ts';

function makeEvent(overrides: Partial<PlannerEvent>): PlannerEvent {
  return {
    id: 'evt-default',
    timestamp: '2026-03-22T12:00:00Z',
    level: 'info',
    source: 'pipeline',
    message: 'Compiled project blueprint',
    metadata: {},
    ...overrides,
  };
}

describe('EventLogPanel', () => {
  beforeEach(() => {
    vi.spyOn(Date, 'now').mockReturnValue(new Date('2026-03-22T12:05:00Z').getTime());
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('expands, filters, and reveals event metadata without losing the summary bar', async () => {
    const user = userEvent.setup();
    render(
      <EventLogPanel
        events={[
          makeEvent({
            id: 'evt-error',
            timestamp: '2026-03-22T12:04:00Z',
            level: 'error',
            source: 'llm_router',
            message: 'Pipeline failed to start',
            duration_ms: 220,
            metadata: { retryable: true, run_id: 'run-123' },
          }),
          makeEvent({
            id: 'evt-info',
            timestamp: '2026-03-22T12:05:00Z',
            level: 'info',
            source: 'pipeline',
            message: 'Compiled project blueprint',
          }),
        ]}
      />,
    );

    expect(screen.getByText('Events')).toBeInTheDocument();
    expect(screen.getByText('Compiled project blueprint')).toBeInTheDocument();

    await user.click(screen.getByTitle('Expand events'));

    expect(screen.getByText('Filter')).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: /Errors\s*\(1\)/i }));

    expect(screen.getByText('Pipeline failed to start')).toBeInTheDocument();
    expect(screen.queryByText('Compiled project blueprint')).not.toBeInTheDocument();

    await user.click(screen.getByText('Pipeline failed to start'));

    expect(screen.getByText('duration:')).toBeInTheDocument();
    expect(screen.getByText('220ms')).toBeInTheDocument();
    expect(screen.getByText('retryable')).toBeInTheDocument();
    expect(screen.getByText('run_id')).toBeInTheDocument();
  });
});
