# planner-solid

`planner-solid` contains Planner's SolidStart frontend, but this repo is not a
frontend-only Vite app.

## Canonical Runtime

The canonical local runtime is the Rust server serving the built frontend:

```bash
cd /home/thetu/planner
npm run build --prefix planner-solid
cargo run -p planner-server -- --port 4174 --static-dir ./planner-solid/dist/static
```

Then open `http://127.0.0.1:4174`.

## Frontend-Only Iteration

`npm run dev` still exists for isolated frontend work:

```bash
npm run dev
```

But it is not the documented Builder/Fusion workflow and it is not the
canonical runtime shape for Planner.

## Builder Workflow

For Builder Fusion, Codex MCP setup, required env handling, and caveats, use:

- [../docs/builder-local-workflow.md](../docs/builder-local-workflow.md)
