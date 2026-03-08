import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import SessionEventsTable from '../SessionEventsTable.tsx';
import type { PlannerEvent } from '../../types.ts';

function makeEvent(overrides: Partial<PlannerEvent>): PlannerEvent {
  return {
    id: 'evt-default',
    timestamp: '2026-03-07T12:00:00Z',
    level: 'info',
    source: 'system',
    message: 'Default event',
    metadata: {},
    ...overrides,
  };
}

describe('SessionEventsTable', () => {
  it('sorts events newest-first', () => {
    render(
      <SessionEventsTable
        events={[
          makeEvent({ id: 'older', timestamp: '2026-03-07T10:00:00Z', message: 'Older event' }),
          makeEvent({ id: 'newer', timestamp: '2026-03-07T11:00:00Z', message: 'Newer event' }),
        ]}
      />,
    );

    const rows = screen.getAllByRole('row');
    expect(rows[1]).toHaveTextContent('Newer event');
    expect(rows[2]).toHaveTextContent('Older event');
  });

  it('applies level and source filters', async () => {
    const user = userEvent.setup();
    render(
      <SessionEventsTable
        events={[
          makeEvent({ id: 'pipeline-warn', level: 'warn', source: 'pipeline', message: 'Pipeline warning' }),
          makeEvent({ id: 'llm-error', level: 'error', source: 'llm_router', message: 'LLM failure' }),
        ]}
      />,
    );

    await user.selectOptions(screen.getByRole('combobox', { name: /event level filter/i }), 'error');
    expect(screen.getByText('LLM failure')).toBeInTheDocument();
    expect(screen.queryByText('Pipeline warning')).not.toBeInTheDocument();

    await user.selectOptions(screen.getByRole('combobox', { name: /event level filter/i }), 'all');
    await user.selectOptions(screen.getByRole('combobox', { name: /event source filter/i }), 'pipeline');
    expect(screen.getByText('Pipeline warning')).toBeInTheDocument();
    expect(screen.queryByText('LLM failure')).not.toBeInTheDocument();
  });

  it('expands rows with metadata and duration', async () => {
    const user = userEvent.setup();
    render(
      <SessionEventsTable
        events={[
          makeEvent({
            id: 'expandable',
            message: 'Expandable event',
            duration_ms: 245,
            metadata: { run_id: 'run-123' },
          }),
        ]}
      />,
    );

    await user.click(screen.getByText('Expandable event'));
    expect(screen.getByText(/full message:/i)).toBeInTheDocument();
    expect(screen.getByText(/duration:/i)).toBeInTheDocument();
    expect(screen.getByText(/run-123/i)).toBeInTheDocument();
  });

  it('renders empty state when no events are available', () => {
    render(<SessionEventsTable events={[]} />);
    expect(screen.getByText(/no events yet\. live session activity will appear here\./i)).toBeInTheDocument();
  });
});
