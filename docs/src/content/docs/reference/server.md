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

**Type** `password` or `accounts` · **Default** `password`

How organisers sign in to `/admin`. `password`: one shared master password, no admin accounts, for small or one-off setups. `accounts`: admins are people, each signing in with a link emailed to them (single use, valid 15 minutes); the first admin is created with the setup code, and further admins are added from the **Admins** page. Switching between the two signs every admin out.

### `admin.password`

**Type** string · **Required** with `password` · `ADVENT_ADMIN__PASSWORD`

The master password. The server refuses to start without one. Five attempts per address every 15 minutes; failures are logged.

### `admin.setup_code`

**Type** string · **Optional** · `ADVENT_ADMIN__SETUP_CODE`

With `accounts`: the one-time code that creates the first admin at `/admin`. Without it, the server generates one and prints it in its log at every start while no admin exists, so only someone who can read the log can claim a new server. It stops working once an admin exists.

## Single sign-on

With `admin.auth: accounts`, admins can also sign in through your company's identity provider (OpenID Connect: Microsoft Entra ID, Okta, Google Workspace, Keycloak, Authentik and others). Members of `admin.oidc.admin_group` are admins; nobody else is. Register the server with the provider as a web application whose redirect URI is `<public_url>/api/admin/oidc/callback`, and have the provider put the user's groups in the ID token.

### `admin.oidc.issuer`

**Type** URL · **Optional**

The provider's issuer, as in its `/.well-known/openid-configuration`. Setting it turns single sign-on on.

### `admin.oidc.client_id`

**Type** string · **Required** with `issuer`

### `admin.oidc.client_secret`

**Type** string · **Optional** · `ADVENT_ADMIN__OIDC__CLIENT_SECRET`

The client secret, for a confidential client. The sign-in always uses PKCE as well.

### `admin.oidc.admin_group`

**Type** string · **Required** with `issuer`

The group whose members are admins, as the provider writes it in the groups claim (a name or, with Entra ID, the group's object id).

### `admin.oidc.groups_claim`

**Type** string · **Default** `groups`

The ID token claim listing the user's groups, if the provider uses another name.

### `admin.oidc.label`

**Type** string · **Default** `single sign-on`

The provider's name on the button: **Sign in with Acme SSO**.
