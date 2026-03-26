import { resolveProjectSurfaceTab, type ProjectSurfaceTab } from "~/lib/project-surface";
import type { ListSessionsResponse, SessionSummary } from "~/lib/types";

export interface ProjectWorkspaceSurfaceState {
  selectedSurfaceTab: ProjectSurfaceTab | null;
  advancedOpen: boolean;
  advancedTab: ProjectSurfaceTab;
}

export function filterProjectSessionsBySlug(
  sessions: ListSessionsResponse | undefined,
  projectSlug: string | undefined,
): SessionSummary[] {
  if (!projectSlug) return [];
  return (sessions?.sessions ?? []).filter(
    session => (session.project_slug ?? "") === projectSlug && !session.archived,
  );
}

export function resolveProjectWorkspaceSurfaceState(
  tab: string | null | undefined,
): ProjectWorkspaceSurfaceState {
  const selectedSurfaceTab = resolveProjectSurfaceTab(tab);
  return {
    selectedSurfaceTab,
    advancedOpen: selectedSurfaceTab !== null,
    advancedTab: selectedSurfaceTab ?? "review",
  };
}
