#!/usr/bin/env python3
from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import repo_graph_mcp_bootstrap as bootstrap


class RepoGraphMcpBootstrapTests(unittest.TestCase):
    def test_lifecycle_state_matrix(self) -> None:
        self.assertEqual(
            bootstrap.lifecycle_state(python_exists=False, import_ok=False, graph_exists=False, graph_dirty=True),
            "not_bootstrapped",
        )
        self.assertEqual(
            bootstrap.lifecycle_state(python_exists=True, import_ok=False, graph_exists=True, graph_dirty=False),
            "bootstrapped_unhealthy",
        )
        self.assertEqual(
            bootstrap.lifecycle_state(python_exists=True, import_ok=True, graph_exists=True, graph_dirty=True),
            "refresh_needed",
        )
        self.assertEqual(
            bootstrap.lifecycle_state(python_exists=True, import_ok=True, graph_exists=True, graph_dirty=False),
            "bootstrapped_healthy",
        )

    def test_status_payload_reports_not_bootstrapped_for_missing_venv(self) -> None:
        root = Path(tempfile.mkdtemp(prefix="repo-graph-mcp-test-"))
        (root / "README.md").write_text("# Demo\n", encoding="utf-8")
        tool_root = root / ".omx" / "tooling" / "repo-graph-mcp"

        payload = bootstrap.status_payload(root, tool_root)

        self.assertEqual(payload["state"], "not_bootstrapped")
        self.assertFalse(payload["python_exists"])


if __name__ == "__main__":
    unittest.main()
