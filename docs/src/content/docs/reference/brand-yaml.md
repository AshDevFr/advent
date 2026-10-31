---
title: brand.yaml
description: "Name, logo and palettes shared by every event."
sidebar:
  order: 2
---

`brand/brand.yaml`, shared by every event in the repository. Optional: without it, the engine's name and colours are used.

```yaml
name: Northwind
logo: logo.svg
palette:
  light:
    primary: "#1f5f8b"
  dark:
    primary: "#7cc4f0"
```

### `name`

**Type** string · **Required**

Shown on the landing page, in the event page's eyebrow and on the certificate.

### `logo`

**Type** file name, or `light` and `dark` file names · **Optional**

An image next to `brand.yaml` (SVG or PNG), shown on the landing page, at the top of each calendar and on the certificate. A logo drawn for a light page can disappear on a dark one, so you can give one per scheme; each page shows the one for the colours it is in:

```yaml
logo:
  light: logo.svg
  dark: logo-dark.svg
```

### `palette`

**Type** object · **Optional**

Colours for each scheme. See [Brand and theme](/guides/brand-and-theme/) for the keys (`primary`, `on-primary`, `surface`, `surface-raised`, `text`, `text-muted`, `accent`, `border`).

### `light`

**Type** map of palette key to CSS colour · **Optional** · Under `palette`

Colours when the page is light. Missing keys keep the engine defaults.

### `dark`

**Type** map of palette key to CSS colour · **Optional** · Under `palette`

Colours when the page is dark (system setting or the player's choice). Missing keys keep the engine's dark defaults.
