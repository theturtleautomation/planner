import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, vi } from 'vitest';
import QuickOptions from '../QuickOptions';

const sampleOptions = [
  { label: 'React', value: 'react' },
  { label: 'Vue', value: 'vue' },
  { label: 'Angular', value: 'angular' },
];

describe('QuickOptions', () => {
  it('renders nothing when options array is empty', () => {
    const { container } = render(<QuickOptions options={[]} onSelect={vi.fn()} />);
    expect(container.firstChild).toBeNull();
  });

  it('renders a button for each option', () => {
    render(<QuickOptions options={sampleOptions} onSelect={vi.fn()} />);
    expect(screen.getByRole('button', { name: 'React' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Vue' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Angular' })).toBeInTheDocument();
  });

  it('calls onSelect with option value when a button is clicked', async () => {
    const user = userEvent.setup();
    const onSelect = vi.fn();
    render(<QuickOptions options={sampleOptions} onSelect={onSelect} />);
    await user.click(screen.getByRole('button', { name: 'Vue' }));
    expect(onSelect).toHaveBeenCalledWith('vue');
  });

  it('calls onSelect with correct value for each distinct option', async () => {
    const user = userEvent.setup();
    const onSelect = vi.fn();
    render(<QuickOptions options={sampleOptions} onSelect={onSelect} />);
    await user.click(screen.getByRole('button', { name: 'React' }));
    expect(onSelect).toHaveBeenCalledWith('react');
    await user.click(screen.getByRole('button', { name: 'Angular' }));
    expect(onSelect).toHaveBeenCalledWith('angular');
  });

  it('buttons are disabled when disabled prop is true', () => {
    render(<QuickOptions options={sampleOptions} onSelect={vi.fn()} disabled={true} />);
    const buttons = screen.getAllByRole('button');
    buttons.forEach((btn) => {
      expect(btn).toBeDisabled();
    });
  });

  it('does not call onSelect when buttons are disabled and clicked', async () => {
    const user = userEvent.setup();
    const onSelect = vi.fn();
    render(<QuickOptions options={sampleOptions} onSelect={onSelect} disabled={true} />);
    // userEvent won't trigger click on disabled buttons
    await user.click(screen.getByRole('button', { name: 'React' }));
    expect(onSelect).not.toHaveBeenCalled();
  });

  it('buttons show label text', () => {
    render(<QuickOptions options={sampleOptions} onSelect={vi.fn()} />);
    expect(screen.getByText('React')).toBeInTheDocument();
    expect(screen.getByText('Vue')).toBeInTheDocument();
    expect(screen.getByText('Angular')).toBeInTheDocument();
  });

  it('renders correct count of buttons', () => {
    render(<QuickOptions options={sampleOptions} onSelect={vi.fn()} />);
    expect(screen.getAllByRole('button')).toHaveLength(3);
  });

  it('renders single option correctly', () => {
    const onSelect = vi.fn();
    render(<QuickOptions options={[{ label: 'Yes', value: 'yes' }]} onSelect={onSelect} />);
    expect(screen.getByRole('button', { name: 'Yes' })).toBeInTheDocument();
  });
});
