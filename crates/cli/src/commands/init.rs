//! `advent init`: write a new event repo from the embedded template and give
//! it fresh key pairs.

use std::fs;
use std::path::Path;

use advent_core::wrap;
use anyhow::{Context, bail};
use chrono::Datelike;
use include_dir::{Dir, DirEntry, include_dir};

use crate::config::EDITORS_KEY_FILE;

static TEMPLATE: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../templates/event");

/// Folder name standing for the event id inside the template.
const EVENT_PLACEHOLDER: &str = "EVENT";

pub fn run(dir: &Path, event: Option<&str>) -> anyhow::Result<()> {
    if dir.exists() && fs::read_dir(dir)?.next().is_some() {
        bail!(
            "{} is not empty; `advent init` only writes into an empty directory",
            dir.display()
        );
    }

    let year = chrono::Local::now().year();
    let event = event.map(str::to_owned).unwrap_or_else(|| year.to_string());
    if event.is_empty() || event.contains(['/', '\\']) || event.starts_with('.') {
        bail!("invalid event id {event:?}: it becomes a folder name under events/");
    }
    // An event named after a year starts on December 1st of that year.
    let start_year = match event.parse::<i32>() {
        Ok(y) if (1000..=9999).contains(&y) => y,
        _ => year,
    };

    let (editors_secret, editors_public) = wrap::generate_identity();
    let (release_secret, release_public) = wrap::generate_identity();

    let vars = [
        ("{{engine}}", env!("CARGO_PKG_VERSION").to_string()),
        ("{{editors}}", editors_public),
        ("{{release}}", release_public),
        ("{{event}}", event.clone()),
        ("{{start}}", format!("{start_year}-12-01")),
    ];

    fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    write_dir(&TEMPLATE, dir, &event, &vars)?;

    let key_path = dir.join(EDITORS_KEY_FILE);
    fs::write(
        &key_path,
        format!(
            "# advent editors key. Never commit it; share it with authors through a\n\
             # password manager or Vault. Losing it means sources can no longer be edited.\n\
             {editors_secret}\n"
        ),
    )?;
    restrict_permissions(&key_path)?;

    println!(
        "Created event repo in {} with event {event}.",
        dir.display()
    );
    println!();
    println!("Release key (shown once, not saved anywhere). Store it as the CI secret");
    println!("ADVENT_RELEASE_KEY now; without it no day can ever be released:");
    println!();
    println!("{release_secret}");
    println!();
    println!("The editors key was written to {EDITORS_KEY_FILE} (gitignored).");
    Ok(())
}

fn write_dir(
    dir: &Dir<'_>,
    root: &Path,
    event: &str,
    vars: &[(&str, String)],
) -> anyhow::Result<()> {
    for entry in dir.entries() {
        let relative = entry.path().to_string_lossy().replacen(
            &format!("events/{EVENT_PLACEHOLDER}"),
            &format!("events/{event}"),
            1,
        );
        let target = root.join(relative);
        match entry {
            DirEntry::Dir(sub) => {
                fs::create_dir_all(&target)?;
                write_dir(sub, root, event, vars)?;
            }
            DirEntry::File(file) => {
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)?;
                }
                let is_template_text = target.extension().is_some_and(|e| e == "yaml");
                match file.contents_utf8().filter(|_| is_template_text) {
                    Some(text) => {
                        let rendered = vars
                            .iter()
                            .fold(text.to_string(), |acc, (k, v)| acc.replace(k, v));
                        fs::write(&target, rendered)?;
                    }
                    None => fs::write(&target, file.contents())?,
                }
            }
        }
    }
    Ok(())
}

#[cfg(unix)]
fn restrict_permissions(path: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) -> anyhow::Result<()> {
    Ok(())
}
