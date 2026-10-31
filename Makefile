.PHONY: help build build-release fmt fmt-check lint test check wasm plaintext ci \
	frontend-install frontend-lint frontend-test frontend-build frontend-ci frontend-dev \
	changelog changelog-unreleased changelog-release release-prepare dist-plan demo acceptance docker-smoke \
	docs docs-dev site showcases screenshots clean setup-hooks

BLUE := \033[0;34m
GREEN := \033[0;32m
YELLOW := \033[0;33m
NC := \033[0m

help: ## Show this help message
	@echo "$(BLUE)advent development commands$(NC)"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(GREEN)%-22s$(NC) %s\n", $$1, $$2}'

# ── Build ────────────────────────────────────────────────────────────────────

build: frontend-build ## Build the debug binary (with the frontend embedded)
	cargo build --workspace

build-release: frontend-build ## Build the release binary (with the frontend embedded)
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

wasm: ## Check that advent-core and advent-server build for wasm32 (no tokio, no filesystem)
	cargo build --target wasm32-unknown-unknown -p advent-core -p advent-server

plaintext: ## Fail if plaintext puzzle content is tracked outside templates/, fixtures/ and showcases/
	./scripts/check-plaintext.sh

check: fmt lint test ## Format, lint and test

# The frontend is built first: the CLI embeds web/dist and its tests use it.
ci: frontend-ci fmt-check lint test wasm plaintext docs ## Everything CI runs

# ── Frontend (web/) ──────────────────────────────────────────────────────────

web/node_modules: web/package.json web/package-lock.json
	cd web && npm ci
	@touch web/node_modules

frontend-install: web/node_modules ## Install frontend dependencies

frontend-lint: web/node_modules ## Lint and format-check the frontend (Biome)
	cd web && npm run lint

frontend-test: web/node_modules ## Run frontend tests (Vitest)
	cd web && npm test

frontend-build: web/node_modules ## Build the frontend into web/dist
	cd web && npm run build

frontend-ci: frontend-lint frontend-test frontend-build ## Frontend lint, tests and build

frontend-dev: web/node_modules ## Start the Vite dev server
	cd web && npm run dev

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

# ── Release (cargo-dist) ─────────────────────────────────────────────────────
# Pushing a tag vX.Y.Z runs .github/workflows/release.yml, which builds the
# frontend and then the advent binaries with it embedded.

docker-smoke: build ## Build the engine and a sample calendar image, run it and check it
	scripts/docker-smoke.sh

dist-plan: ## Show what a release would build
	dist plan

release-prepare: ## Bump versions and the changelog (usage: make release-prepare VERSION=1.0.0)
	@if [ -z "$(VERSION)" ]; then \
		echo "$(YELLOW)Error: VERSION not set. Use: make release-prepare VERSION=1.0.0$(NC)"; \
		exit 1; \
	fi
	sed -i.bak 's/^version = ".*"/version = "$(VERSION)"/' Cargo.toml && rm Cargo.toml.bak
	cd web && npm version $(VERSION) --no-git-tag-version --allow-same-version >/dev/null
	cargo build --quiet
	$(MAKE) changelog-release VERSION=$(VERSION)
	@echo ""
	@echo "$(GREEN)Release v$(VERSION) prepared.$(NC) Review, then:"
	@echo "  git commit -am \"chore(release): v$(VERSION)\" && git tag v$(VERSION) && git push --follow-tags"

# ── Public demo ──────────────────────────────────────────────────────────────

demo: frontend-build ## Build the public demo (every sample day open) into demo-dist/
	cargo build -p advent-cli
	./scripts/build-demo.sh demo-dist

acceptance: frontend-build ## Phase acceptance: release dates, no leaks, identical rebuilds
	cargo build -p advent-cli
	./scripts/acceptance.sh

# ── Documentation and published site ─────────────────────────────────────────

docs/node_modules: docs/package.json docs/package-lock.json
	cd docs && npm ci
	@touch docs/node_modules

docs: docs/node_modules ## Build the documentation into docs/dist
	cd docs && npm run build

docs-dev: docs/node_modules ## Start the documentation dev server
	cd docs && npm run dev

showcases: frontend-build ## Build both showcases into showcases-dist/
	cargo build -p advent-cli
	./scripts/build-showcase.sh northwind showcases-dist/northwind
	./scripts/build-showcase.sh christmas showcases-dist/christmas

screenshots: frontend-build docs/node_modules ## Assemble site/ and capture the docs' screenshots (Playwright Chromium)
	cargo build -p advent-cli
	ADVENT_SCREENSHOTS=1 ./scripts/build-site.sh

site: frontend-build docs/node_modules ## Assemble docs, demo and showcases into site/ (what CI deploys)
	cargo build -p advent-cli
	./scripts/build-site.sh

# ── Misc ─────────────────────────────────────────────────────────────────────

clean: ## Remove build artifacts
	cargo clean
	rm -rf web/dist web/node_modules

setup-hooks: ## Install git pre-commit hooks
	bash scripts/setup-hooks.sh
