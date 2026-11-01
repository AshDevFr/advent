---
title: theme.yaml
description: "One edition's look, layered over the brand."
sidebar:
  order: 3
---

`events/<event>/theme.yaml`. Optional. A theme can restyle an edition but cannot set `name` or `logo`: identity belongs to the brand.

```yaml
palette:
  light:
    primary: "#b3261e"
  dark:
    primary: "#ff8a80"
```

### `palette`

**Type** object with `light` and `dark` maps · **Optional**

Same shape and keys as in [brand.yaml](/reference/brand-yaml/). Values here win over the brand's, for their scheme only.

The palette also colours this event's card on the landing page, where every event is listed; the rest of that page keeps the brand's colours. The card uses `surface-raised`, `text`, `text-muted`, `primary` and `border`, so set `surface-raised` to make the card stand out.

### `light`

**Type** map of palette key to CSS colour · **Optional** · Under `palette`

This edition's light colours; missing keys keep the brand's (then the engine's).

### `dark`

**Type** map of palette key to CSS colour · **Optional** · Under `palette`

This edition's dark colours; missing keys keep the brand's dark ones (then the engine's).

### `background`

**Type** file name, or `light` and `dark` file names · **Optional**

An image next to `theme.yaml`, or one per scheme as for the brand's [`logo`](/reference/brand-yaml/#logo), shown behind the whole calendar under a wash of the page colour so text stays readable in light and dark mode, and behind this event's card on the landing page. A soft texture or illustration works best; SVG, PNG or WebP.

### `doors`

**Type** file name, or list of file names · **Optional**

Artwork next to `theme.yaml`, shown on locked doors, like wrapping paper. Open and solved doors keep it, faded further so they read as opened. With one file every door uses it; with a list the doors take the images in turn (door 1 the first, and back to the first after the last), so 24 images give every door its own:

```yaml
doors: [door01.png, door02.png, door03.png]
```

The door's number and opening date sit on the artwork with a halo in the door colour; a calm picture with some empty space still reads best.

### `columns`

**Type** whole number from 1 to 12 · **Optional**

How many doors sit in a row on a wide screen. Without it the calendar picks the widest row of 4 to 6 doors that leaves no short last row: 24 days get 6 per row, 15 get 5, 16 get 4. When no width divides evenly, it takes the one whose last row is fullest (13 days get 5, 5 and 3). Set it to choose the shape yourself:

```yaml
columns: 5
```

Smaller screens never show more: tablets show at most 4 doors per row and phones at most 3.
