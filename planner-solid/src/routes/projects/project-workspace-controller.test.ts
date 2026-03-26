import type { ListSessionsResponse, SessionSummary } from "~/lib/types";

import {
  filterProjectSessionsBySlug,
  resolveProjectWorkspaceSurfaceState,
} from "./project-workspace-route-state";

const session = (overrides: Partial<SessionSummary> = {}): SessionSummary => ({
  id: "session-1",
  title: "Calendar intake",
  archived: false,
  created_at: "2026-03-26T00:00:00Z",
  last_activity_at: "2026-03-26T01:00:00Z",
  pipeline_running: false,
  intake_phase: "waiting",
  project_description: "Calendar planning",
  project_id: "project-1",
  project_slug: "personal-calendar",
  project_name: "Personal Calendar",
  current_step: null,
  error_message: null,
  can_resume_live: false,
  can_resume_checkpoint: false,
  can_restart_from_description: true,
  can_retry_pipeline: false,
  has_checkpoint: false,
  resume_status: "ready_to_start",
  workspace_status: null,
  ...overrides,
});

describe("project workspace controller helpers", () => {
  it("filters sessions to the active project slug and ignores archived sessions", () => {
    const sessions: ListSessionsResponse = {
      sessions: [
        session(),
        session({ id: "archived", archived: true }),
        session({ id: "other-project", project_slug: "other-project", project_id: "project-2" }),
      ],
    };

    expect(filterProjectSessionsBySlug(sessions, "personal-calendar").map(item => item.id)).toEqual([
      "session-1",
    ]);
  });

  it("derives the attached-surface state from the route search param", () => {
    expect(resolveProjectWorkspaceSurfaceState(undefined)).toEqual({
      selectedSurfaceTab: null,
      advancedOpen: false,
      advancedTab: "review",
    });
    expect(resolveProjectWorkspaceSurfaceState("build")).toEqual({
      selectedSurfaceTab: "build",
      advancedOpen: true,
      advancedTab: "build",
    });
    expect(resolveProjectWorkspaceSurfaceState("unknown")).toEqual({
      selectedSurfaceTab: "review",
      advancedOpen: true,
      advancedTab: "review",
    });
  });
});
