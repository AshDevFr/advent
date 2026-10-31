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
