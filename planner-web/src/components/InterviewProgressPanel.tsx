import { useEffect, useMemo, useState } from 'react';
import type { PlannerEvent } from '../types.ts';
import {
  findLatestWorkflowEvent,
  formatElapsedSince,
  formatWorkflowStep,
} from '../lib/workflowStatus.ts';

interface InterviewProgressPanelProps {
  currentStep: string | null;
  events: PlannerEvent[];
  isConnected: boolean;
}

function getProgressCopy(step: string | null): { title: string; detail: string } {
  switch (step) {
    case 'system.session.start':
      return {
        title: 'Starting your interview',
        detail: 'Planner is accepting your brief and spinning up the Socratic intake flow.',
      };
    case 'socratic.classify.complete':
      return {
        title: 'Classifying your project',
        detail: 'Planner is identifying the project type and the dimensions it still needs to ask about.',
      };
    case 'socratic.verify.complete':
      return {
        title: 'Reviewing your brief',
        detail: 'Planner is extracting the first concrete requirements and uncertainty from what you already wrote.',
      };
    case 'socratic.response.adjudicated':
      return {
        title: 'Generating your next questions',
        detail: 'Planner is turning that requirement summary into a structured batch of answerable questions.',
      };
    case 'socratic.prompt.generated':
      return {
        title: 'Question batch ready',
        detail: 'The next set of questions has been generated and should appear automatically.',
      };
    default:
      return {
        title: 'Preparing your next questions',
        detail: 'Planner is still working through the intake flow before it can show the next prompt batch.',
      };
  }
}

export default function InterviewProgressPanel({
  currentStep,
  events,
  isConnected,
}: InterviewProgressPanelProps) {
  const [nowMs, setNowMs] = useState(() => Date.now());

  useEffect(() => {
    const intervalId = window.setInterval(() => {
      setNowMs(Date.now());
    }, 1000);
    return () => window.clearInterval(intervalId);
  }, []);

  const latestEvent = useMemo(
    () => findLatestWorkflowEvent(events, currentStep),
    [currentStep, events],
  );
  const progressCopy = getProgressCopy(currentStep);
  const formattedStep = formatWorkflowStep(currentStep);
  const elapsed = formatElapsedSince(latestEvent?.timestamp, nowMs);
  const showLongRunningHint = Boolean(
    latestEvent?.timestamp
      && Date.parse(latestEvent.timestamp) <= nowMs - 20_000,
  );

  return (
    <section
      aria-label="Interview progress"
      style={{
        borderTop: '1px solid var(--color-border)',
        background: 'linear-gradient(180deg, rgba(0,212,255,0.07), rgba(0,212,255,0.03))',
        display: 'flex',
        flexDirection: 'column',
        gap: '12px',
        padding: '16px',
        flexShrink: 0,
      }}
    >
      <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
        <span
          style={{
            fontSize: '11px',
            fontWeight: 700,
            letterSpacing: '0.08em',
            textTransform: 'uppercase',
            color: 'var(--color-primary)',
          }}
        >
          {progressCopy.title}
        </span>
        <p style={{ margin: 0, fontSize: '13px', color: 'var(--color-text)', lineHeight: 1.5 }}>
          {progressCopy.detail}
        </p>
      </div>

      <div
        style={{
          display: 'grid',
          gap: '8px',
          gridTemplateColumns: 'repeat(auto-fit, minmax(220px, 1fr))',
        }}
      >
        <div
          style={{
            border: '1px solid var(--color-border)',
            borderRadius: '4px',
            padding: '10px 12px',
            background: 'var(--color-surface)',
            display: 'flex',
            flexDirection: 'column',
            gap: '4px',
          }}
        >
          <span style={{ fontSize: '10px', color: 'var(--color-text-muted)', letterSpacing: '0.06em', textTransform: 'uppercase' }}>
            Current Step
          </span>
          <span style={{ fontSize: '12px', color: 'var(--color-text)', fontWeight: 600 }}>
            {formattedStep ?? 'Waiting for the next intake update'}
          </span>
          {elapsed && (
            <span style={{ fontSize: '11px', color: 'var(--color-text-muted)' }}>
              Working on this step for {elapsed}
            </span>
          )}
        </div>

        <div
          style={{
            border: '1px solid var(--color-border)',
            borderRadius: '4px',
            padding: '10px 12px',
            background: 'var(--color-surface)',
            display: 'flex',
            flexDirection: 'column',
            gap: '4px',
          }}
        >
          <span style={{ fontSize: '10px', color: 'var(--color-text-muted)', letterSpacing: '0.06em', textTransform: 'uppercase' }}>
            Live Updates
          </span>
          <span style={{ fontSize: '12px', color: isConnected ? 'var(--color-success)' : 'var(--color-gold)', fontWeight: 600 }}>
            {isConnected ? 'Connected to live interview runtime' : 'Waiting for the next live update'}
          </span>
          <span style={{ fontSize: '11px', color: 'var(--color-text-muted)' }}>
            The Events tab shows the full trace as planner events arrive.
          </span>
        </div>
      </div>

      {latestEvent && (
        <div
          style={{
            border: '1px solid rgba(0,212,255,0.28)',
            borderRadius: '4px',
            padding: '10px 12px',
            background: 'rgba(0,212,255,0.05)',
            display: 'flex',
            flexDirection: 'column',
            gap: '4px',
          }}
        >
          <span style={{ fontSize: '10px', color: 'var(--color-text-muted)', letterSpacing: '0.06em', textTransform: 'uppercase' }}>
            Latest Update
          </span>
          <span style={{ fontSize: '12px', color: 'var(--color-text)' }}>
            {latestEvent.message}
          </span>
          {showLongRunningHint && (
            <span style={{ fontSize: '11px', color: 'var(--color-text-muted)' }}>
              This step is taking longer than usual, but Planner is still actively working.
            </span>
          )}
        </div>
      )}
    </section>
  );
}
