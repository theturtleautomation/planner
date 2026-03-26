import type { Session } from "~/lib/types";

import {
  formatSavedLabel,
  getSessionReturnTarget,
  viewportClassFromWidth,
} from "./session-workspace-view";

const session = (overrides: Partial<Session> = {}): Session => ({
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

describe("session workspace view helpers", () => {
  it("maps viewport widths into the same route layout buckets", () => {
    expect(viewportClassFromWidth(375)).toBe("mobile");
    expect(viewportClassFromWidth(800)).toBe("tablet");
    expect(viewportClassFromWidth(1280)).toBe("desktop");
  });

  it("keeps return navigation project-aware without losing standalone sessions", () => {
    expect(getSessionReturnTarget(session())).toEqual({
      href: "/projects/personal-calendar",
      label: "Back to project",
    });
    expect(
      getSessionReturnTarget(
        session({
          project_id: null,
          project_slug: null,
          project_name: null,
        }),
      ),
    ).toEqual({
      href: "/sessions",
      label: "Back to sessions",
    });
  });

  it("preserves the draft save copy ladder", () => {
    expect(formatSavedLabel("idle", null)).toBe("Draft ready");
    expect(formatSavedLabel("dirty", null)).toBe("Unsaved changes");
    expect(formatSavedLabel("saved", "Draft cleared")).toBe("Draft cleared");
    expect(formatSavedLabel("error", null)).toBe("Draft save failed");
  });
});
