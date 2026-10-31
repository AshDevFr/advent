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

### `light`

**Type** map of palette key to CSS colour · **Optional** · Under `palette`

This edition's light colours; missing keys keep the brand's (then the engine's).

### `dark`

**Type** map of palette key to CSS colour · **Optional** · Under `palette`

This edition's dark colours; missing keys keep the brand's dark ones (then the engine's).

### `background`

**Type** file name · **Optional**

An image next to `theme.yaml`, published at `<event>/theme/<file>`. Reserved for decoration: the current calendar does not display it yet.

### `doors`

**Type** file name · **Optional**

Door artwork next to `theme.yaml`, published like `background`. Reserved: not displayed yet.

### `intro`

**Type** string · **Optional**

Reserved: not used yet. The calendar's intro text comes from `story.intro` in [event.yaml](/reference/event-yaml/).
