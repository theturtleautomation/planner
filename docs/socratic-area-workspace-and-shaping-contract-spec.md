# Socratic Area Workspace And Shaping Contract Spec

**Status:** draft  
**Date:** 2026-04-03  
**Parent:** [Socratic Project Picture And Convergence Workspace Spec](/home/thetu/planner/docs/socratic-project-picture-and-convergence-workspace-spec.md)  
**Related Planning:** [Socratic Project Picture First-Reveal Screen Spec](/home/thetu/planner/docs/socratic-project-picture-first-reveal-screen-spec.md), [Socratic Project Picture MVP Path And Gap Analysis Spec](/home/thetu/planner/docs/socratic-project-picture-mvp-path-and-gap-analysis-spec.md), [Planner SolidStart Phase 34 Session Question-Bank Workspace Reset Spec](/home/thetu/planner/docs/planner-solidstart-phase-34-session-question-bank-workspace-reset-spec.md), [Planner SolidStart Phase 38 Socratic Multimodal Command Desk Spec](/home/thetu/planner/docs/planner-solidstart-phase-38-socratic-multimodal-command-desk-spec.md), [Planner SolidStart Phase 39 Session Commit Continuity And Prompt-Bank Merge Spec](/home/thetu/planner/docs/planner-solidstart-phase-39-session-commit-continuity-and-prompt-bank-merge-spec.md), [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Review:** 2026-04-03 bounded planning pass after the first-reveal child spec and the product decision that one area should expose a small number of meaningful pressure points, usually 2 to 4, with one visually dominant

## 1. Purpose

Define what happens after the user enters one area from the project picture.

This child spec exists to answer:

- what should the area workspace feel like?
- how much internal structure should it expose on day one?
- how should shaping work without collapsing back into chat, forms, or a nested
  mini-map?

It does **not** define the full silent-update boundary across the whole system.
That remains a separate autonomy child-spec concern.

## 2. Problem

The first-reveal screen can only establish the right center of gravity if
entering one area does not immediately collapse back into the wrong interaction
model.

The likely failure modes after entering an area are:

- a generic form
- a chat pane with attached context
- a raw document editor
- a recursive internal map that turns one area into its own full system

The repo already has implemented prompt-bank and question-bank interaction
substrate, but that substrate is still too prompt-forward to count as the new
area-shaping contract by itself.

## 3. User Outcome

When a user enters one area, they should feel:

- quickly leveled in the current context
- shown the few most meaningful pressure points
- able to improve that part of the project without losing the whole
- supported by the system's judgment without being buried in questions

The area workspace should feel like focused shaping, not like opening a new
application inside the application.

## 4. Scope

### In Scope

- the structure and hierarchy of the area workspace
- the maximum day-one substructure level
- the shaping interaction model inside one area
- the role of suggestions, inline edits, and discussion
- the role of local versus global capture while inside an area
- the contract for how one area should and should not behave in MVP

### Out Of Scope

- the first visible project-picture screen
- full overlay-system design
- the full convergence/autonomy child contract
- backend transport or schema details
- implementation-ready component decomposition

## 5. Locked Inputs From Parent Planning

This child spec assumes:

- the first visible surface is a calm area-based project picture
- the top-level areas are:
  - `Transformation`
  - `Actors`
  - `Constraints`
  - `Approach`
  - `Pressure`
- the dominant interaction loop is:
  - picture -> area -> shape -> picture
- questions remain one shaping tool, not the product's identity
- the product must stay behavior-first, not graph-first
- object editing should dominate where possible

## 6. Area Workspace Contract

### 6.1 Area entry must level the user first

When a user opens an area, the first job is orientation.

The user should be able to understand quickly:

- what this area currently is
- what state it is in
- why it matters now
- what it most strongly relates to

This context should be compact.
It should then recede into an easily retrievable layer rather than staying
permanently pinned.

### 6.2 One area must not become a recursive mini-system

The area workspace must not unfold into a full internal map on day one.

Locked MVP rule:

- one area should expose a small number of meaningful pressure points
- usually 2 to 4
- with one visually dominant

This is enough to give the area internal shape without turning the product into
recursion.

### 6.3 Pressure points are the real substructure

The area should not expose arbitrary nested structure.

It should expose only the sub-parts that materially matter now, such as:

- unresolved definitions
- weak assumptions
- tensions with another area
- obvious shaping opportunities

If a sub-part would not materially change the next move, it should not appear
as first-class visible area substructure in MVP.

### 6.4 The dominant response model inside an area

The primary shaping responses should be:

- `accept`
- `edit inline`
- `discuss`

Interpretation:

- `accept` = adopt a meaningful system suggestion with minimal friction
- `edit inline` = directly refine the visible project object
- `discuss` = open a deeper conceptual exchange only when the issue is
  ambiguous, structural, or high-leverage

`Discuss` must not become the default for ordinary shaping.

### 6.5 Editing should be object-first

The default editable unit should be a project object rather than raw prose.

Representative editable objects:

- a label
- a claim
- a relationship
- a definition
- a constraint
- a proposed structure

Text remains necessary, but the area should not degrade into a paragraph editor
if structure would be more truthful.

### 6.6 Freeform input still matters inside an area

The user must still be able to add freeform thoughts inside an area.

But the system should structure those thoughts aggressively when the pieces
would behave differently inside the project.

The restraint rule remains:

- only split when the distinction changes role, relationship, state,
  importance, or likely next move

### 6.7 Suggestions must feel like help, not overhead

If the system surfaces a suggestion inside an area, the easiest response should
remain lightweight.

The user should not have to:

- write long replies
- open a chat thread
- navigate away to another surface

to do ordinary shaping work.

### 6.8 Local and global capture should coexist cleanly

Inside an area, the user should still be able to:

- add something directly to the focused area
- send a thought through the global capture path if it is not clearly local

The area workspace should not try to absorb every thought just because the user
happens to be inside that area.

## 7. Design Direction

The area workspace should feel:

- focused
- sharp
- supportive
- compact
- context-rich

It should not feel:

- like a form wizard
- like a chat client
- like a document editor
- like a nested control room

The visual hierarchy should communicate:

- one dominant pressure point
- a few secondary pressure points
- restrained access to deeper explanation

## 8. What The Area Workspace Must Not Become

The area workspace must not:

- unfold into a full second project picture
- show a dense internal graph by default
- require discussion for routine shaping work
- mirror the same information in multiple panes
- force the user to choose between too many equally primary actions

## 9. Touched Surfaces

Likely primary implementation surfaces:

- `planner-solid/src/routes/sessions/session-workspace-screen.tsx`
- `planner-solid/src/routes/sessions/session-workspace-controller.ts`
- `planner-solid/src/routes/sessions/session-workspace-view.ts`
- `planner-solid/src/lib/workspace.ts`
- `planner-solid/src/app.css`
- relevant session-route browser proof and focused workspace tests

This child spec should avoid reopening:

- prompt-bank truth
- answer-level continuity
- project-only entry

unless the area-workspace work exposes a concrete blocker in those already-
implemented contracts.

## 10. Acceptance Criteria

1. entering an area quickly levels the user in current context
2. an area exposes only a small number of meaningful pressure points
3. one pressure point is visually dominant
4. the area does not become a recursive mini-map
5. `accept`, `edit inline`, and `discuss` form the dominant response model
6. routine shaping work does not require dropping into discussion
7. editing remains object-first where possible
8. the area workspace cannot be mistaken for a form, chat pane, or document
   editor

## 11. Verification Plan

Before this child spec is promoted or implemented, it should be checked
against:

- the parent brief for drift toward graph-first or prompt-first interaction
- the first-reveal screen child spec for hierarchy conflicts
- one product walkthrough proving the area remains bounded and non-recursive
- future route-level browser proof once a bounded execution slice is selected

## 12. Rollback / Fallback

If the full 2-to-4-pressure-point model proves too broad in one pass, the
fallback is:

- keep one dominant pressure point
- keep one or two secondary points visible
- keep the interaction model the same

The fallback is **not**:

- reverting to a prompt-first question stack inside the area
- unfolding a mini-map to compensate
- shifting normal shaping work into chat

## 13. Open Questions

The main blocker that still remains after this child spec is:

- the exact autonomy and visible-review boundary for area-level updates once
  the system is continuously reconverging in the background

Secondary non-blocking follow-ons:

- whether `discuss` should open inline, drawer-based, or side-panel conversation
- whether the area should show one next move or one next move plus one
  alternate
- whether local freeform capture should default to a project object draft or a
  short text seed

## 14. Readiness Judgment

This spec is **draft but bounded**.

It is specific enough to keep the MVP from drifting into recursive complexity.
It is not yet ready for implementation because the autonomy child-spec still
needs to hard-cut what area-level updates can happen silently versus what must
be surfaced as visible review.
