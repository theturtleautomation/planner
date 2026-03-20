# Planner Agent Instructions

## Workflow

- Start every session by reading `docs/session-start-and-doc-index.md`.
- Read `.codex/project-skill-config.md` before using the generalized process skills.
- Review `docs/project-plan.md` before choosing the next planning or implementation move.
- Prefer `.codex/skills/project-bootstrap/` for initialization and next-step guidance.
- Prefer `.codex/skills/spec-lifecycle/` for planning, spec drafting, readiness promotion, and planning-state synchronization.
- Prefer `.codex/skills/delivery-cycle/` for implementation from ready planning artifacts, verification, review, and closeout synchronization.

## Preferred Delivery Pattern

1. Bootstrap the session from the binding file, doc index, and project plan.
2. Tighten or create the relevant planning artifact.
3. Implement only from a planning artifact that is ready enough for bounded execution.
4. Verify and review against the artifact and touched system surfaces.
5. Synchronize docs and statuses.
