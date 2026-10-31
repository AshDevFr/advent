//! `advent build [--until DATE] [--out DIR]`: write the static site for a
//! date. Released days and due hints are published in plain form, word
//! locks and the certificate stay encrypted, and nothing unreleased is
//! written in any form. Only the release key is used: sources are never
//! opened, so no answer can reach the output. The result depends only on
//! the repo and the date, so rerunning a build is harmless.

use std::fs;
use std::path::Path;

use advent_core::format::{self, Bundle, SealedDay};
use advent_core::model::{Brand, Event, Theme};
use advent_core::primitives;
use advent_core::schedule::{end_of_date, first_release, hints_due, is_released, notice_published};
use advent_core::site::{
    DayPage, EventInfo, HintsDue, Locks, SiteBrand, SiteIndex, SiteNotice, event_scope_css,
    theme_css,
};
use advent_core::wrap::{self, x25519};
use anyhow::{Context, bail};
use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};
use serde::Serialize;

use crate::commands::{keeper, reseal};
use crate::config::{find_repo_root, load_repo_config, release_identities};
use crate::day::{self, DayKeys, SEALED_FILE, SOURCE_FILE};

/// Marks a folder as written by `advent build`, so it can be replaced safely.
const MARKER: &str = ".advent-build";

/// The moment a build shows.
pub enum Moment {
    Now,
    /// The end of this date in each event's timezone.
    Until(NaiveDate),
    /// Every day and hint open (preview only).
    AllOpen,
    /// This instant (the server's clock).
    At(DateTime<Utc>),
}

/// How a build opens the days it publishes.
pub enum Opener {
    /// Unwrap day and hint keys with the release key: what CI does.
    Release(Vec<x25519::Identity>),
    /// Read the keys from each day's source copy with the editors key, so
    /// authors can preview without the release key. Never used to deploy.
    Editors(Vec<x25519::Identity>),
}

pub struct Options<'a> {
    pub moment: Moment,
    pub base: &'a str,
    pub opener: Opener,
    /// Leave out released days that are not sealed yet instead of failing
    /// (a preview of a calendar still being written).
    pub skip_unsealed: bool,
    /// Extra HTML inserted before `</head>` in every page (the preview's
    /// script, the server's mode marker).
    pub head_html: Option<String>,
    /// For `advent serve`: leave out the word locks (the server checks
    /// guesses, and locks in the browser would allow unlimited offline
    /// guessing) and mark `site.json` so the frontend talks to the server.
    pub server: bool,
}

pub fn check_base(base: &str) -> anyhow::Result<()> {
    if !(base.starts_with('/') && base.ends_with('/')) || base.contains(['"', '<', '>', ' ']) {
        bail!(
            "--base must be a path that starts and ends with a slash, such as / or /demo/ (got {base:?})"
        );
    }
    Ok(())
}

pub fn run(until: Option<NaiveDate>, out: &Path, base: &str) -> anyhow::Result<()> {
    check_base(base)?;
    let root = find_repo_root(Path::new("."))?;
    load_repo_config(&root)?;
    let options = Options {
        moment: until.map_or(Moment::Now, Moment::Until),
        base,
        opener: Opener::Release(release_identities()?),
        skip_unsealed: false,
        head_html: None,
        server: false,
    };
    let index = build_site(&root, out, &options)?;
    println!(
        "Built {} event(s) into {}{}.",
        index.events.len(),
        out.display(),
        until.map_or(String::new(), |d| format!(" as of the end of {d}"))
    );
    Ok(())
}

/// Write the whole site into `out` and return its index.
pub fn build_site(root: &Path, out: &Path, options: &Options) -> anyhow::Result<SiteIndex> {
    let base = options.base;
    let event_dirs = reseal::event_dirs(root, None)?;
    let ids: Vec<String> = event_dirs
        .iter()
        .filter_map(|d| d.file_name()?.to_str().map(str::to_owned))
        .collect();
    prepare_out(out)?;
    crate::web::write(out, &ids, base)?;
    if let Some(html) = &options.head_html {
        for page in std::iter::once(out.join("index.html"))
            .chain(ids.iter().map(|id| out.join(id).join("index.html")))
        {
            let text = fs::read_to_string(&page)?;
            fs::write(
                &page,
                text.replacen("</head>", &format!("{html}</head>"), 1),
            )?;
        }
    }

    let brand = read_optional::<Brand>(&root.join("brand/brand.yaml"), Brand::from_yaml)?;
    let brand_asset = |file: Option<&str>| -> anyhow::Result<Option<String>> {
        let Some(file) = file else { return Ok(None) };
        copy_asset(
            &root.join("brand").join(file),
            &out.join("brand").join(file),
        )?;
        Ok(Some(format!("brand/{file}")))
    };
    let logo_image = brand.as_ref().and_then(|b| b.logo.as_ref());
    let logo = brand_asset(logo_image.map(|l| l.light()))?;
    let logo_dark = brand_asset(logo_image.and_then(|l| l.dark()))?;
    let mut site_css = theme_css(brand.as_ref(), None)?;

    let mut events = Vec::new();
    for event_dir in event_dirs {
        let info = build_event(&event_dir, options, brand.as_ref(), out)
            .with_context(|| format!("building {}", event_dir.display()))?;
        // The landing page shows every event; a themed one gets its colours
        // on its own card.
        let theme = read_optional::<Theme>(&event_dir.join("theme.yaml"), Theme::from_yaml)?;
        if theme.is_some() {
            site_css.push_str(&event_scope_css(&info.id, brand.as_ref(), theme.as_ref())?);
        }
        events.push(info);
    }
    fs::write(out.join("theme.css"), site_css)?;

    let index = SiteIndex {
        brand: SiteBrand {
            name: brand
                .as_ref()
                .map_or("Advent calendar".into(), |b| b.name.clone()),
            logo,
            logo_dark,
        },
        events,
        mode: options.server.then(|| "server".to_string()),
    };
    write_json(&out.join("site.json"), &index)?;
    Ok(index)
}

fn build_event(
    event_dir: &Path,
    options: &Options,
    brand: Option<&Brand>,
    out: &Path,
) -> anyhow::Result<EventInfo> {
    let id = event_dir
        .file_name()
        .and_then(|n| n.to_str())
        .context("event folder name is not UTF-8")?
        .to_string();
    let event = Event::from_yaml(&fs::read_to_string(event_dir.join("event.yaml"))?)?;
    let now: DateTime<Utc> = match options.moment {
        Moment::Until(date) => end_of_date(event.timezone, date),
        Moment::Now => Utc::now(),
        // Far enough that every day and every hint is out.
        Moment::AllOpen => DateTime::<Utc>::MAX_UTC - chrono::Duration::days(1000),
        Moment::At(now) => now,
    };
    let site = out.join(&id);
    fs::create_dir_all(&site)?;

    let theme = read_optional::<Theme>(&event_dir.join("theme.yaml"), Theme::from_yaml)?;
    fs::write(site.join("theme.css"), theme_css(brand, theme.as_ref())?)?;
    let decoration = |file: Option<&str>| -> anyhow::Result<Option<String>> {
        let Some(file) = file else { return Ok(None) };
        copy_asset(&event_dir.join(file), &site.join("theme").join(file))?;
        Ok(Some(format!("{id}/theme/{file}")))
    };
    let background_image = theme.as_ref().and_then(|t| t.background.as_ref());
    let background = decoration(background_image.map(|b| b.light()))?;
    let background_dark = decoration(background_image.and_then(|b| b.dark()))?;
    let mut doors = Vec::new();
    for file in theme.as_ref().map(Theme::door_files).unwrap_or_default() {
        doors.extend(decoration(Some(file))?);
    }

    let mut released = Vec::new();
    let mut types = std::collections::BTreeMap::new();
    for n in 1..=event.days {
        if !is_released(&event, n, now) {
            continue;
        }
        let day_dir = event_dir.join(format!("day{n:02}"));
        let sealed_path = day_dir.join(SEALED_FILE);
        if !sealed_path.is_file() {
            if options.skip_unsealed {
                eprintln!("note: day {n} is not sealed yet; left out");
                continue;
            }
            bail!(
                "day {n} is released but not sealed ({} missing)",
                sealed_path.display()
            );
        }
        let kind = publish_day(
            &event,
            n,
            &fs::read(&sealed_path)?,
            &day_dir,
            now,
            options,
            &site,
        )
        .with_context(|| format!("day {n}"))?;
        released.push(n);
        types.insert(n, kind);
    }

    let final_sealed = event_dir.join(keeper::FINAL_DIR).join(SEALED_FILE);
    if final_sealed.is_file() {
        let sealed = format::read_sealed_final(&fs::read(&final_sealed)?)?;
        write_json(&site.join("api/final.json"), &sealed)?;
        // Reward files are encrypted with keys inside the sealed
        // certificate, so they are safe to publish from the start.
        for entry in fs::read_dir(event_dir.join(keeper::FINAL_DIR))? {
            let path = entry?.path();
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            if let Some(index) = name
                .strip_prefix("reward-")
                .and_then(|n| n.strip_suffix(".bin"))
            {
                copy_asset(
                    &path,
                    &site.join("api/rewards").join(format!("{index}.bin")),
                )?;
            }
        }
    }

    let info = EventInfo {
        id,
        title: event.title.clone(),
        timezone: event.timezone.name().to_string(),
        start: event.start.to_string(),
        days: event.days,
        headline: event.story.title.clone(),
        intro: event.story.intro.clone(),
        dates: (1..=event.days)
            .map(|n| event.day_date(n).to_string())
            .collect(),
        // Once a day is out there is nothing to count down to. Saying so here
        // rather than leaving it to the visitor's clock keeps a preview built
        // as of a later date (or with every day open) from showing a
        // countdown in November.
        opens_at: if released.is_empty() {
            first_release(&event).map(|t| t.to_rfc3339_opts(SecondsFormat::Secs, true))
        } else {
            None
        },
        released,
        types,
        notices: event
            .notices
            .iter()
            .filter(|notice| notice_published(&event, notice, now))
            .map(|notice| SiteNotice::new(&notice.text))
            .collect(),
        background,
        background_dark,
        doors,
    };
    write_json(&site.join("event.json"), &info)?;
    Ok(info)
}

/// The day key and hint keys of a sealed day, by whichever key the build has.
fn day_keys(sealed: &SealedDay, day_dir: &Path, opener: &Opener) -> anyhow::Result<DayKeys> {
    match opener {
        Opener::Release(ids) => Ok(DayKeys {
            day_key: wrap::unwrap_key(&sealed.day_key, ids)
                .context("the day key does not open with ADVENT_RELEASE_KEY")?,
            hint_keys: sealed
                .hints
                .iter()
                .map(|h| wrap::unwrap_key(&h.key, ids))
                .collect::<Result<_, _>>()?,
        }),
        Opener::Editors(ids) => {
            let encrypted = fs::read(day_dir.join(SOURCE_FILE))?;
            let source = day::unpack_source(
                &wrap::decrypt_with(&encrypted, ids)
                    .context("source.age does not open with the editors key")?,
            )?;
            source.keys.context("source copy has no day keys")
        }
    }
}

fn publish_day(
    event: &Event,
    n: u32,
    sealed_json: &[u8],
    day_dir: &Path,
    now: DateTime<Utc>,
    options: &Options,
    site: &Path,
) -> anyhow::Result<String> {
    let sealed = format::read_sealed_day(sealed_json)?;
    let overrides: Vec<Option<u32>> = sealed.hints.iter().map(|h| h.after).collect();
    let due = hints_due(event, n, &event.hint_days(&overrides)?, now);
    let keys = day_keys(&sealed, day_dir, &options.opener)?;
    let day_key = keys.day_key;
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
            kind: bundle.kind.clone(),
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
    for (hint, key) in sealed.hints.iter().zip(&keys.hint_keys).take(due) {
        hints.push(primitives::open_hint(key, hint.iv, &hint.ct)?);
    }
    write_json(&assets.join("hint.json"), &HintsDue { hints })?;
    if !options.server {
        write_json(
            &assets.join("locks.json"),
            &Locks {
                word_locks: sealed.word_locks,
            },
        )?;
    }
    Ok(bundle.kind)
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
