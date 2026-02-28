# Planner v2 — Architecture

## Table of Contents

1. [Overview](#overview)
2. [Crate Dependency Graph](#crate-dependency-graph)
3. [Crate Reference](#crate-reference)
   - [planner-schemas](#planner-schemas)
   - [planner-core](#planner-core)
   - [planner-tui](#planner-tui)
   - [planner-server](#planner-server)
   - [planner-web](#planner-web)
4. [Data Flow](#data-flow)
5. [Key Design Decisions](#key-design-decisions)

---

## Overview

Planner v2 is a multi-crate Rust workspace that implements an AI-driven planning pipeline. A user description flows through a series of LLM-powered and deterministic steps — intake, compilation, adversarial review, consequence analysis, code generation, validation, and git projection — producing a fully auditable, content-addressed artifact chain.

The workspace is structured around four Rust crates and one web frontend:

| Component | Language | Role |
|---|---|---|
| `planner-schemas` | Rust | Shared type definitions and artifact registry |
| `planner-core` | Rust | Pipeline engine, LLM clients, storage |
| `planner-tui` | Rust | Ratatui terminal UI |
| `planner-server` | Rust | Axum HTTP/WebSocket server |
| `planner-web` | HTML/CSS/JS | Socratic Lobby single-page frontend |

---

## Crate Dependency Graph

```
planner-schemas
      ▲
      │
      ├──── planner-core
      │           ▲
      │           │
      ├──── planner-tui
      │
      └──── planner-server
```

- **`planner-schemas`** — standalone; no dependencies on other workspace crates.
- **`planner-core`** — depends on `planner-schemas`.
- **`planner-tui`** — depends on `planner-core` and `planner-schemas`.
- **`planner-server`** — depends on `planner-core` and `planner-schemas`.

---

## Crate Reference

### planner-schemas

> **Role:** Types-only crate. Defines the canonical data model shared across all other crates.

#### Core Wrapper: `Turn<T>`

Every artifact in the system is stored as a `Turn<T>`. Turns are immutable and content-addressed.

| Field | Type | Description |
|---|---|---|
| `hash` | blake3 digest | Content address of this turn |
| `parent` | `Option<blake3>` | Hash of the parent turn (forms the parent chain) |
| `created` | timestamp | Wall-clock creation time |
| `payload` | `T` | The typed artifact |

The parent chain enables full traceability of how each artifact was derived.

#### `ArtifactPayload` Trait

Marker/behavior trait that all typed artifacts implement. Enables generic storage and retrieval through the CXDB engine.

#### CXDB Artifact Type Registry

18 artifact types are registered in the CXDB type registry:

| Type | Description |
|---|---|
| `IntakeV1` | Sacred anchors, satisfaction seeds, intent summary |
| `NLSpecV1` | Natural-language specification |
| `GraphDotV1` | DOT-format dependency/component graph |
| `ScenarioSetV1` | Acceptance scenarios for validation |
| `FactoryOutputV1` | Generated code/artifacts from the factory |
| `SatisfactionResultV1` | Scenario validation results |
| `RunBudgetV1` | Token/cost budget for a pipeline run |
| `AgentsManifestV1` | Agent roster and capability declarations |
| `ArReportV1` | Adversarial review findings |
| `ConsequenceCardV1` | Human-readable consequence summary |
| `PreviewSnapshotV1` | Point-in-time pipeline state snapshot |
| `RalphFindingV1` | Consequence analysis and lock-in findings |
| `GitCommitV1` | Git commit reference for factory output |
| `GateResultV1` | Pipeline gate pass/fail result |
| `DecisionV1` | Recorded decision with rationale |
| `ContextPackV1` | Assembled context for AR review |
| `DtuConfigV1` | Deterministic Test Unit configuration |
| `PyramidSummaryV1` | Minto Pyramid structured summary |

---

### planner-core

> **Role:** The engine. Contains the LLM client layer, pipeline orchestration, content-addressed storage, legacy storage, and deterministic test units.

#### Module Map

```
planner-core/
├── llm/
│   ├── mod.rs          # LlmClient trait + core types
│   └── providers.rs    # AnthropicCliClient, GoogleCliClient, OpenAiCliClient, LlmRouter
├── pipeline/
│   ├── mod.rs          # Orchestration entry points
│   ├── project.rs      # Multi-project registry
│   └── steps/
│       ├── intake.rs
│       ├── chunk_planner.rs
│       ├── compile.rs
│       ├── linter.rs
│       ├── context_pack.rs
│       ├── ar.rs
│       ├── ar_refinement.rs
│       ├── ralph.rs
│       ├── factory.rs
│       ├── factory_worker.rs
│       ├── validate.rs
│       ├── telemetry.rs
│       └── git.rs
├── cxdb/
│   ├── mod.rs          # CxdbEngine trait + InMemoryCxdbEngine
│   ├── durable.rs      # DurableCxdbEngine (filesystem-backed)
│   ├── protocol.rs     # Wire protocol types
│   └── query.rs        # Query engine
├── storage/
│   └── mod.rs          # SQLite-based TurnStore (legacy)
├── dtu/
│   ├── mod.rs          # DtuRegistry, DtuRequest/DtuResponse
│   ├── stripe.rs
│   ├── auth0.rs
│   ├── sendgrid.rs
│   ├── supabase.rs
│   └── twilio.rs
├── verification.rs     # Lean4 formal verification stub generation
├── audit.rs            # Anti-lock-in dependency audit
├── pyramid.rs          # Minto Pyramid summary generation
└── main.rs             # CLI binary
```

---

#### `llm/` — LLM Client Layer

##### Core Types (`mod.rs`)

- **`LlmClient` trait** — abstraction over all LLM providers
- **`CompletionRequest`** — model name, messages, optional parameters
- **`CompletionResponse`** — response text, token usage
- **`LlmError`** — typed error variants
- **`Role`** — `System`, `User`, `Assistant`
- **`Message`** — `(Role, String)` pair

##### Providers (`providers.rs`)

All clients are **CLI-native** — they shell out to the user's locally installed CLI tools rather than making HTTP API calls directly.

| Client | CLI Command |
|---|---|
| `AnthropicCliClient` | `claude -p --dangerously-skip-permissions --output-format stream-json --verbose --model <model> "<prompt>"` |
| `GoogleCliClient` | `gemini -p --output-format stream-json --yolo --model <model> "<prompt>"` |
| `OpenAiCliClient` | `codex exec --json --sandbox workspace-write -m <model> "<prompt>"` |

**`LlmRouter`** selects the correct client by model name prefix:

| Prefix | Client |
|---|---|
| `claude-` | `AnthropicCliClient` |
| `gemini-` | `GoogleCliClient` |
| `gpt-` | `OpenAiCliClient` |

Helper utilities: `cli_available()` (checks PATH), `run_cli()` (executes with timeout), `build_prompt()` (formats message history into a single prompt string).

---

#### `pipeline/` — Pipeline Engine

##### Orchestration Entry Points (`mod.rs`)

| Function | Description |
|---|---|
| `run_phase0_front_office()` | Runs Intake → Chunk Planner → Compiler only |
| `run_phase0_full()` | Runs the complete pipeline end-to-end |
| `run_phase0_full_with_worker(worker)` | Phase 7 variant using a pluggable `FactoryWorker` |

##### Project Registry (`project.rs`)

`MultiProjectRegistry` tracks multiple active projects. Each project record stores metadata including project ID, name, description, and creation timestamp.

---

#### `pipeline/steps/` — Pipeline Steps

Each step is an isolated module with a single responsibility.

| # | Module | Step Name | Input → Output |
|---|---|---|---|
| 1 | `intake.rs` | **Intake Gateway** | User description → `IntakeV1` (sacred anchors, satisfaction seeds, intent summary) |
| 2 | `chunk_planner.rs` | **Chunk Planner** | `IntakeV1` → chunk boundaries (splits complex specs into manageable units) |
| 3 | `compile.rs` | **Compiler** | `IntakeV1` → `NLSpecV1` + `GraphDotV1` + `ScenarioSetV1` + `AgentsManifestV1` |
| 4 | `linter.rs` | **Spec Linter** | `NLSpecV1` → `LintResult` (12 deterministic quality rules) |
| 5 | `context_pack.rs` | **Context Packer** | Spec artifacts → `ContextPackV1` (assembled AR review context) |
| 6 | `ar.rs` | **AR Reviewer** | `ContextPackV1` → `ArReportV1` (three-model adversarial panel: Opus + GPT-5.2 + Gemini) |
| 7 | `ar_refinement.rs` | **AR Refiner** | `ArReportV1` → amended `NLSpecV1` |
| 8 | `ralph.rs` | **Ralph** | Amended spec → `RalphFindingV1` + `ConsequenceCardV1` + `DtuConfigV1` |
| 9 | `factory.rs` | **Factory Diplomat** | `NLSpecV1` → `FactoryOutputV1` (Kilroy simulation mode + real factory handoff, `RunDirectory` management, checkpoint polling) |
| 10 | `factory_worker.rs` | **FactoryWorker** | Pluggable code generation backend (see below) |
| 11 | `validate.rs` | **Scenario Validator** | `FactoryOutputV1` + `ScenarioSetV1` → `SatisfactionResultV1` |
| 12 | `telemetry.rs` | **Telemetry Presenter** | Pipeline results → `TelemetryReport` + `ConsequenceCards` |
| 13 | `git.rs` | **Git Projection** | `FactoryOutputV1` → `GitCommitV1` (with simulation fallback) |

##### `FactoryWorker` Trait (`factory_worker.rs`)

The factory step uses a pluggable worker interface for swappable code-generation backends:

| Implementation | Behavior |
|---|---|
| `CodexFactoryWorker` | Shells out to `codex exec` for real code generation |
| `MockFactoryWorker` | Returns deterministic outputs for testing (no CLI calls) |

`WorktreeManager` manages isolated worktree directories per generation task, preventing concurrent runs from interfering with each other.

---

#### `cxdb/` — Content-Addressed Storage

The primary storage subsystem for all pipeline artifacts.

| Module | Description |
|---|---|
| `mod.rs` | `CxdbEngine` trait; `InMemoryCxdbEngine` (testing/ephemeral use) |
| `durable.rs` | `DurableCxdbEngine` — filesystem-backed MessagePack persistence with content-addressed blob store and WAL for crash recovery. **Not SQLite.** |
| `protocol.rs` | CXDB wire protocol types |
| `query.rs` | CXDB query engine |

Key properties of `DurableCxdbEngine`:
- **Serialization:** MessagePack (compact binary)
- **Addressing:** blake3 content hashing
- **Durability:** Write-ahead log (WAL) for crash recovery
- **Layout:** Filesystem blob store (not a relational database)

---

#### `storage/` — Legacy Storage

`mod.rs` defines the `TurnStore` trait backed by **SQLite**. Used by some pipeline paths; may be superseded by CXDB as the codebase matures.

---

#### `dtu/` — Deterministic Test Units

Behavioral clones of third-party service APIs for integration testing without hitting live systems.

| Module | Cloned Service |
|---|---|
| `stripe.rs` | Stripe payment API |
| `auth0.rs` | Auth0 identity API |
| `sendgrid.rs` | SendGrid email API |
| `supabase.rs` | Supabase data API |
| `twilio.rs` | Twilio messaging API |

`DtuRegistry` (`mod.rs`) dispatches `DtuRequest` → `DtuResponse` to the appropriate clone. DTU configurations for a given project are stored as `DtuConfigV1` artifacts by the Ralph step.

---

#### Other `planner-core` Modules

| Module | Description |
|---|---|
| `verification.rs` | Lean4 formal verification stub generation |
| `audit.rs` | Anti-lock-in audit for dependency analysis |
| `pyramid.rs` | Minto Pyramid structured summary generation |

---

#### `main.rs` — CLI Binary

Three operating modes:

| Flag | Mode |
|---|---|
| *(default)* or `--full` | Complete pipeline |
| `--front-office-only` / `--fo` | Front Office only (Intake → Chunk Planner → Compiler) |
| `--help` / `-h` | Usage information |

---

### planner-tui

> **Role:** Ratatui-based terminal UI for interacting with the planner.

| Module | Description |
|---|---|
| `app.rs` | App state model: `ChatMessage`, `PipelineStage`, `StageStatus`, `App` struct with `handle_key()`, `tick()`, and canned Socratic responses |
| `ui.rs` | Rendering: header, scrollable chat history, pipeline status bar, input box |
| `events.rs` | Crossterm event handler for `Key`, `Tick`, and `Resize` events |

> **Note:** The TUI currently uses canned planner responses for demonstration purposes. It is not yet wired to the real pipeline in `planner-core`.

---

### planner-server

> **Role:** Axum-based HTTP server that exposes the planning pipeline as a REST API with WebSocket support stubs.

| Module | Description |
|---|---|
| `main.rs` | Server setup; CLI args (`--port`, `--static-dir`); CORS configuration; static file serving |
| `api.rs` | REST endpoint handlers (see below) |
| `session.rs` | In-memory `SessionStore`, `PlanningSession`, session lifecycle management |
| `ws.rs` | WebSocket message types (`ServerMessage`, `ClientMessage`) — serialization only, handler not yet implemented |

#### REST API

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/health` | Health check |
| `POST` | `/api/sessions` | Create a new planning session |
| `GET` | `/api/sessions/:id` | Get session state |
| `POST` | `/api/sessions/:id/message` | Send a message; returns canned planner response |
| `GET` | `/api/models` | List available models |

> **Note:** WebSocket types are defined in `ws.rs` but the handler is not yet implemented.

---

### planner-web

> **Role:** Single-file static frontend serving the Socratic Lobby chat interface.

- **Location:** `dist/index.html`
- **Stack:** Plain HTML, CSS, and JavaScript (no build step)
- **Theme:** Dark
- **Features:** Chat interface, pipeline stage visualization
- **Backend:** Connects to `planner-server` REST API

---

## Data Flow

```
User Description
      │
      ▼
Intake Gateway (Claude Opus 4.6)
      │ ──► IntakeV1
      ▼
Chunk Planner
      │ ──► Chunk boundaries
      ▼
Compiler (Claude Opus 4.6)
      │ ──► NLSpecV1 + GraphDotV1 + ScenarioSetV1 + AgentsManifestV1
      ▼
Spec Linter (12 deterministic rules)
      │ ──► LintResult
      ▼
Context Packer
      │ ──► ContextPackV1
      ▼
AR Review (3-model panel: Opus + GPT-5.2 + Gemini)
      │ ──► ArReportV1
      ▼
AR Refinement (Claude Opus 4.6)
      │ ──► Amended NLSpecV1
      ▼
Ralph (Claude Sonnet 4.6)
      │ ──► RalphFindingV1 + ConsequenceCardV1 + DtuConfigV1
      ▼
Factory Diplomat → CodexFactoryWorker (GPT-5.3-Codex)
      │ ──► FactoryOutputV1
      ▼
Scenario Validator (Gemini 3.1 Pro)
      │ ──► SatisfactionResultV1
      ▼
Telemetry Presenter (Claude Haiku 4.5)
      │ ──► TelemetryReport + ConsequenceCards
      ▼
Git Projection
      │ ──► GitCommitV1
      ▼
Done
```

Every artifact produced at each step is wrapped in a `Turn<T>` and stored in the CXDB engine. The blake3 content hash of each turn, combined with its parent reference, forms an immutable audit trail from user description to final commit.

---

## Key Design Decisions

### 1. CLI-Native LLM Access

No HTTP API keys are managed by the application. All LLM calls shell out to `claude`, `gemini`, and `codex` CLIs, leveraging the user's existing subscriptions. This eliminates credential management, simplifies deployment, and makes the application portable to any environment where these CLIs are installed.

### 2. Content-Addressed Storage (CXDB)

CXDB uses blake3 hashing to derive storage addresses from content. MessagePack provides compact binary serialization. The filesystem blob store with a write-ahead log provides durability without the overhead or coupling of a relational database. Identical artifacts produced by identical inputs share the same address, enabling deduplication and deterministic reproducibility.

### 3. Turn-Based Event Sourcing

Every pipeline artifact is wrapped in `Turn<T>` carrying a content hash, parent hash, and creation timestamp. The parent chain creates a fully traceable lineage from any artifact back to the original `IntakeV1`. This enables audit, replay, and diff operations across pipeline runs.

### 4. Pluggable Factory

The `FactoryWorker` trait decouples the Factory Diplomat step from any specific code-generation backend. `CodexFactoryWorker` drives real generation via `codex exec`; `MockFactoryWorker` returns deterministic outputs for fast, hermetic testing. Swapping backends requires no changes to the pipeline orchestration.

### 5. Three-Model Adversarial Review

The AR Review step uses three LLM providers simultaneously (Anthropic Opus, OpenAI GPT-5.2, Google Gemini). Each model has distinct training data and failure modes. Using all three in an adversarial panel maximizes issue detection coverage and reduces the risk of a single model's blind spots going unchecked.

### 6. DTU Behavioral Clones

Deterministic Test Units provide faithful behavioral clones of Stripe, Auth0, SendGrid, Supabase, and Twilio. Tests can exercise full integration paths — including error cases and edge conditions — without network access, rate limits, or cost. DTU configurations are materialized as `DtuConfigV1` artifacts by the Ralph step so they travel with the spec.

### 7. Simulation Fallbacks

When CLI tools (`claude`, `gemini`, `codex`, `git`) are unavailable, the pipeline falls back to simulation mode. Simulation fallbacks are present in the Factory Diplomat and Git Projection steps. This allows development and testing in environments where the full CLI toolchain is not installed, and prevents hard failures during demos or CI runs without full credentials.
