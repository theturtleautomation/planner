# Planner v2 — Comprehensive Audit Report

**Date:** 2026-02-28  
**Scope:** Full codebase (planner-core, planner-server, planner-tui, planner-schemas, planner-web)  
**Method:** Three parallel code audits reading every source file, cross-referenced against architecture docs and strongDM industry patterns  

---

## Overall Scores

| Dimension | Score | Grade |
|-----------|-------|-------|
| **Implementation Completeness** | 72% | B- |
| **Real-World Usability** | 45% | D+ |
| **Test Quality** | 61% | C- |
| **Security Posture** | 38% | F |
| **StrongDM / Industry Parity** | 55% | D+ |
| **Code Quality & Architecture** | 85% | A- |

**Composite Score: 59% — Not yet production-ready, but the foundation is solid and the remaining work is well-defined.**

---

## 1. Implementation Completeness (72%)

### What's Genuinely Real and Working

| Component | Status | Notes |
|-----------|--------|-------|
| LLM CLI Clients (Claude, Gemini, Codex) | ✅ Real | Native CLI shelling, stdin piping, streaming parse, timeout, error classification |
| Pipeline Orchestration (16 steps) | ✅ Real | Full chain: Intake → Compile → Lint → AR → Factory → Validate → Git |
| Front Office Steps (11 of 11) | ✅ Real | Intake, ChunkPlanner, CompileSpec, Lint, AR, Refinement, Scenarios, Ralph, GraphDot, Agents, Context Pack |
| Back Office Steps (5 of 5) | ✅ Real | Factory Worker, Scenario Validation, Telemetry, Git Projection, Budget |
| CXDB In-Memory Engine | ✅ Real | Content-addressed, BLAKE3 hashed, MessagePack, dedup |
| CXDB Durable Engine | ✅ Real | Filesystem-backed, index rebuild on restart, blob dedup on disk |
| DTU Behavioral Clones (5) | ✅ Real | Stripe, Auth0, SendGrid, Supabase, Twilio — all have stateful endpoints and state machines |
| Linter (12 rules) | ✅ Real | Deterministic, no LLM dependency |
| Ralph (3 modes) | ✅ Real | ScenarioAugmentation (LLM), GeneTransfusion (deterministic), DTU Config (deterministic) |
| Schema System (18 types) | ✅ Real | All implement ArtifactPayload with BLAKE3 integrity verification |
| Server (Axum HTTP + WebSocket) | ✅ Real | Session isolation, pipeline spawning, WebSocket stage updates |
| TUI (Ratatui) | ✅ Real | Terminal setup, async events, pipeline channel integration, mid-line editing |
| Web UI (React + TypeScript) | ✅ Real | Auth0 integration, WebSocket chat, pipeline visualization, design system |
| Git Projection | ✅ Real | Actual git subprocess calls, commit creation, diff listing |
| Anti-Lock-in Audit | ✅ Real | Vendor pattern detection, migration risk scoring |
| Formal Verification | ⚠️ Stubs | Generates Lean4 theorem stubs with `sorry` — scaffolding, not proofs |
| Context Pack | ✅ Real | Token-budgeted context assembly with priority ordering |

### Defined But Not Wired (8 of 18 artifact types)

| Type | Issue |
|------|-------|
| `DecisionV1` | Sacred Anchor amendment flow not wired |
| `ContextPackV1` | Step exists in `steps/context_pack.rs` but not called from pipeline runner |
| `DtuConfigV1` | Schema defined; DTU config wiring is Phase 4+ |
| `PreviewSnapshotV1` | `DeploySandbox` step listed in recipe but never called |
| `ConsequenceCardV1` | Ralph generates cards but they are only logged, not stored as Turns |
| `PyramidSummaryV1` | Module exists; no call in `run_full_pipeline` |
| Recipe DAG | 17-step DAG defined but pipeline is hardcoded imperative — recipe is dead code |
| CXDB TCP/HTTP APIs | Protocol and query types defined; no servers implemented |

### Feature Parity with Design Documents

| Feature from ARCHITECTURE.md | Implemented | Score |
|------------------------------|-------------|-------|
| 16-step pipeline | ✅ All steps execute | 100% |
| Multi-chunk compilation | ✅ | 100% |
| AR Review (3 reviewers) | ✅ (sequential only) | 80% |
| AR Refinement Loop | ⚠️ Loop exists but broken on multi-iteration lint failures | 60% |
| Scenario Generation + Validation | ⚠️ Generation real; validation can't see code | 50% |
| Factory Worker (Codex) | ⚠️ Invokes codex but no compilation check | 70% |
| DTU Behavioral Clones | ⚠️ 5 clones built; never wired into validation | 40% |
| CXDB Durable Storage | ⚠️ Engine works; only Intake is persisted | 30% |
| CXDB TCP Protocol | ⚠️ Wire format defined; no server | 10% |
| CXDB HTTP Read API | ⚠️ Types defined; no server | 10% |
| Budget Tracking | ✅ Phase0 budget, hard stop, warning thresholds | 90% |
| Pyramid Summarization | ⚠️ Builder exists; not connected to pipeline | 20% |
| Project Registry | ⚠️ Functional; never called from pipeline | 20% |
| Web UI + Auth | ✅ React SPA with Auth0, WebSocket, pipeline bar | 80% |
| TUI | ✅ Functional Ratatui app | 85% |

---

## 2. Real-World Usability Assessment (45%)

**Can someone actually use this system today?**

### What Works End-to-End
1. User types a project description in TUI or Web UI
2. Pipeline runs all 16 steps with real LLM calls (requires claude, gemini, codex CLIs installed)
3. NLSpec is compiled, reviewed, refined, scenarios generated
4. Factory generates code via Codex
5. Git projection commits the output
6. User sees pipeline progress and final summary

### What Blocks Real-World Use

| Blocker | Severity | Impact |
|---------|----------|--------|
| **Auth bypass when AUTH0_SECRET unset** | 🔴 Critical | Any JWT accepted in production RS256 config — full auth bypass |
| **Scenario validation doesn't read code** | 🔴 Critical | Validation step is LLM guessing, not behavioral testing |
| **Factory has no compilation check** | 🔴 Critical | Broken code reports as "Build Success" |
| **CXDB only persists Intake** | 🟠 High | 15 of 16 artifacts lost on restart — no audit trail |
| **No JSON repair/retry on LLM failures** | 🟠 High | Single malformed LLM response fails entire step |
| **DTUs never used in validation** | 🟠 High | 5 production-quality mocks are dead weight |
| **AR refinement loop broken** | 🟠 High | Multi-iteration lint failures produce no-op iterations |
| **Dashboard shows no sessions** | 🟠 High | Users can't list or resume previous sessions |
| **No 401/403 handling in frontend** | 🟠 High | Expired tokens show raw error strings |
| **WebSocket token in query string** | 🟡 Medium | JWT visible in server logs and browser history |
| **No responsive CSS** | 🟡 Medium | Mobile unusable |
| **No frontend tests** | 🟡 Medium | Zero test coverage for React components |

### Usability Verdict

**For a developer running locally with all 3 LLM CLIs installed:** The system works end-to-end for a single pipeline run. The TUI provides a functional interface. The web UI is visually polished but has operational gaps. You get real output — an NLSpec, code, a git commit. The experience is promising.

**For a team deploying in production:** Not ready. The auth bypass is a showstopper. Session persistence across restarts doesn't exist. The "validation" step doesn't actually validate. There's no way to list previous sessions. Error handling for expired tokens will confuse users.

---

## 3. Test Quality (61%)

### Test Inventory

| Crate | Unit Tests | Integration | Total | Quality |
|-------|-----------|-------------|-------|---------|
| planner-core | ~200 | ~46 | ~246 | Mixed — strong for deterministic, absent for LLM-dependent |
| planner-schemas | 2 | — | 2 | Meaningful (Turn roundtrip + integrity) |
| planner-server | 31 | — | 31 | Meaningful (auth, isolation, endpoints) |
| planner-tui | 19 | — | 19 | Meaningful (events, rendering, state) |
| planner-web | 0 | 0 | 0 | None |
| **Total** | | | **333** | |

### What Tests Actually Verify

**Strong coverage (real regression guards):**
- Linter: all 12 rules tested with real spec data
- CXDB: roundtrip, dedup, persistence across re-opens, blob limits
- Git: actual git subprocess in temp directory
- Budget: threshold transitions (warning, hard stop)
- Ralph: gene transfusion patterns, DTU config generation
- Context Pack: truncation, priority ordering
- Server: auth enforcement (403 for wrong user, 401 without token)
- TUI: pipeline event processing, rendering without panics

**Weak/trivial tests (low regression value):**
- `e2e_phase2_ar_severity_classification`: tests `ArSeverity::Blocking == ArSeverity::Blocking`
- `e2e_phase2_refinement_no_blocking_passthrough`: asserts on manually constructed data
- `e2e_phase3_recipe_includes_ar_steps`: asserts step names exist in recipe (which is dead code)

**Not tested at all:**
- Any real LLM call (all LLM-dependent paths use canned data)
- Full `run_full_pipeline` from start to finish (no single test exercises all 16 steps)
- Error propagation when LLM returns malformed JSON mid-pipeline
- WebSocket reconnection logic
- React components and hooks
- Auth0 login flow
- Session listing and resumption

### Test Quality Verdict

The test suite is well-structured and catches real regressions in deterministic components. It provides zero coverage for the LLM-dependent majority of the system (which is understandable given the cost/complexity of LLM testing, but should be acknowledged). The frontend has zero tests, which is the most actionable gap.

---

## 4. Security Assessment (38%)

### Critical Vulnerabilities

| # | Issue | Location | CVSS-like |
|---|-------|----------|-----------|
| S1 | **Auth bypass: insecure_disable_signature_validation()** when AUTH0_SECRET unset | `auth.rs:173-181` | 9.8 |
| S2 | **Expiry validation disabled** in same code path — expired tokens accepted | `auth.rs:177` | 8.5 |
| S3 | **XSS via unsanitized markdown** — no `rehype-sanitize` on LLM output rendering | `ChatPanel.tsx` | 7.1 |
| S4 | **JWT in WebSocket query string** — visible in logs and browser history | `useSessionWebSocket.ts:80` | 5.3 |
| S5 | **RwLock::unwrap() on every read/write** — lock poisoning crashes entire server | `session.rs` (all lock sites) | 6.5 |
| S6 | **No session expiry** — memory grows unboundedly with stale sessions | `session.rs` | 4.0 |
| S7 | **Silent token error masking** — getAccessTokenSilently failures return '' | `useAuthenticatedFetch.ts` | 4.5 |

### What's Done Right
- User ID enforced on every session access (403 on mismatch) — tested
- CORS tightened when auth enabled
- Dev mode clearly separated from production auth path
- No `unsafe` blocks in any crate
- Content-addressed hashing (BLAKE3) on all artifacts
- No hardcoded credentials in codebase

---

## 5. strongDM / Industry Parity (55%)

Comparing against strongDM-style infrastructure access platforms and industry best practices for AI-powered development tools:

| Industry Expectation | Planner Status | Gap |
|---------------------|----------------|-----|
| Cryptographic audit trail | ⚠️ BLAKE3 hashes computed but only Intake persisted | 15 artifacts have no audit trail |
| RBAC / multi-tenancy | ⚠️ Session-level isolation by user_id; no roles, no teams | No admin/viewer/operator distinction |
| Idempotent operations | ✅ Content-addressed dedup in CXDB | Works correctly |
| Graceful degradation | ✅ LLM CLI failures propagate as error messages, not crashes | Both TUI and server handle gracefully |
| Observability / telemetry | ⚠️ Pipeline generates a summary; no structured metrics export | No OpenTelemetry, no Prometheus, no log aggregation |
| Zero-trust auth | ❌ Auth bypass when misconfigured; no JWKS validation | Should fail closed, not open |
| Data encryption at rest | ❌ Raw MessagePack on disk | No encryption |
| Rate limiting | ⚠️ Auth0 DTU clone has rate limiting; server itself has none | No request rate limiting on API |
| API versioning | ❌ No version prefix on API routes | `/api/sessions` not `/api/v1/sessions` |
| Health checks / readiness | ✅ `/health` endpoint exists | Works correctly |
| Session management | ⚠️ In-memory only; no persistence, no expiry, no cleanup | Sessions lost on restart |
| Responsive UI | ❌ Desktop only | Zero `@media` queries |
| Accessibility | ❌ No ARIA attributes, no screen reader support | Poor a11y |

---

## 6. Code Quality & Architecture (85%)

This is where Planner shines. Despite the functional gaps above, the codebase is well-architected:

**Strengths:**
- Clean module boundaries (5 crates with clear responsibilities)
- Real trait-based abstractions (`LlmClient`, `FactoryWorker`, `TurnStore`, `DtuProvider`)
- Consistent error handling via `StepError` and `LlmError` enums
- Zero `any` types in TypeScript, zero `unsafe` in Rust
- Strong type system usage (discriminated unions, const TYPE_IDs, ArtifactPayload trait)
- Idiomatic async Rust (Tokio channels, spawn, RwLock)
- Clean design token system in CSS
- React component structure is logical and consistent

**Areas for improvement:**
- `expect()`/`unwrap()` in hot paths should be proper error handling
- Recipe DAG should either be the execution engine or be removed
- DTU clones should either be wired in or documented as "future work"
- Model catalog entries are speculative (future model names) — should be validated

---

## 7. Critical Bugs (Prioritized)

### P0 — Must Fix Before Any Production Use

| # | Bug | Location | Fix |
|---|-----|----------|-----|
| B1 | Auth bypass: `insecure_disable_signature_validation()` when AUTH0_SECRET unset | `auth.rs:173` | Fetch JWKS from `https://{domain}/.well-known/jwks.json`; validate RS256. Remove fallback entirely or panic at startup. |
| B2 | Scenario validation sends path string to Gemini, not actual code | `validate.rs` | Read factory output files into the prompt so Gemini can evaluate actual code |
| B3 | CodexFactoryWorker always returns `success: true` | `factory_worker.rs` | After codex returns, attempt `cargo check` or equivalent. Set success based on compilation result. |

### P1 — Should Fix Before User Testing

| # | Bug | Location | Fix |
|---|-----|----------|-----|
| B4 | CXDB only persists Intake; 15 artifacts dropped | `pipeline/mod.rs` | Add `config.persist()` calls after each step |
| B5 | AR refinement loop: blocking_findings cleared on lint failure, never refilled | `ar_refinement.rs` | Repopulate `blocking_findings` from lint violations on re-lint failure |
| B6 | No JSON repair/retry on LLM parse failures | All step files | Add a retry with "please output valid JSON" prompt on parse failure |
| B7 | `String::insert(cursor_position, c)` panics on multi-byte UTF-8 | `app.rs:213` (TUI) | Use char-based indexing instead of byte-based |
| B8 | No 401/403 handling in frontend | `api/client.ts` | Detect 401, call `loginWithRedirect()` |
| B9 | `StartPipeline` WS message does nothing | `ws.rs` | Either wire it to `run_pipeline_for_session` or remove the handler |
| B10 | Dashboard shows no sessions (no list endpoint) | `Dashboard.tsx` / server | Implement `GET /api/sessions` and fetch on Dashboard mount |

### P2 — Should Fix Before Beta

| # | Bug | Location | Fix |
|---|-----|----------|-----|
| B11 | RwLock::unwrap() on all session store operations | `session.rs` | Use `parking_lot::RwLock` (no poisoning) or proper error mapping |
| B12 | Potential duplicate messages (REST + WS) | `SessionPage.tsx` | Add message ID dedup in `allMessages` merge |
| B13 | OpenAI CLI prompt via positional arg (length limits) | `providers.rs` | Use stdin like Anthropic/Google clients |
| B14 | WS stage updates every 500ms regardless of change | `ws.rs` | Diff against last-sent state |
| B15 | No session expiry/cleanup | `session.rs` | Add TTL-based cleanup with background task |
| B16 | XSS risk from unsanitized markdown | `ChatPanel.tsx` | Add `rehype-sanitize` plugin |
| B17 | JWT in WebSocket query string | `useSessionWebSocket.ts` | Send token as first WS message instead |

---

## 8. Prioritized Remediation Plan

### Sprint 1: Security & Auth (Est. 2-3 days)
1. Fix JWKS validation in `auth.rs` — fetch from `.well-known/jwks.json`, validate RS256
2. Add `rehype-sanitize` to ChatPanel
3. Move WS auth token from query string to first-message
4. Replace `RwLock::unwrap()` with `parking_lot::RwLock`
5. Add session TTL and cleanup task

### Sprint 2: Pipeline Integrity (Est. 3-5 days)
1. Fix scenario validation — read factory output files into evaluator prompt
2. Add compilation check after factory worker
3. Wire DTU clones into validation step
4. Fix AR refinement loop (repopulate blocking_findings on re-lint failure)
5. Add JSON repair/retry on all LLM parse failures
6. Persist all 16 artifacts to CXDB (not just Intake)

### Sprint 3: Frontend Completeness (Est. 2-3 days)
1. Implement session listing in Dashboard (`GET /api/sessions`)
2. Add 401/403 handling with re-auth redirect
3. Fix duplicate message potential
4. Add error toast for `sendMessage` failures
5. Wire `StartPipeline` WS message or remove dead handler
6. Add textarea auto-grow

### Sprint 4: Polish & Testing (Est. 3-4 days)
1. Add Vitest + React Testing Library for frontend (WebSocket hook, ChatPanel, PipelineBar)
2. Fix TUI UTF-8 cursor position bug
3. Add responsive CSS (`@media` breakpoints)
4. Add ARIA attributes for accessibility
5. Diff-based WS stage updates
6. Parallelize AR reviewers with `tokio::join!`

### Sprint 5: Infrastructure (Est. 2-3 days, lower priority)
1. Implement CXDB TCP binary protocol server
2. Implement CXDB HTTP read API
3. Add API versioning (`/api/v1/`)
4. Add request rate limiting
5. Either make Recipe the execution engine or remove it
6. Update schema doc table (5 missing entries)

---

## 9. Summary

**Planner v2 is a professionally architected codebase that is ~72% implemented.** The Rust code quality is excellent — clean abstractions, strong types, meaningful tests for deterministic components. The LLM client layer correctly uses native CLIs. The pipeline orchestrates 16 real steps.

**The gap to production is concentrated in three areas:**

1. **Security** — The auth bypass bug (S1) is a showstopper. The frontend has no expired-token handling. These must be fixed first.

2. **Validation integrity** — The pipeline claims to validate generated code but actually sends a filesystem path to an LLM and asks it to guess. The 5 DTU behavioral clones (which are high-quality work) are never used. The factory worker never checks if code compiles. These three gaps mean the pipeline's quality gates are largely ceremonial.

3. **Persistence** — Only 1 of 16 artifacts is persisted to CXDB. Users can't list previous sessions. There's no audit trail for anything after the Intake step.

**The good news:** All three gaps have clear, bounded fixes. The architecture supports each fix without requiring rewrites. The codebase is ~12-18 days of focused work away from a credible beta.
