import type {
  PromptBankThread,
  QueuedPromptThread,
  Session,
} from "~/lib/types";
import { countProcessedPromptItems } from "~/lib/workspace";
import { withFrontendMockSearch } from "~/lib/mock/runtime";

export type DraftSaveState = "idle" | "dirty" | "saving" | "saved" | "error";
export type SurfaceTab = "interview" | "artifact";
export type ViewportClass = "mobile" | "tablet" | "desktop";
export type ProjectPictureAreaId =
  | "transformation"
  | "actors"
  | "constraints"
  | "approach"
  | "pressure";
export type ProjectPictureAreaState = "defined" | "incomplete" | "unclear" | "conflicted";

export interface ProjectPicturePressurePoint {
  threadId: string;
  title: string;
  summary: string;
  questionCount: number;
  answeredCount: number;
  state: ProjectPictureAreaState;
}

export interface ProjectPicturePendingRevision {
  id: string;
  title: string;
  summary: string;
  kind: "area_identity" | "major_relationship" | "north_star" | "direction_promotion";
  kindLabel: string;
  conflict: boolean;
}

export interface ProjectPictureArea {
  id: ProjectPictureAreaId;
  title: string;
  summary: string;
  state: ProjectPictureAreaState;
  pressurePoints: ProjectPicturePressurePoint[];
  pendingRevisions: ProjectPicturePendingRevision[];
  relationshipLabels: string[];
  signature: string;
}

export interface ProjectPictureAreaPreview {
  dominant: ProjectPicturePressurePoint | null;
  secondary: ProjectPicturePressurePoint[];
}

export type AreaShapingObjectKind = "label" | "claim" | "constraint";

export interface AreaShapingObject {
  kind: AreaShapingObjectKind;
  title: string;
  value: string;
  helper: string;
}

export const FIRST_REVEAL_PRIMARY_PRESSURE_LIMIT = 1;
export const FIRST_REVEAL_SECONDARY_PRESSURE_LIMIT = 2;
export const FIRST_REVEAL_PRESSURE_LIMIT =
  FIRST_REVEAL_PRIMARY_PRESSURE_LIMIT + FIRST_REVEAL_SECONDARY_PRESSURE_LIMIT;
export const AREA_WORKSPACE_MAX_PRESSURE_POINTS = 4;

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

export function shouldShowQuestionSaveState(state: DraftSaveState) {
  return state !== "idle";
}

export function getSessionReturnTarget(session: Session): SessionReturnTarget {
  if (session.project_slug) {
    return {
      href: withFrontendMockSearch(`/projects/${session.project_slug}`),
      label: "Back to project",
    };
  }

  return {
    href: withFrontendMockSearch("/sessions"),
    label: "Back to sessions",
  };
}

type ThreadLike = {
  title: string;
  summary: string;
  question_count: number;
  revision_kind?: ProjectPicturePendingRevision["kind"] | null;
  revision_area_id?: ProjectPictureAreaId | null;
  revision_conflict?: boolean;
  low_risk_update?: boolean;
  prompt?: {
    items: Array<{
      kind: string;
      text: string;
      target_dimension?: string | null;
    }>;
  };
  status?: string;
};

const PROJECT_AREA_ORDER: ProjectPictureAreaId[] = [
  "transformation",
  "actors",
  "constraints",
  "approach",
  "pressure",
];

const PROJECT_AREA_TITLES: Record<ProjectPictureAreaId, string> = {
  transformation: "Transformation",
  actors: "Actors",
  constraints: "Constraints",
  approach: "Approach",
  pressure: "Pressure",
};

const DEFAULT_AREA_SUMMARIES: Record<ProjectPictureAreaId, string> = {
  transformation: "The core change the project is trying to make still needs a stable shape.",
  actors: "The people, roles, or audiences most affected by this project are not explicit yet.",
  constraints: "The decisive limits that should shape the project are still forming.",
  approach: "The current shape of the solution is still emerging.",
  pressure: "No immediate structural pressure is visible yet.",
};

function normalizeThreadText(thread: ThreadLike): string {
  const itemBits = thread.prompt?.items.flatMap(item => [
    item.kind,
    item.text,
    item.target_dimension ?? "",
  ]) ?? [];
  return [thread.title, thread.summary, thread.status ?? "", ...itemBits]
    .join(" ")
    .trim()
    .toLowerCase();
}

function includesAny(text: string, tokens: string[]): boolean {
  return tokens.some(token => text.includes(token));
}

function mapThreadToArea(thread: ThreadLike): ProjectPictureAreaId {
  const text = normalizeThreadText(thread);
  const headerText = `${thread.title} ${thread.summary}`.trim().toLowerCase();
  const explicitDimensions = thread.prompt?.items
    .map(item => item.target_dimension?.toLowerCase?.() ?? "")
    .filter(Boolean) ?? [];

  if (
    thread.prompt?.items.some(item => item.kind === "contradiction")
    || includesAny(text, ["conflict", "contradiction", "risk", "blocker", "pressure", "queued"])
  ) {
    return "pressure";
  }

  if (explicitDimensions.some(value => ["stakeholders", "users", "user_flows"].includes(value))) {
    return "actors";
  }

  if (explicitDimensions.some(value => [
    "goal",
    "success_criteria",
    "core_features",
  ].includes(value))) {
    return "transformation";
  }

  if (explicitDimensions.some(value => [
    "auth",
    "security",
    "platform",
    "performance",
    "scalability",
    "data_model",
    "out_of_scope",
  ].includes(value))) {
    return "constraints";
  }

  if (includesAny(headerText, ["actor", "actors", "user", "stakeholder", "audience", "role", "persona", "customer"])) {
    return "actors";
  }

  if (includesAny(headerText, ["scope", "boundary", "boundaries", "constraint", "constraints", "limit"])) {
    return "constraints";
  }

  if (includesAny(headerText, [
    "goal",
    "success",
    "workflow",
    "user flow",
    "flow",
    "outcome",
    "transform",
    "core features",
    "core_features",
  ])) {
    return "transformation";
  }

  if (includesAny(text, ["actor", "actors", "user", "stakeholder", "audience", "role", "persona", "customer"])) {
    return "actors";
  }

  if (includesAny(text, [
    "constraint",
    "limit",
    "auth",
    "security",
    "platform",
    "performance",
    "scalability",
    "data model",
    "data_model",
    "boundary",
    "out of scope",
    "out_of_scope",
    "avoid",
  ])) {
    return "constraints";
  }

  return "approach";
}

function summarizeArea(
  areaId: ProjectPictureAreaId,
  session: Session | null,
  pressurePoints: ProjectPicturePressurePoint[],
  pendingRevisions: ProjectPicturePendingRevision[],
): string {
  if (areaId === "transformation" && session?.project_description?.trim()) {
    return session.project_description.trim();
  }

  if (areaId === "pressure") {
    if (pendingRevisions.length > 0) {
      return `${pendingRevisions.length} system-proposed change${pendingRevisions.length === 1 ? "" : "s"} still need review.`;
    }
    if (pressurePoints.some(point => point.state === "conflicted")) {
      return "A current thread is in direct tension with the accepted shape.";
    }
    if (pressurePoints.some(point => point.state !== "defined")) {
      return "Some important work is still weak enough to affect the whole picture.";
    }
  }

  if (pressurePoints.length > 0) {
    return pressurePoints[0]!.summary;
  }

  if (pendingRevisions.length > 0) {
    return `${pendingRevisions.length} pending revision${pendingRevisions.length === 1 ? "" : "s"} waiting local review.`;
  }

  return DEFAULT_AREA_SUMMARIES[areaId];
}

function areaRelationshipLabels(areaId: ProjectPictureAreaId): string[] {
  switch (areaId) {
    case "transformation":
      return ["Guides Approach"];
    case "actors":
      return ["Shapes Transformation"];
    case "constraints":
      return ["Limits Approach"];
    case "approach":
      return ["Depends on Transformation", "Depends on Constraints"];
    case "pressure":
      return ["Challenges current shape"];
    default:
      return [];
  }
}

function pendingRevisionKindLabel(kind: ProjectPicturePendingRevision["kind"]): string {
  switch (kind) {
    case "area_identity":
      return "Area identity";
    case "major_relationship":
      return "Major relationship";
    case "north_star":
      return "North-star";
    case "direction_promotion":
      return "Direction promotion";
    default:
      return "Pending revision";
  }
}

function classifyPendingRevision(thread: ThreadLike): {
  areaId: ProjectPictureAreaId;
  kind: ProjectPicturePendingRevision["kind"];
  conflict: boolean;
} | null {
  if (thread.low_risk_update) return null;

  const text = normalizeThreadText(thread);
  const runtimeKind = thread.revision_kind ?? null;
  const runtimeAreaId = thread.revision_area_id ?? null;
  const runtimeConflict = thread.revision_conflict;

  // Temporary compatibility fallback while runtime metadata rolls out everywhere.
  const fallbackKind = includesAny(text, ["north-star", "north star", "primary goal", "core transformation", "project definition"])
    ? "north_star"
    : includesAny(text, ["relationship", "depends on", "shapes", "guides", "reverse"])
      ? "major_relationship"
      : includesAny(text, ["promote", "canonical", "direction change", "intended path"])
        ? "direction_promotion"
        : includesAny(text, ["rename", "reframe", "identity"])
          ? "area_identity"
          : null;

  const kind = runtimeKind ?? fallbackKind;

  if (!kind) return null;

  return {
    areaId: (runtimeAreaId as ProjectPictureAreaId | null) ?? mapThreadToArea(thread),
    kind,
    conflict: runtimeConflict
      ?? (thread.status === "blocked"
        || includesAny(text, ["conflict", "contradiction", "tension", "blocked"])),
  };
}

function pressurePointState(
  thread: PromptBankThread,
  processedByItemId: Record<string, boolean | undefined>,
): ProjectPictureAreaState {
  if (thread.prompt.items.some(item => item.kind === "contradiction")) {
    return "conflicted";
  }

  const answeredCount = countProcessedPromptItems(thread.prompt, processedByItemId);
  if (thread.prompt.items.length > 0 && answeredCount >= thread.prompt.items.length) {
    return "defined";
  }

  return "incomplete";
}

function areaState(
  areaId: ProjectPictureAreaId,
  pressurePoints: ProjectPicturePressurePoint[],
  pendingRevisions: ProjectPicturePendingRevision[],
  allAreas: Array<{ id: ProjectPictureAreaId; state: ProjectPictureAreaState }>,
): ProjectPictureAreaState {
  if (pendingRevisions.some(revision => revision.conflict)) {
    return "conflicted";
  }

  if (areaId === "pressure") {
    if (pendingRevisions.length > 0 || pressurePoints.some(point => point.state === "conflicted")) {
      return "conflicted";
    }
    if (allAreas.some(area => area.id !== "pressure" && area.state !== "defined")) {
      return "incomplete";
    }
    return "defined";
  }

  if (pressurePoints.length === 0) {
    if (pendingRevisions.length > 0) {
      return "incomplete";
    }
    return "unclear";
  }

  if (pressurePoints.some(point => point.state === "conflicted")) {
    return "conflicted";
  }

  if (pressurePoints.every(point => point.state === "defined")) {
    return "defined";
  }

  return "incomplete";
}

export function deriveProjectPictureAreas(
  session: Session | null,
  bankedThreads: PromptBankThread[],
  queuedThreads: QueuedPromptThread[],
  processedByItemId: Record<string, boolean | undefined>,
): ProjectPictureArea[] {
  const pressurePointsByArea = new Map<ProjectPictureAreaId, ProjectPicturePressurePoint[]>();
  const pendingRevisionsByArea = new Map<ProjectPictureAreaId, ProjectPicturePendingRevision[]>();

  for (const areaId of PROJECT_AREA_ORDER) {
    pressurePointsByArea.set(areaId, []);
    pendingRevisionsByArea.set(areaId, []);
  }

  for (const thread of bankedThreads) {
    const areaId = mapThreadToArea(thread);
    const answeredCount = countProcessedPromptItems(thread.prompt, processedByItemId);
    pressurePointsByArea.get(areaId)!.push({
      threadId: thread.category_id,
      title: thread.title,
      summary: thread.summary,
      questionCount: thread.prompt.items.length,
      answeredCount,
      state: pressurePointState(thread, processedByItemId),
    });
  }

  for (const thread of queuedThreads) {
    const revision = classifyPendingRevision(thread);
    if (!revision) continue;

    pendingRevisionsByArea.get(revision.areaId)!.push({
      id: thread.category_id,
      title: thread.title,
      summary: thread.summary,
      kind: revision.kind,
      kindLabel: pendingRevisionKindLabel(revision.kind),
      conflict: revision.conflict,
    });
  }

  const preStates: Array<{ id: ProjectPictureAreaId; state: ProjectPictureAreaState }> = PROJECT_AREA_ORDER.map(areaId => ({
    id: areaId,
    state: areaId === "pressure"
      ? (pendingRevisionsByArea.get(areaId)!.length > 0 ? "conflicted" : "defined")
      : (pressurePointsByArea.get(areaId)!.length > 0 ? "incomplete" : "unclear"),
  }));

  return PROJECT_AREA_ORDER.map((areaId) => {
    const pressurePoints = pressurePointsByArea.get(areaId)!;
    const pendingRevisions = pendingRevisionsByArea.get(areaId)!;
    const state = areaState(areaId, pressurePoints, pendingRevisions, preStates);
    const summary = summarizeArea(areaId, session, pressurePoints, pendingRevisions);
    const relationshipLabels = areaRelationshipLabels(areaId);

    return {
      id: areaId,
      title: PROJECT_AREA_TITLES[areaId],
      summary,
      state,
      pressurePoints,
      pendingRevisions,
      relationshipLabels,
      signature: JSON.stringify({
        state,
        summary,
        pressureThreadIds: pressurePoints.map(point => point.threadId),
        pendingRevisionIds: pendingRevisions.map(revision => revision.id),
      }),
    };
  });
}

export function selectRecommendedProjectArea(areas: ProjectPictureArea[]): ProjectPictureArea | null {
  if (areas.length === 0) return null;
  return PROJECT_AREA_ORDER
    .map(id => areas.find(area => area.id === id) ?? null)
    .find((area): area is ProjectPictureArea => !!area && area.state !== "defined")
    ?? areas[0]
    ?? null;
}

export function previewProjectPictureArea(
  area: ProjectPictureArea | null | undefined,
  secondaryLimit = 2,
): ProjectPictureAreaPreview {
  if (!area) {
    return {
      dominant: null,
      secondary: [],
    };
  }

  const [dominant, ...secondary] = area.pressurePoints;
  return {
    dominant: dominant ?? null,
    secondary: secondary.slice(0, Math.max(0, secondaryLimit)),
  };
}

export function firstRevealPressurePoints(
  area: ProjectPictureArea | null | undefined,
  maxVisiblePoints = FIRST_REVEAL_PRESSURE_LIMIT,
): ProjectPicturePressurePoint[] {
  if (!area) return [];
  return area.pressurePoints.slice(0, maxVisiblePoints);
}

export function areaWorkspacePressurePoints(
  area: ProjectPictureArea | null | undefined,
  maxVisiblePoints = AREA_WORKSPACE_MAX_PRESSURE_POINTS,
): ProjectPicturePressurePoint[] {
  if (!area) return [];
  return area.pressurePoints.slice(0, maxVisiblePoints);
}

export function deriveAreaShapingObjects(
  area: ProjectPictureArea | null | undefined,
): AreaShapingObject[] {
  if (!area) return [];

  const visiblePressurePoints = areaWorkspacePressurePoints(area);
  const derivedConstraint = area.id === "constraints"
    ? area.summary
    : visiblePressurePoints[1]?.summary
      ?? visiblePressurePoints[0]?.summary
      ?? area.relationshipLabels[0]
      ?? "Add a concrete limit that should shape this area.";

  return [
    {
      kind: "label",
      title: "Label",
      value: area.title,
      helper: "Sharpen the name so this area reads truthfully at a glance.",
    },
    {
      kind: "claim",
      title: "Claim",
      value: area.summary,
      helper: "Refine what this area currently asserts or intends.",
    },
    {
      kind: "constraint",
      title: "Constraint",
      value: derivedConstraint,
      helper: "Tighten the decisive limit that should shape this area right now.",
    },
  ];
}
