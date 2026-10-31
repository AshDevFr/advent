---
title: Write and seal a day
description: "Puzzle, answers, word, hints and images."
sidebar:
  order: 1
---

A day lives in `events/<event>/dayNN/`, where `NN` is its number with two digits (`day01` to `day24`).

```text
events/2026/day05/
├── puzzle.md      # what players read
├── meta.yaml      # answers, word, hints (never shown)
├── map.png        # any images the puzzle uses
├── sealed.json    # written by advent seal, committed
└── source.age     # written by advent seal, committed
```

## The puzzle

`puzzle.md` is Markdown: paragraphs, lists, code blocks, quotes and images. Raw HTML is not rendered. Reference images by file name; they are converted to WebP (at most 1600 px on a side) when sealing, and the links follow. Keep a day's images under about 1.5 MB after conversion; `seal` warns above that. SVG is not supported: export it to PNG.

The title shown above the puzzle comes from `meta.yaml`, so do not repeat it as a heading.

## Answers and the word

```yaml
title: A shifted message
type: cipher
answers:
  - snowflake
word: the
hints:
  - text: Move every letter three places back.
    after: 0
  - V becomes S.
```

- **`answers`**: every accepted answer. Players' guesses and your answers are compared after **normalization**: Unicode decomposition, accents removed, lowercase, then only `a-z` and `0-9` kept. `"L'Été 2014!"` and `lete2014` are the same answer. Characters without a Latin decomposition (`ß`, `æ`, Chinese) are dropped, so an answer must keep at least one letter or digit.
- **`word`**: what a solve gives the player, one piece of the final key. It must already be normalized (lowercase `a-z0-9`, no spaces), or sealing fails: the words are joined to build the certificate key, and any difference would lock everyone out.
- **Guessable answers**: `seal` warns about answers that are digits only, shorter than five characters, or members of small sets (yes/no, weekdays, months, numbers). Anyone can try guesses offline against a static site, so ask for two things at once (`2014 lisbon`) when it matters. See [Security model](/guides/security/).

All fields are in the [meta.yaml reference](/reference/meta-yaml/).

## Seal

```sh
advent seal events/2026/day05
```

Sealing validates everything, converts images, encrypts the day with a fresh key, locks the word once per answer, and checks that everything opens again before writing anything. It needs only the public keys in `advent.yaml`, never a private key.

## Preview

```sh
advent preview --all-open
```

serves the calendar on `http://127.0.0.1:4321/` with every sealed day open, so you can see and solve your day as players will. It uses the editors key, never the release key, and only listens on your machine.

## Edit a sealed day

If the plaintext is still on your machine, edit it and seal again. If not, anyone with the editors key can restore it:

```sh
advent unseal-source events/2026/day05      # refuses to overwrite local changes
advent unseal-source events/2026/day05 --force
```

Re-sealing a day after the certificate was sealed with a different word makes the certificate stale; `advent verify` reports it, and `advent keeper seal` fixes it.
