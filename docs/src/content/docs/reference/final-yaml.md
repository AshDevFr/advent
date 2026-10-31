---
title: final.yaml
description: "The certificate the keeper writes."
sidebar:
  order: 6
---

`events/<event>/final/final.yaml`. Plaintext, never committed. Sealed with `advent keeper seal <event>`. See [The certificate](/guides/certificate/).

```yaml
title: You made it through the longest night
message: >
  Seven doors, seven words, one key.
image: medal.png
```

### `title`

**Type** string · **Required**

The certificate's headline.

### `message`

**Type** string · **Required**

The text under the player's name.

### `image`

**Type** file name · **Optional**

An image next to `final.yaml`, converted to WebP and shown on the certificate.
