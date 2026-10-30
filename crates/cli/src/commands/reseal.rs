//! `advent reseal [event] [--all]`: make every sealed file match the key
//! lists currently in `advent.yaml`. This is how keys are added, rotated and
//! removed: edit the lists, reseal, commit.
//!
//! Only the editors key is needed: each source copy holds its day's keys,
//! so they can be re-wrapped for new release recipients without the old
//! release key. With `--all`, every day is sealed again from its source with
//! fresh keys in the current schema, which is how an event repo moves to a
//! new engine version.

use std::fs;
use std::path::{Path, PathBuf};

use advent_core::format;
use advent_core::wrap::{self, x25519};
use anyhow::{Context, bail, ensure};

use crate::commands::seal;
use crate::config::{editors_identities, find_repo_root, load_repo_config};
use crate::day::{self, Plaintext, SEALED_FILE, SOURCE_FILE};

pub fn run(event: Option<&str>, all: bool) -> anyhow::Result<()> {
    let root = find_repo_root(Path::new("."))?;
    let config = load_repo_config(&root)?;
    let release = wrap::parse_recipients(&config.keys.release)?;
    let editors = wrap::parse_recipients(&config.keys.editors)?;
    let identities = editors_identities(&root)?;

    let mut count = 0;
    for event_dir in event_dirs(&root, event)? {
        for day_dir in sealed_days(&event_dir)? {
            reseal_day(&day_dir, all, &identities, &release, &editors)
                .with_context(|| format!("resealing {}", day_dir.display()))?;
            count += 1;
        }
    }
    println!(
        "Resealed {count} day(s) to {} editors and {} release key(s){}.",
        editors.len(),
        release.len(),
        if all {
            ", with fresh keys in the current schema"
        } else {
            ""
        }
    );
    Ok(())
}

fn reseal_day(
    day_dir: &Path,
    all: bool,
    identities: &[x25519::Identity],
    release: &[x25519::Recipient],
    editors: &[x25519::Recipient],
) -> anyhow::Result<()> {
    let archive = wrap::decrypt_with(&fs::read(day_dir.join(SOURCE_FILE))?, identities)?;
    let source = day::unpack_source(&archive)?;

    if all {
        let day = day::locate(day_dir)?;
        let plain = Plaintext::from_files(source.files)?;
        return seal::seal_day(day.number, &plain, release, editors)?.write(day_dir);
    }

    let mut sealed = format::read_sealed_day(&fs::read(day_dir.join(SEALED_FILE))?)
        .context("use `advent reseal --all` to re-seal it in the current schema")?;
    ensure!(
        sealed.hints.len() == source.keys.hint_keys.len(),
        "sealed file and source copy disagree on the number of hints"
    );
    sealed.day_key = wrap::wrap_key(&source.keys.day_key, release)?;
    for (hint, key) in sealed.hints.iter_mut().zip(&source.keys.hint_keys) {
        hint.key = wrap::wrap_key(key, release)?;
    }
    day::write_atomic(
        &day_dir.join(SEALED_FILE),
        &serde_json::to_vec_pretty(&sealed)?,
    )?;
    day::write_atomic(
        &day_dir.join(SOURCE_FILE),
        &wrap::encrypt_to(&archive, editors)?,
    )
}

pub fn event_dirs(root: &Path, event: Option<&str>) -> anyhow::Result<Vec<PathBuf>> {
    let events = root.join("events");
    if let Some(id) = event {
        let dir = events.join(id);
        if !dir.is_dir() {
            bail!("no event {id:?} under {}", events.display());
        }
        return Ok(vec![dir]);
    }
    let mut dirs: Vec<PathBuf> = fs::read_dir(&events)
        .with_context(|| format!("reading {}", events.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    Ok(dirs)
}

/// Day folders of an event that have a source copy, in day order.
pub fn sealed_days(event_dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut days: Vec<PathBuf> = fs::read_dir(event_dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("day"))
                && p.join(SOURCE_FILE).is_file()
        })
        .collect();
    days.sort();
    Ok(days)
}
