---
title: Environment and files
description: "Environment variables and local files advent reads."
sidebar:
  order: 7
---

### `ADVENT_RELEASE_KEY`

The release private key (`AGE-SECRET-KEY-1…`), for `advent build` and `advent verify --release`. Only ever set in CI. Several keys may be given, one per line.

### `ADVENT_EDITORS_KEY`

The editors private key, instead of the `editors.key` file. Several keys may be given, one per line.

### `ADVENT_BASE`

The path the site is served under, as `--base` for `advent build`. Default `/`.

### `ADVENT_ENGINE`

Overrides `engine` from `advent.yaml`.

### `editors.key`

At the repository root, gitignored. The editors private key(s), one per line; lines starting with `#` are comments. Read when `ADVENT_EDITORS_KEY` is not set.

### `ADVENT_*` server settings

`advent serve` reads every [server setting](/reference/server/) from the environment too: `ADVENT_` followed by the setting's path in capitals, with `__` between levels, such as `ADVENT_PUBLIC_URL`, `ADVENT_LIMITS__GUESSES_PER_MINUTE` or `ADVENT_MAIL__SMTP__PASSWORD`. Secrets belong there rather than in a file.

### `ADVENT_SERVE_NOW`

Starts `advent serve`'s clock at another instant (RFC 3339, such as `2026-12-05T08:00:00Z`), from which it moves on in real time. For rehearsing a calendar before it opens and for tests; never in production.
