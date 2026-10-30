# advent

Engine for puzzle advent calendars. One `advent` binary: authors seal days into an event repo
(a separate, private repository), and a scheduled CI job in that repo builds and deploys a static site.
The PRD and phase specs in `.specs/docs/` are the source of truth.

## Layout

- `crates/core` (`advent-core`): normalization, sealing primitives, sealed formats, content model,
  release schedule. Must build for `wasm32-unknown-unknown`: no tokio, no filesystem, bytes in and out.
- `crates/cli` (`advent-cli`, binary `advent`): commands, figment config, embedded template and frontend.
- `web/`: React + Vite + TypeScript frontend (phase 2).
- `templates/event/`: the event repo template `advent init` writes, with plaintext sample days in `examples/`.
- `fixtures/`: test-only inputs.
- `test-vectors.json`: shared by Rust and TypeScript tests; generated with WebCrypto, not by our own code.

## Rules

- The crypto spec (PRD section 7) is exact. Do not change algorithms, iteration counts, salt/IV sizes or normalization.
- Plaintext puzzle content (`puzzle.md`, `meta.yaml`, day images, `final.yaml`) is never committed outside
  `templates/` and `fixtures/`. `.gitignore` blocks it and `scripts/check-plaintext.sh` enforces it in CI.

## Commands

- `make ci`: fmt check, clippy, tests, wasm32 build of core, plaintext guard. CI runs exactly this.
- `make check`: format, lint and test locally.
- `make setup-hooks`: install pre-commit hooks.

## Task Workflow

- **Spec Repo**: `.specs/` (remote `gitea@git.4sh.io:ash/advent-specs.git`)
- **PRD**: `.specs/docs/PRD.md`
- **Phase Specs**: `.specs/docs/specs/phase-N_<title>.md`
- **Task Directory**: `.specs/docs/tasks/phase-N/`
- **Task File Pattern**: `N.NN-descriptive-name.md`
- **Spec Model**: `flow-back`
