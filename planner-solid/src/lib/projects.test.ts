import {
  buildProjectWorkSummaries,
  selectGuidedEntryProject,
  selectPrimarySession,
  summarizeProjectWork,
} from "./projects";
import type { Project, SessionSummary } from "./types";

const project = (overrides: Partial<Project> = {}): Project => ({
  id: "project-1",
  slug: "calendar-app",
  name: "Calendar App",
  description: "Personal calendar app with task tracking.",
  owner_user_id: "dev|local",
  team_label: null,
  created_at: "2026-03-24T00:00:00Z",
  updated_at: "2026-03-24T01:00:00Z",
  archived_at: null,
  legacy_scope_keys: [],
  ...overrides,
});

const session = (overrides: Partial<SessionSummary> = {}): SessionSummary => ({
  id: "session-1",
  title: "Calendar intake",
  archived: false,
  created_at: "2026-03-24T00:00:00Z",
  last_activity_at: "2026-03-24T01:05:00Z",
  pipeline_running: false,
  intake_phase: "waiting",
  project_description: "Personal calendar app with task tracking.",
  project_id: "project-1",
  project_slug: "calendar-app",
  project_name: "Calendar App",
  current_step: null,
  error_message: null,
  can_resume_live: false,
  can_resume_checkpoint: false,
  can_restart_from_description: false,
  can_retry_pipeline: false,
  has_checkpoint: false,
  resume_status: "ready_to_start",
  ...overrides,
});

describe("project work helpers", () => {
  it("prefers interviewing sessions as the primary analysis target", () => {
    const selected = selectPrimarySession([
      session({ id: "recent", intake_phase: "complete", last_activity_at: "2026-03-24T01:05:00Z" }),
      session({ id: "live", intake_phase: "interviewing", last_activity_at: "2026-03-24T00:55:00Z" }),
    ]);

    expect(selected?.id).toBe("live");
  });

  it("summarizes project work as active when Socratic analysis is in progress", () => {
    const summary = summarizeProjectWork(project(), [
      session({ intake_phase: "interviewing" }),
    ]);

    expect(summary.status).toBe("active");
    expect(summary.statusLabel).toBe("Active Socratic analysis");
    expect(summary.nextActionLabel).toBe("Continue analysis");
  });

  it("sorts project summaries by most relevant recent work", () => {
    const summaries = buildProjectWorkSummaries(
      [
        project({ id: "project-1", slug: "calendar-app", name: "Calendar App" }),
        project({ id: "project-2", slug: "finance-app", name: "Finance App", updated_at: "2026-03-24T02:00:00Z" }),
      ],
      [
        session({ id: "calendar-live", project_slug: "calendar-app", project_id: "project-1", intake_phase: "interviewing", last_activity_at: "2026-03-24T03:00:00Z" }),
      ],
    );

    expect(summaries[0]?.project.slug).toBe("calendar-app");
    expect(selectGuidedEntryProject(summaries)?.project.slug).toBe("calendar-app");
  });
});
