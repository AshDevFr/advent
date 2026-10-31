---
title: Introduction
description: "What advent is and how a calendar works."
sidebar:
  order: 1
---

advent is an engine for puzzle advent calendars. Each day opens one door: a teaser, a code puzzle, a cipher, a logic or image puzzle, a trivia question or a meta puzzle. Each solved door gives the player one **word**, and together the words unlock a **certificate**.

## How a calendar is put together

- **The engine** is a single `advent` binary. It seals puzzles, checks a calendar, and builds a static site that any static host can serve. There is no server and no database: player progress stays in the browser.
- **An event repository** holds your calendar: a private git repository with `advent.yaml` (the engine version and public keys), your brand, and one folder per event under `events/`. One repository can hold every year.
- **A day** is a folder of plaintext: `puzzle.md`, `meta.yaml` (answers, word, hints) and any images. You write it on your machine, then `advent seal` turns it into two committed files, `sealed.json` and `source.age`. The plaintext itself is never committed.
- **CI publishes it.** A scheduled workflow builds the site after each midnight and deploys it. It only ever has the **release key**, which opens a day on its date.

## What players see

A landing page with your events, then a calendar: a key strip with one slot per day, a grid of doors (locked until their date, then open, then solved), an always-visible certificate panel, and any notices you post. A door shows its puzzle; the answer is checked in the browser by trying to open the day's word lock with it. Hints appear on their own schedule.

When every word is collected, the player enters a name and the certificate is decrypted in the browser and can be downloaded as a PNG.

## Two keys

| Key | Who holds it | What it opens |
|---|---|---|
| **Release key** | CI only (a secret) | Each day and each hint, on their dates |
| **Editors key** | Authors who want to edit any day | The `source.age` copies, to reopen and edit days |

Sealing needs neither private key, only the public keys in `advent.yaml`. An author who wants to stay surprised can write and seal days without the editors key. See [Keys and authors](/guides/keys-and-authors/).

Next: [Quick start](/start/quick-start/).
