---
title: Security model
description: "What advent protects, and what it does not."
sidebar:
  order: 8
---

advent is built for friends and colleagues, not adversaries with a lab. Here is what holds and what does not.

## Strongly protected

- **Unreleased days and hints.** Each day is encrypted with its own random 256-bit key (AES-256-GCM), and that key is wrapped with age to the release key. Nothing about a day is on the site before its date, in any form.
- **Plaintext in git.** Only `sealed.json` and `source.age` are committed. `.gitignore` blocks the rest, and `advent verify` fails if plaintext was force-added.
- **The deployed site.** It never contains an answer or a source copy. The build only has the release key, so it cannot read sources even by mistake.

## As strong as the answer

A day's word is locked with a key derived from each accepted answer (PBKDF2-SHA256, 150 000 iterations). The lock is public once the day is released, so anyone can try guesses offline:

- a 4-digit number falls in minutes on a laptop;
- a single dictionary word falls in seconds to minutes on a gaming GPU;
- two unrelated words or a short phrase are impractical to guess.

`advent seal` warns about weak answers. Someone who cracks a lock spoils their own game, not anyone else's.

## The keys

- Whoever holds the **release key** can open every day early. It lives in CI secrets; anyone with push access to the repository can read it through a workflow, so limit push access.
- Whoever holds the **editors key** can read every source. That is by design: authors may read each other's days, deliberately.
- Players' progress lives in their browser and is not secret from them. Faked progress cannot open the certificate, which needs the real words.
