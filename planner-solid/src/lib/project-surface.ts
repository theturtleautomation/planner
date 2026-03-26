import type { AdvancedPanelTab } from "./advanced";

export type ProjectSurfaceTab = AdvancedPanelTab;
export type ProjectSurfaceTone = "active" | "attention" | "recent" | "quiet";

export const PROJECT_SURFACE_DISCLOSURE_LABEL = "Project review, readiness, and advanced surfaces";

export const PROJECT_SURFACE_TAB_OPTIONS: Array<{ value: ProjectSurfaceTab; label: string }> = [
  { value: "review", label: "Review" },
  { value: "readiness", label: "Build readiness" },
  { value: "build", label: "Build path" },
  { value: "execution", label: "Build execution" },
  { value: "outputs", label: "Outputs" },
  { value: "activity", label: "Activity" },
  { value: "knowledge", label: "Knowledge" },
  { value: "blueprint", label: "Blueprint" },
];

const PROJECT_SURFACE_TAB_SET = new Set<ProjectSurfaceTab>(
  PROJECT_SURFACE_TAB_OPTIONS.map(option => option.value),
);

export function isProjectSurfaceTab(value: string): value is ProjectSurfaceTab {
  return PROJECT_SURFACE_TAB_SET.has(value as ProjectSurfaceTab);
}

export function resolveProjectSurfaceTab(value: string | null | undefined): ProjectSurfaceTab | null {
  const normalized = value?.trim();
  if (!normalized) return null;
  return isProjectSurfaceTab(normalized) ? normalized : "review";
}

export function formatProjectSurfaceTimestamp(value: string): string {
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) return value;
  return parsed.toLocaleString([], {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function readinessToneForState(
  state: "ready" | "needs-review" | "in-progress" | "not-started",
): ProjectSurfaceTone {
  switch (state) {
    case "ready":
      return "active";
    case "needs-review":
      return "attention";
    case "in-progress":
      return "recent";
    default:
      return "quiet";
  }
}

export function reviewToneForState(
  state: "pending" | "quiet" | "applied",
): ProjectSurfaceTone {
  switch (state) {
    case "pending":
      return "attention";
    case "applied":
      return "active";
    default:
      return "quiet";
  }
}

export function buildPathToneForState(
  state: "ready" | "blocked" | "staging" | "not-started",
): ProjectSurfaceTone {
  switch (state) {
    case "ready":
      return "active";
    case "blocked":
      return "attention";
    case "staging":
      return "recent";
    default:
      return "quiet";
  }
}

export function buildExecutionToneForState(
  state: "active" | "failed" | "idle" | "complete",
): ProjectSurfaceTone {
  switch (state) {
    case "complete":
      return "active";
    case "failed":
      return "attention";
    case "active":
      return "recent";
    default:
      return "quiet";
  }
}
