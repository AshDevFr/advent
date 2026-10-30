//! `advent keeper seal <event>`: seal the certificate with the key built
//! from the event's words. The words are read from the sealed days' source
//! copies in day order, so the keeper never types a list that could differ
//! from what players actually collect.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use advent_core::format::{self, Certificate, SCHEMA, SealedFinal};
use advent_core::model::{Event, FinalPayload};
use advent_core::primitives;
use advent_core::wrap::{self, x25519};
use anyhow::{Context, bail, ensure};

use crate::config::{editors_identities, find_repo_root, load_repo_config};
use crate::day::{self, Plaintext, SEALED_FILE, SOURCE_FILE};
use crate::{images, random};

pub const FINAL_DIR: &str = "final";
const FINAL_FILE: &str = "final.yaml";

pub fn run(event_id: &str) -> anyhow::Result<()> {
    let root = find_repo_root(Path::new("."))?;
    let config = load_repo_config(&root)?;
    let editors = wrap::parse_recipients(&config.keys.editors)?;
    let identities = editors_identities(&root)?;
    let event_dir = root.join("events").join(event_id);
    let final_dir = event_dir.join(FINAL_DIR);

    let words = event_words(&event_dir, &identities)?;
    let mut files = BTreeMap::new();
    for entry in fs::read_dir(&final_dir)
        .with_context(|| format!("no {} folder with {FINAL_FILE}", final_dir.display()))?
    {
        let path = entry?.path();
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        if path.is_file() && name != SEALED_FILE && name != SOURCE_FILE {
            files.insert(name, fs::read(&path)?);
        }
    }

    seal_final(&final_dir, &files, &words, &editors)?;
    println!(
        "Sealed the certificate for {event_id} with the words of its {} days.",
        words.len()
    );
    println!("Commit {FINAL_DIR}/{SEALED_FILE} and {FINAL_DIR}/{SOURCE_FILE}.");
    Ok(())
}

/// Every day's word in day order, read from the source copies. Fails, naming
/// them, if any day of the event is not sealed yet.
pub fn event_words(
    event_dir: &Path,
    identities: &[x25519::Identity],
) -> anyhow::Result<Vec<String>> {
    let event_file = event_dir.join("event.yaml");
    let event = Event::from_yaml(
        &fs::read_to_string(&event_file)
            .with_context(|| format!("reading {}", event_file.display()))?,
    )?;
    let day_dir = |n: u32| event_dir.join(format!("day{n:02}"));

    let missing: Vec<String> = (1..=event.days)
        .filter(|n| !day_dir(*n).join(SOURCE_FILE).is_file())
        .map(|n| n.to_string())
        .collect();
    if !missing.is_empty() {
        bail!(
            "the certificate needs every day's word, but these days are not sealed: {}",
            missing.join(", ")
        );
    }

    (1..=event.days)
        .map(|n| {
            let encrypted = fs::read(day_dir(n).join(SOURCE_FILE))?;
            let source = day::unpack_source(&wrap::decrypt_with(&encrypted, identities)?)?;
            Ok(Plaintext::from_files(source.files)
                .with_context(|| format!("day {n}"))?
                .meta
                .word)
        })
        .collect()
}

/// Seal `final.yaml` (and its optional image) from `files`, check it opens
/// with `words`, and write `sealed.json` and `source.age` into `final_dir`.
pub fn seal_final(
    final_dir: &Path,
    files: &BTreeMap<String, Vec<u8>>,
    words: &[String],
    editors: &[x25519::Recipient],
) -> anyhow::Result<()> {
    let payload = FinalPayload::from_yaml(std::str::from_utf8(
        files
            .get(FINAL_FILE)
            .with_context(|| format!("no {FINAL_FILE} in {}", final_dir.display()))?,
    )?)?;
    let image = match &payload.image {
        Some(name) => Some(
            images::to_webp(
                files
                    .get(name)
                    .with_context(|| format!("image {name:?} not found"))?,
            )
            .with_context(|| format!("converting {name}"))?,
        ),
        None => None,
    };
    let certificate = Certificate {
        title: payload.title,
        message: payload.message,
        image,
    };
    let plaintext = serde_json::to_vec(&certificate)?;

    let words: Vec<&str> = words.iter().map(String::as_str).collect();
    let salt = random::bytes();
    let iv = random::bytes();
    let sealed = SealedFinal {
        schema: SCHEMA,
        sealed_by: format!("advent {}", env!("CARGO_PKG_VERSION")),
        salt,
        iv,
        ct: primitives::seal_final(&words, salt, iv, &plaintext),
    };
    let sealed_json = serde_json::to_vec_pretty(&sealed)?;

    let reread = format::read_sealed_final(&sealed_json)?;
    let opened = primitives::open_final(&words, reread.salt, reread.iv, &reread.ct)?;
    ensure!(
        opened == plaintext,
        "round-trip check failed; nothing was written"
    );

    let source = wrap::encrypt_to(&day::pack_source(files, None)?, editors)?;
    fs::create_dir_all(final_dir)?;
    day::write_atomic(&final_dir.join(SEALED_FILE), &sealed_json)?;
    day::write_atomic(&final_dir.join(SOURCE_FILE), &source)
}
