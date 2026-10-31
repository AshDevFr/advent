//! When a day and its hints are released. The static build is idempotent
//! per date, so everything here is a pure function of the event and `now`.

use chrono::{DateTime, Days, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;

use crate::model::{Event, Notice};

/// Source of the current time. Core never reads the system clock (it does
/// not exist on wasm32-unknown-unknown); callers provide one.
pub trait Clock {
    fn now(&self) -> DateTime<Utc>;
}

/// A clock stuck at one instant, for tests and for `advent build --until`.
pub struct FixedClock(pub DateTime<Utc>);

impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        self.0
    }
}

/// The first instant of `date` in `tz`. Where a DST change skips midnight,
/// the day starts at the first local time that exists.
fn start_of_day(tz: Tz, date: NaiveDate) -> DateTime<Utc> {
    let mut time = date.and_time(NaiveTime::MIN);
    loop {
        if let Some(local) = tz.from_local_datetime(&time).earliest() {
            return local.with_timezone(&Utc);
        }
        time += chrono::Duration::minutes(15);
    }
}

/// The last second of `date` in `tz`, for building "as of" a date.
pub fn end_of_date(tz: Tz, date: NaiveDate) -> DateTime<Utc> {
    start_of_day(tz, date + Days::new(1)) - chrono::Duration::seconds(1)
}

/// When day 1 opens, or `None` if it is unlocked regardless of the date.
pub fn first_release(event: &Event) -> Option<DateTime<Utc>> {
    release_time(event, 1)
}

fn day_date(event: &Event, day: u32) -> NaiveDate {
    event.day_date(day)
}

/// When `day` is released, or `None` for a day outside the event or one
/// that is unlocked regardless of the date.
pub fn release_time(event: &Event, day: u32) -> Option<DateTime<Utc>> {
    if day == 0 || day > event.days || day <= event.unlocked {
        return None;
    }
    Some(start_of_day(event.timezone, day_date(event, day)))
}

/// Whether `day` is released at `now`.
pub fn is_released(event: &Event, day: u32, now: DateTime<Utc>) -> bool {
    if day == 0 || day > event.days {
        return false;
    }
    day <= event.unlocked || now >= start_of_day(event.timezone, day_date(event, day))
}

/// Whether a notice is shown at `now`: with `day`, once that day is
/// released; with `date`, from local midnight of that date; else always.
pub fn notice_published(event: &Event, notice: &Notice, now: DateTime<Utc>) -> bool {
    match (notice.day, notice.date) {
        (Some(day), _) => is_released(event, day, now),
        (None, Some(date)) => now >= start_of_day(event.timezone, date),
        (None, None) => true,
    }
}

/// How many of a day's hints are due at `now`, given each hint's delay in
/// days after the door opens (`Event::hint_days`); a hint is due from local
/// midnight on that date. Unlocked days have every hint due. The delays do
/// not decrease, so the due hints are always the first ones.
pub fn hints_due(event: &Event, day: u32, after: &[u32], now: DateTime<Utc>) -> usize {
    if !is_released(event, day, now) {
        return 0;
    }
    if day <= event.unlocked {
        return after.len();
    }
    let date = day_date(event, day);
    after
        .iter()
        .take_while(|days| now >= start_of_day(event.timezone, date + Days::new(u64::from(**days))))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(start: &str, unlocked: u32) -> Event {
        Event {
            title: "t".into(),
            timezone: chrono_tz::Europe::Paris,
            start: start.parse().unwrap(),
            days: 24,
            unlocked,
            every: 1,
            weekdays: false,
            dates: None,
            hint_after: vec![1, 2],
            notices: Vec::new(),
            story: Default::default(),
        }
    }

    fn at(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    #[test]
    fn model_schedule_day_releases_at_local_midnight() {
        let e = event("2026-12-01", 0);
        assert!(!is_released(&e, 5, at("2026-12-04T22:59:59Z")));
        assert!(is_released(&e, 5, at("2026-12-04T23:00:00Z")));
    }

    #[test]
    fn model_schedule_hints_follow_the_next_two_midnights() {
        let e = event("2026-12-01", 0);
        assert_eq!(hints_due(&e, 5, &[1, 2], at("2026-12-04T22:00:00Z")), 0);
        assert_eq!(hints_due(&e, 5, &[1, 2], at("2026-12-05T12:00:00Z")), 0);
        assert_eq!(hints_due(&e, 5, &[1, 2], at("2026-12-05T23:00:00Z")), 1);
        assert_eq!(hints_due(&e, 5, &[1, 2], at("2026-12-06T23:00:00Z")), 2);
        assert_eq!(hints_due(&e, 5, &[1, 2], at("2027-01-15T00:00:00Z")), 2);
    }

    #[test]
    fn model_schedule_hints_across_dst_change() {
        // Paris leaves summer time on 2026-10-25: midnight moves from 22:00Z to 23:00Z.
        let e = event("2026-10-24", 0);
        assert_eq!(hints_due(&e, 1, &[1, 2], at("2026-10-24T21:59:59Z")), 0);
        assert_eq!(hints_due(&e, 1, &[1, 2], at("2026-10-24T22:00:00Z")), 1);
        assert_eq!(hints_due(&e, 1, &[1, 2], at("2026-10-25T22:59:59Z")), 1);
        assert_eq!(hints_due(&e, 1, &[1, 2], at("2026-10-25T23:00:00Z")), 2);
    }

    #[test]
    fn model_schedule_unlocked_days_ignore_the_date() {
        let mut e = event("2026-12-01", 7);
        e.days = 8;
        let march = at("2026-03-01T12:00:00Z");
        for day in 1..=7 {
            assert!(is_released(&e, day, march), "day {day}");
            assert_eq!(hints_due(&e, day, &[1, 2], march), 2);
        }
        assert!(!is_released(&e, 8, march));
        assert_eq!(release_time(&e, 7), None);
    }

    #[test]
    fn model_schedule_days_outside_the_event_never_release() {
        let e = event("2026-12-01", 0);
        let later = at("2030-01-01T00:00:00Z");
        assert!(!is_released(&e, 0, later));
        assert!(!is_released(&e, 25, later));
    }

    #[test]
    fn model_schedule_skipped_midnight_starts_at_first_valid_time() {
        // Santiago springs forward at midnight: 2026-09-06 00:00 does not exist locally.
        let date: NaiveDate = "2026-09-06".parse().unwrap();
        let start = start_of_day(chrono_tz::America::Santiago, date);
        assert_eq!(start, at("2026-09-06T04:00:00Z"));
    }

    #[test]
    fn model_schedule_end_of_date_is_the_last_local_second() {
        let date: NaiveDate = "2026-12-03".parse().unwrap();
        assert_eq!(
            end_of_date(chrono_tz::Europe::Paris, date),
            at("2026-12-03T22:59:59Z")
        );
    }

    #[test]
    fn model_schedule_spread_days_release_on_their_own_date() {
        let mut e = event("2026-12-01", 0);
        e.every = 2;
        assert!(!is_released(&e, 2, at("2026-12-02T12:00:00Z")));
        assert!(
            is_released(&e, 2, at("2026-12-02T23:00:00Z")),
            "Dec 3, 00:00 Paris"
        );
        assert_eq!(hints_due(&e, 2, &[1, 2], at("2026-12-03T12:00:00Z")), 0);
        assert_eq!(
            hints_due(&e, 2, &[1, 2], at("2026-12-03T23:00:00Z")),
            1,
            "Dec 4, 00:00 Paris"
        );
    }

    #[test]
    fn model_schedule_hints_follow_their_own_delay() {
        let e = event("2026-12-01", 0);
        // Day 5 opens Dec 5 (Paris); hints after 0 and 3 days.
        assert_eq!(hints_due(&e, 5, &[0, 3], at("2026-12-04T22:59:59Z")), 0);
        assert_eq!(hints_due(&e, 5, &[0, 3], at("2026-12-04T23:00:00Z")), 1);
        assert_eq!(hints_due(&e, 5, &[0, 3], at("2026-12-07T22:59:59Z")), 1);
        assert_eq!(hints_due(&e, 5, &[0, 3], at("2026-12-07T23:00:00Z")), 2);
    }

    #[test]
    fn model_schedule_notices_follow_their_day_or_date() {
        let e = event("2026-12-01", 0);
        let notice = |day: Option<u32>, date: Option<&str>| Notice {
            text: "n".into(),
            day,
            date: date.map(|d| d.parse().unwrap()),
        };
        // End of Dec 3 in Paris.
        let now = at("2026-12-03T22:59:59Z");
        assert!(notice_published(&e, &notice(None, None), now));
        assert!(notice_published(&e, &notice(Some(3), None), now));
        assert!(notice_published(&e, &notice(None, Some("2026-12-02")), now));
        assert!(!notice_published(&e, &notice(Some(4), None), now));
        assert!(!notice_published(
            &e,
            &notice(None, Some("2026-12-10")),
            now
        ));
    }

    #[test]
    fn model_schedule_fixed_clock_reports_its_instant() {
        let now = at("2026-12-05T08:00:00Z");
        assert_eq!(FixedClock(now).now(), now);
    }
}
