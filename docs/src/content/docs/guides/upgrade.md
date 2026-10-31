---
title: Upgrade the engine
description: "Pin a newer advent and move sealed files to its format."
sidebar:
  order: 7
---

`engine` in `advent.yaml` pins the version CI downloads. To upgrade:

1. Install the new `advent` locally and set `engine` to its version.
2. Run `advent verify`. If it reports files sealed in another schema, run:

   ```sh
   advent reseal --all
   ```

   This seals every day and the certificate again from their source copies, in the new format, with fresh keys. It needs the editors key. Players keep their progress: their words do not change.
3. Commit and push.

Each sealed file records its `schema` and the engine version that sealed it (`sealed_by`). An engine only reads its own schema and names the version to install otherwise, so an old repository can always be built by pinning the engine it was sealed with.
