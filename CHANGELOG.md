# Changelog

All notable changes to advent will be documented in this file.

## [0.2.0] - 2026-10-31

### Features

- Spread an event's days with every, weekdays or explicit dates
- Let authors choose when hints appear
- Show dismissable notices at the top of the calendar
- Serve a calendar under a sub-path with --base or ADVENT_BASE
- *(docs)* Add the documentation site and publish it with the demo
- Add subtitle, signature, image position and background to the certificate
- *(cli)* Preview the calendar locally with `advent preview`
- Add a branded company showcase and a 24-day Christmas showcase
- Show the brand logo, theme background and door artwork
- Unlock reward files and links with the certificate

### Bug Fixes

- *(cli)* Exit quietly when the output pipe closes
- *(web)* Give the certificate room and embed the logo as PNG

### Documentation

- Add the license files, README and contributing guide
- Write the guide and the configuration reference
- Capture the documentation's screenshots from the live showcases
- Link the gallery from the docs home page and the README
- Point the README at advent-engine.4sh.dev
- Publish under advent-engine.4sh.dev

### Testing

- Fail when a configuration field is missing from the docs reference

## [0.1.0] - 2026-10-31

### Features

- *(core)* Normalize answers and flag guessable ones
- *(core)* Add word, bundle, hint and final sealing primitives
- *(core)* Define sealed day and final formats with schema versioning
- *(core)* Wrap keys and encrypt sources with age
- *(core)* Add the event repo content model and release schedule
- *(cli)* Add command structure, config loading and `advent init`
- *(template)* Add one sample day per puzzle type
- *(cli)* Seal a day folder with `advent seal`
- *(cli)* Restore sources and reseal to the current key lists
- *(cli)* Seal the certificate with `advent keeper seal`
- *(cli)* Check an event repo with `advent verify`
- *(web)* Scaffold the React frontend
- *(web)* Open word locks and the certificate with WebCrypto
- *(cli)* Build the static site with `advent build`
- *(cli)* Embed the frontend and write it during `advent build`
- *(web)* Add the landing page and the calendar grid
- *(web)* Open doors, check answers, reveal hints, keep progress
- *(web)* Unlock and export the completion certificate
- *(template)* Build and deploy event repos from GitHub Actions
- *(web)* Redesign after the mockup, with light and dark palettes
- Deploy a public demo of the sample calendar
- Phase acceptance script, and focus back on the door after closing

### Bug Fixes

- *(web)* Explain instead of hanging when WebCrypto is unavailable

### Miscellaneous Tasks

- Scaffold the engine workspace
- Release advent binaries with cargo-dist

