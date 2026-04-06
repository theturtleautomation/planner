# Planner SolidStart Phase 40 Project-Only Entry And Stale-Draft Hardening Spec

**Status:** implemented  
**Date:** 2026-04-02  
**Parent:** [Planner SolidStart Platform Direction Spec](/home/thetu/planner/docs/planner-solidstart-platform-direction-spec.md)  
**Related Planning:** [Planner SolidStart Phase 32 Work Entry IA And Session Route Topology Spec](/home/thetu/planner/docs/planner-solidstart-phase-32-work-entry-ia-and-session-route-topology.spec.md), [Planner SolidStart Phase 39 Session Commit Continuity And Prompt-Bank Merge Spec](/home/thetu/planner/docs/planner-solidstart-phase-39-session-commit-continuity-and-prompt-bank-merge-spec.md), [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Review:** 2026-04-02 direct inspection of `planner-solid/src/routes/index.tsx`, `planner-solid/src/routes/sessions/index.tsx`, `planner-solid/src/routes/sessions/new.tsx`, `planner-solid/src/routes/sessions/session-workspace-controller.ts`, and `planner-server/src/api.rs`

## 1. Purpose

Remove projectless direct-session entry from the Solid app and harden draft-save behavior so superseded prompts do not surface stale-save errors during live prompt-bank evolution.

## 2. Problem

Two concrete product mismatches remained:

- the app still exposed `Direct session` entry points even though the product direction is now fully project-first
- session draft autosave could still hit `prompt_stale` when an older prompt was superseded before a delayed draft save completed

## 3. User Outcome

After this slice:

- all new work starts from a project
- `/sessions/new` no longer behaves as a projectless creation surface
- superseded prompts no longer surface `prompt_stale` as visible user-facing noise during normal progression

## 4. Scope

### In Scope

- removing user-facing direct-session entry points from the home and sessions routes
- redirecting `/sessions/new` to project creation
- stale-prompt draft-save guarding in the Solid session controller
- focused browser proof for project-only entry and stale-save absence

### Out Of Scope

- changing existing session/project data ownership semantics
- broader session route redesign beyond the stale-save guard
- backend API contract changes unless needed for truthful error handling

## 5. Implementation Outcome

Implemented on 2026-04-02.

Delivered behavior:

- removes `Direct session` as a visible CTA from the home and sessions routes
- redirects `/sessions/new` to `/projects/new` so projectless creation is no longer offered as a product path
- repoints the empty-session fallback CTA toward project creation
- adds a local stale-prompt guard in the session controller so draft saves for superseded prompts no-op instead of surfacing `prompt_stale`
- treats server `prompt_stale` draft-save responses as continuity-safe no-ops during normal prompt supersession

## 6. Verification Evidence

- `npm --prefix planner-solid test -- --run src/lib/workspace.test.ts src/lib/prompt-bank.test.ts src/lib/mock/store.test.ts`
- `npm --prefix planner-solid run build`
- `cd planner-solid && VITE_PLANNER_FRONTEND_MOCK=1 npx playwright test --config playwright.frontend-mock.config.ts e2e/phase-35-frontend-mock.spec.ts`
- `git diff --check`

## 7. Open Questions

None block this slice.
