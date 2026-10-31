//! `advent seal <day-dir>`: turn a plaintext day folder into its committed
//! form, `sealed.json` and `source.age`. Only the public keys in
//! `advent.yaml` are needed, so any author can seal. Nothing is written
//! unless everything sealed opens again with the keys just generated.

use std::path::Path;

use advent_core::format::{self, Bundle, Encrypted, SCHEMA, SealedDay, SealedHint};
use advent_core::model::{DayMeta, Event};
use advent_core::primitives::{self, WordLock};
use advent_core::wrap::{self, x25519};
use advent_core::{GuessableReason, guessable, is_normalized_word, normalize};
use anyhow::{Context, bail, ensure};

use crate::config::{find_repo_root, load_repo_config};
use crate::day::{self, DayKeys, Plaintext, SEALED_FILE, SOURCE_FILE};
use crate::{images, random};

pub fn run(day_dir: &Path) -> anyhow::Result<()> {
    let day = day::locate(day_dir)?;
    let root = find_repo_root(&day.path)?;
    let config = load_repo_config(&root)?;
    let release = wrap::parse_recipients(&config.keys.release)?;
    let editors = wrap::parse_recipients(&config.keys.editors)?;
    let plain = day::load_plaintext(&day.path)?;

    let output = seal_day(&day.event, day.number, &plain, &release, &editors)?;
    output.write(&day.path)?;

    println!(
        "Sealed day {}: {} answer lock(s), {} hint(s), {} image(s).",
        day.number, output.locks, output.hints, output.images
    );
    println!("Commit {SEALED_FILE} and {SOURCE_FILE}; the plaintext stays ignored.");
    Ok(())
}

/// The committed form of one day, ready to write.
pub struct Sealed {
    pub sealed_json: Vec<u8>,
    pub source: Vec<u8>,
    locks: usize,
    hints: usize,
    images: usize,
}

impl Sealed {
    pub fn write(&self, day_path: &Path) -> anyhow::Result<()> {
        day::write_atomic(&day_path.join(SEALED_FILE), &self.sealed_json)?;
        day::write_atomic(&day_path.join(SOURCE_FILE), &self.source)
    }
}

/// Seal a day's plaintext with fresh keys. Used by `seal`, and by
/// `reseal --all` with plaintext taken from the source copy.
pub fn seal_day(
    event: &Event,
    number: u32,
    plain: &Plaintext,
    release: &[x25519::Recipient],
    editors: &[x25519::Recipient],
) -> anyhow::Result<Sealed> {
    let meta = &plain.meta;
    if !is_normalized_word(&meta.word) {
        bail!(
            "word {:?} must already be in normalized form: lowercase a-z and 0-9 only, \
             no spaces or accents (it would normalize to {:?}). Words are joined to build \
             the final key, so any difference would lock every player out of the certificate.",
            meta.word,
            normalize(&meta.word)
        );
    }

    // Answers that normalize alike would only produce identical locks.
    let mut answers: Vec<&String> = Vec::new();
    for answer in &meta.answers {
        let normalized = normalize(answer);
        if normalized.is_empty() {
            bail!("answer {answer:?} is empty after normalization (only a-z and 0-9 are kept)");
        }
        if !answers.iter().any(|a| normalize(a) == normalized) {
            answers.push(answer);
        }
    }
    for answer in &answers {
        warn_if_guessable(answer);
    }
    let overrides: Vec<Option<u32>> = meta.hints.iter().map(|h| h.after).collect();
    event.hint_days(&overrides)?;

    let converted = images::convert(&plain.files, &plain.puzzle)?;
    if converted.total_bytes > images::DAY_IMAGE_BUDGET {
        eprintln!(
            "warning: day {number} has {:.1} MB of images after conversion (budget {:.1} MB); \
             players on phones will wait",
            converted.total_bytes as f64 / 1e6,
            images::DAY_IMAGE_BUDGET as f64 / 1e6
        );
    }

    let bundle = Bundle {
        title: meta.title.clone(),
        kind: meta.kind.clone(),
        markdown: converted.markdown,
        assets: converted.assets,
    };
    let keys = DayKeys {
        day_key: random::bytes(),
        hint_keys: meta.hints.iter().map(|_| random::bytes()).collect(),
    };
    let bundle_iv = random::bytes();
    let bundle_ct =
        primitives::seal_bundle(&keys.day_key, bundle_iv, &serde_json::to_vec(&bundle)?);

    let word_locks = answers
        .iter()
        .map(|a| primitives::seal_word(a, &meta.word, random::bytes(), random::bytes()))
        .collect::<Result<Vec<_>, _>>()?;

    let mut hints = Vec::new();
    for (hint, key) in meta.hints.iter().zip(&keys.hint_keys) {
        let iv = random::bytes();
        hints.push(SealedHint {
            iv,
            ct: primitives::seal_hint(key, iv, &hint.text),
            key: wrap::wrap_key(key, release)?,
            after: hint.after,
        });
    }

    let sealed = SealedDay {
        schema: SCHEMA,
        sealed_by: format!("advent {}", env!("CARGO_PKG_VERSION")),
        day: number,
        bundle: Encrypted {
            iv: bundle_iv,
            ct: bundle_ct,
        },
        day_key: wrap::wrap_key(&keys.day_key, release)?,
        word_locks,
        hints,
    };
    let sealed_json = serde_json::to_vec_pretty(&sealed)?;
    let source = wrap::encrypt_to(&day::pack_source(&plain.files, Some(&keys))?, editors)?;

    round_trip(&sealed_json, &bundle, &keys, &answers, meta)
        .context("round-trip check failed; nothing was written")?;

    Ok(Sealed {
        sealed_json,
        source,
        locks: sealed.word_locks.len(),
        hints: sealed.hints.len(),
        images: bundle.assets.len(),
    })
}

fn warn_if_guessable(answer: &str) {
    let reasons = guessable(answer);
    if reasons.is_empty() {
        return;
    }
    let why: Vec<&str> = reasons
        .iter()
        .map(|r| match r {
            GuessableReason::DigitsOnly => "digits only",
            GuessableReason::TooShort => "shorter than 5 characters",
            GuessableReason::ClosedSet => "one of a small well-known set",
        })
        .collect();
    eprintln!(
        "warning: answer {answer:?} is easy to guess offline ({}). Consider asking for \
         two things at once, such as \"2014 lisbon\".",
        why.join(", ")
    );
}

/// Open everything just sealed with the keys still in memory. The release
/// and editors private keys are not available here, so their wrapping is
/// checked for presence only; `advent verify` opens them fully.
fn round_trip(
    sealed_json: &[u8],
    bundle: &Bundle,
    keys: &DayKeys,
    answers: &[&String],
    meta: &DayMeta,
) -> anyhow::Result<()> {
    let sealed = format::read_sealed_day(sealed_json)?;
    let opened: Bundle = serde_json::from_slice(&primitives::open_bundle(
        &keys.day_key,
        sealed.bundle.iv,
        &sealed.bundle.ct,
    )?)?;
    ensure!(
        &opened == bundle,
        "bundle does not decrypt to what was sealed"
    );

    for answer in answers {
        let opens = sealed.word_locks.iter().any(|lock: &WordLock| {
            primitives::open_word(answer, lock).ok().as_ref() == Some(&meta.word)
        });
        ensure!(opens, "answer {answer:?} does not open the word");
    }
    ensure!(
        sealed.hints.len() == meta.hints.len(),
        "hint count changed while sealing"
    );
    for ((hint, key), sealed_hint) in meta.hints.iter().zip(&keys.hint_keys).zip(&sealed.hints) {
        let text = primitives::open_hint(key, sealed_hint.iv, &sealed_hint.ct)?;
        ensure!(
            text == hint.text,
            "a hint does not decrypt to what was sealed"
        );
        ensure!(!sealed_hint.key.is_empty(), "a hint key was not wrapped");
    }
    ensure!(!sealed.day_key.is_empty(), "day key was not wrapped");
    Ok(())
}
