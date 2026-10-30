# web

The player-facing frontend: React + Vite + TypeScript, Tailwind v4 + shadcn/ui, Zustand.
It is built and embedded into the `advent` binary, which writes it out during `advent build`.

Every colour comes from CSS variables (`src/index.css`) named after the palette keys in
`brand.yaml`; the theme CSS written by `advent build` redefines them.

```sh
make frontend-dev    # Vite dev server
make frontend-ci     # lint, tests, build
```
