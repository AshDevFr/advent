# Changelog

All notable changes to advent will be documented in this file.

## [0.5.1] - 2026-10-31

### Bug Fixes

- Render tables in puzzles

## [0.5.0] - 2026-10-31

### Features

- Split the event template into a static and a company edition
- Let preview serve beyond loopback, and drop the countdown once a day is out

### Documentation

- Show how to run the engine from its Docker image without installing it

## [0.4.0] - 2026-10-31

### Features

- Theme each calendar's landing card and give doors their own artwork
- Keep a faded door picture on open and solved doors
- Let logos and backgrounds have a version for dark pages

### Bug Fixes

- Centre the countdown's numbers over their units at equal widths

## [0.3.1] - 2026-10-31

### Bug Fixes

- Build Apple silicon releases on macos-15

## [0.3.0] - 2026-10-31

### Features

- Add the runtime-independent server core for the company edition
- Host the calendar for invited players with `advent serve`
- Email sign-in links through SMTP, Resend or a log
- Sign players in and play through the server on a hosted calendar
- Manage players and follow the calendar from an admin section
- Ship the company edition as a Docker image, with its docs
- Let organisers sign in as themselves with admin accounts
- Sign admins in through the company's identity provider

### Refactor

- Put the calendar's answers and progress behind a client

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

