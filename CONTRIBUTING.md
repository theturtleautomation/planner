# Contributing to Planner v2

Thank you for your interest in contributing to Planner v2! This document covers everything you need to get started, from setting up your development environment to submitting well-structured changes.

**Repository:** [https://github.com/theturtleautomation/planner](https://github.com/theturtleautomation/planner)  
**License:** MIT

---

## Table of Contents

- [Development Setup](#development-setup)
- [Code Style](#code-style)
- [Testing Guidelines](#testing-guidelines)
- [Workspace Crate Guidelines](#workspace-crate-guidelines)
- [Adding a New Pipeline Step](#adding-a-new-pipeline-step)
- [Adding a New Artifact Type](#adding-a-new-artifact-type)
- [Adding a New DTU (Deterministic Test Unit)](#adding-a-new-dtu-deterministic-test-unit)
- [Commit Convention](#commit-convention)
- [Known Limitations / Areas for Contribution](#known-limitations--areas-for-contribution)

---

## Development Setup

### Prerequisites

Install the Rust toolchain via [rustup](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Ensure your toolchain is up to date:

```bash
rustup update stable
```

### Clone the Repository

```bash
git clone https://github.com/theturtleautomation/planner.git
cd planner
```

### Build

Build all crates in the workspace:

```bash
cargo build --workspace
```

### Run Tests

Run the full test suite across all workspace crates:

```bash
cargo test --workspace
```

The project currently has **323 tests**. All tests must pass and produce **0 warnings** before a PR can be merged.

### Run with Logging

```bash
RUST_LOG=info cargo run -p planner-core -- "your description"
```

---

## Code Style

All contributions must follow the conventions below. These are not optional — they ensure consistency across the codebase and keep the project maintainable.

### Edition

All crates use **Rust 2021 edition**.

### Derives

Use the full standard derive set on all public types:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyType {
    // ...
}
```

### Test Modules

Every module must include a `#[cfg(test)]` block, even if it starts empty:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // ...
}
```

### Documentation Comments

- Use `///` for doc comments on all public items (structs, enums, functions, traits, fields).
- Use `//!` for module-level documentation at the top of each file.

```rust
//! This module handles artifact serialization.

/// Represents a single planning artifact with a type and payload.
pub struct Artifact {
    // ...
}
```

### Error Handling

- Use [`thiserror`](https://docs.rs/thiserror) for all custom error types.
- Functions that can fail must return `Result<T, E>` — avoid `unwrap()` outside of tests.

```rust
#[derive(Debug, thiserror::Error)]
pub enum PlannerError {
    #[error("artifact not found: {0}")]
    NotFound(String),
}
```

### Async

- Use the **tokio** runtime for all async code.
- Use [`async-trait`](https://docs.rs/async-trait) for async methods in traits:

```rust
#[async_trait::async_trait]
pub trait MyTrait {
    async fn run(&self) -> Result<(), PlannerError>;
}
```

---

## Testing Guidelines

The project maintains a comprehensive test suite. When adding or modifying code, follow these conventions:

| Test Type | Location | Purpose |
|---|---|---|
| Unit tests | Each module's `#[cfg(test)] mod tests` block | Test individual functions and types in isolation |
| Integration tests | `planner-core/tests/integration_e2e.rs` | End-to-end pipeline tests |
| Schema tests | Within `planner-schemas` | Verify serde round-trips for all artifact types |
| TUI tests | Within `planner-tui` | Test `App` state transitions |
| Server tests | Within `planner-server` | Test API endpoints using Axum test infrastructure |

### Test Helpers

- Use `MockFactoryWorker` for factory-related tests.
- Use `InMemoryCxdbEngine` for storage-layer tests.

### Quality Bar

- **0 compiler warnings** — treat all warnings as errors.
- **All 323+ tests pass** — no regressions.
- New code must be covered by tests. PRs that reduce coverage will not be merged.

---

## Workspace Crate Guidelines

The workspace is divided into four crates with clear separation of concerns. Put new code in the right place.

### `planner-schemas`

- **Types only.** No business logic, no I/O, no async.
- This is the right place for new artifact definitions, shared enums, and serialization types.
- All types must derive `Debug`, `Clone`, `Serialize`, `Deserialize`.

### `planner-core`

- **All pipeline logic lives here.**
- New pipeline steps go in `pipeline/steps/`.
- New DTUs go in `dtu/`.
- This crate depends on `planner-schemas` but must not depend on `planner-tui` or `planner-server`.

### `planner-tui`

- **Terminal UI only.** No business logic.
- Interacts with the pipeline exclusively through `planner-core`'s public API.
- Do not add pipeline logic here — wire it into `planner-core` and call it from here.

### `planner-server`

- **HTTP/WebSocket server only.** No business logic.
- Interacts with the pipeline exclusively through `planner-core`'s public API.
- Do not add pipeline logic here — wire it into `planner-core` and call it from here.

---

## Adding a New Pipeline Step

1. Create a new module in `planner-core/src/pipeline/steps/`:

   ```
   planner-core/src/pipeline/steps/my_step.rs
   ```

2. Define the step function signature following existing patterns in the same directory.

3. Register the module in `pipeline/steps/mod.rs`:

   ```rust
   pub mod my_step;
   ```

4. Wire the step into the pipeline orchestrator in `pipeline/mod.rs`.

5. Add tests — both unit tests in the module itself and an integration test in `planner-core/tests/integration_e2e.rs`.

---

## Adding a New Artifact Type

1. Create a new file in `planner-schemas/src/artifacts/`:

   ```
   planner-schemas/src/artifacts/my_artifact.rs
   ```

2. Implement the `ArtifactPayload` trait for your new type.

3. Register the type in the CXDB type registry in `planner-schemas/src/lib.rs`.

4. Re-export your new type from `planner-schemas/src/lib.rs`:

   ```rust
   pub use artifacts::my_artifact::MyArtifact;
   ```

5. Add a serde round-trip test in the artifact module's `#[cfg(test)]` block.

---

## Adding a New DTU (Deterministic Test Unit)

1. Create a new module in `planner-core/src/dtu/`:

   ```
   planner-core/src/dtu/my_dtu.rs
   ```

2. Implement the `Dtu` trait for your new type.

3. Register your DTU in `DtuRegistry`.

4. Add tests covering your DTU's deterministic behavior.

---

## Commit Convention

This project uses [Conventional Commits](https://www.conventionalcommits.org/).

### Format

```
<type>(<module>): <short description>
```

### Types

| Type | When to use |
|---|---|
| `feat` | New feature or capability |
| `fix` | Bug fix |
| `test` | Adding or updating tests |
| `docs` | Documentation changes only |
| `refactor` | Code change that neither fixes a bug nor adds a feature |
| `chore` | Tooling, dependencies, or build changes |

### Examples

```
feat(pipeline): add lean4 verification step
fix(schemas): correct serde tag for FunctionArtifact
test(core): add integration test for e2e planning run
docs(contributing): add DTU registration instructions
```

### Rules

- Each development phase gets **a single commit** containing all related changes.
- Commits must be **atomic and self-contained** — the build and tests should pass at every commit.

---

## Known Limitations / Areas for Contribution

These are active gaps where contributions are especially welcome:

| Area | Status | Notes |
|---|---|---|
| TUI pipeline wiring | Incomplete | TUI currently uses canned responses; needs to call the real `planner-core` pipeline |
| Server WebSocket handler | Stub only | Needs real-time pipeline event streaming implementation |
| LLM pipeline steps | Fallback mode | Steps fall back to simulation when CLI tools are not installed |
| Lean4 verification | Stubs only | Generates "sorry" proofs — real Lean4 proofs are very welcome |
| CI/CD | Missing | No GitHub Actions configuration yet; a working workflow for `cargo test --workspace` would be a great first contribution |
| Web frontend | Minimal | Currently a single static HTML file at `planner-web/dist/index.html`; could be upgraded to a proper React/Vite app |

If you are picking up one of these, please open an issue first to coordinate with maintainers and avoid duplicated effort.

---

*Questions? Open an issue on [GitHub](https://github.com/theturtleautomation/planner/issues) or start a discussion. We're happy to help.*
