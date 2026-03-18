import { useState, useCallback, useEffect, useRef } from 'react';
import type { KeyboardEvent, ChangeEvent } from 'react';

interface MessageInputProps {
  onSend: (content: string) => void;
  disabled?: boolean;
  pipelineRunning?: boolean;
  isLoading?: boolean;
  convergencePct?: number;
  intakePhase?: 'waiting' | 'interviewing' | 'pipeline_running' | 'complete' | 'error';
  onDone?: () => void;
}

export default function MessageInput({
  onSend,
  disabled = false,
  pipelineRunning = false,
  isLoading = false,
  convergencePct = 0,
  intakePhase,
  onDone,
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

  const buildSubmission = useCallback((): string => {
    const trimmed = value.trim();
    return trimmed;
  }, [value]);

  const hasPendingSubmission = Boolean(buildSubmission());

  const phaseBlocked =
    intakePhase === 'pipeline_running' ||
    intakePhase === 'complete' ||
    intakePhase === 'error' ||
    intakePhase === 'waiting';

  const isBlocked = disabled || pipelineRunning || isLoading || phaseBlocked;

  const send = useCallback((): void => {
    const submission = buildSubmission();
    if (!submission) return;
    onSend(submission);
    setValue('');
    // Reset height after clearing
    const el = textareaRef.current;
    if (el) {
      el.style.height = 'auto';
    }
  }, [buildSubmission, onSend]);

  const handleKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>): void => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      if (!isBlocked) {
        send();
      }
    }
  };

  const handleChange = (e: ChangeEvent<HTMLTextAreaElement>): void => {
    setValue(e.target.value);
  };

  // Phase-aware placeholder
  let placeholder = 'Send a message… (Enter to send, Shift+Enter for newline)';
  if (intakePhase === 'waiting') {
    placeholder = 'Describe your planning brief above to begin…';
  } else if (intakePhase === 'interviewing') {
    placeholder = 'Type your answer…';
  } else if (intakePhase === 'pipeline_running' || pipelineRunning) {
    placeholder = 'Pipeline is running — please wait…';
  } else if (intakePhase === 'complete') {
    placeholder = 'Session complete';
  } else if (intakePhase === 'error') {
    placeholder = 'Session error — check event log for details';
  } else if (isLoading) {
    placeholder = 'Waiting for response…';
  }

  const showDone = intakePhase === 'interviewing' && onDone;

  return (
    <div style={{
      padding: '12px 16px',
      background: 'var(--color-surface)',
      borderTop: '1px solid var(--color-border)',
      flexShrink: 0,
    }}>
      {/* Main input row */}
      <div style={{
        display: 'flex',
        gap: '10px',
        alignItems: 'flex-end',
        background: 'var(--color-surface-2)',
        border: `1px solid ${isBlocked ? 'var(--color-border)' : 'var(--color-primary)'}`,
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
            color: isBlocked ? 'var(--color-text-muted)' : 'var(--color-text)',
            fontSize: '13px',
            lineHeight: '1.5',
            resize: 'none',
            cursor: isBlocked ? 'not-allowed' : 'text',
            minHeight: '22px',
            maxHeight: '200px',
            overflowY: 'auto',
            fontFamily: 'inherit',
          }}
        />
        <button
          onClick={send}
          disabled={isBlocked || !hasPendingSubmission}
          aria-label="Send message"
          style={{
            background: isBlocked || !hasPendingSubmission ? 'transparent' : 'var(--color-primary)',
            border: `1px solid ${isBlocked || !hasPendingSubmission ? 'var(--color-border)' : 'var(--color-primary)'}`,
            color: isBlocked || !hasPendingSubmission ? 'var(--color-text-muted)' : 'var(--color-bg)',
            padding: '5px 14px',
            fontSize: '12px',
            cursor: isBlocked || !hasPendingSubmission ? 'not-allowed' : 'pointer',
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

      {/* Done button row (interviewing only) */}
      {showDone && (
        <div style={{
          display: 'flex',
          gap: '8px',
          marginTop: '8px',
        }}>
          <button
            onClick={onDone}
            className={convergencePct >= 80 ? 'done-btn-ready' : undefined}
            aria-label="Done with interview"
            style={{
              background: 'transparent',
              border: '1px solid var(--color-success)',
              borderRadius: '3px',
              color: 'var(--color-success)',
              fontSize: '11px',
              fontFamily: 'inherit',
              letterSpacing: '0.04em',
              padding: '4px 12px',
              cursor: 'pointer',
              transition: 'background 0.15s ease, color 0.15s ease',
            }}
            onMouseEnter={(e) => {
              (e.currentTarget as HTMLButtonElement).style.background = 'rgba(0,255,136,0.08)';
            }}
            onMouseLeave={(e) => {
              (e.currentTarget as HTMLButtonElement).style.background = 'transparent';
            }}
          >
            Done — start building
          </button>
        </div>
      )}

      {/* Legacy pipeline running hint (when intakePhase not provided but pipelineRunning is true) */}
      {pipelineRunning && !intakePhase && (
        <div style={{
          marginTop: '6px',
          fontSize: '11px',
          color: 'var(--color-gold)',
          paddingLeft: '2px',
        }}>
          pipeline running — input will re-enable when complete
        </div>
      )}
    </div>
  );
}
