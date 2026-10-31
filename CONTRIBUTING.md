# Contributing

Thanks for helping. A few things keep the engine safe to run every December.

## Setup

- Rust stable (installed from `rust-toolchain.toml`, including the
  `wasm32-unknown-unknown` target)
- Node 24 (`mise install` picks it up from `mise.toml`)
- `make setup-hooks` installs the pre-commit hooks

## The loop

```sh
make ci          # frontend lint, tests and build; rustfmt, clippy, all Rust tests;
                 # wasm32 build of advent-core; plaintext guard. CI runs exactly this.
make demo        # build the public demo (all sample days open) into demo-dist/
make acceptance  # build the sample event for a date and check nothing leaks
```

Write the failing test first where the behaviour can be captured.

## Rules that are not negotiable

- **The crypto spec is exact.** Algorithms, iteration counts, salt and IV
  sizes and answer normalization must match the browser byte for byte.
  `test-vectors.json` is generated with WebCrypto (`node scripts/gen-test-vectors.mjs`)
  and both sides are tested against it; never regenerate it from Rust.
- **`advent-core` builds for `wasm32-unknown-unknown`.** No tokio, no
  filesystem, no system clock in core.
- **Plaintext puzzles are never committed** outside `templates/`, `fixtures/`
  and `showcases/`, which hold public examples. `.gitignore` blocks the rest
  and `scripts/check-plaintext.sh` fails CI if one slips through.
- **Comments explain why**, with the real constraint, not a pointer to an
  issue or a plan.

## Commits

Conventional commits (`feat:`, `fix:`, `docs:`, ...): the changelog is
generated from them with git-cliff.
