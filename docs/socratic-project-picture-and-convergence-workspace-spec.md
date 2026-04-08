# Socratic Project Picture And Convergence Workspace Spec

**Status:** draft  
**Date:** 2026-04-03  
**Parent:** [Socratic SolidStart Greenfield Platform Spec](/home/thetu/planner/docs/socratic-solidstart-greenfield-platform-spec.md)  
**Related Planning:** [Planner SolidStart Platform Direction Spec](/home/thetu/planner/docs/planner-solidstart-platform-direction-spec.md), [Socratic Lobby Master-Detail Local Workspace Spec](/home/thetu/planner/docs/socratic-lobby-master-detail-local-workspace-spec.md), [Socratic Lobby Local-First Browser Architecture Review](/home/thetu/planner/docs/socratic-lobby-local-first-browser-architecture-review.md), [Planner SolidStart Phase 38 Socratic Multimodal Command Desk Spec](/home/thetu/planner/docs/planner-solidstart-phase-38-socratic-multimodal-command-desk-spec.md), [Planner SolidStart Phase 39 Session Commit Continuity And Prompt-Bank Merge Spec](/home/thetu/planner/docs/planner-solidstart-phase-39-session-commit-continuity-and-prompt-bank-merge-spec.md), [Planner SolidStart Phase 40 Project-Only Entry And Stale-Draft Hardening Spec](/home/thetu/planner/docs/planner-solidstart-phase-40-project-only-entry-and-stale-draft-hardening-spec.md), [Blueprint Project Root And CodeGraph Integration](/home/thetu/planner/docs/blueprint-project-root-codegraph-integration.md), [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Source Review:** 2026-04-03 product review session covering the Socratic lobby future-state, direct inspection of the current Socratic and blueprint planning artifacts, and user-provided external research notes on AI-native workspaces, C4, and EventStorming used as non-binding input for product diagnosis rather than as canonical product truth

## 1. Executive Judgment

The current SolidStart Socratic route is denser, calmer, and more truthful than
the earlier lobby work, but it still frames the product around prompt handling
and answer progression more than around whole-project understanding.

That is now the wrong center of gravity.

The next future-state target for the Socratic route is not:

- a better question form
- a wider command desk
- a prettier blueprint graph
- a chat surface with attached structure

It is a **project-picture-first collaborative workspace**.

In this model:

- the primary artifact is a living project picture
- the picture is the user's main orientation surface
- questions become one shaping tool rather than the product itself
- a hidden blueprint-like truth model keeps the system rigorous underneath
- the system continuously reconverges the project in the background so AI speed
  does not dissolve whole-project meaning

This spec is a v1 product brief for that future-state direction.

It is intentionally more comprehensive than a lightweight brief, but it is not
yet ready for bounded implementation. The main visual form, the first delivery
slice, and several high-leverage interaction boundaries still need tightening.

## 2. Product Intent

The Socratic lobby exists to solve one core problem:

- AI can generate detail faster than a human can preserve whole-project
  understanding

Darkfactory must therefore behave like a colleague that helps shape a project
without letting the user drown in fragments, rabbit holes, or local optimizations.

The user should feel:

- the idea is becoming clearer
- the system understands the real goal
- weak thinking is being challenged
- the whole still makes sense even as detail grows
- the work remains easy to navigate and calm to use

## 3. User Outcome

After this future-state direction is realized, a user entering the lobby should
be able to tell quickly:

- what this project currently is
- what parts are load-bearing
- what parts are defined
- what parts are weak, unclear, incomplete, or conflicted
- where deeper exploration or research lives
- what the best next moves are

The user should be able to:

- enter directly into the project picture instead of a blank prompt flow
- open one area and shape it without losing the whole
- add ideas either globally or into a focused area
- understand tension without being interrupted constantly
- recover the whole project state instantly when deep in a rabbit hole
- leave and return later to the same living project rather than to a stale file

## 4. Problems To Solve

### 4.1 The current product still privileges prompts over project shape

The current route is still fundamentally prompt-bank and answer-flow first.
That keeps the user close to what the system is asking, but not close enough to
what the project is becoming.

### 4.2 AI-generated detail can outrun project meaning

Without continuous reconvergence, the product will accumulate:

- local answers
- useful fragments
- side research
- draft refinements

while still failing to answer the more important question:

- what does all of this amount to now?

### 4.3 The current blueprint concept is too literal for the primary UX

The repo already has a useful blueprint model, but earlier blueprint surfaces
have often been too internal, too graph-literal, or too architecture-flavored
to serve as the humane product surface.

### 4.4 Relationship truth is hard to preserve without visual clutter

The product needs first-class relationship legibility, but if every dependency
or influence is shown at once the picture collapses into a dense diagram that
normal users will not trust or enjoy using.

### 4.5 Guidance can easily degrade into either chat drift or dashboard clutter

If the system explains too much, the workspace becomes noisy and performative.
If it explains too little, the workspace becomes arbitrary and untrustworthy.

## 5. Scope

### In Scope

- the future-state product model for the Socratic lobby
- the role of the project picture as the primary user-facing artifact
- the hidden truth-model relationship between the project picture and the
  existing blueprint direction
- the core interaction loop for shaping one project area at a time
- visible area state language
- visible action language
- first-class relationship behavior
- soft-idea or seed handling
- overlay and reorientation model
- autonomy boundaries between internal convergence and user-committed meaning
- spatial stability rules for the main picture

### Out Of Scope

- implementation-ready backend contracts
- exact frontend component decomposition
- transport, schema, or persistence details
- final visual art direction
- specific rendering engines, layout algorithms, or graph libraries
- implementation task breakdown
- readiness promotion to a bounded execution slice

## 6. Core Product Decisions

### 6.1 The main thing is the project picture

The first screen should be the project picture.

The product should not start from:

- a blank prompt box
- a stack of questions
- a document draft
- a generic graph tool

The user should meet the current shape of the project first, with visible cues
about where attention is needed.

Locked MVP direction for the first visual form:

- the picture should be a calm area-based project picture
- major zones should remain spatially stable
- selective tension markers should surface pressure
- the dominant visual should not be a node-edge graph

### 6.2 The project picture is not the raw blueprint

The user-facing artifact should be a fluid, humane expression built on top of a
hidden blueprint-like truth model.

That hidden model can remain rigorous about:

- what areas exist
- what is foundational versus downstream
- what relationships are real
- what tensions or conflicts are active
- what matters next

But the visible surface must stay more humane and product-shaped than a raw
architecture graph.

Locked MVP top-level area model:

- `Transformation`
- `Actors`
- `Constraints`
- `Approach`
- `Pressure`

Working meaning:

- `Transformation` = what changes if the project succeeds
- `Actors` = who is affected or served
- `Constraints` = the decisive limits that shape the project
- `Approach` = the current shape of the solution without collapsing into
  backlog or implementation detail
- `Pressure` = the conflicts, weaknesses, and unresolved tensions most likely
  to break coherence

### 6.3 The system must think behavior first, structure second

The correct convergence order is:

1. core transformation
2. primary actors
3. decisive constraints
4. broader project shape

The system must not begin by assuming components, containers, or formal system
structure before it understands what the project is trying to change.

### 6.4 The user's actual goal is the north star

The user's real goal must remain the most durable truth across the entire
session.

If the system loses that, it can still look intelligent while quietly becoming
wrong.

### 6.5 The user owns the final call

Darkfactory should guide, sharpen, challenge, and synthesize, but the user must
retain final authorship and decision authority.

The system may disagree visibly.
It may suggest stronger alternatives.
It may pressure weak reasoning.
It may not silently replace the user's intended project with its own.

### 6.6 The product should borrow selectively from research, not become it

The user-provided C4 and EventStorming research is useful, but neither model
should become the visible product identity.

Useful borrowings:

- from C4: progressive zoom and layered structural understanding
- from EventStorming: behavior-first reasoning, explicit tensions, and
  chronology-aware shaping

Rejected as primary UX identities:

- a literal architecture tool
- a sticky-note workshop UI
- an "agentic knowledge graph" as the product story users must inhabit

## 7. Core Artifact Model

### 7.1 One canonical current understanding must exist

Darkfactory should always maintain one living, canonical understanding of what
the project currently is.

That canonical understanding is the center of gravity for:

- questions
- suggestions
- contradictions
- next moves
- seeds
- review overlays
- background reconvergence

### 7.2 The project picture is the primary truth surface

If Darkfactory becomes smarter, the first visible proof should be:

- the project picture gets sharper

Not:

- logs
- self-reporting status copy
- abstract system chatter

The prompt-bank and answer substrate may continue underneath this artifact, but
they should no longer be the user's primary orientation surface.

### 7.3 The picture must stay aligned to the hidden truth model

The user-facing picture must remain truthful about at least:

- what areas exist
- what areas are foundational
- what areas are downstream
- what is weak versus strong
- what tensions are real
- what should matter next

If the visible picture drifts from those, the system becomes decorative rather
than intelligent.

## 8. Interaction Model

### 8.1 Primary interaction loop

The dominant loop is:

- project picture -> enter area -> shape area -> picture updates

Other interaction paths may exist, but they must remain secondary.

### 8.2 Area entry should level the user first

When the user enters an area, the system should first expose compact current
context:

- what this area is
- what state it is in
- why it matters now
- what it relates to

That context should then recede into an easily retrievable layer rather than
remaining pinned permanently.

### 8.3 Area work is not generic chat or generic form fill

The area workspace should feel like focused shaping of one part of the project.

It should not degrade into:

- a form
- a long document editor
- a full chat window

The primary response model inside an area should be:

- accept
- edit inline
- discuss

`Discuss` should be reserved for conceptual, ambiguous, or structurally
consequential moments, not for ordinary wording changes.

### 8.4 Editing must be hybrid, but object-first

The default editable unit should be a project object:

- a label
- a claim
- a relationship
- a definition
- a constraint
- a proposed structure

Text remains necessary, but object editing should dominate where possible so
the artifact does not decay back into notes.

### 8.5 Freeform input is mandatory in two places

The user must be able to add ideas:

- directly into the area they are focused on
- globally from outside any one area

Global additions should receive one best placement suggestion from the system.
If the input is too vague to place honestly, the system should ask one sharp
clarifying question plus suggested interpretations before pretending it knows
where the input belongs.

### 8.6 Freeform input should be structured aggressively when it matters

When freeform text contains multiple ideas, the system should split it into
separate objects when those pieces would behave differently in the project.

The restraint rule is:

- only split when the distinction changes role, relationship, state, importance,
  or likely next move

## 9. Guidance Model

### 9.1 The system should act like a colleague, not a reviewer

If Darkfactory raises an issue, it must also offer a suggested path forward.

It should not bring problems to the user without first-pass thinking.

### 9.2 Guidance should create visible pressure, not interruptions

The default pattern is:

- show pressure in the picture
- show resolution paths in overlays or local area work

Not:

- constant modals
- continuous chat interruptions
- a stream of alerts

### 9.3 Critique should be selective

The system should usually surface one major critique at a time, even if it is
tracking more internally.

It should prioritize the critique with the highest leverage on project shape,
especially when it affects foundational truth.

### 9.4 "Next moves" should be a small set of invitations

The system should not reduce guidance to one commanded next step or to a noisy
task feed.

Visible next moves should remain grouped invitations that can shift
contextually, with the strongest current verbs being:

- `explore`
- `review`
- `clarify`

## 10. Visible State And Relationship Language

### 10.1 State words

The main visible state language should be:

- `conflicted`
- `unclear`
- `incomplete`
- `defined`

Meaning:

- `unclear`: the system does not yet know what this area properly is
- `incomplete`: the area is understood, but not yet structurally sufficient
- `conflicted`: this area is in direct contradiction or serious tension with
  another project truth
- `defined`: good enough for now, not final forever

Visible precedence:

1. `conflicted`
2. `unclear`
3. `incomplete`
4. `defined`

Bias rule:

- if the system is uncertain whether an area is `unclear` or `incomplete`, it
  should bias toward `unclear`

### 10.2 Action words

Action words should remain distinct from state words.

Current visible action language:

- `explore`
- `review`
- `clarify`

These are invitations, not diagnoses.

### 10.3 Relationship types

Relationships should be first-class in the visible picture.

The baseline user-facing relationship vocabulary should be:

- supports
- depends on
- conflicts with
- influences

The product must not render every relationship by default.

Default visible density should emphasize:

- foundational dependencies
- critical conflicts

Additional relationship detail belongs in overlays or drill-down.

## 11. Seeds, Soft Material, And Nearby Instability

### 11.1 Seeds should exist, but remain subordinate

The user needs a place for soft ideas, but the main project picture must not be
polluted by unresolved debris.

Seeds should be:

- lightly visible
- subordinate in emphasis and authority
- able to attach near relevant areas

### 11.2 Seeds should not become a junk drawer

The system should not treat every seed as equally important.

A seed should matter again only when it:

- affects a foundational area
- conflicts with a current decision
- unlocks a meaningful path
- becomes relevant to the area the user is actively shaping

### 11.3 Nearby unresolved seeds may lower confidence

Confidence should reflect:

- the quality of the area itself
- the instability introduced by unresolved nearby material

This prevents false certainty around areas that look clean only because soft
material was visually ignored.

## 12. Overlays And Recoverability

The product should optimize for **always recoverable**, not **always visible**.

The user should be able to retrieve, quickly and from anywhere:

- the whole project picture
- the current pressure points
- the most meaningful next moves

Essential overlay families:

- project overview
- pressure or unresolved areas
- next moves

Optional secondary families:

- under-the-hood hierarchy view
- relationship detail view
- provenance or source view
- historical comparison or reorientation view

These overlays should support the main artifact, not compete with it.

## 13. Autonomy, Convergence, And Trust

### 13.1 Internal convergence should be continuous

Darkfactory should continuously update its internal understanding of the
project in the background.

That convergence should be ambient, not performative.

The user should mostly feel:

- better nudges
- sharper relabeling
- more coherent project shape

not the machinery that produced them.

### 13.2 Internal convergence and visible edits are different things

The system may autonomously update low-risk visible signals such as:

- confidence
- state
- visible tensions
- suggested labels
- suggested relationships

But it must not silently rewrite user-committed meaning.

Locked MVP boundary:

Darkfactory may silently update:

- state
- confidence
- suggested labels
- tension markers
- suggested relationships

Darkfactory may not silently update:

- area identity
- accepted major relationships
- the current north-star definition
- any visible change that materially alters project shape

### 13.3 User-committed meaning must remain protected

The system should not silently rewrite:

- area identity
- major relationships
- the current north-star definition
- choices the user explicitly accepted
- structural shifts large enough to change project shape

If the system thinks a stronger alternative exists, it should keep the user's
path visible while surfacing its disagreement and the likely consequence.

### 13.4 No silent goal drift

Darkfactory may clarify, connect, relabel, and elevate patterns.
It may not silently change what project the user is trying to build.

## 14. Spatial Stability

Major areas must remain spatially stable enough that users can build memory of
the picture.

Recommended stability hierarchy:

- major areas: strongly stable
- important visible tensions: semi-stable
- detailed internals: more fluid

If the main picture rearranges itself too freely, the product will feel clever
and unusable rather than intelligent and calm.

## 15. Maturity Weighting

Early in the session, the picture should visibly privilege:

- the core transformation
- the primary actors
- the decisive constraints

As those become strong enough, the picture may flatten its emphasis so broader
areas earn more visual authority.

The maturity gate should be:

- the transformation, actors, and decisive constraints are all strong enough
  that they are no longer the main source of uncertainty

## 16. Touched Surfaces

This future-state direction is likely to touch:

- `planner-solid/src/routes/sessions/session-workspace-screen.tsx`
- `planner-solid/src/routes/sessions/session-workspace-controller.ts`
- `planner-solid/src/routes/sessions/session-workspace-view.ts`
- `planner-solid/src/lib/prompt-bank.ts`
- `planner-solid/src/lib/workspace.ts`
- `planner-solid/src/lib/session-transport.ts`
- `planner-solid/src/routes/projects/[projectSlug].tsx`
- `planner-solid/src/app.css`
- `planner-schemas/src/artifacts/socratic.rs`
- `planner-schemas/src/artifacts/blueprint.rs`
- `planner-core/src/pipeline/steps/socratic/`
- `planner-core/src/pipeline/steps/`
- related browser proof and route-level tests

This list is directional only.
This spec does not yet claim that any one bounded slice is ready to touch all
of these surfaces.

## 17. Acceptance Criteria

This product brief is coherent only if the future implementation can satisfy at
least the following:

1. the first thing the user sees is the project picture, not a blank prompt or
   a generic question stack
2. the project picture makes it easy to understand what the project currently
   is, what is weak, and what matters next
3. entering one area preserves orientation and levels the user quickly in
   current context
4. the dominant loop remains picture -> area -> shape -> picture
5. questions remain available as a shaping tool, but no longer define the
   product's entire identity
6. relationships are visually meaningful without forcing the default view into
   a spaghetti diagram
7. seeds remain subordinate and do not muddy the main project artifact
8. the system continuously improves the artifact without silently rewriting
   user-committed meaning
9. major areas remain spatially stable enough for return navigation and deep
   work to feel calm
10. leaving and returning restores the whole current project state first rather
   than a narrow local task state

## 18. Verification Plan

Before this direction is promoted into bounded implementation work, it should
be pressure-tested through:

- one tightened visual and interaction brief for the first project-picture
  screen
- one explicit autonomy boundary review covering what the system may update
  silently versus what requires user-visible review
- one product-walkthrough critique proving the primary loop does not collapse
  back into chat, form fill, or raw graph manipulation
- comparison against the current session route so the future-state delta stays
  honest
- future implementation proof through route-level browser validation and the
  relevant contract/unit tests once a bounded slice is selected

## 19. Rollback / Fallback

If this future-state direction is too broad to implement in one coherent move,
the fallback should not be to keep widening the existing prompt-bank command
desk indefinitely.

The bounded fallback path is:

- preserve the current command-desk route as the implemented baseline
- add one project-picture-first future-state slice at a time
- prove the picture can become the primary truth surface before replacing the
  existing route hierarchy wholesale

The product should not:

- ship a decorative picture layer that does not carry real truth
- turn the current route into a generic graph editor
- fork into one "serious" internal blueprint surface and one "pretty" user
  surface that drift apart

## 20. Open Questions

The following still materially block readiness promotion:

1. how much substructure should one area expose before it becomes a recursive
   mini-system?
2. what are the minimum visible causes of confidence loss on the main surface?
3. which overlays are essential on day one versus deferred?
4. how should the project picture express research-heavy areas without becoming
   dashboard-like?
5. how should the system show that a major shape shift happened without feeling
   jumpy or theatrical?
6. what is the first bounded slice that proves the picture can displace prompt
   flow as the user's primary point of orientation?

The following are now directionally locked for MVP and should not be reopened
without specific contradictory evidence:

- the first visual form should be a calm area-based project picture, not a
  node-edge graph
- the top-level areas should begin with `Transformation`, `Actors`,
  `Constraints`, `Approach`, and `Pressure`
- low-risk silent updates may affect state, confidence, labels, tension
  markers, and suggested relationships, but may not rewrite area identity,
  accepted major relationships, the north-star definition, or overall project
  shape

## 21. Readiness Judgment

This spec is **not ready for implementation**.

It is strong enough to serve as a real v1 product brief and to replace the
lighter session summary produced during the conversation, but it still lacks
the tightened first-slice boundary required for truthful execution.

The next planning move should be one of:

- tighten this spec into a narrower child slice focused on the first project-
  picture screen and its interaction contract
- tighten the autonomy boundary into one separate child spec if that becomes
  the main execution risk

Implementation should not begin directly from this parent brief.
