# advent

An engine for puzzle advent calendars. Each day opens one door: a teaser, a code puzzle, a
cipher, a logic or image puzzle, a company trivia question or a meta puzzle. Each solved day
yields one word, and the words together unlock a completion certificate.

The engine is a single `advent` binary. Your calendar lives in a separate, private event repo
that pins an engine version, holds your brand and every year's sealed puzzles, and deploys a
static site on a schedule. Authors never commit plaintext: puzzles are sealed with age and
AES-GCM, and each day is only published on its date.

Status: early development.

## Development

```sh
make ci        # format check, clippy, tests, wasm32 build of advent-core, plaintext guard
make check     # format, lint, test
```
