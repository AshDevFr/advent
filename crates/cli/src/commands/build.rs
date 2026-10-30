//! `advent build [--until DATE] [--out DIR]`: write the static site for a
//! date. Released days and due hints are published in plain form, word
//! locks and the certificate stay encrypted, and nothing unreleased is
//! written in any form. Only the release key is used: sources are never
//! opened, so no answer can reach the output. The result depends only on
//! the repo and the date, so rerunning a build is harmless.

use std::fs;
use std::path::Path;

use advent_core::format::{self, Bundle};
use advent_core::model::{Brand, Event, Theme};
use advent_core::primitives;
use advent_core::schedule::{end_of_date, first_release, hints_due, is_released};
use advent_core::site::{DayPage, EventInfo, HintsDue, Locks, SiteBrand, SiteIndex, theme_css};
use advent_core::wrap::{self, x25519};
use anyhow::{Context, bail};
use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};
use serde::Serialize;

use crate::commands::{keeper, reseal};
use crate::config::{find_repo_root, load_repo_config, release_identities};
use crate::day::SEALED_FILE;

/// Marks a folder as written by `advent build`, so it can be replaced safely.
const MARKER: &str = ".advent-build";

pub fn run(until: Option<NaiveDate>, out: &Path) -> anyhow::Result<()> {
    let root = find_repo_root(Path::new("."))?;
    load_repo_config(&root)?;
    let release = release_identities()?;
    prepare_out(out)?;

    let brand = read_optional::<Brand>(&root.join("brand/brand.yaml"), Brand::from_yaml)?;
    let logo = match brand.as_ref().and_then(|b| b.logo.as_ref()) {
        Some(file) => {
            copy_asset(
                &root.join("brand").join(file),
                &out.join("brand").join(file),
            )?;
            Some(format!("/brand/{file}"))
        }
        None => None,
    };
    fs::write(out.join("theme.css"), theme_css(brand.as_ref(), None)?)?;

    let mut events = Vec::new();
    for event_dir in reseal::event_dirs(&root, None)? {
        let info = build_event(&event_dir, until, brand.as_ref(), &release, out)
            .with_context(|| format!("building {}", event_dir.display()))?;
        events.push(info);
    }

    let index = SiteIndex {
        brand: SiteBrand {
            name: brand
                .as_ref()
                .map_or("Advent calendar".into(), |b| b.name.clone()),
            logo,
        },
        events,
    };
    write_json(&out.join("site.json"), &index)?;
    println!(
        "Built {} event(s) into {}{}.",
        index.events.len(),
        out.display(),
        until.map_or(String::new(), |d| format!(" as of the end of {d}"))
    );
    Ok(())
}

fn build_event(
    event_dir: &Path,
    until: Option<NaiveDate>,
    brand: Option<&Brand>,
    release: &[x25519::Identity],
    out: &Path,
) -> anyhow::Result<EventInfo> {
    let id = event_dir
        .file_name()
        .and_then(|n| n.to_str())
        .context("event folder name is not UTF-8")?
        .to_string();
    let event = Event::from_yaml(&fs::read_to_string(event_dir.join("event.yaml"))?)?;
    let now: DateTime<Utc> = match until {
        Some(date) => end_of_date(event.timezone, date),
        None => Utc::now(),
    };
    let site = out.join(&id);
    fs::create_dir_all(&site)?;

    let theme = read_optional::<Theme>(&event_dir.join("theme.yaml"), Theme::from_yaml)?;
    fs::write(site.join("theme.css"), theme_css(brand, theme.as_ref())?)?;
    let decoration = |file: Option<&String>| -> anyhow::Result<Option<String>> {
        let Some(file) = file else { return Ok(None) };
        copy_asset(&event_dir.join(file), &site.join("theme").join(file))?;
        Ok(Some(format!("/{id}/theme/{file}")))
    };
    let background = decoration(theme.as_ref().and_then(|t| t.background.as_ref()))?;
    let doors = decoration(theme.as_ref().and_then(|t| t.doors.as_ref()))?;

    let mut released = Vec::new();
    for n in 1..=event.days {
        if !is_released(&event, n, now) {
            continue;
        }
        let day_dir = event_dir.join(format!("day{n:02}"));
        let sealed_path = day_dir.join(SEALED_FILE);
        if !sealed_path.is_file() {
            bail!(
                "day {n} is released but not sealed ({} missing)",
                sealed_path.display()
            );
        }
        publish_day(
            n,
            &fs::read(&sealed_path)?,
            hints_due(&event, n, now),
            release,
            &site,
        )
        .with_context(|| format!("day {n}"))?;
        released.push(n);
    }

    let final_sealed = event_dir.join(keeper::FINAL_DIR).join(SEALED_FILE);
    if final_sealed.is_file() {
        let sealed = format::read_sealed_final(&fs::read(&final_sealed)?)?;
        write_json(&site.join("api/final.json"), &sealed)?;
    }

    let info = EventInfo {
        id,
        title: event.title.clone(),
        timezone: event.timezone.name().to_string(),
        start: event.start.to_string(),
        days: event.days,
        intro: event.story.intro.clone(),
        opens_at: first_release(&event).map(|t| t.to_rfc3339_opts(SecondsFormat::Secs, true)),
        released,
        background,
        doors,
    };
    write_json(&site.join("event.json"), &info)?;
    Ok(info)
}

fn publish_day(
    n: u32,
    sealed_json: &[u8],
    hints_due: u8,
    release: &[x25519::Identity],
    site: &Path,
) -> anyhow::Result<()> {
    let sealed = format::read_sealed_day(sealed_json)?;
    let day_key = wrap::unwrap_key(&sealed.day_key, release)
        .context("the day key does not open with ADVENT_RELEASE_KEY")?;
    let bundle: Bundle = serde_json::from_slice(&primitives::open_bundle(
        &day_key,
        sealed.bundle.iv,
        &sealed.bundle.ct,
    )?)?;

    let api = site.join("api/day");
    let assets = api.join(n.to_string());
    fs::create_dir_all(&assets)?;
    write_json(
        &api.join(format!("{n}.json")),
        &DayPage {
            day: n,
            title: bundle.title,
            kind: bundle.kind,
            markdown: bundle.markdown,
        },
    )?;
    for (name, data) in &bundle.assets {
        if name.split('/').any(|p| p == ".." || p.is_empty()) {
            bail!("unsafe asset name {name:?}");
        }
        let target = assets.join(name);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(target, data)?;
    }

    let mut hints = Vec::new();
    for hint in sealed.hints.iter().take(usize::from(hints_due)) {
        let key = wrap::unwrap_key(&hint.key, release)?;
        hints.push(primitives::open_hint(&key, hint.iv, &hint.ct)?);
    }
    write_json(&assets.join("hint.json"), &HintsDue { hints })?;
    write_json(
        &assets.join("locks.json"),
        &Locks {
            word_locks: sealed.word_locks,
        },
    )
}

/// Start from an empty folder. A non-empty one is only cleared when an
/// earlier build wrote it, so a wrong `--out` cannot wipe anything else.
fn prepare_out(out: &Path) -> anyhow::Result<()> {
    if out.exists() && fs::read_dir(out)?.next().is_some() {
        if !out.join(MARKER).is_file() {
            bail!(
                "{} is not empty and was not written by `advent build`; choose another --out",
                out.display()
            );
        }
        fs::remove_dir_all(out)?;
    }
    fs::create_dir_all(out)?;
    fs::write(out.join(MARKER), "")?;
    Ok(())
}

fn read_optional<T>(
    path: &Path,
    parse: fn(&str) -> Result<T, advent_core::model::ModelError>,
) -> anyhow::Result<Option<T>> {
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(path)?;
    Ok(Some(
        parse(&text).with_context(|| format!("in {}", path.display()))?,
    ))
}

fn copy_asset(from: &Path, to: &Path) -> anyhow::Result<()> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(from, to).with_context(|| format!("copying {}", from.display()))?;
    Ok(())
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    fs::write(path, bytes).with_context(|| format!("writing {}", path.display()))
}
