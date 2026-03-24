import type {
  ImportStatus,
  ProjectImportDiffSummary,
  ProjectImportHistoryComparisonResponse,
  ProjectImportHistoryEntry,
  ProjectImportHistoryPairComparisonResponse,
} from "./types";

export function formatImportStatusLabel(status: ImportStatus): string {
  switch (status) {
    case "review_pending":
      return "Review pending";
    case "applied":
      return "Applied";
    case "queued":
      return "Queued";
    case "cloning":
      return "Cloning";
    case "analyzing":
      return "Analyzing";
    case "failed":
      return "Failed";
    default:
      return status;
  }
}

export function buildCurrentComparisonNotes(
  comparison: ProjectImportHistoryComparisonResponse,
): string[] {
  const notes: string[] = [];
  if (comparison.current_import_job_uses_selection_filter) {
    notes.push("Current import comparison uses selected nodes from saved merge controls.");
  }
  if (comparison.selected_entry_uses_selection_filter) {
    notes.push("Selected history entry comparison uses selected nodes from saved merge controls.");
  }
  return notes;
}

export function buildPairComparisonNotes(
  comparison: ProjectImportHistoryPairComparisonResponse,
): string[] {
  const notes: string[] = [];
  if (comparison.baseline_entry_uses_selection_filter) {
    notes.push("Baseline history entry comparison uses selected nodes from saved merge controls.");
  }
  if (comparison.compared_entry_uses_selection_filter) {
    notes.push("Compared history entry comparison uses selected nodes from saved merge controls.");
  }
  return notes;
}

export function formatEntrySelection(entry: ProjectImportHistoryEntry): string | null {
  if (entry.effective_included_node_count == null) return null;
  const excluded = entry.effective_excluded_node_count;
  return `Effective selection: ${entry.effective_included_node_count} included${
    excluded != null ? `, ${excluded} excluded` : ""
  }`;
}

export function hasSelectionExclusions(entry: ProjectImportHistoryEntry): boolean {
  return (entry.effective_excluded_node_count ?? 0) > 0;
}

export function summarizeDiffHeadline(diff: ProjectImportDiffSummary): string {
  return `${diff.added_nodes.length} added, ${diff.removed_nodes.length} removed`;
}
