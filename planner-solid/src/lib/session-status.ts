import type {
  Session,
  SessionSummary,
  WorkspaceStatus,
} from "./types";

export type SessionSocketState = "connecting" | "open" | "closed" | "error";
export type SessionSummarySurfaceTone = "active" | "attention" | "recent" | "quiet";

type SessionStatusSource = Session | SessionSummary;

function fallbackWorkspaceStatus(session: SessionStatusSource): WorkspaceStatus {
  if (session.intake_phase === "error") {
    return {
      state: "attention_required",
      label: "Needs attention",
      detail: session.error_message ?? "The last run stopped before completion.",
      tone: "error",
    };
  }

  if (session.intake_phase === "complete") {
    return {
      state: "complete",
      label: "Plan complete",
      detail: "The session is complete and ready for review.",
      tone: "success",
    };
  }

  if (session.intake_phase === "pipeline_running") {
    return {
      state: "pipeline_running",
      label: "Running the build pipeline",
      detail: "Planner is turning the current session state into build artifacts.",
      tone: "active",
    };
  }

  if (session.intake_phase === "interviewing") {
    if (session.has_checkpoint) {
      return {
        state: "awaiting_response",
        label: "Waiting for your response",
        detail: "A saved prompt checkpoint is ready to resume.",
        tone: "neutral",
      };
    }

    return {
      state: "starting_analysis",
      label: "Starting analysis",
      detail: "Waiting for the live Socratic runtime to begin from the saved brief.",
      tone: "active",
    };
  }

  return {
    state: "ready_to_start",
    label: "Ready to start analysis",
    detail: session.project_description?.trim()
      ? "Waiting for the session workspace to begin from the saved brief."
      : "This session does not have a saved brief yet.",
    tone: "neutral",
  };
}

export function getWorkspaceStatus(session: SessionStatusSource): WorkspaceStatus {
  return session.workspace_status ?? fallbackWorkspaceStatus(session);
}

export function getSessionSummaryStatus(session: SessionStatusSource): {
  state: WorkspaceStatus["state"];
  label: string;
  detail: string | null;
  tone: WorkspaceStatus["tone"];
  nextActionLabel: string;
} {
  const status = getWorkspaceStatus(session);
  const hasSavedBrief = Boolean(session.project_description?.trim());

  switch (status.state) {
    case "ready_to_start":
      return {
        state: status.state,
        label: hasSavedBrief ? "Ready to start" : "Needs saved brief",
        detail: status.detail ?? null,
        tone: hasSavedBrief ? status.tone : "warning",
        nextActionLabel: hasSavedBrief ? "Open project" : "Add saved brief",
      };
    case "starting_analysis":
    case "classifying":
    case "assembling_prompt_bank":
      return {
        state: status.state,
        label: "Starting analysis",
        detail: status.detail ?? null,
        tone: "active",
        nextActionLabel: "Continue analysis",
      };
    case "awaiting_response":
      return {
        state: status.state,
        label: "Waiting for response",
        detail: status.detail ?? null,
        tone: status.tone,
        nextActionLabel: "Continue analysis",
      };
    case "build_ready":
      return {
        state: status.state,
        label: "Build ready",
        detail: status.detail ?? null,
        tone: status.tone,
        nextActionLabel: "Open project",
      };
    case "pipeline_running":
      return {
        state: status.state,
        label: "Pipeline running",
        detail: status.detail ?? null,
        tone: status.tone,
        nextActionLabel: "Open project",
      };
    case "complete":
      return {
        state: status.state,
        label: "Plan complete",
        detail: status.detail ?? null,
        tone: status.tone,
        nextActionLabel: "Review project",
      };
    case "attention_required":
      return {
        state: status.state,
        label: "Needs attention",
        detail: status.detail ?? null,
        tone: status.tone,
        nextActionLabel: "Open project",
      };
    default:
      return {
        state: status.state,
        label: status.label,
        detail: status.detail ?? null,
        tone: status.tone,
        nextActionLabel: "Open project",
      };
  }
}

export function getSessionSummarySurfaceTone(
  session: SessionStatusSource,
): SessionSummarySurfaceTone {
  switch (getSessionSummaryStatus(session).state) {
    case "attention_required":
      return "attention";
    case "awaiting_response":
    case "build_ready":
    case "complete":
      return "active";
    case "starting_analysis":
    case "classifying":
    case "assembling_prompt_bank":
    case "pipeline_running":
      return "recent";
    default:
      return "quiet";
  }
}

export function getSessionStatusCopy(
  session: SessionStatusSource,
  socketState: SessionSocketState,
): { label: string; detail: string | null; tone: WorkspaceStatus["tone"] } {
  const status = getWorkspaceStatus(session);
  const hasSavedBrief = Boolean(session.project_description?.trim());

  if (
    socketState === "connecting"
    && ((status.state === "ready_to_start" && hasSavedBrief)
      || status.state === "starting_analysis"
      || status.state === "classifying"
      || status.state === "assembling_prompt_bank")
  ) {
    return {
      label: "Starting analysis",
      detail: "Opening the live Socratic connection.",
      tone: "active",
    };
  }

  if (
    socketState === "open"
    && ((status.state === "ready_to_start" && hasSavedBrief)
      || status.state === "starting_analysis"
      || status.state === "classifying")
  ) {
    return {
      label: "Starting analysis",
      detail: status.detail ?? "The first Socratic runtime is now live.",
      tone: "active",
    };
  }

  if (
    socketState === "error"
    && ((status.state === "ready_to_start" && hasSavedBrief)
      || status.state === "starting_analysis"
      || status.state === "classifying"
      || status.state === "assembling_prompt_bank")
  ) {
    return {
      label: "Connection needs attention",
      detail: "The live Socratic connection dropped before startup could finish.",
      tone: "warning",
    };
  }

  return {
    label: status.label,
    detail: status.detail ?? null,
    tone: status.tone,
  };
}

export function shouldOpenSessionSocket(session: Session): boolean {
  const description = session.project_description?.trim();
  if (!description) {
    return session.intake_phase === "interviewing";
  }

  if (session.intake_phase === "waiting" && session.resume_status === "ready_to_start") {
    return true;
  }

  return session.intake_phase === "interviewing";
}

export function shouldSendStartupHandshake(session: Session): boolean {
  const description = session.project_description?.trim();
  if (!description) return false;
  if (session.has_checkpoint) return false;

  const status = getWorkspaceStatus(session).state;
  return status === "ready_to_start"
    || status === "starting_analysis"
    || status === "classifying";
}

export function canRetryStartup(session: Session, socketState: SessionSocketState): boolean {
  const description = session.project_description?.trim();
  if (!description) return false;

  const status = getWorkspaceStatus(session).state;
  if (
    !session.has_checkpoint
    && socketState === "error"
    && (status === "ready_to_start"
      || status === "starting_analysis"
      || status === "classifying"
      || status === "assembling_prompt_bank")
  ) {
    return true;
  }

  return session.intake_phase === "error" && status === "attention_required";
}

export function needsSavedBriefAction(session: Session): boolean {
  return getWorkspaceStatus(session).state === "ready_to_start"
    && !session.project_description?.trim();
}
