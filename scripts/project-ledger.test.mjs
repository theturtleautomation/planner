import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  analyzePlannerLedgerSpine,
  analyzeLedgerMaintenance,
  applyAutomation,
  automateLedger,
  buildSummary,
  loadLedger,
  renderAutomationReportMarkdown,
  renderStatusMarkdown,
  validateLedger,
} from "./project-ledger.mjs";

const TEST_DIR = path.dirname(fileURLToPath(import.meta.url));
const ROOT_DIR = path.resolve(TEST_DIR, "..");
const PASS_1_ROOT_IDS = [
  "governance:readme",
  "governance:agents",
  "governance:claude",
  "governance:project-skill-config",
  "governance:session-start-doc-index",
  "governance:project-plan",
  "governance:ledger-surfaces",
];
const PASS_1_PARENT_IDS = [
  "initiative:planner-ledger",
  "initiative:planner-solidstart-platform-direction",
  "initiative:import-existing-project-program",
  "workstream:socratic-project-picture",
];
const PASS_5_PLAN_ID = "plan:planner-ledger-population-pass-5-solidstart-family";
const PASS_5_SLICE_ID = "slice:planner-ledger-population-pass-5-solidstart-family";
const PASS_6_PLAN_ID = "plan:planner-ledger-population-pass-6-solidstart-cross-family-convergence";
const PASS_6_SLICE_ID = "slice:planner-ledger-population-pass-6-solidstart-cross-family-convergence";
const PASS_7_PLAN_ID = "plan:planner-ledger-population-pass-7-superseded-trailer-cleanup";
const PASS_7_SLICE_ID = "slice:planner-ledger-population-pass-7-superseded-trailer-cleanup";
const AUTOMATION_PLAN_ID = "plan:planner-ledger-omx-automation-lane";
const AUTOMATION_SLICE_ID = "slice:planner-ledger-omx-automation-lane";
const COMBINED_GRAPH_PLAN_ID = "plan:combined-ledger-automation-graphify-omx-integration-pass";
const COMBINED_GRAPH_SLICE_ID = "slice:combined-ledger-automation-graphify-omx-integration-pass";
const TRUST_PLAN_ID = "plan:repo-graph-coupled-routing-heuristics-and-operator-trust-pass";
const TRUST_SLICE_ID = "slice:repo-graph-coupled-routing-heuristics-and-operator-trust-pass";
const NEXT_VISIBLE_QUEUE_ITEMS = [
  ["plan:planner-design-system-command-center", "Planner design system command center plan"],
  ["plan:planner-ui-reset-route-by-route-queue", "Planner UI reset route-by-route queue"],
];
const TOUCHED_IDS = [
  "initiative:planner-ledger",
  "initiative:planner-solidstart-platform-direction",
  PASS_5_PLAN_ID,
  PASS_6_PLAN_ID,
  PASS_6_SLICE_ID,
  PASS_7_PLAN_ID,
  PASS_7_SLICE_ID,
  AUTOMATION_PLAN_ID,
  AUTOMATION_SLICE_ID,
  COMBINED_GRAPH_PLAN_ID,
  COMBINED_GRAPH_SLICE_ID,
  TRUST_PLAN_ID,
  TRUST_SLICE_ID,
  "initiative:planner-ui-reset",
  "initiative:planner-design-system",
  "initiative:planning-audit-remediation",
  "slice:planner-solidstart-phase-35-backendless-mock-route-coverage",
  "slice:planner-solidstart-phase-38-socratic-multimodal-command-desk",
  "initiative:import-existing-project-program",
  "workstream:socratic-project-picture",
  "risk:artifact-sprawl",
];

function assertNoPass6Trailer(item) {
  assert.doesNotMatch(item.summary, /superseded in part by Pass 6/i);
  assert.deepEqual(item.links.superseded_by, [PASS_6_SLICE_ID]);
}

function assertRoutingAutomationState(item, expectedState) {
  assert.equal(item.automation.routing.state, expectedState.state);
  assert.equal(item.automation.routing.confidence, expectedState.confidence);
  assert.equal(item.automation.routing.approval_required, expectedState.approval_required);
  assert.equal(item.automation.routing.recommended_routing_state, expectedState.recommended_routing_state);
}

test("ledger validates cleanly", async () => {
  const ledger = await loadLedger();
  assert.deepEqual(validateLedger(ledger), []);
});

test("summary exposes routing queues and active work", async () => {
  const ledger = await loadLedger();
  const summary = buildSummary(ledger);

  assert.ok(summary.itemCount >= 10);
  assert.ok(summary.activeWork.some(item => item.id === "initiative:planner-ledger"));
  assert.ok(summary.routingQueues.some(queue => queue.id === "needs_deep_interview" && queue.items.length > 0));
  assert.ok(summary.routingQueues.some(queue => queue.id === "ready_for_ralplan" && queue.items.some(item => item.id === "plan:planner-design-system-command-center")));
});

test("rendered markdown surfaces key ledger sections", async () => {
  const ledger = await loadLedger();
  const markdown = renderStatusMarkdown(ledger);

  assert.match(markdown, /Planner Ledger — Current Status/);
  assert.match(markdown, /Routing Queue/);
  assert.match(markdown, /Deferred Items/);
  assert.match(markdown, /Automation Surfaces/);
  assert.match(markdown, /Planner Ledger Spine Integrity/);
  assert.match(markdown, /Planner Ledger Maintenance Signal/);
  assert.match(markdown, /automation-report\.md/);
  assert.match(markdown, /npm run project:status/);
  assert.match(markdown, /Planner design system command center plan/);
  assert.match(markdown, /npm run project:ledger:auto/);
  assert.doesNotMatch(markdown, /trailer retained for later cleanup/i);
});

test("checked-in readable status surface stays in sync with the canonical ledger", async () => {
  const ledger = await loadLedger();
  const rendered = renderStatusMarkdown(ledger);
  const currentStatus = await readFile(path.join(ROOT_DIR, ".omx/ledger/current-status.md"), "utf8");

  assert.equal(currentStatus, rendered);
});

test("pass 1 root governance artifacts and parent layer are populated", async () => {
  const ledger = await loadLedger();
  const itemById = new Map(ledger.items.map(item => [item.id, item]));

  for (const id of [...PASS_1_ROOT_IDS, ...PASS_1_PARENT_IDS]) {
    assert.ok(itemById.has(id), `missing ledger item ${id}`);
  }
});

test("later queued planner work remains concrete and routed for ralplan", async () => {
  const ledger = await loadLedger();
  const itemById = new Map(ledger.items.map(item => [item.id, item]));

  for (const [id, title] of NEXT_VISIBLE_QUEUE_ITEMS) {
    const item = itemById.get(id);
    assert.ok(item, `missing queued item ${id}`);
    assert.equal(item.title, title);
    assert.equal(item.routing_state, "ready_for_ralplan");
  }
});

test("pass 3 import-builder-blueprint and pass 4 design/ui/audit are implemented", async () => {
  const ledger = await loadLedger();
  const itemById = new Map(ledger.items.map(item => [item.id, item]));

  const pass3Plan = itemById.get("plan:planner-ledger-population-pass-3-import-builder-blueprint");
  const pass3Slice = itemById.get("slice:planner-ledger-population-pass-3-import-builder-blueprint");
  const pass4Plan = itemById.get("plan:planner-ledger-population-pass-4-design-system-ui-reset-audits");
  const pass4Slice = itemById.get("slice:planner-ledger-population-pass-4-design-system-ui-reset-audits");

  assert.ok(pass3Plan);
  assert.equal(pass3Plan.status, "complete");
  assert.equal(pass3Plan.routing_state, "complete");
  assert.ok(pass3Slice);
  assert.equal(pass3Slice.status, "implemented");
  assert.ok(pass4Plan);
  assert.equal(pass4Plan.status, "complete");
  assert.equal(pass4Plan.routing_state, "complete");
  assert.ok(pass4Slice);
  assert.equal(pass4Slice.status, "implemented");
});

test("pass 5 solidstart family, pass 6 convergence, and pass 7 cleanup are implemented", async () => {
  const ledger = await loadLedger();
  const itemById = new Map(ledger.items.map(item => [item.id, item]));

  const pass5Plan = itemById.get(PASS_5_PLAN_ID);
  const pass5Slice = itemById.get(PASS_5_SLICE_ID);
  const pass6Plan = itemById.get(PASS_6_PLAN_ID);
  const pass6Slice = itemById.get(PASS_6_SLICE_ID);
  const pass7Plan = itemById.get(PASS_7_PLAN_ID);
  const pass7Slice = itemById.get(PASS_7_SLICE_ID);

  assert.ok(pass5Plan);
  assert.equal(pass5Plan.status, "complete");
  assert.equal(pass5Plan.routing_state, "complete");
  assert.ok(pass5Slice);
  assert.equal(pass5Slice.status, "implemented");
  assert.ok(pass6Plan);
  assert.equal(pass6Plan.status, "complete");
  assert.equal(pass6Plan.routing_state, "complete");
  assert.ok(pass6Slice);
  assert.equal(pass6Slice.status, "implemented");
  assert.ok(pass7Plan);
  assert.equal(pass7Plan.status, "complete");
  assert.equal(pass7Plan.routing_state, "complete");
  assert.ok(pass7Slice);
  assert.equal(pass7Slice.status, "implemented");
});

test("automation lane plan and slice are implemented", async () => {
  const ledger = await loadLedger();
  const itemById = new Map(ledger.items.map(item => [item.id, item]));

  const automationPlan = itemById.get(AUTOMATION_PLAN_ID);
  const automationSlice = itemById.get(AUTOMATION_SLICE_ID);

  assert.ok(automationPlan);
  assert.equal(automationPlan.status, "complete");
  assert.equal(automationPlan.routing_state, "complete");
  assert.ok(automationSlice);
  assert.equal(automationSlice.status, "implemented");
  assert.equal(automationSlice.routing_state, "complete");
});

test("combined graphify coupling plan and slice are implemented", async () => {
  const ledger = await loadLedger();
  const itemById = new Map(ledger.items.map(item => [item.id, item]));

  const couplingPlan = itemById.get(COMBINED_GRAPH_PLAN_ID);
  const couplingSlice = itemById.get(COMBINED_GRAPH_SLICE_ID);

  assert.ok(couplingPlan);
  assert.equal(couplingPlan.status, "complete");
  assert.equal(couplingPlan.routing_state, "complete");
  assert.ok(couplingSlice);
  assert.equal(couplingSlice.status, "implemented");
  assert.equal(couplingSlice.routing_state, "complete");
});

test("trust heuristics plan and slice are implemented", async () => {
  const ledger = await loadLedger();
  const itemById = new Map(ledger.items.map(item => [item.id, item]));

  const trustPlan = itemById.get(TRUST_PLAN_ID);
  const trustSlice = itemById.get(TRUST_SLICE_ID);

  assert.ok(trustPlan);
  assert.equal(trustPlan.status, "complete");
  assert.equal(trustPlan.routing_state, "complete");
  assert.ok(trustSlice);
  assert.equal(trustSlice.status, "implemented");
  assert.equal(trustSlice.routing_state, "complete");
});

test("socratic pass 2 is implemented and later non-socratic passes remain queued", async () => {
  const ledger = await loadLedger();
  const itemById = new Map(ledger.items.map(item => [item.id, item]));

  const pass2Plan = itemById.get("plan:planner-ledger-population-pass-2-socratic");
  const pass2Slice = itemById.get("slice:planner-ledger-population-pass-2-socratic");
  const socraticWorkstream = itemById.get("workstream:socratic-project-picture");

  assert.ok(pass2Plan);
  assert.equal(pass2Plan.status, "complete");
  assert.equal(pass2Plan.routing_state, "complete");
  assert.ok(pass2Slice);
  assert.equal(pass2Slice.status, "implemented");
  assert.deepEqual(socraticWorkstream.links.follow_on, [
    "deferred_item:hidden-truth-model",
    "deferred_item:whole-project-recoverability",
    "deferred_item:overlay-reorientation",
    "deferred_item:provenance-change-inspection",
    "deferred_item:preview-hierarchy-refinement",
  ]);
});

test("planner-ledger spine integrity reports no stale follow-ons after cleanup", async () => {
  const ledger = await loadLedger();
  const spine = analyzePlannerLedgerSpine(ledger);
  const itemById = new Map(ledger.items.map(item => [item.id, item]));

  assert.equal(spine.staleFollowOnCount, 0);
  assert.equal(spine.missingFollowOnTargetCount, 0);
  assert.equal(spine.isClean, true);
  assert.equal(itemById.get("risk:artifact-sprawl").status, "complete");
  assert.deepEqual(itemById.get("slice:planner-ledger-population-analysis").links.follow_on, undefined);
  assert.deepEqual(itemById.get("slice:planner-ledger-population-pass-1-root-governance").links.follow_on, undefined);
  assert.deepEqual(itemById.get("slice:planner-ledger-population-pass-2-socratic").links.follow_on, undefined);
  assert.deepEqual(itemById.get("slice:planner-ledger-population-pass-3-import-builder-blueprint").links.follow_on, undefined);
  assert.deepEqual(itemById.get("slice:planner-ledger-population-pass-5-solidstart-family").links.follow_on, undefined);
});

test("maintenance signal reports fresh current ledger state when no tracked artifacts outrun automation", async () => {
  const ledger = await loadLedger();
  const result = await automateLedger({ dryRun: true });
  const maintenance = analyzeLedgerMaintenance(ledger, { trace: result.trace });

  assert.equal(maintenance.state, "fresh");
  assert.equal(maintenance.isFresh, true);
  assert.equal(maintenance.staleTrackedArtifactCount, 0);
  assert.equal(typeof maintenance.automationLastRunAt, "string");
  assert.ok(maintenance.trackedArtifactCount > 0);
});

test("maintenance signal reports attention when tracked artifacts outrun automation", async () => {
  const ledger = await loadLedger();
  const maintenance = analyzeLedgerMaintenance(ledger, {
    trace: {
      generated_at: "2000-01-01T00:00:00.000Z",
    },
  });

  assert.equal(maintenance.state, "attention");
  assert.equal(maintenance.isFresh, false);
  assert.ok(maintenance.staleTrackedArtifactCount > 0);
  assert.match(maintenance.attentionReasons.join(" "), /changed after the last automation run/);
});

test("touched seeded entries use canonical relation keys", async () => {
  const ledger = await loadLedger();
  const itemById = new Map(ledger.items.map(item => [item.id, item]));
  const legacyKeys = new Set(["decisions", "risks", "reviews", "implementation", "follow_ons", "supports", "mitigated_by"]);

  for (const id of TOUCHED_IDS) {
    const item = itemById.get(id);
    assert.ok(item, `missing touched item ${id}`);
    for (const key of Object.keys(item.links ?? {})) {
      assert.ok(!legacyKeys.has(key), `legacy relation key ${key} still present on ${id}`);
    }
  }
});

test("socratic review and seed-handling relations are reciprocally normalized", async () => {
  const ledger = await loadLedger();
  const itemById = new Map(ledger.items.map(item => [item.id, item]));

  const review = itemById.get("review:socratic-current-state-vs-thesis");
  const seedSlice = itemById.get("slice:socratic-seed-handling");
  const seedImplementation = itemById.get("implementation:socratic-seed-handling-route");
  const seedDecision = itemById.get("decision:seed-loop-bounded");

  assert.deepEqual(review.links.review_of, ["workstream:socratic-project-picture"]);
  assert.ok(review.links.informs.includes("deferred_item:hidden-truth-model"));
  assert.ok(!review.links.informs.includes("deferred_item:branch-work-queue"));
  assert.deepEqual(seedSlice.links.implemented_by, ["implementation:socratic-seed-handling-route"]);
  assert.deepEqual(seedSlice.links.constrained_by_decision, ["decision:seed-loop-bounded"]);
  assert.deepEqual(seedImplementation.links.implementation_for, ["slice:socratic-seed-handling"]);
  assert.deepEqual(seedDecision.links.decision_for, ["slice:socratic-seed-handling"]);
});

test("lower-priority Socratic deferred items remain visible but unpromoted", async () => {
  const ledger = await loadLedger();
  const itemById = new Map(ledger.items.map(item => [item.id, item]));
  const review = itemById.get("review:socratic-current-state-vs-thesis");

  for (const id of ["deferred_item:branch-work-queue", "deferred_item:multimodal-capture"]) {
    const item = itemById.get(id);
    assert.ok(item);
    assert.match(item.summary, /Still visible in the Socratic family but not promoted in Pass 2/);
    assert.deepEqual(item.links.parent, ["workstream:socratic-project-picture"]);
    assert.ok(!review.links.informs.includes(id));
  }
});

test("automation upgrades planning-complete plans and applies linked-state parent routing without repo-graph evidence by default", async () => {
  const ledger = await loadLedger();
  const clonedLedger = JSON.parse(JSON.stringify(ledger));
  const itemById = new Map(clonedLedger.items.map(item => [item.id, item]));

  const socraticWorkstream = itemById.get("workstream:socratic-project-picture");
  socraticWorkstream.routing_state = "monitoring";

  const pass7Plan = itemById.get(PASS_7_PLAN_ID);
  pass7Plan.status = "draft";
  pass7Plan.routing_state = "ready_for_ralplan";

  clonedLedger.items.push({
    id: "plan:synthetic-automation-ready",
    kind: "plan",
    title: "Synthetic automation ready plan",
    status: "draft",
    routing_state: "ready_for_ralplan",
    summary: "Synthetic test item for planner-ledger automation.",
    artifacts: [
      ".omx/plans/prd-planner-ledger-omx-automation-lane.md",
      ".omx/plans/test-spec-planner-ledger-omx-automation-lane.md",
    ],
    links: {},
    tags: ["test"],
  });

  const { ledger: automatedLedger, changes } = applyAutomation(clonedLedger);
  const automatedById = new Map(automatedLedger.items.map(item => [item.id, item]));

  assert.ok(changes.some(change => change.itemId === PASS_7_PLAN_ID && change.field === "routing_state" && change.to === "complete"));
  assert.equal(automatedById.get(PASS_7_PLAN_ID).status, "complete");
  assert.equal(automatedById.get(PASS_7_PLAN_ID).routing_state, "complete");
  assert.equal(automatedById.get("plan:synthetic-automation-ready").status, "ready_for_implementation");
  assert.equal(automatedById.get("plan:synthetic-automation-ready").routing_state, "ready_for_ralph");
  assert.equal(automatedById.get("workstream:socratic-project-picture").routing_state, "needs_deep_interview");
});

test("graph-coupled automation requires repo-graph evidence and leaves a why-trail", async () => {
  const ledger = await loadLedger();
  const clonedLedger = JSON.parse(JSON.stringify(ledger));
  const itemById = new Map(clonedLedger.items.map(item => [item.id, item]));

  const importProgram = itemById.get("initiative:import-existing-project-program");
  importProgram.routing_state = "monitoring";
  const solidstartImport = itemById.get("slice:planner-solidstart-phase-14-project-import-review-route");
  solidstartImport.routing_state = "ready_for_ralplan";

  const repoGraphEvidenceByItem = {
    "initiative:import-existing-project-program": {
      query: "docs/import-existing-project-plan.md",
      matched: true,
      matches: [
        {
          id: "file:docs/import-existing-project-plan.md",
          label: "docs/import-existing-project-plan.md",
          source_file: "docs/import-existing-project-plan.md",
          community_id: 1,
        },
      ],
      explanation: "Explain: docs/import-existing-project-plan.md",
    },
  };

  const { ledger: automatedLedger, changes } = applyAutomation(clonedLedger, {
    repoGraphEvidenceByItem,
    requireRepoGraphEvidence: true,
  });
  const automatedById = new Map(automatedLedger.items.map(item => [item.id, item]));

  assert.ok(changes.some(change => change.itemId === "initiative:import-existing-project-program" && /repo-graph evidence/.test(change.reason)));
  assert.equal(automatedById.get("initiative:import-existing-project-program").routing_state, "ready_for_ralplan");
  assertRoutingAutomationState(automatedById.get("initiative:import-existing-project-program"), {
    state: "applied",
    confidence: "high",
    approval_required: false,
    recommended_routing_state: "ready_for_ralplan",
  });
});

test("automateLedger returns repo-graph-backed automation trace metadata in dry-run mode", async () => {
  const stableFiles = [
    ".omx/ledger/planner-ledger.json",
    ".omx/ledger/current-status.md",
    ".omx/ledger/automation-trace.json",
    ".omx/ledger/automation-report.md",
  ];
  const before = await Promise.all(stableFiles.map(file => readFile(path.join(ROOT_DIR, file), "utf8")));
  const result = await automateLedger({ dryRun: true });
  const after = await Promise.all(stableFiles.map(file => readFile(path.join(ROOT_DIR, file), "utf8")));

  assert.equal(result.dryRun, true);
  assert.equal(typeof result.trace.generated_at, "string");
  assert.equal(result.trace.mode, "dry-run");
  assert.ok(result.trace.item_evidence["initiative:planner-ledger"]);
  assert.equal(typeof result.trace.item_evidence["initiative:planner-ledger"].matched, "boolean");
  assert.deepEqual(after, before);
});

test("volatile repo-graph provenance does not create synthetic routing mutations", async () => {
  const ledger = await loadLedger();
  const clonedLedger = JSON.parse(JSON.stringify(ledger));
  const itemById = new Map(clonedLedger.items.map(item => [item.id, item]));
  const socratic = itemById.get("workstream:socratic-project-picture");

  socratic.automation = {
    routing: {
      state: "applied",
      confidence: "medium",
      approval_required: false,
      recommended_routing_state: "needs_deep_interview",
      reason: "repo-graph evidence + linked item routing state",
      provenance: {
        source: "repo-graph",
        query: "docs/socratic-current-state-vs-thesis-review.md",
      },
      last_evaluated_at: "2026-04-08T00:00:00.000Z",
    },
  };
  socratic.routing_state = "monitoring";

  const evidence = {
    "workstream:socratic-project-picture": {
      query: "docs/socratic-current-state-vs-thesis-review.md",
      matched: true,
      matches: [
        {
          id: "file:docs/socratic-current-state-vs-thesis-review.md",
          label: "docs/socratic-current-state-vs-thesis-review.md",
          source_file: "docs/socratic-current-state-vs-thesis-review.md",
          community_id: 999,
        },
      ],
      explanation: "Explain: changed volatile payload",
    },
  };

  const { changes, ledger: automatedLedger } = applyAutomation(clonedLedger, {
    repoGraphEvidenceByItem: evidence,
    requireRepoGraphEvidence: true,
  });
  const automatedById = new Map(automatedLedger.items.map(item => [item.id, item]));

  assert.equal(automatedById.get("workstream:socratic-project-picture").routing_state, "needs_deep_interview");
  assert.ok(!changes.some(change => change.itemId === "workstream:socratic-project-picture" && change.field === "automation.routing"));
});

test("medium-confidence graph-coupled routing now auto-mutates while preserving confidence/provenance", async () => {
  const ledger = await loadLedger();
  const clonedLedger = JSON.parse(JSON.stringify(ledger));
  const itemById = new Map(clonedLedger.items.map(item => [item.id, item]));

  const socratic = itemById.get("workstream:socratic-project-picture");
  socratic.routing_state = "monitoring";

  const evidence = {
    "workstream:socratic-project-picture": {
      query: "docs/socratic-current-state-vs-thesis-review.md",
      matched: true,
      matches: [
        {
          id: "file:docs/socratic-current-state-vs-thesis-review.md",
          label: "docs/socratic-current-state-vs-thesis-review.md",
          source_file: "docs/socratic-current-state-vs-thesis-review.md",
          community_id: 91,
        },
      ],
      explanation: "Explain: docs/socratic-current-state-vs-thesis-review.md",
    },
  };

  const { ledger: automatedLedger } = applyAutomation(clonedLedger, {
    repoGraphEvidenceByItem: evidence,
    requireRepoGraphEvidence: true,
  });
  const automatedById = new Map(automatedLedger.items.map(item => [item.id, item]));
  const automatedSocratic = automatedById.get("workstream:socratic-project-picture");

  assert.equal(automatedSocratic.routing_state, "needs_deep_interview");
  assertRoutingAutomationState(automatedSocratic, {
    state: "applied",
    confidence: "medium",
    approval_required: false,
    recommended_routing_state: "needs_deep_interview",
  });
});

test("low-confidence graph-coupled routing does not mutate and records skipped trust state", async () => {
  const ledger = await loadLedger();
  const clonedLedger = JSON.parse(JSON.stringify(ledger));
  const itemById = new Map(clonedLedger.items.map(item => [item.id, item]));

  const socratic = itemById.get("workstream:socratic-project-picture");
  socratic.routing_state = "monitoring";

  const { ledger: automatedLedger } = applyAutomation(clonedLedger, {
    repoGraphEvidenceByItem: {},
    requireRepoGraphEvidence: true,
  });
  const automatedById = new Map(automatedLedger.items.map(item => [item.id, item]));
  const automatedSocratic = automatedById.get("workstream:socratic-project-picture");

  assert.equal(automatedSocratic.routing_state, "monitoring");
  assertRoutingAutomationState(automatedSocratic, {
    state: "skipped",
    confidence: "low",
    approval_required: false,
    recommended_routing_state: null,
  });
});

test("current ledger surfaces durable confidence/provenance without provisional recommendation noise", async () => {
  const ledger = await loadLedger();
  const itemById = new Map(ledger.items.map(item => [item.id, item]));
  const socratic = itemById.get("workstream:socratic-project-picture");
  const markdown = renderStatusMarkdown(ledger);

  assert.equal(socratic.automation.routing.state, "applied");
  assert.equal(socratic.automation.routing.confidence, "medium");
  assert.equal(socratic.automation.routing.approval_required, false);
  assert.match(markdown, /## Automation Surfaces/);
  assert.match(markdown, /## Planner Ledger Maintenance Signal/);
  assert.match(markdown, /Maintenance state: \*\*fresh\*\*/);
  assert.match(markdown, /automation-report\.md/);
});

test("automation report renders a human-readable rolling history from the canonical trace", async () => {
  const trace = {
    generated_at: "2026-04-06T00:00:00.000Z",
    mode: "apply",
    change_count: 1,
    maintenance: {
      state: "fresh",
      automationLastRunAt: "2026-04-06T00:00:00.000Z",
      trackedArtifactCount: 4,
      trackedItemCount: 2,
      latestTrackedArtifact: {
        path: "docs/example.md",
        modified_at: "2026-04-05T23:59:00.000Z",
      },
      staleTrackedArtifactCount: 0,
      staleTrackedArtifacts: [],
      attentionReasons: [],
    },
    changes: [
      {
        itemId: "workstream:socratic-project-picture",
        field: "automation.routing",
        reason: "repo-graph evidence + linked item routing state (docs/socratic-current-state-vs-thesis-review.md)",
        to: {
          confidence: "medium",
          recommended_routing_state: "needs_deep_interview",
          state: "applied",
        },
      },
    ],
    history: [
      {
        generated_at: "2026-04-06T00:00:00.000Z",
        mode: "apply",
        change_count: 1,
        confidence: { high: 0, medium: 1, low: 0 },
        states: { applied: 1, skipped: 0, provisional: 0 },
      },
    ],
  };

  const report = renderAutomationReportMarkdown(trace);

  assert.match(report, /# Automation Operator Report/);
  assert.match(report, /Machine-readable canonical trace: `.omx\/ledger\/automation-trace\.json`/);
  assert.match(report, /## Freshness \/ Maintenance/);
  assert.match(report, /Maintenance state: \*\*fresh\*\*/);
  assert.match(report, /## Rolling History/);
  assert.match(report, /changes=1; high=0; medium=1; low=0; applied=1; skipped=0; provisional=0/);
});

test("pass 7 cleanup removes verbose trailer prose while structured supersession history remains", async () => {
  const ledger = await loadLedger();
  const itemById = new Map(ledger.items.map(item => [item.id, item]));

  const solidstart = itemById.get("initiative:planner-solidstart-platform-direction");
  const phase00 = itemById.get("slice:planner-solidstart-phase-00-shell-sessions-and-socratic-anchor");
  const phase31 = itemById.get("slice:planner-solidstart-phase-31-session-workspace-route-family-decomposition");
  const phase35 = itemById.get("slice:planner-solidstart-phase-35-backendless-mock-route-coverage");
  const phase35_10 = itemById.get("slice:planner-solidstart-phase-35-10-builder-frontend-mock-runtime-alignment");
  const phase36 = itemById.get("slice:planner-solidstart-phase-36-home-project-directory-consolidation");
  const phase37 = itemById.get("slice:planner-solidstart-phase-37-session-workspace-command-rail-hierarchy");
  const phase38 = itemById.get("slice:planner-solidstart-phase-38-socratic-multimodal-command-desk");
  const phase40 = itemById.get("slice:planner-solidstart-phase-40-project-only-entry-and-stale-draft-hardening");
  const pass6 = itemById.get(PASS_6_PLAN_ID);
  const pass6Slice = itemById.get(PASS_6_SLICE_ID);
  const pass7 = itemById.get(PASS_7_PLAN_ID);
  const pass7Slice = itemById.get(PASS_7_SLICE_ID);
  const uiReset = itemById.get("initiative:planner-ui-reset");
  const designSystem = itemById.get("initiative:planner-design-system");
  const audit = itemById.get("initiative:planning-audit-remediation");
  const importProgram = itemById.get("initiative:import-existing-project-program");
  const importWorkstream = itemById.get("workstream:import-existing-project-history-and-reconciliation");
  const blueprintProgram = itemById.get("initiative:blueprint-knowledge-program");
  const builder = itemById.get("workstream:builder-fusion-runtime-sync");
  const socratic = itemById.get("workstream:socratic-project-picture");

  assert.ok(solidstart);
  assert.equal(solidstart.status, "implemented");
  assert.equal(solidstart.routing_state, "complete");
  assert.ok(solidstart.links.children.includes("slice:planner-solidstart-phase-00-shell-sessions-and-socratic-anchor"));
  assert.ok(solidstart.links.children.includes("slice:planner-solidstart-phase-40-project-only-entry-and-stale-draft-hardening"));
  assert.ok(!("follow_on" in solidstart.links));
  assert.match(solidstart.summary, /Convergence center/);

  assert.ok(phase00);
  assert.deepEqual(phase00.links.parent, ["initiative:planner-solidstart-platform-direction"]);
  assert.ok(phase00.links.informed_by.includes("slice:planner-ui-reset-phase-00-shell-navigation-and-auth"));
  assert.ok(phase00.links.informed_by.includes("workstream:socratic-project-picture"));

  assert.ok(phase31);
  assert.ok(phase31.links.children.includes("slice:planner-solidstart-phase-37-session-workspace-command-rail-hierarchy"));
  assert.ok(phase31.links.children.includes("slice:planner-solidstart-phase-38-socratic-multimodal-command-desk"));

  assert.ok(phase35);
  assert.ok(phase35.links.children.includes("slice:planner-solidstart-phase-35-1-shared-frontend-mock-foundation"));
  assert.ok(phase35.links.children.includes("slice:planner-solidstart-phase-35-10-builder-frontend-mock-runtime-alignment"));

  assert.ok(phase35_10);
  assert.deepEqual(phase35_10.links.parent, ["slice:planner-solidstart-phase-35-backendless-mock-route-coverage"]);
  assert.ok(phase35_10.links.children.includes("slice:planner-solidstart-phase-36-home-project-directory-consolidation"));

  assert.ok(phase36);
  assert.deepEqual(phase36.links.parent, ["slice:planner-solidstart-phase-35-10-builder-frontend-mock-runtime-alignment"]);
  assert.ok(phase36.links.children.includes("slice:planner-solidstart-phase-36-1-frontend-mock-vite-shell-duplication-remediation"));
  assert.ok(phase36.links.children.includes("slice:planner-solidstart-phase-36-2-home-route-canonicality-remediation"));

  assert.ok(phase37);
  assert.deepEqual(phase37.links.parent, ["slice:planner-solidstart-phase-31-session-workspace-route-family-decomposition"]);
  assert.ok(phase37.links.children.includes("slice:planner-solidstart-phase-37-5-session-header-signal-consolidation"));
  assert.ok(phase37.links.informed_by.includes("initiative:planner-design-system"));

  assert.ok(phase38);
  assert.deepEqual(phase38.links.parent, ["slice:planner-solidstart-phase-31-session-workspace-route-family-decomposition"]);
  assert.ok(phase38.links.children.includes("slice:planner-solidstart-phase-38-3-session-command-desk-ultra-wide-layout"));
  assert.ok(phase38.links.informed_by.includes("workstream:socratic-project-picture"));

  assert.ok(phase40);
  assert.deepEqual(phase40.links.parent, ["initiative:planner-solidstart-platform-direction"]);
  assert.ok(phase40.links.informed_by.includes("workstream:socratic-project-picture"));

  assert.ok(pass6);
  assert.equal(pass6.routing_state, "complete");
  assert.ok(pass6Slice);
  assert.ok(pass6Slice.links.supersedes.includes("initiative:planner-ui-reset"));
  assert.ok(!("follow_on" in pass6Slice.links));
  assert.ok(pass7);
  assert.equal(pass7.routing_state, "complete");
  assert.ok(pass7Slice);
  assert.deepEqual(pass7Slice.links.informed_by, [PASS_6_SLICE_ID]);

  assertNoPass6Trailer(uiReset);
  assert.ok(uiReset.links.children.includes("slice:planner-solidstart-phase-00-shell-sessions-and-socratic-anchor"));
  assert.ok(uiReset.links.children.includes("slice:planner-solidstart-phase-31-session-workspace-route-family-decomposition"));

  assertNoPass6Trailer(designSystem);
  assert.ok(designSystem.links.children.includes("slice:planner-solidstart-phase-37-session-workspace-command-rail-hierarchy"));
  assert.ok(designSystem.links.children.includes("slice:planner-solidstart-phase-38-3-session-command-desk-ultra-wide-layout"));

  assertNoPass6Trailer(audit);
  assert.ok(audit.links.children.includes("slice:planner-solidstart-phase-35-8-backendless-mock-closeout-remediation"));
  assert.ok(audit.links.children.includes("slice:planner-solidstart-phase-37-3-canonical-static-runtime-parity-remediation"));

  assertNoPass6Trailer(importProgram);
  assert.ok(importProgram.links.children.includes("slice:planner-solidstart-phase-14-project-import-review-route"));
  assert.ok(importProgram.links.children.includes("slice:planner-solidstart-phase-35-5-import-review-frontend-mock"));

  assertNoPass6Trailer(importWorkstream);
  assert.ok(importWorkstream.links.children.includes("slice:planner-solidstart-phase-15-project-import-history-and-restore-route"));
  assert.ok(importWorkstream.links.children.includes("slice:planner-solidstart-phase-35-5-import-review-frontend-mock"));

  assertNoPass6Trailer(blueprintProgram);
  assert.ok(blueprintProgram.links.children.includes("slice:planner-solidstart-phase-10-knowledge-inventory-route"));
  assert.ok(blueprintProgram.links.children.includes("slice:planner-solidstart-phase-35-6-knowledge-and-blueprint-frontend-mock"));

  assertNoPass6Trailer(builder);
  assert.ok(builder.links.children.includes("slice:planner-solidstart-phase-35-backendless-mock-route-coverage"));
  assert.ok(builder.links.children.includes("slice:planner-solidstart-phase-36-home-project-directory-consolidation"));

  assertNoPass6Trailer(socratic);
  assert.ok(socratic.links.children.includes("slice:planner-solidstart-phase-24-socratic-runtime-contract-reset"));
  assert.ok(socratic.links.children.includes("slice:planner-solidstart-phase-38-socratic-multimodal-command-desk"));
});
