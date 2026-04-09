---
name: project-ledger
description: Show the canonical planner ledger, validate its linkage, and surface what needs what next across the project.
---

# Project Ledger

Use the planner-wide canonical ledger to level-set the project and identify the next valid workflow mode.

## Use When

- the user asks "where are we at?"
- the user wants to know what needs deep-interview / ralplan / ralph / testing / analysis next
- a session needs project-wide grounding before planning or execution
- durable artifacts are starting to sprawl and need linkage/status clarity
- the user wants to know whether the repo is actually advancing feature delivery versus only reorganizing planning/spec artifacts

## Canonical Sources

- `.omx/ledger/planner-ledger.json`
- `.omx/ledger/current-status.md`
- `scripts/project-ledger.mjs`

## Commands

- `npm run project:status` — print the current ledger summary
- `npm run project:ledger:validate` — validate ledger structure and artifact links
- `npm run project:ledger:refresh` — regenerate the readable status surface
- `npm run project:ledger:auto` — apply bounded ledger/status/routing automation and refresh the readable surface
- `npm run test:ledger` — verify the ledger command/model

## Workflow

1. Read `.omx/ledger/current-status.md`.
2. If the request depends on current routing/tracking truth, run:
   - `npm run project:ledger:validate`
   - `npm run project:status`
3. Inspect the current working tree when feature-progress or review guidance matters:
   - `git status --short`
   - optionally `git diff --stat` or `git diff --name-only`
   - classify whether recent work is primarily:
     - **feature/code delivery**
     - **tests/verification**
     - **planning/spec synchronization**
     - **mixed**
4. If durable artifacts were added or statuses changed and deterministic automation should apply, run:
   - `npm run project:ledger:auto`
   - The automation path may now use repo-graph evidence directly when evaluating bounded routing / next-mode mutations.
   - Treat any resulting routing mutation as inspectable; the automation trace should explain why it happened.
   - Confidence/provenance may also appear in durable ledger state when the trust policy requires it.
   - Operators should use the human-readable report at `.omx/ledger/automation-report.md` for rolling history rather than relying on the compact status surface alone.
5. If durable artifacts were added but automation should not mutate state, run:
   - `npm run project:ledger:refresh`
6. Compare the routed queue with the working tree:
   - If the queue is planning-heavy but the working tree contains real code/test diffs, say that explicitly.
   - If the queue is still mostly planning/deep-interview and there are **no** code-bearing changes, call out the risk of spec churn.
   - If a major feature slice appears implemented or partially implemented, surface whether `$code-review` is now a more appropriate next move than more planning.
7. Use the routing queue to identify the next valid OMX move, but bias the final guidance toward **feature progression**:
   - prefer execution-ready feature work over additional reorganization when a bounded slice already exists
   - recommend `$code-review` when recent code-bearing changes need quality/security/maintainability validation
   - recommend more planning only when a real requirement or branch-choice ambiguity still blocks delivery

## Output Expectations

Provide a compact status report covering:
- active initiatives/workstreams
- queued next-mode work
- deferred items still alive
- important risks or blockers
- next valid move

Also include:
- **Feature Progress Signal** — is the repo currently advancing code/features, tests/verification, docs/specs, or mostly reorganizing planning truth?
- **Code Review Guidance** — when `$code-review` is the best next move, say so explicitly and explain why
- **Feature-Forward Recommendation** — if there is tension between “canonical queue correctness” and “actually shipping the next slice,” call it out directly

When the user is worried about endless spec churn, explicitly answer:
- whether the current state is mostly planning vs implementation
- whether a bounded feature slice already exists and should be executed next
- whether the repo would benefit more from `$code-review` than another planning pass

## Feature-Progress Bias

Project-ledger should not stop at “what is routed next” if that would quietly
push the repo deeper into planning-only motion.

When reading the ledger:
- distinguish **planning center** from **delivery center**
- identify whether the current queue is blocked by real ambiguity, or merely
  following historical planning inertia
- prefer guidance that helps the user ship the next bounded slice when the
  necessary plan/spec already exists

Examples:
- If a feature PRD exists and code changes are already in flight, suggest
  `$code-review` or `$ralph` on that feature before suggesting more parent-doc
  cleanup.
- If multiple child slices are already implemented, say that the planning map
  is likely good enough and the repo should move into feature execution or
  review.
- If the user asks for project status after several doc-only passes, explicitly
  call out that the repo is reorganizing specs more than delivering features.

## Code-Review Handoff Guidance

Recommend `$code-review` when any of these are true:
- recent changes touch product code, tests, or runtime behavior
- a bounded feature slice appears implemented or partially implemented
- the user is about to commit/merge
- the user asks “are we actually progressing?” and the working tree shows code-
  bearing changes that need quality validation

Do **not** default to `$code-review` when:
- the session only changed planning/spec/ledger docs
- the next real blocker is still a missing branch-choice decision
- there is no meaningful code-bearing diff to review yet

When recommending `$code-review`, name the reason plainly, for example:
- “Recent changes are code-bearing and the next best move is quality validation, not more planning.”
- “The repo has already crossed from planning into delivery on this slice; run `$code-review` before additional scope expansion.”
- “This looks like docs/ledger-only motion; `$code-review` would not add much yet — choose the next executable feature slice instead.”

## Refuse To

- treat the ledger as optional when project-wide status truth matters
- invent lifecycle state without updating the canonical ledger
- leave durable new work unlinked when the ledger is in scope
- give queue-only guidance that hides the difference between planning progress and feature progress
