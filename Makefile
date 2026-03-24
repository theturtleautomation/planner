.PHONY: all build check test clean \
       rust rust-check rust-test rust-build rust-release \
       web web-install web-build web-test web-lint web-dev \
       fmt clippy

# ---------------------------------------------------------------------------
# Combo targets
# ---------------------------------------------------------------------------

all: check build ## Check + build everything (default)

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
