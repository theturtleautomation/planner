# Planner Agent Instructions

## Workflow

- Start every session by reading `.omx/ledger/session-start-and-doc-index.md`.
- Read `.codex/project-skill-config.md` before using repo workflow skills.
- Review `.omx/ledger/project-plan.md` and `.omx/ledger/current-status.md` before choosing the next planning or implementation move.
- Prefer `.codex/skills/project-ledger/` for bootstrap/status guidance.
- Prefer `.codex/skills/deep-interview/` or `.codex/skills/ralplan/` for requirements and planning.
- Prefer `.codex/skills/ralph/`, `.codex/skills/team/`, or `.codex/skills/autopilot/` for bounded execution after planning.

Legacy repo-local workflow shims (`project-bootstrap`, `spec-lifecycle`,
`delivery-cycle`) were intentionally removed in favor of these OMX-native
surfaces.

## Preferred Delivery Pattern

1. Bootstrap the session from the OMX session-start index, project plan, ledger surfaces, and the binding file.
2. Tighten or create the relevant planning artifact.
3. Implement only from a planning artifact that is ready enough for bounded execution.
4. Verify and review against the artifact and touched system surfaces.
5. Synchronize docs and statuses.
