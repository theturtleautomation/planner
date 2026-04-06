# Socratic Convergence Autonomy Boundary Spec

**Status:** draft  
**Date:** 2026-04-03  
**Parent:** [Socratic Project Picture And Convergence Workspace Spec](/home/thetu/planner/docs/socratic-project-picture-and-convergence-workspace-spec.md)  
**Related Planning:** [Socratic Project Picture First-Reveal Screen Spec](/home/thetu/planner/docs/socratic-project-picture-first-reveal-screen-spec.md), [Socratic Area Workspace And Shaping Contract Spec](/home/thetu/planner/docs/socratic-area-workspace-and-shaping-contract-spec.md), [Socratic Project Picture MVP Path And Gap Analysis Spec](/home/thetu/planner/docs/socratic-project-picture-mvp-path-and-gap-analysis-spec.md), [Planner SolidStart Phase 39 Session Commit Continuity And Prompt-Bank Merge Spec](/home/thetu/planner/docs/planner-solidstart-phase-39-session-commit-continuity-and-prompt-bank-merge-spec.md), [Blueprint Project Root And CodeGraph Integration](/home/thetu/planner/docs/blueprint-project-root-codegraph-integration.md), [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Review:** 2026-04-03 bounded planning pass after the parent brief, first-reveal child spec, and area-workspace child spec locked the product-picture model, the top-level area system, and the area-shaping contract

## 1. Purpose

Define the boundary between:

- internal continuous convergence
- low-risk visible updates that may happen automatically
- meaning-changing updates that must be surfaced for review

This child spec exists to stop the product from becoming either:

- opaque and slippery
- or so approval-heavy that the system loses initiative

## 2. Problem

The product thesis depends on background reconvergence.

If Darkfactory does not continuously update its understanding, the project
picture becomes stale and detail drift wins.

But if Darkfactory mutates the visible artifact too aggressively, the user
loses trust and spatial continuity.

The boundary must therefore be hard enough to answer:

- what may happen silently?
- what must become a visible proposed change?
- what, if anything, is temporarily blocking?

## 3. User Outcome

The user should feel:

- the system is actively keeping the project coherent
- the project picture stays fresh without becoming jumpy
- the system can improve weak local truth on its own
- major changes do not happen behind the user's back

The user should not feel:

- that they are monitoring a hidden swarm
- that accepted project meaning is unstable
- that every improvement requires manual approval

## 4. Scope

### In Scope

- the silent-update boundary for visible project-state changes
- the visible treatment for system-proposed revisions
- the default blocking versus non-blocking behavior
- area-level transparency cues for recent system updates
- escalation behavior for direct conflicts

### Out Of Scope

- the full overlay design
- transport/schema implementation details
- a full history/provenance feature set
- execution-task decomposition

## 5. Locked Inputs From Parent Planning

This child spec assumes:

- the project picture is the primary truth surface
- the product picture is area-based, not graph-first
- the visible top-level areas are:
  - `Transformation`
  - `Actors`
  - `Constraints`
  - `Approach`
  - `Pressure`
- the area workspace uses 2 to 4 meaningful pressure points, with one
  dominant
- the user owns final authorship

## 6. Autonomy Boundary

### 6.1 Internal convergence is always on

Darkfactory should continuously update its internal understanding of the
project.

That internal work is ambient and not itself a user-facing feature.

The system may:

- reinterpret recent input
- strengthen or weaken confidence
- discover new tensions
- sharpen labels
- propose better relationships

without narrating its internal process constantly.

### 6.2 Silent visible updates are allowed only for low-risk changes

Darkfactory may apply low-risk visible updates automatically.

Current locked MVP set:

- state
- confidence
- suggested labels
- tension markers
- suggested relationships

These updates should be visible in the artifact, but they do not require
separate approval before they appear.

### 6.3 Transparency for low-risk silent updates

Silent does **not** mean invisible.

When a low-risk visible update is applied, the affected area should show a
restrained freshness cue.

The default product treatment should be:

- the area shows that it changed recently
- the user can inspect what changed on demand
- the user does not need to visit a separate review desk just to understand
  that something tightened

The cue should remain lighter than an alert or blocking notice.

## 7. Pending Revisions

### 7.1 Meaning-changing updates must not apply silently

If the system wants to make a change that would materially alter project
meaning, it should not silently rewrite the artifact.

Instead, it should create a **pending revision** in the affected area.

### 7.2 Pending revisions live in context

Pending revisions should appear inside the affected area, not in a separate
global review queue by default.

This keeps the review work:

- local
- comprehensible
- tied to the area the user is actually shaping

### 7.3 What qualifies as a pending revision

At minimum, the following must become visible proposed changes rather than
silent updates:

- changing area identity
- changing accepted major relationships
- changing the current north-star definition
- folding a speculative idea into canonical truth when that materially changes
  project direction
- any visible restructure that materially changes project shape

### 7.4 Pending revisions are non-blocking by default

Most pending revisions should remain:

- visible
- reviewable
- non-blocking

The user should usually be able to continue shaping the area while a revision
is waiting.

This preserves initiative without making the system controlling.

## 8. Conflict Escalation

### 8.1 Direct conflicts deserve stronger treatment

If a proposed revision exposes a direct conflict with already accepted
structure, the system may escalate the treatment beyond a normal pending
revision.

Escalation means:

- the conflict is made more visually obvious
- the user is steered toward resolving it
- conflicting edits may be temporarily gated when continuing would multiply
  incoherence

### 8.2 Escalation should stay narrow

Do not let every disagreement become a blocking event.

Only direct, structurally meaningful conflicts should receive the stronger
treatment.

Ordinary improvements, alternate phrasings, or non-critical refinements should
remain pending and non-blocking.

## 9. What Must Not Happen

The system must not:

- silently rewrite user-accepted project meaning
- mutate the project picture in a way that feels arbitrary
- force the user into a separate review inbox for every meaningful system
  proposal
- turn every disagreement into a blocker
- hide major shape changes behind a subtle freshness cue

## 10. Touched Surfaces

Likely primary implementation surfaces:

- `planner-solid/src/routes/sessions/session-workspace-screen.tsx`
- `planner-solid/src/routes/sessions/session-workspace-controller.ts`
- `planner-solid/src/routes/sessions/session-workspace-view.ts`
- `planner-solid/src/lib/workspace.ts`
- `planner-solid/src/lib/prompt-bank.ts`
- `planner-solid/src/app.css`
- relevant session-route tests and browser proof

This child spec should avoid reopening:

- prompt-bank truth
- answer-level continuity
- broad route layout work

unless the autonomy work proves one of those contracts is still a blocker.

## 11. Acceptance Criteria

1. the system can continuously reconverge internally without narrating its
   machinery constantly
2. low-risk visible updates can appear automatically
3. low-risk visible updates surface restrained transparency cues on the
   affected area
4. meaning-changing updates appear as pending revisions inside the affected
   area
5. pending revisions are non-blocking by default
6. direct conflicts can escalate and temporarily gate conflicting edits
7. accepted user meaning is not silently rewritten

## 12. Verification Plan

Before this child spec is promoted or implemented, it should be checked
against:

- the parent brief for trust-boundary consistency
- the first-reveal child spec so freshness cues do not overwhelm the main
  picture
- the area-workspace child spec so revisions stay local to the area
- one product walkthrough proving the system feels active but not slippery

## 13. Rollback / Fallback

If the full pending-revision model proves too broad in one pass, the fallback
is:

- keep low-risk silent updates plus freshness cues
- keep major changes visible in-area
- reduce the sophistication of the revision UI before reducing the trust
  boundary itself

The fallback is **not**:

- reverting to silent meaning-changing edits
- moving all revisions into a detached global review system
- requiring approval for every low-risk update

## 14. Open Questions

The main remaining blocker after this child spec is:

- what exact first bounded MVP slice should be implemented first using the now-
  locked first-reveal, area-workspace, and autonomy rules

Secondary non-blocking follow-ons:

- the exact visual treatment of freshness cues
- whether pending revisions should expose one proposed alternative or a small
  set
- whether direct conflicts gate all work in the area or only the specific
  conflicting edit path

## 15. Readiness Judgment

This spec is **draft but bounded**.

It is strong enough to serve as the third planning child under the parent
brief.
The next planning move should no longer be foundational product discovery.
The next move should be the bounded MVP execution-slice spec.
