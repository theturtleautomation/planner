.PHONY: all build check test clean \
       rust rust-check rust-test rust-build rust-release \
       web web-install web-build web-test web-lint web-dev \
       fmt clippy \
       builder-auth-status builder-print-config builder-validate-config \
       builder-launch builder-create-project builder-connect-repo \
       builder-connect-repo-dryrun builder-index-repo builder-code \
       builder-list-projects builder-get-project builder-update-project \
       builder-verify-sync builder-diagnose-project-visibility \
       builder-dsi-status \
       builder-server-print-config builder-server-validate-config \
       builder-server-launch builder-server-create-project builder-server-update-project \
       builder-server-verify-sync \
       builder-figma-generate builder-figma-publish builder-figma-migrate \
       builder-sync-project

# ---------------------------------------------------------------------------
# Combo targets
# ---------------------------------------------------------------------------

all: check build ## Check + build everything (default)

ARGS ?=

build: rust-build web-build ## Build Rust (debug) + Web (prod)

check: rust-check web-build ## Type-check Rust + Web

test: rust-test web-test ## Run all tests

clean: ## Remove build artifacts
	cargo clean
	rm -rf planner-solid/dist planner-solid/node_modules/.tmp

# ---------------------------------------------------------------------------
# Rust
# ---------------------------------------------------------------------------

rust: rust-build ## Alias for rust-build

rust-check: ## cargo check (fast type-check)
	cargo check --workspace

rust-build: ## cargo build (debug)
	cargo build --workspace

rust-release: ## cargo build --release
	cargo build --workspace --release

rust-test: ## cargo test
	cargo test --workspace

fmt: ## cargo fmt --check
	cargo fmt --all -- --check

clippy: ## cargo clippy
	cargo clippy --workspace -- -D warnings

# ---------------------------------------------------------------------------
# Web (planner-solid)
# ---------------------------------------------------------------------------

web: web-build ## Alias for web-build

web-install: ## npm install (planner-solid)
	npm install --prefix planner-solid

web-build: node_modules ## tsc + vite build
	npm run build --prefix planner-solid

web-test: node_modules ## vitest run
	npm run test --prefix planner-solid

web-lint: node_modules ## eslint
	npm run lint --prefix planner-solid

web-dev: node_modules ## vite dev server
	npm run dev --prefix planner-solid

# ---------------------------------------------------------------------------
# Builder
# ---------------------------------------------------------------------------

builder-auth-status: ## Show Builder CLI auth status
	./scripts/builder-auth-status.sh $(ARGS)

builder-print-config: ## Print the canonical Builder UI-review config contract
	./scripts/builder-print-config.sh $(ARGS)

builder-validate-config: ## Validate the canonical Builder UI-review config contract
	./scripts/builder-validate-config.sh $(ARGS)

builder-launch: ## Launch the canonical Builder UI-review project against frontend mock mode
	./scripts/builder-launch.sh $(ARGS)

builder-create-project: ## Create a fresh Builder UI-review Fusion project and record local history
	./scripts/builder-create-project.sh $(ARGS)

builder-connect-repo: ## Connect this repo to Builder Fusion
	./scripts/builder-connect-repo.sh $(ARGS)

builder-connect-repo-dryrun: ## Preview Builder Fusion repo connection config
	./scripts/builder-connect-repo-dryrun.sh $(ARGS)

builder-index-repo: ## Index this repo in Builder
	./scripts/builder-index-repo.sh $(ARGS)

builder-code: ## Run Builder code generation CLI in this repo
	./scripts/builder-code.sh $(ARGS)

builder-list-projects: ## List Fusion projects visible to the current Builder auth context
	./scripts/builder-list-projects.sh $(ARGS)

builder-get-project: ## Inspect the saved Fusion project or an explicit project ID
	./scripts/builder-get-project.sh $(ARGS)

builder-update-project: ## Sync the saved Fusion project's runtime settings to the canonical UI-review config
	./scripts/builder-update-project.sh $(ARGS)

builder-verify-sync: ## Verify the canonical Builder UI-review config against saved and visible remote Fusion state
	./scripts/builder-verify-sync.sh $(ARGS)

builder-diagnose-project-visibility: ## Diagnose why the saved Fusion project is or is not visible in the current auth context
	./scripts/builder-diagnose-project-visibility.sh $(ARGS)

builder-dsi-status: ## Verify repo-local Builder DSI plugin wiring and prerequisites
	./scripts/builder-dsi-status.sh $(ARGS)

builder-server-print-config: ## Print the alternate server-backed Builder config contract
	BUILDER_PROJECT_CONFIG_PATH=./builder.server.config.json ./scripts/builder-print-config.sh $(ARGS)

builder-server-validate-config: ## Validate the alternate server-backed Builder config contract
	BUILDER_PROJECT_CONFIG_PATH=./builder.server.config.json ./scripts/builder-validate-config.sh $(ARGS)

builder-server-launch: ## Launch Builder Fusion against the server-backed integration runtime
	BUILDER_PROJECT_CONFIG_PATH=./builder.server.config.json ./scripts/builder-launch.sh $(ARGS)

builder-server-create-project: ## Create a fresh server-backed Builder Fusion project and record local history
	BUILDER_PROJECT_CONFIG_PATH=./builder.server.config.json ./scripts/builder-create-project.sh $(ARGS)

builder-server-update-project: ## Sync the saved Fusion project to the server-backed integration config explicitly
	BUILDER_PROJECT_CONFIG_PATH=./builder.server.config.json ./scripts/builder-update-project.sh $(ARGS)

builder-server-verify-sync: ## Verify the server-backed Builder config against saved and visible remote Fusion state
	BUILDER_PROJECT_CONFIG_PATH=./builder.server.config.json ./scripts/builder-verify-sync.sh $(ARGS)

builder-figma-generate: ## Run Builder Figma generate flow
	./scripts/builder-figma-generate.sh $(ARGS)

builder-figma-publish: ## Publish Builder Figma mappings
	./scripts/builder-figma-publish.sh $(ARGS)

builder-figma-migrate: ## Migrate Builder Figma mappings into this repo
	./scripts/builder-figma-migrate.sh $(ARGS)

builder-sync-project: ## Sync this repo into Builder CMS via fallback skill
	./scripts/builder-sync-project.sh $(ARGS)

# Auto-install if node_modules missing
node_modules: planner-solid/package.json
	npm install --prefix planner-solid
	@touch $@

# ---------------------------------------------------------------------------
# Help
# ---------------------------------------------------------------------------

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-16s\033[0m %s\n", $$1, $$2}'
