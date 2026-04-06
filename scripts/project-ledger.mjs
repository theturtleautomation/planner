#!/usr/bin/env node

import { execFile as execFileCallback } from "node:child_process";
import { readFile, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import path from "node:path";
import { promisify } from "node:util";
import { fileURLToPath } from "node:url";

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const ROOT_DIR = path.resolve(SCRIPT_DIR, "..");
const LEDGER_PATH = path.join(ROOT_DIR, ".omx/ledger/planner-ledger.json");
const STATUS_PATH = path.join(ROOT_DIR, ".omx/ledger/current-status.md");
const AUTOMATION_TRACE_PATH = path.join(ROOT_DIR, ".omx/ledger/automation-trace.json");
const AUTOMATION_REPORT_PATH = path.join(ROOT_DIR, ".omx/ledger/automation-report.md");
const REPO_GRAPH_SCRIPT_PATH = path.join(ROOT_DIR, "scripts/repo-graph.sh");
const execFile = promisify(execFileCallback);

const REQUIRED_KINDS = [
  "governance_artifact",
  "initiative",
  "workstream",
  "slice",
  "plan",
  "implementation",
  "review",
  "deferred_item",
  "decision",
  "risk",
];

const REQUIRED_ROUTING_STATES = [
  "needs_deep_interview",
  "ready_for_ralplan",
  "ready_for_ralph",
  "needs_testing",
  "needs_analysis",
  "monitoring",
  "complete",
];

const AUTOMATION_ROUTING_PRIORITY = [
  "needs_deep_interview",
  "needs_analysis",
  "ready_for_ralplan",
  "ready_for_ralph",
  "needs_testing",
  "monitoring",
  "complete",
];

const EXECUTION_READY_STATUS = "ready_for_implementation";

export async function loadLedger() {
  const raw = await readFile(LEDGER_PATH, "utf8");
  return JSON.parse(raw);
}

async function writeLedger(ledger) {
  await writeFile(LEDGER_PATH, `${JSON.stringify(ledger, null, 2)}\n`, "utf8");
}

export function validateLedger(ledger) {
  const errors = [];

  const kinds = new Set((ledger.object_kinds ?? []).map(kind => kind.id));
  for (const kind of REQUIRED_KINDS) {
    if (!kinds.has(kind)) {
      errors.push(`Missing object kind: ${kind}`);
    }
  }

  const routingStates = new Set((ledger.routing_states ?? []).map(state => state.id));
  for (const state of REQUIRED_ROUTING_STATES) {
    if (!routingStates.has(state)) {
      errors.push(`Missing routing state: ${state}`);
    }
  }

  const statuses = new Set((ledger.statuses ?? []).map(status => status.id));
  const items = ledger.items ?? [];
  const itemIds = new Set();

  for (const item of items) {
    if (!item.id) {
      errors.push("Found item with missing id");
      continue;
    }
    if (itemIds.has(item.id)) {
      errors.push(`Duplicate item id: ${item.id}`);
    }
    itemIds.add(item.id);

    if (!kinds.has(item.kind)) {
      errors.push(`Item ${item.id} uses unknown kind: ${item.kind}`);
    }
    if (!statuses.has(item.status)) {
      errors.push(`Item ${item.id} uses unknown status: ${item.status}`);
    }
    if (!routingStates.has(item.routing_state)) {
      errors.push(`Item ${item.id} uses unknown routing state: ${item.routing_state}`);
    }

    for (const artifact of item.artifacts ?? []) {
      const artifactPath = path.join(ROOT_DIR, artifact);
      if (!existsSync(artifactPath)) {
        errors.push(`Item ${item.id} references missing artifact: ${artifact}`);
      }
    }
  }

  for (const item of items) {
    for (const referencedIds of Object.values(item.links ?? {})) {
      for (const referencedId of referencedIds) {
        if (!itemIds.has(referencedId)) {
          errors.push(`Item ${item.id} links to missing item: ${referencedId}`);
        }
      }
    }
  }

  return errors;
}

export function buildSummary(ledger) {
  const items = ledger.items ?? [];
  const routingLabelById = Object.fromEntries((ledger.routing_states ?? []).map(state => [state.id, state.label]));
  const groupedByRouting = new Map();

  for (const item of items) {
    if (!groupedByRouting.has(item.routing_state)) {
      groupedByRouting.set(item.routing_state, []);
    }
    groupedByRouting.get(item.routing_state).push(item);
  }

  const activeWork = items.filter(item => ["initiative", "workstream"].includes(item.kind) && ["active", "in_progress", "implemented"].includes(item.status));
  const deferredItems = items.filter(item => item.kind === "deferred_item");
  const activeRisks = items.filter(item => item.kind === "risk" && item.status === "active");

  return {
    project: ledger.project,
    coverage: ledger.coverage,
    itemCount: items.length,
    activeWork,
    deferredItems,
    activeRisks,
    routingQueues: [...groupedByRouting.entries()].map(([id, queue]) => ({
      id,
      label: routingLabelById[id] ?? id,
      items: queue,
    })),
  };
}

function hasPlanningArtifacts(item) {
  const artifacts = item.artifacts ?? [];
  return artifacts.some(artifact => artifact.startsWith(".omx/plans/prd-"))
    && artifacts.some(artifact => artifact.startsWith(".omx/plans/test-spec-"));
}

function correspondingSliceId(planId) {
  return planId.startsWith("plan:") ? `slice:${planId.slice(5)}` : null;
}

function pickHighestPriorityRoutingState(states) {
  const filtered = states.filter(Boolean);
  if (filtered.length === 0) {
    return null;
  }
  return [...filtered].sort((left, right) => (
    AUTOMATION_ROUTING_PRIORITY.indexOf(left) - AUTOMATION_ROUTING_PRIORITY.indexOf(right)
  ))[0];
}

function recordMutation(changes, itemId, field, from, to, reason) {
  if (from === to) {
    return;
  }
  changes.push({ itemId, field, from, to, reason });
}

function formatChangeValue(value) {
  return typeof value === "string" || typeof value === "number" || typeof value === "boolean" || value == null
    ? String(value)
    : JSON.stringify(value);
}

async function loadExistingAutomationTrace() {
  if (!existsSync(AUTOMATION_TRACE_PATH)) {
    return null;
  }

  const raw = await readFile(AUTOMATION_TRACE_PATH, "utf8");
  return JSON.parse(raw);
}

function repoGraphProvenance(evidence) {
  if (!evidence?.matched) {
    return null;
  }

  return {
    source: "repo-graph",
    query: evidence.query,
    matches: evidence.matches ?? [],
    explanation: evidence.explanation,
  };
}

function setRoutingAutomationState(item, routingState, changes, itemId) {
  const automation = item.automation ?? {};
  const previousRouting = automation.routing ?? null;
  const previousComparable = previousRouting
    ? { ...previousRouting, last_evaluated_at: undefined }
    : null;
  const nextComparable = {
    ...(automation.routing ?? {}),
    ...routingState,
    last_evaluated_at: undefined,
  };
  const shouldUpdateTimestamp = JSON.stringify(previousComparable) !== JSON.stringify(nextComparable);
  const nextRouting = {
    ...(automation.routing ?? {}),
    ...routingState,
    last_evaluated_at: shouldUpdateTimestamp
      ? new Date().toISOString()
      : previousRouting?.last_evaluated_at,
  };

  if (JSON.stringify(previousRouting) !== JSON.stringify(nextRouting)) {
    changes.push({
      itemId,
      field: "automation.routing",
      from: previousRouting,
      to: nextRouting,
      reason: routingState.reason ?? "automation routing state",
    });
  }

  item.automation = {
    ...automation,
    routing: nextRouting,
  };
}

function trackedRepoGraphArtifacts(item) {
  return (item.artifacts ?? []).filter(artifact => (
    artifact.startsWith("docs/")
    || artifact.startsWith("scripts/")
    || artifact.startsWith(".codex/skills/")
  ));
}

function candidateQueryTerms(item) {
  const terms = [];

  for (const artifact of trackedRepoGraphArtifacts(item)) {
    terms.push(artifact);
    terms.push(path.basename(artifact, path.extname(artifact)));
  }

  terms.push(item.title);
  return [...new Set(terms.map(term => term.trim()).filter(Boolean))];
}

async function defaultRepoGraphRunner(args) {
  const { stdout } = await execFile(REPO_GRAPH_SCRIPT_PATH, args, { cwd: ROOT_DIR });
  return stdout.trim();
}

async function collectRepoGraphEvidenceByItem(ledger, { runner = defaultRepoGraphRunner } = {}) {
  const evidenceByItem = {};

  for (const item of ledger.items ?? []) {
    if (!["initiative", "workstream", "risk"].includes(item.kind) || item.status !== "active") {
      continue;
    }

    const queries = candidateQueryTerms(item);
    let selectedQuery = null;
    let matches = [];
    let explanation = null;

    for (const query of queries) {
      try {
        const nodeJson = await runner(["node", "--json", query]);
        const parsed = JSON.parse(nodeJson);
        if ((parsed.matches ?? []).length === 0) {
          continue;
        }

        selectedQuery = query;
        matches = parsed.matches.slice(0, 3).map(match => ({
          id: match.id,
          label: match.label,
          source_file: match.source_file,
          community_id: match.community_id,
        }));
        explanation = await runner(["explain", query]);
        break;
      } catch {
        // Fall through to the next candidate query term.
      }
    }

    evidenceByItem[item.id] = {
      query: selectedQuery ?? queries[0] ?? item.title,
      matched: matches.length > 0,
      matches,
      explanation,
    };
  }

  return evidenceByItem;
}

function buildAutomationTrace({ changes, repoGraphEvidenceByItem, mode }) {
  return {
    generated_at: new Date().toISOString(),
    mode,
    change_count: changes.length,
    changes: changes.map(change => ({
      ...change,
      repo_graph_evidence: repoGraphEvidenceByItem?.[change.itemId] ?? null,
    })),
    item_evidence: repoGraphEvidenceByItem ?? {},
  };
}

function summarizeTraceEntry(trace) {
  const confidence = { high: 0, medium: 0, low: 0 };
  const states = { applied: 0, skipped: 0, provisional: 0 };

  for (const change of trace.changes ?? []) {
    if (change.field !== "automation.routing" || !change.to) {
      continue;
    }

    if (change.to.confidence && change.to.confidence in confidence) {
      confidence[change.to.confidence] += 1;
    }
    if (change.to.state && change.to.state in states) {
      states[change.to.state] += 1;
    }
  }

  return {
    generated_at: trace.generated_at,
    mode: trace.mode,
    change_count: trace.change_count,
    confidence,
    states,
    changed_items: [...new Set((trace.changes ?? []).map(change => change.itemId))],
  };
}

function mergeAutomationTrace(previousTrace, nextTrace) {
  const history = previousTrace?.history ?? [];
  return {
    ...nextTrace,
    history: [summarizeTraceEntry(nextTrace), ...history].slice(0, 20),
  };
}

export function renderAutomationReportMarkdown(trace) {
  const lines = [];
  const history = trace.history ?? [];
  const latest = history[0] ?? summarizeTraceEntry(trace);

  lines.push("# Automation Operator Report");
  lines.push("");
  lines.push("Machine-readable canonical trace: `.omx/ledger/automation-trace.json`");
  lines.push("");
  lines.push("## Latest Run");
  lines.push("");
  lines.push(`- Generated at: \`${latest.generated_at}\``);
  lines.push(`- Mode: \`${latest.mode}\``);
  lines.push(`- Change count: **${latest.change_count}**`);
  lines.push(`- Confidence mix: high=${latest.confidence.high}, medium=${latest.confidence.medium}, low=${latest.confidence.low}`);
  lines.push(`- Routing states: applied=${latest.states.applied}, skipped=${latest.states.skipped}, provisional=${latest.states.provisional}`);
  lines.push("");
  lines.push("## Latest Change Details");
  lines.push("");
  if ((trace.changes ?? []).length === 0) {
    lines.push("- No automation changes were applied.");
  } else {
    for (const change of trace.changes) {
      const routing = change.to?.recommended_routing_state ?? "n/a";
      const confidence = change.to?.confidence ?? "n/a";
      lines.push(`- **${change.itemId}** — ${change.reason} _(routing: ${routing}; confidence: ${confidence})_`);
    }
  }
  lines.push("");
  lines.push("## Rolling History");
  lines.push("");
  if (history.length === 0) {
    lines.push("- None");
  } else {
    for (const entry of history) {
      lines.push(`- \`${entry.generated_at}\` — changes=${entry.change_count}; high=${entry.confidence.high}; medium=${entry.confidence.medium}; low=${entry.confidence.low}; applied=${entry.states.applied}; skipped=${entry.states.skipped}; provisional=${entry.states.provisional}`);
    }
  }
  lines.push("");

  return `${lines.join("\n")}\n`;
}

export function applyAutomation(ledger, { repoGraphEvidenceByItem = {}, requireRepoGraphEvidence = false } = {}) {
  const nextLedger = JSON.parse(JSON.stringify(ledger));
  const itemById = new Map(nextLedger.items.map(item => [item.id, item]));
  const changes = [];

  for (const item of nextLedger.items) {
    if (item.kind !== "plan" || !hasPlanningArtifacts(item)) {
      continue;
    }

    const siblingSlice = itemById.get(correspondingSliceId(item.id));

    if (siblingSlice?.status === "implemented") {
      recordMutation(changes, item.id, "status", item.status, "complete", "implemented sibling slice");
      recordMutation(changes, item.id, "routing_state", item.routing_state, "complete", "implemented sibling slice");
      item.status = "complete";
      item.routing_state = "complete";
      continue;
    }

    if (item.status === "complete" && item.routing_state === "complete") {
      continue;
    }

    recordMutation(changes, item.id, "status", item.status, EXECUTION_READY_STATUS, "planning artifacts present");
    recordMutation(changes, item.id, "routing_state", item.routing_state, "ready_for_ralph", "planning artifacts present");
    item.status = EXECUTION_READY_STATUS;
    item.routing_state = "ready_for_ralph";
  }

  for (const item of nextLedger.items) {
    if (!["initiative", "workstream", "risk"].includes(item.kind)) {
      continue;
    }

    if (item.status !== "active") {
      continue;
    }

    const linkedIds = [...new Set([
      ...(item.links?.children ?? []),
      ...(item.links?.follow_on ?? []),
    ])];

    if (linkedIds.length === 0) {
      continue;
    }

    const linkedStates = linkedIds
      .map(id => itemById.get(id)?.routing_state)
      .filter(state => state && state !== "complete");

    if (linkedStates.length === 0) {
      continue;
    }

    const nextRoutingState = pickHighestPriorityRoutingState(linkedStates);
    if (!nextRoutingState) {
      continue;
    }

    const evidence = repoGraphEvidenceByItem[item.id] ?? null;
    if (requireRepoGraphEvidence && !evidence?.matched) {
      setRoutingAutomationState(item, {
        state: "skipped",
        confidence: "low",
        approval_required: false,
        recommended_routing_state: null,
        reason: "repo-graph evidence missing",
        provenance: null,
      }, changes, item.id);
      continue;
    }

    const distinctStates = [...new Set(linkedStates)];
    const confidence = distinctStates.length === 1 ? "high" : "medium";
    const reason = evidence?.matched
      ? `repo-graph evidence + linked item routing state (${evidence.query})`
      : "linked item routing state";

    if (confidence === "high" || confidence === "medium") {
      recordMutation(changes, item.id, "routing_state", item.routing_state, nextRoutingState, reason);
      item.routing_state = nextRoutingState;
      setRoutingAutomationState(item, {
        state: "applied",
        confidence,
        approval_required: false,
        recommended_routing_state: nextRoutingState,
        reason,
        provenance: repoGraphProvenance(evidence),
      }, changes, item.id);
      continue;
    }
  }

  return { ledger: nextLedger, changes };
}

export function renderStatusMarkdown(ledger) {
  const summary = buildSummary(ledger);
  const lines = [];

  lines.push("# Planner Ledger — Current Status");
  lines.push("");
  lines.push(`Canonical source: \`${ledger.project.canonical_ledger}\``);
  lines.push(`Project skill: \`${ledger.project.project_skill}\``);
  lines.push("");
  lines.push("## Coverage");
  lines.push("");
  lines.push(`- Mode: **${summary.coverage.mode}**`);
  lines.push(`- Summary: ${summary.coverage.summary}`);
  lines.push(`- Included workstreams: ${summary.coverage.included_workstreams.join(", ")}`);
  lines.push(`- Explicitly not required in v1: ${summary.coverage.explicitly_not_required.join(", ")}`);
  lines.push("");
  lines.push("## Routing Queue");
  lines.push("");
  for (const queue of summary.routingQueues.filter(queue => queue.id !== "complete")) {
    lines.push(`### ${queue.label}`);
    lines.push("");
    if (queue.items.length === 0) {
      lines.push("- None");
      lines.push("");
      continue;
    }
    for (const item of queue.items) {
      lines.push(`- **${item.title}** (${item.kind}, ${item.status}) — ${item.summary}`);
    }
    lines.push("");
  }

  lines.push("## Active Workstreams and Initiatives");
  lines.push("");
  for (const item of summary.activeWork) {
    lines.push(`- **${item.title}** — ${item.summary} _(next: ${item.routing_state})_`);
  }
  lines.push("");

  lines.push("## Deferred Items");
  lines.push("");
  for (const item of summary.deferredItems) {
    lines.push(`- **${item.title}** — ${item.summary} _(status: ${item.status}; next: ${item.routing_state})_`);
  }
  lines.push("");

  lines.push("## Active Risks");
  lines.push("");
  for (const item of summary.activeRisks) {
    lines.push(`- **${item.title}** — ${item.summary} _(next: ${item.routing_state})_`);
  }
  lines.push("");

  lines.push("## Automation Surfaces");
  lines.push("");
  lines.push("- Canonical machine-readable trace: `.omx/ledger/automation-trace.json`");
  lines.push("- Human-readable operator report: `.omx/ledger/automation-report.md`");
  lines.push("");

  lines.push("## Commands");
  lines.push("");
  lines.push("- `npm run project:status` — print current ledger summary");
  lines.push("- `npm run project:ledger:validate` — validate ledger structure and artifact links");
  lines.push("- `npm run project:ledger:refresh` — regenerate this readable status surface");
  lines.push("- `npm run project:ledger:auto` — apply bounded ledger/status/routing automation");
  lines.push("- `npm run test:ledger` — run ledger tests");
  lines.push("");

  return `${lines.join("\n")}\n`;
}

export async function refreshReadableSurface() {
  const ledger = await loadLedger();
  const errors = validateLedger(ledger);
  if (errors.length > 0) {
    throw new Error(`Ledger validation failed before render:\n- ${errors.join("\n- ")}`);
  }
  const markdown = renderStatusMarkdown(ledger);
  await writeFile(STATUS_PATH, markdown, "utf8");
  return markdown;
}

export async function automateLedger({ dryRun = false } = {}) {
  const ledger = await loadLedger();
  const previousTrace = await loadExistingAutomationTrace();
  const repoGraphEvidenceByItem = await collectRepoGraphEvidenceByItem(ledger);
  const { ledger: automatedLedger, changes } = applyAutomation(ledger, {
    repoGraphEvidenceByItem,
    requireRepoGraphEvidence: true,
  });
  const errors = validateLedger(automatedLedger);

  if (errors.length > 0) {
    throw new Error(`Ledger validation failed after automation:\n- ${errors.join("\n- ")}`);
  }

  const trace = mergeAutomationTrace(previousTrace, buildAutomationTrace({
    changes,
    repoGraphEvidenceByItem,
    mode: dryRun ? "dry-run" : "apply",
  }));

  if (!dryRun && changes.length > 0) {
    await writeLedger(automatedLedger);
    await writeFile(STATUS_PATH, renderStatusMarkdown(automatedLedger), "utf8");
  }

  if (!dryRun) {
    await writeFile(AUTOMATION_TRACE_PATH, `${JSON.stringify(trace, null, 2)}\n`, "utf8");
    await writeFile(AUTOMATION_REPORT_PATH, renderAutomationReportMarkdown(trace), "utf8");
  }

  return {
    changed: changes.length > 0,
    changeCount: changes.length,
    changes,
    ledger: automatedLedger,
    dryRun,
    trace,
  };
}

export function renderConsoleSummary(ledger) {
  const summary = buildSummary(ledger);
  const lines = [];
  lines.push(`Planner ledger summary (${summary.itemCount} items)`);
  lines.push(`Coverage: ${summary.coverage.summary}`);
  lines.push("");
  lines.push("Next-step queues:");
  for (const queue of summary.routingQueues.filter(queue => queue.id !== "complete")) {
    lines.push(`- ${queue.label}: ${queue.items.length}`);
  }
  lines.push("");
  lines.push("Active work:");
  for (const item of summary.activeWork) {
    lines.push(`- ${item.title} -> ${item.routing_state}`);
  }
  return `${lines.join("\n")}\n`;
}

async function main() {
  const args = new Set(process.argv.slice(2));
  const shouldAutomate = args.has("--auto");
  const dryRun = args.has("--dry-run");
  let ledger = await loadLedger();

  if (shouldAutomate) {
    const result = await automateLedger({ dryRun });
    ledger = result.ledger;
    if (result.changeCount === 0) {
      console.log(dryRun ? "Automation dry run found no changes." : "Ledger automation found no changes.");
    } else {
      console.log(dryRun ? "Automation dry run changes:" : "Applied ledger automation changes:");
      for (const change of result.changes) {
        console.log(`- ${change.itemId} ${change.field}: ${formatChangeValue(change.from)} -> ${formatChangeValue(change.to)} (${change.reason})`);
      }
    }
  }

  if (args.has("--validate")) {
    const errors = validateLedger(ledger);
    if (errors.length > 0) {
      console.error(errors.map(error => `- ${error}`).join("\n"));
      process.exitCode = 1;
      return;
    }
    console.log("Ledger validation passed.");
  }

  if (args.has("--render")) {
    await refreshReadableSurface();
    console.log(`Updated ${path.relative(ROOT_DIR, STATUS_PATH)}`);
  }

  if (args.has("--json")) {
    console.log(JSON.stringify(buildSummary(ledger), null, 2));
    return;
  }

  if (args.has("--summary") || process.argv.length <= 2) {
    process.stdout.write(renderConsoleSummary(ledger));
  }
}

const isMain = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) {
  main().catch(error => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  });
}
