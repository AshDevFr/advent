---
title: Server settings
description: "Every setting of advent serve (company edition), as YAML keys and environment variables."
sidebar:
  order: 9
---

`advent serve` takes its settings from, in order of precedence: command-line flags, `ADVENT_*` environment variables, the YAML file given with `--config`, then the defaults below. In the environment, a setting's path is written in capitals with `__` between levels: `mail.smtp.password` is `ADVENT_MAIL__SMTP__PASSWORD`. Put secrets in the environment, not in the file.

```yaml
listen: 0.0.0.0:8080
public_url: https://advent.example.com
data: /data
trust_proxy: true
limits:
  guesses_per_minute: 10
mail:
  sender: smtp
  from: "Advent <advent@example.com>"
  smtp:
    host: smtp.example.com
    port: 587
    username: advent
admin:
  auth: password
```

## Server

### `listen`

**Type** address and port · **Default** `0.0.0.0:8080`

Where the server listens. `--listen` overrides it.

### `public_url`

**Type** URL · **Required** to send real email

The address players open the calendar at, such as `https://advent.example.com`. Sign-in links in emails start with it.

### `data`

**Type** folder · **Default** `data` (`/data` in the image)

Where the SQLite database `advent.db` lives: players, progress and admin sessions. Back it up. `--data` overrides it.

### `insecure_cookies`

**Type** boolean · **Default** `false`

Send cookies without the `Secure` flag, so signing in works over plain `http://` on your own machine. Never in production.

### `trust_proxy`

**Type** boolean · **Default** `false`

Take each visitor's address from the last `X-Forwarded-For` entry. Turn it on behind a reverse proxy that sets that header; otherwise every request seems to come from the proxy and shares one set of limits. Without a proxy, leave it off: clients could pick their own address.

## Limits

Always on, per visitor address, to keep the server up. The [attempt limit](/reference/event-yaml/#attempts) for players is set per event in `event.yaml`.

### `limits.requests_per_minute`

**Type** integer · **Default** `600`

Any request.

### `limits.guesses_per_minute`

**Type** integer · **Default** `10`

Answers checked. Each costs a key derivation, tens of milliseconds of CPU.

### `limits.signin_per_hour`

**Type** integer · **Default** `20`

Sign-ins and "send my link" requests.

### `limits.links_per_hour`

**Type** integer · **Default** `3`

Sign-in links emailed to one address, whoever asks, so the form cannot flood an inbox.

### `limits.concurrent_checks`

**Type** integer ≥ 1 · **Default** `4`

Answer checks running at the same time, across all visitors. Others wait their turn.

## Mail

### `mail.sender`

**Type** `log`, `smtp` or `resend` · **Default** `log`

How emails are sent. `log` prints them (or writes them to `mail.log_dir`) instead of sending: for trying the server out.

### `mail.from`

**Type** string · **Required** for `smtp` and `resend`

The sender, as `Name <address>` or a bare address.

### `mail.log_dir`

**Type** folder · **Optional**

With `log`: write each email as a JSON file in this folder instead of printing it.

### `mail.smtp.host`

**Type** host name · **Required** for `smtp`

Your mail relay.

### `mail.smtp.port`

**Type** integer · **Default** `587`

### `mail.smtp.username`

**Type** string · **Optional**

Sign in to the relay as this user. Without it, mail is sent without signing in (a relay that trusts the server's network).

### `mail.smtp.password`

**Type** string · **Optional** · `ADVENT_MAIL__SMTP__PASSWORD`

### `mail.smtp.tls`

**Type** `starttls`, `tls` or `none` · **Default** `starttls`

`starttls` upgrades a plain connection (usually port 587), `tls` encrypts from the start (usually 465), `none` sends in the clear: only for a relay on the same host or private network.

### `mail.resend.api_key`

**Type** string · **Required** for `resend` · `ADVENT_MAIL__RESEND__API_KEY`

A [Resend](https://resend.com) API key allowed to send from `mail.from`'s domain.

### `mail.resend.endpoint`

**Type** URL · **Default** `https://api.resend.com`

## Admin

### `admin.auth`

**Type** `password` · **Default** `password`

How organisers sign in to `/admin`. `password`: one shared master password, no admin accounts.

### `admin.password`

**Type** string · **Required** with `password` · `ADVENT_ADMIN__PASSWORD`

The master password. The server refuses to start without one. Five attempts per address every 15 minutes; failures are logged.
