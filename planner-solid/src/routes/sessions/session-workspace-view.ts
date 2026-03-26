import type { Session } from "~/lib/types";

export type DraftSaveState = "idle" | "dirty" | "saving" | "saved" | "error";
export type SurfaceTab = "interview" | "artifact";
export type ViewportClass = "mobile" | "tablet" | "desktop";

export interface SessionReturnTarget {
  href: string;
  label: string;
}

export function viewportClassFromWidth(width: number): ViewportClass {
  if (width < 640) return "mobile";
  if (width < 1024) return "tablet";
  return "desktop";
}

export function formatSavedLabel(state: DraftSaveState, message: string | null) {
  if (state === "saving") return "Saving draft";
  if (state === "saved") return message ?? "Draft saved";
  if (state === "error") return message ?? "Draft save failed";
  if (state === "dirty") return "Unsaved changes";
  return message ?? "Draft ready";
}

export function getSessionReturnTarget(session: Session): SessionReturnTarget {
  if (session.project_slug) {
    return {
      href: `/projects/${session.project_slug}`,
      label: "Back to project",
    };
  }

  return {
    href: "/sessions",
    label: "Back to sessions",
  };
}
