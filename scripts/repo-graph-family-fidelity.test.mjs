import assert from "node:assert/strict";
import test from "node:test";

import {
  analyzeFamilyFidelity,
  collectFamiliesFromLedger,
  renderReportMarkdown,
} from "./repo-graph-family-fidelity.mjs";

test("collectFamiliesFromLedger groups current family buckets from initiative/workstream items", () => {
  const ledger = {
    items: [
      {
        id: "initiative:planner-ledger",
        kind: "initiative",
        title: "Ledger",
        routing_state: "ready_for_ralplan",
        tags: ["family:planner-ledger/repo-graph"],
        artifacts: ["a.md", "b.md"],
      },
      {
        id: "workstream:socratic-project-picture",
        kind: "workstream",
        title: "Socratic",
        routing_state: "needs_deep_interview",
        tags: ["family:socratic"],
        artifacts: ["c.md"],
      },
    ],
  };

  const families = collectFamiliesFromLedger(ledger);
  assert.deepEqual(families.map(family => family.family_id), [
    "planner-ledger/repo-graph",
    "socratic",
  ]);
  assert.deepEqual(families[0].canonical_artifacts, ["a.md", "b.md"]);
  assert.deepEqual(families[0].family_sources, ["canonical_tag"]);
});

test("analyzeFamilyFidelity detects graph coverage gaps and separates recommendations", () => {
  const ledger = {
    items: [
      {
        id: "initiative:planner-ledger",
        kind: "initiative",
        title: "Ledger",
        routing_state: "ready_for_ralplan",
        tags: ["family:planner-ledger/repo-graph"],
        artifacts: ["scripts/project-ledger.mjs", ".omx/ledger/planner-ledger.json"],
      },
    ],
  };

  const graph = {
    nodes: [
      {
        id: "file:scripts/project-ledger.mjs",
        kind: "file",
        source_file: "scripts/project-ledger.mjs",
        category: "code",
        community_id: 10,
      },
      {
        id: "heading:scripts/project-ledger.mjs#main",
        kind: "heading",
        source_file: "scripts/project-ledger.mjs",
      },
    ],
    edges: [
      {
        source: "file:scripts/project-ledger.mjs",
        target: "heading:scripts/project-ledger.mjs#main",
        relation: "contains",
      },
    ],
  };

  const manifest = {
    built_at: "2026-04-06T00:00:00Z",
    build_reason: "test",
    total_files: 1,
    nodes: 2,
    edges: 1,
    communities: 1,
  };

  const report = analyzeFamilyFidelity(ledger, graph, manifest);
  const family = report.families[0];

  assert.equal(family.family_id, "planner-ledger/repo-graph");
  assert.equal(family.family_provenance.primary_source, "canonical_tag");
  assert.equal(family.family_provenance.fallback_used, false);
  assert.deepEqual(family.missing_graph_artifacts, [".omx/ledger/planner-ledger.json"]);
  assert.match(family.findings[0].summary, /absent from repo-graph file nodes/i);
  assert.ok(family.recommendations.length > 0);
});

test("collectFamiliesFromLedger still supports narrow fallback when canonical family metadata is missing", () => {
  const ledger = {
    items: [
      {
        id: "workstream:socratic-project-picture",
        kind: "workstream",
        title: "Socratic",
        routing_state: "needs_deep_interview",
        tags: ["workspace", "socratic"],
        artifacts: ["c.md"],
      },
    ],
  };

  const families = collectFamiliesFromLedger(ledger);
  assert.equal(families[0].family_id, "socratic");
  assert.deepEqual(families[0].family_sources, ["fallback_tag_alias"]);
});

test("analyzeFamilyFidelity marks fallback provenance when canonical family metadata is missing", () => {
  const ledger = {
    items: [
      {
        id: "workstream:socratic-project-picture",
        kind: "workstream",
        title: "Socratic",
        routing_state: "needs_deep_interview",
        tags: ["workspace", "socratic"],
        artifacts: ["docs/socratic-current-state-vs-thesis-review.md"],
      },
    ],
  };
  const graph = {
    nodes: [
      {
        id: "file:docs/socratic-current-state-vs-thesis-review.md",
        kind: "file",
        source_file: "docs/socratic-current-state-vs-thesis-review.md",
        category: "docs",
        community_id: 1,
      },
    ],
    edges: [],
  };
  const manifest = { built_at: "2026-04-07T00:00:00Z", total_files: 1, nodes: 1, edges: 0, communities: 1 };

  const report = analyzeFamilyFidelity(ledger, graph, manifest);

  assert.equal(report.families[0].family_provenance.primary_source, "fallback_tag_alias");
  assert.equal(report.families[0].family_provenance.fallback_used, true);
});

test("collectFamiliesFromLedger leaves items unclassified when canonical and alias tags are both missing", () => {
  const ledger = {
    items: [
      {
        id: "initiative:unknown-family",
        kind: "initiative",
        title: "Unknown",
        routing_state: "ready_for_ralplan",
        tags: ["planner"],
        artifacts: ["x.md"],
      },
    ],
  };

  const families = collectFamiliesFromLedger(ledger);
  assert.equal(families.length, 0);
});

test("analyzeFamilyFidelity summarizes repo-wide relationship categories with representative examples", () => {
  const ledger = { items: [] };
  const graph = {
    nodes: [
      { id: "file:docs/a.md", kind: "file", source_file: "docs/a.md", category: "docs", community_id: 1 },
      { id: "file:docs/b.md", kind: "file", source_file: "docs/b.md", category: "docs", community_id: 2 },
      { id: "file:src/a.py", kind: "file", source_file: "src/a.py", category: "code", community_id: 3 },
      { id: "file:src/b.py", kind: "file", source_file: "src/b.py", category: "code", community_id: 4 },
    ],
    edges: [
      { source: "file:docs/a.md", target: "file:docs/b.md", relation: "references" },
      { source: "file:src/a.py", target: "file:src/b.py", relation: "imports" },
      { source: "file:docs/a.md", target: "file:src/a.py", relation: "references" },
    ],
  };
  const manifest = { built_at: "2026-04-07T00:00:00Z", total_files: 4, nodes: 4, edges: 3, communities: 4 };

  const report = analyzeFamilyFidelity(ledger, graph, manifest);

  assert.deepEqual(
    report.relationship_categories.map(item => [item.category, item.count]),
    [["doc↔doc", 1], ["code↔code", 1], ["doc↔code", 1]],
  );
});

test("relationship ranking prefers product-central examples while retaining lower-priority ones", () => {
  const ledger = {
    items: [
      {
        id: "initiative:planner-solidstart-platform-direction",
        kind: "initiative",
        title: "SolidStart",
        routing_state: "complete",
        tags: ["family:planner-solidstart"],
        artifacts: ["docs/planner-solidstart-platform-direction-spec.md"],
      },
    ],
  };
  const graph = {
    nodes: [
      { id: "file:.codex/project-skill-config.md", kind: "file", source_file: ".codex/project-skill-config.md", category: "docs", community_id: 1 },
      { id: "file:.omx/ledger/README.md", kind: "file", source_file: ".omx/ledger/README.md", category: "docs", community_id: 1 },
      { id: "file:docs/planner-solidstart-platform-direction-spec.md", kind: "file", source_file: "docs/planner-solidstart-platform-direction-spec.md", category: "docs", community_id: 2 },
      { id: "file:docs/project-plan.md", kind: "file", source_file: "docs/project-plan.md", category: "docs", community_id: 2 },
    ],
    edges: [
      { source: "file:.codex/project-skill-config.md", target: "file:.omx/ledger/README.md", relation: "references" },
      { source: "file:docs/planner-solidstart-platform-direction-spec.md", target: "file:docs/project-plan.md", relation: "references" },
    ],
  };
  const manifest = { built_at: "2026-04-07T00:00:00Z", total_files: 4, nodes: 4, edges: 2, communities: 2 };

  const report = analyzeFamilyFidelity(ledger, graph, manifest);
  const docDoc = report.relationship_categories.find(item => item.category === "doc↔doc");

  assert.equal(docDoc.representative_examples[0].source_file, "docs/planner-solidstart-platform-direction-spec.md");
  assert.equal(docDoc.representative_examples[1].source_file, ".codex/project-skill-config.md");
});

test("renderReportMarkdown keeps findings and recommendations separate", () => {
  const markdown = renderReportMarkdown({
    generated_at: "2026-04-06T00:00:00Z",
    graph: { built_at: "2026-04-06T00:00:00Z", total_files: 1, nodes: 2, edges: 1, communities: 1 },
    relationship_categories: [
      {
        category: "doc↔code",
        count: 1,
        ranking_strategy: "blend(...)",
        representative_examples: [
          { source_file: "docs/guide.md", target_file: "src/a.py", relation: "references", score: 10 },
        ],
        additional_examples: [
          { source_file: ".codex/project-skill-config.md", target_file: ".omx/ledger/README.md", relation: "references", score: 1 },
        ],
      },
    ],
    clustering_summary: [
      {
        id: 7,
        size: 42,
        file_count: 12,
        cohesion: 0.41,
        dominant_family: "planner-solidstart",
        dominant_family_share: 0.75,
        sample_labels: ["docs/a.md", "docs/b.md"],
      },
    ],
    overall_findings: [{ classification: "likely_real_issue", area: "graph_file_coverage", summary: "Gap" }],
    overall_recommendations: [{ summary: "Fix later" }],
    families: [
      {
        family_id: "planner-ledger/repo-graph",
        family_provenance: {
          primary_source: "canonical_tag",
          fallback_used: false,
          sources_seen: ["canonical_tag"],
        },
        canonical_artifact_count: 2,
        graph_backed_artifact_count: 1,
        missing_filesystem_artifacts: [],
        missing_graph_artifacts: ["foo"],
        membership_evaluation: {
          status: "partial-file-node-match",
          note: "note",
          wrong_family_membership_detected: false,
        },
        relationship_evaluation: {
          average_incident_edges: 1,
          cross_file_edge_count: 0,
        },
        clustering_evaluation: {
          unique_communities: 1,
          dominant_community_share: 1,
        },
        findings: [{ classification: "likely_real_issue", area: "graph_file_coverage", summary: "family gap" }],
        recommendations: [{ summary: "family fix later" }],
      },
    ],
  });

  assert.match(markdown, /## Overall Findings/);
  assert.match(markdown, /## Overall Recommendations/);
  assert.match(markdown, /## Relationship Category Evidence/);
  assert.match(markdown, /Ranking strategy:/);
  assert.match(markdown, /Additional lower-priority examples/);
  assert.match(markdown, /## Community Cohesion Signals/);
  assert.match(markdown, /### Family Provenance/);
  assert.match(markdown, /Primary source: \*\*canonical_tag\*\*/);
  assert.match(markdown, /Community \*\*7\*\*/);
  assert.match(markdown, /dominant_family=planner-solidstart/);
  assert.match(markdown, /docs\/guide\.md/);
  assert.match(markdown, /### Findings/);
  assert.match(markdown, /### Recommendations/);
  assert.match(markdown, /family fix later/);
});
