//! `advent schedule [event]`: when each day opens, so authors can check
//! what `every`, `weekdays` or `dates` in `event.yaml` work out to.

use std::fs;
use std::path::Path;

use advent_core::model::Event;
use advent_core::schedule::release_time;

use crate::commands::reseal;
use crate::config::find_repo_root;

pub fn run(event: Option<&str>) -> anyhow::Result<()> {
    let root = find_repo_root(Path::new("."))?;
    for event_dir in reseal::event_dirs(&root, event)? {
        let id = event_dir.file_name().unwrap_or_default().to_string_lossy();
        let event = Event::from_yaml(&fs::read_to_string(event_dir.join("event.yaml"))?)?;
        println!(
            "{id}: {} ({} days, {})",
            event.title,
            event.days,
            event.timezone.name()
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
            println!("  day {n:>2}  {}  {opens}", date.format("%a %Y-%m-%d"));
        }
    }
    Ok(())
}
