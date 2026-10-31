//! `advent schedule [event]`: when each day opens and when its hints
//! appear, so authors can check what `every`, `weekdays`, `dates`,
//! `hint_after` and per-hint `after` work out to.

use std::fs;
use std::path::Path;

use advent_core::format;
use advent_core::model::Event;
use advent_core::schedule::release_time;
use chrono::Days;

use crate::commands::reseal;
use crate::config::find_repo_root;
use crate::day::SEALED_FILE;

pub fn run(event: Option<&str>) -> anyhow::Result<()> {
    let root = find_repo_root(Path::new("."))?;
    for event_dir in reseal::event_dirs(&root, event)? {
        let id = event_dir.file_name().unwrap_or_default().to_string_lossy();
        let event = Event::from_yaml(&fs::read_to_string(event_dir.join("event.yaml"))?)?;
        println!(
            "{id}: {} ({} days, {}; hints {:?} days after a door opens)",
            event.title,
            event.days,
            event.timezone.name(),
            event.hint_after
        );
        for n in 1..=event.days {
            let date = event.day_date(n);
            let opens = match release_time(&event, n) {
                Some(at) => at
                    .with_timezone(&event.timezone)
                    .format("%H:%M %Z")
                    .to_string(),
                None => "unlocked".into(),
            };
            // A sealed day knows how many hints it has and any per-hint
            // timing; an unsealed one shows the event default.
            let sealed = fs::read(event_dir.join(format!("day{n:02}")).join(SEALED_FILE))
                .ok()
                .and_then(|bytes| format::read_sealed_day(&bytes).ok());
            let overrides: Vec<Option<u32>> = match &sealed {
                Some(day) => day.hints.iter().map(|h| h.after).collect(),
                None => vec![None; event.hint_after.len()],
            };
            let hints = event
                .hint_days(&overrides)?
                .iter()
                .map(|after| {
                    (date + Days::new(u64::from(*after)))
                        .format("%b %-d")
                        .to_string()
                })
                .collect::<Vec<_>>()
                .join(", ");
            println!(
                "  day {n:>2}  {}  {opens}  hints {hints}{}",
                date.format("%a %Y-%m-%d"),
                if sealed.is_some() { "" } else { " (default)" }
            );
        }
    }
    Ok(())
}
