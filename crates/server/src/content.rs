//! What the server needs from a sealed event: its schedule, and per day the
//! word locks and hint timing. Loading the files is the adapter's job.

use std::collections::BTreeMap;

use advent_core::format::SealedDay;
use advent_core::model::{Event, ModelError};
use advent_core::primitives::WordLock;

#[derive(Debug, Clone)]
pub struct EventData {
    /// The event's folder name, as in its URLs (`2026`).
    pub id: String,
    pub event: Event,
    days: BTreeMap<u32, DayData>,
}

#[derive(Debug, Clone)]
struct DayData {
    locks: Vec<WordLock>,
    hint_days: Vec<u32>,
}

impl EventData {
    pub fn new(
        id: impl Into<String>,
        event: Event,
        sealed: impl IntoIterator<Item = SealedDay>,
    ) -> Result<Self, ModelError> {
        let mut days = BTreeMap::new();
        for day in sealed {
            if day.day == 0 || day.day > event.days {
                return Err(ModelError::Invalid {
                    field: "days",
                    reason: format!(
                        "day {} is sealed but the event has {} days",
                        day.day, event.days
                    ),
                });
            }
            let overrides: Vec<Option<u32>> = day.hints.iter().map(|h| h.after).collect();
            let hint_days = event.hint_days(&overrides)?;
            days.insert(
                day.day,
                DayData {
                    locks: day.word_locks,
                    hint_days,
                },
            );
        }
        Ok(Self {
            id: id.into(),
            event,
            days,
        })
    }

    /// The word locks of a sealed day.
    pub fn locks(&self, day: u32) -> Option<&[WordLock]> {
        self.days.get(&day).map(|d| d.locks.as_slice())
    }

    /// When each of a day's hints is due, in days after the door opens.
    pub fn hint_days(&self, day: u32) -> Option<&[u32]> {
        self.days.get(&day).map(|d| d.hint_days.as_slice())
    }
}
