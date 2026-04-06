#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import venv
from pathlib import Path

import repo_graph


def lifecycle_state(*, python_exists: bool, import_ok: bool, graph_exists: bool, graph_dirty: bool) -> str:
    if not python_exists:
        return "not_bootstrapped"
    if not import_ok:
        return "bootstrapped_unhealthy"
    if graph_dirty or not graph_exists:
        return "refresh_needed"
    return "bootstrapped_healthy"


def mcp_import_ok(python_bin: Path) -> bool:
    if not python_bin.exists():
        return False
    result = subprocess.run(
        [str(python_bin), "-c", "import mcp"],
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    return result.returncode == 0


def repo_graph_status(repo_root: Path) -> dict:
    graph_dir = Path(os.environ.get("REPO_GRAPH_DIR", str(repo_root / ".omx/graphs/repo-graph")))
    graph_dir = graph_dir if graph_dir.is_absolute() else repo_root / graph_dir
    return repo_graph.graph_status(repo_root, graph_dir)


def status_payload(repo_root: Path, tool_root: Path) -> dict:
    venv_dir = tool_root / "venv"
    python_bin = venv_dir / "bin" / "python"
    graph = repo_graph_status(repo_root)
    import_ok = mcp_import_ok(python_bin)
    state = lifecycle_state(
        python_exists=python_bin.exists(),
        import_ok=import_ok,
        graph_exists=graph["graph_exists"],
        graph_dirty=graph["dirty"],
    )
    return {
        "state": state,
        "tool_root": str(tool_root),
        "venv_python": str(python_bin),
        "python_exists": python_bin.exists(),
        "mcp_import_ok": import_ok,
        "graph": graph,
    }


def ensure_bootstrap(repo_root: Path, tool_root: Path, python_bin_name: str) -> dict:
    tool_root.mkdir(parents=True, exist_ok=True)
    venv_dir = tool_root / "venv"
    venv_python = venv_dir / "bin" / "python"

    if not venv_python.exists():
        venv.EnvBuilder(with_pip=True).create(venv_dir)

    before = status_payload(repo_root, tool_root)
    if before["mcp_import_ok"]:
        return {"outcome": "already_bootstrapped", "status": before}

    pip_result = subprocess.run(
        [str(venv_python), "-m", "pip", "install", "--quiet", "mcp"],
        check=False,
        capture_output=True,
        text=True,
    )
    after = status_payload(repo_root, tool_root)
    return {
        "outcome": "ensured" if pip_result.returncode == 0 and after["mcp_import_ok"] else "ensure_failed",
        "status": after,
        "pip_stdout": pip_result.stdout,
        "pip_stderr": pip_result.stderr,
        "requested_python": python_bin_name,
    }


def run_server(repo_root: Path, tool_root: Path, python_bin_name: str) -> int:
    ensured = ensure_bootstrap(repo_root, tool_root, python_bin_name)
    if ensured["outcome"] == "ensure_failed":
        print("repo-graph MCP ensure failed", file=sys.stderr)
        if ensured.get("pip_stderr"):
            print(ensured["pip_stderr"], file=sys.stderr)
        return 1
    subprocess.run([str(repo_root / "scripts" / "repo-graph.sh"), "update"], check=True, stdout=subprocess.DEVNULL)
    venv_python = tool_root / "venv" / "bin" / "python"
    os.execv(str(venv_python), [str(venv_python), str(repo_root / "scripts" / "repo_graph_mcp.py")])
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Repo-graph MCP bootstrap helper")
    parser.add_argument("--repo-root", default=None)
    parser.add_argument("--tool-root", default=None)
    parser.add_argument("--python-bin", default=os.environ.get("PYTHON_BIN", "python3"))
    sub = parser.add_subparsers(dest="command", required=True)

    status = sub.add_parser("status")
    status.add_argument("--json", action="store_true")

    ensure = sub.add_parser("ensure")
    ensure.add_argument("--json", action="store_true")

    sub.add_parser("run")
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)

    repo_root = Path(args.repo_root or Path(__file__).resolve().parents[1]).resolve()
    tool_root = Path(args.tool_root or repo_root / ".omx" / "tooling" / "repo-graph-mcp").resolve()

    if args.command == "status":
        payload = status_payload(repo_root, tool_root)
        if args.json:
            print(json.dumps(payload, indent=2))
        else:
            print(f"State: {payload['state']}")
            print(f"Tool root: {payload['tool_root']}")
            print(f"Python: {payload['venv_python']}")
            print(f"Graph dirty: {'yes' if payload['graph']['dirty'] else 'no'}")
        return 0

    if args.command == "ensure":
        payload = ensure_bootstrap(repo_root, tool_root, args.python_bin)
        if args.json:
            print(json.dumps(payload, indent=2))
        else:
            print(f"Outcome: {payload['outcome']}")
            print(f"State: {payload['status']['state']}")
            print(f"Python: {payload['status']['venv_python']}")
        return 0 if payload["outcome"] != "ensure_failed" else 1

    if args.command == "run":
        return run_server(repo_root, tool_root, args.python_bin)

    parser.error(f"Unknown command: {args.command}")
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
