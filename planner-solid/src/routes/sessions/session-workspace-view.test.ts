import { describe, expect, it } from "vitest";

import type { Session } from "~/lib/types";
import type { PromptBankThread, QueuedPromptThread } from "~/lib/types";

import {
  areaWorkspacePressurePoints,
  deriveAreaShapingObjects,
  deriveProjectPictureAreas,
  firstRevealPressurePoints,
  formatSavedLabel,
  getSessionReturnTarget,
  previewProjectPictureArea,
  selectRecommendedProjectArea,
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

const bankedThread = (
  categoryId: string,
  title: string,
  summary: string,
  itemId: string,
  targetDimension?: string | null,
): PromptBankThread => ({
  category_id: categoryId,
  title,
  summary,
  question_count: 1,
  prompt: {
    prompt_id: `prompt-${categoryId}`,
    title,
    kind: "question_batch",
    origin_category_id: categoryId,
    items: [
      {
        item_id: itemId,
        kind: "discovery",
        text: summary,
        options: [],
        required: true,
        target_dimension: targetDimension ?? null,
      },
    ],
    allow_partial_submit: true,
  },
});

const queuedThread = (categoryId: string, title: string, summary: string): QueuedPromptThread => ({
  category_id: categoryId,
  title,
  summary,
  question_count: 1,
  status: "queued",
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

  it("derives the project-picture areas from the current session and prompt bank", () => {
    const areas = deriveProjectPictureAreas(
      session({ project_description: "A local-first planning desk for weekly work." }),
      [
        bankedThread("workflow", "Workflow", "What is the main user flow?", "item-1", "goal"),
        bankedThread("scope", "Scope", "What should the first release avoid?", "item-2", "out_of_scope"),
      ],
      [{
        ...queuedThread("delivery", "North-star revision", "Promote review mode as the primary goal."),
        revision_kind: "north_star",
        revision_area_id: "transformation",
      }],
      { "item-1": true },
    );

    expect(areas.map(area => area.title)).toEqual([
      "Transformation",
      "Actors",
      "Constraints",
      "Approach",
      "Pressure",
    ]);
    expect(areas.find(area => area.id === "transformation")?.state).toBe("defined");
    expect(areas.find(area => area.id === "constraints")?.state).toBe("incomplete");
    expect(areas.find(area => area.id === "actors")?.state).toBe("unclear");
    expect(areas.find(area => area.id === "transformation")?.pendingRevisions).toHaveLength(1);
  });

  it("picks the highest-leverage area as the recommended next move", () => {
    const areas = deriveProjectPictureAreas(
      session(),
      [bankedThread("workflow", "Workflow", "What is the main user flow?", "item-1", "goal")],
      [],
      {},
    );

    expect(selectRecommendedProjectArea(areas)?.id).toBe("transformation");
  });

  it("caps the first-reveal pressure preview at one dominant and two secondary points", () => {
    const areas = deriveProjectPictureAreas(
      session(),
      [
        bankedThread("workflow", "Workflow", "What is the main user flow?", "item-1", "goal"),
        bankedThread("delivery", "Delivery", "What should ship first?", "item-2", "goal"),
        bankedThread("handoff", "Handoff", "What needs to stay local?", "item-3", "goal"),
        bankedThread("metrics", "Metrics", "How will progress be measured?", "item-4", "goal"),
      ],
      [],
      {},
    );
    const preview = previewProjectPictureArea(selectRecommendedProjectArea(areas));

    expect(preview.dominant?.title).toBe("Workflow");
    expect(preview.secondary).toHaveLength(2);
    expect(firstRevealPressurePoints(selectRecommendedProjectArea(areas))).toHaveLength(3);
  });

  it("caps deeper area pressure points at four while remaining truthful for smaller sets", () => {
    const fullAreas = deriveProjectPictureAreas(
      session(),
      [
        bankedThread("workflow", "Workflow", "Main flow", "item-1", "goal"),
        bankedThread("delivery", "Delivery", "Ship first", "item-2", "goal"),
        bankedThread("handoff", "Handoff", "Keep local", "item-3", "goal"),
        bankedThread("metrics", "Metrics", "Measure progress", "item-4", "goal"),
        bankedThread("review", "Review", "Check readiness", "item-5", "goal"),
      ],
      [],
      {},
    );

    expect(areaWorkspacePressurePoints(selectRecommendedProjectArea(fullAreas))).toHaveLength(4);

    const smallAreas = deriveProjectPictureAreas(
      session(),
      [bankedThread("workflow", "Workflow", "Main flow", "item-1", "goal")],
      [],
      {},
    );

    expect(areaWorkspacePressurePoints(selectRecommendedProjectArea(smallAreas))).toHaveLength(1);
  });

  it("derives the minimum object-first editing set for deeper area shaping", () => {
    const areas = deriveProjectPictureAreas(
      session(),
      [
        bankedThread("workflow", "Workflow", "Main flow", "item-1", "goal"),
        bankedThread("scope", "Scope", "Avoid broad first release", "item-2", "out_of_scope"),
      ],
      [],
      {},
    );

    const objects = deriveAreaShapingObjects(areas.find(area => area.id === "constraints"));

    expect(objects.map(object => object.kind)).toEqual(["label", "claim", "constraint"]);
    expect(objects.find(object => object.kind === "label")?.value).toBe("Constraints");
    expect(objects.find(object => object.kind === "claim")?.value).toBe("Avoid broad first release");
    expect(objects.find(object => object.kind === "constraint")?.value).toBe("Avoid broad first release");
  });

  it("treats typed queued threads as local pending revisions while ignoring low-risk updates", () => {
    const areas = deriveProjectPictureAreas(
      session(),
      [bankedThread("workflow", "Workflow", "Main flow", "item-1", "goal")],
      [
        {
          ...queuedThread("rename", "Area rename", "Rename Transformation to Planning Engine"),
          revision_kind: "area_identity",
          revision_area_id: "transformation",
        },
        {
          ...queuedThread("relationship", "Relationship conflict", "Reverse the accepted relationship between Transformation and Approach"),
          revision_kind: "major_relationship",
          revision_area_id: "transformation",
          revision_conflict: true,
        },
        {
          ...queuedThread("north-star", "North-star revision", "Promote review mode as the primary goal."),
          revision_kind: "north_star",
          revision_area_id: "transformation",
        },
        {
          ...queuedThread("promotion", "Direction promotion", "Promote the draft direction into the canonical path."),
          revision_kind: "direction_promotion",
          revision_area_id: "transformation",
        },
        {
          ...queuedThread("freshness", "Confidence refresh", "Raise confidence after recent answers"),
          low_risk_update: true,
        },
        queuedThread("generic", "Layout note", "Minor copy cleanup for the supporting panel"),
      ],
      {},
    );

    const transformation = areas.find(area => area.id === "transformation");
    expect(transformation?.pendingRevisions).toHaveLength(4);
    expect(transformation?.pendingRevisions.map(revision => revision.kindLabel)).toEqual([
      "Area identity",
      "Major relationship",
      "North-star",
      "Direction promotion",
    ]);
    expect(transformation?.pendingRevisions.some(revision => revision.conflict)).toBe(true);
    expect(transformation?.pendingRevisions.every(revision => !("blocking" in revision))).toBe(true);
    expect(transformation?.pendingRevisions.some(revision => revision.title === "Confidence refresh")).toBe(false);
    expect(transformation?.pendingRevisions.some(revision => revision.title === "Layout note")).toBe(false);
    expect(transformation?.summary).toBe("Calendar planning");
    expect(transformation?.relationshipLabels).toContain("Guides Approach");
  });
});
