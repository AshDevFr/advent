---
title: final.yaml
description: "The certificate the keeper writes."
sidebar:
  order: 6
---

`events/<event>/final/final.yaml`. Plaintext, never committed. Sealed with `advent keeper seal <event>`. See [The certificate](/guides/certificate/).

```yaml
title: You made it through the longest night
subtitle: is hereby recognised for
message: >
  Seven doors, seven words, one key.
signature: The Holiday Committee
image: medal.png
image_position: bottom
background: paper.png
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

### `image_position`

**Type** `top` or `bottom` · **Default** `bottom`

Where `image` goes: `bottom` under the message, or `top` in place of the brand logo.

### `subtitle`

**Type** string · **Optional**

A line between the title and the player's name, such as "is hereby recognised for".

### `signature`

**Type** string · **Optional**

A sign-off above the footer, such as "The Holiday Committee".

### `background`

**Type** file name · **Optional**

An image next to `final.yaml` shown behind the whole certificate, under a translucent wash of the page colour so the text stays readable. Converted to WebP when sealing; a soft texture works better than a busy photo.

### `rewards`

**Type** list · **Optional**

What else finishing gives, revealed only once the certificate is unlocked. Everyone who finishes gets the same rewards. Each reward has a `label` and exactly one of `file` or `url`:

```yaml
rewards:
  - label: Wallpaper pack
    file: wallpapers.zip
  - label: Bonus puzzle
    url: https://example.com/secret-bonus
```

### `label`

**Type** string · **Required** · Under `rewards`

What the reward is called on the page.

### `file`

**Type** file name · **Optional** · Under `rewards`

A file next to `final.yaml`, of any type, at most 25 MB (the Cloudflare Pages limit). It is encrypted with its own key, which is sealed inside the certificate: the file is published as `<event>/api/rewards/N.bin` but can only be opened, and is only downloaded, once the player has unlocked the certificate. It is saved under its original name. `keeper seal` writes it as `final/reward-N.bin`; commit those files.

### `url`

**Type** URL · **Optional** · Under `rewards`

A link revealed on unlock, sealed inside the certificate. Anyone who finishes can share it, so use it for things that are fine to spread (a bonus puzzle, a playlist); a reward that must be unique per person needs a server.
