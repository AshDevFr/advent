---
title: Quick start
description: "From nothing to a deployed calendar."
sidebar:
  order: 2
---

## 1. Install advent

Download the archive for your platform from the [releases](https://github.com/AshDevFr/advent/releases) and put `advent` on your `PATH`, or use the installer script attached to each release.

```sh
advent --version
```

## 2. Create an event repository

```sh
advent init my-calendar --event 2026
```

This writes `advent.yaml` with fresh key pairs, a default brand, `events/2026/event.yaml`, seven example days in `examples/`, a `.gitignore` that keeps plaintext out of git, and the deploy workflow. It prints the **release key** once: store it now as a CI secret (step 6), because it is not saved anywhere. The **editors key** is written to `editors.key`, which is gitignored; share it through a password manager.

## 3. Write a day

```sh
cd my-calendar
cp -R examples/day01 events/2026/day01
```

Edit `events/2026/day01/puzzle.md` and `meta.yaml`. See [Write and seal a day](/guides/write-a-day/).

## 4. Seal it

```sh
advent seal events/2026/day01
```

This writes `sealed.json` and `source.age` next to your plaintext. Only those two files are committed; `git status` will not even show the rest.

## 5. Check it

```sh
advent schedule      # when each door and hint opens
advent verify        # every day sealed, nothing plaintext tracked, sources match
```

## 6. Publish

Push the repository to GitHub, then in its settings add:

- secrets `ADVENT_RELEASE_KEY`, `CLOUDFLARE_API_TOKEN` (permission *Cloudflare Pages: Edit*) and `CLOUDFLARE_ACCOUNT_ID`;
- the variable `CLOUDFLARE_PAGES_PROJECT` with the project name you want.

The workflow builds and deploys on every push to `main`, after each midnight in December, and on demand. It creates the Pages project the first time. See [Deploy](/guides/deploy/).
