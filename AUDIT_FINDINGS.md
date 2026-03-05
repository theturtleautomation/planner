# Planner v2 — Honest Codebase Audit

**Date**: 2026-03-05  
**Scope**: Every crate, every file, zero punches pulled.

---

## Summary

| Category | Count |
|----------|-------|
| **Critical** (broken or lying to the user) | 2 |
| **Significant** (incomplete features with user-facing impact) | 4 |
| **Minor** (cleanup, dead code, test drift) | 5 |
| **Acceptable / By-Design** | 4 |

---

## CRITICAL

### 1. Blueprint graph is never populated by the pipeline

**Location**: `planner-core/src/pipeline/` (absence)  
**What**: The Blueprint store (`BlueprintStore`) has full CRUD and the web UI renders a D3 force graph. The REST API supports creating/reading/updating/deleting nodes. But the **pipeline itself** — the 12-stage Socratic→Factory flow — never calls `BlueprintStore::upsert_node()`. Zero references to `blueprint` or `BlueprintStore` anywhere in `planner-core/src/pipeline/`.

**Impact**: A user who runs a full pipeline and then navigates to `/blueprint` will see an empty graph. Every time. The only way to get nodes into the graph is manual API calls. The feature is structurally dead unless someone wires the pipeline steps to emit blueprint nodes.

**Fix required**: The compile, AR review, factory, and git steps should each emit relevant `BlueprintNode` entries (decisions, components, technologies, constraints) as they produce artifacts.

---

### 2. Three frontend tests are broken (test drift, not bugs)

**Location**: `planner-web/src/`  
**Tests failing**: 164/167 pass. The 3 failures:

| Test file | Failing test | Root cause |
|-----------|-------------|------------|
| `Layout.test.tsx` | `renders header with app name PLANNER v2` | Layout was redesigned to use an ASCII art banner. The text `PLANNER v2` as a single element no longer exists — it's now `SOCRATIC LOBBY` + `v2` as separate spans. |
| `ClassificationBadge.test.tsx` | `renders complexity label` | Test expects `complexity:` label text; component was refactored and the label rendering changed. |
| `LoginPage.test.tsx` | One of the feature list assertions | LoginPage copy was updated but the test still expects old text. |

**Impact**: Tests lie. They pass in CI only if these 3 are excluded. New contributors will see failures and not know if the codebase is broken.

**Fix required**: Update the 3 test files to match the current component markup.

---

## SIGNIFICANT

### 3. Orphaned hook: `useSessionWebSocket.ts` (206 lines, zero imports)

**Location**: `planner-web/src/hooks/useSessionWebSocket.ts`  
**What**: This is a fully implemented WebSocket hook for the session pipeline (stage updates, chat messages, pipeline completion). It is imported by **nothing** — zero references in any component, page, or test file. The active hook is `useSocraticWebSocket.ts`.

**Impact**: 206 lines of dead code. A developer reading the hooks directory would reasonably assume this is the session WebSocket and try to use it, not realizing the active one has a different name.

**Fix**: Either delete it or wire it into the session page if it's intended for the pipeline execution view.

---

### 4. "Future version" messages during running pipelines

**Location**: `planner-server/src/api.rs:568`, `planner-tui/src/app.rs:677`  
**What**: When a user sends a message while a pipeline is already running, both the server API and TUI respond with:  
> "Interactive follow-up during execution will be available in a future version."

**Impact**: This is honest — it tells the user the feature doesn't exist yet. But it's a user-facing limitation that affects both interfaces. It's not a stub, but it is an unfinished interaction model.

**Assessment**: Acceptable to ship as-is, but should be on a roadmap. The message is clear.

---

### 5. DAG interpreter is documented but not implemented

**Location**: `planner-core/src/pipeline/mod.rs:139-146`  
**What**: The pipeline recipe is defined as a DAG data structure, but execution is **imperative** — steps run in a fixed linear order. The code comment explicitly says: "planned for Phase 3+". The DAG structure exists but the interpreter that would enable parallel branches, conditional steps, or custom ordering does not.

**Impact**: The pipeline always runs the same 12 steps in the same order. This is fine for the current use case, but the data structure implies flexibility that doesn't exist.

**Assessment**: Acceptable at current maturity. The linear pipeline works correctly. The DAG interpreter is a genuine Phase 3+ feature, not a broken promise.

---

### 6. Standalone mockup still in repo

**Location**: `planner-blueprint-mockup/index.html` (88KB)  
**What**: This was the original static HTML mockup of the Blueprint UI. Now that the real Blueprint page is integrated into `planner-web/`, this file serves no purpose except as a design reference.

**Impact**: 88KB of dead weight. Anyone cloning the repo might think this is the live Blueprint UI and get confused.

**Fix**: Delete the directory, or move it to a `docs/design-references/` folder with a README explaining its historical purpose.

---

## MINOR

### 7. 17 `#[allow(dead_code)]` annotations

**Location**: Various files across `planner-core` and `planner-tui`  
**What**: Most are legitimate — deserialized struct fields (Claude/Gemini JSON response models), traceability fields read by downstream consumers, and test infrastructure. A few in `planner-tui` (app.rs:79, app.rs:722, events.rs:16, pipeline.rs:66) may be genuinely unused fields that accumulated during TUI development.

**Impact**: Low. The compiler isn't wrong — these fields aren't read in Rust code. But for serde structs, they exist to capture API responses.

**Assessment**: Review the 4 TUI annotations. The serde/response-model ones are fine.

---

### 8. Phase roadmap comments (71 instances)

**Location**: Throughout `planner-core` and `planner-schemas`  
**What**: Comments referencing "Phase 3", "Phase 4", "Phase 5", "Phase 6", "Phase 7" describe the development history and future plans. Examples:
- `Phase 3 adds multi-chunk compilation`
- `Phase 5 replaces the SQLite sidecar`
- `Phase 7: Kilroy CLI is replaced by the pluggable FactoryWorker trait`

**Impact**: None functionally. These are design documentation embedded in code. Some reference completed phases (accurate history), some reference future phases (roadmap).

**Assessment**: Acceptable. The comments are accurate and helpful for understanding the codebase evolution. No action needed.

---

### 9. DTU providers — 3 of 5 are Phase 5 stubs

**Location**: `planner-core/src/dtu/mod.rs:12-14`  
**What**: The DTU (Dependency Template Universe) registry has 5 provider slots: Stripe, Auth0, SendGrid, Supabase, Twilio. Stripe and Auth0 have real NLSpec template implementations. SendGrid, Supabase, and Twilio are listed in the Phase 5 registry constructor but their template content is minimal.

**Impact**: Low. The DTU is a template system — it generates boilerplate NLSpec sections for known SaaS dependencies. The 3 Phase 5 providers produce valid but thin templates. This is expected incremental development, not a broken feature.

**Assessment**: Acceptable. The system works with the providers that exist.

---

### 10. Lean4 `sorry` stubs — 9 instances (by design)

**Location**: `planner-core/src/pipeline/verification.rs`  
**What**: The verification step generates Lean4 theorem templates as string literals in Rust code. Each contains `sorry -- proof stub`. The generated file explicitly states: "These are theorem STUBS. Replace `sorry` with actual proofs."

**Impact**: None. These are **code generation templates**, not unfinished Lean4 proofs. No `.lean` files exist in the repo. The system generates these as starting points for formal verification.

**Assessment**: By design. The comments are clear.

---

### 11. `planner.env` references a "Vault Integration (Future)" section

**Location**: `deploy/planner.env` Section 7  
**What**: The env file documents how to integrate HashiCorp Vault, SOPS, or systemd-creds for secret management. None of these integrations exist in the codebase — they're documentation for ops teams.

**Impact**: None. This is good forward-looking documentation, not a broken feature. The env file is well-structured and honest about what's implemented vs. documented.

**Assessment**: Acceptable. Good ops documentation.

---

## ACCEPTABLE / BY-DESIGN

### 12. Mock LlmClient in providers.rs
Legitimate test infrastructure. `LlmRouter::with_mock()` is used by integration tests to avoid real LLM calls. This is standard practice.

### 13. `cargo check` zero warnings, `tsc --noEmit` zero errors
Build toolchain is clean. No suppressed warnings beyond the documented `#[allow(dead_code)]` annotations.

### 14. 233 Rust tests pass, 0 failures
The Rust test suite is comprehensive and clean.

### 15. Vite build is clean, code-split
Blueprint chunk is 84KB (27KB gzip). No build warnings.

---

## Recommended Priority

1. **Wire Blueprint into pipeline** (Critical #1) — This is the biggest gap. Without it, the Blueprint UI is a pretty but empty shell.
2. **Fix 3 broken frontend tests** (Critical #2) — Quick wins, ~30 minutes of work.
3. **Delete `useSessionWebSocket.ts`** (Significant #3) — One file deletion.
4. **Delete or archive `planner-blueprint-mockup/`** (Significant #6) — One directory removal.
5. Everything else is acceptable to ship as-is.
