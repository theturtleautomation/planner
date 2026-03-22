import type { PlannerEvent } from '../types.ts';

const STEP_LABELS: Record<string, string> = {
  'system.session.start': 'Starting interview',
  'socratic.classify.complete': 'Classifying your project',
  'socratic.verify.complete': 'Reviewing your brief',
  'socratic.response.adjudicated': 'Planning the next question batch',
  'socratic.prompt.generated': 'Question batch ready',
  'socratic.prompt.submitted': 'Applying your answers',
  'socratic.prompt.partial_submitted': 'Applying your answers',
  'socratic.prompt.reissued': 'Refreshing the active question batch',
  'socratic.prompt.invalidated': 'Replacing an outdated question batch',
  'socratic.converged': 'Starting the build pipeline',
  'pipeline.wait': 'Waiting for the next pipeline update',
  'pipeline.stage.started': 'Running a pipeline stage',
  'pipeline.stage.completed': 'Pipeline stage completed',
  'pipeline.validation.completed': 'Finalizing validation',
  'pipeline.retry.feedback': 'Preparing retry guidance',
  'pipeline.artifact.persisted': 'Saving generated artifacts',
};

function safeEventTime(raw?: string): number {
  if (!raw) return Number.NEGATIVE_INFINITY;
  const parsed = Date.parse(raw);
  return Number.isNaN(parsed) ? Number.NEGATIVE_INFINITY : parsed;
}

export function formatWorkflowStep(step?: string | null): string | null {
  if (!step?.trim()) return null;
  if (STEP_LABELS[step]) return STEP_LABELS[step];

  return step
    .split('.')
    .map((part) => part.replace(/_/g, ' '))
    .join(' / ');
}

export function findLatestWorkflowEvent(
  events: PlannerEvent[],
  currentStep?: string | null,
): PlannerEvent | null {
  if (events.length === 0) return null;

  if (currentStep) {
    const matching = events
      .filter((event) => event.step === currentStep)
      .sort((a, b) => safeEventTime(b.timestamp) - safeEventTime(a.timestamp));
    if (matching.length > 0) return matching[0];
  }

  return [...events].sort((a, b) => safeEventTime(b.timestamp) - safeEventTime(a.timestamp))[0] ?? null;
}

export function formatElapsedSince(iso?: string | null, nowMs = Date.now()): string | null {
  if (!iso) return null;
  const parsed = Date.parse(iso);
  if (Number.isNaN(parsed)) return null;

  const elapsedMs = Math.max(0, nowMs - parsed);
  if (elapsedMs < 1000) return 'just now';

  const seconds = Math.floor(elapsedMs / 1000);
  if (seconds < 60) return `${seconds}s`;

  const minutes = Math.floor(seconds / 60);
  const remainderSeconds = seconds % 60;
  return remainderSeconds === 0 ? `${minutes}m` : `${minutes}m ${remainderSeconds}s`;
}
