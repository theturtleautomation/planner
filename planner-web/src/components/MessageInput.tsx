import { useState, useCallback, useEffect, useRef } from 'react';
import type { KeyboardEvent, ChangeEvent } from 'react';

interface MessageInputProps {
  onSend: (content: string) => void;
  disabled?: boolean;
  pipelineRunning?: boolean;
  isLoading?: boolean;
}

export default function MessageInput({
  onSend,
  disabled = false,
  pipelineRunning = false,
  isLoading = false,
}: MessageInputProps) {
  const [value, setValue] = useState('');
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Auto-grow textarea height based on content
  useEffect(() => {
    const el = textareaRef.current;
    if (el) {
      el.style.height = 'auto';
      el.style.height = Math.min(el.scrollHeight, 200) + 'px';
    }
  }, [value]);

  const send = useCallback((): void => {
    const trimmed = value.trim();
    if (!trimmed) return;
    onSend(trimmed);
    setValue('');
    // Reset height after clearing
    const el = textareaRef.current;
    if (el) {
      el.style.height = 'auto';
    }
  }, [value, onSend]);

  const handleKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>): void => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      if (!disabled && !isLoading && !pipelineRunning) {
        send();
      }
    }
  };

  const handleChange = (e: ChangeEvent<HTMLTextAreaElement>): void => {
    setValue(e.target.value);
  };

  const isBlocked = disabled || pipelineRunning || isLoading;

  let placeholder = 'Send a message… (Enter to send, Shift+Enter for newline)';
  if (pipelineRunning) placeholder = 'Pipeline is running — please wait…';
  if (isLoading) placeholder = 'Waiting for response…';

  return (
    <div style={{
      padding: '12px 16px',
      background: 'var(--bg-secondary)',
      borderTop: '1px solid var(--border)',
      flexShrink: 0,
    }}>
      <div style={{
        display: 'flex',
        gap: '10px',
        alignItems: 'flex-end',
        background: 'var(--bg-tertiary)',
        border: `1px solid ${isBlocked ? 'var(--border)' : 'var(--accent-cyan)'}`,
        borderRadius: '3px',
        padding: '8px 12px',
        transition: 'border-color 0.18s',
      }}>
        <textarea
          ref={textareaRef}
          value={value}
          onChange={handleChange}
          onKeyDown={handleKeyDown}
          disabled={isBlocked}
          placeholder={placeholder}
          rows={1}
          aria-label="Message input"
          style={{
            flex: 1,
            background: 'transparent',
            border: 'none',
            outline: 'none',
            color: isBlocked ? 'var(--text-secondary)' : 'var(--text-primary)',
            fontSize: '13px',
            lineHeight: '1.5',
            resize: 'none',
            cursor: isBlocked ? 'not-allowed' : 'text',
            minHeight: '22px',
            maxHeight: '200px',
            overflowY: 'auto',
          }}
        />
        <button
          onClick={send}
          disabled={isBlocked || !value.trim()}
          aria-label="Send message"
          style={{
            background: isBlocked || !value.trim() ? 'transparent' : 'var(--accent-cyan)',
            border: `1px solid ${isBlocked || !value.trim() ? 'var(--border)' : 'var(--accent-cyan)'}`,
            color: isBlocked || !value.trim() ? 'var(--text-secondary)' : 'var(--bg-primary)',
            padding: '5px 14px',
            fontSize: '12px',
            cursor: isBlocked || !value.trim() ? 'not-allowed' : 'pointer',
            borderRadius: '2px',
            fontFamily: 'inherit',
            fontWeight: 600,
            transition: 'background 0.18s, border-color 0.18s, color 0.18s',
            flexShrink: 0,
          }}
        >
          {isLoading ? '…' : 'send'}
        </button>
      </div>
      {pipelineRunning && (
        <div style={{
          marginTop: '6px',
          fontSize: '11px',
          color: 'var(--accent-yellow)',
          paddingLeft: '2px',
        }}>
          pipeline running — input will re-enable when complete
        </div>
      )}
    </div>
  );
}
