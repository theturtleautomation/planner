#!/usr/bin/env python3
"""Optional MCP server for repo-graph queries."""
from __future__ import annotations

import asyncio
import json
from pathlib import Path

from repo_graph import (
    DEFAULT_GRAPH_DIR,
    adjacency,
    community_lookup,
    ensure_graph,
    explain_node,
    find_node_matches,
    node_lookup,
    render_context,
    shortest_path,
    top_god_nodes,
    get_community_nodes,
    format_path,
)


def load_dependencies():
    try:
        from mcp import types
        from mcp.server import Server
        from mcp.server.stdio import stdio_server
    except ImportError as exc:  # pragma: no cover - runtime dependency
        raise SystemExit(
            "mcp Python package is not installed. Run scripts/repo-graph-mcp.sh or install mcp into the repo-graph tooling venv."
        ) from exc
    return Server, stdio_server, types


def start_server(root: Path, graph_dir: Path) -> None:
    Server, stdio_server, types = load_dependencies()
    graph = ensure_graph(root, graph_dir, refresh=True)
    adj = adjacency(graph)
    nodes = node_lookup(graph)
    server = Server("planner-repo-graph")

    @server.list_tools()
    async def list_tools() -> list[types.Tool]:
        return [
            types.Tool(
                name="query_graph",
                description="Search the repo graph and return bounded context.",
                inputSchema={
                    "type": "object",
                    "properties": {
                        "question": {"type": "string"},
                        "mode": {"type": "string", "enum": ["bfs", "dfs"], "default": "bfs"},
                        "depth": {"type": "integer", "default": 2},
                        "token_budget": {"type": "integer", "default": 1400},
                    },
                    "required": ["question"],
                },
            ),
            types.Tool(
                name="graph_stats",
                description="Return repo graph summary statistics.",
                inputSchema={"type": "object", "properties": {}},
            ),
            types.Tool(
                name="get_node",
                description="Find matching nodes by label or path fragment.",
                inputSchema={
                    "type": "object",
                    "properties": {"label": {"type": "string"}},
                    "required": ["label"],
                },
            ),
            types.Tool(
                name="get_neighbors",
                description="Get graph neighbors for the first matching node.",
                inputSchema={
                    "type": "object",
                    "properties": {"label": {"type": "string"}},
                    "required": ["label"],
                },
            ),
            types.Tool(
                name="get_community",
                description="Return nodes in one repo-graph community.",
                inputSchema={
                    "type": "object",
                    "properties": {
                        "community_id": {"type": "integer"},
                        "limit": {"type": "integer", "default": 25},
                    },
                    "required": ["community_id"],
                },
            ),
            types.Tool(
                name="god_nodes",
                description="Return the most connected repo-graph nodes.",
                inputSchema={
                    "type": "object",
                    "properties": {"limit": {"type": "integer", "default": 10}},
                },
            ),
            types.Tool(
                name="explain_node",
                description="Explain a matched node in plain language using its graph context.",
                inputSchema={
                    "type": "object",
                    "properties": {"label": {"type": "string"}},
                    "required": ["label"],
                },
            ),
            types.Tool(
                name="shortest_path",
                description="Find the shortest graph path between two terms.",
                inputSchema={
                    "type": "object",
                    "properties": {
                        "source": {"type": "string"},
                        "target": {"type": "string"},
                        "max_hops": {"type": "integer", "default": 8},
                    },
                    "required": ["source", "target"],
                },
            ),
        ]

    @server.call_tool()
    async def call_tool(name: str, arguments: dict):
        if name == "query_graph":
            text = render_context(
                graph,
                arguments["question"],
                mode=str(arguments.get("mode", "bfs")),
                depth=int(arguments.get("depth", 2)),
                token_budget=int(arguments.get("token_budget", 1400)),
            )
            return [types.TextContent(type="text", text=text)]
        if name == "graph_stats":
            payload = {
                "root": graph["root"],
                "total_files": graph["total_files"],
                "counts": graph["counts"],
                "nodes": len(graph["nodes"]),
                "edges": len(graph["edges"]),
                "communities": len(graph.get("communities", [])),
            }
            return [types.TextContent(type="text", text=json.dumps(payload, indent=2))]
        if name == "get_node":
            matches = find_node_matches(graph, arguments["label"])[:10]
            return [types.TextContent(type="text", text=json.dumps(matches, indent=2))]
        if name == "get_neighbors":
            matches = find_node_matches(graph, arguments["label"])
            if not matches:
                return [types.TextContent(type="text", text="No matching nodes found.")]
            node = matches[0]
            payload = [
                {
                    "relation": edge["relation"],
                    "target_id": neighbor,
                    "target_label": nodes[neighbor].get("label"),
                }
                for neighbor, edge in adj.get(node["id"], [])
            ]
            return [types.TextContent(type="text", text=json.dumps(payload, indent=2))]
        if name == "get_community":
            community_id = int(arguments["community_id"])
            summary = community_lookup(graph).get(community_id, {})
            nodes_in_community = get_community_nodes(graph, community_id)[: int(arguments.get("limit", 25))]
            payload = {
                "community_id": community_id,
                "sample_labels": summary.get("sample_labels", []),
                "cohesion": summary.get("cohesion"),
                "nodes": nodes_in_community,
            }
            return [types.TextContent(type="text", text=json.dumps(payload, indent=2))]
        if name == "god_nodes":
            payload = top_god_nodes(graph, limit=int(arguments.get("limit", 10)))
            return [types.TextContent(type="text", text=json.dumps(payload, indent=2))]
        if name == "explain_node":
            return [types.TextContent(type="text", text=explain_node(graph, arguments["label"]))]
        if name == "shortest_path":
            path = shortest_path(
                graph,
                arguments["source"],
                arguments["target"],
                int(arguments.get("max_hops", 8)),
            )
            return [types.TextContent(type="text", text=format_path(graph, path))]
        raise ValueError(f"Unknown tool: {name}")

    async def runner() -> None:
        async with stdio_server() as (read_stream, write_stream):
            await server.run(read_stream, write_stream, server.create_initialization_options())

    asyncio.run(runner())


def main() -> int:
    repo_root = Path(__file__).resolve().parents[1]
    graph_dir = repo_root / DEFAULT_GRAPH_DIR
    start_server(repo_root, graph_dir)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
