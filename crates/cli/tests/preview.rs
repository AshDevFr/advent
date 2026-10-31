//! `advent preview --out`: the preview build, without serving it.

mod common;

use std::fs;

use common::{SAMPLE_WORDS, sample_event};
use predicates::str::contains;

#[test]
fn preview_needs_only_the_editors_key() {
    let repo = sample_event();
    // No ADVENT_RELEASE_KEY: Repo::advent clears it.
    repo.advent()
        .args(["preview", "--out", "pv", "--until", "2026-12-03"])
        .assert()
        .success();
    let api = repo.root.join("pv/2026/api/day");
    for n in 1..=3 {
        assert!(api.join(format!("{n}.json")).is_file(), "day {n}");
    }
    assert!(!api.join("4.json").exists());
}

#[test]
fn preview_all_open_publishes_every_day_and_hint() {
    let repo = sample_event();
    repo.advent()
        .args(["preview", "--out", "pv", "--all-open"])
        .assert()
        .success();
    let api = repo.root.join("pv/2026/api/day");
    for n in 1..=7 {
        assert!(api.join(format!("{n}.json")).is_file(), "day {n}");
    }
    let hints: serde_json::Value =
        serde_json::from_slice(&fs::read(api.join("7/hint.json")).unwrap()).unwrap();
    assert_eq!(hints["hints"].as_array().unwrap().len(), 2);
}

#[test]
fn preview_solved_prefills_every_word() {
    let repo = sample_event();
    repo.advent()
        .args(["preview", "--out", "pv", "--all-open", "--solved"])
        .assert()
        .success();
    for page in ["pv/index.html", "pv/2026/index.html"] {
        let html = fs::read_to_string(repo.root.join(page)).unwrap();
        assert!(html.contains("advent preview --solved"), "{page}");
        for word in SAMPLE_WORDS {
            assert!(html.contains(&format!("\"{word}\"")), "{page} lacks {word}");
        }
    }
}

#[test]
fn preview_explains_it_needs_the_editors_key() {
    let repo = sample_event();
    fs::remove_file(repo.root.join("editors.key")).unwrap();
    repo.advent()
        .args(["preview", "--out", "pv"])
        .assert()
        .failure()
        .stderr(contains("editors key"));
}

#[test]
fn preview_skips_days_still_being_written() {
    let repo = sample_event();
    fs::remove_file(repo.day_dir(2).join("sealed.json")).unwrap();
    fs::remove_file(repo.day_dir(2).join("source.age")).unwrap();
    repo.advent()
        .args(["preview", "--out", "pv", "--all-open"])
        .assert()
        .success()
        .stderr(contains("day 2 is not sealed yet"));
    assert!(!repo.root.join("pv/2026/api/day/2.json").exists());
    assert!(repo.root.join("pv/2026/api/day/3.json").exists());
}
