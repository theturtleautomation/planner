import {
  buildCurrentComparisonNotes,
  buildPairComparisonNotes,
  formatEntrySelection,
  formatImportStatusLabel,
  hasSelectionExclusions,
  summarizeDiffHeadline,
} from "./import-history";
import type {
  ProjectImportHistoryComparisonResponse,
  ProjectImportHistoryEntry,
  ProjectImportHistoryPairComparisonResponse,
} from "./types";

const historyEntry = (overrides: Partial<ProjectImportHistoryEntry> = {}): ProjectImportHistoryEntry => ({
  import_job: {
    id: "job-1",
    project_id: "project-1",
    provider: "github",
    requested_ref: "https://github.com/example/planner",
    status: "applied",
    restored_from_job_id: null,
    seed_session_id: null,
    analysis_summary: "Imported draft.",
    progress_message: null,
    error_message: null,
    created_at: "2026-03-24T00:00:00Z",
    updated_at: "2026-03-24T00:10:00Z",
  },
  source_metadata: {
    provider: "github",
    canonical_ref: "https://github.com/example/planner",
    local_root: "/tmp/planner",
    default_branch: "main",
    head_revision: "deadbeef",
  },
  discovered_node_count: 4,
  effective_included_node_count: 3,
  effective_excluded_node_count: 1,
  ...overrides,
});

describe("import history helpers", () => {
  it("formats import statuses for route copy", () => {
    expect(formatImportStatusLabel("review_pending")).toBe("Review pending");
    expect(formatImportStatusLabel("applied")).toBe("Applied");
  });

  it("builds truthful comparison notes for current-state comparisons", () => {
    const comparison: ProjectImportHistoryComparisonResponse = {
      project: {} as never,
      source_binding: {} as never,
      selected_entry: historyEntry(),
      current_import_job: historyEntry().import_job,
      selected_entry_uses_selection_filter: false,
      current_import_job_uses_selection_filter: true,
      diff_summary: {
        current_job_id: "job-2",
        compared_to_job_id: "job-1",
        added_nodes: [],
        removed_nodes: [],
        added_node_types: [],
        removed_node_types: [],
      },
    };

    expect(buildCurrentComparisonNotes(comparison)).toEqual([
      "Current import comparison uses selected nodes from saved merge controls.",
    ]);
  });

  it("builds truthful pair-comparison notes", () => {
    const pair: ProjectImportHistoryPairComparisonResponse = {
      project: {} as never,
      source_binding: {} as never,
      baseline_entry: historyEntry(),
      compared_entry: historyEntry({ import_job: { ...historyEntry().import_job, id: "job-2" } }),
      baseline_entry_uses_selection_filter: true,
      compared_entry_uses_selection_filter: true,
      diff_summary: {
        current_job_id: "job-2",
        compared_to_job_id: "job-1",
        added_nodes: [],
        removed_nodes: [],
        added_node_types: [],
        removed_node_types: [],
      },
    };

    expect(buildPairComparisonNotes(pair)).toEqual([
      "Baseline history entry comparison uses selected nodes from saved merge controls.",
      "Compared history entry comparison uses selected nodes from saved merge controls.",
    ]);
  });

  it("formats entry selection summaries and exclusion state", () => {
    const entry = historyEntry();
    expect(formatEntrySelection(entry)).toBe("Effective selection: 3 included, 1 excluded");
    expect(hasSelectionExclusions(entry)).toBe(true);
  });

  it("summarizes added and removed counts", () => {
    expect(
      summarizeDiffHeadline({
        current_job_id: "job-2",
        compared_to_job_id: "job-1",
        added_nodes: [{ node_id: "n1", node_name: "Rust", node_type: "technology" }],
        removed_nodes: [],
        added_node_types: [],
        removed_node_types: [],
      }),
    ).toBe("1 added, 0 removed");
  });
});
