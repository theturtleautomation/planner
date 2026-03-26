import {
  canRetryStartup,
  getSessionSummaryStatus,
  getSessionSummarySurfaceTone,
  getSessionStatusCopy,
  needsSavedBriefAction,
  shouldOpenSessionSocket,
  shouldSendStartupHandshake,
} from "./session-status";
import type { Session } from "./types";

const baseSession = (overrides: Partial<Session> = {}): Session => ({
  id: "session-1",
  title: "Calendar intake",
  archived: false,
  created_at: "2026-03-25T00:00:00Z",
  last_activity_at: "2026-03-25T00:05:00Z",
  pipeline_running: false,
  intake_phase: "waiting",
  project_description: "Personal calendar app with task tracking",
  project_id: "project-1",
  project_slug: "personal-calendar",
  project_name: "Personal Calendar",
  current_step: null,
  error_message: null,
  can_resume_live: false,
  can_resume_checkpoint: false,
  can_restart_from_description: false,
  can_retry_pipeline: false,
  has_checkpoint: false,
  resume_status: "ready_to_start",
  workspace_status: {
    state: "ready_to_start",
    label: "Ready to start analysis",
    detail: "Waiting for the session workspace to begin from the saved brief.",
    tone: "neutral",
  },
  ...overrides,
});

describe("session status helpers", () => {
  it("opens the websocket for waiting sessions that already have a saved brief", () => {
    expect(shouldOpenSessionSocket(baseSession())).toBe(true);
    expect(shouldSendStartupHandshake(baseSession())).toBe(true);
    expect(getSessionStatusCopy(baseSession(), "connecting").label).toBe("Starting analysis");
  });

  it("projects concise summary language from the truthful workspace status", () => {
    const summary = getSessionSummaryStatus(baseSession());

    expect(summary.label).toBe("Ready to start");
    expect(summary.nextActionLabel).toBe("Open project");
    expect(getSessionSummarySurfaceTone(baseSession())).toBe("quiet");
  });

  it("does not send a fresh startup handshake when a checkpoint already exists", () => {
    const session = baseSession({
      intake_phase: "interviewing",
      has_checkpoint: true,
      resume_status: "interview_checkpoint_resumable",
      workspace_status: {
        state: "awaiting_response",
        label: "Waiting for your response",
        detail: "The first prompt bank is ready for local switching and answers.",
        tone: "neutral",
      },
    });

    expect(shouldOpenSessionSocket(session)).toBe(true);
    expect(shouldSendStartupHandshake(session)).toBe(false);
    expect(getSessionSummaryStatus(session).label).toBe("Waiting for response");
    expect(getSessionSummarySurfaceTone(session)).toBe("active");
  });

  it("surfaces socket startup as a secondary detail without changing the backend label", () => {
    const copy = getSessionStatusCopy(
      baseSession({
        intake_phase: "interviewing",
        workspace_status: {
          state: "starting_analysis",
          label: "Starting analysis",
          detail: "Waiting for the live Socratic runtime to begin from the saved brief.",
          tone: "active",
        },
      }),
      "connecting",
    );

    expect(copy.label).toBe("Starting analysis");
    expect(copy.detail).toBe("Opening the live Socratic connection.");
  });

  it("flags the explicit idle action only when the session truly lacks a saved brief", () => {
    const session = baseSession({
      project_description: null,
      workspace_status: {
        state: "ready_to_start",
        label: "Ready to start analysis",
        detail: "This session does not have a saved brief yet.",
        tone: "neutral",
      },
    });

    expect(needsSavedBriefAction(session)).toBe(true);
    expect(shouldOpenSessionSocket(session)).toBe(false);
  });

  it("allows a startup retry when the socket drops before first reveal", () => {
    expect(canRetryStartup(baseSession(), "error")).toBe(true);
  });

  it("allows a startup retry after an early backend startup failure", () => {
    const session = baseSession({
      intake_phase: "error",
      workspace_status: {
        state: "attention_required",
        label: "Needs attention",
        detail: "Socratic interview failed: startup mock failure",
        tone: "error",
      },
    });

    expect(canRetryStartup(session, "closed")).toBe(true);
    expect(getSessionSummaryStatus(session).label).toBe("Needs attention");
    expect(getSessionSummarySurfaceTone(session)).toBe("attention");
  });
});
