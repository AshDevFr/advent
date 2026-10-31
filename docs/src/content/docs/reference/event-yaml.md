---
title: event.yaml
description: "One edition: dates, schedule, hints, notices and story."
sidebar:
  order: 4
---

`events/<event>/event.yaml`. The folder name is the event's id and its address (`/<event>/`).

```yaml
title: "Advent 2026"
timezone: Europe/Paris
start: 2026-12-01
days: 24
hint_after: [1, 2]
story:
  title: "The Longest Night Hunt"
  intro: "One door a day."
```

### `title`

**Type** string · **Required**

The edition's name, shown on the landing page and the certificate.

### `timezone`

**Type** IANA time zone (`Europe/Paris`, `UTC`, `America/New_York`) · **Required**

Doors, hints and dated notices open at local midnight in this zone.

### `start`

**Type** date (`YYYY-MM-DD`) · **Required**

Day 1's date.

### `days`

**Type** integer ≥ 1 · **Default** `24`

How many doors. Folders `day01` to `dayNN` must exist and be sealed by the time each opens.

### `unlocked`

**Type** integer, 0 to `days` · **Default** `0`

Days 1 to `unlocked` are open regardless of the date, with all their hints. The public demo opens every day this way.

### `every`

**Type** integer ≥ 1 · **Default** `1`

A door every `every` days (counting only workdays with `weekdays`). Cannot be combined with `dates`.

### `weekdays`

**Type** boolean · **Default** `false`

Doors only Monday to Friday; `start` must be a weekday. Cannot be combined with `dates`.

### `dates`

**Type** list of dates · **Optional**

Each day's date explicitly: one per day, increasing, starting with `start`. Cannot be combined with `every` or `weekdays`.

### `hint_after`

**Type** list of two integers ≥ 0, not decreasing · **Default** `[1, 2]`

When hints appear, in days after their door opens (0 means with the door). A hint in [meta.yaml](/reference/meta-yaml/) can set its own `after`.

### `notices`

**Type** list · **Default** none

Announcements at the top of the calendar. Each notice has:

### `text`

**Type** string · **Required** · Under `notices`

The announcement. Editing it shows it again to players who dismissed it.

### `day`

**Type** day number · **Optional** · Under `notices`

Show the notice once this day opens. Cannot be combined with `date`.

### `date`

**Type** date · **Optional** · Under `notices`

Show the notice from local midnight of this date. Cannot be combined with `day`. Without `day` or `date`, the notice shows from the start.

### `story`

**Type** object · **Optional**

Text for the calendar page:

### `title`

**Type** string · **Optional** · Under `story`

A display headline (for example "The Longest Night Hunt"). With it, the event `title` moves to the small eyebrow above.

### `intro`

**Type** string · **Optional** · Under `story`

The paragraph under the headline, also shown on the landing page.

### `attempts`

**Type** object · **Optional** · **Default** no limit

Company edition (server mode) only; the static site ignores it. Limits how many guesses each player can make on a day, in a window that refills, so a player who runs out only has to wait:

```yaml
attempts:
  max: 10
  per: 1h
```

### `max`

**Type** integer ≥ 1 · **Required** · Under `attempts`

Guesses allowed per player and day in one window.

### `per`

**Type** duration (`30m`, `1h`, `1d`) · **Required** · Under `attempts`

Length of the window, counted from the first guess in it. Once it ends, the player can guess again.
