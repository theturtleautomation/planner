# planner-core Audit Report
**Date:** 2026-02-28  
**Scope:** `/home/user/workspace/planner/planner-core/`  
**Auditor:** Automated code review (all source files read in full)

---

## File-by-File Assessment

### `src/lib.rs`
**Rating: Real (minimal re-export module)**  
Four `pub mod` declarations. No logic. Correctly re-exports `cxdb`, `dtu`, `llm`, and `pipeline` for integration tests and downstream consumers. Comment on line 7 ("Phase 0: Many types and functions are built for the full pipeline but not yet wired into main.rs") is accurate but slightly outdated — main.rs does now wire the full pipeline.

---

### `src/main.rs`
**Rating: Real (functional binary entry point)**  
Real CLI argument parsing, real `LlmRouter::from_env()` check, real dispatch to either `run_full_pipeline` or `run_phase0_front_office`. Output formatting reads real artifact fields. No TODO comments. The binary is a legitimate orchestrator. Two gaps:
- The `--full` logic has a subtle tautology: the flag is set to `true` whenever `!front_office_only`, so `full_mode` can never be `true` AND `front_office_only` can never be `true` simultaneously — the full-pipeline branch is always taken when no `--fo` flag is passed. This is probably intentional but the condition `if full_mode && !front_office_only` is redundant.
- No `--help` output describes the `--full` flag as "default" even though it is.

---

### `src/llm/mod.rs`
**Rating: Real (trait + catalog)**  
Defines the core `LlmClient` trait, `CompletionRequest`, `CompletionResponse`, `LlmError`, and the `MODELS` catalog. Model IDs match the system design doc (claude-opus-4-6, gemini-3.1-pro, gpt-5.3-codex, etc.). `DefaultModels` constants are complete for all pipeline components. No stubs, no TODOs.

---

### `src/llm/providers.rs`
**Rating: Real — but with important behavioral caveats (see LLM Client Assessment)**  
Full CLI-shelling implementations for Anthropic, Google, and OpenAI. Real `run_cli` helper with timeout, stdin piping, and proper error classification. Tests are meaningful. See dedicated section below for deep analysis.

---

### `src/pipeline/mod.rs`
**Rating: Real (complete pipeline orchestration)**  
`run_full_pipeline` genuinely chains all layers: Front Office (11 steps) → Factory Worker → Validation Loop (up to 3 retries) → Telemetry → Git. `run_phase0_front_office_with_config` executes 11 sequenced steps with logging. `PipelineConfig` is a thin but real configuration bundle. `Recipe::phase0()` defines a 17-step DAG. The Recipe struct is defined but **not used** by `run_full_pipeline` — the pipeline executes steps directly in code, not by interpreting the recipe. This is a design gap (recipe is documentation, not execution).

---

### `src/pipeline/steps/mod.rs`
**Rating: Real**  
Declares all 13 step modules, defines `StepResult<T>`, and `StepError` enum covering all real failure modes. `From<LlmError>` impl is correct.

---

### `src/pipeline/steps/intake.rs`
**Rating: Real (full implementation)**  
Constructs a real prompt with a detailed schema, calls `router.complete()`, parses the JSON response into `IntakeV1`, handles code fences, maps domain/variant enums. Conversation log is sparse (only 2 entries for a "non-interactive" Phase 0 mode — acceptable given the design intent). Tests cover valid JSON, code fences, and malformed responses.

**Critical note:** The system prompt instructs the model to respond with only JSON but includes no JSON repair/retry logic. If the model preambles the JSON (common), the entire call fails with a `JsonError`. Only `strip_code_fences` provides any robustness.

---

### `src/pipeline/steps/chunk_planner.rs`
**Rating: Real**  
Heuristic-first design: MicroTools always return a single root chunk without an LLM call (correct shortcut). Complex projects call an LLM with a real decomposition prompt. Parsed `ChunkPlan` validates that root is first and every sacred anchor is covered (with a warning, not an error, for uncovered anchors). Tests cover valid/invalid/empty chunk plans.

---

### `src/pipeline/steps/compile.rs`
**Rating: Real (most complex step — ~1145 lines)**  
Implements `compile_spec`, `compile_spec_multichunk`, `compile_graph_dot`, `compile_graph_dot_multichunk`, `generate_scenarios`, and `compile_agents_manifest`. Each step:
- Constructs a real, detailed system prompt
- Uses `context_pack` to budget the input
- Calls the LLM
- Parses the full response into typed structs with full field mapping

`parse_spec_response` maps all 12 NLSpec fields, including priority enums, traces_to arrays, and open questions. `parse_graph_dot_response` produces a real `GraphDotV1` with model routing assignments. Tests cover valid JSON, code fences, and domain chunk parsing. No stubs. `line_count` estimation (`content.len() / 60`) is a rough heuristic but acceptable.

---

### `src/pipeline/steps/linter.rs`
**Rating: Real (deterministic, no LLM)**  
Implements all 12 linting rules deterministically:
1. Root required sections present
2. DoD non-empty
3. Sacred anchor → FR traceability
4. FR imperative language check
5. Open questions resolved
6. Phase 1 Contracts non-empty in root
7. DTU priority check (exists but is a no-op — `let _ = &dep.dtu_priority;` does nothing)
8. Chunk ≤500 lines
9. FR ID format (FR-N prefix)
10. Out of Scope non-empty
11. At least one critical satisfaction criterion
12. Amendment log structural check (comment says "just verify log exists")

**Gap:** Rule 7 is a dead check — it accesses the field but performs no validation. Rule 12 is not implemented (comment admits "no previous version to compare"). `lint_spec_set` adds 5 cross-chunk rules (9a–9e). Tests are comprehensive and test real rule behavior.

---

### `src/pipeline/steps/context_pack.rs`
**Rating: Real**  
Token-budgeted context assembly. Priority-ordered sections, truncation for must-include sections, skip for lower-priority sections. Token estimation uses 4 chars/token heuristic (conservative, intentionally). `render_context_pack` produces clean markdown headers. Tests verify truncation behavior, priority ordering, and rendering. No stubs.

---

### `src/pipeline/steps/ar.rs`
**Rating: Real**  
Three real reviewer prompts with distinct lenses (Intent, Implementability, Scope). `execute_adversarial_review` calls three LLMs sequentially (not in parallel — "Phase 0/1/2 — sequential for simplicity" is the comment). Each reviewer has 1 retry on parse failure. Finding IDs are assigned after merge. `execute_adversarial_review_set` loops over chunks. `execute_cross_chunk_coherence_review` is a fourth pass using Opus. Finding parsing is robust with `serde(default)` on all fields. Tests verify parsing, code fences, severity defaults, and `recalculate()`.

**Gap:** Reviewers run sequentially even though the comment says Phase 3 can parallelize with `tokio::join!`. No parallel execution is implemented.

---

### `src/pipeline/steps/ar_refinement.rs`
**Rating: Partial — amendment application has limited coverage**  
The refinement loop structure is real: calls LLM, parses amendments, applies them, re-lints, loops up to 3 times. `parse_refinement_response` is real. `apply_amendment` handles: requirements (modify/add), DoD (modify/add by DOD-N index), architectural_constraints (add), out_of_scope (add), satisfaction_criteria (add).

**Critical gaps:**
- `remove` action is not implemented for any section (silently ignored)
- When re-lint fails, `blocking_findings` is cleared but never repopulated from the lint violations — the next iteration of the loop runs without any findings to address, making iteration 2+ of a lint-failure scenario effectively a no-op
- "FR traces_to" cannot be amended — adding a requirement always sets `traces_to: vec![]`, which will fail Rule 3 on re-lint
- `generate_oq_consequence_cards` is real and correctly produces `ConsequenceCardV1` objects

---

### `src/pipeline/steps/ralph.rs`
**Rating: Real (~1272 lines, substantial implementation)**  
Three modes fully implemented:
1. **ScenarioAugmentation**: real LLM call with edge-case focus prompt, real scenario parsing
2. **GeneTransfusion**: deterministic keyword-matching against 6 known patterns (auth, payment, file-upload, api, database, realtime) — no LLM call. Reasonably clever pitfall matching with a "2 of 3 keywords covered" threshold
3. **DTU Configuration**: deterministic configs for Stripe and Auth0 with real behavioral rules, state transitions, seed state, failure modes. Unknown providers are skipped (silently). LLM-based config generation exists for complex cases

ConsequenceCard generation only surfaces High-severity findings. Tests are thorough including Stripe RBAC detection, payment rule conditionals, and priority filtering.

**Gap:** Only 5 providers in `DTU_PROVIDER_MAP` (stripe, auth0, sendgrid, supabase, twilio). Unknown providers silently return empty configs from `generate_default_dtu_config` even when their priority is High.

---

### `src/pipeline/steps/factory.rs`
**Rating: Real**  
`render_nlspec_markdown` produces a valid YAML-frontmatted markdown representation of the NLSpec. `execute_factory_with_worker` prepares a real worktree, builds a real prompt, delegates to the worker, and builds `FactoryOutputV1` from the result. `dod_results` is always `vec![]` in `build_factory_output_from_worker` — DoD checking is deferred to `validate::check_definition_of_done`. Tests use `MockFactoryWorker`. `run_budget_usd` is read from `GraphDotV1` but not enforced in the factory step itself.

---

### `src/pipeline/steps/factory_worker.rs`
**Rating: Real (CodexFactoryWorker does the right thing; MockFactoryWorker is clean)**  
`WorktreeManager::prepare` is real filesystem I/O — creates dirs, writes SPEC.md, graph.dot, AGENTS.md. `CodexFactoryWorker::generate` actually invokes `codex exec --json --sandbox workspace-write -m <model> -C <worktree> <prompt>` and parses the JSON response. `scan_worktree_files` recursively walks the worktree excluding `.planner-context`. `MockFactoryWorker` returns deterministic success/failure for tests.

**Critical gap:** `CodexFactoryWorker` always sets `success: true` in `WorkerResult` even when `codex exec` returns a non-empty stderr, because `run_cli` only returns an error on non-zero exit codes. If `codex exec` exits 0 but generates broken code, the factory reports `BuildStatus::Success`. There is no verification that the generated code actually compiles or passes tests.

---

### `src/pipeline/steps/validate.rs`
**Rating: Partial — structurally real but validation is shallow**  
`execute_scenario_validation` runs each scenario 3 times against the factory output using Gemini. Gate thresholds are correct (100% critical, 95% high, 90% medium). Retry logic on parse failure (2 retries) is real.

**Critical structural gap:** The evaluator receives `factory_output.output_path` (a filesystem path string) but **cannot actually read or execute the code**. The evaluation prompt sends the scenario BDD text and the `output_path` string to Gemini, which must infer pass/fail purely from the build status and node names — it cannot run the app or inspect files. This means scenario validation is effectively asking Gemini "does this build output sound like it would satisfy this scenario?" based on metadata alone, not actual behavioral testing.

`check_definition_of_done` is deterministic keyword-matching (build/compile, test/scenario/pass, persist/save/store) against `factory_output.build_status` and `satisfaction.gates_passed`. This is a reasonable heuristic but not a real mechanical check.

---

### `src/pipeline/steps/telemetry.rs`
**Rating: Real**  
`execute_telemetry_presentation` calls Claude Haiku with a prompt instructing it to produce a plain-English summary. Response parsing into `TelemetryLlmReport` is real. Consequence card generation for budget warning/exhaustion and critical/high gate failures is deterministic and real. `build_telemetry_report_deterministic` is a fully functional LLM-free fallback. Tests cover both paths.

---

### `src/pipeline/steps/git.rs`
**Rating: Real (actual git subprocess calls)**  
`execute_git_projection` shells out to real `git` commands: `git init`, `git config user.name/email`, `git add -A`, `git commit -m ... --allow-empty`, `git rev-parse HEAD`, `git diff --name-only HEAD~1 HEAD`. Handles "nothing to commit" gracefully. Falls back to `git ls-files` for first-commit file listing. Test actually runs git commands in a temp directory. No stubs.

**Note:** `FileChangeType` is always `Added` for all files, even on subsequent commits where files may be modified. The type information is cosmetic and wrong.

---

### `src/cxdb/mod.rs`
**Rating: Real (in-memory CXDB engine, full implementation)**  
`CxdbEngine` implements `TurnStore` with real `RwLock<HashMap>` storage. Content-addressed deduplication via BLAKE3 hash (the hash is computed externally in `planner_schemas::Turn::new` before reaching here). Blob size validation, run-type index, project-run index are all real. `store_turn_internal` properly locks, validates, stores blob and metadata, updates both indexes. `reconstruct_turn` uses `rmp_serde` (MessagePack) deserialization. Tests cover roundtrip, dedup, not-found, blob size limits, parent-child relationships.

**Gap:** `store_turn` passes `project_id: None` — project IDs are never populated in the CxdbEngine via the pipeline. Project-level indexing (`register_run`, `list_runs`) works if called directly but the pipeline never calls it.

---

### `src/cxdb/durable.rs`
**Rating: Real (filesystem-backed, production-grade for a v0)**  
`DurableCxdbEngine::open` creates the directory tree. `write_blob` uses content-addressed paths (`blobs/ab/cd/abcdef...`) with directory creation. `write_turn_record` writes MessagePack to `turns/<run_id>/<type_id>/<turn_id>.msgpack`. `rebuild_indices` walks the full directory tree on startup to rebuild in-memory indices. `register_run` writes project→runs list to `projects/<project_id>.msgpack`. `list_runs` reads from disk.

**Concurrent access:** Write operations are serialized via `write_lock: RwLock<()>` — this is advisory within a single process. No cross-process file locking (no flock/lockfile). Safe for single-process use; unsafe if two processes write to the same CXDB root simultaneously.

**WAL recovery:** No WAL. If a write to the blob file succeeds but the turn metadata write fails (e.g., crash between the two writes), the blob is orphaned and the turn is lost silently. Not a concern for the current scale but worth noting.

Tests cover: roundtrip, persistence across re-opens (simulated process restart), dedup on disk, project runs, empty run.

---

### `src/cxdb/protocol.rs`
**Rating: Real**  
`Frame::encode/decode` implement a real 5-byte-header (4 length + 1 type) binary protocol. `StoreTurnMessage` and `StoreTurnAck` are real MessagePack-serializable structs. `MAX_FRAME_SIZE` enforcement is real. Tests cover roundtrip, insufficient data, unknown type, frame too large.

**Gap:** This protocol is defined but **there is no TCP server implementation** in the codebase. No listener, no connection handling, no client. The protocol is well-designed scaffolding for a future TCP server.

---

### `src/cxdb/query.rs`
**Rating: Partial (types real, execution not implemented)**  
`CxdbQuery` enum and `QueryResult` struct are well-defined. Route URL constants are defined. `QueryResult::empty/single/paginated` constructors are real.

**Gap:** There is no HTTP server. The `routes` module is documentation. No query execution engine is implemented — `CxdbQuery` cannot be executed against `CxdbEngine` or `DurableCxdbEngine`. This is complete scaffolding for a future read API.

---

### `src/dtu/mod.rs`
**Rating: Real**  
`DtuProvider` trait, `DtuRegistry`, routing logic, and `reset_all` / `apply_configs` are all real. `with_phase4_defaults` and `with_phase5_defaults` constructors work correctly. Tests verify provider presence and routing errors.

---

### `src/dtu/stripe.rs`, `auth0.rs`, `sendgrid.rs`, `supabase.rs`, `twilio.rs`
**Rating: Real (substantial behavioral clone implementations)**  
Each DTU implements `DtuProvider` with real endpoint routing, stateful in-memory entity stores (using `Mutex<HashMap>`), state machine transitions, failure mode injection, and `apply_config` for Ralph-generated configs. All use `serde_json` for request/response bodies.

Stripe: handles `/v1/customers`, `/v1/payment_intents`, confirm/capture/cancel, refunds, state machine (requires_payment_method → requires_confirmation → requires_capture → succeeded/canceled). Auth0 (27k bytes) is the most comprehensive, handling token grants, user CRUD, role assignment, rate limiting.

These DTUs are genuinely useful behavioral clones for testing — they would meaningfully catch real integration bugs if wired into the validation step.

**Gap:** The scenario validator (`validate.rs`) never routes requests through the `DtuRegistry`. `dtu_deps` in `Scenario` are parsed and stored but never executed during validation. DTUs are defined and tested in isolation but never used during the pipeline's actual scenario evaluation.

---

### `src/pipeline/verification.rs`
**Rating: Partial (generates stubs, not proofs)**  
`generate_propositions` creates Lean4 theorem _stubs_ with `sorry` placeholders — this is by design ("Replace `sorry` with actual proofs"). The stubs are syntactically well-formed Lean4. No `lean` binary is invoked. This is intentional scaffolding for a formal methods workflow.

**Key limitation:** All propositions have `sorry` — they assert things but prove nothing. The value is in surfacing _what_ should be proven, not in actually proving it.

---

### `src/pipeline/audit.rs`
**Rating: Real (deterministic, useful)**  
Keyword-matching against 7 vendor patterns. Flags migration complexity (auth0, firebase, aws), data export issues (sendgrid, twilio), cost escalation (firebase per-read, aws per-resource). Checks for abstraction layer in architectural constraints. Single-vendor category detection. Risk scoring and recommendations are deterministic. Tests cover all vendor patterns and the abstraction constraint bypass.

---

### `src/pipeline/pyramid.rs`
**Rating: Partial (builder exists, LLM aggregation not wired)**  
`PyramidBuilder` exists with `build_leaves` (deterministic truncation), `build_branches` (aggregates leaf summaries), and `build_root` (LLM call for high-level summary). However:
- `PyramidBuilder` is never called from the pipeline
- `build_branches` and `build_root` use LLM calls but have no integration with CXDB
- This is a complete design with a reasonable implementation, but it's not connected to anything in the actual pipeline execution

---

### `src/pipeline/project.rs`
**Rating: Real (but unused in pipeline)**  
`ProjectRegistry` implements register, get, get_by_slug, list, count, update_status, and increment_run_count. Duplicate slug validation works. In-memory only (no persistence). Never called from `run_full_pipeline` or `run_phase0_front_office`.

---

### `tests/integration_e2e.rs`
**Rating: See dedicated Test Quality section below**

---

## LLM Client Layer Assessment

### `AnthropicCliClient`
**Invocation:** `claude -p --dangerously-skip-permissions --output-format stream-json --verbose --model <model>`  
**Prompt delivery:** Via stdin (correct — avoids shell quoting nightmares)  
**Streaming parse:** Iterates lines, tries `serde_json::from_str::<ClaudeResult>` on each. Takes the _last_ line that parses successfully with a non-empty `result` field. This is correct — Claude's stream-json puts the final result in the last result-bearing event.

**Error handling when CLI fails:**
- Non-zero exit → `LlmError::CliExecError` with stderr (or stdout if stderr empty)
- Binary not found → `LlmError::CliBinaryNotFound`
- Timeout → `LlmError::Timeout`
- Empty content after all lines parsed → fallback to `stdout.trim()` as the response

**Structural risk:** `ClaudeResult` and `ClaudeStreamEvent` are separate structs but the parser only uses `ClaudeResult`. If Claude's actual stream-json format uses different field names in the final `result` block, the parser silently produces an empty `content`. The fallback (`stdout.trim()`) would then include all the raw stream-json lines, which would cascade as a JSON parse failure at the step level.

**Token reporting:** Works if Claude CLI emits `input_tokens` / `output_tokens` in the result block.

### `GoogleCliClient`
**Invocation:** `gemini -p --output-format stream-json --yolo --model <model>`  
**Same streaming approach as Anthropic.** `GeminiStreamEvent` is simpler than `ClaudeResult` — no `cost_usd`. Same fallback logic.

**Risk:** The `--yolo` flag skips Gemini's safety confirmations. This is intentional for agentic use but means the client cannot detect safety-blocked responses (they would appear as empty content or non-JSON output).

### `OpenAiCliClient`
**Invocation:** `codex exec --json --sandbox workspace-write -m <model> <prompt>`  
**Key difference:** Prompt is passed as a positional argument (not stdin), which means it's subject to shell argument length limits and potential quoting issues for very long prompts.

**Response parse:** Tries `CodexExecResponse` with three optional fields (`output`, `result`, `response`). Falls back to raw stdout. Token counts are always 0.

**No worktree flag for plain LLM calls:** When used for non-factory calls (e.g., AR review), there's no `-C worktree` flag — the codex CLI operates in whatever directory it's invoked from. This is probably fine for text-only tasks but inconsistent with `CodexFactoryWorker` which uses `-C`.

### `LlmRouter`
Model routing by prefix (`claude-` → anthropic, `gemini-` → google, `gpt-` → openai). Unknown models default to Anthropic. Clean and correct.

### Critical Unanswered Questions
1. **Claude's actual stream-json format**: The `result` field in `ClaudeStreamEvent`/`ClaudeResult` assumes Claude CLI emits `{"type": "result", "result": "<full text>", ...}`. If the actual format differs (e.g., content deltas with `delta.text` or a different top-level key), the parser silently produces empty responses.
2. **Gemini stream-json format**: Same concern — the `result` field assumption is unverified.
3. **codex exec JSON schema**: The `CodexExecResponse` tries three field names (`output`, `result`, `response`) — clearly a guess. If the actual schema is different, every codex-based step silently returns raw stdout.

---

## Pipeline Assessment

### Can `run_full_pipeline` actually run end-to-end?

**Yes, for the happy path — with real CLIs installed.**

The pipeline correctly chains:
1. Intake (LLM call → IntakeV1)
2. Chunk Plan (heuristic or LLM → ChunkPlan)
3. Compile Spec(s) (LLM → NLSpecV1)
4. Lint (deterministic)
5. AR Review (3 LLM calls → ArReportV1)
6. AR Refinement (LLM if blocking, up to 3 iterations)
7. Scenario Generation (LLM)
8. Ralph Loop (1 LLM call + 2 deterministic passes)
9. Compile GraphDot (LLM)
10. Compile AGENTS.md (LLM)
11. Formal Verification propositions (deterministic)
12. Anti-lock-in audit (deterministic)
13. Factory Worker (codex exec)
14. Scenario Validation (Gemini, 3 runs per scenario)
15. Telemetry Presentation (Haiku)
16. Git Projection (real git)

**Structural concerns:**

- **Recipe vs execution:** `Recipe::phase0()` defines a 17-step DAG but `run_full_pipeline` executes steps directly in imperative code. The recipe is documentation, not an interpreter. Step ordering changes to the recipe don't affect execution.
- **Retry loop:** The factory→validation retry loop (up to 3 attempts) is correctly implemented. Budget check (`can_proceed`) is checked between retries.
- **Storage:** `PipelineConfig::minimal()` sets `store: None` and `dtu_registry: None`. If storage is wired, only `Intake` is persisted via `config.persist()` — other artifacts are not persisted even when storage is available.
- **DTU clones:** `dtu_reg.reset_all()` is called between validation attempts but the DTUs are never injected into the validation step itself.

---

## CXDB Assessment

### Is durable storage production-ready?

**For single-process single-tenant use: Yes, mostly.**

`DurableCxdbEngine`:
- Real filesystem storage with content-addressed blobs
- Real MessagePack serialization (not JSON text files)
- Correct index rebuild on startup (handles process restart)
- Content-addressed deduplication works and is tested
- Within-process write serialization via `RwLock`

**Production gaps:**
1. **No cross-process locking:** Two processes writing to the same root concurrently will corrupt the project index files (read-modify-write without a file lock). Acceptable for single-process use.
2. **No WAL:** Crash between blob write and metadata write orphans the blob. Turn is lost silently.
3. **No compaction:** The `compaction_threshold` config field exists but compaction is never triggered.
4. **Index rebuild is O(all turns):** On restart, `rebuild_indices` walks the entire `turns/` directory tree. For large deployments this could be slow.
5. **No encryption at rest:** Blobs are raw MessagePack on disk.
6. **HTTP read API not implemented:** `query.rs` defines the types and route strings but there is no web server. The "HTTP reads" described in the architecture diagram don't exist.
7. **TCP binary protocol not implemented:** `protocol.rs` defines the wire format but there is no TCP listener.

The in-memory `CxdbEngine` is fully functional for in-process use.

---

## Test Quality Assessment

### Integration tests (`tests/integration_e2e.rs`)
**Total: ~2280 lines, ~30 test functions**

**Tests that exercise real behavior:**
- `e2e_phase0_pipeline_simulation`: Real worktree creation (filesystem I/O), real git operations, real linter, real budget tracking, real telemetry deterministic path. Factory uses `MockFactoryWorker`. **This test catches real regressions in worktree, git, linter, and telemetry.**
- `e2e_phase0_pipeline_failure_path`: Tests failure propagation through telemetry.
- `e2e_phase0_budget_exhaustion`: Tests `BudgetStatus::Warning` and `HardStop` transitions.
- `e2e_phase1_multi_tier_gate_evaluation`: Tests all boundary conditions on gate thresholds (0.95, 0.94, 0.90, 0.89). **Genuinely rigorous.**
- `e2e_phase1_dod_checker_integration`: Tests DoD mechanical checker with both success and failure states.
- `e2e_phase3_lint_spec_set_*`: Multi-chunk lint validation with real duplicate FR IDs and orphaned Sacred Anchor detection. **Real regression coverage.**
- `e2e_phase3_chunk_planner_microtool_single_chunk`: Actually calls `plan_chunks` with a real `LlmRouter::from_env()` — no LLM call needed for MicroTool, so this passes without CLIs.
- `e2e_storage_turn_lifecycle`: Real MessagePack roundtrip via `CxdbEngine`.
- `e2e_phase6_durable_cxdb_*`: Real filesystem I/O across engine re-opens, dedup on disk.
- `e2e_phase6_dtu_registry_wired`: Verifies all 5 Phase 5 DTU providers are present and `reset_all` doesn't panic.
- `e2e_phase6_project_registry`: Real duplicate-slug rejection.
- `e2e_phase6_verification_propositions`, `e2e_phase6_audit_lock_in`: Real proposition generation and audit finding detection.
- Ralph gene transfusion tests: Real pattern matching behavior.
- Context pack tests: Real budget truncation and priority ordering.

**Tests that are trivial assertions:**
- `e2e_phase2_ar_severity_classification`: Tests `ArSeverity::Blocking == ArSeverity::Blocking` and `ArReviewer::Opus != ArReviewer::Gpt`. Zero regression value.
- `e2e_phase2_refinement_no_blocking_passthrough`: Asserts `!report.has_blocking` on a manually constructed report. Proves nothing about the actual refinement logic.
- `e2e_phase3_ar_report_per_chunk`: Manually constructs two `ArReportV1` structs and asserts their `chunk_name` fields. Tests struct construction, not pipeline behavior.
- `e2e_phase3_recipe_includes_ar_steps` and `e2e_phase3_recipe_includes_new_steps`: Assert step names exist in the recipe. Does not test that the pipeline actually executes those steps.

**What tests do NOT cover:**
- Any real LLM calls (all LLM-dependent steps are bypassed via canned data or mock workers)
- `execute_adversarial_review` end-to-end with a real LLM
- `execute_scenario_validation` with real Gemini evaluation
- `execute_ar_refinement` with blocking findings that require actual amendment
- The complete `run_full_pipeline` from top to bottom (no single test exercises all 16 steps)
- Error propagation when an LLM call returns malformed JSON mid-pipeline
- The TCP binary protocol
- The HTTP read API

**Verdict:** Tests catch real regressions in deterministic/structural components (linter, git, worktree, CXDB, budget). They do not and cannot catch regressions in LLM-dependent behavior (prompt engineering, response parsing, multi-model coordination) without real LLM calls.

---

## Critical Gaps

### 1. Scenario Validation Cannot See Code
`validate.rs` sends the scenario BDD text and `output_path` string to Gemini. Gemini must evaluate "pass/fail" without reading or running the generated code. This is not behavioral validation — it's asking an LLM to guess whether code at a filesystem path probably satisfies a scenario. The 3-run majority-vote mechanism adds apparent rigor to a fundamentally shallow check.

### 2. DTU Clones Are Never Used During Validation
Five production-quality DTU behavioral clones (Stripe, Auth0, SendGrid, Supabase, Twilio) exist. Scenarios have `dtu_deps` fields. The validation step never routes requests through `DtuRegistry`. The DTUs are tested in isolation but never participate in the actual pipeline.

### 3. AR Refinement's Re-Lint Loop Is Broken on Lint Failure
When re-linting after an amendment iteration fails with `LintFailure`, the blocking findings vector is emptied without being repopulated. The next iteration runs with no findings to address, making multi-iteration lint-driven refinement a no-op after the first iteration.

### 4. AR Reviewers Are Sequential, Not Parallel
The design doc says "Phase 3 can parallelize with tokio::join!" but it remains sequential. Three sequential LLM calls (each up to 5 minutes) for a single AR pass is the bottleneck on long runs.

### 5. The Recipe Is Dead Code
`Recipe::phase0()` defines a 17-step DAG with dependencies but `run_full_pipeline` is a hard-coded imperative sequence. Changes to the Recipe have no effect on execution.

### 6. CXDB Only Persists Intake
`config.persist()` is called only after the Intake step. All other 15 artifacts (NLSpec, GraphDot, ScenarioSet, AgentsManifest, ArReports, FactoryOutput, etc.) are never persisted even when a `DurableCxdbEngine` is configured. The "durable storage" feature is largely inert during a normal pipeline run.

### 7. TCP and HTTP APIs Are Not Implemented
The architecture diagram shows "TCP binary protocol" for writes and "HTTP read API" for queries. `protocol.rs` has the wire format. `query.rs` has the types. There are no servers. `CxdbEngine` can only be accessed in-process.

### 8. CodexFactoryWorker Always Reports Success
If `codex exec` exits 0 (even on partial output), `success: true` is returned. There is no verification that the generated code compiles, type-checks, or passes tests. The pipeline would report `BuildStatus::Success` for code that is syntactically invalid.

### 9. OpenAI CLI Prompt via Positional Arg
`OpenAiCliClient` passes the full prompt as a positional argument to `codex exec`. For large multi-context prompts (spec + graph + agents = potentially thousands of chars), this will hit OS argument length limits or fail due to special character quoting. Anthropic and Google use stdin instead.

### 10. No JSON Repair on LLM Response Parse Failures
Every step has a single `serde_json::from_str` attempt plus optional code fence stripping. No retry with a "please output valid JSON" follow-up, no partial recovery, no schema repair. A single model hallucination (e.g., embedding markdown inside the JSON) fails the entire step.

---

## Honest Summary

planner-core is a carefully structured, professionally written codebase that is **substantially real** — not scaffolding. The LLM client layer correctly shells to native CLIs. The pipeline orchestration chains 16 real steps. The linter, context packs, Ralph gene transfusion, DTU behavioral clones, CXDB durable storage, git projection, and budget tracking are all fully implemented and covered by meaningful tests. The architecture is coherent and the abstractions are well-chosen.

However, the codebase is at an honest **Phase 0/1 maturity level** with specific Phase 5–6 components (DTUs, durable CXDB) that are ahead of where they're actually needed. Three production-critical gaps prevent real-world use: (1) scenario validation is LLM-opinion-on-metadata rather than actual behavioral testing, (2) DTU clones are never wired into the validation step they were built for, and (3) the factory worker has no mechanism to verify the code it receives actually works. Additionally, no LLM response has JSON repair/retry, the AR refinement loop has a subtle broken-state bug on multi-iteration lint failures, and the Recipe DAG is dead code. The test suite is solid for deterministic components but cannot catch regressions in the LLM-dependent majority of the system without real CLI integrations. The codebase is well-positioned for a Phase 1 hardening sprint that focuses on: wiring DTUs into validation, adding LLM response retries, fixing the AR refinement loop, parallelizing AR reviewers, and implementing at minimum a code compilation check in the factory output path.
