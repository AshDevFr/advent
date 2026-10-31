---
title: Schedule, hints and notices
description: "When doors open, when hints appear, and announcements."
sidebar:
  order: 5
---

Everything is released at local midnight in the event's `timezone`. Check the result any time with:

```sh
advent schedule
```

```text
2026: Advent 2026 (12 days, Europe/Paris; hints [1, 2] days after a door opens)
  day  1  Tue 2026-12-01  00:00 CET  hints Dec 2, Dec 3
  day  2  Thu 2026-12-03  00:00 CET  hints Dec 4, Dec 5 (default)
```

## When doors open

By default days are consecutive from `start`. To spread them out, use **one** of:

```yaml
every: 2          # a door every 2 days
weekdays: true    # Monday to Friday only (combine with every: 2 for every other workday)
```

or list each date:

```yaml
dates: [2026-12-01, 2026-12-03, 2026-12-08, 2026-12-24]
```

`dates` needs one date per day, in increasing order, starting with `start`, and cannot be combined with `every` or `weekdays`. With `weekdays`, `start` must be a weekday.

`unlocked: N` opens days 1 to N regardless of the date, with all their hints (the public demo uses it).

## When hints appear

A day has one or two hints. Each appears at local midnight a number of days after its door opens (0 means with the door). The event sets the default:

```yaml
hint_after: [1, 2]
```

and a hint in `meta.yaml` can set its own:

```yaml
hints:
  - text: Look at the second line again.
    after: 0
  - Think about capitals.       # uses the event default
```

Hints are revealed in order, so a later hint cannot be due sooner; `seal` refuses that.

## Notices

Short announcements shown at the top of the calendar:

```yaml
notices:
  - text: "Hints arrive at midnight."
  - text: "Day 12's puzzle had a typo; it is fixed now."
    day: 12
  - text: "Last week! Collect your words."
    date: 2026-12-18
```

A notice with `day` appears once that day opens, one with `date` from that date, others from the start. Players can dismiss each one; their browser remembers it. Editing a notice's text shows it again.
