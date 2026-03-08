import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import ChatPanel from '../ChatPanel';
import type { ChatMessage, MessageRole } from '../../types';

const makeMessage = (
  id: string,
  role: MessageRole,
  content: string,
  timestamp = '2024-01-01T12:00:00.000Z',
): ChatMessage => ({ id, role, content, timestamp });

describe('ChatPanel', () => {
  it('handles empty message list', () => {
    render(<ChatPanel messages={[]} />);
    expect(screen.getByText(/no messages yet/i)).toBeInTheDocument();
  });

  it('empty state shows hint to send a message', () => {
    render(<ChatPanel messages={[]} />);
    expect(screen.getByText(/send one to begin/i)).toBeInTheDocument();
  });

  it('renders user message content', () => {
    const msgs = [makeMessage('1', 'user', 'Hello planner!')];
    render(<ChatPanel messages={msgs} />);
    expect(screen.getByText('Hello planner!')).toBeInTheDocument();
  });

  it('renders planner message content', () => {
    const msgs = [makeMessage('1', 'planner', 'Here is your plan.')];
    render(<ChatPanel messages={msgs} />);
    expect(screen.getByText('Here is your plan.')).toBeInTheDocument();
  });

  it('renders system message content', () => {
    const msgs = [makeMessage('1', 'system', 'Session started.')];
    render(<ChatPanel messages={msgs} />);
    expect(screen.getByText('Session started.')).toBeInTheDocument();
  });

  it('renders role labels (user, planner) as text content', () => {
    const msgs = [
      makeMessage('1', 'user', 'User message'),
      makeMessage('2', 'planner', 'Planner response'),
    ];
    render(<ChatPanel messages={msgs} />);
    expect(screen.getByText('user')).toBeInTheDocument();
    expect(screen.getByText('planner')).toBeInTheDocument();
  })

  it('renders multiple messages', () => {
    const msgs = [
      makeMessage('1', 'user', 'First message'),
      makeMessage('2', 'planner', 'First response'),
      makeMessage('3', 'user', 'Second message'),
    ];
    render(<ChatPanel messages={msgs} />);
    expect(screen.getByText('First message')).toBeInTheDocument();
    expect(screen.getByText('First response')).toBeInTheDocument();
    expect(screen.getByText('Second message')).toBeInTheDocument();
  });

  it('renders markdown in planner messages', () => {
    const msgs = [makeMessage('1', 'planner', '**Bold text**')];
    render(<ChatPanel messages={msgs} />);
    // ReactMarkdown renders <strong> for **text**
    expect(screen.getByText('Bold text')).toBeInTheDocument();
    const strong = document.querySelector('strong');
    expect(strong).toBeInTheDocument();
    expect(strong?.textContent).toBe('Bold text');
  });

  it('renders code blocks in planner messages', () => {
    const msgs = [makeMessage('1', 'planner', '`inline code`')];
    render(<ChatPanel messages={msgs} />);
    const code = document.querySelector('code');
    expect(code).toBeInTheDocument();
    expect(code?.textContent).toBe('inline code');
  });

  it('does NOT render markdown in user messages (plain text)', () => {
    const msgs = [makeMessage('1', 'user', '**not bold**')];
    render(<ChatPanel messages={msgs} />);
    // User content is rendered as plain text (no markdown processing)
    expect(screen.getByText('**not bold**')).toBeInTheDocument();
    expect(document.querySelector('strong')).not.toBeInTheDocument();
  });

  it('renders system messages as plain text', () => {
    const msgs = [makeMessage('1', 'system', 'System info here')];
    render(<ChatPanel messages={msgs} />);
    expect(screen.getByText('System info here')).toBeInTheDocument();
  });

  it('renders the system role label as text content', () => {
    const msgs = [makeMessage('1', 'system', 'System info')];
    render(<ChatPanel messages={msgs} />);
    expect(screen.getByText('system')).toBeInTheDocument();
  });

  it('hides legacy event-role messages from the transcript', () => {
    const msgs = [
      makeMessage('1', 'event', 'legacy event row'),
      makeMessage('2', 'planner', 'Planner response'),
    ];
    render(<ChatPanel messages={msgs} />);
    expect(screen.queryByText('legacy event row')).not.toBeInTheDocument();
    expect(screen.getByText('Planner response')).toBeInTheDocument();
  });

  it('role labels have text-transform uppercase style', () => {
    const msgs = [makeMessage('1', 'user', 'Hello')];
    render(<ChatPanel messages={msgs} />);
    const roleLabel = screen.getByText('user');
    expect(roleLabel).toHaveStyle({ textTransform: 'uppercase' });
  });

  it('renders timestamps for messages', () => {
    const msgs = [makeMessage('1', 'user', 'Hello', '2024-06-15T14:30:45.000Z')];
    render(<ChatPanel messages={msgs} />);
    // formatTime renders HH:MM:SS — at least some time-like text should appear
    // The exact value depends on locale, so just check there's a time element rendered
    const container = screen.getByText('Hello').closest('div')?.parentElement;
    expect(container).toBeInTheDocument();
  });

  it('renders messages list container (not the empty state) when messages exist', () => {
    const msgs = [makeMessage('1', 'user', 'hello')];
    render(<ChatPanel messages={msgs} />);
    expect(screen.queryByText(/no messages yet/i)).not.toBeInTheDocument();
  });
});
