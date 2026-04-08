# Socratic Project Picture MVP Slice Spec

**Status:** implemented  
**Date:** 2026-04-03  
**Parent:** [Socratic Project Picture And Convergence Workspace Spec](/home/thetu/planner/docs/socratic-project-picture-and-convergence-workspace-spec.md)  
**Related Planning:** [Socratic Project Picture First-Reveal Screen Spec](/home/thetu/planner/docs/socratic-project-picture-first-reveal-screen-spec.md), [Socratic Area Workspace And Shaping Contract Spec](/home/thetu/planner/docs/socratic-area-workspace-and-shaping-contract-spec.md), [Socratic Convergence Autonomy Boundary Spec](/home/thetu/planner/docs/socratic-convergence-autonomy-boundary-spec.md), [Socratic Project Picture MVP Path And Gap Analysis Spec](/home/thetu/planner/docs/socratic-project-picture-mvp-path-and-gap-analysis-spec.md), [Planner SolidStart Phase 39 Session Commit Continuity And Prompt-Bank Merge Spec](/home/thetu/planner/docs/planner-solidstart-phase-39-session-commit-continuity-and-prompt-bank-merge-spec.md), [Planner SolidStart Phase 40 Project-Only Entry And Stale-Draft Hardening Spec](/home/thetu/planner/docs/planner-solidstart-phase-40-project-only-entry-and-stale-draft-hardening-spec.md), [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Source Review:** 2026-04-03 bounded MVP planning pass after the parent brief plus the first-reveal, area-workspace, and autonomy-boundary child specs were all drafted and aligned to the current SolidStart route substrate

## 1. Purpose

Define the first honest implementation slice for the new project-picture-first
Socratic direction.

This slice exists to prove one specific claim:

- the project picture can become the user's primary orientation surface without
  reopening already-implemented prompt-bank truth, continuity, or project-
  first entry work

## 2. Problem

The parent and child specs now define the product direction, but the repo still
lacks one bounded execution artifact.

Without that slice, the likely failure modes are:

- coding directly from the broad parent brief
- shipping a decorative project picture layered on the current prompt flow
- reopening already-solved prompt-bank or route-shell work
- trying to build the whole organism at once

## 3. User Outcome

After this MVP slice:

- a user opens a Socratic session and sees the project picture first
- the picture is the clear primary truth surface
- the picture exposes five top-level areas:
  - `Transformation`
  - `Actors`
  - `Constraints`
  - `Approach`
  - `Pressure`
- the picture shows only bounded, high-value relationships by default
- the user can enter one area and shape it through a compact, object-first
  workspace
- the system can surface low-risk updates and in-area pending revisions without
  mutating accepted project meaning behind the user's back

This slice does **not** need to finish the entire future-state system.

## 4. Scope

### In Scope

- replacing the current prompt-first first impression with the project-picture
  first-reveal hierarchy
- rendering the five MVP top-level areas as the dominant first surface
- showing visible state and pressure at the top-level area layer
- showing only foundational dependencies and critical conflicts by default
- exposing one visible but subordinate next-move surface
- exposing one visible but subordinate global-capture affordance
- area entry into a compact shaping workspace with:
  - quick context
  - 2 to 4 meaningful pressure points
  - one dominant pressure point
  - `accept`, `edit inline`, and `discuss`
- low-risk silent updates plus restrained freshness cues
- in-area pending revisions for meaning-changing updates
- non-blocking pending revisions by default
- direct-conflict escalation when needed

### Out Of Scope

- rich overlay systems beyond what the first reveal absolutely needs
- full seed-tray productization
- provenance-heavy or under-the-hood views
- a graph-tool experience
- a raw blueprint route or direct blueprint visualization as the main surface
- broad media or multimodal capture
- a generalized branch-management product model
- unrelated project, knowledge, discovery, or admin route work

## 5. Reuse And Non-Reopen Rules

This MVP slice must reuse the current route substrate where it is already
truthful.

Do **not** reopen these as new product problems unless implementation exposes a
concrete blocker:

- prompt-bank persistence
- answer-level progression
- local-first prompt-bank merge continuity
- project-only entry
- the existence of a hidden blueprint-like truth layer

The current command-desk route and prompt-bank substrate are baseline inputs,
not the target product story.

## 6. MVP Contract

### 6.1 First reveal

The session route must resolve first into:

- a compact project identity and north-star framing layer
- a dominant project picture
- a secondary next-move surface
- a subordinate global-capture affordance

It must not resolve first into:

- a question stack
- a blank input
- an equal-weight dashboard
- a graph canvas

### 6.2 Top-level area model

The picture must render these five visible areas:

- `Transformation`
- `Actors`
- `Constraints`
- `Approach`
- `Pressure`

These must remain spatially stable enough for memory to form.

### 6.3 Area entry

When the user enters one area, the route must:

1. level the user in current area context
2. expose only a small number of meaningful pressure points
3. keep one pressure point visually dominant
4. avoid unfolding a recursive mini-map

### 6.4 Response model

The primary shaping responses inside an area must be:

- `accept`
- `edit inline`
- `discuss`

Routine shaping must stay out of chat.

### 6.5 Convergence boundary

The route must support:

- low-risk silent visible updates for state, confidence, suggested labels,
  tension markers, and suggested relationships
- restrained freshness cues on affected areas
- in-area pending revisions for meaning-changing proposals
- non-blocking revisions by default
- stronger escalation for direct conflicts

The route must not silently rewrite:

- area identity
- accepted major relationships
- the current north-star definition
- any visible change that materially alters project shape

## 7. Touched Surfaces

Primary likely implementation surfaces:

- `planner-solid/src/routes/sessions/session-workspace-screen.tsx`
- `planner-solid/src/routes/sessions/session-workspace-controller.ts`
- `planner-solid/src/routes/sessions/session-workspace-view.ts`
- `planner-solid/src/lib/workspace.ts`
- `planner-solid/src/lib/prompt-bank.ts`
- `planner-solid/src/app.css`
- session-route tests and browser proof

Secondary backend/schema surfaces should be touched only if implementation
proves the existing substrate cannot support the MVP truthfully.

## 8. Acceptance Criteria

1. the first visible session surface is the project picture, not a prompt stack
2. the five MVP top-level areas are visible and understandable on first reveal
3. the default screen shows only bounded, high-value relationships
4. the first screen includes one visible next move and one subordinate global
   capture affordance without diluting the picture
5. entering one area gives compact context plus a bounded set of pressure
   points
6. the area workspace uses `accept`, `edit inline`, and `discuss` as the
   dominant response model
7. low-risk silent updates surface freshness cues rather than invisible drift
8. meaning-changing updates appear as in-area pending revisions
9. pending revisions are non-blocking by default
10. direct conflicts can escalate visibly when needed

## 9. Verification Plan

Before claiming this slice complete, verify at minimum:

- focused session-route browser proof that the first visible surface is now the
  project picture
- route-level proof that the user can enter an area and see bounded pressure
  points instead of a prompt-first stack
- proof that low-risk updates surface restrained freshness cues
- proof that meaning-changing updates render as in-area pending revisions
- proof that direct conflicts can escalate without collapsing the route
- `npm --prefix planner-solid run build`
- `git diff --check`

If the implementation touches backend or schema seams, add the smallest focused
tests needed to keep those changes truthful.

## 10. Rollback / Fallback

If the full slice proves too broad in one pass, the fallback is:

- land the first-reveal project-picture hierarchy first
- keep area entry bounded to one dominant pressure point plus one or two
  secondary points
- keep the same autonomy boundary model

The fallback is **not**:

- reverting to a prompt-first first screen
- exposing the raw blueprint graph
- moving major proposed changes into a detached review inbox

## 11. Remaining Risks

The biggest implementation risks are:

- building a decorative picture that is not the actual truth surface
- letting `Approach` become a junk drawer
- allowing area entry to expand into recursive complexity
- treating freshness cues as enough for meaning-changing revisions
- overusing conflict escalation

## 12. Implementation Outcome

Implemented on 2026-04-04 as the first bounded project-picture delivery lane
for the Solid session route.

Delivered behavior:

- the session route now resolves first into a project-picture-first surface
  instead of a prompt-first first impression
- the route now renders the five MVP top-level areas:
  - `Transformation`
  - `Actors`
  - `Constraints`
  - `Approach`
  - `Pressure`
- the default picture now exposes only bounded, high-value relationship hints
  instead of a graph-first topology
- the active shaping surface now renders as a compact area workspace with a
  bounded set of pressure points rather than a raw prompt stack as the primary
  product identity
- low-risk freshness cues and in-area pending revisions now exist as visible
  route-level concepts
- the implementation remains bounded by reusing the existing prompt-bank,
  commit, and continuity substrate instead of reopening those earlier route
  contracts

Intentional limitation carried forward:

- the first slice keeps area cards as static summaries and keeps the active
  shaping path centered on the currently active area rather than shipping a
  fully interactive multi-area selection model in the same pass

## 13. Verification Evidence

- `npm --prefix planner-solid test -- --run src/routes/sessions/session-workspace-view.test.ts`
- `npm --prefix planner-solid run build`
- `cd planner-solid && VITE_PLANNER_FRONTEND_MOCK=1 npx playwright test --config playwright.frontend-mock.config.ts e2e/phase-35-frontend-mock.spec.ts`
- `cd planner-solid && npx playwright test --config playwright.canonical-static.config.ts e2e/phase-37-canonical-static-runtime.spec.ts`
- `git diff --check`
