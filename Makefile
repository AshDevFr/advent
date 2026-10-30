.PHONY: help build build-release fmt fmt-check lint test check wasm plaintext ci \
	changelog changelog-unreleased changelog-release clean setup-hooks

BLUE := \033[0;34m
GREEN := \033[0;32m
YELLOW := \033[0;33m
NC := \033[0m

help: ## Show this help message
	@echo "$(BLUE)advent development commands$(NC)"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(GREEN)%-22s$(NC) %s\n", $$1, $$2}'

# ── Build ────────────────────────────────────────────────────────────────────

build: ## Build the debug binary
	cargo build --workspace

build-release: ## Build the release binary
	cargo build --release

# ── Code quality ─────────────────────────────────────────────────────────────

fmt: ## Format Rust code
	cargo fmt --all

fmt-check: ## Check Rust formatting
	cargo fmt --all -- --check

lint: ## Run clippy with warnings as errors
	cargo clippy --workspace --all-targets -- -D warnings

test: ## Run tests
	cargo test --workspace

wasm: ## Check that advent-core builds for wasm32 (no tokio, no filesystem)
	cargo build --target wasm32-unknown-unknown -p advent-core

plaintext: ## Fail if plaintext puzzle content is tracked outside templates/ and fixtures/
	./scripts/check-plaintext.sh

check: fmt lint test ## Format, lint and test

ci: fmt-check lint test wasm plaintext ## Everything CI runs

# ── Changelog (git-cliff) ────────────────────────────────────────────────────

changelog: ## Prepend unreleased changes to CHANGELOG.md
	@touch CHANGELOG.md
	git-cliff --unreleased --prepend CHANGELOG.md

changelog-unreleased: ## Preview unreleased changes
	git-cliff --unreleased

changelog-release: ## Generate the changelog for a version (usage: make changelog-release VERSION=1.0.0)
	@if [ -z "$(VERSION)" ]; then \
		echo "$(YELLOW)Error: VERSION not set. Use: make changelog-release VERSION=1.0.0$(NC)"; \
		exit 1; \
	fi
	@touch CHANGELOG.md
	git-cliff --unreleased --tag v$(VERSION) --prepend CHANGELOG.md

# ── Misc ─────────────────────────────────────────────────────────────────────

clean: ## Remove build artifacts
	cargo clean

setup-hooks: ## Install git pre-commit hooks
	bash scripts/setup-hooks.sh
