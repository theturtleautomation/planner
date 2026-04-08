#!/usr/bin/env node

import { existsSync } from "node:fs";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const ROOT_DIR = path.resolve(SCRIPT_DIR, "..");
const LEDGER_PATH = path.join(ROOT_DIR, ".omx/ledger/planner-ledger.json");
const GRAPH_PATH = path.join(ROOT_DIR, ".omx/graphs/repo-graph/graph.json");
const MANIFEST_PATH = path.join(ROOT_DIR, ".omx/graphs/repo-graph/manifest.json");
const REPORT_DIR = path.join(ROOT_DIR, ".omx/reports");
const REPORT_JSON_PATH = path.join(REPORT_DIR, "repo-graph-family-fidelity-report.json");
const REPORT_MARKDOWN_PATH = path.join(REPORT_DIR, "repo-graph-family-fidelity-report.md");

const PRODUCT_CENTRAL_PREFIXES = [
  "planner-solid/src/",
  "planner-web/src/",
  "planner-server/src/",
  "planner-core/src/",
  "planner-schemas/src/",
  "planner-tui/src/",
];

const PRODUCT_CENTRAL_DOC_PREFIXES = [
  ".omx/ledger/current-status.md",
  "docs/planner-solidstart-",
  "docs/import-existing-project",
  "docs/blueprint-",
  "docs/knowledge-library",
  "docs/socratic-",
  "docs/builder-fusion-",
  "docs/planner-ui-reset-",
  "docs/planner-design-system-",
  "docs/planner-ledger-",
];

const FALLBACK_FAMILY_TAG_ALIASES = new Map([
  ["planner-wide", "planner-ledger/repo-graph"],
  ["solidstart", "planner-solidstart"],
  ["design-system", "design-ui-audit"],
  ["ui-reset", "design-ui-audit"],
  ["audit", "design-ui-audit"],
  ["import", "import-existing-project"],
  ["socratic", "socratic"],
  ["builder", "builder-fusion"],
  ["blueprint", "blueprint-knowledge"],
]);

function canonicalFamilyTag(item) {
  return (item.tags ?? []).find(tag => tag.startsWith("family:")) ?? null;
}

function classifyFamily(item) {
  const canonicalTag = canonicalFamilyTag(item);
  if (canonicalTag) {
    return {
      family_id: canonicalTag.slice("family:".length),
      source: "canonical_tag",
    };
  }

  for (const tag of item.tags ?? []) {
    const fallbackFamily = FALLBACK_FAMILY_TAG_ALIASES.get(tag);
    if (fallbackFamily) {
      return {
        family_id: fallbackFamily,
        source: "fallback_tag_alias",
      };
    }
  }

  return {
    family_id: "other",
    source: "unclassified",
  };
}

export function collectFamiliesFromLedger(ledger) {
  const families = new Map();

  for (const item of ledger.items ?? []) {
    if (!["initiative", "workstream"].includes(item.kind)) {
      continue;
    }

    const familyInfo = classifyFamily(item);
    const familyId = familyInfo.family_id;
    if (familyId === "other") {
      continue;
    }

    if (!families.has(familyId)) {
      families.set(familyId, {
        family_id: familyId,
        items: [],
        canonical_artifacts: [],
        family_sources: [],
      });
    }

    const family = families.get(familyId);
    family.items.push({
      id: item.id,
      kind: item.kind,
      title: item.title,
      routing_state: item.routing_state,
      family_source: familyInfo.source,
    });
    if (!family.family_sources.includes(familyInfo.source)) {
      family.family_sources.push(familyInfo.source);
    }

    for (const artifact of item.artifacts ?? []) {
      if (!family.canonical_artifacts.includes(artifact)) {
        family.canonical_artifacts.push(artifact);
      }
    }
  }

  return [...families.values()];
}

function buildGraphIndexes(graph) {
  const nodeById = new Map();
  const fileNodesByPath = new Map();
  const incidentEdgeCountByFile = new Map();
  const communityFiles = new Map();

  for (const node of graph.nodes ?? []) {
    nodeById.set(node.id, node);
    if (node.kind === "file" && node.source_file) {
      fileNodesByPath.set(node.source_file, node);
      incidentEdgeCountByFile.set(node.source_file, 0);
      if (!communityFiles.has(node.community_id)) {
        communityFiles.set(node.community_id, []);
      }
      communityFiles.get(node.community_id).push(node.source_file);
    }
  }

  const crossFileEdgePairs = [];
  for (const edge of graph.edges ?? []) {
    const sourceNode = nodeById.get(edge.source);
    const targetNode = nodeById.get(edge.target);
    const sourceFile = sourceNode?.source_file ?? null;
    const targetFile = targetNode?.source_file ?? null;

    if (sourceFile && incidentEdgeCountByFile.has(sourceFile)) {
      incidentEdgeCountByFile.set(sourceFile, (incidentEdgeCountByFile.get(sourceFile) ?? 0) + 1);
    }
    if (targetFile && incidentEdgeCountByFile.has(targetFile)) {
      incidentEdgeCountByFile.set(targetFile, (incidentEdgeCountByFile.get(targetFile) ?? 0) + 1);
    }

    if (sourceFile && targetFile && sourceFile !== targetFile) {
      crossFileEdgePairs.push({
        source_file: sourceFile,
        target_file: targetFile,
        relation: edge.relation ?? "unknown",
      });
    }
  }

  return {
    fileNodesByPath,
    incidentEdgeCountByFile,
    communityFiles,
    crossFileEdgePairs,
  };
}

function mean(values) {
  if (values.length === 0) {
    return 0;
  }
  return values.reduce((sum, value) => sum + value, 0) / values.length;
}

function relationshipCategoryFor(sourceCategory, targetCategory) {
  if (sourceCategory === "docs" && targetCategory === "docs") {
    return "doc↔doc";
  }
  if (sourceCategory === "code" && targetCategory === "code") {
    return "code↔code";
  }
  if ((sourceCategory === "docs" && targetCategory === "code") || (sourceCategory === "code" && targetCategory === "docs")) {
    return "doc↔code";
  }
  return null;
}

function isProductCentralPath(filePath) {
  return PRODUCT_CENTRAL_PREFIXES.some(prefix => filePath.startsWith(prefix))
    || PRODUCT_CENTRAL_DOC_PREFIXES.some(prefix => filePath.startsWith(prefix));
}

function pathSignalScore(filePath, familyArtifactPaths) {
  let productCentrality = 0;
  let familyRelevance = 0;
  let architecturalImportance = 0;
  let systemPenalty = 0;

  if (isProductCentralPath(filePath)) {
    productCentrality += 6;
  }
  if (familyArtifactPaths.has(filePath)) {
    familyRelevance += 4;
  }
  if (
    filePath.startsWith("planner-core/src/")
    || filePath.startsWith("planner-server/src/")
    || filePath.startsWith("planner-schemas/src/")
    || filePath.startsWith("planner-tui/src/")
    || filePath.startsWith("planner-web/src/")
    || filePath.startsWith("planner-solid/src/")
  ) {
    architecturalImportance += 3;
  } else if (filePath.startsWith("docs/")) {
    architecturalImportance += 1;
  }

  if (filePath.startsWith(".codex/")) {
    systemPenalty -= 8;
  } else if (filePath.startsWith(".omx/")) {
    systemPenalty -= 5;
  } else if (filePath.startsWith("docs/report/")) {
    systemPenalty -= 4;
  }

  return {
    product_centrality: productCentrality,
    family_relevance: familyRelevance,
    architectural_importance: architecturalImportance,
    system_penalty: systemPenalty,
    total: productCentrality + familyRelevance + architecturalImportance + systemPenalty,
  };
}

function rankedRelationshipExamples(crossFileEdgePairs, fileNodesByPath, families) {
  const familyArtifactPaths = new Set(
    families.flatMap(family => family.canonical_artifacts ?? []),
  );
  const examplesByCategory = new Map();
  const countsByCategory = new Map();

  for (const edge of crossFileEdgePairs) {
    const sourceNode = fileNodesByPath.get(edge.source_file);
    const targetNode = fileNodesByPath.get(edge.target_file);
    const category = relationshipCategoryFor(sourceNode?.category, targetNode?.category);
    if (!category) {
      continue;
    }

    countsByCategory.set(category, (countsByCategory.get(category) ?? 0) + 1);
    if (!examplesByCategory.has(category)) {
      examplesByCategory.set(category, []);
    }

    const sourceSignals = pathSignalScore(edge.source_file, familyArtifactPaths);
    const targetSignals = pathSignalScore(edge.target_file, familyArtifactPaths);
    const relationWeight = edge.relation === "imports" || edge.relation === "contains_module" ? 2 : 1;
    const totalScore = sourceSignals.total + targetSignals.total + relationWeight;

    examplesByCategory.get(category).push({
      source_file: edge.source_file,
      target_file: edge.target_file,
      relation: edge.relation,
      score: totalScore,
      score_breakdown: {
        source: sourceSignals,
        target: targetSignals,
        relation_weight: relationWeight,
      },
    });
  }

  return ["doc↔doc", "code↔code", "doc↔code"].map(category => {
    const ranked = (examplesByCategory.get(category) ?? [])
      .slice()
      .sort((left, right) => (
        right.score - left.score
        || left.source_file.localeCompare(right.source_file)
        || left.target_file.localeCompare(right.target_file)
      ));

    return {
      category,
      count: countsByCategory.get(category) ?? 0,
      ranking_strategy: "blend(product_centrality,family_relevance,architectural_importance,system_penalty)+relation_weight",
      representative_examples: ranked.slice(0, 5),
      additional_examples: ranked.slice(5, 10),
    };
  });
}

function addFinding(target, finding) {
  target.findings.push(finding);
}

function addRecommendation(target, recommendation) {
  target.recommendations.push(recommendation);
}

function fileFamilyMemberships(families) {
  const memberships = new Map();
  for (const family of families) {
    for (const artifact of family.canonical_artifacts ?? []) {
      if (!memberships.has(artifact)) {
        memberships.set(artifact, new Set());
      }
      memberships.get(artifact).add(family.family_id);
    }
  }
  return memberships;
}

export function analyzeFamilyFidelity(ledger, graph, manifest) {
  const families = collectFamiliesFromLedger(ledger);
  const { fileNodesByPath, incidentEdgeCountByFile, communityFiles, crossFileEdgePairs } = buildGraphIndexes(graph);
  const familyMemberships = fileFamilyMemberships(families);
  const relationshipCategories = rankedRelationshipExamples(crossFileEdgePairs, fileNodesByPath, families);

  const report = {
    generated_at: new Date().toISOString(),
    graph: {
      built_at: manifest.built_at ?? graph.built_at ?? null,
      build_reason: manifest.build_reason ?? graph.build_reason ?? null,
      total_files: manifest.total_files ?? graph.total_files ?? 0,
      nodes: manifest.nodes ?? (graph.nodes ?? []).length,
      edges: manifest.edges ?? (graph.edges ?? []).length,
      communities: manifest.communities ?? graph.communities ?? 0,
    },
    clustering_summary: (graph.communities ?? [])
      .slice()
      .map(community => {
        const communityFilesList = (community.sample_labels ?? [])
          .filter(label => fileNodesByPath.has(label));
        const familyCounts = new Map();
        for (const file of (communityFiles.get(community.id) ?? [])) {
          for (const familyId of familyMemberships.get(file) ?? []) {
            familyCounts.set(familyId, (familyCounts.get(familyId) ?? 0) + 1);
          }
        }
        const dominantFamily = [...familyCounts.entries()].sort((a, b) => b[1] - a[1])[0] ?? null;
        const fileCount = community.file_count ?? 0;
        const dominantFamilyShare = !dominantFamily || fileCount === 0
          ? 0
          : Number((dominantFamily[1] / fileCount).toFixed(2));
        return {
          id: community.id,
          size: community.size,
          file_count: community.file_count ?? null,
          cohesion: community.cohesion ?? null,
          dominant_family: dominantFamily?.[0] ?? null,
          dominant_family_share: dominantFamilyShare,
          sample_labels: community.sample_labels ?? [],
          purity_score: Math.max(dominantFamilyShare, community.cohesion ?? 0),
        };
      })
      .sort((left, right) => (
        (right.purity_score ?? 0) - (left.purity_score ?? 0)
        || (right.file_count ?? right.size ?? 0) - (left.file_count ?? left.size ?? 0)
      ))
      .slice(0, 8),
    relationship_categories: relationshipCategories,
    families: [],
    overall_findings: [],
    overall_recommendations: [],
  };

  for (const family of families) {
    const existingArtifacts = family.canonical_artifacts.filter(artifact => existsSync(path.join(ROOT_DIR, artifact)));
    const missingFilesystemArtifacts = family.canonical_artifacts.filter(artifact => !existsSync(path.join(ROOT_DIR, artifact)));
    const graphBackedArtifacts = existingArtifacts.filter(artifact => fileNodesByPath.has(artifact));
    const missingGraphArtifacts = existingArtifacts.filter(artifact => !fileNodesByPath.has(artifact));
    const graphBackedSet = new Set(graphBackedArtifacts);

    const communityCounts = new Map();
    for (const artifact of graphBackedArtifacts) {
      const communityId = fileNodesByPath.get(artifact).community_id;
      communityCounts.set(communityId, (communityCounts.get(communityId) ?? 0) + 1);
    }

    const crossFileEdges = crossFileEdgePairs.filter(edge => (
      graphBackedSet.has(edge.source_file) && graphBackedSet.has(edge.target_file)
    ));

    const incidentEdgeCounts = graphBackedArtifacts.map(artifact => incidentEdgeCountByFile.get(artifact) ?? 0);
    const categories = [...new Set(graphBackedArtifacts.map(artifact => fileNodesByPath.get(artifact)?.category).filter(Boolean))];

    const familyReport = {
      family_id: family.family_id,
      family_provenance: {
        primary_source: family.family_sources.includes("canonical_tag") ? "canonical_tag" : family.family_sources[0] ?? "unknown",
        fallback_used: family.family_sources.some(source => source !== "canonical_tag"),
        sources_seen: family.family_sources,
      },
      items: family.items,
      canonical_artifact_count: family.canonical_artifacts.length,
      existing_artifact_count: existingArtifacts.length,
      graph_backed_artifact_count: graphBackedArtifacts.length,
      missing_filesystem_artifacts: missingFilesystemArtifacts,
      missing_graph_artifacts: missingGraphArtifacts,
      membership_evaluation: {
        status: missingGraphArtifacts.length === 0 ? "exact-file-node-match" : "partial-file-node-match",
        note: missingGraphArtifacts.length === 0
          ? "Indexed canonical artifacts map cleanly to exact file nodes."
          : "Some canonical artifacts exist on disk but have no file node in the repo graph.",
        wrong_family_membership_detected: false,
      },
      relationship_evaluation: {
        average_incident_edges: Number(mean(incidentEdgeCounts).toFixed(2)),
        min_incident_edges: incidentEdgeCounts.length ? Math.min(...incidentEdgeCounts) : 0,
        max_incident_edges: incidentEdgeCounts.length ? Math.max(...incidentEdgeCounts) : 0,
        cross_file_edge_count: crossFileEdges.length,
      },
      clustering_evaluation: {
        unique_communities: communityCounts.size,
        dominant_community_share: graphBackedArtifacts.length === 0
          ? 0
          : Number((Math.max(...communityCounts.values(), 0) / graphBackedArtifacts.length).toFixed(2)),
        community_sizes: [...communityCounts.entries()].sort((a, b) => b[1] - a[1]).map(([community_id, artifact_count]) => ({
          community_id,
          artifact_count,
          total_graph_files_in_community: (communityFiles.get(community_id) ?? []).length,
          sample_files: (communityFiles.get(community_id) ?? []).slice(0, 5),
        })),
      },
      findings: [],
      recommendations: [],
      evidence_notes: {
        graph_backed_categories: categories,
        graph_backed_artifact_sample: graphBackedArtifacts.slice(0, 10),
      },
    };

    if (missingFilesystemArtifacts.length > 0) {
      addFinding(familyReport, {
        classification: "likely_real_issue",
        area: "canonical_file_coverage",
        severity: "medium",
        summary: `${missingFilesystemArtifacts.length} canonical artifact(s) are referenced by the ledger but missing from disk.`,
        evidence: missingFilesystemArtifacts.slice(0, 10),
      });
      addRecommendation(familyReport, {
        type: "follow_up",
        summary: "Validate whether missing filesystem artifacts are stale ledger references or missing source files.",
      });
    }

    if (missingGraphArtifacts.length > 0) {
      addFinding(familyReport, {
        classification: "likely_real_issue",
        area: "graph_file_coverage",
        severity: "high",
        summary: `${missingGraphArtifacts.length} existing canonical artifact(s) are absent from repo-graph file nodes.`,
        evidence: missingGraphArtifacts.slice(0, 10),
      });
      addRecommendation(familyReport, {
        type: "follow_up",
        summary: "Review repo-graph inclusion rules for these canonical artifacts before using graph fidelity claims as complete coverage.",
      });
    }

    if (graphBackedArtifacts.length > 1 && crossFileEdges.length === 0) {
      addFinding(familyReport, {
        classification: "likely_real_issue",
        area: "relationship_coverage",
        severity: "medium",
        summary: "Canonical family files have no cross-file graph relationships, so family-level relationship coverage is weak.",
        evidence: graphBackedArtifacts.slice(0, 10),
      });
      addRecommendation(familyReport, {
        type: "follow_up",
        summary: "If cross-family reasoning is important, extend repo-graph extraction beyond intra-file structure and exact-node lookup.",
      });
    }

    if (graphBackedArtifacts.length > 1 && communityCounts.size === graphBackedArtifacts.length) {
      addFinding(familyReport, {
        classification: "likely_acceptable_noise",
        area: "community_clustering",
        severity: "low",
        summary: "Each indexed canonical file sits in its own file-level community, so clustering is currently weak as a family-cohesion signal.",
        evidence: familyReport.clustering_evaluation.community_sizes.slice(0, 5),
      });
      addRecommendation(familyReport, {
        type: "follow_up",
        summary: "Treat community spread as advisory until repo-graph supports stronger cross-file clustering for family-level analysis.",
      });
    }

    if (familyReport.findings.length === 0) {
      addFinding(familyReport, {
        classification: "no_obvious_mismatch_detected",
        area: "overall",
        severity: "low",
        summary: "No obvious graph-fidelity mismatches were detected for the current family inputs.",
        evidence: familyReport.evidence_notes.graph_backed_artifact_sample.slice(0, 5),
      });
    }

    report.families.push(familyReport);
  }

  const familiesWithMissingGraphArtifacts = report.families.filter(family => family.missing_graph_artifacts.length > 0);
  if (familiesWithMissingGraphArtifacts.length > 0) {
    report.overall_findings.push({
      classification: "likely_real_issue",
      area: "graph_file_coverage",
      summary: `${familiesWithMissingGraphArtifacts.length} family/families contain canonical artifacts that exist on disk but are absent from the repo graph.`,
      evidence: familiesWithMissingGraphArtifacts.map(family => ({
        family_id: family.family_id,
        missing_graph_artifact_count: family.missing_graph_artifacts.length,
      })),
    });
    report.overall_recommendations.push({
      type: "follow_up",
      summary: "Prioritize indexing gaps for `.omx/*` canonical planner artifacts before treating repo-graph as full-family coverage for planner-ledger/socratic families.",
    });
  }

  const weakRelationshipFamilies = report.families.filter(family => family.relationship_evaluation.cross_file_edge_count === 0 && family.graph_backed_artifact_count > 1);
  if (weakRelationshipFamilies.length > 0) {
    report.overall_findings.push({
      classification: "likely_real_issue",
      area: "relationship_coverage",
      summary: "Cross-file relationship coverage is weak across the analyzed families; current graph structure is mostly intra-file.",
      evidence: weakRelationshipFamilies.map(family => family.family_id),
    });
    report.overall_recommendations.push({
      type: "follow_up",
      summary: "Use current repo-graph confidently for exact file-node coverage, but treat family-level relationship/clustering conclusions as partial until cross-file extraction improves.",
    });
  }

  for (const relationshipCategory of report.relationship_categories) {
    if (relationshipCategory.count === 0) {
      report.overall_findings.push({
        classification: "likely_real_issue",
        area: "relationship_category_coverage",
        summary: `No repo-wide cross-file evidence was found for ${relationshipCategory.category}.`,
        evidence: [],
      });
      report.overall_recommendations.push({
        type: "follow_up",
        summary: `Improve extraction so ${relationshipCategory.category} has materially useful representative examples.`,
      });
    }
  }

  return report;
}

function summarizeFamilyRow(family) {
  return `| ${family.family_id} | ${family.canonical_artifact_count} | ${family.graph_backed_artifact_count} | ${family.missing_graph_artifacts.length} | ${family.relationship_evaluation.cross_file_edge_count} | ${family.clustering_evaluation.unique_communities} |`;
}

export function renderReportMarkdown(report) {
  const lines = [];

  lines.push("# Repo-Graph Family Fidelity Report");
  lines.push("");
  lines.push(`Generated: \`${report.generated_at}\``);
  lines.push(`Graph built at: \`${report.graph.built_at ?? "unknown"}\``);
  lines.push(`Graph counts: files=${report.graph.total_files}, nodes=${report.graph.nodes}, edges=${report.graph.edges}, communities=${report.graph.communities}`);
  lines.push("");
  lines.push("## Summary");
  lines.push("");
  lines.push("| Family | Canonical artifacts | Graph-backed | Missing from graph | Cross-file edges | Unique communities |");
  lines.push("| --- | ---: | ---: | ---: | ---: | ---: |");
  for (const family of report.families) {
    lines.push(summarizeFamilyRow(family));
  }
  lines.push("");

  lines.push("## Overall Findings");
  lines.push("");
  if (report.overall_findings.length === 0) {
    lines.push("- No cross-family issues detected.");
  } else {
    for (const finding of report.overall_findings) {
      lines.push(`- **${finding.classification}** (${finding.area}) — ${finding.summary}`);
    }
  }
  lines.push("");

  lines.push("## Overall Recommendations");
  lines.push("");
  if (report.overall_recommendations.length === 0) {
    lines.push("- None.");
  } else {
    for (const recommendation of report.overall_recommendations) {
      lines.push(`- ${recommendation.summary}`);
    }
  }
  lines.push("");

  lines.push("## Relationship Category Evidence");
  lines.push("");
  for (const category of report.relationship_categories ?? []) {
    lines.push(`### ${category.category}`);
    lines.push("");
    lines.push(`- Cross-file edge count: **${category.count}**`);
    lines.push(`- Ranking strategy: \`${category.ranking_strategy}\``);
    if ((category.representative_examples ?? []).length === 0) {
      lines.push("- Representative examples: none");
    } else {
      lines.push("- Representative examples:");
      for (const example of category.representative_examples) {
        lines.push(`  - \`${example.source_file}\` → \`${example.target_file}\` _(${example.relation}; score=${example.score})_`);
      }
    }
    if ((category.additional_examples ?? []).length > 0) {
      lines.push("- Additional lower-priority examples:");
      for (const example of category.additional_examples) {
        lines.push(`  - \`${example.source_file}\` → \`${example.target_file}\` _(${example.relation}; score=${example.score})_`);
      }
    }
    lines.push("");
  }

  lines.push("## Community Cohesion Signals");
  lines.push("");
  for (const community of report.clustering_summary ?? []) {
    lines.push(`- Community **${community.id}** — files=${community.file_count ?? "n/a"}; nodes=${community.size}; cohesion=${community.cohesion ?? "n/a"}; dominant_family=${community.dominant_family ?? "n/a"}; dominant_family_share=${community.dominant_family_share ?? "n/a"}; sample=${(community.sample_labels ?? []).slice(0, 5).join(", ")}`);
  }
  if ((report.clustering_summary ?? []).length === 0) {
    lines.push("- None");
  }
  lines.push("");

  for (const family of report.families) {
    lines.push(`## Family: ${family.family_id}`);
    lines.push("");
    lines.push("### Family Provenance");
    lines.push("");
    lines.push(`- Primary source: **${family.family_provenance.primary_source}**`);
    lines.push(`- Fallback used: **${family.family_provenance.fallback_used ? "yes" : "no"}**`);
    lines.push(`- Sources seen: ${family.family_provenance.sources_seen.join(", ")}`);
    lines.push("");

    lines.push("### Coverage");
    lines.push("");
    lines.push(`- Canonical artifacts: **${family.canonical_artifact_count}**`);
    lines.push(`- Graph-backed artifacts: **${family.graph_backed_artifact_count}**`);
    lines.push(`- Missing from filesystem: **${family.missing_filesystem_artifacts.length}**`);
    lines.push(`- Missing from graph: **${family.missing_graph_artifacts.length}**`);
    if (family.missing_graph_artifacts.length > 0) {
      lines.push(`- Missing graph examples: ${family.missing_graph_artifacts.slice(0, 5).map(file => `\`${file}\``).join(", ")}`);
    }
    lines.push("");

    lines.push("### Membership Evaluation");
    lines.push("");
    lines.push(`- Status: **${family.membership_evaluation.status}**`);
    lines.push(`- Note: ${family.membership_evaluation.note}`);
    lines.push(`- Wrong family membership detected: **${family.membership_evaluation.wrong_family_membership_detected ? "yes" : "no"}**`);
    lines.push("");

    lines.push("### Relationship Evaluation");
    lines.push("");
    lines.push(`- Average incident graph edges per indexed file: **${family.relationship_evaluation.average_incident_edges}**`);
    lines.push(`- Cross-file edges within family: **${family.relationship_evaluation.cross_file_edge_count}**`);
    lines.push("");

    lines.push("### Community Clustering");
    lines.push("");
    lines.push(`- Unique communities across indexed family files: **${family.clustering_evaluation.unique_communities}**`);
    lines.push(`- Dominant community share: **${family.clustering_evaluation.dominant_community_share}**`);
    lines.push("");

    lines.push("### Findings");
    lines.push("");
    for (const finding of family.findings) {
      lines.push(`- **${finding.classification}** (${finding.area}) — ${finding.summary}`);
    }
    lines.push("");

    lines.push("### Recommendations");
    lines.push("");
    for (const recommendation of family.recommendations) {
      lines.push(`- ${recommendation.summary}`);
    }
    if (family.recommendations.length === 0) {
      lines.push("- None.");
    }
    lines.push("");
  }

  lines.push("## Boundary Notes");
  lines.push("");
  lines.push("- This report is read-only and diagnostic.");
  lines.push("- Findings summarize evidence from the current ledger + repo-graph artifacts.");
  lines.push("- Recommendations are labeled inference and are not treated as truth or automatically applied changes.");
  lines.push("");

  return `${lines.join("\n")}\n`;
}

export async function loadJson(filePath) {
  return JSON.parse(await readFile(filePath, "utf8"));
}

export async function writeReportFiles(report) {
  await mkdir(REPORT_DIR, { recursive: true });
  await writeFile(REPORT_JSON_PATH, `${JSON.stringify(report, null, 2)}\n`, "utf8");
  await writeFile(REPORT_MARKDOWN_PATH, renderReportMarkdown(report), "utf8");
}

export async function runFamilyFidelityReport() {
  const ledger = await loadJson(LEDGER_PATH);
  const graph = await loadJson(GRAPH_PATH);
  const manifest = await loadJson(MANIFEST_PATH);
  const report = analyzeFamilyFidelity(ledger, graph, manifest);
  await writeReportFiles(report);
  return report;
}

async function main() {
  const report = await runFamilyFidelityReport();
  console.log(`Wrote ${REPORT_JSON_PATH}`);
  console.log(`Wrote ${REPORT_MARKDOWN_PATH}`);
  console.log(`Families analyzed: ${report.families.length}`);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  await main();
}
