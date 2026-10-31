---
title: meta.yaml
description: "A day's answers, word and hints. Never published."
sidebar:
  order: 5
---

`events/<event>/dayNN/meta.yaml`. Plaintext, never committed: only its sealed form is.

```yaml
title: Count the candles
type: code
answers:
  - "55"
  - fifty five
word: returns
hints:
  - text: The loop adds every night from 1 to 10.
    after: 0
  - 1 + 2 + ... + 10.
```

### `title`

**Type** string · **Required**

Shown above the puzzle.

### `type`

**Type** string · **Required**

A label shown on the door and in the puzzle header. Any text works; the samples use `teaser`, `code`, `company`, `cipher`, `logic`, `image` and `meta`. The engine treats all types alike.

### `answers`

**Type** list of strings · **Required**, at least one

Every accepted answer, compared after normalization (accents removed, lowercase, only `a-z0-9`). Each must keep at least one letter or digit. See [Write and seal a day](/guides/write-a-day/).

### `word`

**Type** string, already normalized (`a-z0-9`) · **Required**

What solving the day gives the player; the words in day order form the certificate key.

### `hints`

**Type** list of one or two hints · **Required**

Each hint is either plain text, timed by the event's `hint_after`, or an object:

### `text`

**Type** string · **Required** · Under `hints`

The hint.

### `after`

**Type** integer ≥ 0 · **Optional** · Under `hints`

Days after the door opens when this hint appears (0 means with the door). Hints are revealed in order, so a later hint cannot be due sooner.
