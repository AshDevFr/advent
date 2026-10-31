//! A guess: the attempt limit, the answer check against the day's word
//! locks, and the solve record.

use std::future::Future;

use advent_core::normalize;
use advent_core::primitives::{WordLock, open_word};
use advent_core::schedule::{hints_due, is_released};
use chrono::{DateTime, Utc};

use crate::content::EventData;
use crate::store::{Solved, Store};
use crate::{MaybeSend, MaybeSync, ServerError};

/// Runs the answer check. One check is a full PBKDF2 derivation per lock
/// (150 000 rounds), tens of milliseconds of CPU; the native adapter moves
/// it off the async executor and caps how many run at once, a Worker runs
/// it inline. The check itself is [`check_locks`].
pub trait Checker: MaybeSend + MaybeSync {
    fn check(
        &self,
        answer: &str,
        locks: &[WordLock],
    ) -> impl Future<Output = Option<String>> + MaybeSend;
}

/// Checks on the calling task.
pub struct InlineChecker;

impl Checker for InlineChecker {
    async fn check(&self, answer: &str, locks: &[WordLock]) -> Option<String> {
        check_locks(answer, locks)
    }
}

/// The day's word if `answer` opens any of its locks.
pub fn check_locks(answer: &str, locks: &[WordLock]) -> Option<String> {
    locks.iter().find_map(|lock| open_word(answer, lock).ok())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Right: the day's word.
    Solved { word: String },
    /// The day was already solved; nothing was counted.
    AlreadySolved { word: String },
    /// Wrong, with the guesses left in this window under an attempt limit.
    Wrong { remaining: Option<u32> },
    /// Refused without checking: the attempt limit is reached until
    /// `retry_at`.
    Limited { retry_at: DateTime<Utc> },
}

/// Check a player's guess for a day.
///
/// The cheap refusals come first (unknown or unreleased day, already
/// solved, limit reached) so that only a guess that will be checked costs a
/// derivation. The guess is counted before the check, so parallel requests
/// cannot run checks past the limit while earlier ones are still running.
pub async fn guess<S: Store, C: Checker>(
    store: &S,
    checker: &C,
    content: &EventData,
    email: &str,
    day: u32,
    answer: &str,
    now: DateTime<Utc>,
) -> Result<Outcome, ServerError> {
    let event = &content.event;
    let (Some(locks), Some(hint_days)) = (content.locks(day), content.hint_days(day)) else {
        return Err(ServerError::UnknownDay(day));
    };
    if !is_released(event, day, now) {
        return Err(ServerError::NotReleased(day));
    }

    let mut state = store
        .day_state(email, &content.id, day)
        .await?
        .unwrap_or_default();
    if let Some(solved) = &state.solved {
        return Ok(Outcome::AlreadySolved {
            word: solved.word.clone(),
        });
    }
    if normalize(answer).is_empty() {
        // Nothing to check; not worth an attempt.
        return Ok(Outcome::Wrong {
            remaining: event
                .attempts
                .map(|a| a.max.saturating_sub(state.window_count)),
        });
    }

    if let Some(limit) = event.attempts {
        let window = limit.per.duration();
        let expired = state.window_start.is_none_or(|start| now >= start + window);
        if expired {
            state.window_start = Some(now);
            state.window_count = 0;
        }
        if state.window_count >= limit.max {
            let start = state.window_start.unwrap_or(now);
            return Ok(Outcome::Limited {
                retry_at: start + window,
            });
        }
        state.window_count += 1;
    }
    state.guesses += 1;
    store.put_day_state(email, &content.id, day, &state).await?;

    match checker.check(answer, locks).await {
        Some(word) => {
            state.solved = Some(Solved {
                word: word.clone(),
                at: now,
                hint_out: hints_due(event, day, hint_days, now) > 0,
            });
            store.put_day_state(email, &content.id, day, &state).await?;
            Ok(Outcome::Solved { word })
        }
        None => Ok(Outcome::Wrong {
            remaining: event
                .attempts
                .map(|a| a.max.saturating_sub(state.window_count)),
        }),
    }
}
