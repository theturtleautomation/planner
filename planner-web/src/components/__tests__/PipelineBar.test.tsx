import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import PipelineBar from '../PipelineBar';
import type { PipelineStage } from '../../types';

// Note: text-transform: uppercase is CSS-only — DOM text remains as-is.
// Use case-insensitive matching to match what the component renders.

const mockStages: PipelineStage[] = [
  { name: 'Intake', status: 'complete' },
  { name: 'Chunk', status: 'running' },
  { name: 'Compile', status: 'pending' },
  { name: 'Lint', status: 'failed' },
];

describe('PipelineBar', () => {
  it('renders all pipeline stages by name', () => {
    render(<PipelineBar stages={mockStages} />);
    expect(screen.getByText('Intake')).toBeInTheDocument();
    expect(screen.getByText('Chunk')).toBeInTheDocument();
    expect(screen.getByText('Compile')).toBeInTheDocument();
    expect(screen.getByText('Lint')).toBeInTheDocument();
  });

  it('renders stage names as text content', () => {
    render(<PipelineBar stages={[{ name: 'Intake', status: 'pending' }]} />);
    expect(screen.getByText('Intake')).toBeInTheDocument();
  });

  it('renders stage names with text-transform uppercase style', () => {
    render(<PipelineBar stages={[{ name: 'Intake', status: 'pending' }]} />);
    const stageLabel = screen.getByText('Intake');
    expect(stageLabel).toHaveStyle({ textTransform: 'uppercase' });
  });

  it('renders empty bar with no stages', () => {
    const { container } = render(<PipelineBar stages={[]} />);
    // The outer div should be present but with no stage content
    expect(container.firstChild).toBeInTheDocument();
    expect(screen.queryByText(/intake|chunk|compile/i)).not.toBeInTheDocument();
  });

  it('shows arrow separators between stages', () => {
    render(<PipelineBar stages={mockStages} />);
    const separators = screen.getAllByText('›');
    // 4 stages → 3 separators
    expect(separators).toHaveLength(3);
  });

  it('does not show separator after last stage', () => {
    render(<PipelineBar stages={[{ name: 'Intake', status: 'complete' }, { name: 'Chunk', status: 'pending' }]} />);
    const separators = screen.getAllByText('›');
    expect(separators).toHaveLength(1);
  });

  it('applies pulse class to running stage indicator', () => {
    render(<PipelineBar stages={[{ name: 'Chunk', status: 'running' }]} />);
    const dots = document.querySelectorAll('.pulse');
    expect(dots.length).toBe(1);
  });

  it('does not apply pulse class to non-running stages', () => {
    render(<PipelineBar stages={[{ name: 'Intake', status: 'complete' }, { name: 'Lint', status: 'failed' }]} />);
    const dots = document.querySelectorAll('.pulse');
    expect(dots.length).toBe(0);
  });

  it('renders all 12 default pipeline stages when provided', () => {
    const allStages: PipelineStage[] = [
      { name: 'Intake', status: 'complete' },
      { name: 'Chunk', status: 'complete' },
      { name: 'Compile', status: 'complete' },
      { name: 'Lint', status: 'complete' },
      { name: 'AR Review', status: 'running' },
      { name: 'Refine', status: 'pending' },
      { name: 'Scenarios', status: 'pending' },
      { name: 'Ralph', status: 'pending' },
      { name: 'Graph', status: 'pending' },
      { name: 'Factory', status: 'pending' },
      { name: 'Validate', status: 'pending' },
      { name: 'Git', status: 'pending' },
    ];
    render(<PipelineBar stages={allStages} />);
    expect(screen.getByText('AR Review')).toBeInTheDocument();
    expect(screen.getByText('Scenarios')).toBeInTheDocument();
    expect(screen.getByText('Ralph')).toBeInTheDocument();
    expect(screen.getByText('Graph')).toBeInTheDocument();
    expect(screen.getByText('Factory')).toBeInTheDocument();
    expect(screen.getByText('Validate')).toBeInTheDocument();
    expect(screen.getByText('Git')).toBeInTheDocument();
  });

  it('renders a single stage without separator', () => {
    render(<PipelineBar stages={[{ name: 'Git', status: 'complete' }]} />);
    expect(screen.getByText('Git')).toBeInTheDocument();
    expect(screen.queryByText('›')).not.toBeInTheDocument();
  });

  it('running stage has bold text (fontWeight 700)', () => {
    render(<PipelineBar stages={[{ name: 'Compile', status: 'running' }]} />);
    const stageLabel = screen.getByText('Compile');
    expect(stageLabel).toHaveStyle({ fontWeight: '700' });
  });

  it('pending stage does not have bold text', () => {
    render(<PipelineBar stages={[{ name: 'Compile', status: 'pending' }]} />);
    const stageLabel = screen.getByText('Compile');
    expect(stageLabel).toHaveStyle({ fontWeight: '400' });
  });

  it('complete stage has correct status shown via styles', () => {
    render(<PipelineBar stages={[{ name: 'Intake', status: 'complete' }]} />);
    // Complete stages have font-weight 400 (not bold)
    const stageLabel = screen.getByText('Intake');
    expect(stageLabel).toHaveStyle({ fontWeight: '400' });
  });

  it('failed stage has correct status shown via styles', () => {
    render(<PipelineBar stages={[{ name: 'Lint', status: 'failed' }]} />);
    const stageLabel = screen.getByText('Lint');
    expect(stageLabel).toHaveStyle({ fontWeight: '400' });
  });
});
