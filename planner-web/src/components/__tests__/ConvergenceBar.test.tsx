import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import ConvergenceBar from '../ConvergenceBar';

const mockClassification = {
  project_type: 'Web App',
  complexity: 'medium',
};

describe('ConvergenceBar', () => {
  it('renders "convergence" label', () => {
    render(<ConvergenceBar convergencePct={50} classification={null} />);
    expect(screen.getByText(/convergence/i)).toBeInTheDocument();
  });

  it('shows percentage text when classification is provided', () => {
    render(<ConvergenceBar convergencePct={65} classification={mockClassification} />);
    expect(screen.getByText(/65%/)).toBeInTheDocument();
  });

  it('clamps percentage below 0 to 0', () => {
    render(<ConvergenceBar convergencePct={-20} classification={mockClassification} />);
    expect(screen.getByText(/0%/)).toBeInTheDocument();
  });

  it('clamps percentage above 100 to 100', () => {
    render(<ConvergenceBar convergencePct={150} classification={mockClassification} />);
    expect(screen.getByText(/100%/)).toBeInTheDocument();
  });

  it('shows green color (accent-green) when pct >= 80', () => {
    const { container } = render(
      <ConvergenceBar convergencePct={85} classification={mockClassification} />
    );
    // The progress fill bar is inside the container; check its style
    const fillBar = container.querySelector('[style*="width: 85%"]') as HTMLElement | null;
    expect(fillBar).not.toBeNull();
    expect(fillBar!.style.background).toBe('var(--accent-green)');
  });

  it('shows yellow color (accent-yellow) when pct >= 50 but < 80', () => {
    const { container } = render(
      <ConvergenceBar convergencePct={60} classification={mockClassification} />
    );
    const fillBar = container.querySelector('[style*="width: 60%"]') as HTMLElement | null;
    expect(fillBar).not.toBeNull();
    expect(fillBar!.style.background).toBe('var(--accent-yellow)');
  });

  it('shows "Analyzing project…" when no classification provided', () => {
    render(<ConvergenceBar convergencePct={30} classification={null} />);
    expect(screen.getByText(/analyzing project/i)).toBeInTheDocument();
  });

  it('does not show "Analyzing project…" when classification is provided', () => {
    render(<ConvergenceBar convergencePct={50} classification={mockClassification} />);
    expect(screen.queryByText(/analyzing project/i)).not.toBeInTheDocument();
  });

  it('progress bar width matches clamped percentage', () => {
    const { container } = render(
      <ConvergenceBar convergencePct={72} classification={null} />
    );
    const fillBar = container.querySelector('[style*="width: 72%"]') as HTMLElement | null;
    expect(fillBar).not.toBeNull();
  });

  it('progress bar width is 0% when convergencePct is 0', () => {
    const { container } = render(
      <ConvergenceBar convergencePct={0} classification={null} />
    );
    const fillBar = container.querySelector('[style*="width: 0%"]') as HTMLElement | null;
    expect(fillBar).not.toBeNull();
  });

  it('shows project_type in text when classification is provided', () => {
    render(<ConvergenceBar convergencePct={50} classification={mockClassification} />);
    expect(screen.getByText(/Web App/)).toBeInTheDocument();
  });
});
