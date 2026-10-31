//! The server core over the in-memory store, with a fixed `now`.

use std::sync::atomic::{AtomicU64, Ordering};

use advent_core::format::{Encrypted, SealedDay, SealedHint};
use advent_core::model::Event;
use advent_core::primitives::{WordLock, seal_word};
use advent_server::memory::MemoryStore;
use advent_server::players::{self, Invite};
use advent_server::stats::{finishers, progress, stats};
use advent_server::{
    Checker, Entropy, EventData, InlineChecker, Outcome, ServerError, Store, guess,
};
use chrono::{DateTime, Duration, Utc};
use pollster::block_on;

/// Distinct, predictable bytes for each token.
struct Counter(AtomicU64);

impl Entropy for Counter {
    fn fill(&self, buf: &mut [u8]) {
        let n = self.0.fetch_add(1, Ordering::Relaxed).to_le_bytes();
        for (i, b) in buf.iter_mut().enumerate() {
            *b = n[i % 8];
        }
    }
}

fn entropy() -> Counter {
    Counter(AtomicU64::new(1))
}

/// Counts checks, so tests can tell a refused guess cost no derivation.
struct Counting(AtomicU64);

impl Checker for Counting {
    async fn check(&self, answer: &str, locks: &[WordLock]) -> Option<String> {
        self.0.fetch_add(1, Ordering::Relaxed);
        InlineChecker.check(answer, locks).await
    }
}

fn counting() -> Counting {
    Counting(AtomicU64::new(0))
}

fn at(text: &str) -> DateTime<Utc> {
    text.parse().unwrap()
}

/// A day sealed with one answer; the bundle and keys are never opened here.
fn sealed(day: u32, answer: &str, word: &str, hints: usize) -> SealedDay {
    let n = day as u8;
    SealedDay {
        schema: 1,
        sealed_by: "test".into(),
        day,
        bundle: Encrypted {
            iv: [0; 12],
            ct: vec![0],
        },
        day_key: vec![0],
        word_locks: vec![seal_word(answer, word, [n; 16], [n; 12]).unwrap()],
        hints: (0..hints)
            .map(|_| SealedHint {
                iv: [0; 12],
                ct: vec![0],
                key: vec![0],
                after: None,
            })
            .collect(),
    }
}

fn content(extra: &str) -> EventData {
    let event = Event::from_yaml(&format!(
        "title: T\ntimezone: UTC\nstart: 2026-12-01\ndays: 3\n{extra}"
    ))
    .unwrap();
    EventData::new(
        "2026",
        event,
        [
            sealed(1, "Paris", "light", 2),
            sealed(2, "lantern", "ember", 1),
            sealed(3, "north", "star", 0),
        ],
    )
    .unwrap()
}

/// A store with alice invited; returns her link.
fn with_alice(store: &MemoryStore) -> String {
    let report = block_on(players::import(
        store,
        &entropy(),
        &[Invite {
            email: "alice@acme.com".into(),
            name: Some("Alice".into()),
        }],
        at("2026-11-20T00:00:00Z"),
    ))
    .unwrap();
    assert_eq!(report.created, ["alice@acme.com"]);
    block_on(store.player("alice@acme.com"))
        .unwrap()
        .unwrap()
        .link
}

#[test]
fn import_dedupes_addresses_and_keeps_names() {
    let store = MemoryStore::new();
    let report = block_on(players::import(
        &store,
        &entropy(),
        &[
            Invite {
                email: " Alice@Acme.com ".into(),
                name: Some("Alice".into()),
            },
            Invite {
                email: "alice@acme.com".into(),
                name: None,
            },
            Invite {
                email: "not-an-email".into(),
                name: None,
            },
        ],
        at("2026-11-20T00:00:00Z"),
    ))
    .unwrap();
    assert_eq!(report.created, ["alice@acme.com"]);
    assert_eq!(report.invalid, ["not-an-email"]);

    let all = block_on(store.players()).unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].email, "alice@acme.com");
    assert_eq!(all[0].display_email, "Alice@Acme.com");
    assert_eq!(all[0].name.as_deref(), Some("Alice"));

    // A later import with a name updates it; without one leaves it.
    let report = block_on(players::import(
        &store,
        &entropy(),
        &[Invite {
            email: "ALICE@acme.com".into(),
            name: Some("Alice Martin".into()),
        }],
        at("2026-11-21T00:00:00Z"),
    ))
    .unwrap();
    assert_eq!(report.updated, ["alice@acme.com"]);
    let alice = block_on(store.player("alice@acme.com")).unwrap().unwrap();
    assert_eq!(alice.name.as_deref(), Some("Alice Martin"));
}

#[test]
fn link_signs_in_until_reset() {
    let store = MemoryStore::new();
    let rng = entropy();
    let link = with_alice(&store);
    let now = at("2026-12-01T08:00:00Z");
    let expires = at("2027-01-01T00:00:00Z");

    // Looking at the link changes nothing.
    let shown = block_on(players::link_player(&store, &link)).unwrap();
    assert_eq!(shown.email, "alice@acme.com");
    assert_eq!(shown.last_seen, None);

    let (laptop, player) = block_on(players::sign_in(&store, &rng, &link, now, expires)).unwrap();
    assert_eq!(player.last_seen, Some(now));
    let (phone, _) = block_on(players::sign_in(&store, &rng, &link, now, expires)).unwrap();
    assert_ne!(laptop, phone, "each device gets its own session");
    for session in [&laptop, &phone] {
        assert_eq!(
            block_on(players::session_player(&store, session, now))
                .unwrap()
                .email,
            "alice@acme.com"
        );
    }

    let new_link = block_on(players::reset_link(&store, &rng, "alice@acme.com")).unwrap();
    assert_ne!(new_link, link);
    assert_eq!(
        block_on(players::sign_in(&store, &rng, &link, now, expires)).unwrap_err(),
        ServerError::InvalidLink
    );
    for session in [&laptop, &phone] {
        assert_eq!(
            block_on(players::session_player(&store, session, now)).unwrap_err(),
            ServerError::NoSession
        );
    }
    let (fresh, _) = block_on(players::sign_in(&store, &rng, &new_link, now, expires)).unwrap();
    assert!(block_on(players::session_player(&store, &fresh, now)).is_ok());
}

#[test]
fn sessions_expire_and_sign_out() {
    let store = MemoryStore::new();
    let rng = entropy();
    let link = with_alice(&store);
    let now = at("2026-12-01T08:00:00Z");
    let (session, _) = block_on(players::sign_in(
        &store,
        &rng,
        &link,
        now,
        now + Duration::hours(1),
    ))
    .unwrap();
    assert_eq!(
        block_on(players::session_player(
            &store,
            &session,
            now + Duration::hours(1)
        ))
        .unwrap_err(),
        ServerError::NoSession
    );

    let (session, _) = block_on(players::sign_in(
        &store,
        &rng,
        &link,
        now,
        now + Duration::hours(1),
    ))
    .unwrap();
    block_on(players::sign_out(&store, &session)).unwrap();
    assert_eq!(
        block_on(players::session_player(&store, &session, now)).unwrap_err(),
        ServerError::NoSession
    );
    assert_eq!(
        block_on(players::session_player(&store, "made-up", now)).unwrap_err(),
        ServerError::NoSession
    );
}

#[test]
fn guess_checks_against_the_word_locks() {
    let store = MemoryStore::new();
    with_alice(&store);
    let data = content("");
    let checker = counting();
    let now = at("2026-12-02T09:00:00Z");
    let try_ = |answer: &str| {
        block_on(guess(
            &store,
            &checker,
            &data,
            "alice@acme.com",
            2,
            answer,
            now,
        ))
        .unwrap()
    };

    assert_eq!(try_("Lanterne!"), Outcome::Wrong { remaining: None });
    assert_eq!(
        try_(" LANTERN "),
        Outcome::Solved {
            word: "ember".into()
        }
    );
    assert_eq!(
        try_("anything"),
        Outcome::AlreadySolved {
            word: "ember".into()
        }
    );
    assert_eq!(
        checker.0.load(Ordering::Relaxed),
        2,
        "a solved day is not checked"
    );

    let state = block_on(store.day_state("alice@acme.com", "2026", 2))
        .unwrap()
        .unwrap();
    assert_eq!(state.guesses, 2);
    let solved = state.solved.unwrap();
    assert_eq!(solved.at, now);
    assert!(
        !solved.hint_out,
        "day 2's hint comes out a day after the door"
    );

    // An empty guess costs nothing.
    assert_eq!(
        block_on(guess(
            &store,
            &checker,
            &data,
            "alice@acme.com",
            1,
            "?!",
            now
        ))
        .unwrap(),
        Outcome::Wrong { remaining: None }
    );
    assert_eq!(checker.0.load(Ordering::Relaxed), 2);
}

#[test]
fn guess_records_whether_a_hint_was_out() {
    let store = MemoryStore::new();
    with_alice(&store);
    let data = content("");
    // Day 1 opened on the 1st; its first hint came out on the 2nd.
    let outcome = block_on(guess(
        &store,
        &InlineChecker,
        &data,
        "alice@acme.com",
        1,
        "paris",
        at("2026-12-02T09:00:00Z"),
    ))
    .unwrap();
    assert!(matches!(outcome, Outcome::Solved { .. }));
    let state = block_on(store.day_state("alice@acme.com", "2026", 1))
        .unwrap()
        .unwrap();
    assert!(state.solved.unwrap().hint_out);
}

#[test]
fn guess_refuses_unreleased_and_unknown_days_without_checking() {
    let store = MemoryStore::new();
    with_alice(&store);
    let data = content("");
    let checker = counting();
    let now = at("2026-12-02T09:00:00Z");
    assert_eq!(
        block_on(guess(
            &store,
            &checker,
            &data,
            "alice@acme.com",
            3,
            "north",
            now
        ))
        .unwrap_err(),
        ServerError::NotReleased(3)
    );
    assert_eq!(
        block_on(guess(
            &store,
            &checker,
            &data,
            "alice@acme.com",
            4,
            "x",
            now
        ))
        .unwrap_err(),
        ServerError::UnknownDay(4)
    );
    assert_eq!(checker.0.load(Ordering::Relaxed), 0);
}

#[test]
fn attempt_limit_refills_after_the_window() {
    let store = MemoryStore::new();
    with_alice(&store);
    let data = content("attempts:\n  max: 3\n  per: 1h\n");
    let checker = counting();
    let try_at = |answer: &str, now: &str| {
        block_on(guess(
            &store,
            &checker,
            &data,
            "alice@acme.com",
            2,
            answer,
            at(now),
        ))
        .unwrap()
    };

    assert_eq!(
        try_at("a", "2026-12-02T10:00:00Z"),
        Outcome::Wrong { remaining: Some(2) }
    );
    assert_eq!(
        try_at("b", "2026-12-02T10:05:00Z"),
        Outcome::Wrong { remaining: Some(1) }
    );
    assert_eq!(
        try_at("c", "2026-12-02T10:10:00Z"),
        Outcome::Wrong { remaining: Some(0) }
    );
    assert_eq!(
        try_at("lantern", "2026-12-02T10:20:00Z"),
        Outcome::Limited {
            retry_at: at("2026-12-02T11:00:00Z")
        }
    );
    assert_eq!(
        checker.0.load(Ordering::Relaxed),
        3,
        "a refused guess is not checked"
    );

    assert_eq!(
        try_at("lantern", "2026-12-02T11:00:00Z"),
        Outcome::Solved {
            word: "ember".into()
        }
    );
    let state = block_on(store.day_state("alice@acme.com", "2026", 2))
        .unwrap()
        .unwrap();
    assert_eq!(state.guesses, 4, "refused guesses are not counted");
}

#[test]
fn without_a_limit_every_guess_is_checked() {
    let store = MemoryStore::new();
    with_alice(&store);
    let data = content("");
    let checker = counting();
    for i in 0..50 {
        let outcome = block_on(guess(
            &store,
            &checker,
            &data,
            "alice@acme.com",
            1,
            &format!("wrong{i}"),
            at("2026-12-01T10:00:00Z"),
        ))
        .unwrap();
        assert_eq!(outcome, Outcome::Wrong { remaining: None });
    }
    assert_eq!(checker.0.load(Ordering::Relaxed), 50);
}

#[test]
fn progress_and_stats() {
    let store = MemoryStore::new();
    let rng = entropy();
    let invites: Vec<Invite> = ["alice", "bob", "carol", "dave"]
        .iter()
        .map(|n| Invite {
            email: format!("{n}@acme.com"),
            name: None,
        })
        .collect();
    block_on(players::import(
        &store,
        &rng,
        &invites,
        at("2026-11-20T00:00:00Z"),
    ))
    .unwrap();
    for name in ["alice", "bob", "carol", "dave"] {
        block_on(players::mark_invited(
            &store,
            &format!("{name}@acme.com"),
            at("2026-11-21T00:00:00Z"),
        ))
        .unwrap();
    }
    // Dave never signs in.
    for name in ["alice", "bob", "carol"] {
        let player = block_on(store.player(&format!("{name}@acme.com")))
            .unwrap()
            .unwrap();
        block_on(players::sign_in(
            &store,
            &rng,
            &player.link,
            at("2026-12-01T08:00:00Z"),
            at("2027-01-01T00:00:00Z"),
        ))
        .unwrap();
    }
    block_on(players::set_name(&store, "alice@acme.com", "  Alice ")).unwrap();

    let data = content("");
    let play = |email: &str, day: u32, answers: &[&str], now: &str| {
        for answer in answers {
            block_on(guess(
                &store,
                &InlineChecker,
                &data,
                email,
                day,
                answer,
                at(now),
            ))
            .unwrap();
        }
    };
    // Alice finishes; Bob solves day 1 in three guesses; Carol only tries.
    play("alice@acme.com", 1, &["paris"], "2026-12-01T09:00:00Z");
    play(
        "alice@acme.com",
        2,
        &["x", "lantern"],
        "2026-12-02T09:00:00Z",
    );
    play("alice@acme.com", 3, &["north"], "2026-12-03T09:30:00Z");
    play(
        "bob@acme.com",
        1,
        &["x", "y", "paris"],
        "2026-12-02T09:00:00Z",
    );
    play("carol@acme.com", 1, &["x"], "2026-12-01T09:00:00Z");

    let alice = block_on(progress(&store, &data, "alice@acme.com")).unwrap();
    assert_eq!(alice.name.as_deref(), Some("Alice"));
    assert_eq!(alice.words.get(&2).map(String::as_str), Some("ember"));
    assert_eq!(alice.guesses.get(&2), Some(&2));
    assert_eq!(alice.finished_at, Some(at("2026-12-03T09:30:00Z")));
    let carol = block_on(progress(&store, &data, "carol@acme.com")).unwrap();
    assert!(carol.words.is_empty());
    assert_eq!(carol.guesses.get(&1), Some(&1));
    assert_eq!(carol.finished_at, None);

    let s = block_on(stats(&store, &data)).unwrap();
    assert_eq!(
        (s.players, s.invited, s.signed_in, s.finished),
        (4, 4, 3, 1)
    );
    let day1 = &s.days[0];
    assert_eq!(day1.solved, 2);
    assert!((day1.completion - 2.0 / 3.0).abs() < 1e-9);
    assert_eq!(day1.median_guesses, Some(2.0), "median of 1 and 3");
    assert_eq!(day1.hint_out, 1, "Bob solved on the 2nd, with a hint out");
    assert_eq!(s.days[2].solved, 1);

    let done = block_on(finishers(&store, &data)).unwrap();
    assert_eq!(done.len(), 1);
    assert_eq!(done[0].email, "alice@acme.com");
    assert_eq!(done[0].name.as_deref(), Some("Alice"));

    // A removed player leaves the stats.
    block_on(players::remove(&store, "alice@acme.com")).unwrap();
    let s = block_on(stats(&store, &data)).unwrap();
    assert_eq!((s.players, s.finished), (3, 0));
    assert!(
        block_on(store.player_days("alice@acme.com", "2026"))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn names_are_checked() {
    let store = MemoryStore::new();
    with_alice(&store);
    for bad in ["   ", &"x".repeat(81), "a\u{0}b"] {
        assert!(matches!(
            block_on(players::set_name(&store, "alice@acme.com", bad)),
            Err(ServerError::InvalidName(_))
        ));
    }
}

#[test]
fn sealed_days_outside_the_event_are_refused() {
    let event = Event::from_yaml("title: T\ntimezone: UTC\nstart: 2026-12-01\ndays: 2\n").unwrap();
    assert!(EventData::new("2026", event, [sealed(3, "a", "b", 0)]).is_err());
}

/// The axum adapter needs `Send` futures over a `Send` store.
#[test]
fn guess_future_is_send() {
    fn assert_send<T: Send>(_: &T) {}
    let store = MemoryStore::new();
    let data = content("");
    let future = guess(
        &store,
        &InlineChecker,
        &data,
        "a@b.c",
        1,
        "x",
        at("2026-12-01T00:00:00Z"),
    );
    assert_send(&future);
    let rng = entropy();
    let future = players::sign_in(
        &store,
        &rng,
        "x",
        at("2026-12-01T00:00:00Z"),
        at("2026-12-02T00:00:00Z"),
    );
    assert_send(&future);
}
