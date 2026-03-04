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

  // ─── New tests for intakePhase / currentQuestion / onSkip / onDone ───────────

  it('shows Skip button when currentQuestion.allowSkip is true and intakePhase is interviewing', () => {
    render(
      <MessageInput
        onSend={vi.fn()}
        intakePhase="interviewing"
        currentQuestion={{ text: 'What stack?', allowSkip: true }}
        onSkip={vi.fn()}
      />
    );
    expect(screen.getByRole('button', { name: /skip question/i })).toBeInTheDocument();
  });

  it('does NOT show Skip button when currentQuestion.allowSkip is false', () => {
    render(
      <MessageInput
        onSend={vi.fn()}
        intakePhase="interviewing"
        currentQuestion={{ text: 'What stack?', allowSkip: false }}
        onSkip={vi.fn()}
      />
    );
    expect(screen.queryByRole('button', { name: /skip question/i })).not.toBeInTheDocument();
  });

  it('shows Done button when intakePhase is interviewing', () => {
    render(
      <MessageInput
        onSend={vi.fn()}
        intakePhase="interviewing"
        onDone={vi.fn()}
      />
    );
    expect(screen.getByRole('button', { name: /done with interview/i })).toBeInTheDocument();
  });

  it('does NOT show Done button when intakePhase is not interviewing', () => {
    render(
      <MessageInput
        onSend={vi.fn()}
        intakePhase="waiting"
        onDone={vi.fn()}
      />
    );
    expect(screen.queryByRole('button', { name: /done with interview/i })).not.toBeInTheDocument();
  });

  it('does NOT show Done button when intakePhase is undefined', () => {
    render(
      <MessageInput
        onSend={vi.fn()}
        onDone={vi.fn()}
      />
    );
    expect(screen.queryByRole('button', { name: /done with interview/i })).not.toBeInTheDocument();
  });

  it('calls onSkip when Skip button is clicked', async () => {
    const user = userEvent.setup();
    const onSkip = vi.fn();
    render(
      <MessageInput
        onSend={vi.fn()}
        intakePhase="interviewing"
        currentQuestion={{ text: 'What stack?', allowSkip: true }}
        onSkip={onSkip}
      />
    );
    await user.click(screen.getByRole('button', { name: /skip question/i }));
    expect(onSkip).toHaveBeenCalledTimes(1);
  });

  it('calls onDone when Done button is clicked', async () => {
    const user = userEvent.setup();
    const onDone = vi.fn();
    render(
      <MessageInput
        onSend={vi.fn()}
        intakePhase="interviewing"
        onDone={onDone}
      />
    );
    await user.click(screen.getByRole('button', { name: /done with interview/i }));
    expect(onDone).toHaveBeenCalledTimes(1);
  });

  it('shows phase-appropriate placeholder for interviewing phase', () => {
    render(
      <MessageInput
        onSend={vi.fn()}
        intakePhase="interviewing"
      />
    );
    const textarea = screen.getByRole('textbox', { name: /message input/i });
    expect(textarea).toHaveAttribute('placeholder', expect.stringMatching(/type your answer/i));
  });

  it('renders quick options when currentQuestion has quickOptions', () => {
    const options = [
      { label: 'React', value: 'react' },
      { label: 'Vue', value: 'vue' },
    ];
    render(
      <MessageInput
        onSend={vi.fn()}
        intakePhase="interviewing"
        currentQuestion={{ text: 'What framework?', quickOptions: options }}
      />
    );
    expect(screen.getByRole('button', { name: 'React' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Vue' })).toBeInTheDocument();
  });

  it('clicking a quick option calls onSend with the option value', async () => {
    const user = userEvent.setup();
    const onSend = vi.fn();
    const options = [
      { label: 'React', value: 'react' },
      { label: 'Vue', value: 'vue' },
    ];
    render(
      <MessageInput
        onSend={onSend}
        intakePhase="interviewing"
        currentQuestion={{ text: 'What framework?', quickOptions: options }}
      />
    );
    await user.click(screen.getByRole('button', { name: 'React' }));
    expect(onSend).toHaveBeenCalledWith('react');
  });
});
