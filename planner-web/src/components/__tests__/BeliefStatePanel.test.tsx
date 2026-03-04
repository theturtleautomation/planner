import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import BeliefStatePanel from '../BeliefStatePanel';
import type { BeliefState, Classification } from '../../types';

const mockClassification: Classification = {
  project_type: 'Web App',
  complexity: 'medium',
  question_budget: 10,
};

const makeBeliefState = (overrides: Partial<BeliefState> = {}): BeliefState => ({
  filled: {},
  uncertain: {},
  missing: [],
  out_of_scope: [],
  convergence_pct: 0,
  ...overrides,
});

describe('BeliefStatePanel', () => {
  it('shows placeholder text when beliefState is null', () => {
    render(<BeliefStatePanel beliefState={null} classification={null} />);
    expect(
      screen.getByText(/belief state will appear here during the interview/i)
    ).toBeInTheDocument();
  });

  it('renders filled dimensions with their values', () => {
    const beliefState = makeBeliefState({
      filled: {
        stack: { value: 'React', confidence: 0.95 },
        database: { value: 'PostgreSQL', confidence: 0.9 },
      },
    });
    render(<BeliefStatePanel beliefState={beliefState} classification={null} />);
    expect(screen.getByText('stack')).toBeInTheDocument();
    expect(screen.getByText('React')).toBeInTheDocument();
    expect(screen.getByText('database')).toBeInTheDocument();
    expect(screen.getByText('PostgreSQL')).toBeInTheDocument();
  });

  it('renders uncertain dimensions with confidence percentage', () => {
    // The component renders slot.value and slot.confidence for each uncertain entry.
    // Provide value as a string-compatible shape so the component can render it.
    const beliefState = makeBeliefState({
      uncertain: {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        deployment: { value: 'AWS' as any, confidence: 0.6 },
      },
    });
    render(<BeliefStatePanel beliefState={beliefState} classification={null} />);
    expect(screen.getByText('deployment')).toBeInTheDocument();
    // 0.6 * 100 = 60%
    expect(screen.getByText('(60%)')).toBeInTheDocument();
  });

  it('renders missing dimensions as bullet list items', () => {
    const beliefState = makeBeliefState({
      missing: ['auth', 'ci_cd', 'hosting'],
    });
    render(<BeliefStatePanel beliefState={beliefState} classification={null} />);
    expect(screen.getByText('auth')).toBeInTheDocument();
    expect(screen.getByText('ci_cd')).toBeInTheDocument();
    expect(screen.getByText('hosting')).toBeInTheDocument();
  });

  it('renders out_of_scope dimensions', () => {
    const beliefState = makeBeliefState({
      out_of_scope: ['marketing', 'legal'],
    });
    render(<BeliefStatePanel beliefState={beliefState} classification={null} />);
    expect(screen.getByText('marketing')).toBeInTheDocument();
    expect(screen.getByText('legal')).toBeInTheDocument();
  });

  it('sections are collapsible — clicking header hides content', () => {
    const beliefState = makeBeliefState({
      filled: { stack: { value: 'React', confidence: 1 } },
    });
    render(<BeliefStatePanel beliefState={beliefState} classification={null} />);

    // The filled value is visible initially
    expect(screen.getByText('React')).toBeInTheDocument();

    // Click the "✓ Filled" section header button to collapse it
    const filledHeader = screen.getByRole('button', { name: /✓ filled/i });
    fireEvent.click(filledHeader);

    // Value should no longer be visible
    expect(screen.queryByText('React')).not.toBeInTheDocument();
  });

  it('clicking collapsed section header expands it again', () => {
    const beliefState = makeBeliefState({
      filled: { stack: { value: 'React', confidence: 1 } },
    });
    render(<BeliefStatePanel beliefState={beliefState} classification={null} />);

    const filledHeader = screen.getByRole('button', { name: /✓ filled/i });
    // collapse
    fireEvent.click(filledHeader);
    expect(screen.queryByText('React')).not.toBeInTheDocument();
    // expand again
    fireEvent.click(filledHeader);
    expect(screen.getByText('React')).toBeInTheDocument();
  });

  it('shows counts in section headers', () => {
    const beliefState = makeBeliefState({
      filled: {
        stack: { value: 'React', confidence: 1 },
        auth: { value: 'JWT', confidence: 1 },
      },
      missing: ['deployment'],
    });
    render(<BeliefStatePanel beliefState={beliefState} classification={null} />);

    // Find the count badge "2" near the Filled section
    // The count appears in a <span> inside the section header button
    const filledHeader = screen.getByRole('button', { name: /✓ filled/i });
    expect(filledHeader).toHaveTextContent('2');

    const missingHeader = screen.getByRole('button', { name: /○ missing/i });
    expect(missingHeader).toHaveTextContent('1');
  });

  it('renders ClassificationBadge when classification is provided', () => {
    const beliefState = makeBeliefState();
    render(
      <BeliefStatePanel
        beliefState={beliefState}
        classification={mockClassification}
      />
    );
    // ClassificationBadge renders the project_type
    expect(screen.getByText('Web App')).toBeInTheDocument();
  });

  it('does not render ClassificationBadge when classification is null', () => {
    const beliefState = makeBeliefState();
    render(<BeliefStatePanel beliefState={beliefState} classification={null} />);
    expect(screen.queryByText('Web App')).not.toBeInTheDocument();
  });
});
