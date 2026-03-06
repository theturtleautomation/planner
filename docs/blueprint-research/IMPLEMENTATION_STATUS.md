# Blueprint Implementation — Phase A Status Tracker

**Started:** March 5, 2026
**Last Updated:** March 5, 2026

## Research Documents (committed to repo)
- `docs/blueprint-research/BLUEPRINT_DEEP_DIVE.md` — Decision audit, spec vs. code gap analysis
- `docs/blueprint-research/BLUEPRINT_MOCKUP_VS_IMPLEMENTATION.md` — Mockup v2 vs. current code, feature roadmap
- `docs/blueprint-research/architecture_tools_research.md` — Industry tool research (Backstage, Structurizr, etc.)

## Implementation Phases

### Phase A: Type Alignment & Cleanup [IN PROGRESS]
- [ ] A.1 — Sync TypeScript types with Rust structs (CRITICAL)
- [ ] A.2 — Fix Rust doc comment shapes (Decision=rounded rect, Constraint=diamond)
- [ ] A.3 — Add edge DELETE endpoint
- [ ] A.4 — Add history GET endpoint
- [ ] A.5 — Add Edit + Propose Change buttons to drawer
- [ ] A.6 — Add "Create Node" button + form modal
- [ ] A.7 — Add node deletion UI with confirmation dialog

### Phase B: Event Sourcing — PENDING
### Phase C: Detail Drawer Editing — PENDING
### Phase C.5: Knowledge & Library Pages — PENDING
### Phase D: Reconvergence Engine — PENDING
### Phase E: Graph UX Polish — PENDING
### Phase F: Lifecycle & History — PENDING
### Phase G: Automated Discovery — PENDING
### Phase H: TUI Blueprint Table — PENDING

## Key Decisions (from GitHub conversation)
1. ✅ NodeId: human-readable slug + UUID8
2. ⚠️ Event sourced: snapshot-only, needs Phase B
3. ⚠️ Reconvergence autonomy: types defined, no execution, needs Phase D
4. ⚠️ One per project: global singleton, OK for now
5. ✅ WebUI primary, TUI table-only

## Critical Issue
TypeScript types in `types/blueprint.ts` are mismatched with Rust structs — will cause runtime deserialization failures. Must fix first.
