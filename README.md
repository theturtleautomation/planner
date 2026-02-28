# Planner v2

**A "Dark Factory" pipeline for AI-powered software planning and code generation.**

Planner v2 is a Rust workspace that takes a plain-English feature description and produces structured specifications, dependency graphs, generated code, scenario validation, and a Git commit — all driven by native CLI tools from your own AI subscriptions. No HTTP API keys required.

---

![Build](https://img.shields.io/badge/build-passing-brightgreen)
![Tests](https://img.shields.io/badge/tests-323%20passing-brightgreen)
![License](https://img.shields.io/badge/license-MIT-blue)

---

## Features

- **CLI-native LLM routing** — shells out to `claude`, `gemini`, and `codex` binaries; uses your own Max/Pro/ChatGPT Pro subscriptions, not HTTP API keys
- **Full Dark Factory pipeline** — 12 sequential stages from natural-language intake through Git commit
- **Three-model Adversarial Review panel** — Claude Opus 4.6, GPT-5.2, and Gemini 3.1 Pro review every spec in parallel
- **Ralph anti-lock-in audit** — static analysis of generated specs for third-party dependency risk
- **Lean4 formal verification stubs** — generates proposition stubs from NLSpec for downstream proof workflows
- **DTU Registry** — behavioral test clones for Stripe, Auth0, SendGrid, Supabase, and Twilio
- **Durable event-sourcing storage** — filesystem MessagePack blob store (CXDB) with content-addressed keys
- **Isolated code-gen worktrees** — `WorktreeManager` gives the Factory Worker a clean directory per run
- **Ratatui terminal UI** — full Socratic planning session in the terminal (`planner-tui`)
- **Axum HTTP + WebSocket server** — serves the Socratic Lobby web frontend (`planner-server`)
- **323 tests, 0 warnings** — 241 unit · 45 integration · 2 schema · 19 TUI · 16 server

---

## Quick Start

```bash
# 1. Install prerequisites (see Installation below)
rustup update stable

# 2. Clone and build
git clone https://github.com/theturtleautomation/planner
cd planner
cargo build --release

# 3. Run the full pipeline
./target/release/planner-core "Build me a task tracker widget"
```

The pipeline will detect whichever of `claude`, `gemini`, or `codex` you have installed and route model calls accordingly.

---

## Installation

### Rust Toolchain

Install via [rustup](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update stable
```

### LLM CLI Tools (at least one required)

Planner v2 shells out to native CLI binaries — no API keys are read from environment variables. You need at least one of the following installed and authenticated:

| CLI Binary | Provider | Subscription Required |
|---|---|---|
| `claude` | Anthropic | Claude Max or Pro |
| `gemini` | Google | Gemini Pro |
| `codex` | OpenAI | ChatGPT Pro |

If none are found on `$PATH`, `planner-core` will exit with a clear error listing what to install.

> **Note:** The three-model AR review panel and the Factory Worker require all three CLIs for full fidelity. The pipeline degrades gracefully — unavailable providers are skipped during routing.

### Git

Required for the Git Projection pipeline stage:

```bash
# macOS
brew install git

# Debian/Ubuntu
apt install git
```

### Build

```bash
cargo build --release
```

Binaries are written to `./target/release/`.

---

## Usage

### `planner-core` — Pipeline Runner

Runs the full Dark Factory pipeline end-to-end.

```bash
# Full pipeline (default): Intake → ... → Git
planner-core "Build me a task tracker widget"

# Explicit full mode
planner-core --full "Build me a countdown timer"

# Front Office only (stops after Compile — no code generation)
planner-core --front-office-only "Build me a pomodoro timer"
planner-core --fo "Build me a pomodoro timer"   # alias

# Help
planner-core --help
```

**Output (full pipeline):**

```
=== Phase 0 Pipeline Complete ===

Project: task-tracker-widget (task-tracker-widget)
Intent:  A reusable widget component for tracking tasks with CRUD operations

Compilation:
  NLSpecV1:           3 chunk(s), 18 total requirements, 9 DoD items
  GraphDotV1:         12 nodes, $0.42 budget
  ScenarioSetV1:      6 scenarios

Factory:
  Build Status:       Success
  Spend:              $0.38 / $5.00
  Nodes:              12 completed

Validation:
  Gates Passed:       6
  Satisfaction:       All scenarios passed ✓

Result:
  Task Tracker Widget — complete
  Generated 847 lines across 6 files; all 6 scenarios passed.

Git:
  Commit:             a3f9c12b4e7d
  Branch:             planner/task-tracker-widget
  Repo:               ./worktrees/task-tracker-widget
```

**Output (front office only):**

```
=== Phase 0 Front Office Complete ===

Project: pomodoro-timer (pomodoro-timer)
Intent:  A 25/5 pomodoro interval timer with session tracking

Artifacts produced:
  IntakeV1:           4 sacred anchors, 7 satisfaction seeds
  NLSpecV1:           2 chunk(s), 14 total requirements, 6 DoD items
  GraphDotV1:         8 nodes, $0.28 budget
  ScenarioSetV1:      4 scenarios
  AgentsManifestV1:   1842 bytes

Next: planner-core --full "Build me a pomodoro timer"
```

---

### `planner-tui` — Ratatui Terminal UI

Interactive Socratic planning session in the terminal. Conducts a structured interview to elicit requirements before handing off to the pipeline.

```bash
# New session
planner-tui

# Resume an existing session by project UUID
planner-tui --project-id 550e8400-e29b-41d4-a716-446655440000

# Help
planner-tui --help
```

**Layout:**

```
┌─────────────────────────────────────────────┐
│  Planner v2 — Socratic Planning Session     │  ← Header
├─────────────────────────────────────────────┤
│                                             │
│  [System] Welcome to Planner v2...          │  ← Chat history
│  [You] Build me a task tracker              │
│  [Planner] Let me ask some questions...     │
│                                             │
├─────────────────────────────────────────────┤
│  Pipeline: Intake ■ Compile □ Factory □ ... │  ← Stage status bar
├─────────────────────────────────────────────┤
│  > Type your response...                    │  ← Input
└─────────────────────────────────────────────┘
```

Key bindings: `Enter` to send, `Ctrl+C` / `q` to quit.

---

### `planner-server` — HTTP + WebSocket Backend

Serves the Socratic Lobby web frontend and exposes a REST + WebSocket API for browser-based planning sessions.

```bash
# Default port 3100, serves ./planner-web/dist
planner-server

# Custom port
planner-server --port 8080

# Custom static directory
planner-server --port 3100 --static-dir /path/to/planner-web/dist

# Help
planner-server --help
```

Then open `http://localhost:3100` in your browser.

**API Endpoints:**

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/health` | Health check |
| `POST` | `/api/sessions` | Create a new planning session |
| `GET` | `/api/sessions/:id` | Get session state |
| `POST` | `/api/sessions/:id/message` | Send a message to the session |
| `GET` | `/api/sessions/:id/ws` | WebSocket for real-time updates |
| `GET` | `/api/models` | List available LLM models |
| `GET` | `/*` | Static file serving (Socratic Lobby frontend) |

If `--static-dir` does not exist, the server starts in API-only mode.

---

## Architecture Overview

Planner v2 is a four-crate Rust workspace built around an event-sourced pipeline engine.

```
User prompt
    │
    ▼
LlmRouter (model prefix dispatch)
    ├── claude-*  →  AnthropicCliClient  →  claude CLI
    ├── gemini-*  →  GoogleCliClient     →  gemini CLI
    └── gpt-*     →  OpenAiCliClient     →  codex CLI
    │
    ▼
Pipeline (12 stages, linear DAG)
    │
    ├── Artifacts → DurableCxdbEngine (MessagePack blob store)
    ├── Events    → TurnStore (event sourcing)
    └── Code      → WorktreeManager (isolated directories)
```

See [ARCHITECTURE.md](./ARCHITECTURE.md) for the full design document, including the CXDB content-addressed storage protocol, DTU behavioral clone design, and Lean4 verification integration.

---

## Pipeline Stages

Each stage produces one or more typed artifacts (defined in `planner-schemas`) and persists them to CXDB as immutable `Turn` events.

| # | Stage | Description | Model |
|---|---|---|---|
| 1 | **Intake** | Socratic interview → `IntakeV1` (sacred anchors, satisfaction seeds) | Claude Opus 4.6 |
| 2 | **ChunkPlan** | Decides single vs. multi-chunk compilation strategy | Claude Opus 4.6 |
| 3 | **Compile** | `IntakeV1` → `NLSpecV1` (requirements, DoD, constraints) | Claude Opus 4.6 |
| 4 | **Lint** | 12-rule NLSpec validation + cross-chunk consistency checks | *(static)* |
| 5 | **AR Review** | Three-model parallel adversarial review → `ArReportV1` | Opus 4.6 · GPT-5.2 · Gemini 3.1 Pro |
| 6 | **Refinement** | Blocking AR findings → spec amendments → re-lint loop | Claude Sonnet 4.6 |
| 7 | **Ralph** | Anti-lock-in dependency audit → `RalphFindingV1` | Claude Sonnet 4.6 |
| 8 | **GraphDot** | `NLSpecV1` → `GraphDotV1` (dependency DAG, run budget) | Claude Opus 4.6 |
| 9 | **Factory** | Node-by-node code generation → `FactoryOutputV1` | GPT-5.3-Codex |
| 10 | **Validate** | `ScenarioSetV1` + generated code → `SatisfactionResultV1` | Gemini 3.1 Pro |
| 11 | **Telemetry** | Plain-English summary + `ConsequenceCardV1` items | Claude Haiku 4.5 |
| 12 | **Git** | Commits generated files to an isolated worktree branch → `GitCommitV1` | *(shell)* |

The `--front-office-only` flag runs stages 1–4 (Intake through Lint) and stops before code generation.

---

## Project Structure

```
planner/
├── Cargo.toml                      # Workspace manifest
├── Cargo.lock
├── .gitignore
│
├── planner-schemas/                # Artifact types & event sourcing
│   └── src/
│       ├── lib.rs
│       ├── turn.rs                 # Turn<T> event envelope
│       └── artifacts/
│           ├── intake.rs           # IntakeV1
│           ├── nlspec.rs           # NLSpecV1
│           ├── graph_dot.rs        # GraphDotV1
│           ├── factory_output.rs   # FactoryOutputV1
│           ├── scenario_set.rs     # ScenarioSetV1
│           ├── satisfaction_result.rs
│           ├── ar_report.rs        # ArReportV1
│           ├── ralph_finding.rs    # RalphFindingV1
│           ├── consequence_card.rs # ConsequenceCardV1
│           ├── git_commit.rs       # GitCommitV1
│           ├── agents_manifest.rs
│           ├── pyramid_summary.rs
│           ├── preview_snapshot.rs
│           ├── run_budget.rs
│           ├── runtime.rs
│           └── dtu.rs
│
├── planner-core/                   # Pipeline engine (CLI binary)
│   ├── src/
│   │   ├── main.rs                 # CLI entrypoint
│   │   ├── llm/
│   │   │   ├── mod.rs              # LlmClient trait, CompletionRequest/Response
│   │   │   └── providers.rs        # AnthropicCliClient, GoogleCliClient, OpenAiCliClient, LlmRouter
│   │   ├── pipeline/
│   │   │   ├── mod.rs              # PipelineConfig, Recipe, run_phase0_*
│   │   │   ├── steps/              # One module per pipeline stage
│   │   │   │   ├── intake.rs
│   │   │   │   ├── chunk_planner.rs
│   │   │   │   ├── compile.rs
│   │   │   │   ├── linter.rs
│   │   │   │   ├── ar.rs
│   │   │   │   ├── ar_refinement.rs
│   │   │   │   ├── ralph.rs
│   │   │   │   ├── factory.rs
│   │   │   │   ├── factory_worker.rs  # FactoryWorker trait, CodexFactoryWorker, MockFactoryWorker
│   │   │   │   ├── validate.rs
│   │   │   │   ├── telemetry.rs
│   │   │   │   ├── git.rs
│   │   │   │   └── context_pack.rs
│   │   │   ├── verification.rs     # Lean4 proposition stub generation
│   │   │   ├── audit.rs            # Anti-lock-in audit (Ralph module)
│   │   │   ├── pyramid.rs          # PyramidSummary rollup
│   │   │   └── project.rs          # WorktreeManager
│   │   ├── cxdb/
│   │   │   ├── mod.rs
│   │   │   ├── durable.rs          # DurableCxdbEngine (MessagePack + Blake3)
│   │   │   ├── protocol.rs         # CXDB wire protocol
│   │   │   └── query.rs
│   │   ├── storage/
│   │   │   └── mod.rs              # TurnStore trait + SQLite impl
│   │   └── dtu/
│   │       ├── mod.rs              # DtuRegistry
│   │       ├── stripe.rs
│   │       ├── auth0.rs
│   │       ├── sendgrid.rs
│   │       ├── supabase.rs
│   │       └── twilio.rs
│   └── tests/
│       └── integration_e2e.rs
│
├── planner-tui/                    # Ratatui terminal UI (CLI binary)
│   └── src/
│       ├── main.rs
│       ├── app.rs                  # App state machine
│       ├── ui.rs                   # Ratatui render loop
│       └── events.rs               # Crossterm event handler
│
├── planner-server/                 # Axum HTTP + WebSocket server (CLI binary)
│   └── src/
│       ├── main.rs
│       ├── api.rs                  # REST route handlers
│       ├── session.rs              # SessionStore
│       └── ws.rs                   # WebSocket upgrade + message loop
│
├── planner-web/
│   └── dist/
│       └── index.html              # Socratic Lobby static frontend
│
└── reference/
    ├── kilroy_preferences.yaml     # Model routing preferences
    └── kilroy_reference_template.dot  # GraphDot reference template
```

---

## Testing

```bash
# Run the full test suite
cargo test

# Run only unit tests
cargo test --lib

# Run integration tests
cargo test --test integration_e2e

# Run tests for a specific crate
cargo test -p planner-schemas
cargo test -p planner-core
cargo test -p planner-tui
cargo test -p planner-server

# Run with output (useful for pipeline stage traces)
cargo test -- --nocapture

# Run with logging enabled
RUST_LOG=planner_core=debug cargo test
```

**Test breakdown:**

| Suite | Count | Location |
|---|---|---|
| Unit tests | 241 | `src/**/*.rs` (`#[cfg(test)]` blocks) |
| Integration tests | 45 | `planner-core/tests/integration_e2e.rs` |
| Schema tests | 2 | `planner-schemas/src/**` |
| TUI tests | 19 | `planner-tui/src/**` |
| Server tests | 16 | `planner-server/src/**` |
| **Total** | **323** | |

The integration tests use `MockFactoryWorker` — they do not shell out to `claude`, `gemini`, or `codex`, so no subscriptions are needed to run the full test suite.

---

## Configuration

### Logging

Planner v2 uses `tracing` with `RUST_LOG` env-filter syntax:

```bash
# Info level for all planner crates (default)
RUST_LOG=planner_core=info planner-core "Build a widget"

# Debug pipeline steps
RUST_LOG=planner_core=debug planner-core "Build a widget"

# Trace everything (very verbose)
RUST_LOG=trace planner-core "Build a widget"
```

### Model Overrides

Default model assignments (from `reference/kilroy_preferences.yaml`):

| Component | Default Model | CLI Binary |
|---|---|---|
| Intake Gateway | `claude-opus-4-6` | `claude` |
| Compiler (NLSpec) | `claude-opus-4-6` | `claude` |
| Factory Worker | `gpt-5.3-codex` | `codex` |
| Scenario Validator | `gemini-3.1-pro` | `gemini` |
| Telemetry Presenter | `claude-haiku-4-5` | `claude` |
| Ralph Loops | `claude-sonnet-4-6` | `claude` |
| AR Reviewer (panel) | Opus 4.6 · GPT-5.2 · Gemini 3.1 Pro | all three |

Model IDs are resolved by prefix: `claude-*` → Anthropic CLI, `gemini-*` → Google CLI, `gpt-*` → OpenAI CLI. Unrecognized prefixes fall back to Anthropic.

### CLI Tool Timeouts

Each LLM CLI invocation has a default 5-minute timeout (`DEFAULT_TIMEOUT_SECS = 300`). This covers the Factory Worker's agentic code-generation loops. The timeout is not currently configurable via environment variable; change it in `planner-core/src/llm/providers.rs` and rebuild if needed.

### Server Port

```bash
# Default: 3100
planner-server

# Override at runtime
planner-server --port 8080
```

### Static Frontend Path

```bash
# Default: ./planner-web/dist
planner-server

# Override at runtime
planner-server --static-dir /var/www/planner
```

---

## .gitignore

```
/target/
*.swp
*.swo
.DS_Store
```

---

## License

MIT — see [LICENSE](./LICENSE) for the full text.
