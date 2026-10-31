# advent

An engine for puzzle advent calendars. Each day opens one door: a teaser, a
code puzzle, a cipher, a logic or image puzzle, a company trivia question or a
meta puzzle. Each solved door gives the player one word, and together the
words unlock a completion certificate.

**[Play the demo](https://advent-demo-ap6.pages.dev)**: seven sample doors,
all open, certificate included.

## How it works

- **One binary, one static site.** `advent` seals your puzzles, then builds a
  static site that any static host can serve. There is no server and no
  database; player progress stays in the browser.
- **Your calendar lives in its own private repository.** It pins an engine
  version, holds your brand and every year's events, and a scheduled CI job
  builds and deploys the site after each midnight.
- **Puzzles are never committed in plaintext.** `advent seal` encrypts each
  day (AES-256-GCM, keys wrapped with [age](https://age-encryption.org)).
  A day's content can only be opened with the release key, which lives in CI
  secrets, so nothing is published before its date, not even by accident.
- **Answers never leave the author.** The site only carries a lock that the
  right answer opens, in the player's browser; the word inside is the
  player's piece of the final key.
- **Authors can work together.** An editors key lets any author reopen and
  edit any day; an author who wants to stay surprised simply does not take
  it, and can still seal their own days.
- **Your brand, every year.** Name, logo and light and dark palettes in
  `brand.yaml`, a yearly look in `theme.yaml`, and days spread however you
  like: consecutive, every few days, workdays only, or on chosen dates.

## Quick start

Download the `advent` binary for your platform from the
[releases](https://github.com/AshDevFr/advent/releases), then:

```sh
advent init my-calendar --event 2026     # prints the release key once: keep it
cd my-calendar
cp -R examples/day01 events/2026/day01   # start from a sample, then edit it
advent seal events/2026/day01            # writes sealed.json and source.age
advent schedule                          # when each door and hint opens
```

Commit, then in your repository's settings add the secrets
`ADVENT_RELEASE_KEY`, `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` and
the variable `CLOUDFLARE_PAGES_PROJECT`. The included workflow verifies,
builds and deploys the calendar to Cloudflare Pages on every push and after
each midnight in December. The README written into your calendar repository
walks through each step.

## Documentation

The full guide and the reference for every configuration option live in
[`docs/`](docs/) and are published with each release.

## Development

```sh
make ci          # everything CI runs: frontend and Rust checks, wasm build, plaintext guard
make demo        # build the public demo into demo-dist/
make acceptance  # end-to-end checks of the static build
```

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in this project by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
