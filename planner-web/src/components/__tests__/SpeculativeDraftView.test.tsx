import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, vi } from 'vitest';
import SpeculativeDraftView from '../SpeculativeDraftView';
import type { SpeculativeDraft } from '../../types';

const mockDraft: SpeculativeDraft = {
  sections: [
    { heading: 'Overview', content: 'A web application for task management.' },
    { heading: 'Tech Stack', content: 'React frontend, Node.js backend.' },
  ],
  assumptions: [
    { dimension: 'auth', assumption: 'JWT-based authentication assumed.' },
    { dimension: 'deployment', assumption: 'Deployed to AWS EC2.' },
  ],
  not_discussed: ['ci_cd', 'monitoring', 'testing_strategy'],
};

describe('SpeculativeDraftView', () => {
  it('renders "Draft Spec" header', () => {
    render(<SpeculativeDraftView draft={mockDraft} onBack={vi.fn()} />);
    expect(screen.getByText(/draft spec/i)).toBeInTheDocument();
  });

  it('renders each section heading', () => {
    render(<SpeculativeDraftView draft={mockDraft} onBack={vi.fn()} />);
    expect(screen.getByText('Overview')).toBeInTheDocument();
    expect(screen.getByText('Tech Stack')).toBeInTheDocument();
  });

  it('renders each section content', () => {
    render(<SpeculativeDraftView draft={mockDraft} onBack={vi.fn()} />);
    expect(screen.getByText('A web application for task management.')).toBeInTheDocument();
    expect(screen.getByText('React frontend, Node.js backend.')).toBeInTheDocument();
  });

  it('renders assumption dimension text', () => {
    render(<SpeculativeDraftView draft={mockDraft} onBack={vi.fn()} />);
    expect(screen.getByText('auth')).toBeInTheDocument();
    expect(screen.getByText('deployment')).toBeInTheDocument();
  });

  it('renders assumption text content', () => {
    render(<SpeculativeDraftView draft={mockDraft} onBack={vi.fn()} />);
    expect(screen.getByText('JWT-based authentication assumed.')).toBeInTheDocument();
    expect(screen.getByText('Deployed to AWS EC2.')).toBeInTheDocument();
  });

  it('renders assumptions section header', () => {
    render(<SpeculativeDraftView draft={mockDraft} onBack={vi.fn()} />);
    expect(screen.getByText(/assumptions \(unconfirmed\)/i)).toBeInTheDocument();
  });

  it('renders not_discussed items as chips', () => {
    render(<SpeculativeDraftView draft={mockDraft} onBack={vi.fn()} />);
    expect(screen.getByText('ci_cd')).toBeInTheDocument();
    expect(screen.getByText('monitoring')).toBeInTheDocument();
    expect(screen.getByText('testing_strategy')).toBeInTheDocument();
  });

  it('renders "Not yet discussed:" section header', () => {
    render(<SpeculativeDraftView draft={mockDraft} onBack={vi.fn()} />);
    expect(screen.getByText(/not yet discussed/i)).toBeInTheDocument();
  });

  it('calls onBack when back button is clicked', async () => {
    const user = userEvent.setup();
    const onBack = vi.fn();
    render(<SpeculativeDraftView draft={mockDraft} onBack={onBack} />);
    await user.click(screen.getByRole('button', { name: /back to state/i }));
    expect(onBack).toHaveBeenCalledTimes(1);
  });

  it('does not render assumptions section when assumptions array is empty', () => {
    const draftNoAssumptions: SpeculativeDraft = {
      ...mockDraft,
      assumptions: [],
    };
    render(<SpeculativeDraftView draft={draftNoAssumptions} onBack={vi.fn()} />);
    expect(screen.queryByText(/assumptions \(unconfirmed\)/i)).not.toBeInTheDocument();
  });

  it('does not render not_discussed section when array is empty', () => {
    const draftNoNotDiscussed: SpeculativeDraft = {
      ...mockDraft,
      not_discussed: [],
    };
    render(<SpeculativeDraftView draft={draftNoNotDiscussed} onBack={vi.fn()} />);
    expect(screen.queryByText(/not yet discussed/i)).not.toBeInTheDocument();
  });

  it('renders back button with accessible label', () => {
    render(<SpeculativeDraftView draft={mockDraft} onBack={vi.fn()} />);
    expect(screen.getByRole('button', { name: /back to state/i })).toBeInTheDocument();
  });
});
