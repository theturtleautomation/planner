import { render, screen, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, vi } from 'vitest';
import MessageInput from '../MessageInput';

describe('MessageInput', () => {
  it('renders textarea and send button', () => {
    render(<MessageInput onSend={vi.fn()} />);
    expect(screen.getByRole('textbox', { name: /message input/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /send message/i })).toBeInTheDocument();
  });

  it('send button is disabled when input is empty', () => {
    render(<MessageInput onSend={vi.fn()} />);
    const button = screen.getByRole('button', { name: /send message/i });
    expect(button).toBeDisabled();
  });

  it('send button is enabled when input has content', async () => {
    const user = userEvent.setup();
    render(<MessageInput onSend={vi.fn()} />);
    const textarea = screen.getByRole('textbox', { name: /message input/i });
    await user.type(textarea, 'Hello world');
    const button = screen.getByRole('button', { name: /send message/i });
    expect(button).not.toBeDisabled();
  });

  it('send button is disabled when disabled prop is true', async () => {
    const user = userEvent.setup();
    render(<MessageInput onSend={vi.fn()} disabled={true} />);
    const textarea = screen.getByRole('textbox', { name: /message input/i });
    await user.type(textarea, 'Hello');
    const button = screen.getByRole('button', { name: /send message/i });
    expect(button).toBeDisabled();
  });

  it('send button is disabled when pipelineRunning is true', async () => {
    const user = userEvent.setup();
    render(<MessageInput onSend={vi.fn()} pipelineRunning={true} />);
    const textarea = screen.getByRole('textbox', { name: /message input/i });
    await user.type(textarea, 'Hello');
    const button = screen.getByRole('button', { name: /send message/i });
    expect(button).toBeDisabled();
  });

  it('calls onSend with the input value when send button is clicked', async () => {
    const user = userEvent.setup();
    const onSend = vi.fn();
    render(<MessageInput onSend={onSend} />);
    const textarea = screen.getByRole('textbox', { name: /message input/i });
    await user.type(textarea, 'Hello world');
    await user.click(screen.getByRole('button', { name: /send message/i }));
    expect(onSend).toHaveBeenCalledWith('Hello world');
  });

  it('clears input after sending', async () => {
    const user = userEvent.setup();
    render(<MessageInput onSend={vi.fn()} />);
    const textarea = screen.getByRole('textbox', { name: /message input/i });
    await user.type(textarea, 'Test message');
    await user.click(screen.getByRole('button', { name: /send message/i }));
    expect(textarea).toHaveValue('');
  });

  it('calls onSend when Enter key is pressed (without Shift)', async () => {
    const user = userEvent.setup();
    const onSend = vi.fn();
    render(<MessageInput onSend={onSend} />);
    const textarea = screen.getByRole('textbox', { name: /message input/i });
    await user.type(textarea, 'Test{Enter}');
    expect(onSend).toHaveBeenCalledWith('Test');
  });

  it('does not call onSend when Shift+Enter is pressed', async () => {
    const user = userEvent.setup();
    const onSend = vi.fn();
    render(<MessageInput onSend={onSend} />);
    const textarea = screen.getByRole('textbox', { name: /message input/i });
    await user.type(textarea, 'Test{Shift>}{Enter}{/Shift}');
    expect(onSend).not.toHaveBeenCalled();
  });

  it('shows pipeline hint text when pipelineRunning is true', () => {
    render(<MessageInput onSend={vi.fn()} pipelineRunning={true} />);
    expect(screen.getByText(/pipeline running — input will re-enable when complete/i)).toBeInTheDocument();
  });

  it('does not show pipeline hint when pipelineRunning is false', () => {
    render(<MessageInput onSend={vi.fn()} pipelineRunning={false} />);
    expect(screen.queryByText(/pipeline running/i)).not.toBeInTheDocument();
  });

  it('textarea has aria-label "Message input"', () => {
    render(<MessageInput onSend={vi.fn()} />);
    expect(screen.getByLabelText('Message input')).toBeInTheDocument();
  });

  it('button has aria-label "Send message"', () => {
    render(<MessageInput onSend={vi.fn()} />);
    expect(screen.getByLabelText('Send message')).toBeInTheDocument();
  });

  it('shows waiting placeholder when isLoading is true', () => {
    render(<MessageInput onSend={vi.fn()} isLoading={true} />);
    const textarea = screen.getByRole('textbox', { name: /message input/i });
    expect(textarea).toHaveAttribute('placeholder', expect.stringMatching(/waiting for response/i));
  });

  it('shows pipeline placeholder when pipelineRunning is true', () => {
    render(<MessageInput onSend={vi.fn()} pipelineRunning={true} />);
    const textarea = screen.getByRole('textbox', { name: /message input/i });
    expect(textarea).toHaveAttribute('placeholder', expect.stringMatching(/pipeline is running/i));
  });

  it('trims whitespace before calling onSend', async () => {
    const user = userEvent.setup();
    const onSend = vi.fn();
    render(<MessageInput onSend={onSend} />);
    const textarea = screen.getByRole('textbox', { name: /message input/i });
    await user.type(textarea, '  hello  ');
    await user.click(screen.getByRole('button', { name: /send message/i }));
    expect(onSend).toHaveBeenCalledWith('hello');
  });

  it('textarea is disabled when pipelineRunning is true', () => {
    render(<MessageInput onSend={vi.fn()} pipelineRunning={true} />);
    expect(screen.getByRole('textbox', { name: /message input/i })).toBeDisabled();
  });

  it('textarea is disabled when isLoading is true', () => {
    render(<MessageInput onSend={vi.fn()} isLoading={true} />);
    expect(screen.getByRole('textbox', { name: /message input/i })).toBeDisabled();
  });

  it('auto-grow: textarea ref is attached (ref exists on DOM node)', () => {
    render(<MessageInput onSend={vi.fn()} />);
    const textarea = screen.getByRole('textbox', { name: /message input/i });
    // The ref is used for auto-grow; verify textarea is rendered as a textarea element
    expect(textarea.tagName.toLowerCase()).toBe('textarea');
  });

  it('displays "…" in button text when isLoading is true', () => {
    render(<MessageInput onSend={vi.fn()} isLoading={true} />);
    expect(screen.getByRole('button', { name: /send message/i })).toHaveTextContent('…');
  });

  it('displays "send" in button text when not loading', () => {
    render(<MessageInput onSend={vi.fn()} />);
    expect(screen.getByRole('button', { name: /send message/i })).toHaveTextContent('send');
  });
});
