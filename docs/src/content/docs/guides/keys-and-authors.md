---
title: Keys and authors
description: "The editors and release keys, sharing them, and rotating them."
sidebar:
  order: 2
---

`advent init` creates two age key pairs. Their **public** halves are listed in `advent.yaml`; nobody can open anything with them.

## The editors key

Opens the `source.age` copies, so the holder can reopen and edit any day and verify a day against its source.

- `advent init` writes it to `editors.key` (gitignored). Share it through a password manager or Vault; `ADVENT_EDITORS_KEY` works instead of the file.
- Keep a copy somewhere safe for the life of the repository: without it, sealed days still work but can never be edited, and the engine cannot be upgraded across a format change.
- An author who does not take it can still write and seal days, and stays surprised by everyone else's.

## The release key

Opens each day and each hint on their dates. It belongs in CI secrets only (`ADVENT_RELEASE_KEY`), never on a laptop. `advent init` prints it once and does not save it.

Anyone who can push to the event repository can read it through a workflow, so keep push access to people you trust with it, or protect `main`.

## Several keys and rotation

Both entries in `advent.yaml` are lists, and anything is sealed to every key listed. To add, replace or remove a key:

1. Edit the list in `advent.yaml` (add the new public key, keep the old one for now).
2. Run `advent reseal` with an editors key: every source is re-encrypted to the current editors and every day and hint key re-wrapped to the current release keys. It needs only the editors key, since each source copy holds its day's keys.
3. Switch the private key in CI or Vault, remove the old public key, and reseal again.

A key file (or `ADVENT_EDITORS_KEY`) may hold several keys, one per line, with `#` comments.
