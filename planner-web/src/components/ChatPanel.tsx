import { useEffect, useRef } from 'react';
import ReactMarkdown from 'react-markdown';
import type { ChatMessage } from '../types.ts';

interface ChatPanelProps {
  messages: ChatMessage[];
}

function formatTime(iso: string): string {
  const d = new Date(iso);
  return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false });
}

export default function ChatPanel({ messages }: ChatPanelProps) {
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  if (messages.length === 0) {
    return (
      <div style={{
        flex: 1,
        overflow: 'auto',
        padding: '20px',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
      }}>
        <span style={{ color: 'var(--text-secondary)', fontSize: '13px' }}>
          no messages yet — send one to begin
        </span>
      </div>
    );
  }

  return (
    <div style={{
      flex: 1,
      overflow: 'auto',
      padding: '16px 20px',
      display: 'flex',
      flexDirection: 'column',
      gap: '12px',
    }}>
      {messages.map((msg) => (
        <MessageItem key={msg.id} msg={msg} />
      ))}
      <div ref={bottomRef} />
    </div>
  );
}

function MessageItem({ msg }: { msg: ChatMessage }) {
  const roleColors: Record<string, string> = {
    system: 'var(--text-secondary)',
    user: 'var(--accent-green)',
    planner: 'var(--accent-cyan)',
  };

  const labelColor = roleColors[msg.role] ?? 'var(--text-secondary)';

  return (
    <div style={{
      display: 'flex',
      flexDirection: 'column',
      gap: '4px',
    }}>
      {/* Role label + timestamp */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
        <span style={{
          color: labelColor,
          fontSize: '11px',
          fontWeight: 700,
          letterSpacing: '0.08em',
          textTransform: 'uppercase',
        }}>
          {msg.role}
        </span>
        <span style={{ color: 'var(--text-secondary)', fontSize: '10px' }}>
          {formatTime(msg.timestamp)}
        </span>
      </div>

      {/* Content */}
      {msg.role === 'planner' ? (
        <div style={{
          color: 'var(--text-primary)',
          fontSize: '13px',
          lineHeight: '1.7',
          paddingLeft: '8px',
          borderLeft: '2px solid var(--accent-cyan)',
        }}>
          <ReactMarkdown
            components={{
              p: ({ children }) => <p style={{ margin: '0 0 8px 0' }}>{children}</p>,
              code: ({ children }) => (
                <code style={{
                  background: 'var(--bg-tertiary)',
                  color: 'var(--accent-yellow)',
                  padding: '1px 5px',
                  borderRadius: '2px',
                  fontSize: '12px',
                }}>
                  {children}
                </code>
              ),
              pre: ({ children }) => (
                <pre style={{
                  background: 'var(--bg-tertiary)',
                  border: '1px solid var(--border)',
                  padding: '10px 14px',
                  borderRadius: '3px',
                  overflow: 'auto',
                  fontSize: '12px',
                  margin: '8px 0',
                }}>
                  {children}
                </pre>
              ),
              ul: ({ children }) => <ul style={{ paddingLeft: '20px', margin: '4px 0' }}>{children}</ul>,
              ol: ({ children }) => <ol style={{ paddingLeft: '20px', margin: '4px 0' }}>{children}</ol>,
              li: ({ children }) => <li style={{ margin: '2px 0' }}>{children}</li>,
              strong: ({ children }) => <strong style={{ color: 'var(--accent-cyan)' }}>{children}</strong>,
            }}
          >
            {msg.content}
          </ReactMarkdown>
        </div>
      ) : msg.role === 'system' ? (
        <div style={{
          color: 'var(--text-secondary)',
          fontSize: '12px',
          fontStyle: 'italic',
          paddingLeft: '8px',
        }}>
          {msg.content}
        </div>
      ) : (
        <div style={{
          color: 'var(--text-primary)',
          fontSize: '13px',
          paddingLeft: '8px',
          borderLeft: '2px solid var(--accent-green)',
          whiteSpace: 'pre-wrap',
          wordBreak: 'break-word',
        }}>
          {msg.content}
        </div>
      )}
    </div>
  );
}
