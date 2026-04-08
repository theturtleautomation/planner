#!/usr/bin/env node

import { existsSync } from "node:fs";
import { mkdir, readFile, readdir, stat, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const ROOT_DIR = path.resolve(SCRIPT_DIR, "..");
const REPORT_DIR = path.join(ROOT_DIR, "docs/report");
const REPORT_INDEX_PATH = path.join(REPORT_DIR, "docs-index.json");
const REPORT_MARKDOWN_PATH = path.join(REPORT_DIR, "docs-report.md");

const ROOT_DOCS = [
  "README.md",
  "AGENTS.md",
  "CLAUDE.md",
  ".codex/project-skill-config.md",
];

const DOC_DIRS = [
  "docs",
  ".omx/ledger",
];

const DOC_REPORT_EXCLUDES = new Set([
  "docs/report/docs-index.json",
  "docs/report/docs-report.md",
  "docs/report/verification.json",
  "docs/report/verification.md",
]);
const REPO_ROOTISH_PREFIXES = [
  ".omx/",
  ".codex/",
  ".agents/",
  "docs/",
  "scripts/",
  "planner-",
];

const PINNED_DOCS = new Set([
  "README.md",
  "AGENTS.md",
  "CLAUDE.md",
  ".codex/project-skill-config.md",
  ".omx/ledger/session-start-and-doc-index.md",
  ".omx/ledger/project-plan.md",
  ".omx/ledger/current-status.md",
  ".omx/ledger/README.md",
]);

const LINK_RE = /\[[^\]]*\]\(([^)]+)\)/g;
const INLINE_PATH_RE = /`([^`\n]*\/[^`\n]*)`/g;
const PATHY_EXT_RE = /\.(md|json|mjs|js|ts|tsx|rs|sh|toml|yaml|yml)$/i;

function relativeRepoPath(filePath) {
  return path.relative(ROOT_DIR, filePath).replaceAll(path.sep, "/");
}

function normalizeDocRef(ref, sourceFile) {
  if (!ref) {
    return null;
  }
  if (ref.startsWith("http://") || ref.startsWith("https://") || ref.startsWith("#") || ref.startsWith("mailto:")) {
    return null;
  }

  let cleaned = ref.trim();
  if (cleaned.startsWith("/home/thetu/planner/")) {
    cleaned = cleaned.slice("/home/thetu/planner/".length);
  }
  if (cleaned.startsWith("/")) {
    cleaned = cleaned.slice(1);
  }
  const anchorIndex = cleaned.indexOf("#");
  if (anchorIndex >= 0) {
    cleaned = cleaned.slice(0, anchorIndex);
  }
  const queryIndex = cleaned.indexOf("?");
  if (queryIndex >= 0) {
    cleaned = cleaned.slice(0, queryIndex);
  }
  if (!cleaned) {
    return null;
  }
  if (/[*?[\]{}()<>|]/.test(cleaned)) {
    return null;
  }

  const sourceDir = path.dirname(sourceFile);
  const treatAsRepoRoot = ROOT_DOCS.includes(cleaned)
    || REPO_ROOTISH_PREFIXES.some(prefix => cleaned.startsWith(prefix));
  const candidate = treatAsRepoRoot
    ? path.normalize(cleaned)
    : cleaned.startsWith(".")
      ? path.normalize(path.join(sourceDir, cleaned))
      : path.normalize(cleaned);
  return candidate.replaceAll(path.sep, "/");
}

function extractLinks(markdown, sourceFile) {
  const links = new Set();
  for (const match of markdown.matchAll(LINK_RE)) {
    const normalized = normalizeDocRef(match[1], sourceFile);
    if (normalized) {
      links.add(normalized);
    }
  }
  return [...links];
}

function extractInlinePathRefs(markdown, sourceFile) {
  const refs = new Set();
  for (const match of markdown.matchAll(INLINE_PATH_RE)) {
    const candidate = match[1].trim();
    if (candidate.includes(" ")) {
      continue;
    }
    if (!PATHY_EXT_RE.test(candidate)) {
      continue;
    }
    const normalized = normalizeDocRef(candidate, sourceFile);
    if (normalized) {
      refs.add(normalized);
    }
  }
  return [...refs];
}

async function walkMarkdownFiles(dirPath, results) {
  const entries = await readdir(path.join(ROOT_DIR, dirPath), { withFileTypes: true });
  for (const entry of entries) {
    if (entry.name === "node_modules" || entry.name === ".git") {
      continue;
    }
    const rel = path.posix.join(dirPath, entry.name);
    if (entry.isDirectory()) {
      await walkMarkdownFiles(rel, results);
      continue;
    }
    if (!entry.name.endsWith(".md")) {
      continue;
    }
    if (DOC_REPORT_EXCLUDES.has(rel)) {
      continue;
    }
    results.push(rel);
  }
}

async function collectDocPaths() {
  const docs = [];
  for (const file of ROOT_DOCS) {
    if (existsSync(path.join(ROOT_DIR, file))) {
      docs.push(file);
    }
  }
  for (const dir of DOC_DIRS) {
    if (existsSync(path.join(ROOT_DIR, dir))) {
      await walkMarkdownFiles(dir, docs);
    }
  }
  return [...new Set(docs)].sort();
}

function docScore({ broken, missingRefs, orphan, staleDays }) {
  let score = 100;
  score -= Math.min(40, broken.length * 8);
  score -= Math.min(30, missingRefs.length * 2);
  score -= orphan ? 10 : 0;
  score -= Math.min(20, Math.floor(staleDays / 30) * 2);
  return Math.max(0, score);
}

function clusterKey(docPath) {
  const stem = path.basename(docPath, ".md").toLowerCase();
  return stem
    .replace(/^planner-/, "")
    .replace(/^socratic-/, "")
    .replace(/^import-existing-project-/, "import-existing-project")
    .replace(/^phase-\d+-/, "phase")
    .split("-")
    .slice(0, 3)
    .join("-");
}

export async function buildDocsIndex() {
  const docPaths = await collectDocPaths();
  const records = [];
  const incomingLinks = new Map(docPaths.map(doc => [doc, 0]));
  const docsByPath = new Set(docPaths);

  const rawDocs = new Map();
  for (const docPath of docPaths) {
    const absolute = path.join(ROOT_DIR, docPath);
    const content = await readFile(absolute, "utf8");
    const stats = await stat(absolute);
    const links = extractLinks(content, docPath);
    const inlineRefs = extractInlinePathRefs(content, docPath);
    rawDocs.set(docPath, { content, stats, links, inlineRefs });
    for (const link of links) {
      if (docsByPath.has(link)) {
        incomingLinks.set(link, (incomingLinks.get(link) ?? 0) + 1);
      }
    }
  }

  for (const docPath of docPaths) {
    const { stats, links, inlineRefs } = rawDocs.get(docPath);
    const broken = links.filter(link => !existsSync(path.join(ROOT_DIR, link)));
    const missingRefs = inlineRefs.filter(ref => !existsSync(path.join(ROOT_DIR, ref)));
    const staleDays = Math.max(0, Math.floor((Date.now() - stats.mtimeMs) / 86_400_000));
    const orphan = !PINNED_DOCS.has(docPath) && (incomingLinks.get(docPath) ?? 0) === 0;
    records.push({
      path: docPath,
      score: docScore({ broken, missingRefs, orphan, staleDays }),
      orphan,
      broken,
      missingRefs,
      staleDays,
      updated: Math.floor(stats.mtimeMs),
    });
  }

  return {
    generatedAt: new Date().toISOString(),
    scope: "repo-owned markdown docs",
    docs: records.sort((left, right) => left.path.localeCompare(right.path)),
  };
}

function buildClusters(records) {
  const groups = new Map();
  for (const record of records) {
    const key = clusterKey(record.path);
    if (!groups.has(key)) {
      groups.set(key, []);
    }
    groups.get(key).push(record.path);
  }
  return [...groups.values()]
    .filter(group => group.length > 1)
    .sort((left, right) => right.length - left.length || left[0].localeCompare(right[0]))
    .slice(0, 25);
}

export function renderDocsReport(index) {
  const lines = [];
  const riskyDocs = [...index.docs]
    .filter(doc => doc.score < 70)
    .sort((left, right) => left.score - right.score || left.path.localeCompare(right.path))
    .slice(0, 40);
  const clusters = buildClusters(index.docs);

  lines.push("# Docs Health Report");
  lines.push("");
  lines.push(`Generated: ${index.generatedAt}`);
  lines.push("");
  lines.push("This is a **derived report**, regenerated from repo-owned markdown docs.");
  lines.push("Canonical planning/bootstrap truth lives in the OMX ledger surfaces, not here.");
  lines.push("");
  lines.push("## Top risky docs (score < 70)");
  if (riskyDocs.length === 0) {
    lines.push("- None");
  } else {
    for (const doc of riskyDocs) {
      const broken = doc.broken.length > 0 ? `, broken links: ${doc.broken.slice(0, 3).join(", ")}${doc.broken.length > 3 ? "…" : ""}` : "";
      const missing = doc.missingRefs.length > 0 ? `, missing refs: ${doc.missingRefs.slice(0, 3).join(", ")}${doc.missingRefs.length > 3 ? "…" : ""}` : "";
      lines.push(`- **${doc.path}** — score ${doc.score}, staleDays ${doc.staleDays}, orphan ${doc.orphan ? "yes" : "no"}${broken}${missing}`);
    }
  }
  lines.push("");
  lines.push("## Near-duplicate clusters");
  if (clusters.length === 0) {
    lines.push("- None");
  } else {
    clusters.forEach((cluster, indexNumber) => {
      lines.push(`- Cluster ${indexNumber + 1}: ${cluster.join(", ")}`);
    });
  }
  lines.push("");
  lines.push("## Suggested centralization");
  lines.push("- Treat this report as advisory only.");
  lines.push("- Use `.omx/ledger/session-start-and-doc-index.md` and `.omx/ledger/project-plan.md` for repo-owned planning guidance.");
  lines.push("");

  return `${lines.join("\n")}\n`;
}

export async function writeDocsReportArtifacts() {
  const index = await buildDocsIndex();
  await mkdir(REPORT_DIR, { recursive: true });
  await writeFile(REPORT_INDEX_PATH, `${JSON.stringify(index, null, 2)}\n`, "utf8");
  await writeFile(REPORT_MARKDOWN_PATH, renderDocsReport(index), "utf8");
  return index;
}

async function main() {
  const index = await writeDocsReportArtifacts();
  console.log(`Regenerated docs report for ${index.docs.length} repo-owned markdown docs.`);
}

const isMain = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) {
  main().catch(error => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  });
}
