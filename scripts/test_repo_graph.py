#!/usr/bin/env python3
from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parent
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

import repo_graph


class RepoGraphManifestTests(unittest.TestCase):
    def test_manifest_diff_tracks_changed_and_removed_files(self) -> None:
        with tempfile.TemporaryDirectory() as tmp_dir:
            root = Path(tmp_dir)
            graph_dir = root / '.omx' / 'graphs' / 'repo-graph'
            (root / 'src').mkdir(parents=True)
            (root / 'docs').mkdir(parents=True)
            code_file = root / 'src' / 'sample.py'
            doc_file = root / 'docs' / 'guide.md'
            code_file.write_text('def alpha():\n    return 1\n', encoding='utf-8')
            doc_file.write_text('# Guide\n\nSee alpha.\n', encoding='utf-8')

            repo_graph.save_graph(root, graph_dir)
            clean = repo_graph.graph_status(root, graph_dir)
            self.assertFalse(clean['dirty'])
            self.assertEqual(clean['changed_count'], 0)
            self.assertEqual(clean['removed_count'], 0)

            code_file.write_text('def alpha():\n    return 2\n', encoding='utf-8')
            dirty = repo_graph.graph_status(root, graph_dir)
            self.assertTrue(dirty['dirty'])
            self.assertIn('src/sample.py', dirty['changed_sample'])

            doc_file.unlink()
            removed = repo_graph.graph_status(root, graph_dir)
            self.assertTrue(removed['dirty'])
            self.assertIn('docs/guide.md', removed['removed_sample'])


if __name__ == '__main__':
    unittest.main()
