#!/usr/bin/env python3
"""Build and query a bounded repo graph for OMX repo-understanding tasks."""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import re
from collections import Counter, defaultdict, deque
from datetime import datetime, timezone
from pathlib import Path
from typing import Dict, List, Optional, Sequence, Set, Tuple

EXCLUDED_DIRS = {
    ".git",
    ".hg",
    ".svn",
    "node_modules",
    "dist",
    "build",
    "coverage",
    ".output",
    "target",
    ".next",
    ".turbo",
    ".nitro",
    ".cache",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    "__pycache__",
    ".omx/tooling",
}
EXCLUDED_PREFIXES = (
    ".codex/.tmp",
    ".codex/cache",
    ".codex/log",
    ".codex/plugins",
    ".omx",
)
STOP_WORDS = {
    "a",
    "an",
    "and",
    "are",
    "as",
    "at",
    "be",
    "by",
    "connect",
    "connects",
    "does",
    "for",
    "how",
    "in",
    "into",
    "is",
    "of",
    "on",
    "or",
    "the",
    "to",
    "what",
    "with",
}
GENERIC_HEADING_LABELS = {
    "acceptance criteria",
    "automated",
    "boundary",
    "boundaries",
    "commands",
    "current-state evidence",
    "dependencies",
    "examples",
    "files",
    "implementation sync",
    "notes",
    "purpose",
    "routing intent",
    "scope boundaries",
}

CODE_EXTENSIONS = {
    ".py",
    ".rs",
    ".ts",
    ".tsx",
    ".js",
    ".jsx",
    ".mjs",
    ".cjs",
    ".go",
    ".java",
    ".kt",
    ".swift",
    ".rb",
    ".php",
    ".sh",
}
DOC_EXTENSIONS = {".md", ".mdx", ".txt", ".rst"}
MAX_FILE_BYTES = 512 * 1024
DEFAULT_GRAPH_DIR = ".omx/graphs/repo-graph"
MANIFEST_FILENAME = "manifest.json"
GRAPH_VERSION = 2
MAX_COMMUNITY_FRACTION = 0.25
MIN_SPLIT_SIZE = 10

TS_IMPORT_RE = re.compile(
    r"""(?:import\s+.+?\s+from\s+|export\s+.+?\s+from\s+|require\(|import\()['"]([^'"]+)['"]"""
)
PY_IMPORT_RE = re.compile(
    r"""^\s*(?:from\s+([.\w]+)\s+import|import\s+([.\w]+))""",
    re.MULTILINE,
)
RUST_USE_RE = re.compile(r"""^\s*use\s+([^;]+);""", re.MULTILINE)
RUST_MOD_RE = re.compile(r"""^\s*mod\s+(\w+)\s*;""", re.MULTILINE)
MD_LINK_RE = re.compile(r"""\[[^\]]+\]\(([^)#]+)(?:#[^)]+)?\)""")
HEADING_RE = re.compile(r"""^(#{1,6})\s+(.+)$""", re.MULTILINE)
WORD_RE = re.compile(r"""[A-Za-z0-9_\-/.:]+""")
SYMBOL_PATTERNS = {
    "python": re.compile(
        r"""^\s*(?:async\s+def|def|class)\s+([A-Za-z_][A-Za-z0-9_]*)""",
        re.MULTILINE,
    ),
    "rust": re.compile(
        r"""^\s*(?:pub\s+)?(?:async\s+)?(?:fn|struct|enum|trait|mod)\s+([A-Za-z_][A-Za-z0-9_]*)""",
        re.MULTILINE,
    ),
    "ts": re.compile(
        r"""^\s*(?:export\s+)?(?:async\s+)?(?:function|class|interface|type|enum|const)\s+([A-Za-z_][A-Za-z0-9_]*)""",
        re.MULTILINE,
    ),
    "js": re.compile(
        r"""^\s*(?:export\s+)?(?:async\s+)?(?:function|class|const)\s+([A-Za-z_][A-Za-z0-9_]*)""",
        re.MULTILINE,
    ),
    "go": re.compile(r"""^\s*(?:type|func)\s+([A-Za-z_][A-Za-z0-9_]*)""", re.MULTILINE),
    "java": re.compile(r"""\b(?:class|interface|enum|record)\s+([A-Za-z_][A-Za-z0-9_]*)"""),
    "shell": re.compile(r"""^\s*([A-Za-z_][A-Za-z0-9_]*)\s*\(\)\s*\{""", re.MULTILINE),
}


def is_excluded_dir(path: Path) -> bool:
    parts = path.parts
    joined = "/".join(parts)
    if joined in EXCLUDED_DIRS:
        return True
    if any(joined.startswith(prefix) for prefix in EXCLUDED_PREFIXES):
        return True
    return any(part in EXCLUDED_DIRS for part in parts)


def classify_extension(path: Path) -> Optional[str]:
    suffix = path.suffix.lower()
    if suffix in CODE_EXTENSIONS:
        return "code"
    if suffix in DOC_EXTENSIONS:
        return "docs"
    return None


def language_for(path: Path) -> str:
    suffix = path.suffix.lower()
    if suffix == ".py":
        return "python"
    if suffix == ".rs":
        return "rust"
    if suffix in {".ts", ".tsx"}:
        return "ts"
    if suffix in {".js", ".jsx", ".mjs", ".cjs"}:
        return "js"
    if suffix == ".go":
        return "go"
    if suffix in {".java", ".kt", ".swift", ".rb", ".php"}:
        return suffix.lstrip(".")
    if suffix == ".sh":
        return "shell"
    if suffix in DOC_EXTENSIONS:
        return "docs"
    return "text"


def safe_rel(path: Path, root: Path) -> str:
    return path.relative_to(root).as_posix()


def utc_now() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def should_include_file(path: Path) -> bool:
    if not path.is_file():
        return False
    if path.stat().st_size > MAX_FILE_BYTES:
        return False
    return classify_extension(path) is not None


def collect_files(root: Path) -> List[Path]:
    files: List[Path] = []
    for dirpath, dirnames, filenames in os.walk(root):
        dirpath_p = Path(dirpath)
        rel_dir = (
            dirpath_p.relative_to(root)
            if dirpath_p != root
            else Path(".")
        )
        dirnames[:] = [d for d in dirnames if not is_excluded_dir(rel_dir / d)]
        for filename in filenames:
            path = dirpath_p / filename
            rel_path = path.relative_to(root)
            if is_excluded_dir(rel_path.parent):
                continue
            if should_include_file(path):
                files.append(path)
    return sorted(files)


def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        return path.read_text(encoding="utf-8", errors="ignore")


def slugify(value: str) -> str:
    slug = re.sub(r"""[^a-z0-9]+""", "-", value.lower()).strip("-")
    return slug or "item"


def add_node(nodes: Dict[str, dict], node: dict) -> None:
    existing = nodes.get(node["id"])
    if existing:
        existing.update({k: v for k, v in node.items() if v not in (None, "", [])})
    else:
        nodes[node["id"]] = node


def add_edge(
    edges: Dict[Tuple[str, str, str], dict],
    source: str,
    target: str,
    relation: str,
    confidence: str = "EXTRACTED",
) -> None:
    if source == target:
        return
    key = (source, target, relation)
    edges.setdefault(
        key,
        {
            "source": source,
            "target": target,
            "relation": relation,
            "confidence": confidence,
        },
    )


def resolve_relative_module(parent: Path, specifier: str) -> Optional[Path]:
    candidates: List[Path] = []
    base = (parent / specifier).resolve()
    if base.is_file():
        candidates.append(base)
    suffixes = ["", ".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".py", ".rs", ".go"]
    for suffix in suffixes:
        candidate = Path(str(base) + suffix)
        if candidate.is_file():
            candidates.append(candidate)
    for index_name in [
        "index.ts",
        "index.tsx",
        "index.js",
        "index.jsx",
        "index.mjs",
        "index.cjs",
        "__init__.py",
        "mod.rs",
    ]:
        candidate = base / index_name
        if candidate.is_file():
            candidates.append(candidate)
    return candidates[0] if candidates else None


def resolve_python_import(root: Path, parent: Path, module: str) -> Optional[Path]:
    leading_dots = len(module) - len(module.lstrip("."))
    base_module = module.lstrip(".")
    search_base = parent
    for _ in range(max(leading_dots - 1, 0)):
        search_base = search_base.parent
    parts = [p for p in base_module.split(".") if p]
    candidate_roots = []
    if leading_dots:
        candidate_roots.append(search_base)
    candidate_roots.extend(
        [root / "planner-web" / "src", root / "planner-server", root / "planner-schemas", root]
    )
    for candidate_root in candidate_roots:
        candidate = candidate_root.joinpath(*parts)
        if candidate.with_suffix(".py").is_file():
            return candidate.with_suffix(".py")
        init_py = candidate / "__init__.py"
        if init_py.is_file():
            return init_py
    return None


def find_crate_root(path: Path, repo_root: Path) -> Optional[Path]:
    for current in [path.parent, *path.parents]:
        if current == repo_root.parent:
            break
        if (current / "Cargo.toml").is_file():
            return current
    return None


def resolve_rust_module(path: Path, repo_root: Path, module_expr: str) -> Optional[Path]:
    crate_root = find_crate_root(path, repo_root)
    if not crate_root:
        return None
    src_root = crate_root / "src"
    current_dir = path.parent
    module_expr = module_expr.strip()
    if module_expr.startswith("crate::"):
        parts = module_expr[len("crate::") :].split("::")
        base = src_root.joinpath(*parts)
    elif module_expr.startswith("self::"):
        parts = module_expr[len("self::") :].split("::")
        base = current_dir.joinpath(*parts)
    elif module_expr.startswith("super::"):
        parts = module_expr.split("::")
        base_dir = current_dir
        while parts and parts[0] == "super":
            base_dir = base_dir.parent
            parts.pop(0)
        base = base_dir.joinpath(*parts)
    else:
        return None
    for candidate in [base.with_suffix(".rs"), base / "mod.rs"]:
        if candidate.is_file():
            return candidate
    return None


def resolve_markdown_link(root: Path, parent: Path, link: str) -> Optional[Path]:
    if link.startswith(("http://", "https://", "mailto:")):
        return None
    target = (parent / link).resolve() if not link.startswith("/") else (root / link.lstrip("/")).resolve()
    if target.is_file():
        return target
    return None


def extract_file_graph(path: Path, root: Path, nodes: Dict[str, dict], edges: Dict[Tuple[str, str, str], dict]) -> None:
    rel = safe_rel(path, root)
    category = classify_extension(path) or "other"
    language = language_for(path)
    file_id = f"file:{rel}"
    text = read_text(path)
    add_node(
        nodes,
        {
            "id": file_id,
            "label": rel,
            "kind": "file",
            "category": category,
            "language": language,
            "source_file": rel,
            "word_count": len(WORD_RE.findall(text)),
        },
    )

    symbol_pattern = SYMBOL_PATTERNS.get(language)
    if symbol_pattern:
        for match in symbol_pattern.finditer(text):
            name = match.group(1)
            symbol_id = f"symbol:{rel}#{name}"
            add_node(
                nodes,
                {
                    "id": symbol_id,
                    "label": name,
                    "kind": "symbol",
                    "category": category,
                    "language": language,
                    "source_file": rel,
                },
            )
            add_edge(edges, file_id, symbol_id, "contains")

    if category == "code":
        parent = path.parent
        if language in {"ts", "js"}:
            for match in TS_IMPORT_RE.finditer(text):
                specifier = match.group(1)
                if specifier and specifier.startswith((".", "/")):
                    target = resolve_relative_module(parent, specifier)
                    if target and target.exists():
                        add_edge(edges, file_id, f"file:{safe_rel(target, root)}", "imports")
        elif language == "python":
            for match in PY_IMPORT_RE.finditer(text):
                module = match.group(1) or match.group(2)
                if module:
                    target = resolve_python_import(root, parent, module)
                    if target and target.exists():
                        add_edge(edges, file_id, f"file:{safe_rel(target, root)}", "imports")
        elif language == "rust":
            for match in RUST_USE_RE.finditer(text):
                expr = match.group(1).split("{", 1)[0].strip()
                target = resolve_rust_module(path, root, expr)
                if target and target.exists():
                    add_edge(edges, file_id, f"file:{safe_rel(target, root)}", "imports")
            for match in RUST_MOD_RE.finditer(text):
                mod_name = match.group(1)
                target = resolve_relative_module(parent, mod_name)
                if target and target.exists():
                    add_edge(edges, file_id, f"file:{safe_rel(target, root)}", "contains_module")
    elif category == "docs":
        for match in HEADING_RE.finditer(text):
            heading = match.group(2).strip()
            heading_id = f"heading:{rel}#{slugify(heading)}"
            add_node(
                nodes,
                {
                    "id": heading_id,
                    "label": heading,
                    "kind": "heading",
                    "category": "docs",
                    "source_file": rel,
                },
            )
            add_edge(edges, file_id, heading_id, "contains")
        for match in MD_LINK_RE.finditer(text):
            target = resolve_markdown_link(root, path.parent, match.group(1).strip())
            if target and should_include_file(target):
                add_edge(edges, file_id, f"file:{safe_rel(target, root)}", "references")


def file_snapshot(root: Path, files: Sequence[Path]) -> List[dict]:
    snapshot = []
    for path in files:
        snapshot.append(
            {
                "path": safe_rel(path, root),
                "mtime": path.stat().st_mtime,
                "category": classify_extension(path) or "other",
                "language": language_for(path),
            }
        )
    return snapshot


def connected_components(graph: dict) -> List[List[str]]:
    adj = adjacency(graph)
    nodes_by_id = node_lookup(graph)
    seen: Set[str] = set()
    components: List[List[str]] = []
    for node_id in sorted(nodes_by_id):
        if node_id in seen:
            continue
        queue = deque([node_id])
        seen.add(node_id)
        component: List[str] = []
        while queue:
            current = queue.popleft()
            component.append(current)
            for neighbor, _edge in adj.get(current, []):
                if neighbor in seen:
                    continue
                seen.add(neighbor)
                queue.append(neighbor)
        components.append(sorted(component))
    components.sort(key=lambda item: (-len(item), item[0] if item else ""))
    return components


def component_adjacency(graph: dict, component: Sequence[str]) -> Dict[str, Set[str]]:
    component_set = set(component)
    adj = defaultdict(set)
    for node_id in component:
        adj[node_id]
    for edge in graph["edges"]:
        source = edge["source"]
        target = edge["target"]
        if source in component_set and target in component_set:
            adj[source].add(target)
            adj[target].add(source)
    return adj


def bridge_edges(component_adj: Dict[str, Set[str]]) -> Set[Tuple[str, str]]:
    bridges: Set[Tuple[str, str]] = set()
    visited: Set[str] = set()
    discovery: Dict[str, int] = {}
    low: Dict[str, int] = {}
    time = 0

    def dfs(node: str, parent: Optional[str]) -> None:
        nonlocal time
        visited.add(node)
        discovery[node] = time
        low[node] = time
        time += 1
        for neighbor in component_adj[node]:
            if neighbor == parent:
                continue
            if neighbor not in visited:
                dfs(neighbor, node)
                low[node] = min(low[node], low[neighbor])
                if low[neighbor] > discovery[node]:
                    bridges.add(tuple(sorted((node, neighbor))))
            else:
                low[node] = min(low[node], discovery[neighbor])

    for node in component_adj:
        if node not in visited:
            dfs(node, None)
    return bridges


def split_component(graph: dict, component: Sequence[str], max_size: int) -> List[List[str]]:
    component = sorted(component)
    if len(component) <= max_size:
        return [list(component)]

    comp_adj = component_adjacency(graph, component)
    bridges = bridge_edges(comp_adj)
    if not bridges:
        return partition_component_by_distance(graph, component, max_size)

    seen: Set[str] = set()
    result: List[List[str]] = []
    for node_id in component:
        if node_id in seen:
            continue
        queue = deque([node_id])
        seen.add(node_id)
        group: List[str] = []
        while queue:
            current = queue.popleft()
            group.append(current)
            for neighbor in comp_adj[current]:
                if tuple(sorted((current, neighbor))) in bridges:
                    continue
                if neighbor in seen:
                    continue
                seen.add(neighbor)
                queue.append(neighbor)
        group = sorted(group)
        if len(group) > max_size and len(group) < len(component):
            result.extend(split_component(graph, group, max_size))
        else:
            result.append(group)
    if len(result) <= 1:
        return partition_component_by_distance(graph, component, max_size)
    return result


def shortest_distances(component_adj: Dict[str, Set[str]], start: str) -> Dict[str, int]:
    distances = {start: 0}
    queue = deque([start])
    while queue:
        current = queue.popleft()
        for neighbor in component_adj[current]:
            if neighbor in distances:
                continue
            distances[neighbor] = distances[current] + 1
            queue.append(neighbor)
    return distances


def farthest_node(component_adj: Dict[str, Set[str]], start: str) -> str:
    distances = shortest_distances(component_adj, start)
    return max(distances.items(), key=lambda item: (item[1], item[0]))[0]


def partition_component_by_distance(graph: dict, component: Sequence[str], max_size: int) -> List[List[str]]:
    component = sorted(component)
    if len(component) <= max_size:
        return [list(component)]
    comp_adj = component_adjacency(graph, component)
    seed_a = farthest_node(comp_adj, component[0])
    seed_b = farthest_node(comp_adj, seed_a)
    if seed_a == seed_b:
        midpoint = len(component) // 2
        return [component[:midpoint], component[midpoint:]]

    dist_a = shortest_distances(comp_adj, seed_a)
    dist_b = shortest_distances(comp_adj, seed_b)
    group_a: List[str] = []
    group_b: List[str] = []
    for node_id in component:
        a = dist_a.get(node_id, 10**9)
        b = dist_b.get(node_id, 10**9)
        if a < b:
            group_a.append(node_id)
        elif b < a:
            group_b.append(node_id)
        elif len(group_a) <= len(group_b):
            group_a.append(node_id)
        else:
            group_b.append(node_id)
    result: List[List[str]] = []
    for group in (sorted(group_a), sorted(group_b)):
        if not group:
            continue
        if len(group) > max_size:
            result.extend(partition_component_by_distance(graph, group, max_size))
        else:
            result.append(group)
    return result or [list(component)]


def community_partition(graph: dict) -> List[List[str]]:
    nodes_by_id = node_lookup(graph)
    if not nodes_by_id:
        return []
    max_size = max(MIN_SPLIT_SIZE, int(len(nodes_by_id) * MAX_COMMUNITY_FRACTION))
    communities: List[List[str]] = []
    for component in connected_components(graph):
        communities.extend(split_component(graph, sorted(component), max_size))
    communities = [community for community in communities if community]
    communities.sort(key=lambda item: (-len(item), item[0] if item else ""))
    return communities


def community_cohesion(graph: dict, community_nodes: Sequence[str]) -> float:
    nodes = list(community_nodes)
    if len(nodes) <= 1:
        return 1.0
    node_set = set(nodes)
    actual = 0
    for edge in graph["edges"]:
        if edge["source"] in node_set and edge["target"] in node_set:
            actual += 1
    possible = len(nodes) * (len(nodes) - 1) / 2
    if possible == 0:
        return 0.0
    return round(actual / possible, 2)


def apply_community_metadata(graph: dict) -> dict:
    nodes = node_lookup(graph)
    communities = []
    adj = adjacency(graph)
    for community_id, component in enumerate(community_partition(graph)):
        ranked = sorted(component, key=lambda node_id: len(adj.get(node_id, [])), reverse=True)
        labels = [nodes[node_id].get("label", node_id) for node_id in ranked[:5]]
        cohesion = community_cohesion(graph, component)
        for node_id in component:
            nodes[node_id]["community_id"] = community_id
        communities.append(
            {
                "id": community_id,
                "size": len(component),
                "sample_labels": labels,
                "cohesion": cohesion,
            }
        )
    graph["communities"] = communities
    return graph


def build_graph(root: Path, files: Optional[Sequence[Path]] = None) -> dict:
    files = list(files) if files is not None else collect_files(root)
    nodes: Dict[str, dict] = {}
    edges: Dict[Tuple[str, str, str], dict] = {}
    counts = Counter()
    for path in files:
        counts[classify_extension(path) or "other"] += 1
        extract_file_graph(path, root, nodes, edges)
    valid_node_ids = set(nodes)
    filtered_edges = [
        edge
        for edge in edges.values()
        if edge["source"] in valid_node_ids and edge["target"] in valid_node_ids
    ]
    graph = {
        "root": str(root.resolve()),
        "total_files": len(files),
        "counts": dict(counts),
        "nodes": sorted(nodes.values(), key=lambda item: item["id"]),
        "edges": sorted(filtered_edges, key=lambda item: (item["source"], item["target"], item["relation"])),
        "graph_version": GRAPH_VERSION,
    }
    return apply_community_metadata(graph)


def graph_paths(root: Path, graph_dir: Path) -> Tuple[Path, Path, Path]:
    absolute_dir = graph_dir if graph_dir.is_absolute() else root / graph_dir
    absolute_dir.mkdir(parents=True, exist_ok=True)
    return absolute_dir, absolute_dir / "graph.json", absolute_dir / "manifest.json"


def load_manifest(root: Path, graph_dir: Path) -> dict:
    _out_dir, _graph_path, manifest_path = graph_paths(root, graph_dir)
    if not manifest_path.is_file():
        return {}
    return json.loads(manifest_path.read_text(encoding="utf-8"))


def write_manifest(root: Path, graph_dir: Path, graph: dict, snapshot: Sequence[dict], build_reason: str) -> dict:
    _out_dir, _graph_path, manifest_path = graph_paths(root, graph_dir)
    manifest = {
        "graph_version": GRAPH_VERSION,
        "built_at": utc_now(),
        "build_reason": build_reason,
        "root": graph["root"],
        "total_files": graph["total_files"],
        "counts": graph["counts"],
        "nodes": len(graph["nodes"]),
        "edges": len(graph["edges"]),
        "communities": len(graph.get("communities", [])),
        "files": list(snapshot),
    }
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    return manifest


def manifest_diff(previous: dict, current_snapshot: Sequence[dict]) -> dict:
    prev_files = {item["path"]: item for item in previous.get("files", [])}
    curr_files = {item["path"]: item for item in current_snapshot}

    added = sorted(path for path in curr_files if path not in prev_files)
    removed = sorted(path for path in prev_files if path not in curr_files)
    changed = sorted(
        path
        for path, item in curr_files.items()
        if path in prev_files and prev_files[path].get("mtime") != item.get("mtime")
    )
    return {
        "added": added,
        "removed": removed,
        "changed": changed,
        "stale": bool(added or removed or changed),
        "current_total_files": len(curr_files),
        "previous_total_files": len(prev_files),
    }


def save_graph(root: Path, graph_dir: Path, files: Optional[Sequence[Path]] = None, build_reason: str = "build") -> dict:
    out_dir, graph_path, _manifest_path = graph_paths(root, graph_dir)
    files = list(files) if files is not None else collect_files(root)
    snapshot = file_snapshot(root, files)
    graph = build_graph(root, files=files)
    graph["graph_dir"] = str(out_dir)
    graph["built_at"] = utc_now()
    graph["build_reason"] = build_reason
    graph_path.write_text(json.dumps(graph, indent=2) + "\n", encoding="utf-8")
    write_manifest(root, graph_dir, graph, snapshot, build_reason)
    return graph


def load_graph(root: Path, graph_dir: Path) -> dict:
    _out_dir, graph_path, _manifest_path = graph_paths(root, graph_dir)
    if not graph_path.is_file():
        raise FileNotFoundError(f"Graph not found at {graph_path}. Run build first.")
    graph = json.loads(graph_path.read_text(encoding="utf-8"))
    if "communities" not in graph:
        graph = apply_community_metadata(graph)
    return graph


def file_is_newer_than(path: Path, ts: float) -> bool:
    try:
        return path.stat().st_mtime > ts
    except FileNotFoundError:
        return False


def needs_rebuild(root: Path, graph_dir: Path) -> bool:
    _out_dir, graph_path, _manifest_path = graph_paths(root, graph_dir)
    if not graph_path.is_file():
        return True
    manifest = load_manifest(root, graph_dir)
    if manifest.get("graph_version") != GRAPH_VERSION:
        return True
    snapshot = file_snapshot(root, collect_files(root))
    return manifest_diff(manifest, snapshot)["stale"]


def ensure_graph(root: Path, graph_dir: Path, refresh: bool = True) -> dict:
    if refresh and needs_rebuild(root, graph_dir):
        return save_graph(root, graph_dir, build_reason="refresh")
    return load_graph(root, graph_dir)


def graph_status(root: Path, graph_dir: Path) -> dict:
    _out_dir, graph_path, _manifest_path = graph_paths(root, graph_dir)
    manifest = load_manifest(root, graph_dir)
    files = collect_files(root)
    snapshot = file_snapshot(root, files)
    diff = manifest_diff(manifest, snapshot)
    return {
        "root": str(root),
        "graph_path": str(graph_path),
        "graph_exists": graph_path.is_file(),
        "built_at": manifest.get("built_at"),
        "build_reason": manifest.get("build_reason"),
        "last_graph_files": manifest.get("total_files", 0),
        "last_graph_nodes": manifest.get("nodes", 0),
        "last_graph_edges": manifest.get("edges", 0),
        "last_graph_communities": manifest.get("communities", 0),
        "changed_count": len(diff["changed"]),
        "removed_count": len(diff["removed"]),
        "added_count": len(diff["added"]),
        "changed_sample": diff["changed"][:10],
        "removed_sample": diff["removed"][:10],
        "dirty": bool(diff["stale"] or not graph_path.is_file()),
    }


def changed_paths_from_git_diff(root: Path) -> List[str]:
    result = subprocess.run(
        ["git", "-C", str(root), "diff", "--name-only", "--relative"],
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        return []
    return [line.strip() for line in result.stdout.splitlines() if line.strip()]


def normalize_changed_paths(root: Path, changed_paths: Sequence[str]) -> List[str]:
    normalized: List[str] = []
    for raw in changed_paths:
        value = raw.strip()
        if not value:
            continue
        path = Path(value)
        if path.is_absolute():
            try:
                normalized.append(path.relative_to(root).as_posix())
            except ValueError:
                continue
        else:
            normalized.append(path.as_posix())
    return sorted(dict.fromkeys(normalized))


def tracked_relative_paths(root: Path) -> List[str]:
    return [safe_rel(path, root) for path in collect_files(root)]


def post_execution_refresh(root: Path, graph_dir: Path, changed_paths: Sequence[str]) -> dict:
    normalized_paths = normalize_changed_paths(root, changed_paths)
    tracked_paths = set(tracked_relative_paths(root))
    relevant_paths = [path for path in normalized_paths if path in tracked_paths]
    before = graph_status(root, graph_dir)

    result = {
        "outcome": "skipped",
        "reason": "",
        "changed_paths": normalized_paths,
        "relevant_paths": relevant_paths,
        "before": before,
        "after": before,
    }

    if not normalized_paths:
        result["reason"] = "no_changed_paths"
        return result
    if not relevant_paths:
        result["reason"] = "no_repo_graph_relevant_changes"
        return result
    if not before["dirty"]:
        result["reason"] = "graph_already_fresh"
        return result

    updated = save_graph(root, graph_dir, files=collect_files(root), build_reason="post-execution-update")
    after_update = graph_status(root, graph_dir)
    result["after"] = after_update
    if not after_update["dirty"]:
        result["outcome"] = "refreshed-via-update"
        result["reason"] = "relevant_changes_refreshed"
        result["after"]["nodes"] = len(updated["nodes"])
        result["after"]["edges"] = len(updated["edges"])
        return result

    rebuilt = save_graph(root, graph_dir, files=collect_files(root), build_reason="post-execution-rebuild")
    after_rebuild = graph_status(root, graph_dir)
    result["after"] = after_rebuild
    result["after"]["nodes"] = len(rebuilt["nodes"])
    result["after"]["edges"] = len(rebuilt["edges"])
    if not after_rebuild["dirty"]:
        result["outcome"] = "refreshed-via-rebuild"
        result["reason"] = "update_insufficient"
    else:
        result["outcome"] = "refresh-failed"
        result["reason"] = "graph_still_dirty_after_rebuild"
    return result


def normalize_changed_paths(root: Path, changed_paths: Sequence[str]) -> List[str]:
    normalized: List[str] = []
    for raw in changed_paths:
        value = raw.strip()
        if not value:
            continue
        path = Path(value)
        if path.is_absolute():
            try:
                normalized.append(path.relative_to(root).as_posix())
            except ValueError:
                continue
        else:
            normalized.append(Path(value).as_posix())
    return sorted(dict.fromkeys(normalized))


def tracked_relative_paths(root: Path) -> List[str]:
    return [safe_rel(path, root) for path in collect_files(root)]


def post_execution_refresh(
    root: Path,
    graph_dir: Path,
    changed_paths: Sequence[str],
) -> dict:
    normalized_paths = normalize_changed_paths(root, changed_paths)
    tracked_paths = set(tracked_relative_paths(root))
    relevant_paths = [path for path in normalized_paths if path in tracked_paths]
    status_before = graph_status(root, graph_dir)

    result = {
        "outcome": "skipped",
        "reason": "",
        "changed_paths": normalized_paths,
        "relevant_paths": relevant_paths,
        "before": status_before,
        "after": status_before,
    }

    if not normalized_paths:
        result["reason"] = "no_changed_paths"
        return result
    if not relevant_paths:
        result["reason"] = "no_repo_graph_relevant_changes"
        return result
    if not status_before["graph_exists"]:
        graph = save_graph(root, graph_dir, build_reason="post-execution-rebuild")
        result["outcome"] = "refreshed-via-rebuild"
        result["reason"] = "graph_missing"
        result["after"] = graph_status(root, graph_dir)
        result["after"]["nodes"] = len(graph["nodes"])
        result["after"]["edges"] = len(graph["edges"])
        return result
    if not status_before["dirty"]:
        result["reason"] = "graph_already_fresh"
        return result

    graph = save_graph(root, graph_dir, build_reason="post-execution-update")
    status_after = graph_status(root, graph_dir)
    result["after"] = status_after
    result["after"]["nodes"] = len(graph["nodes"])
    result["after"]["edges"] = len(graph["edges"])
    if not status_after["dirty"]:
        result["outcome"] = "refreshed-via-update"
        result["reason"] = "relevant_changes_refreshed"
        return result

    rebuilt = save_graph(root, graph_dir, build_reason="post-execution-rebuild")
    rebuilt_status = graph_status(root, graph_dir)
    result["after"] = rebuilt_status
    result["after"]["nodes"] = len(rebuilt["nodes"])
    result["after"]["edges"] = len(rebuilt["edges"])
    if not rebuilt_status["dirty"]:
        result["outcome"] = "refreshed-via-rebuild"
        result["reason"] = "update_insufficient"
    else:
        result["outcome"] = "refresh-failed"
        result["reason"] = "graph_still_dirty_after_rebuild"
    return result


def refresh_existing_clusters(graph: dict) -> dict:
    graph["nodes"] = [dict(node) for node in graph["nodes"]]
    return apply_community_metadata(graph)


def adjacency(graph: dict) -> Dict[str, List[Tuple[str, dict]]]:
    adj: Dict[str, List[Tuple[str, dict]]] = defaultdict(list)
    for edge in graph["edges"]:
        adj[edge["source"]].append((edge["target"], edge))
        adj[edge["target"]].append((edge["source"], edge))
    return adj


def node_lookup(graph: dict) -> Dict[str, dict]:
    return {node["id"]: node for node in graph["nodes"]}


def expanded_tokens(value: str) -> List[str]:
    lowered = value.lower()
    tokens: List[str] = []
    for token in WORD_RE.findall(lowered):
        if len(token) > 1:
            tokens.append(token)
        for part in re.split(r"[-_./:]+", token):
            if len(part) > 1:
                tokens.append(part)
    return list(dict.fromkeys(tokens))


def node_kind_priority(node: dict) -> int:
    return {
        "symbol": 0,
        "file": 1,
        "heading": 2,
    }.get(node.get("kind", ""), 3)


def node_noise_penalty(node: dict, matched_terms: int) -> float:
    penalty = 0.0
    source_file = str(node.get("source_file", "")).lower()
    label = str(node.get("label", "")).lower()
    if source_file.startswith("docs/report/"):
        penalty += 2.0
    if source_file.endswith("docs-index.json"):
        penalty += 2.0
    if source_file.startswith("docs/") and node.get("kind") == "file" and matched_terms < 2:
        penalty += 2.5
    if node.get("kind") == "heading" and label in GENERIC_HEADING_LABELS and matched_terms < 2:
        penalty += 1.0
    return penalty


def score_nodes(graph: dict, query: str) -> List[Tuple[float, dict]]:
    terms = [
        term.lower()
        for term in WORD_RE.findall(query.lower())
        if len(term) > 1 and term.lower() not in STOP_WORDS
    ]
    if not terms:
        return []
    query_phrase = query.lower().strip()
    node_views = []
    term_frequencies = Counter()
    for node in graph["nodes"]:
        label = str(node.get("label", ""))
        source_file = str(node.get("source_file", ""))
        label_lower = label.lower()
        source_lower = source_file.lower()
        haystack = " ".join(str(node.get(key, "")).lower() for key in ("label", "source_file", "kind", "language"))
        label_tokens = set(expanded_tokens(label))
        source_tokens = set(expanded_tokens(source_file))
        matched = set()
        for term in terms:
            if term in label_tokens or term in label_lower or term in source_tokens or term in source_lower:
                matched.add(term)
        for term in matched:
            term_frequencies[term] += 1
        node_views.append((node, label_lower, source_lower, haystack, label_tokens, source_tokens, matched))

    scored: List[Tuple[float, dict]] = []
    for node, label_lower, source_lower, haystack, label_tokens, source_tokens, _pref_matched in node_views:
        score = 0.0
        matched_terms = set()
        source_name = Path(source_file if (source_file := str(node.get("source_file", ""))) else "").name.lower()
        source_stem = Path(source_name).stem.lower() if source_name else ""
        for term in terms:
            frequency = max(term_frequencies.get(term, 1), 1)
            weight = 1.0 + (3.0 / frequency)
            term_matched = False
            if term == label_lower:
                score += 8.0 * weight
                term_matched = True
            if term and source_name and term in {source_name, source_stem}:
                score += 7.0 * weight
                term_matched = True
            if term in label_tokens:
                score += 4.0 * weight
                term_matched = True
            elif term in label_lower:
                score += 1.5 * weight
                term_matched = True
            if term in source_tokens:
                score += 3.0 * weight
                term_matched = True
            elif term in source_lower and term not in label_lower:
                score += 1.0 * weight
                term_matched = True
            if node.get("kind") == "symbol" and term == label_lower:
                score += 2.0 * weight
            if term_matched:
                matched_terms.add(term)
        if query_phrase and query_phrase in haystack:
            score += 4.0
        score += len(matched_terms) * 3.0
        if terms and len(matched_terms) == len(terms):
            score += 4.0
        if node.get("kind") == "symbol" and matched_terms:
            score += 1.0
        elif node.get("kind") == "file" and matched_terms:
            score += 0.5
        score -= node_noise_penalty(node, len(matched_terms))
        if score > 0:
            scored.append((score, {**node, "_matched_terms": len(matched_terms)}))
    scored.sort(
        key=lambda item: (
            -item[0],
            -item[1].get("_matched_terms", 0),
            node_kind_priority(item[1]),
            item[1]["id"],
        )
    )
    return scored


def bfs_context(graph: dict, start_ids: Sequence[str], depth: int = 2, max_nodes: int = 32) -> Tuple[List[str], List[dict]]:
    adj = adjacency(graph)
    seen = set(start_ids)
    queue = deque((node_id, 0) for node_id in start_ids)
    ordered = list(start_ids)
    edges_seen: List[dict] = []
    while queue and len(ordered) < max_nodes:
        node_id, current_depth = queue.popleft()
        if current_depth >= depth:
            continue
        for neighbor, edge in adj.get(node_id, []):
            if edge not in edges_seen:
                edges_seen.append(edge)
            if neighbor in seen:
                continue
            seen.add(neighbor)
            ordered.append(neighbor)
            queue.append((neighbor, current_depth + 1))
            if len(ordered) >= max_nodes:
                break
    return ordered, edges_seen


def dfs_context(graph: dict, start_ids: Sequence[str], depth: int = 2, max_nodes: int = 32) -> Tuple[List[str], List[dict]]:
    adj = adjacency(graph)
    seen = set(start_ids)
    ordered = list(start_ids)
    edges_seen: List[dict] = []
    stack = [(node_id, 0) for node_id in reversed(start_ids)]
    while stack and len(ordered) < max_nodes:
        node_id, current_depth = stack.pop()
        if current_depth >= depth:
            continue
        for neighbor, edge in reversed(adj.get(node_id, [])):
            if edge not in edges_seen:
                edges_seen.append(edge)
            if neighbor in seen:
                continue
            seen.add(neighbor)
            ordered.append(neighbor)
            stack.append((neighbor, current_depth + 1))
            if len(ordered) >= max_nodes:
                break
    return ordered, edges_seen


def community_lookup(graph: dict) -> Dict[int, dict]:
    return {community["id"]: community for community in graph.get("communities", [])}


def top_god_nodes(graph: dict, limit: int = 10) -> List[dict]:
    adj = adjacency(graph)
    nodes = node_lookup(graph)
    ranked = sorted(
        (
            {
                "id": node_id,
                "label": node.get("label", node_id),
                "kind": node.get("kind"),
                "source_file": node.get("source_file", ""),
                "degree": len(adj.get(node_id, [])),
            }
            for node_id, node in nodes.items()
        ),
        key=lambda item: (-item["degree"], item["label"]),
    )
    return ranked[:limit]


def explain_node(graph: dict, label: str) -> str:
    matches = find_node_matches(graph, label)
    if not matches:
        return f"No matching nodes found for '{label}'."
    node = matches[0]
    neighbors = adjacency(graph).get(node["id"], [])
    community = community_lookup(graph).get(node.get("community_id"))
    lines = [
        f"Explain: {node.get('label', node['id'])}",
        f"ID: {node['id']}",
        f"Kind: {node.get('kind', '')}",
        f"Source file: {node.get('source_file', '')}",
        f"Community: {node.get('community_id', 'n/a')}",
    ]
    lines.append(
        "Why it matters: "
        + (
            f"this {node.get('kind', 'node')} is directly connected to {len(neighbors)} graph neighbors"
            if neighbors
            else "this node currently has limited graph connectivity"
        )
    )
    if community:
        lines.append(f"Community sample: {', '.join(community.get('sample_labels', []))}")
    if neighbors:
        lines.append("Graph evidence:")
        nodes = node_lookup(graph)
        for neighbor_id, edge in neighbors[:6]:
            lines.append(
                f"- {nodes[neighbor_id].get('label', neighbor_id)} [{edge['relation']}]"
            )
    return "\n".join(lines)


def format_path(graph: dict, path: Sequence[str]) -> str:
    nodes = node_lookup(graph)
    adj = adjacency(graph)
    if not path:
        return "No path found."
    if len(path) == 1:
        node = nodes[path[0]]
        return f"Shortest path (0 hops):\n- {node.get('label', path[0])} [{path[0]}]"
    lines = [f"Shortest path ({len(path) - 1} hops):"]
    for index, node_id in enumerate(path):
        node = nodes[node_id]
        lines.append(f"- {node.get('label', node_id)} [{node_id}]")
        if index == len(path) - 1:
            continue
        next_id = path[index + 1]
        relation = "related_to"
        for neighbor_id, edge in adj.get(node_id, []):
            if neighbor_id == next_id:
                relation = edge["relation"]
                break
        lines.append(f"  --{relation}-->")
    lines.append("Path meaning:")
    for index in range(len(path) - 1):
        current = nodes[path[index]].get("label", path[index])
        nxt = nodes[path[index + 1]].get("label", path[index + 1])
        relation = "related_to"
        for neighbor_id, edge in adj.get(path[index], []):
            if neighbor_id == path[index + 1]:
                relation = edge["relation"]
                break
        lines.append(f"- {current} {relation} {nxt}")
    return "\n".join(lines)


def render_context(graph: dict, query: str, depth: int = 2, token_budget: int = 1400, mode: str = "bfs") -> str:
    scored = score_nodes(graph, query)
    if not scored:
        return "No matching graph nodes found."
    seeds = [node["id"] for _, node in scored[:4]]
    nodes_by_id = node_lookup(graph)
    traversal = dfs_context if mode == "dfs" else bfs_context
    ordered, edges_seen = traversal(graph, seeds, depth=depth)
    included_ids = set(ordered)
    lines = [f"Graph query context ({mode.upper()}):", "Top matches:"]
    for score, node in scored[: min(5, len(scored))]:
        lines.append(
            f"MATCH {node.get('label')} [score={score:.1f} id={node['id']} kind={node.get('kind')} file={node.get('source_file', '')}]"
        )
    lines.append("Supporting context:")
    for node_id in ordered:
        node = nodes_by_id[node_id]
        lines.append(
            f"NODE {node.get('label')} [id={node_id} kind={node.get('kind')} file={node.get('source_file', '')} lang={node.get('language', '')} community={node.get('community_id', 'n/a')}]"
        )
    visible_edges = [
        edge
        for edge in edges_seen
        if edge["source"] in included_ids and edge["target"] in included_ids
    ]
    for edge in visible_edges[: max(12, len(ordered) * 2)]:
        lines.append(f"EDGE {edge['source']} --{edge['relation']}--> {edge['target']}")
    output = "\n".join(lines)
    char_budget = token_budget * 4
    if len(output) > char_budget:
        output = output[:char_budget] + f"\n... (truncated to ~{token_budget} token budget)"
    return output


def find_node_matches(graph: dict, label: str) -> List[dict]:
    term = label.lower()
    matches = []
    for node in graph["nodes"]:
        if (
            term in str(node.get("label", "")).lower()
            or term in str(node.get("source_file", "")).lower()
            or term == node["id"].lower()
        ):
            matches.append(node)
    matches.sort(key=lambda node: (node.get("kind", ""), node.get("label", "")))
    return matches


def get_community_nodes(graph: dict, community_id: int) -> List[dict]:
    return [
        node
        for node in graph["nodes"]
        if node.get("community_id") == community_id
    ]


def shortest_path(graph: dict, source_term: str, target_term: str, max_hops: int = 8) -> List[str]:
    matches_a = find_node_matches(graph, source_term)
    matches_b = {node["id"] for node in find_node_matches(graph, target_term)}
    if not matches_a or not matches_b:
        return []
    adj = adjacency(graph)
    queue = deque([(matches_a[0]["id"], [matches_a[0]["id"]])])
    seen = {matches_a[0]["id"]}
    while queue:
        node_id, path = queue.popleft()
        if len(path) - 1 > max_hops:
            continue
        if node_id in matches_b:
            return path
        for neighbor, _ in adj.get(node_id, []):
            if neighbor in seen:
                continue
            seen.add(neighbor)
            queue.append((neighbor, path + [neighbor]))
    return []


def print_json(data: dict) -> None:
    print(json.dumps(data, indent=2))


def cmd_build(args: argparse.Namespace) -> int:
    root = Path(args.root).resolve()
    files = collect_files(root)
    graph = save_graph(root, Path(args.graph_dir), files=files, build_reason="build")
    output_path = (
        (root / args.graph_dir / "graph.json").resolve()
        if not Path(args.graph_dir).is_absolute()
        else Path(args.graph_dir) / "graph.json"
    )
    print(f"Built repo graph for {root}")
    print(f"Files: {graph['total_files']} | code: {graph['counts'].get('code', 0)} | docs: {graph['counts'].get('docs', 0)}")
    print(f"Nodes: {len(graph['nodes'])} | Edges: {len(graph['edges'])} | Communities: {len(graph.get('communities', []))}")
    print(f"Graph: {output_path}")
    return 0


def cmd_update(args: argparse.Namespace) -> int:
    root = Path(args.root).resolve()
    graph_dir = Path(args.graph_dir)
    status = graph_status(root, graph_dir)
    if not status["dirty"]:
        graph = load_graph(root, graph_dir)
        print(f"Repo graph already fresh for {root}")
        print(f"Files: {graph['total_files']} | code: {graph['counts'].get('code', 0)} | docs: {graph['counts'].get('docs', 0)}")
        print(f"Nodes: {len(graph['nodes'])} | Edges: {len(graph['edges'])} | Communities: {len(graph.get('communities', []))}")
        return 0

    files = collect_files(root)
    graph = save_graph(root, graph_dir, files=files, build_reason="update")
    print(f"Updated repo graph for {root}")
    print(
        "Changed inputs: "
        f"+{status['added_count']} / ~{status['changed_count']} / -{status['removed_count']}"
    )
    print(f"Files: {graph['total_files']} | code: {graph['counts'].get('code', 0)} | docs: {graph['counts'].get('docs', 0)}")
    print(f"Nodes: {len(graph['nodes'])} | Edges: {len(graph['edges'])} | Communities: {len(graph.get('communities', []))}")
    return 0


def cmd_cluster_only(args: argparse.Namespace) -> int:
    root = Path(args.root).resolve()
    graph_dir = Path(args.graph_dir)
    out_dir, graph_path, manifest_path = graph_paths(root, graph_dir)
    graph = refresh_existing_clusters(load_graph(root, graph_dir))
    graph["build_reason"] = "cluster-only"
    graph["built_at"] = utc_now()
    graph["graph_dir"] = str(out_dir)
    graph_path.write_text(json.dumps(graph, indent=2) + "\n", encoding="utf-8")
    manifest = load_manifest(root, graph_dir)
    if manifest:
        manifest["built_at"] = graph["built_at"]
        manifest["build_reason"] = "cluster-only"
        manifest["communities"] = len(graph.get("communities", []))
        manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(f"Recomputed repo-graph communities for {root}")
    print(f"Communities: {len(graph.get('communities', []))}")
    return 0


def cmd_stats(args: argparse.Namespace) -> int:
    root = Path(args.root).resolve()
    graph = ensure_graph(root, Path(args.graph_dir), refresh=args.ensure_fresh)
    stats = {
        "root": graph["root"],
        "total_files": graph["total_files"],
        "counts": graph["counts"],
        "nodes": len(graph["nodes"]),
        "edges": len(graph["edges"]),
        "communities": len(graph.get("communities", [])),
        "built_at": graph.get("built_at"),
    }
    if args.json:
        print_json(stats)
    else:
        print(f"Root: {stats['root']}")
        print(f"Files: {stats['total_files']} | code: {stats['counts'].get('code', 0)} | docs: {stats['counts'].get('docs', 0)}")
        print(f"Nodes: {stats['nodes']} | Edges: {stats['edges']} | Communities: {stats['communities']}")
        if stats["built_at"]:
            print(f"Built at: {stats['built_at']}")
    return 0


def cmd_status(args: argparse.Namespace) -> int:
    root = Path(args.root).resolve()
    graph_dir = Path(args.graph_dir)
    manifest = load_manifest(root, graph_dir)
    files = collect_files(root)
    snapshot = file_snapshot(root, files)
    diff = manifest_diff(manifest, snapshot)
    graph_exists = graph_paths(root, graph_dir)[1].is_file()
    payload = {
        "root": str(root),
        "graph_exists": graph_exists,
        "stale": diff["stale"] or not graph_exists,
        "last_build": manifest.get("built_at", "not built yet") if manifest else "not built yet",
        "build_reason": manifest.get("build_reason", "unknown") if manifest else "unknown",
        "last_graph": {
            "files": manifest.get("total_files", 0) if manifest else 0,
            "nodes": manifest.get("nodes", 0) if manifest else 0,
            "edges": manifest.get("edges", 0) if manifest else 0,
            "communities": manifest.get("communities", 0) if manifest else 0,
        },
        "input_diff": {
            "added": len(diff["added"]),
            "changed": len(diff["changed"]),
            "removed": len(diff["removed"]),
        },
    }
    if args.json:
        print_json(payload)
        return 0
    print(f"Root: {payload['root']}")
    print(f"Graph exists: {'yes' if payload['graph_exists'] else 'no'}")
    print(f"Stale: {'yes' if payload['stale'] else 'no'}")
    if manifest:
        print(f"Last build: {payload['last_build']} [{payload['build_reason']}]")
        print(
            f"Last graph: files={payload['last_graph']['files']} "
            f"nodes={payload['last_graph']['nodes']} edges={payload['last_graph']['edges']} "
            f"communities={payload['last_graph']['communities']}"
        )
    else:
        print("Last build: not built yet")
    print(
        "Input diff: "
        f"+{payload['input_diff']['added']} / ~{payload['input_diff']['changed']} / -{payload['input_diff']['removed']}"
    )
    return 0


def cmd_query(args: argparse.Namespace) -> int:
    root = Path(args.root).resolve()
    graph = ensure_graph(root, Path(args.graph_dir), refresh=args.ensure_fresh)
    mode = "dfs" if args.dfs else "bfs"
    print(render_context(graph, args.question, depth=args.depth, token_budget=args.token_budget, mode=mode))
    return 0


def cmd_path(args: argparse.Namespace) -> int:
    root = Path(args.root).resolve()
    graph = ensure_graph(root, Path(args.graph_dir), refresh=args.ensure_fresh)
    path = shortest_path(graph, args.source, args.target, max_hops=args.max_hops)
    if not path:
        print("No path found.")
        return 1
    print(format_path(graph, path))
    return 0


def cmd_explain(args: argparse.Namespace) -> int:
    root = Path(args.root).resolve()
    graph = ensure_graph(root, Path(args.graph_dir), refresh=args.ensure_fresh)
    print(explain_node(graph, args.label))
    return 0


def cmd_community(args: argparse.Namespace) -> int:
    root = Path(args.root).resolve()
    graph = ensure_graph(root, Path(args.graph_dir), refresh=args.ensure_fresh)
    nodes = get_community_nodes(graph, args.community_id)
    if not nodes:
        print(f"Community {args.community_id} not found.")
        return 1
    summary = community_lookup(graph).get(args.community_id, {})
    print(
        f"Community {args.community_id} ({len(nodes)} nodes)"
        + (
            f" | sample: {', '.join(summary.get('sample_labels', []))}"
            if summary.get("sample_labels")
            else ""
        )
        + (
            f" | cohesion: {summary.get('cohesion')}"
            if summary.get("cohesion") is not None
            else ""
        )
    )
    for node in nodes[: args.limit]:
        print(f"- {node.get('label', node['id'])} [{node['id']}]")
    return 0


def cmd_god_nodes(args: argparse.Namespace) -> int:
    root = Path(args.root).resolve()
    graph = ensure_graph(root, Path(args.graph_dir), refresh=args.ensure_fresh)
    for node in top_god_nodes(graph, limit=args.limit):
        print(
            f"{node['label']} [{node['id']}] degree={node['degree']} "
            f"kind={node.get('kind', '')} file={node.get('source_file', '')}"
        )
    return 0


def cmd_node(args: argparse.Namespace) -> int:
    root = Path(args.root).resolve()
    graph = ensure_graph(root, Path(args.graph_dir), refresh=args.ensure_fresh)
    matches = find_node_matches(graph, args.label)
    if not matches:
        print("No matching nodes found.")
        return 1
    if args.json:
        print_json({"matches": matches[: args.limit]})
    else:
        for node in matches[: args.limit]:
            print(f"{node['id']} | {node.get('label')} | kind={node.get('kind')} | file={node.get('source_file', '')}")
    return 0


def cmd_neighbors(args: argparse.Namespace) -> int:
    root = Path(args.root).resolve()
    graph = ensure_graph(root, Path(args.graph_dir), refresh=args.ensure_fresh)
    matches = find_node_matches(graph, args.label)
    if not matches:
        print("No matching nodes found.")
        return 1
    node = matches[0]
    adj = adjacency(graph)
    nodes = node_lookup(graph)
    for neighbor, edge in adj.get(node["id"], []):
        target = nodes[neighbor]
        print(f"{edge['relation']}: {target.get('label')} [{neighbor}]")
    return 0


def cmd_post_execution_refresh(args: argparse.Namespace) -> int:
    root = Path(args.root).resolve()
    graph_dir = Path(args.graph_dir)
    changed_paths = list(args.paths)
    if args.paths_file:
        changed_paths.extend(
            line.strip()
            for line in Path(args.paths_file).read_text(encoding="utf-8").splitlines()
            if line.strip()
        )
    if args.git_diff:
        changed_paths.extend(changed_paths_from_git_diff(root))
    result = post_execution_refresh(root, graph_dir, changed_paths)
    if args.json:
        print_json(result)
        return 0 if result["outcome"] != "refresh-failed" else 1

    print(f"Repo-graph post-execution refresh: {result['outcome']}")
    if result["reason"]:
        print(f"Reason: {result['reason']}")
    if result["changed_paths"]:
        print(f"Changed paths: {', '.join(result['changed_paths'])}")
    if result["relevant_paths"]:
        print(f"Relevant paths: {', '.join(result['relevant_paths'])}")
    after = result.get("after", {})
    if result["outcome"].startswith("refreshed"):
        print(
            "After refresh: "
            f"dirty={'yes' if after.get('dirty') else 'no'} "
            f"files={after.get('last_graph_files', 0)} "
            f"nodes={after.get('nodes', after.get('last_graph_nodes', 0))} "
            f"edges={after.get('edges', after.get('last_graph_edges', 0))}"
        )
    return 0 if result["outcome"] != "refresh-failed" else 1


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Build and query a bounded repo graph for OMX.")
    parser.add_argument("--root", default=".", help="Repo root to index (default: current directory)")
    parser.add_argument(
        "--graph-dir",
        default=DEFAULT_GRAPH_DIR,
        help=f"Graph output directory (default: {DEFAULT_GRAPH_DIR})",
    )
    sub = parser.add_subparsers(dest="command", required=True)

    build_p = sub.add_parser("build", help="Build or rebuild the repo graph")
    build_p.set_defaults(func=cmd_build)

    update_p = sub.add_parser("update", help="Refresh the repo graph only when tracked files changed")
    update_p.set_defaults(func=cmd_update)

    cluster_p = sub.add_parser("cluster-only", help="Recompute communities from the current graph without rescanning files")
    cluster_p.set_defaults(func=cmd_cluster_only)

    stats_p = sub.add_parser("stats", help="Show graph stats")
    stats_p.add_argument("--json", action="store_true")
    stats_p.add_argument("--ensure-fresh", action="store_true")
    stats_p.set_defaults(func=cmd_stats)

    status_p = sub.add_parser("status", help="Show freshness/build status without forcing a rebuild")
    status_p.add_argument("--json", action="store_true")
    status_p.set_defaults(func=cmd_status)

    query_p = sub.add_parser("query", help="Query the repo graph")
    query_p.add_argument("question")
    query_p.add_argument("--depth", type=int, default=2)
    query_p.add_argument("--token-budget", type=int, default=1400)
    query_p.add_argument("--dfs", action="store_true")
    query_p.add_argument("--ensure-fresh", action="store_true")
    query_p.set_defaults(func=cmd_query)

    path_p = sub.add_parser("path", help="Find shortest path between two nodes/terms")
    path_p.add_argument("source")
    path_p.add_argument("target")
    path_p.add_argument("--max-hops", type=int, default=8)
    path_p.add_argument("--ensure-fresh", action="store_true")
    path_p.set_defaults(func=cmd_path)

    explain_p = sub.add_parser("explain", help="Explain one node in plain language")
    explain_p.add_argument("label")
    explain_p.add_argument("--ensure-fresh", action="store_true")
    explain_p.set_defaults(func=cmd_explain)

    node_p = sub.add_parser("node", help="Find matching nodes")
    node_p.add_argument("label")
    node_p.add_argument("--limit", type=int, default=10)
    node_p.add_argument("--json", action="store_true")
    node_p.add_argument("--ensure-fresh", action="store_true")
    node_p.set_defaults(func=cmd_node)

    neigh_p = sub.add_parser("neighbors", help="Show neighbors for a node/term")
    neigh_p.add_argument("label")
    neigh_p.add_argument("--ensure-fresh", action="store_true")
    neigh_p.set_defaults(func=cmd_neighbors)

    community_p = sub.add_parser("community", help="Show nodes for one community")
    community_p.add_argument("community_id", type=int)
    community_p.add_argument("--limit", type=int, default=25)
    community_p.add_argument("--ensure-fresh", action="store_true")
    community_p.set_defaults(func=cmd_community)

    god_nodes_p = sub.add_parser("god-nodes", help="Show the most connected nodes")
    god_nodes_p.add_argument("--limit", type=int, default=10)
    god_nodes_p.add_argument("--ensure-fresh", action="store_true")
    god_nodes_p.set_defaults(func=cmd_god_nodes)

    post_p = sub.add_parser("post-execution-refresh", help="Refresh repo-graph after eligible OMX execution changes")
    post_p.add_argument("paths", nargs="*")
    post_p.add_argument("--paths-file")
    post_p.add_argument("--git-diff", action="store_true")
    post_p.add_argument("--json", action="store_true")
    post_p.set_defaults(func=cmd_post_execution_refresh)

    return parser


def main(argv: Optional[Sequence[str]] = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
