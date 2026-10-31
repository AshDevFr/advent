---
title: Brand and theme
description: "Name, logo, colours for light and dark, and a yearly look."
sidebar:
  order: 4
---

Two layers restyle the calendar without touching code. Each defines a **light** and a **dark** palette; players follow their system setting by default and can switch with the System / Light / Dark toggle, remembered in their browser.

1. **Engine defaults**: a warm light palette and a dark navy with amber.
2. **`brand/brand.yaml`**: who runs the calendar. Shared by every event in the repository.
3. **`events/<event>/theme.yaml`**: one edition's look, layered over the brand. It can change colours but never the name or logo.

A key left out of a layer keeps the one below, for that scheme only.

```yaml
# brand/brand.yaml
name: Northwind
logo: logo.svg
palette:
  light:
    primary: "#1f5f8b"
    surface: "#f4f7fa"
  dark:
    primary: "#7cc4f0"
    surface: "#0d1620"
```

## Palette keys

| Key | Used for |
|---|---|
| `primary` | buttons, solved doors, accents |
| `on-primary` | text on `primary` |
| `surface` | page background |
| `surface-raised` | cards and doors |
| `text` | main text |
| `text-muted` | secondary text, locked doors |
| `accent` | highlights |
| `border` | borders and dividers |
| `radius` | corner roundness of doors, cards and buttons (a CSS length such as `0.25rem`; default `0.75rem`) |

Values are CSS colours (`#1f5f8b`, `rgb(…)`, `oklch(…)`). Characters that could break out of a CSS rule (`;`, `{`, `}`, `<`, `>`, `\`) are refused at build time.

## Logo and decoration

The brand's `logo` appears in the browser tab, on the landing page, next to the calendar's title and on the certificate.

An event's `theme.yaml` can add a `background` image, shown behind the calendar under a wash of the page colour, and `doors` artwork, shown on locked doors like wrapping paper until they open. SVG, PNG and WebP all work; a soft image reads better than a busy one.

See the [brand.yaml](/reference/brand-yaml/) and [theme.yaml](/reference/theme-yaml/) references.
