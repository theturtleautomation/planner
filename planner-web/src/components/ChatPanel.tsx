import { useEffect, useRef, useCallback, useState } from 'react';
import ReactMarkdown from 'react-markdown';
import rehypeSanitize from 'rehype-sanitize';
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
  const containerRef = useRef<HTMLDivElement>(null);
  const userScrolled = useRef(false);

  // Auto-scroll only when user hasn't scrolled up
  useEffect(() => {
    if (!userScrolled.current) {
      bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
    }
  }, [messages]);

  const handleScroll = useCallback((): void => {
    const el = containerRef.current;
    if (!el) return;
    const nearBottom = el.scrollTop + el.clientHeight >= el.scrollHeight - 50;
    if (nearBottom) {
      // User scrolled back to bottom — re-enable auto-scroll
      userScrolled.current = false;
    } else {
      // User scrolled up — pause auto-scroll
      userScrolled.current = true;
    }
  }, []);

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
        <span style={{ color: 'var(--color-text-muted)', fontSize: '13px' }}>
          no messages yet — send one to begin
        </span>
      </div>
    );
  }

  return (
    <div
      ref={containerRef}
      onScroll={handleScroll}
      style={{
        flex: 1,
        overflow: 'auto',
        padding: '16px 20px',
        display: 'flex',
        flexDirection: 'column',
        gap: '12px',
      }}
    >
      {messages.map((msg) => (
        <MessageItem key={msg.id} msg={msg} />
      ))}
      <div ref={bottomRef} />
    </div>
  );
}

function CollapsibleEvent({ content }: { content: string }) {
  const [expanded, setExpanded] = useState(false);

  // Try to extract a short summary from the content
  let summary = 'Event';
  try {
    const parsed = JSON.parse(content);
    if (parsed.message) {
      summary = String(parsed.message).slice(0, 80);
    } else if (parsed.step) {
      summary = String(parsed.step).slice(0, 80);
    } else if (parsed.source) {
      summary = String(parsed.source).slice(0, 80);
    }
  } catch {
    // Not JSON — use first 80 chars of raw content
    const firstLine = content.split('\n')[0] ?? '';
    if (firstLine.length > 0) {
      summary = firstLine.slice(0, 80);
    }
  }

  return (
    <div style={{
      borderLeft: '2px solid var(--color-gold)',
      borderRadius: '0 3px 3px 0',
      background: 'rgba(255,215,0,0.04)',
    }}>
      {/* Collapsed header — always visible */}
      <div
        onClick={() => setExpanded((v) => !v)}
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '6px',
          padding: '4px 8px',
          cursor: 'pointer',
          userSelect: 'none',
        }}
        title={expanded ? 'Collapse event' : 'Expand event'}
      >
        <span
          style={{
            fontSize: '10px',
            color: 'var(--color-gold)',
            flexShrink: 0,
            transition: 'transform 0.15s',
            transform: expanded ? 'rotate(90deg)' : 'rotate(0deg)',
            display: 'inline-block',
          }}
        >
          ▶
        </span>
        <span style={{
          fontSize: '11px',
          color: 'var(--color-gold)',
          opacity: 0.85,
          overflow: 'hidden',
          textOverflow: 'ellipsis',
          whiteSpace: 'nowrap',
        }}>
          {summary}
        </span>
      </div>

      {/* Expanded content */}
      {expanded && (
        <div style={{
          padding: '4px 8px 6px 22px',
          color: 'var(--color-text)',
          fontSize: '11px',
          lineHeight: '1.5',
          whiteSpace: 'pre-wrap',
          wordBreak: 'break-word',
          fontFamily: 'monospace',
          borderTop: '1px solid rgba(255,215,0,0.12)',
        }}>
          {content}
        </div>
      )}
    </div>
  );
}

function MessageItem({ msg }: { msg: ChatMessage }) {
  const roleColors: Record<string, string> = {
    system: 'var(--color-text-muted)',
    user: 'var(--color-success)',
    planner: 'var(--color-primary)',
    event: 'var(--color-gold)',
  };

  const labelColor = roleColors[msg.role] ?? 'var(--color-text-muted)';

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
        <span style={{ color: 'var(--color-text-muted)', fontSize: '10px' }}>
          {formatTime(msg.timestamp)}
        </span>
      </div>

      {/* Content */}
      {msg.role === 'planner' ? (
        <div style={{
          color: 'var(--color-text)',
          fontSize: '13px',
          lineHeight: '1.7',
          paddingLeft: '8px',
          borderLeft: '2px solid var(--color-primary)',
        }}>
          <ReactMarkdown
            rehypePlugins={[rehypeSanitize]}
            components={{
              p: ({ children }) => <p style={{ margin: '0 0 8px 0' }}>{children}</p>,
              code: ({ children }) => (
                <code style={{
                  background: 'var(--color-surface-2)',
                  color: 'var(--color-gold)',
                  padding: '1px 5px',
                  borderRadius: '2px',
                  fontSize: '12px',
                }}>
                  {children}
                </code>
              ),
              pre: ({ children }) => (
                <pre style={{
                  background: 'var(--color-surface-2)',
                  border: '1px solid var(--color-border)',
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
              strong: ({ children }) => <strong style={{ color: 'var(--color-primary)' }}>{children}</strong>,
            }}
          >
            {msg.content}
          </ReactMarkdown>
        </div>
      ) : msg.role === 'event' ? (
        <CollapsibleEvent content={msg.content} />
      ) : msg.role === 'system' ? (
        <div style={{
          color: 'var(--color-text-muted)',
          fontSize: '12px',
          fontStyle: 'italic',
          paddingLeft: '8px',
        }}>
          {msg.content}
        </div>
      ) : (
        <div style={{
          color: 'var(--color-text)',
          fontSize: '13px',
          paddingLeft: '8px',
          borderLeft: '2px solid var(--color-success)',
          whiteSpace: 'pre-wrap',
          wordBreak: 'break-word',
        }}>
          {msg.content}
        </div>
      )}
    </div>
  );
}
