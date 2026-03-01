# Contributing to Planner v2

Thank you for your interest in contributing to Planner v2! This document covers everything you need to get started, from setting up your development environment to submitting well-structured changes.

**Repository:** [https://github.com/theturtleautomation/planner](https://github.com/theturtleautomation/planner)  
**License:** MIT

---

## Table of Contents

- [Development Setup](#development-setup)
- [Frontend Development](#frontend-development)
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

For frontend work, Node.js 18 or later is also required:

```bash
node --version   # must be 18+
```

### Clone the Repository

```bash
git clone https://github.com/theturtleautomation/planner.git
cd planner
```

### Build

Build all Rust crates in the workspace:

```bash
cargo build --workspace
```

### Run Tests

Run the full Rust test suite across all workspace crates:

```bash
cargo test --workspace
```

Run the frontend test suite:

```bash
cd planner-web && npm test -- --run
```

The project currently has **474 tests** (377 Rust + 97 frontend). All tests must pass before a PR can be merged.

### Run with Logging

```bash
RUST_LOG=info cargo run -p planner-core -- "your description"
```

---

## Frontend Development

The `planner-web/` directory is a full React + TypeScript + Vite single-page application. It communicates with `planner-server` via REST and WebSocket.

### Running the Dev Server

```bash
cd planner-web
npm install
npm run dev
```

This starts the Vite dev server at `http://localhost:5173`. Hot module replacement is enabled.

The dev server proxies API requests to `http://localhost:3100` by default. Start `planner-server` separately to get a working backend:

```bash
# In another terminal
cargo run --bin planner-server
```

### Auth0 in Development

Auth0 is **optional for local development**. When the `VITE_AUTH0_DOMAIN` environment variable is not set, the frontend skips authentication and the server injects a synthetic `dev|local` user. No Auth0 account is required to work on the frontend.

For testing Auth0-gated flows, see [AUTH0_SETUP.md](./AUTH0_SETUP.md).

### Running Frontend Tests

```bash
cd planner-web

# Watch mode (re-runs on file changes)
npm test

# Single run (CI)
npm run test -- --run

# With coverage
npm run test -- --run --coverage
```

Tests use [Vitest](https://vitest.dev/) and [React Testing Library](https://testing-library.com/docs/react-testing-library/intro/). The test runner is configured in `vite.config.ts`.

### Mocking Conventions

Auth0 is mocked globally in `src/test/setup.ts`. The mock provides a default unauthenticated state and exposes helpers to simulate logged-in users:

```ts
// In any test file — Auth0 is already mocked via setup.ts
import { mockAuth0 } from '../test/setup'

// Simulate authenticated user
mockAuth0({ isAuthenticated: true, user: { sub: 'user|123' } })
```

WebSocket connections are mocked using the `vi.stubGlobal('WebSocket', ...)` pattern. See existing tests in `src/components/__tests__/ChatPanel.test.tsx` for examples.

When adding a new component that uses `useAuthenticatedFetch` or any Auth0 hook, wrap your test's rendered component in the `Auth0ProviderWithNavigate` mock rather than the real provider.

### Building for Production

```bash
cd planner-web
npm run build
```

Output is written to `planner-web/dist/`. Point `planner-server --static-dir ./planner-web/dist` at this directory to serve the built app.

---

## Code Style

All contributions must follow the conventions below. These are not optional — they ensure consistency across the codebase and keep the project maintainable.

### Edition

All Rust crates use **Rust 2021 edition**.

### Derives

Use the full standard derive set on all public types:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyType {
    // ...
}
```

### Test Modules

Every Rust module must include a `#[cfg(test)]` block, even if it starts empty:

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

The project maintains a comprehensive test suite across Rust and TypeScript. When adding or modifying code, follow these conventions:

### Rust

| Test Type | Location | Purpose |
|---|---|---|
| Unit tests | Each module's `#[cfg(test)] mod tests` block | Test individual functions and types in isolation |
| Integration tests | `planner-core/tests/integration_e2e.rs` | End-to-end pipeline tests |
| Schema tests | Within `planner-schemas` | Verify serde round-trips for all artifact types |
| TUI tests | Within `planner-tui` | Test `App` state transitions |
| Server tests | Within `planner-server` | Test API endpoints using Axum test infrastructure |

### Frontend

| Test Type | Location | Purpose |
|---|---|---|
| Component tests | `src/components/__tests__/` | Render and interaction tests with React Testing Library |
| Page tests | `src/pages/__tests__/` | Route-level rendering and auth flow tests |
| API client tests | `src/api/__tests__/` | Typed fetch wrapper and error handling tests |

### Test Helpers

- Use `MockFactoryWorker` for factory-related Rust tests.
- Use `InMemoryCxdbEngine` for storage-layer tests.
- Use the Auth0 mock in `src/test/setup.ts` for all frontend tests that touch auth.

### Quality Bar

- **0 compiler warnings** — treat all warnings as errors (`RUSTFLAGS="-D warnings"`).
- **All 474 tests pass** — 377 Rust + 97 frontend. No regressions.
- New code must be covered by tests. PRs that reduce coverage will not be merged.

---

## Workspace Crate Guidelines

The workspace is divided into four Rust crates and one Node.js package, each with clear separation of concerns. Put new code in the right place.

### `planner-schemas`

- **Types only.** No business logic, no I/O, no async.
- The right place for new artifact definitions, shared enums, and serialization types.
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
- Security-sensitive code (auth, rate limiting, RBAC) lives here and must not leak into other crates.
- Do not add pipeline logic here — wire it into `planner-core` and call it from here.

### `planner-web`

- **React SPA only.** No server-side logic.
- All API calls go through `src/api/client.ts` — do not use raw `fetch` directly in components.
- Authentication state is managed via Auth0 hooks — do not store tokens manually.
- All new components must have corresponding tests in a sibling `__tests__/` directory.

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

4. Wire the DTU clone into the validation stage in `pipeline/steps/validate.rs`.

5. Add tests covering your DTU's deterministic behavior.

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
feat(web): add session listing dashboard
docs(contributing): add frontend development section
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
| LLM pipeline steps | Fallback mode | Steps fall back to simulation when CLI tools are not installed |
| Lean4 verification | Stubs only | Generates "sorry" proofs — real Lean4 proofs are very welcome |
| CI/CD | Missing | No GitHub Actions configuration yet; a working workflow for `cargo test --workspace && cd planner-web && npm test -- --run` would be a great first contribution |
| Docker | Planned | A Dockerfile that builds the Rust binaries and React app in a multi-stage build is planned but not yet written |
| Rate limit configuration | Hardcoded | Rate limit thresholds are compile-time constants; runtime configuration via env vars is desirable |

If you are picking up one of these, please open an issue first to coordinate with maintainers and avoid duplicated effort.

---

*Questions? Open an issue on [GitHub](https://github.com/theturtleautomation/planner/issues) or start a discussion. We're happy to help.*
