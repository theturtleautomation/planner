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
    const { container } = render(<QuickOptions options={[]} onToggle={vi.fn()} />);
    expect(container.firstChild).toBeNull();
  });

  it('renders a button for each option', () => {
    render(<QuickOptions options={sampleOptions} onToggle={vi.fn()} />);
    expect(screen.getByRole('button', { name: 'React' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Vue' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Angular' })).toBeInTheDocument();
  });

  it('calls onToggle with option value when a button is clicked', async () => {
    const user = userEvent.setup();
    const onToggle = vi.fn();
    render(<QuickOptions options={sampleOptions} onToggle={onToggle} />);
    await user.click(screen.getByRole('button', { name: 'Vue' }));
    expect(onToggle).toHaveBeenCalledWith('vue');
  });

  it('calls onToggle with correct value for each distinct option', async () => {
    const user = userEvent.setup();
    const onToggle = vi.fn();
    render(<QuickOptions options={sampleOptions} onToggle={onToggle} />);
    await user.click(screen.getByRole('button', { name: 'React' }));
    expect(onToggle).toHaveBeenCalledWith('react');
    await user.click(screen.getByRole('button', { name: 'Angular' }));
    expect(onToggle).toHaveBeenCalledWith('angular');
  });

  it('buttons are disabled when disabled prop is true', () => {
    render(<QuickOptions options={sampleOptions} onToggle={vi.fn()} disabled={true} />);
    const buttons = screen.getAllByRole('button');
    buttons.forEach((btn) => {
      expect(btn).toBeDisabled();
    });
  });

  it('does not call onToggle when buttons are disabled and clicked', async () => {
    const user = userEvent.setup();
    const onToggle = vi.fn();
    render(<QuickOptions options={sampleOptions} onToggle={onToggle} disabled={true} />);
    // userEvent won't trigger click on disabled buttons
    await user.click(screen.getByRole('button', { name: 'React' }));
    expect(onToggle).not.toHaveBeenCalled();
  });

  it('buttons show label text', () => {
    render(<QuickOptions options={sampleOptions} onToggle={vi.fn()} />);
    expect(screen.getByText('React')).toBeInTheDocument();
    expect(screen.getByText('Vue')).toBeInTheDocument();
    expect(screen.getByText('Angular')).toBeInTheDocument();
  });

  it('renders correct count of buttons', () => {
    render(<QuickOptions options={sampleOptions} onToggle={vi.fn()} />);
    expect(screen.getAllByRole('button')).toHaveLength(3);
  });

  it('renders single option correctly', () => {
    const onToggle = vi.fn();
    render(<QuickOptions options={[{ label: 'Yes', value: 'yes' }]} onToggle={onToggle} />);
    expect(screen.getByRole('button', { name: 'Yes' })).toBeInTheDocument();
  });

  it('marks selected values as pressed', () => {
    render(
      <QuickOptions
        options={sampleOptions}
        selectedValues={['vue']}
        onToggle={vi.fn()}
      />,
    );
    expect(screen.getByRole('button', { name: /vue/i })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByRole('button', { name: /react/i })).toHaveAttribute('aria-pressed', 'false');
  });
});
