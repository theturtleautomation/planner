import {
  summarizeBlueprint,
  summarizeBuildPath,
  summarizeBuildExecution,
  summarizeBuildReadiness,
  summarizeKnowledge,
  summarizeOutputArtifacts,
  summarizeProjectActivity,
  summarizeReview,
} from "./advanced";
import type { BlueprintResponse, ProjectImportResponse, PromptBankResponse, SessionSummary } from "./types";

const sessionSummary = (overrides: Partial<SessionSummary> = {}): SessionSummary => ({
  id: "session-1",
  title: "Calendar intake",
  archived: false,
  created_at: "2026-03-24T00:00:00Z",
  last_activity_at: "2026-03-24T05:00:00Z",
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
  ...overrides,
});

const blueprint = (overrides: Partial<BlueprintResponse> = {}): BlueprintResponse => ({
  total_nodes: 5,
  total_edges: 4,
  counts: {
    project: 1,
    decision: 1,
    component: 1,
    pattern: 1,
    technology: 1,
  },
  edges: [
    { source: "project-1", target: "decision-1", edge_type: "contains" },
    { source: "decision-1", target: "component-1", edge_type: "decided_by" },
  ],
  nodes: [
    {
      id: "project-1",
      name: "Personal Calendar",
      node_type: "project",
      status: "active",
      scope_class: "project",
      scope_visibility: "project_local",
      is_shared: false,
      lifecycle: "active",
      project_id: "project-1",
      project_name: "Personal Calendar",
      secondary_scope: {},
      linked_project_ids: [],
      tags: [],
      has_documentation: true,
      updated_at: "2026-03-24T04:00:00Z",
    },
    {
      id: "decision-1",
      name: "Web first",
      node_type: "decision",
      status: "accepted",
      scope_class: "project",
      scope_visibility: "project_local",
      is_shared: false,
      lifecycle: "active",
      project_id: "project-1",
      project_name: "Personal Calendar",
      secondary_scope: {},
      linked_project_ids: [],
      tags: [],
      has_documentation: true,
      updated_at: "2026-03-24T05:00:00Z",
    },
    {
      id: "component-1",
      name: "Task Service",
      node_type: "component",
      status: "planned",
      scope_class: "project_contextual",
      scope_visibility: "shared",
      is_shared: true,
      lifecycle: "active",
      project_id: "project-1",
      project_name: "Personal Calendar",
      secondary_scope: { component: "Task Service" },
      linked_project_ids: [],
      tags: [],
      has_documentation: false,
      updated_at: "2026-02-20T05:00:00Z",
    },
  ],
  ...overrides,
});

describe("advanced project helpers", () => {
  it("builds a compact knowledge summary from the project blueprint", () => {
    const summary = summarizeKnowledge(blueprint());
    expect(summary.totalNodes).toBe(3);
    expect(summary.documentedNodes).toBe(2);
    expect(summary.sharedNodes).toBe(1);
    expect(summary.featuredNodes[0]?.node_type).toBe("decision");
  });

  it("builds a structural blueprint summary for the attached advanced surface", () => {
    const summary = summarizeBlueprint(blueprint());
    expect(summary.totalNodes).toBe(3);
    expect(summary.totalEdges).toBe(4);
    expect(summary.decisionNodes).toBe(1);
    expect(summary.componentNodes).toBe(1);
    expect(summary.structuralNodes.length).toBeGreaterThan(0);
  });

  it("prioritizes import review when a project-local review queue exists", () => {
    const importReview: ProjectImportResponse = {
      project: {
        id: "project-1",
        slug: "personal-calendar",
        name: "Personal Calendar",
        description: "Calendar app",
        owner_user_id: "dev|local",
        team_label: null,
        created_at: "2026-03-24T00:00:00Z",
        updated_at: "2026-03-24T05:00:00Z",
        archived_at: null,
        legacy_scope_keys: [],
      },
      import_job: {
        id: "job-1",
        project_id: "project-1",
        provider: "local",
        requested_ref: "/tmp/personal-calendar",
        status: "review_pending",
        analysis_summary: "New import draft detected architecture changes.",
        created_at: "2026-03-24T04:00:00Z",
        updated_at: "2026-03-24T05:00:00Z",
      },
      source_binding: {
        project_id: "project-1",
        provider: "local",
        canonical_ref: "/tmp/personal-calendar",
        managed_checkout: false,
        created_at: "2026-03-24T04:00:00Z",
        updated_at: "2026-03-24T05:00:00Z",
      },
      import_draft: null,
      import_review_selection: {
        job_id: "job-1",
        excluded_node_ids: [],
        included_node_count: 2,
        excluded_node_count: 1,
      },
      review_nodes: [
        { node_id: "node-1", node_name: "Task Service", node_type: "component", included: true },
        { node_id: "node-2", node_name: "Sync Adapter", node_type: "component", included: false },
      ],
    };

    const review = summarizeReview({ importReview });
    expect(review.state).toBe("pending");
    expect(review.pendingCount).toBe(2);
    expect(review.rows[0]?.title).toBe("Task Service");
  });

  it("marks build readiness as blocked when review and queued analysis remain", () => {
    const primarySession: SessionSummary = sessionSummary({
      intake_phase: "interviewing",
      current_step: "socratic.question.generated",
      can_restart_from_description: true,
      resume_status: "interview_restart_only",
    });
    const promptBank: PromptBankResponse = {
      session_id: "session-1",
      active_thread_id: "verify-platform",
      banked_threads: [
        {
          category_id: "verify-platform",
          title: "Verify Platform",
          summary: "Confirm platform",
          question_count: 1,
          prompt: {
            prompt_id: "prompt-1",
            title: "Verify Platform",
            kind: "verification_batch",
            origin_category_id: "verify-platform",
            items: [],
            allow_partial_submit: true,
          },
        },
      ],
      queued_threads: [
        {
          category_id: "user-flows",
          title: "Explore User Flows",
          summary: "Clarify key flows",
          question_count: 1,
          status: "queued",
        },
      ],
      build_ready: false,
      build_readiness_message: null,
      initial_bank_complete: true,
    };

    const readiness = summarizeBuildReadiness({
      primarySession,
      promptBank,
      blueprintSummary: summarizeBlueprint(blueprint()),
    });

    expect(readiness.state).toBe("in-progress");
    expect(readiness.blockers.length).toBeGreaterThan(0);
    expect(readiness.confirmations.length).toBeGreaterThan(0);
  });

  it("turns readiness into an explicit build handoff summary", () => {
    const readiness = summarizeBuildReadiness({
      promptBank: {
        session_id: "session-1",
        active_thread_id: "verify-platform",
        banked_threads: [],
        queued_threads: [],
        build_ready: true,
        build_readiness_message: "Project analysis is ready to move into the build path.",
        initial_bank_complete: true,
      },
      blueprintSummary: summarizeBlueprint(blueprint()),
    });

    const buildPath = summarizeBuildPath({
      projectName: "Personal Calendar",
      readiness,
      blueprintSummary: summarizeBlueprint(blueprint()),
      promptBank: {
        session_id: "session-1",
        active_thread_id: "verify-platform",
        banked_threads: [],
        queued_threads: [],
        build_ready: true,
        build_readiness_message: "Project analysis is ready to move into the build path.",
        initial_bank_complete: true,
      },
    });

    expect(buildPath.state).toBe("ready");
    expect(buildPath.label).toBe("Handoff ready");
    expect(buildPath.handoffTarget).toContain("Personal Calendar");
  });

  it("builds a concise project activity stream from project-local state", () => {
    const buildPath = summarizeBuildPath({
      projectName: "Personal Calendar",
      readiness: {
        state: "needs-review",
        label: "Needs review",
        headline: "A review gate is still blocking build readiness",
        nextAction: "Resolve the review queue first.",
        blockers: ["Import review pending"],
        confirmations: [],
      },
      promptBank: null,
      blueprintSummary: summarizeBlueprint(blueprint()),
    });

    const activity = summarizeProjectActivity({
      sessions: [
        sessionSummary({
          intake_phase: "interviewing",
          current_step: "socratic.question.generated",
          can_restart_from_description: true,
          resume_status: "interview_restart_only",
        }),
      ],
      buildPath,
      promptBank: null,
      importState: null,
    });

    expect(activity.items[0]?.title).toBe("Calendar intake");
    expect(activity.items.some(item => item.title === "Build path")).toBe(true);
  });

  it("derives build execution posture from session runtime and pipeline events", () => {
    const execution = summarizeBuildExecution({
      primarySession: {
        ...sessionSummary({
          intake_phase: "pipeline_running",
          pipeline_running: true,
          current_step: "pipeline.compile",
          can_resume_live: true,
          resume_status: "live_attach_available",
        }),
        pipeline_running: true,
      },
      runs: {
        runs: ["run-12345678"],
      },
      events: [
        {
          id: "event-1",
          timestamp: "2026-03-24T05:02:00Z",
          level: "info",
          source: "pipeline",
          session_id: "session-1",
          step: "pipeline.compile",
          message: "Compiling project blueprint",
          metadata: {},
        },
      ],
    });

    expect(execution.state).toBe("active");
    expect(execution.runCount).toBe(1);
    expect(execution.items[0]?.title).toBe("pipeline.compile");
  });

  it("builds an outputs summary from blueprint export history", () => {
    const outputs = summarizeOutputArtifacts({
      projectName: "Personal Calendar",
      history: {
        total: 1,
        entries: [
          {
            export_id: "exp-1",
            kind: "single_record",
            actor: "dev|local",
            node_id: "node-1",
            node_count: 2,
            edge_count: 1,
            project_id: "project-1",
            project_name: "Personal Calendar",
            scope_snapshot: null,
            scope_snapshot_redacted: false,
            scope_snapshot_redacted_fields: [],
            retention_expires_at: null,
            summary: "Exported project snapshot",
            timestamp: "2026-03-24T05:20:00Z",
          },
        ],
      },
    });

    expect(outputs.artifactCount).toBe(1);
    expect(outputs.items[0]?.copy).toContain("Exported project snapshot");
  });
});
