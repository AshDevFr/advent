# Advent calendar (company edition)

Puzzles for this calendar are written in plaintext on your machine and sealed
before they are committed. The repository only ever holds locked files.

The calendar runs with `advent serve` for invited players: sign-in links by
email, answers checked on the server, progress in SQLite, and an admin
section at `/admin`. It ships as a Docker image holding the engine and this
repository's sealed files.

## Getting started

1. Install the engine this repository is pinned to ({{engine}}):

   ```sh
   curl --proto '=https' --tlsv1.2 -LsSf https://github.com/AshDevFr/advent/releases/download/v{{engine}}/advent-cli-installer.sh | sh
   ```

   Or skip the install and run it from its Docker image: this shell function
   (bash or zsh) stands in for `advent` until the terminal is closed. It only
   sees the folder it is run from, so run it at the repository root.

   ```sh
   advent() {
     docker run --rm -i --init -u "$(id -u):$(id -g)" -v "$PWD:/work" -w /work \
       ghcr.io/ashdevfr/advent:{{engine}} "$@"
   }
   ```

   Every command below works through it. For `advent preview`, publish the
   port on this machine only and let the preview listen inside the container:

   ```sh
   docker run --rm -i --init -u "$(id -u):$(id -g)" -v "$PWD:/work" -w /work \
     -p 127.0.0.1:4321:4321 ghcr.io/ashdevfr/advent:{{engine}} preview --host 0.0.0.0 --all-open
   ```

   The image has no git, so `advent verify` skips its check for committed
   plaintext there; CI still runs it.

2. Put `editors.key` at the root of the repository (from `advent init`, or
   from whoever runs the calendar). It is gitignored.
3. Write your first day: copy `examples/day01` to `events/{{event}}/day01`,
   edit it, then `advent seal events/{{event}}/day01`.
4. Look at the puzzles on your machine with `advent preview --all-open`
   (http://127.0.0.1:4321; needs the editors key).
5. Run the server itself, as players will use it:

   ```sh
   ADVENT_RELEASE_KEY=AGE-SECRET-KEY-1... \
   ADVENT_ADMIN__PASSWORD=change-me \
   ADVENT_PUBLIC_URL=http://localhost:8080 \
   ADVENT_INSECURE_COOKIES=true \
   advent serve
   ```

   Through the Docker image, the server needs its port published and the
   database kept in the repository's `data/`:

   ```sh
   docker run --rm --init -p 127.0.0.1:8080:8080 \
     -u "$(id -u):$(id -g)" -v "$PWD:/work" -w /work \
     -e ADVENT_RELEASE_KEY=AGE-SECRET-KEY-1... \
     -e ADVENT_ADMIN__PASSWORD=change-me \
     -e ADVENT_PUBLIC_URL=http://localhost:8080 \
     -e ADVENT_INSECURE_COOKIES=true \
     -e ADVENT_DATA=/work/data \
     ghcr.io/ashdevfr/advent:{{engine}} serve
   ```

   Open http://localhost:8080/admin, sign in with the password, invite
   yourself, and send the invitations: with no mail settings, emails are
   printed in the server log, sign-in link included. Progress goes to
   `data/advent.db` (gitignored). To see December before December, add
   `ADVENT_SERVE_NOW=2026-12-05T09:00:00Z`; the clock starts there and runs.
6. Commit and push. Only sealed files go in; the plaintext stays ignored.

## Keys

- **Editors key** (`editors.key`, never committed): opens the encrypted
  sources so any day can be reopened and edited. Share it among authors
  through a password manager or Vault. If it is lost, sealed days still work
  but can no longer be edited, so keep a copy somewhere safe. Authors who
  want to stay surprised can simply not take it: sealing a day only needs the
  public keys in `advent.yaml`.
- **Release key**: opens each day on its date. It lives only with the running
  server, as `ADVENT_RELEASE_KEY` in its `.env` or secret store, never in the
  image or the repository.

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

## Building the image

`Dockerfile` copies this repository into the engine image, and
`.dockerignore` lets in only `advent.yaml`, the brand, each event's settings
and theme, and sealed files: puzzles, answers and `editors.key` stay out
even when you build from a working copy that has them.

- **In CI:** `.github/workflows/image.yml` builds the image and pushes it to
  this repository's GitHub container registry. It runs on demand from the
  Actions tab; add a `push` trigger to rebuild on every push to `main`.
- **By hand:**

  ```sh
  docker build --build-arg ENGINE=$(grep '^engine:' advent.yaml | tr -d '" ' | cut -d: -f2) -t my-calendar .
  ```

The release key is never in the image: you give it to the container when it
runs.

## Running it

`docker-compose.example.yml` is a starting point: copy it to the server as
`docker-compose.yml`, set your settings, put the secrets in an `.env` file
beside it, then `docker compose up -d`. Put it behind your usual reverse
proxy for https.

Every setting, Kubernetes and other ways to run it:
https://advent-engine.4sh.dev/guides/self-hosting/ and
https://advent-engine.4sh.dev/reference/server/.
