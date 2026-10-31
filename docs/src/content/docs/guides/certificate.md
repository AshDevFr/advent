---
title: The certificate
description: "What the keeper writes and how players unlock it."
sidebar:
  order: 3
---

The certificate is the reward: a title, a message and an optional image, encrypted with a key built from **every word of the event, in day order, joined by single spaces**.

## Write and seal it

The keeper writes `events/<event>/final/final.yaml`:

```yaml
title: You made it through the longest night
message: >
  Seven doors, seven words, one key. Thank you for playing.
image: medal.png      # optional, next to final.yaml
```

Optional extras: `subtitle` (a line above the name), `signature` (a sign-off at the bottom), `image_position` (`top` to put the image where the logo goes) and `background` (an image behind the whole certificate). See the [final.yaml reference](/reference/final-yaml/).

Then, once every day is sealed:

```sh
advent keeper seal 2026
```

The words are read from the days' source copies (it needs the editors key), so nobody retypes them. Sealing refuses while any day is missing. If a day is re-sealed later with a different word, `advent verify` reports the certificate as stale: seal it again.

## Rewards

Finishing can unlock more than the certificate: files to download and links, listed under the certificate once it is unlocked.

```yaml
rewards:
  - label: Wallpaper pack
    file: wallpapers.zip
  - label: Bonus puzzle
    url: https://example.com/secret-bonus
```

Files are encrypted one by one and only downloaded when the player asks for them. Everyone who finishes gets the same rewards, so a code that must be unique per person (a gift card, for instance) does not fit here. See the [final.yaml reference](/reference/final-yaml/#rewards).

## Preview it

```sh
advent preview --all-open --solved
```

opens the calendar on your machine with every word already collected: type any name in the certificate panel to see the real certificate and download the PNG.

## What players get

When every word is collected, the player enters a name and the certificate opens in the browser. It shows your brand, the player's name, the edition and its year, and can be downloaded as a PNG. It does not show hints used or attempts: those live in the player's browser and could be reset, so they would prove nothing.
