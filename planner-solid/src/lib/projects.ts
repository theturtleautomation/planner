import type { Project, SessionSummary } from "./types";
import { getSessionSummaryStatus, getWorkspaceStatus } from "./session-status";

export type ProjectWorkStatus =
  | "active"
  | "attention"
  | "recent"
  | "quiet";

export interface ProjectWorkSummary {
  project: Project;
  sessions: SessionSummary[];
  primarySession: SessionSummary | null;
  sessionCount: number;
  status: ProjectWorkStatus;
  statusLabel: string;
  nextActionLabel: string;
}

function timestampValue(value: string | undefined | null): number {
  if (!value) return 0;
  const parsed = Date.parse(value);
  return Number.isNaN(parsed) ? 0 : parsed;
}

export function sortSessionsByRecent(sessions: SessionSummary[]): SessionSummary[] {
  return [...sessions].sort((left, right) => {
    return timestampValue(right.last_activity_at) - timestampValue(left.last_activity_at);
  });
}

export function groupSessionsByProject(
  sessions: SessionSummary[],
): Record<string, SessionSummary[]> {
  return sessions.reduce<Record<string, SessionSummary[]>>((acc, session) => {
    const key = session.project_slug ?? session.project_id ?? "";
    if (!key) return acc;
    if (!acc[key]) acc[key] = [];
    acc[key].push(session);
    return acc;
  }, {});
}

export function selectPrimarySession(sessions: SessionSummary[]): SessionSummary | null {
  const activeSessions = sessions.filter(session => !session.archived);
  if (activeSessions.length === 0) return sessions[0] ?? null;

  const rankSession = (session: SessionSummary): number => {
    switch (getWorkspaceStatus(session).state) {
      case "attention_required":
        return 80;
      case "awaiting_response":
        return 70;
      case "starting_analysis":
      case "classifying":
      case "assembling_prompt_bank":
        return 60;
      case "pipeline_running":
        return 50;
      case "build_ready":
        return 40;
      case "ready_to_start":
        return session.project_description?.trim() ? 30 : 10;
      case "complete":
        return 20;
      default:
        return 0;
    }
  };

  return [...activeSessions].sort((left, right) => {
    const priorityDelta = rankSession(right) - rankSession(left);
    if (priorityDelta !== 0) return priorityDelta;
    return timestampValue(right.last_activity_at) - timestampValue(left.last_activity_at);
  })[0] ?? null;
}

export function summarizeProjectWork(
  project: Project,
  sessions: SessionSummary[],
): ProjectWorkSummary {
  const activeSessions = sessions.filter(session => !session.archived);
  const primarySession = selectPrimarySession(activeSessions);

  let status: ProjectWorkStatus = "quiet";
  let statusLabel = "Ready to start";
  let nextActionLabel = "Start analysis";

  if (primarySession) {
    const summaryStatus = getSessionSummaryStatus(primarySession);
    statusLabel = summaryStatus.label;
    nextActionLabel = summaryStatus.nextActionLabel;

    switch (summaryStatus.state) {
      case "attention_required":
      case "pipeline_running":
      case "build_ready":
        status = "attention";
        break;
      case "awaiting_response":
      case "starting_analysis":
      case "classifying":
      case "assembling_prompt_bank":
        status = "active";
        break;
      case "ready_to_start":
      case "complete":
      default:
        status = "recent";
        break;
    }
  }

  return {
    project,
    sessions: sortSessionsByRecent(activeSessions),
    primarySession,
    sessionCount: activeSessions.length,
    status,
    statusLabel,
    nextActionLabel,
  };
}

export function buildProjectWorkSummaries(
  projects: Project[],
  sessions: SessionSummary[],
): ProjectWorkSummary[] {
  const grouped = groupSessionsByProject(sessions);
  return [...projects]
    .filter(project => !project.archived_at)
    .map(project => summarizeProjectWork(project, grouped[project.slug] ?? grouped[project.id] ?? []))
    .sort((left, right) => {
      const rightPrimary = timestampValue(right.primarySession?.last_activity_at ?? right.project.updated_at);
      const leftPrimary = timestampValue(left.primarySession?.last_activity_at ?? left.project.updated_at);
      return rightPrimary - leftPrimary;
    });
}

export function selectGuidedEntryProject(
  summaries: ProjectWorkSummary[],
): ProjectWorkSummary | null {
  return (
    summaries.find(summary => summary.status === "active") ??
    summaries.find(summary => summary.status === "attention") ??
    summaries[0] ??
    null
  );
}
