//! The calendar `advent serve` hosts: the same site `advent build` writes
//! for the current moment (without word locks), and per event what the
//! server core needs to check guesses.
//!
//! The event folder does not change while the server runs (it is part of
//! the image); the site is regenerated only when a day or hint is released.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use advent_core::format;
use advent_core::model::Event;
use advent_core::schedule::end_of_date;
use advent_core::wrap::x25519;
use advent_server::EventData;
use anyhow::Context;
use chrono::{DateTime, Days, NaiveDate, Utc};

use crate::commands::build::{self, Moment, Opener, Options};
use crate::commands::reseal;
use crate::day::SEALED_FILE;

pub struct Content {
    /// Folder holding the generated site.
    pub site: PathBuf,
    /// Removes the folder once the last request using it is done.
    _dir: tempfile::TempDir,
    pub events: BTreeMap<String, Arc<EventData>>,
    /// When the last day of the last event ends: sessions last until then.
    pub ends_at: DateTime<Utc>,
}

fn options(
    identities: &[x25519::Identity],
    moment: Moment,
    skip_unsealed: bool,
) -> Options<'static> {
    Options {
        moment,
        base: "/",
        opener: Opener::Release(identities.to_vec()),
        skip_unsealed,
        head_html: None,
        server: true,
    }
}

/// Check everything the server will ever open: every sealed day, its hints
/// and the certificate, with the release key. Run at startup, so a broken
/// image fails at deploy time rather than on the morning a day opens.
pub fn validate(root: &Path, identities: &[x25519::Identity]) -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    build::build_site(
        root,
        &dir.path().join("site"),
        &options(identities, Moment::AllOpen, true),
    )
    .context("the event does not open with ADVENT_RELEASE_KEY")?;
    load_events(root)?;
    Ok(())
}

/// The site as of `now`, and the events' sealed data.
pub fn load(
    root: &Path,
    identities: &[x25519::Identity],
    now: DateTime<Utc>,
) -> anyhow::Result<Content> {
    let dir = tempfile::Builder::new().prefix("advent-serve-").tempdir()?;
    let site = dir.path().join("site");
    build::build_site(root, &site, &options(identities, Moment::At(now), false))?;
    let events = load_events(root)?;
    let ends_at = events
        .values()
        .map(|e| end_of_date(e.event.timezone, e.event.day_date(e.event.days)))
        .max()
        .unwrap_or(now);
    Ok(Content {
        site,
        _dir: dir,
        events,
        ends_at,
    })
}

fn load_events(root: &Path) -> anyhow::Result<BTreeMap<String, Arc<EventData>>> {
    let mut events = BTreeMap::new();
    for event_dir in reseal::event_dirs(root, None)? {
        let id = event_dir
            .file_name()
            .and_then(|n| n.to_str())
            .context("event folder name is not UTF-8")?
            .to_string();
        let event = Event::from_yaml(&fs::read_to_string(event_dir.join("event.yaml"))?)
            .with_context(|| format!("in {}", event_dir.join("event.yaml").display()))?;
        let mut sealed = Vec::new();
        for n in 1..=event.days {
            let path = event_dir.join(format!("day{n:02}")).join(SEALED_FILE);
            if path.is_file() {
                sealed.push(
                    format::read_sealed_day(&fs::read(&path)?)
                        .with_context(|| format!("in {}", path.display()))?,
                );
            }
        }
        let data = EventData::new(id.clone(), event, sealed)
            .with_context(|| format!("in {}", event_dir.display()))?;
        events.insert(id, Arc::new(data));
    }
    Ok(events)
}

/// The next instant after `now` when something new is published: a day,
/// a hint or a notice. Everything is released at local midnight, so it is
/// the next midnight in some event's timezone within its active dates.
pub fn next_moment<'a>(
    events: impl IntoIterator<Item = &'a EventData>,
    now: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    events
        .into_iter()
        .filter_map(|data| {
            let event = &data.event;
            let mut dates: Vec<NaiveDate> = (1..=event.days).map(|d| event.day_date(d)).collect();
            for day in 1..=event.days {
                for after in data.hint_days(day).unwrap_or_default() {
                    dates.push(event.day_date(day) + Days::new(u64::from(*after)));
                }
            }
            dates.extend(event.notices.iter().filter_map(|n| n.date));
            dates
                .into_iter()
                // The start of a date is the end of the previous one, plus a second.
                .filter_map(|date| date.pred_opt())
                .map(|prev| end_of_date(event.timezone, prev) + chrono::Duration::seconds(1))
                .filter(|start| *start > now)
                .min()
        })
        .min()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serve_next_moment_is_the_next_local_midnight_with_news() {
        let event = Event::from_yaml(
            "title: T\ntimezone: Europe/Paris\nstart: 2026-12-01\ndays: 2\nnotices:\n  - text: Later\n    date: 2026-12-10\n",
        )
        .unwrap();
        let data = EventData::new("2026", event, []).unwrap();
        let at = |s: &str| s.parse::<DateTime<Utc>>().unwrap();
        // Midnight in Paris is 23:00 UTC the day before.
        assert_eq!(
            next_moment([&data], at("2026-11-15T12:00:00Z")),
            Some(at("2026-11-30T23:00:00Z"))
        );
        assert_eq!(
            next_moment([&data], at("2026-12-01T10:00:00Z")),
            Some(at("2026-12-01T23:00:00Z"))
        );
        assert_eq!(
            next_moment([&data], at("2026-12-02T10:00:00Z")),
            Some(at("2026-12-09T23:00:00Z")),
            "the dated notice"
        );
        assert_eq!(next_moment([&data], at("2026-12-11T00:00:00Z")), None);
    }
}
