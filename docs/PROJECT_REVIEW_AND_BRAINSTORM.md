# Planner v2 — Project Review & Strategic Brainstorm

**Date:** March 5, 2026  
**Baseline Commit:** `f75202b` ("Implement deferred blueprint feature backlog")  
**Compilation:** Clean (zero warnings on `cargo check --workspace`)  
**Tests:** 618 Rust + 169 Frontend = **787 total, 0 failures**  
**TypeScript:** `tsc --noEmit` clean

---

## Part 1: Project Health Assessment

### Codebase Metrics

| Crate / Component | LOC | Tests | Role |
|---|---|---|---|
| `planner-schemas` | 4,218 | 29 | Type definitions, artifact registry |
| `planner-core` | 27,007 | 413 | Pipeline engine, LLM clients, storage, discovery |
| `planner-server` | 6,978 | 120 | Axum HTTP/WS server, API, auth |
| `planner-tui` | 3,941 | 56 | Ratatui terminal UI |
| `planner-web` (TS) | 14,143 | 169 | React SPA frontend |
| `planner-web` (CSS) | 1,420 | — | Styling |
| **Total** | **57,707** | **787** | |

### Largest Modules (Complexity Indicators)

| Module | LOC | Concern |
|---|---|---|
| `api.rs` | 3,247 | Server API — approaching monolith territory |
| `factory_worker.rs` | 2,412 | Code generation backend |
| `blueprint.rs` (core) | 1,413 | Blueprint store + events |
| `providers.rs` | 1,358 | LLM CLI clients |
| `pipeline/mod.rs` | 1,108 | Orchestration entry point |
| `BlueprintGraph.tsx` | 688 | D3 force graph component |
| `BlueprintPage.tsx` | 676 | Main blueprint page |
| `DetailDrawer.tsx` | 672 | Node detail sidebar |
| `discovery.rs` | 658 | Proposal scanners |

### Phase Completion Status

| Phase | Description | Status |
|---|---|---|
| Phase 0 | Core pipeline (12 stages), CLI, CXDB | **COMPLETE** |
| Phase 1 | Server (Axum, JWT, RBAC, rate limiting) | **COMPLETE** |
| Phase 2 | Web UI (React, Auth0, WebSocket chat) | **COMPLETE** |
| Blueprint A | Type alignment, CRUD endpoints, Create/Delete UI | **COMPLETE** |
| Blueprint B | Event sourcing (5 event types, persistence, API) | **COMPLETE** |
| Blueprint C | Detail editing, edge creation, Knowledge Library | **COMPLETE** |
| Blueprint D | Reconvergence engine (REST + WebSocket streaming) | **COMPLETE** |
| Blueprint E | Graph polish (pre-bake, minimap, focus mode) | **COMPLETE** |
| Blueprint F | Lifecycle (event timeline, diffs, stale/orphan detection) | **COMPLETE** |
| Blueprint G | Discovery (frontend + Rust scanners, proposal store) | **COMPLETE** |
| Blueprint H | TUI Blueprint Table (split-pane, navigation, detail) | **COMPLETE** |
| C.4 | JSON Merge Patch for partial PATCH | **COMPLETE** |
| C.5.7 | Documentation field on all node types | **COMPLETE** |
| D.3 | WebSocket streaming reconvergence | **COMPLETE** |

**All planned features from DEFERRED_RUST_FEATURES.md are implemented.** No open items remain from the original roadmap.

### Architecture Strengths

1. **Principled storage** — CXDB with blake3 content-addressing and MessagePack is a genuine differentiator. Content-addressed, immutable turns with parent chains give you audit/replay for free. This is better than what most production systems have.

2. **CLI-native LLM access** — Shelling out to `claude`, `gemini`, `codex` binaries is unorthodox but clever. Zero credential management, leverages existing subscriptions, degrades gracefully. No other system I've seen does this.

3. **Three-model adversarial review** — Using Opus + GPT-5.2 + Gemini in parallel for spec review is ahead of the curve. Anthropic's 2026 report explicitly calls out multi-agent quality control as a 2026 trend.

4. **Living Blueprint as reactive spec graph** — The core concept (typed dependency graph where editing any node triggers reconvergence) is novel. As stated in the spec: "No existing tool does this." That assessment holds.

5. **Full-stack test coverage** — 787 tests across Rust + frontend is solid for a system of this size and age. The test-to-LOC ratio (~1 test per 73 lines) indicates real coverage, not token gestures.

### Architecture Risks

1. **`api.rs` at 3,247 lines** — This file handles health, sessions, models, all blueprint CRUD, events, reconvergence, discovery, and WebSocket endpoints. It's becoming a god-module. Splitting into `api/blueprint.rs`, `api/discovery.rs`, `api/sessions.rs` would improve maintainability.

2. **No integration test for the end-to-end pipeline with real LLMs** — All pipeline tests use mocks. The system has never been tested against live `claude`/`gemini`/`codex` CLIs in an automated fashion. The first real user will be the first real test.

3. **Single-project limitation** — The Blueprint is a global singleton ("one per project: global singleton, OK for now"). Multi-project support will be necessary before this is useful for teams.

4. **No persistence for sessions across restarts** — Sessions live in memory (`SessionStore`). Server restart loses all active sessions. CXDB persists artifacts, but the session metadata (who's talking, what stage) does not survive.

5. **Socratic engine doesn't feed the Blueprint** — The Socratic interview extracts requirements, but those requirements don't automatically become Blueprint nodes. The two systems are decoupled — the interview produces artifacts that sit in CXDB, while Blueprint nodes are manually created or discovered. This is the biggest conceptual gap.

---

## Part 2: Product Vision Alignment

### Vision Statement (from spec)

> "Every parameter that influences a system's design and architecture must be visible, editable, and reactive. Editing any parameter triggers AI reconvergence of affected downstream artifacts."

### Where You Are vs. Where the Vision Points

| Vision Element | Current State | Gap |
|---|---|---|
| Parameters visible | All 6 node types with full CRUD, Knowledge Library, graph viz | **Met** |
| Parameters editable | Inline editing, merge patch, documentation field | **Met** |
| Parameters reactive | Reconvergence engine with impact preview + auto-apply | **Partially met** — reconvergence doesn't actually call LLMs yet. It applies severity policy but doesn't re-run pipeline stages. |
| Specification IS the system | Blueprint is a separate artifact from the generated code | **Not yet met** — The Blueprint describes the system but doesn't drive code generation. Editing a Technology node doesn't regenerate the Factory output. |
| AI reconvergence of artifacts | Reconvergence marks steps as done/pending but doesn't trigger real AI work | **Not yet met** — This is the hardest part. True reconvergence requires re-running pipeline stages selectively. |

### The Critical Gap

The pipeline runs forward (user prompt → 12 stages → git commit), and the Blueprint sits beside it as a knowledge system. The vision says they should be **the same thing** — the Blueprint should be the source of truth that drives the pipeline, not a parallel artifact.

Right now:
```
User prompt → Pipeline → Artifacts → (manually) → Blueprint
```

The vision says:
```
Blueprint (editable) → Change detected → Selective reconvergence → Regenerated artifacts
```

This is the "Terraform for system design" analogy from the spec, and it's the hardest remaining problem.

---

## Part 3: Frontier Research & Novel Ideas

### Idea 1: Blueprint-as-Spec — Close the Loop

**The problem:** The pipeline and Blueprint are decoupled.

**The pattern:** AWS Kiro's "Spec-First" workflow (released July 2025) validates the approach — spec → plan → tasks → code, with human gates. Anthropic's 2026 report confirms "specifications are becoming standard development artifacts" and "code as a derived artifact."

**Proposed architecture:**

```
Blueprint Nodes (source of truth)
    │
    ▼
Spec Compiler (new stage)
    │ Traverses the graph, resolves edges, produces a consolidated NLSpec
    │ from the current Blueprint state
    ▼
Diff Engine
    │ Compares new NLSpec against last-committed NLSpec (blake3 diff)
    │ Identifies which chunks/nodes changed
    ▼
Selective Re-pipeline
    │ Only re-runs stages for changed chunks
    │ Factory Worker regenerates only affected files
    ▼
Validation → Git Commit
```

This makes the Blueprint the actual input to the pipeline, not a side artifact. Editing a Decision node that affects Component X would trigger re-compilation of only X's chunk.

**Complexity:** High. Requires chunk-level dependency tracking and selective stage execution. But the chunk planner already exists, and CXDB already tracks parent chains.

### Idea 2: Knowledge Graph Memory Layer

**The insight:** From ZBrain, Beam.ai, and the 2026 knowledge graph research — the most effective agentic systems use a structured knowledge graph as shared memory across agents, not just vector embeddings.

**Your Blueprint IS already a knowledge graph.** Six node types, eight edge types, typed relationships. But it's only used for visualization and manual editing.

**Proposed enhancement:** Make the Blueprint queryable by the pipeline stages.

- The **Intake** stage could query the Blueprint: "What technologies are already adopted? What constraints exist?" — producing better IntakeV1 artifacts.
- The **AR Review** could query: "What decisions have been made about this component?" — catching contradictions.
- The **Factory Worker** could query: "What component structure exists?" — generating code that aligns with the architectural intent.

This turns the Blueprint from a passive record into an active context source. The LLM prompt for each stage would include relevant Blueprint context via graph traversal.

**Implementation:** Add a `blueprint_context(node_ids: &[String], depth: usize) -> String` method to BlueprintStore that serializes a subgraph to markdown. Inject it into pipeline prompts.

**Complexity:** Medium. The graph traversal exists. The prompt injection is straightforward. The hard part is knowing which nodes are relevant for which stage.

### Idea 3: MCP Server for External Tool Integration

**The trend:** Model Context Protocol (MCP) has become the de facto standard for LLM-to-tool integration by 2026. Anthropic, OpenAI, and Google all support it.

**Proposed implementation:** Expose the Planner Blueprint as an MCP server.

```rust
// planner-mcp/src/main.rs
// MCP server that exposes Blueprint as tools:
//   - list_blueprint_nodes(type_filter?) → NodeSummary[]
//   - get_node(id) → full node
//   - search_nodes(query) → matching nodes
//   - get_impact(node_id) → downstream impact
//   - propose_change(node_id, patch) → impact preview
```

This would allow any MCP-compatible client (Claude Desktop, Cursor, VS Code Copilot) to query and modify the Blueprint. A developer using Claude could say "what decisions affect the authentication component?" and get a structured answer from the live Blueprint graph.

**Complexity:** Medium. MCP is JSON-RPC over stdio or HTTP. The Blueprint API already exists — this is mostly a protocol adapter.

### Idea 4: Formal Verification Integration (Upgrade Lean4 Stubs)

**Current state:** `verification.rs` generates Lean4 proposition stubs from NLSpec. These are placeholder files — nobody runs them.

**The 2026 reality:** LLMs can now generate valid Lean4 proofs for 78% of standard theorems and 65% of safety-critical invariants (up from 25% in 2024). Tools like `llm-lean-bridge` allow real-time LLM-to-Lean kernel queries. The Z3 solver can be called during code generation.

**Proposed upgrade:**

1. Replace stub generation with actual invariant extraction — parse the NLSpec for safety properties ("must not exceed N concurrent connections", "all inputs sanitized before use").
2. Generate Z3 assertions (via z3-llm Python bindings) alongside factory code.
3. Add a verification gate after the Factory Worker — run Z3 on the generated assertions. If verification fails, feed the counterexample back to the Factory for regeneration.

This would make Planner the first agentic coding system with integrated formal verification in the loop, not as a post-hoc check.

**Complexity:** High. Z3 integration requires Python interop (could shell out like LLM clients). Invariant extraction from NLSpec is an LLM task. But the infrastructure pattern (shell out to tool, parse result, gate on pass/fail) already exists in the Factory Worker.

### Idea 5: Self-Healing Pipeline

**The pattern:** From NeurIPS 2024's "Self-Healing Machine Learning" paper — monitoring → diagnosis → adaptation → testing. The pipeline already has monitoring (telemetry stage) and testing (validation stage). It's missing diagnosis and adaptation.

**Proposed architecture:**

When validation fails (scenarios don't pass):
1. **Diagnosis agent** — A dedicated LLM call that receives: the failing scenario, the generated code, the NLSpec, and the error output. It produces a structured diagnosis: what went wrong, which spec requirement was violated, and a proposed fix strategy.
2. **Adaptation** — Based on the diagnosis, either:
   - Retry Factory Worker with an amended prompt (including the diagnosis)
   - Amend the NLSpec to resolve an inherent contradiction (flagged for human review)
   - Add a new constraint to the Blueprint ("this approach doesn't work because X")
3. **Retry loop** — Up to 3 attempts with exponential backoff, each informed by the previous failure.

Currently, a validation failure just reports the failure. This would make it try to fix itself.

**Complexity:** Medium. The retry loop is simple. The diagnosis agent is a new LLM call. The key question is how many retries before you bother the human.

### Idea 6: Multi-Project Blueprint with Cross-Project Edges

**The limitation:** One Blueprint per server instance. No way to model how Project A's decisions affect Project B.

**Proposed architecture:**

- `BlueprintStore` becomes `BlueprintRegistry` with multiple named blueprints
- Cross-project edges: `COMP-003@project-a depends_on TECH-007@project-b`
- Global technology radar: aggregated view of all technology nodes across projects, showing adoption posture per project
- Cross-project impact analysis: changing a shared technology shows impact across all projects that use it

This directly addresses the enterprise architecture use case. The 2026 EA tooling research (Gartner, Forrester) emphasizes "repository integrity" and "cross-domain impact assessment" as foundational capabilities.

**Complexity:** Medium-high. Mostly data model changes (namespace nodes by project) and UI changes (project selector, cross-project views).

### Idea 7: Continuous Specification Drift Detection

**The insight:** From the spec-driven development research — the hardest problem is keeping specs and code in sync. Planner has the unique advantage that both the spec (Blueprint) and the code (Factory output) are in the same system.

**Proposed feature:**

A background agent that periodically:
1. Scans the generated codebase (via the discovery scanners already built)
2. Compares discovered facts against Blueprint assertions
3. Flags drift: "Blueprint says we use Tokio 1.35, but Cargo.toml shows 1.38" or "Blueprint says Component X provides API Y, but the code doesn't export it"

This is the "living documentation" idea taken to its logical conclusion — the Blueprint isn't just documentation, it's a continuously verified assertion about the system.

**Complexity:** Medium. The discovery scanners already extract facts from Cargo.toml and directory structure. The comparison logic is new but straightforward.

### Idea 8: Sandboxed LLM Execution (Refactor --yolo/--dangerously-skip-permissions)

**Your own observation:** "We have a lot of instances using 'yolo' and 'dangerous' flags. This is an anti-pattern."

**Current state:** LLM CLI clients pass `--dangerously-skip-permissions` (Claude) and `--yolo` (Gemini) to bypass safety prompts.

**Proposed refactor:**

1. Run LLM CLI processes inside a sandboxed environment:
   - Use `unshare` (Linux namespaces) or `bubblewrap` to create a restricted filesystem
   - Mount only the worktree directory as writable
   - Block network access (LLM CLIs don't need it — they use local sockets)
   - Set resource limits (CPU time, memory)

2. Replace flag-based permission bypass with environment-based restriction:
   - Instead of "skip all permissions", give the LLM a sandbox where it CAN'T do damage even with full permissions
   - The LLM thinks it has full access; the sandbox constrains it

3. Add a `SandboxConfig` to `FactoryWorker`:
   ```rust
   pub struct SandboxConfig {
       pub writable_paths: Vec<PathBuf>,
       pub network_access: bool,
       pub max_cpu_seconds: u64,
       pub max_memory_mb: u64,
   }
   ```

This is the "principle of least privilege" applied to LLM code generation. It's how Codex's `--sandbox workspace-write` already works — but applied uniformly to all providers.

**Complexity:** Medium. Linux namespace isolation is well-understood. The main work is ensuring CLI tools function correctly inside the sandbox.

---

## Part 4: Prioritized Roadmap Recommendation

### Tier 1 — High Impact, Builds on Existing Infrastructure

| # | Idea | Why Now | Effort |
|---|---|---|---|
| 1 | **Blueprint-as-Context** (Idea 2) | Immediate value — pipeline stages become smarter with zero new infrastructure. Just prompt engineering + graph traversal. | 1-2 weeks |
| 2 | **Sandboxed LLM Execution** (Idea 8) | Removes a security anti-pattern. Makes the system production-safe. | 1 week |
| 3 | **Drift Detection** (Idea 7) | Discovery scanners already exist. Comparison logic is additive. Makes Blueprint trustworthy. | 1-2 weeks |

### Tier 2 — Strategic, Requires New Architecture

| # | Idea | Why Now | Effort |
|---|---|---|---|
| 4 | **Self-Healing Pipeline** (Idea 5) | Transforms validation failures from dead ends into retry loops. Biggest UX improvement. | 2-3 weeks |
| 5 | **Blueprint-as-Spec** (Idea 1) | The vision fulfillment. Hardest problem. Depends on chunk-level dependency tracking. | 4-6 weeks |
| 6 | **MCP Server** (Idea 3) | Opens the system to the entire MCP ecosystem. Strategic positioning. | 2 weeks |

### Tier 3 — Differentiation, Longer Horizon

| # | Idea | Why Now | Effort |
|---|---|---|---|
| 7 | **Multi-Project Blueprint** (Idea 6) | Enterprise readiness. Requires data model rework. | 4-6 weeks |
| 8 | **Formal Verification Loop** (Idea 4) | Frontier capability. No other agentic system has this. But the tooling (Z3, Lean4) is just barely mature enough. | 6-8 weeks |

### The One-Sentence Strategy

**Make the Blueprint the brain, not the notebook.** Right now it records what happened. It should drive what happens next.

---

## Part 5: Structural Recommendations (Non-Feature)

### 1. Split `api.rs`

3,247 lines is too much for one file. Recommended split:

```
planner-server/src/api/
├── mod.rs          # Router composition, shared types (ErrorResponse, etc.)
├── health.rs       # Health + models endpoints
├── sessions.rs     # Session CRUD + message handling
├── blueprint.rs    # Blueprint node/edge/event CRUD + reconvergence
├── discovery.rs    # Discovery scan, proposals, accept/reject
└── ws.rs           # WebSocket handlers (already partially separate)
```

### 2. Add a CI Pipeline

No CI exists. The Makefile is local-only. A GitHub Actions workflow that runs:
- `cargo check --workspace`
- `cargo test --workspace`
- `cargo clippy -- -D warnings`
- `cd planner-web && npx tsc --noEmit && npx vitest run`

...would catch regressions and make the green badge in the README truthful.

### 3. Connection Between Socratic and Blueprint

The Socratic interview produces `IntakeV1` (sacred anchors, satisfaction seeds) and `NLSpecV1` (requirements, components). These should auto-generate Blueprint nodes:

- Each "sacred anchor" → a Constraint node
- Each technology mentioned → a Technology node (Pending status)
- Each component in GraphDotV1 → a Component node
- Each design decision → a Decision node (Proposed status)

This is the "pipeline feeds Blueprint" direction. Combined with "Blueprint feeds pipeline" (Idea 1), you get the full bidirectional loop.

### 4. Consider `workspace-level` Cargo Lints

Currently zero warnings, but that's partly because `#[allow(dead_code)]` is implicit in some places. Adding workspace-level clippy configuration would enforce consistency:

```toml
# Cargo.toml
[workspace.lints.clippy]
all = "warn"
pedantic = { level = "warn", priority = -1 }
```

### 5. Document the Data Model

The 18 CXDB artifact types + 6 Blueprint node types + 8 edge types + 5 event types constitute a rich domain model, but there's no single reference document that shows them all with their relationships. An `ERD.md` or generated diagram would help onboarding.

---

## Appendix: Research Sources

- [Anthropic 2026 Agentic Coding Trends Report](https://resources.anthropic.com/hubfs/2026%20Agentic%20Coding%20Trends%20Report.pdf) — Multi-agent orchestration, human oversight scaling, task horizon expansion
- [AWS Kiro Spec-First Analysis](https://dev.to/kirodotdev/the-paradigm-shift-from-reactive-to-proactive-ai-in-software-development-a-comparative-analysis-of-148p) — Specification-driven approach reduces logic errors 23-37%
- [Formal Verification with LLMs (AptiCode)](https://apticode.in/blogs/formal-verification-with-llms-automating-proof-synthesis-in-2026) — LLMs achieve 78% valid Lean4 proofs, Z3 integration via APIs
- [Knowledge Graphs for Agentic AI (ZBrain)](https://zbrain.ai/knowledge-graphs-for-agentic-ai/) — KGs as shared memory for multi-agent collaboration
- [Enterprise Architecture Tooling 2026](https://digitalmehmet.com/2026/03/04/enterprise-architecture-tooling-in-2026/) — Repository integrity, cross-domain impact, AI-assisted documentation
- [MCP: Standard for Agentic Integration (Anthropic)](https://www.anthropic.com/news/model-context-protocol) — Universal protocol for LLM-to-tool connections
- [Spec-Driven Development (arXiv)](https://arxiv.org/html/2602.00180v1) — Spec-first → spec-anchored → spec-as-source progression
- [Self-Healing ML (NeurIPS 2024)](https://proceedings.neurips.cc/paper_files/paper/2024/file/4a86ec12e94ef1fe306362e7bdcd5894-Paper-Conference.pdf) — Monitor → diagnose → adapt → test framework
- [Reactive Links for Change Propagation (PMC)](https://pmc.ncbi.nlm.nih.gov/articles/PMC12289748/) — Fine-grained constraint propagation across engineering artifacts
- [METR 2025 RCT / Baytech Analysis](https://www.baytechconsulting.com/blog/mastering-ai-code-revolution-2026) — AI coding review burden, senior developer productivity paradox
