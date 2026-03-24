import { useEffect, useRef } from 'react';

interface SeamlessInputProps {
  value: string;
  onChange: (nextValue: string) => void;
  onBlur?: () => void;
  placeholder?: string;
  ariaLabel: string;
  disabled?: boolean;
  autoFocus?: boolean;
  rows?: number;
}

export default function SeamlessInput({
  value,
  onChange,
  onBlur,
  placeholder = 'Type your answer',
  ariaLabel,
  disabled = false,
  autoFocus = false,
  rows = 1,
}: SeamlessInputProps) {
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);

  useEffect(() => {
    const textarea = textareaRef.current;
    if (!textarea) return;
    textarea.style.height = 'auto';
    textarea.style.height = `${Math.max(textarea.scrollHeight, 56)}px`;
  }, [value]);

  useEffect(() => {
    if (!autoFocus || disabled) return;
    textareaRef.current?.focus();
  }, [autoFocus, disabled]);

  return (
    <textarea
      ref={textareaRef}
      value={value}
      onChange={(event) => onChange(event.target.value)}
      onBlur={onBlur}
      placeholder={placeholder}
      rows={rows}
      disabled={disabled}
      autoFocus={autoFocus}
      aria-label={ariaLabel}
      className="socratic-seamless-input"
    />
  );
}
