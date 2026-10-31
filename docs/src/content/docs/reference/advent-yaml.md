---
title: advent.yaml
description: "Repository configuration: engine version and public keys."
sidebar:
  order: 1
---

At the root of an event repository. Written by `advent init`.

```yaml
engine: "0.1.0"
keys:
  editors:
    - "age1qyqs..."
  release:
    - "age1zvky..."
```

### `engine`

**Type** string · **Required**

The engine version CI downloads and builds with. Bump it to upgrade; see [Upgrade the engine](/guides/upgrade/). `ADVENT_ENGINE` overrides it.

### `keys`

**Type** object · **Required**

The public halves of the two key pairs.

### `editors`

**Type** list of age public keys (`age1…`) · **Required**, at least one · Under `keys`

Every `source.age` is encrypted to all of them; any one matching private key opens it. See [Keys and authors](/guides/keys-and-authors/).

### `release`

**Type** list of age public keys (`age1…`) · **Required**, at least one · Under `keys`

Every day key and hint key is wrapped to all of them. The private key belongs in CI only.
