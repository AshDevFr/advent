//! Hint timing: the event default (`hint_after`) and per-hint `after`.

mod common;

use std::fs;

use advent_core::format::read_sealed_day;
use common::Repo;
use predicates::str::contains;

/// An event of `days` days with `hint_after` set and day 1 written from
/// `meta_hints`. Build tests use a 1-day event: a released day that is not
/// sealed stops the build, as it should.
fn event_with(days: u32, hint_after: &str, meta_hints: &str) -> Repo {
    let repo = Repo::init();
    let path = repo.event_dir().join("event.yaml");
    let text = fs::read_to_string(&path)
        .unwrap()
        .replace("days: 24", &format!("days: {days}"))
        .replace("hint_after: [1, 2]", &format!("hint_after: {hint_after}"));
    fs::write(path, text).unwrap();
    repo.add_day(
        1,
        &format!("title: t\ntype: teaser\nanswers: [lantern]\nword: light\nhints:\n{meta_hints}"),
        "Puzzle.",
    );
    repo
}

fn hints_published(repo: &Repo, until: &str) -> usize {
    let out = format!("out-{until}");
    repo.advent()
        .args(["build", "--until", until, "--out", &out])
        .env("ADVENT_RELEASE_KEY", &repo.release_secret)
        .assert()
        .success();
    let path = repo.root.join(&out).join("2026/api/day/1/hint.json");
    let value: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    value["hints"].as_array().unwrap().len()
}

fn seal_day_1(repo: &Repo) {
    repo.advent()
        .args(["seal", "events/2026/day01"])
        .assert()
        .success();
}

#[test]
fn hints_follow_the_event_default() {
    let repo = event_with(1, "[0, 3]", "  - First.\n  - Second.\n");
    seal_day_1(&repo);
    // Day 1 opens Dec 1: hint 1 with the door, hint 2 on Dec 4.
    assert_eq!(hints_published(&repo, "2026-12-01"), 1);
    assert_eq!(hints_published(&repo, "2026-12-03"), 1);
    assert_eq!(hints_published(&repo, "2026-12-04"), 2);
}

#[test]
fn hints_can_set_their_own_delay() {
    let repo = event_with(
        1,
        "[1, 2]",
        "  - {text: Right away., after: 0}\n  - Later.\n",
    );
    seal_day_1(&repo);
    let sealed = read_sealed_day(&fs::read(repo.day_dir(1).join("sealed.json")).unwrap()).unwrap();
    assert_eq!(sealed.hints[0].after, Some(0));
    assert_eq!(sealed.hints[1].after, None, "plain text keeps the default");

    assert_eq!(hints_published(&repo, "2026-12-01"), 1);
    assert_eq!(hints_published(&repo, "2026-12-02"), 1);
    assert_eq!(hints_published(&repo, "2026-12-03"), 2);
}

#[test]
fn hints_out_of_order_are_rejected_at_seal() {
    let repo = event_with(1, "[1, 2]", "  - {text: Late., after: 5}\n  - Early.\n");
    repo.advent()
        .args(["seal", "events/2026/day01"])
        .assert()
        .failure()
        .stderr(contains("out of order"));
    assert!(!repo.day_dir(1).join("sealed.json").exists());
}

#[test]
fn hints_sealed_without_timing_use_the_default() {
    // A day sealed before per-hint timing existed has no `after` field.
    let repo = event_with(1, "[1, 2]", "  - First.\n  - Second.\n");
    seal_day_1(&repo);
    let path = repo.day_dir(1).join("sealed.json");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    for hint in value["hints"].as_array_mut().unwrap() {
        hint.as_object_mut().unwrap().remove("after");
    }
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert_eq!(hints_published(&repo, "2026-12-02"), 1);
}

#[test]
fn hints_schedule_prints_hint_dates() {
    let repo = event_with(3, "[1, 2]", "  - {text: Now., after: 0}\n  - Later.\n");
    seal_day_1(&repo);
    repo.advent()
        .arg("schedule")
        .assert()
        .success()
        .stdout(contains(
            "day  1  Tue 2026-12-01  00:00 UTC  hints Dec 1, Dec 3",
        ))
        .stdout(contains(
            "day  2  Wed 2026-12-02  00:00 UTC  hints Dec 3, Dec 4 (default)",
        ));
}
