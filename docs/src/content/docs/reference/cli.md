---
title: CLI
description: "Every advent command and option."
sidebar:
  order: 8
---

Commands that work on a repository find it from the current folder upwards (the nearest `advent.yaml`).

### `advent init <DIR>`

Create an event repository in an empty or new folder: `advent.yaml` with fresh key pairs, a brand, a first event, example days, the `.gitignore` and the deploy workflow. Prints the release key once and writes the editors key to `editors.key`.

- `--event <ID>`: the first event's id (default: the current year). An event named after a year starts on December 1st of that year.

### `advent seal <DAY_DIR>`

Seal a day folder into `sealed.json` and `source.age`. Needs only the public keys. Nothing is written unless everything sealed opens again.

### `advent unseal-source <DAY_DIR>`

Restore a day's plaintext from `source.age` with the editors key.

- `--force`: overwrite local files that differ from the source (otherwise nothing is written).

### `advent reseal [EVENT]`

Re-encrypt sources and re-wrap day and hint keys to the key lists in `advent.yaml`, for one event or all. Needs the editors key.

- `--all`: also seal every day and the certificate again from their sources, in the current format, with fresh keys (engine upgrades).

### `advent keeper seal <EVENT>`

Seal the event's certificate from `final/final.yaml`, with the words read from the days' sources. Needs the editors key; refuses while a day is unsealed.

### `advent preview`

Build the calendar with the editors key (no release key needed) into a temporary folder outside the repository and serve it on `http://127.0.0.1:4321/`, this machine only. Days not sealed yet are left out with a note. The folder is removed when you stop it with Ctrl-C.

- `--until <YYYY-MM-DD>`: show the calendar as of the end of that date.
- `--all-open`: open every day and every hint regardless of the date.
- `--solved`: pre-fill every word, so the certificate can be unlocked right away with any name.
- `--port <N>`: the port (default 4321).
- `--out <DIR>`: write the preview there and exit instead of serving it. It can contain unreleased days: keep it out of git.

### `advent schedule [EVENT]`

Print when each day opens and when its hints appear.

### `advent build`

Write the static site for the current moment: released days and due hints in plain form, word locks, the sealed certificate, the brand and theme stylesheets, and the frontend. Needs `ADVENT_RELEASE_KEY`.

- `--until <YYYY-MM-DD>`: build as of the end of that date in each event's timezone.
- `--out <DIR>`: output folder (default `dist`). An existing folder is only replaced if a previous build wrote it.
- `--base <PATH>`: the path the site is served under (default `/`, or `ADVENT_BASE`).

### `advent verify [EVENT]`

Check that a repository is fit to deploy: every day sealed and none outside the event, files readable by this engine, the certificate sealed, no plaintext tracked by git. With an editors key it also compares every day with its source and checks the certificate opens with the current words.

- `--release`: also open every day and hint key with `ADVENT_RELEASE_KEY`. Its output names broken days and never content.
- `--released`: only require the days released so far, and the certificate once the last day is out (the CI deploy gate).
