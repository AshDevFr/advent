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
