import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import InterviewProgressPanel from '../InterviewProgressPanel.tsx';
import type { PlannerEvent } from '../../types.ts';

function makeEvent(overrides: Partial<PlannerEvent> = {}): PlannerEvent {
  return {
    id: 'evt-1',
    timestamp: new Date().toISOString(),
    level: 'info',
    source: 'socratic_engine',
    message: 'Prompt response adjudicated at 15% convergence',
    metadata: {},
    ...overrides,
  };
}

describe('InterviewProgressPanel', () => {
  it('renders readable intake progress while the next prompt is pending', () => {
    render(
      <InterviewProgressPanel
        currentStep="socratic.response.adjudicated"
        events={[
          makeEvent({
            step: 'socratic.response.adjudicated',
          }),
        ]}
        isConnected={true}
      />,
    );

    expect(screen.getByRole('region', { name: /interview progress/i })).toBeInTheDocument();
    expect(screen.getByText(/generating your next questions/i)).toBeInTheDocument();
    expect(screen.getByText(/planning the next question batch/i)).toBeInTheDocument();
    expect(screen.getByText(/connected to live interview runtime/i)).toBeInTheDocument();
    expect(screen.getByText(/prompt response adjudicated at 15% convergence/i)).toBeInTheDocument();
  });

  it('shows a longer-than-usual hint for stale intake steps', () => {
    render(
      <InterviewProgressPanel
        currentStep="socratic.response.adjudicated"
        events={[
          makeEvent({
            timestamp: new Date(Date.now() - 30_000).toISOString(),
            step: 'socratic.response.adjudicated',
          }),
        ]}
        isConnected={false}
      />,
    );

    expect(screen.getByText(/this step is taking longer than usual/i)).toBeInTheDocument();
    expect(screen.getByText(/waiting for the next live update/i)).toBeInTheDocument();
  });
});
