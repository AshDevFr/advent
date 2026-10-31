---
title: Deploy
description: "The CI workflow, Cloudflare Pages, sub-paths and other hosts."
sidebar:
  order: 6
---

## The included workflow

`advent init` writes `.github/workflows/build.yml`. It runs after each midnight in December and on the first days of January (for the last hints), on every push to `main`, and on demand. Each run:

1. downloads the `advent` release pinned by `engine` in `advent.yaml`;
2. runs `advent verify --release --released`: nothing plaintext is tracked, and every day released so far is sealed and opens (days still being written do not block a deploy, so the coming-soon page can go out in November);
3. runs `advent build`, which publishes exactly what is out at that moment;
4. creates the Cloudflare Pages project if needed and deploys `dist/`.

Rerunning it is harmless: the output depends only on the date.

### Settings

| Name | Kind | Purpose |
|---|---|---|
| `ADVENT_RELEASE_KEY` | secret | the release key printed by `advent init` |
| `CLOUDFLARE_API_TOKEN` | secret | token with *Cloudflare Pages: Edit* |
| `CLOUDFLARE_ACCOUNT_ID` | secret | your Cloudflare account id |
| `CLOUDFLARE_PAGES_PROJECT` | variable | Pages project name |
| `ADVENT_BASE` | variable | optional sub-path, such as `/calendar/` |
| `ADVENT_ENGINE_REPO` | variable | optional, to download the engine from a fork |

The cron times are in UTC. If your event is elsewhere, shift them so a run lands just after your local midnight in both summer and winter time; extra runs do no harm.

## Serving under a sub-path

A calendar works at a domain root or under any path, such as a GitLab or GitHub Pages project site:

```sh
advent build --base /calendar/        # or ADVENT_BASE=/calendar/
```

Every page gets `<base href="/calendar/">` and the site uses relative URLs only.

## Other hosts

The output of `advent build` is plain static files: upload `dist/` anywhere. For GitLab Pages, run the same steps in `.gitlab-ci.yml` and publish `dist/` as `public/`. Serve it over **https**: browsers only allow the answer checking (WebCrypto) on secure pages.
