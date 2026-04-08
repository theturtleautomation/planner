# Project Skill Config

## Project

- Name: Planner
- Workflow model: spec-first planning with phased implementation docs

## Core Files

- Doc index: `.omx/ledger/session-start-and-doc-index.md`
- Backlog or tracker: `.omx/ledger/planner-ledger.json`

## Root Instructions

- `AGENTS.md`
- `CLAUDE.md`

## Always-Load Docs

- `README.md`
- `.omx/ledger/session-start-and-doc-index.md`
- `.omx/ledger/current-status.md`
- `.omx/ledger/project-plan.md`
- `.omx/ledger/README.md`
- `docs/import-existing-project-plan.md`
- `docs/session-workflow-webui-plan.md`

## Planning Containers

- Primary planning docs directory: `docs/`
- Planning model: OMX ledger plus phase and feature implementation docs under `docs/`
- Parent-child tracking model: canonical ledger plus task-relevant phase or feature plan documents

## Status Model

- `active`
- `draft`
- `ready for implementation`
- `in progress`
- `implemented`
- `complete`
- `deferred`

Interpret existing doc statuses semantically rather than requiring one exact label.

## Task-Relevant Doc Families

- UI and workflow redesign: `docs/project-first-ui-research-sessions.md`, `docs/phase-0*.md`, `docs/phase-1*.md`, `docs/phase-2*.md`, `docs/phase-3*.md`, `docs/phase-4*.md`, `docs/phase-5*.md`, `docs/phase-6*.md`, `docs/phase-7*.md`
- Import and blueprint: `docs/import-existing-project-plan.md`, `docs/blueprint-project-root-codegraph-integration.md`, `docs/knowledge-library-project-scope-plan.md`
- Operations and observability: `docs/admin-observability-plan.md`

## Project Policy Checks

Apply these during planning, implementation, and review:

- preserve bounded execution against the active phase or feature plan
- keep planning and implementation aligned to the actual artifact state
- update the canonical ledger when durable artifacts or workflow states change
- keep `.agents/plugins/marketplace.json` aligned with local plugin manifests in `plugins/*/.codex-plugin/plugin.json`
- do not silently broaden scope across unrelated phases
- do not claim verification if the relevant tests or checks were not run

## Review Red Flags

- the active planning thread in `.omx/ledger/current-status.md` no longer matches the actual next move
- `.agents/plugins/marketplace.json` drifts from local plugin manifest truth in `plugins/*/.codex-plugin/plugin.json`
- implementation docs claim completion without corresponding verification evidence
- new durable planning docs exist but are not reflected in the OMX ledger/bootstrap surfaces
- work drifts across phases without an explicit planning update
