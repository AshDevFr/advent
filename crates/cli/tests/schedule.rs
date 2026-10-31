//! `advent schedule` prints what the event's schedule options work out to.

mod common;

use std::fs;

use common::Repo;
use predicates::prelude::*;
use predicates::str::contains;

fn set_schedule(repo: &Repo, extra: &str) {
    let path = repo.event_dir().join("event.yaml");
    let text = fs::read_to_string(&path)
        .unwrap()
        .replace("days: 24", &format!("days: 12\n{extra}"));
    fs::write(path, text).unwrap();
}

#[test]
fn schedule_prints_every_day_with_its_date() {
    let repo = Repo::init();
    set_schedule(&repo, "every: 2");
    repo.advent()
        .arg("schedule")
        .assert()
        .success()
        .stdout(contains("day  1  Tue 2026-12-01  00:00 UTC"))
        .stdout(contains("day  2  Thu 2026-12-03"))
        .stdout(contains("day 12  Wed 2026-12-23"));
}

#[test]
fn schedule_workdays_skip_the_weekend() {
    let repo = Repo::init();
    set_schedule(&repo, "weekdays: true");
    repo.advent()
        .arg("schedule")
        .assert()
        .success()
        .stdout(contains("day  5  Mon 2026-12-07"))
        .stdout(contains("Sat").not())
        .stdout(contains("Sun").not());
}

#[test]
fn schedule_explains_invalid_dates() {
    let repo = Repo::init();
    set_schedule(&repo, "dates: [2026-12-01, 2026-12-02]");
    repo.advent()
        .arg("schedule")
        .assert()
        .failure()
        .stderr(contains("has 2 dates but the event has 12 days"));
}

#[cfg(unix)]
#[test]
fn schedule_exits_quietly_when_output_is_cut_short() {
    // `advent schedule | head -1` closes the pipe early; a CLI should stop
    // quietly then rather than panic.
    let repo = Repo::init();
    let bin = assert_cmd::cargo::cargo_bin("advent");
    let out = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("'{}' schedule | head -1", bin.display()))
        .current_dir(&repo.root)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("panicked"), "{stderr}");
}
