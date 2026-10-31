---
title: Company edition
description: "Host a calendar for invited players with advent serve: sign-in links, progress on the server, attempt limits and an admin section."
sidebar:
  order: 9
---

The static site suits a public game: anyone can play, progress stays in each browser, and nothing needs a server. A company calendar usually wants more: only colleagues can play, organisers see who plays and who finished, and progress follows a player from laptop to phone. `advent serve` does that from the same event repository and the same sealed days.

| | Static site | Company edition |
|---|---|---|
| Who plays | Anyone with the address | Invited email addresses only |
| Answers checked | In the browser | On the server |
| Progress | In each browser | On the server, on every device |
| Attempt limit | None | Optional, per event |
| Organisers see | Nothing | Players, progress, completion per day, finishers |
| Runs on | Cloudflare Pages (or any static host) | A Docker container you host |

Writing and sealing days, brands, themes, schedules, hints and the certificate all work exactly as for the static site. Only deployment differs.

## How it works

- The server holds the same release key as the static build and publishes released days and hints on the same schedule, from its own clock.
- Players never receive word locks. A guess goes to the server, which counts it, checks it against the sealed lock, records the solve and returns the word. The certificate still opens in the browser, with the words.
- Each invited player has one personal link, sent by email. Opening it shows **Sign in as alice@example.com**; signing in needs that click, because company mail filters open every link in an email to scan it. The link works on any device for the whole event, and a lost link can be requested again from the sign-in page.
- Organisers sign in at `/admin`, with a shared master password or, in accounts mode, each with their own email link (see [admin sign-in](#admin-sign-in)).

## Build the calendar image

The server runs from a Docker image that holds the engine and your sealed files, built from your event repository. `advent init` writes what it needs:

- `Dockerfile`: `FROM ghcr.io/ashdevfr/advent:<engine>`, copying the repository into `/event`.
- `.dockerignore`: lets in only `advent.yaml`, the brand, each event's settings and theme, and sealed files. Puzzles, answers, source copies and `editors.key` stay out, even when you build from a working copy that has them.
- `.github/workflows/image.yml`: builds the image and pushes it to your repository's private GitHub container registry. It runs on demand from the Actions tab; add a `push` trigger to rebuild on every push to `main`.

To build by hand instead:

```sh
docker build --build-arg ENGINE=$(grep '^engine:' advent.yaml | tr -d '" ' | cut -d: -f2) -t my-calendar .
```

The release key is not in the image: you give it to the container when it runs.

## Run it

`docker-compose.example.yml` in your repository is a starting point. Copy it to the server as `docker-compose.yml`, set the image name and your settings, and put the secrets in an `.env` file beside it:

```sh
ADVENT_RELEASE_KEY=AGE-SECRET-KEY-1...
ADVENT_ADMIN__PASSWORD=a-long-random-password
ADVENT_MAIL__SMTP__PASSWORD=...
```

Then `docker compose up -d`. The container listens on port 8080; put it behind your usual reverse proxy for HTTPS, and set `ADVENT_TRUST_PROXY=true` so limits apply per visitor rather than to the proxy. Every setting is listed in the [server settings](/reference/server/).

Before it listens, the server opens every sealed day, hint and the certificate with the release key. A wrong key or a broken file stops it at once, with the reason in the log, rather than on the morning a day opens.

### Email

Players get their links by email, through your company's SMTP relay (`mail.sender: smtp`) or [Resend](https://resend.com) (`mail.sender: resend`). `public_url` must be the address players open the calendar at, since links start with it. To try the server without sending anything, leave `mail.sender` at `log`: emails are printed in the log.

## Admin sign-in

Two modes, set with [`admin.auth`](/reference/server/#adminauth):

- **`password`** (the default): one master password from `ADVENT_ADMIN__PASSWORD`, shared by the organisers. Simple for a small team or a one-off event.
- **`accounts`**: each organiser signs in with a link emailed to them, which works once, within 15 minutes. On a new server, open `/admin`: it asks for the **setup code** printed in the server's log (or set with `ADVENT_ADMIN__SETUP_CODE`) and your address, creates you as the first admin and signs you in. The code stops working once an admin exists, so the first visitor cannot claim the server. Add and remove other admins from the **Admins** page; the last one cannot be removed.

## Invite players

1. Open `/admin` and sign in.
2. **Invite**: paste one address per line, optionally followed by a name (`alice@example.com, Alice Martin`), or choose a CSV file with `email` and `name` columns. **Preview** shows what is new, what already exists and what is not an address; nothing changes until you confirm.
3. **Overview**: **Send invitations** emails everyone not invited yet, one at a time, and shows the progress and any failures.

Names go on certificates; players without one type it when they finish.

A player whose link went astray (forwarded by mistake, say) can be given a new one from their page in **Players**: **Reset link** makes the old link and every session it opened stop working, and **Resend invitation** sends the new one.

## Follow the calendar

- **Overview**: how many players are invited, signed in, solved today's door and finished; per day, the share of signed-in players who solved it, the median number of guesses and how many solved it while a hint was out. Days not open yet show only their date.
- **Players**: everyone with their progress; each player's page shows when they solved each day and in how many guesses.
- **Exports**: the finishers (name, email, time) and all players as CSV. advent does not deliver prizes: export the finishers and hand out rewards your way. The certificate's message, files and links are still the in-game reward.

## Limit attempts

Add to `event.yaml`:

```yaml
attempts:
  max: 10
  per: 1h
```

Each player then has 10 guesses per day in any hour, counted from their first guess; after that the door says when they can try again. Without `attempts` guesses are unlimited. A hard cap is deliberately not offered: every word is needed for the certificate, so a player locked out of one day could never finish.

Separately, the server always limits requests per visitor address, to protect itself: see the `limits` settings.

## Update the content

Days are sealed ahead and released by the clock, so the image rarely changes during the event. To fix a day or add one, seal it as usual, push, build a new image and restart the container with it. Players' progress lives in the database volume, not in the image, and is kept.

## Back up

Everything players do is in `/data/advent.db`. Back up the volume like any other database. To move the server, copy the volume and run the same image with the same secrets.
