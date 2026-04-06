#!/usr/bin/env python3
from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import repo_graph


class RepoGraphPhaseTests(unittest.TestCase):
    def make_repo(self) -> Path:
        root = Path(tempfile.mkdtemp(prefix="repo-graph-test-"))
        (root / "src").mkdir()
        (root / "src" / "a.py").write_text("def alpha():\n    return 1\n", encoding="utf-8")
        (root / "src" / "b.py").write_text(
            "from .a import alpha\n\n\ndef beta():\n    return alpha()\n",
            encoding="utf-8",
        )
        (root / "README.md").write_text("# Demo\n\nSee [A](src/a.py)\n", encoding="utf-8")
        (root / ".output").mkdir()
        (root / ".output" / "noise.js").write_text("export const noise = true;\n", encoding="utf-8")
        return root

    def test_save_graph_writes_manifest_and_communities(self) -> None:
        root = self.make_repo()
        graph_dir = root / ".omx" / "graphs" / "repo-graph"

        graph = repo_graph.save_graph(root, graph_dir, build_reason="build")
        manifest = repo_graph.load_manifest(root, graph_dir)

        self.assertEqual(graph["graph_version"], repo_graph.GRAPH_VERSION)
        self.assertTrue((graph_dir / "graph.json").is_file())
        self.assertTrue((graph_dir / "manifest.json").is_file())
        self.assertGreaterEqual(len(graph.get("communities", [])), 1)
        self.assertEqual(manifest["build_reason"], "build")
        self.assertEqual(manifest["communities"], len(graph.get("communities", [])))
        self.assertTrue(any("community_id" in node for node in graph["nodes"]))

    def test_manifest_diff_detects_changed_inputs(self) -> None:
        root = self.make_repo()
        graph_dir = root / ".omx" / "graphs" / "repo-graph"
        files = repo_graph.collect_files(root)
        repo_graph.save_graph(root, graph_dir, files=files, build_reason="build")

        manifest = repo_graph.load_manifest(root, graph_dir)
        before = repo_graph.manifest_diff(manifest, repo_graph.file_snapshot(root, files))
        self.assertFalse(before["stale"])

        target = root / "src" / "a.py"
        target.write_text("def alpha():\n    return 2\n", encoding="utf-8")

        after = repo_graph.manifest_diff(
            manifest,
            repo_graph.file_snapshot(root, repo_graph.collect_files(root)),
        )
        self.assertTrue(after["stale"])
        self.assertIn("src/a.py", after["changed"])

    def test_collect_files_excludes_output_artifacts(self) -> None:
        root = self.make_repo()
        files = [path.relative_to(root).as_posix() for path in repo_graph.collect_files(root)]
        self.assertIn("src/a.py", files)
        self.assertNotIn(".output/noise.js", files)

    def test_post_execution_refresh_skips_when_no_relevant_changes(self) -> None:
        root = self.make_repo()
        graph_dir = root / ".omx" / "graphs" / "repo-graph"
        repo_graph.save_graph(root, graph_dir, build_reason="build")

        result = repo_graph.post_execution_refresh(root, graph_dir, ["notes/todo.txt"])

        self.assertEqual(result["outcome"], "skipped")
        self.assertEqual(result["reason"], "no_repo_graph_relevant_changes")

    def test_post_execution_refresh_uses_update_for_relevant_changes(self) -> None:
        root = self.make_repo()
        graph_dir = root / ".omx" / "graphs" / "repo-graph"
        repo_graph.save_graph(root, graph_dir, build_reason="build")

        target = root / "src" / "a.py"
        target.write_text("def alpha():\n    return 3\n", encoding="utf-8")
        result = repo_graph.post_execution_refresh(root, graph_dir, ["src/a.py"])

        self.assertEqual(result["outcome"], "refreshed-via-update")
        self.assertIn("src/a.py", result["relevant_paths"])
        self.assertFalse(result["after"]["dirty"])

    def test_query_path_and_explain_surfaces_have_phase2_signal(self) -> None:
        root = self.make_repo()
        graph_dir = root / ".omx" / "graphs" / "repo-graph"
        graph = repo_graph.save_graph(root, graph_dir, build_reason="build")

        bfs = repo_graph.render_context(graph, "alpha beta", depth=2, token_budget=500, mode="bfs")
        dfs = repo_graph.render_context(graph, "alpha beta", depth=2, token_budget=500, mode="dfs")
        path = repo_graph.shortest_path(graph, "beta", "alpha", max_hops=4)
        explained = repo_graph.explain_node(graph, "alpha")
        god_nodes = repo_graph.top_god_nodes(graph, limit=3)

        self.assertIn("Graph query context (BFS)", bfs)
        self.assertIn("Graph query context (DFS)", dfs)
        self.assertIn("Top matches:", bfs)
        self.assertIn("MATCH", bfs)
        self.assertGreaterEqual(len(path), 2)
        self.assertIn("Explain:", explained)
        self.assertIn("Why it matters:", explained)
        self.assertIn("Graph evidence:", explained)
        self.assertTrue(god_nodes)
        self.assertIn("alpha", repo_graph.format_path(graph, path).lower())
        self.assertIn("Path meaning:", repo_graph.format_path(graph, path))

    def test_score_nodes_prefers_exact_anchor_matches_over_generic_headings(self) -> None:
        graph = {
            "nodes": [
                {"id": "file:AGENTS.md", "label": "AGENTS.md", "kind": "file", "source_file": "AGENTS.md", "language": "docs"},
                {"id": "symbol:project-ledger", "label": "project-ledger", "kind": "symbol", "source_file": "scripts/project-ledger.mjs", "language": "js"},
                {"id": "heading:routing", "label": "Routing Intent", "kind": "heading", "source_file": "docs/spec.md", "language": "docs"},
            ],
            "edges": [],
        }
        ranked = [node["id"] for _score, node in repo_graph.score_nodes(graph, "project-ledger AGENTS routing connect")]
        self.assertCountEqual(ranked[:2], ["symbol:project-ledger", "file:AGENTS.md"])
        self.assertEqual(ranked[-1], "heading:routing")

    def test_apply_community_metadata_splits_oversized_component_and_adds_cohesion(self) -> None:
        nodes = []
        edges = []
        for index in range(12):
            nodes.append(
                {
                    "id": f"n{index}",
                    "label": f"Node {index}",
                    "kind": "symbol",
                    "source_file": f"src/{index}.py",
                    "language": "python",
                }
            )
        clique_a = [f"n{i}" for i in range(6)]
        clique_b = [f"n{i}" for i in range(6, 12)]
        for group in (clique_a, clique_b):
            for i, source in enumerate(group):
                for target in group[i + 1 :]:
                    edges.append({"source": source, "target": target, "relation": "links"})
        edges.append({"source": "n5", "target": "n6", "relation": "bridge"})
        graph = {"nodes": nodes, "edges": edges}

        updated = repo_graph.apply_community_metadata(graph)

        self.assertGreaterEqual(len(updated["communities"]), 2)
        self.assertTrue(all("cohesion" in community for community in updated["communities"]))
        self.assertTrue(all("community_id" in node for node in updated["nodes"]))


if __name__ == "__main__":
    unittest.main()
