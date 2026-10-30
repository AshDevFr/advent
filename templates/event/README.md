# Advent calendar

Puzzles for this calendar are written in plaintext on your machine and sealed
before they are committed. The repository only ever holds locked files.

## Keys

- **Editors key** (`editors.key`, never committed): opens the encrypted
  sources so any day can be reopened and edited. Share it among authors
  through a password manager or Vault. If it is lost, sealed days still work
  but can no longer be edited, so keep a copy somewhere safe. Authors who
  want to stay surprised can simply not take it: sealing a day only needs the
  public keys in `advent.yaml`.
- **Release key**: opens each day on its date. It lives only in CI secrets as
  `ADVENT_RELEASE_KEY`. Anyone who can push to this repository could read it
  through a workflow, so keep push access to people you trust with it.

## Writing a day

1. Copy an example from `examples/` to `events/<event>/dayNN/`.
2. Edit `puzzle.md` and `meta.yaml`. Words must be lowercase letters and
   digits only.
3. `advent seal events/<event>/dayNN`
4. Commit. Only `sealed.json` and `source.age` are picked up; the plaintext
   stays ignored.

To edit a sealed day elsewhere: `advent unseal-source events/<event>/dayNN`.

`examples/` holds public sample days and is the one place plaintext is
committed on purpose. Never write a real puzzle there.

## Publishing

`.github/workflows/build.yml` builds the site and deploys it to Cloudflare
Pages after each midnight in December, on every push to `main`, and from the
Actions tab on demand. Each run publishes exactly what is out at that moment,
so running it again is harmless.

One-time setup:

1. Create a Cloudflare Pages project (Direct Upload) and note its name.
2. In this repository's settings, add the secrets `ADVENT_RELEASE_KEY` (from
   `advent init`), `CLOUDFLARE_API_TOKEN` (Pages: Edit) and
   `CLOUDFLARE_ACCOUNT_ID`, and the variable `CLOUDFLARE_PAGES_PROJECT`.
3. If your event is not in UTC, adjust the cron hours in the workflow so one
   run lands just after your local midnight.

Before a deploy, CI checks that no plaintext is tracked and that every day
released so far opens; days still being written do not block it.
