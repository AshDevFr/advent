//! `advent unseal-source <day-dir>`: restore a day's plaintext from its
//! source copy with the editors key, so it can be edited on any machine.

use std::fs;
use std::path::Path;

use advent_core::wrap;
use anyhow::{Context, bail};

use crate::config::{editors_identities, find_repo_root};
use crate::day::{self, SOURCE_FILE};

pub fn run(day_dir: &Path, force: bool) -> anyhow::Result<()> {
    let day = day::locate(day_dir)?;
    let root = find_repo_root(&day.path)?;
    let identities = editors_identities(&root)?;
    let source_path = day.path.join(SOURCE_FILE);
    let encrypted = fs::read(&source_path)
        .with_context(|| format!("{} not found: is this day sealed?", source_path.display()))?;
    let source = day::unpack_source(&wrap::decrypt_with(&encrypted, &identities)?)?;

    // Check every file before writing any, so a refusal changes nothing.
    let conflicts: Vec<&String> = source
        .files
        .iter()
        .filter(|(name, data)| {
            fs::read(day.path.join(name)).is_ok_and(|existing| existing != **data)
        })
        .map(|(name, _)| name)
        .collect();
    if !conflicts.is_empty() && !force {
        bail!(
            "these files differ from the sealed source and would be overwritten: {}. \
             Use --force to overwrite them.",
            conflicts
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    for (name, data) in &source.files {
        let target = day.path.join(name);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&target, data).with_context(|| format!("writing {}", target.display()))?;
    }
    println!(
        "Restored {} file(s) for day {} from {SOURCE_FILE}.",
        source.files.len(),
        day.number
    );
    Ok(())
}
