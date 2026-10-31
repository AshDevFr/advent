//! A player's progress, and the organisers' view of an event.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::content::EventData;
use crate::store::Store;
use crate::{ServerError, store::DayState};

/// What the calendar needs about one player on one event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Progress {
    /// The words collected, by day.
    pub words: BTreeMap<u32, String>,
    /// Guesses made, by day (solved or not).
    pub guesses: BTreeMap<u32, u32>,
    pub name: Option<String>,
    /// When the last day was solved, once every day is.
    pub finished_at: Option<DateTime<Utc>>,
}

pub async fn progress<S: Store>(
    store: &S,
    content: &EventData,
    email: &str,
) -> Result<Progress, ServerError> {
    let player = store
        .player(email)
        .await?
        .ok_or(ServerError::UnknownPlayer)?;
    let days = store.player_days(email, &content.id).await?;
    let mut words = BTreeMap::new();
    let mut guesses = BTreeMap::new();
    for (day, state) in &days {
        guesses.insert(*day, state.guesses);
        if let Some(solved) = &state.solved {
            words.insert(*day, solved.word.clone());
        }
    }
    Ok(Progress {
        words,
        guesses,
        name: player.name,
        finished_at: finished_at(content, days.iter().map(|(d, s)| (*d, s))),
    })
}

/// When the last of every day was solved, if all are.
fn finished_at<'a>(
    content: &EventData,
    days: impl Iterator<Item = (u32, &'a DayState)>,
) -> Option<DateTime<Utc>> {
    let solved: BTreeMap<u32, DateTime<Utc>> = days
        .filter_map(|(day, state)| state.solved.as_ref().map(|s| (day, s.at)))
        .collect();
    let all = (1..=content.event.days).all(|day| solved.contains_key(&day));
    all.then(|| solved.values().max().copied()).flatten()
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DayStats {
    pub day: u32,
    pub solved: u32,
    /// Share of the players who signed in at least once (0 to 1). Players
    /// invited but never seen would otherwise drag every day down.
    pub completion: f64,
    /// Median guesses of the players who solved it.
    pub median_guesses: Option<f64>,
    /// How many solved it while a hint was out.
    pub hint_out: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EventStats {
    pub players: u32,
    pub invited: u32,
    pub signed_in: u32,
    pub finished: u32,
    pub days: Vec<DayStats>,
}

/// One finisher, for the export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finisher {
    pub email: String,
    pub name: Option<String>,
    pub finished_at: DateTime<Utc>,
}

pub async fn stats<S: Store>(store: &S, content: &EventData) -> Result<EventStats, ServerError> {
    let players = store.players().await?;
    let known: BTreeSet<&str> = players.iter().map(|p| p.email.as_str()).collect();
    let signed_in = players.iter().filter(|p| p.last_seen.is_some()).count() as u32;
    let rows = store.event_days(&content.id).await?;

    let mut by_day: BTreeMap<u32, Vec<&DayState>> = BTreeMap::new();
    let mut by_player: BTreeMap<&str, Vec<(u32, &DayState)>> = BTreeMap::new();
    for (email, day, state) in &rows {
        if !known.contains(email.as_str()) {
            continue;
        }
        by_day.entry(*day).or_default().push(state);
        by_player
            .entry(email.as_str())
            .or_default()
            .push((*day, state));
    }

    let days = (1..=content.event.days)
        .map(|day| {
            let solved: Vec<&crate::store::Solved> = by_day
                .get(&day)
                .into_iter()
                .flatten()
                .filter_map(|s| s.solved.as_ref())
                .collect();
            let mut guesses: Vec<u32> = by_day
                .get(&day)
                .into_iter()
                .flatten()
                .filter(|s| s.solved.is_some())
                .map(|s| s.guesses)
                .collect();
            guesses.sort_unstable();
            let count = solved.len() as u32;
            DayStats {
                day,
                solved: count,
                completion: if signed_in == 0 {
                    0.0
                } else {
                    f64::from(count) / f64::from(signed_in)
                },
                median_guesses: median(&guesses),
                hint_out: solved.iter().filter(|s| s.hint_out).count() as u32,
            }
        })
        .collect();

    let finished = by_player
        .values()
        .filter(|days| finished_at(content, days.iter().copied()).is_some())
        .count() as u32;

    Ok(EventStats {
        players: players.len() as u32,
        invited: players.iter().filter(|p| p.invited_at.is_some()).count() as u32,
        signed_in,
        finished,
        days,
    })
}

/// Players who solved every day, earliest first.
pub async fn finishers<S: Store>(
    store: &S,
    content: &EventData,
) -> Result<Vec<Finisher>, ServerError> {
    let players = store.players().await?;
    let rows = store.event_days(&content.id).await?;
    let mut out: Vec<Finisher> = players
        .into_iter()
        .filter_map(|player| {
            let days = rows
                .iter()
                .filter(|(email, _, _)| *email == player.email)
                .map(|(_, day, state)| (*day, state));
            finished_at(content, days).map(|finished_at| Finisher {
                email: player.display_email,
                name: player.name,
                finished_at,
            })
        })
        .collect();
    out.sort_by_key(|f| f.finished_at);
    Ok(out)
}

fn median(sorted: &[u32]) -> Option<f64> {
    match sorted.len() {
        0 => None,
        n if n % 2 == 1 => Some(f64::from(sorted[n / 2])),
        n => Some((f64::from(sorted[n / 2 - 1]) + f64::from(sorted[n / 2])) / 2.0),
    }
}
