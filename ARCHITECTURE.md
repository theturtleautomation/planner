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
5. [Authentication](#authentication)
6. [Key Design Decisions](#key-design-decisions)

---

## Overview

Planner v2 is a multi-crate Rust workspace that implements an AI-driven planning pipeline. A user description flows through a series of LLM-powered and deterministic steps — intake, compilation, adversarial review, consequence analysis, code generation, validation, and git projection — producing a fully auditable, content-addressed artifact chain.

The workspace is structured around four Rust crates and one web frontend:

| Component | Language | Role |
|---|---|---|
| `planner-schemas` | Rust | Shared type definitions and artifact registry |
| `planner-core` | Rust | Pipeline engine, LLM clients, storage |
| `planner-tui` | Rust | Ratatui terminal UI |
| `planner-server` | Rust | Axum HTTP/WebSocket server with Auth0 JWT middleware |
| `planner-web` | React + TypeScript | Socratic Lobby SPA with Auth0 authentication |

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
      └──── planner-server ───── planner-web (static build)
```

- **`planner-schemas`** — standalone; no dependencies on other workspace crates.
- **`planner-core`** — depends on `planner-schemas`.
- **`planner-tui`** — depends on `planner-core` and `planner-schemas`.
- **`planner-server`** — depends on `planner-core` and `planner-schemas`. Serves the `planner-web` static build.
- **`planner-web`** — Vite + React + TypeScript SPA. Builds to `dist/` and is served by `planner-server`.

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

> **Role:** The engine. Contains the LLM client layer, pipeline orchestration, content-addressed storage, and deterministic test units.

#### Module Map

```
planner-core/
├── llm/
│   ├── mod.rs          # LlmClient trait + core types
│   └── providers.rs    # AnthropicCliClient, GoogleCliClient, OpenAiCliClient, LlmRouter
├── pipeline/
│   ├── mod.rs          # Orchestration: run_full_pipeline() with FactoryWorker
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
│   ├── mod.rs          # CxdbEngine trait, InMemoryCxdbEngine, TurnStore trait
│   ├── durable.rs      # DurableCxdbEngine (filesystem-backed)
│   ├── protocol.rs     # Wire protocol types
│   └── query.rs        # Query engine
├── dtu/
│   ├── mod.rs          # DtuRegistry, DtuRequest/DtuResponse
│   ├── stripe.rs
│   ├── auth0.rs
│   ├── sendgrid.rs
│   ├── supabase.rs
│   └── twilio.rs
├── verification.rs     # Lean4 formal verification proposition generation
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
| `run_full_pipeline(config, worker, project_id, description)` | Runs the complete pipeline end-to-end with a pluggable `FactoryWorker` |
| `run_phase0_front_office()` | Runs Intake → Chunk Planner → Compiler only |

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
| 9 | `factory.rs` | **Factory Diplomat** | `NLSpecV1` → `FactoryOutputV1` (`RunDirectory` management, checkpoint polling) |
| 10 | `factory_worker.rs` | **FactoryWorker** | Pluggable code generation backend (see below) |
| 11 | `validate.rs` | **Scenario Validator** | `FactoryOutputV1` + `ScenarioSetV1` → `SatisfactionResultV1` |
| 12 | `telemetry.rs` | **Telemetry Presenter** | Pipeline results → `TelemetryReport` + `ConsequenceCards` |
| 13 | `git.rs` | **Git Projection** | `FactoryOutputV1` → `GitCommitV1` (returns `StepError::GitNotAvailable` if git is not on PATH) |

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
| `mod.rs` | `CxdbEngine` trait; `InMemoryCxdbEngine` (testing/ephemeral); `TurnStore` trait; `StorageError` enum |
| `durable.rs` | `DurableCxdbEngine` — filesystem-backed MessagePack persistence with content-addressed blob store and WAL for crash recovery. **Not SQLite.** |
| `protocol.rs` | CXDB wire protocol types |
| `query.rs` | CXDB query engine |

Key properties of `DurableCxdbEngine`:
- **Serialization:** MessagePack (compact binary)
- **Addressing:** blake3 content hashing
- **Durability:** Write-ahead log (WAL) for crash recovery
- **Layout:** Filesystem blob store (not a relational database)

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
| `verification.rs` | Lean4 formal verification — proposition template generation from NLSpec |
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
| `app.rs` | App state model: `ChatMessage`, `PipelineStage`, `StageStatus`, `App` struct with `handle_key()`, `tick()`, pipeline event processing |
| `ui.rs` | Rendering: header, scrollable chat history, pipeline status bar, input box |
| `events.rs` | Crossterm event handler for `Key`, `Tick`, and `Resize` events |
| `pipeline.rs` | `PipelineEvent` enum, `spawn_pipeline()` — runs the real pipeline as a background tokio task, streams events via `mpsc::unbounded_channel` |

The TUI is wired to the real pipeline. On first user message, it spawns the pipeline in a background tokio task and renders live progress events (stage transitions, planner messages, completion) via the mpsc channel.

---

### planner-server

> **Role:** Axum-based HTTP server with Auth0 JWT authentication, REST API, WebSocket support, and static file serving for the React frontend.

| Module | Description |
|---|---|
| `main.rs` | Server setup; CLI args (`--port`, `--static-dir`); CORS configuration; `AppState` with `SessionStore` + `AuthConfig`; static file serving |
| `auth.rs` | Auth0 JWT middleware: `Claims` struct, `AuthConfig::from_env()`, `auth_middleware()` (validates Bearer token or query param `?token=`), `Claims` extractor for handlers. Dev mode bypasses auth with synthetic `dev\|local` claims |
| `api.rs` | REST endpoint handlers, split into public (health) and protected (auth-required) routes |
| `session.rs` | User-scoped `SessionStore` with `user_id` field. `create(user_id)`, `list_for_user(user_id)`, ownership enforcement |
| `ws.rs` | WebSocket message types + `handle_ws()` — real-time pipeline progress with 500ms polling, message forwarding, and client message handling |

#### REST API

| Method | Path | Auth | Description |
|---|---|---|---|
| `GET` | `/api/health` | Public | Health check |
| `GET` | `/api/models` | Protected | List available LLM models |
| `GET` | `/api/sessions` | Protected | List sessions for authenticated user |
| `POST` | `/api/sessions` | Protected | Create a new planning session |
| `GET` | `/api/sessions/:id` | Protected | Get session state (owner only) |
| `POST` | `/api/sessions/:id/message` | Protected | Send a message, spawns pipeline on first message |
| `GET` | `/api/sessions/:id/ws` | Protected | WebSocket for real-time updates |

#### Auth Modes

| Environment | Behavior |
|---|---|
| `AUTH0_DOMAIN` unset | Dev mode — no auth required, synthetic `dev\|local` user claims |
| `AUTH0_DOMAIN` set | Auth0 mode — JWT validation required on protected routes |
| `AUTH0_SECRET` set | HS256 validation with shared secret |
| `AUTH0_SECRET` unset | RS256 validation (production: use JWKS endpoint) |

---

### planner-web

> **Role:** React + TypeScript SPA serving the Socratic Lobby interface. Built with Vite, deployed as static files served by `planner-server`.

#### Tech Stack

- **Build tool:** Vite
- **Framework:** React 19 + TypeScript
- **Auth:** `@auth0/auth0-react` SDK
- **Routing:** `react-router-dom`
- **Styling:** Hand-written CSS (dark terminal theme, monospace fonts)

#### Project Structure

```
planner-web/
├── src/
│   ├── main.tsx                    # Entry point: ErrorBoundary + BrowserRouter + Auth0Provider
│   ├── App.tsx                     # Route definitions
│   ├── config.ts                   # Environment variables (Auth0, API base)
│   ├── types.ts                    # TypeScript types for Session, Message, Stage, WS messages
│   ├── index.css                   # Global styles (dark theme)
│   ├── auth/
│   │   ├── Auth0ProviderWithNavigate.tsx  # Auth0 provider with React Router integration
│   │   ├── ProtectedRoute.tsx             # Route guard for authenticated routes
│   │   └── useAuthenticatedFetch.ts       # Hook: attaches Bearer token to all API calls
│   ├── api/
│   │   └── client.ts              # Typed API client factory with token injection
│   ├── hooks/
│   │   └── useSessionWebSocket.ts # WebSocket hook with auto-reconnect + exponential backoff
│   ├── components/
│   │   ├── Layout.tsx             # Header, user info, logout, connection status
│   │   ├── ChatPanel.tsx          # Scrollable message list with role-based styling
│   │   ├── PipelineBar.tsx        # 12-stage pipeline visualization with status dots
│   │   └── MessageInput.tsx       # Input box with send button
│   └── pages/
│       ├── LoginPage.tsx          # Landing page with Auth0 sign-in
│       ├── Dashboard.tsx          # Session list + new session button
│       └── SessionPage.tsx        # Main chat + pipeline view + WebSocket integration
├── dist/                          # Build output (served by planner-server)
├── .env.example                   # Auth0 configuration template
├── vite.config.ts                 # Vite config: build to dist/, dev proxy to :3100
├── package.json
├── tsconfig.json
└── tsconfig.app.json
```

#### Routes

| Path | Component | Auth |
|---|---|---|
| `/` | `LoginPage` or `Dashboard` | Conditional |
| `/callback` | Auth0 callback handler | — |
| `/session/new` | `SessionPage` (creates new) | Protected |
| `/session/:id` | `SessionPage` (loads existing) | Protected |

#### Dev Mode

When `VITE_AUTH0_DOMAIN` is not set, the app runs without Auth0 — login page routes directly to session creation. This enables local development without an Auth0 tenant.

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

## Authentication

### Architecture

```
Browser → Auth0 Universal Login → JWT (access token)
  │
  ▼
React SPA (planner-web)
  │ Authorization: Bearer <jwt>
  ▼
Axum Server (planner-server)
  │ auth_middleware() validates JWT
  ▼
Claims { sub, email, ... } → user-scoped session access
```

### Flow

1. User visits the app → redirected to Auth0 Universal Login
2. Auth0 authenticates → redirects back with authorization code
3. `@auth0/auth0-react` SDK exchanges code for access token
4. All API calls include `Authorization: Bearer <token>` header
5. WebSocket connections pass token as `?token=<jwt>` query parameter
6. Server middleware validates JWT and extracts `Claims` (user ID = `sub` claim)
7. Sessions are scoped to users — each session has a `user_id` field

### Environment Variables

| Variable | Where | Description |
|---|---|---|
| `VITE_AUTH0_DOMAIN` | `planner-web` | Auth0 tenant domain (e.g., `your-tenant.us.auth0.com`) |
| `VITE_AUTH0_CLIENT_ID` | `planner-web` | Auth0 application client ID |
| `VITE_AUTH0_AUDIENCE` | `planner-web` | Auth0 API audience identifier |
| `AUTH0_DOMAIN` | `planner-server` | Same domain, for JWT issuer validation |
| `AUTH0_AUDIENCE` | `planner-server` | Same audience, for JWT audience validation |
| `AUTH0_SECRET` | `planner-server` | Optional: HS256 signing secret for dev/testing |

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

### 7. Auth0 with Dev Mode Bypass

Authentication uses Auth0 JWTs for production security while supporting a zero-config dev mode. When `AUTH0_DOMAIN` is not set, the server injects synthetic `dev|local` claims, and the React app skips Auth0 entirely. This means local development requires no Auth0 tenant, no environment variables, and no network access — you just run the server and open the browser.

### 8. Static SPA Served by Axum

The React frontend is built by Vite into static assets and served directly by the Axum server via `tower-http::services::ServeDir`. This eliminates the need for a separate frontend server in production, simplifies deployment to a single binary, and ensures the API and frontend share the same origin (no CORS issues in production).
