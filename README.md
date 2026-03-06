# Planner v2

**A "Dark Factory" pipeline for AI-powered software planning and code generation.**

Planner v2 is a Rust workspace that takes a plain-English feature description and produces structured specifications, dependency graphs, generated code, scenario validation, and a Git commit — all driven by native CLI tools from your own AI subscriptions. No HTTP API keys required.

---

![Build](https://img.shields.io/badge/build-passing-brightgreen)
![Tests](https://img.shields.io/badge/tests-474%20passing-brightgreen)
![License](https://img.shields.io/badge/license-MIT-blue)

---

## Features

- **CLI-native LLM routing** — shells out to `claude`, `gemini`, and `codex` binaries; uses your own Max/Pro/ChatGPT Pro subscriptions, not HTTP API keys
- **Full Dark Factory pipeline** — 12 sequential stages from natural-language intake through Git commit
- **Three-model Adversarial Review panel** — Claude Opus 4.6, GPT-5.2, and Gemini 3.1 Pro review every spec in parallel (parallelized via `tokio::join!`)
- **Ralph anti-lock-in audit** — static analysis of generated specs for third-party dependency risk
- **Lean4 formal verification stubs** — generates proposition stubs from NLSpec for downstream proof workflows
- **DTU Registry** — behavioral test clones for Stripe, Auth0, SendGrid, Supabase, and Twilio; clones wired into validation pipeline
- **Durable event-sourcing storage** — filesystem MessagePack blob store (CXDB) with content-addressed keys; all 12 artifact types persisted
- **Isolated code-gen worktrees** — `WorktreeManager` gives the Factory Worker a clean directory per run
- **Factory compilation check** — post-generation `cargo check` validates produced code before acceptance
- **JSON repair utility** — 4-strategy malformed-JSON recovery for resilient LLM output parsing
- **Ratatui terminal UI** — full Socratic planning session in the terminal (`planner-tui`)
- **Axum HTTP + WebSocket server** — serves the React frontend and exposes a versioned REST + WebSocket API (`planner-server`)
- **React SPA frontend** — Auth0-integrated dashboard with WebSocket chat, pipeline visualization, and XSS prevention (`planner-web`)
- **Fail-closed JWT authentication** — no auth bypass; `parking_lot::RwLock` (no poisoning); session TTL cleanup (1 hr TTL, 5-min sweep)
- **Rate limiting** — 100 requests/min per IP; returns `429 Too Many Requests`
- **RBAC type system** — 4 roles, 9 permissions; enforced at the handler level
- **API versioning** — all endpoints under `/api/v1`
- **474 tests, 0 failures** — 377 Rust (245 unit · 45 integration · 4 schema · 61 server · 22 TUI) + 97 frontend (Vitest + React Testing Library)

---

## Quick Start

### Linux system install (`deploy/install.sh`)

Use the bundled installer if you want Planner running as a persistent `systemd` service on a Linux host:

```bash
git clone https://github.com/theturtleautomation/planner
cd planner
sudo ./deploy/install.sh

sudo systemctl status planner
journalctl -u planner -f
```

The installer builds the Rust workspace and frontend, installs `planner-server`, deploys the web assets, writes the `systemd` unit and default env file, and prepares isolated LLM CLI homes under `/opt/planner/cli-home/`.

### Source build / local development

```bash
git clone https://github.com/theturtleautomation/planner
cd planner
make build          # cargo build + npm install + vite build
make test           # cargo test + vitest

# Run the server (serves web UI at http://localhost:3100)
./target/release/planner-server

# Or run the TUI
./target/release/planner-tui
```

The pipeline will detect whichever of `claude`, `gemini`, or `codex` you have installed and route model calls accordingly.

---

## Installation

### Linux Installer (`deploy/install.sh`)

`deploy/install.sh` is the supported production install path for Linux + `systemd`.

It expects Rust, Node.js/npm, and Git to already be installed, then it will:

- build the release workspace and `planner-web`
- create the `planner` service user
- install `planner-server` to `/usr/local/bin/planner-server`
- install the web bundle to `/opt/planner/web`
- install `/etc/systemd/system/planner.service`
- install `/etc/planner/planner.env` if it does not already exist
- set up isolated CLI homes under `/opt/planner/cli-home` and a clean sandbox under `/opt/planner/cli-sandbox`
- attempt to install `claude`, `gemini`, and `codex` into `/opt/planner/bin`
- verify which installed providers are authenticated

```bash
# Full install
sudo ./deploy/install.sh

# Rebuild, reinstall, and restart without replacing /etc/planner/planner.env
sudo ./deploy/install.sh --update

# Remove the service, install tree, config, and service user
sudo ./deploy/install.sh --uninstall
```

### Rust Toolchain

Install via [rustup](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update stable
```

### LLM CLI Tools (at least one required)

Planner v2 shells out to native CLI binaries — no API keys are read from environment variables. You need at least one of the following installed and authenticated:

For local development, install them on your own `$PATH`. For a `systemd` deployment, `deploy/install.sh` will try to install them into `/opt/planner/bin` automatically and then check their auth state.

`deploy/install.sh` also refreshes npm-based CLIs (`gemini`, `codex`) to the latest published version it can resolve at install time. For Gemini CLI, the installer applies a Planner compatibility patch so subscription auth works in non-interactive mode even when Planner disables all Gemini tools.

| CLI Binary | Provider | Install Method | Subscription Required |
|---|---|---|---|
| `claude` | Anthropic | Native installer (recommended) | Claude Max or Pro |
| `gemini` | Google | `npm install -g @google/gemini-cli` | Gemini Pro |
| `codex` | OpenAI | `npm install -g @openai/codex` | ChatGPT Pro |

```bash
# Claude — native installer (self-contained binary, no Node.js needed)
curl -fsSL https://claude.ai/install.sh | bash

# Gemini + Codex — npm (requires Node.js 18+)
npm install -g @google/gemini-cli
npm install -g @openai/codex
```

If none are found on `$PATH`, `planner-core` will exit with a clear error listing what to install.

> **Note:** The three-model AR review panel and the Factory Worker require all three CLIs for full fidelity. The pipeline degrades gracefully — unavailable providers are skipped during routing.

### Git

Required for the Git Projection pipeline stage and for `deploy/install.sh`:

```bash
# macOS
brew install git

# Debian/Ubuntu
apt install git
```

### Node.js (required for `planner-web` and `deploy/install.sh`)

Required to build the React frontend locally, and also required by `deploy/install.sh`, which runs `npm install --prefix planner-web` and `npm run build --prefix planner-web` during installation:

```bash
# Node.js 18+ required
node --version
```

### Build (Makefile)

The root Makefile orchestrates both Rust and web builds:

| Command | What it does |
|---|---|
| `make` | Check + build everything |
| `make build` | `cargo build` + `vite build` |
| `make test` | `cargo test` + `vitest` |
| `make check` | `cargo check` + tsc/vite |
| `make rust-release` | `cargo build --release` |
| `make web-dev` | Start vite dev server |
| `make clippy` | Lint Rust |
| `make web-lint` | Lint TS/JS |
| `make clean` | Remove all artifacts |
| `make help` | List all targets |

Node modules are auto-installed if missing.

---

## Deployment

### systemd Service

Planner ships with a systemd service unit and install script for production deployment.

#### Install

```bash
sudo ./deploy/install.sh              # Full install: build, create user, enable service
sudo ./deploy/install.sh --update     # Rebuild + restart (preserves config)
sudo ./deploy/install.sh --uninstall  # Remove everything
```

#### Installed Paths

| Path | Contents |
|---|---|
| `/usr/local/bin/planner-server` | Release binary |
| `/opt/planner/web/` | Vite production build |
| `/opt/planner/data/` | CXDB MessagePack storage |
| `/opt/planner/bin/` | Installed LLM CLI binaries + copied `node` runtime |
| `/opt/planner/cli-home/` | Isolated `HOME` directories for `claude`, `gemini`, and `codex` |
| `/opt/planner/cli-sandbox/` | Clean working directory for provider CLI invocations |
| `/etc/planner/planner.env` | Environment configuration |
| `/etc/systemd/system/planner.service` | systemd unit |

#### Service Management

```bash
sudo systemctl status planner         # Check status
sudo systemctl restart planner        # Restart
journalctl -u planner -f              # Tail logs
```

The service runs as a dedicated `planner` system user with systemd hardening (ProtectSystem=strict, NoNewPrivileges, PrivateTmp).

### Environment Configuration

All configuration lives in `/etc/planner/planner.env`. The file is preserved across `--update` runs. Key sections:

- **Server** — Port, bind address
- **Logging** — `RUST_LOG` with per-crate granularity
- **Authentication** — Auth0 JWT (optional; omit for dev mode)
- **LLM Providers** — No API keys needed; authenticate CLIs as the service user:
  ```bash
  sudo -u planner HOME=/opt/planner/cli-home/claude /opt/planner/bin/claude login
  sudo -u planner HOME=/opt/planner/cli-home/gemini /opt/planner/bin/gemini auth login
  sudo -u planner HOME=/opt/planner/cli-home/codex CODEX_HOME=/opt/planner/cli-home/codex/.codex /opt/planner/bin/codex login
  ```
- **Factory Worker** — Worktree root, sandbox mode
- **Vault Integration** — HashiCorp Vault Agent, systemd LoadCredential, SOPS

See `deploy/planner.env` for the fully documented template.

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

Serves the React frontend and exposes a versioned REST + WebSocket API for browser-based planning sessions. Endpoints are available under both `/api` and `/api/v1`. JWT authentication is fail-closed — requests without a valid token are rejected with `401`.

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

| Method | Path | Auth | Description |
|---|---|---|---|
| `GET` | `/api/health` | None | Health check |
| `GET` | `/api/models` | Required | List available LLM models |
| `GET` | `/api/sessions` | Required | List sessions for current user |
| `POST` | `/api/sessions` | Required | Create a new planning session |
| `GET` | `/api/sessions/:id` | Required | Get session state |
| `POST` | `/api/sessions/:id/message` | Required | Send a message to the session |
| `GET` | `/api/sessions/:id/ws` | Required | WebSocket for real-time updates |
| `POST` | `/api/sessions/:id/socratic` | Required | Start Socratic interview |
| `GET` | `/api/sessions/:id/socratic/ws` | Required | Socratic interview WebSocket |
| `GET` | `/api/sessions/:id/belief-state` | Required | Get current belief state |
| `GET` | `/api/sessions/:id/turns` | Required | List CXDB turns |
| `GET` | `/api/sessions/:id/runs` | Required | List pipeline runs |
| `GET` | `/*` | None | Static file serving (React frontend) |

Endpoints are available under both `/api` and `/api/v1`. Rate limiting applies to all API routes: 100 requests/minute per IP. Excess requests receive `429 Too Many Requests`.

If `--static-dir` does not exist, the server starts in API-only mode.

---

### `planner-web` — React Frontend

A full React + TypeScript + Vite single-page application. Communicates with `planner-server` via REST and WebSocket. Auth0 is optional — omitting Auth0 environment variables activates dev mode (no login required).

```bash
# From repo root (proxied via root package.json)
npm run build
npm run dev
npm test

# Or from planner-web/ directly
cd planner-web
npm install
npm run dev
```

See [planner-web/README.md](./planner-web/README.md) for full frontend documentation and [AUTH0_SETUP.md](./AUTH0_SETUP.md) for authentication configuration.

---

## Architecture Overview

Planner v2 is a four-crate Rust workspace with a React frontend, built around an event-sourced pipeline engine.

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

Browser
    │
    ▼
planner-web (React SPA, Auth0, WebSocket)
    │
    ▼
planner-server (Axum, JWT auth, rate limiting, RBAC)
    │
    ▼
planner-core (pipeline engine)
```

See [ARCHITECTURE.md](./ARCHITECTURE.md) for the full design document, including the CXDB content-addressed storage protocol, DTU behavioral clone design, and Lean4 verification integration.

---

## Pipeline Stages

Each stage produces one or more typed artifacts (defined in `planner-schemas`) and persists them to CXDB as immutable `Turn` events. `ConsequenceCards` are also persisted as Turns.

| # | Stage | Description | Model |
|---|---|---|---|
| 1 | **Intake** | Socratic interview → `IntakeV1` (sacred anchors, satisfaction seeds) | Claude Opus 4.6 |
| 2 | **ChunkPlan** | Decides single vs. multi-chunk compilation strategy | Claude Opus 4.6 |
| 3 | **Compile** | `IntakeV1` → `NLSpecV1` (requirements, DoD, constraints); `ContextPack` wired in | Claude Opus 4.6 |
| 4 | **Lint** | 12-rule NLSpec validation + cross-chunk consistency checks | *(static)* |
| 5 | **AR Review** | Three-model **parallel** adversarial review via `tokio::join!` → `ArReportV1` | Opus 4.6 · GPT-5.2 · Gemini 3.1 Pro |
| 6 | **Refinement** | Blocking AR findings → spec amendments → re-lint loop | Claude Sonnet 4.6 |
| 7 | **Ralph** | Anti-lock-in dependency audit → `RalphFindingV1` | Claude Sonnet 4.6 |
| 8 | **GraphDot** | `NLSpecV1` → `GraphDotV1` (dependency DAG, run budget); `PyramidBuilder` wired | Claude Opus 4.6 |
| 9 | **Factory** | Node-by-node code generation → `FactoryOutputV1`; post-generation compile check | GPT-5.3-Codex |
| 10 | **Validate** | `ScenarioSetV1` + generated code + DTU clones → `SatisfactionResultV1` | Gemini 3.1 Pro |
| 11 | **Telemetry** | Plain-English summary + `ConsequenceCardV1` items (persisted as Turns) | Claude Haiku 4.5 |
| 12 | **Git** | Commits generated files to an isolated worktree branch → `GitCommitV1` | *(shell)* |

The `--front-office-only` flag runs stages 1–4 (Intake through Lint) and stops before code generation.

---

## Socratic Intake Engine

The Socratic intake replaces the simple text-prompt intake with a structured interview loop. Six Rust modules cooperate to elicit requirements through dialogue:

| Module | Role |
|---|---|
| `domain_classifier` | Classifies project type (web app, CLI, API, etc.) and complexity tier |
| `constitution` | Generates required dimensions (auth, storage, UI, etc.) based on classification |
| `question_planner` | Selects the next question targeting the most uncertain dimension |
| `belief_state` | Tracks filled/uncertain/missing slots with confidence scores |
| `convergence` | Decides when enough information has been gathered (staleness detection, budget) |
| `speculative_draft` | Generates a mid-interview draft for user correction |

The engine supports two I/O backends via the `SocraticIO` trait:
- **TUI** — Split-pane Ratatui terminal interface
- **WebSocket** — Real-time browser session with typed messages

### Web Frontend Components

The Socratic interview renders in a split-pane layout:

| Component | Purpose |
|---|---|
| `SessionPage` | Orchestrates the interview, routes WS messages to child components |
| `ChatPanel` | Message history with role-based styling (user/planner/system/event) |
| `BeliefStatePanel` | Live visualization of filled/uncertain/missing slots with edit support |
| `SpeculativeDraftView` | Mid-interview draft with per-section [Correct]/[Fix] buttons |
| `ConvergenceBar` | Visual progress toward interview completion |
| `ClassificationBadge` | Project type and complexity display |
| `MessageInput` | Auto-grow textarea with convergence-aware "done" button |
| `useSocraticWebSocket` | Hook managing WS connection, typed message dispatch, reconnection |

---

## Project Structure

```
planner/
├── Cargo.toml                      # Workspace manifest
├── Cargo.lock
├── Makefile                        # Orchestrates Rust + web builds
├── package.json                    # Root npm scripts (proxied to planner-web)
├── AUTH0_SETUP.md                  # Auth0 configuration guide
├── ARCHITECTURE.md                 # Full design document
├── CONTRIBUTING.md
├── DEPLOYMENT.md
│
├── deploy/                         # Production deployment
│   ├── install.sh                  # systemd install/update/uninstall script
│   ├── planner.service             # systemd unit file
│   ├── planner.env                 # Environment configuration template
│   └── package.json
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
│   │   │   ├── providers.rs        # AnthropicCliClient, GoogleCliClient, OpenAiCliClient, LlmRouter
│   │   │   └── json_repair.rs      # 4-strategy malformed-JSON recovery
│   │   ├── pipeline/
│   │   │   ├── mod.rs              # PipelineConfig, Recipe, run_phase0_*
│   │   │   ├── steps/              # One module per pipeline stage
│   │   │   │   ├── intake.rs
│   │   │   │   ├── chunk_planner.rs
│   │   │   │   ├── compile.rs
│   │   │   │   ├── linter.rs
│   │   │   │   ├── ar.rs           # Parallelized via tokio::join!
│   │   │   │   ├── ar_refinement.rs
│   │   │   │   ├── ralph.rs
│   │   │   │   ├── factory.rs
│   │   │   │   ├── factory_worker.rs  # FactoryWorker trait, CodexFactoryWorker, MockFactoryWorker
│   │   │   │   ├── validate.rs
│   │   │   │   ├── telemetry.rs
│   │   │   │   ├── git.rs
│   │   │   │   ├── context_pack.rs
│   │   │   │   └── socratic/       # Socratic intake engine
│   │   │   │       ├── mod.rs
│   │   │   │       ├── domain_classifier.rs
│   │   │   │       ├── constitution.rs
│   │   │   │       ├── question_planner.rs
│   │   │   │       ├── belief_state.rs
│   │   │   │       ├── convergence.rs
│   │   │   │       └── speculative_draft.rs
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
│       ├── api.rs                  # REST route handlers (/api/v1)
│       ├── auth.rs                 # Fail-closed JWT middleware
│       ├── rate_limit.rs           # Token-bucket rate limiter (100 req/min per IP)
│       ├── rbac.rs                 # RBAC type system (4 roles, 9 permissions)
│       ├── session.rs              # SessionStore (parking_lot::RwLock, TTL cleanup)
│       ├── ws.rs                   # WebSocket upgrade + message loop
│       └── ws_socratic.rs          # Socratic interview WebSocket handler
│
├── planner-web/                    # React + TypeScript + Vite SPA
│   ├── src/
│   │   ├── main.tsx                # App entry point
│   │   ├── App.tsx                 # Root component + routing
│   │   ├── config.ts               # Runtime configuration
│   │   ├── types.ts                # Shared TypeScript types
│   │   ├── api/
│   │   │   ├── client.ts           # ApiError class, typed fetch wrappers
│   │   │   └── __tests__/
│   │   │       └── client.test.ts
│   │   ├── auth/
│   │   │   ├── Auth0ProviderWithNavigate.tsx
│   │   │   ├── ProtectedRoute.tsx
│   │   │   └── useAuthenticatedFetch.ts
│   │   ├── components/
│   │   │   ├── BeliefStatePanel.tsx    # Live slot visualization with edit support
│   │   │   ├── ChatPanel.tsx           # Message list with scroll preservation
│   │   │   ├── ClassificationBadge.tsx # Project type and complexity display
│   │   │   ├── ConvergenceBar.tsx      # Interview completion progress
│   │   │   ├── Layout.tsx              # App shell
│   │   │   ├── MessageInput.tsx        # Auto-grow textarea
│   │   │   ├── PipelineBar.tsx         # Stage visualization bar
│   │   │   ├── QuickOptions.tsx
│   │   │   ├── SpeculativeDraftView.tsx  # Mid-interview draft with correction UI
│   │   │   └── __tests__/
│   │   │       ├── ChatPanel.test.tsx
│   │   │       ├── Layout.test.tsx
│   │   │       ├── MessageInput.test.tsx
│   │   │       └── PipelineBar.test.tsx
│   │   ├── hooks/
│   │   │   ├── useSocraticWebSocket.ts  # WS connection, typed dispatch, reconnection
│   │   │   └── useSessionWebSocket.ts   # WebSocket with reconnection logic
│   │   ├── pages/
│   │   │   ├── Dashboard.tsx       # Session listing dashboard
│   │   │   ├── LoginPage.tsx       # Auth0 login / dev-mode bypass
│   │   │   ├── SessionPage.tsx     # Chat + pipeline view
│   │   │   └── __tests__/
│   │   │       └── LoginPage.test.tsx
│   │   └── test/
│   │       └── setup.ts            # Vitest setup + Auth0 mock
│   ├── dist/                       # Production build output
│   └── README.md
│
└── reference/
    ├── kilroy_preferences.yaml     # Model routing preferences
    └── kilroy_reference_template.dot  # GraphDot reference template
```

---

## Testing

### Rust Tests

```bash
# Run the full Rust test suite
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

### Frontend Tests

```bash
cd planner-web

# Run the Vitest test suite (watch mode)
npm test

# Run once (CI mode)
npm run test -- --run
```

**Test breakdown:**

| Suite | Count | Location |
|---|---|---|
| planner-core unit tests | 245 | `src/**/*.rs` (`#[cfg(test)]` blocks) |
| Integration tests | 45 | `planner-core/tests/integration_e2e.rs` |
| Schema tests | 4 | `planner-schemas/src/**` |
| Server tests | 61 | `planner-server/src/**` |
| TUI tests | 22 | `planner-tui/src/**` |
| **Rust subtotal** | **377** | |
| Frontend — API client | — | `src/api/__tests__/client.test.ts` |
| Frontend — ChatPanel | — | `src/components/__tests__/ChatPanel.test.tsx` |
| Frontend — Layout | — | `src/components/__tests__/Layout.test.tsx` |
| Frontend — MessageInput | — | `src/components/__tests__/MessageInput.test.tsx` |
| Frontend — PipelineBar | — | `src/components/__tests__/PipelineBar.test.tsx` |
| Frontend — LoginPage | — | `src/pages/__tests__/LoginPage.test.tsx` |
| **Frontend subtotal** | **97** | `planner-web/src/**/__tests__/` |
| **Total** | **474** | |

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

### Rate Limiting

The server enforces 100 requests/minute per IP address on all `/api/v1` routes using a token-bucket algorithm. Requests that exceed the limit receive a `429 Too Many Requests` response with a `Retry-After` header. The rate limit is currently set at compile time in `planner-server/src/rate_limit.rs`.

### Session TTL

Server-side sessions expire after **1 hour** of inactivity. A background task sweeps for expired sessions every **5 minutes** and removes them from memory. Active WebSocket connections are closed at expiry.

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

### Auth0

Auth0 environment variables are optional. When omitted, the server injects a synthetic `dev|local` user and the frontend skips the login screen. See [AUTH0_SETUP.md](./AUTH0_SETUP.md) for full configuration instructions.

---

## License

MIT — see [LICENSE](./LICENSE) for the full text.
