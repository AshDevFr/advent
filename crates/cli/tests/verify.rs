//! `advent verify`, and the sample event end to end: init, add every
//! example day, seal, keeper seal, verify at every depth.

mod common;

use std::fs;
use std::process::Command as Std;

use advent_core::format::read_sealed_day;
use common::Repo;
use predicates::prelude::*;
use predicates::str::contains;

/// The full sample event, in a git repo with everything staged.
fn sample_event() -> Repo {
    let repo = common::sample_event();
    git(&repo, &["init", "-q"]);
    git(&repo, &["add", "-A"]);
    repo
}

fn git(repo: &Repo, args: &[&str]) {
    assert!(
        Std::new("git")
            .args(args)
            .current_dir(&repo.root)
            .status()
            .unwrap()
            .success()
    );
}

#[test]
fn verify_sample_event_passes_with_both_keys() {
    let repo = sample_event();
    repo.advent()
        .arg("verify")
        .assert()
        .success()
        .stdout(contains("OK"));
    repo.advent()
        .args(["verify", "--release"])
        .env("ADVENT_RELEASE_KEY", &repo.release_secret)
        .assert()
        .success()
        .stdout(contains("OK"));
}

#[test]
fn verify_release_names_a_corrupted_day_without_content() {
    let repo = sample_event();
    let path = repo.day_dir(3).join("sealed.json");
    let mut sealed = read_sealed_day(&fs::read(&path).unwrap()).unwrap();
    sealed.bundle.ct[0] ^= 1;
    fs::write(&path, serde_json::to_vec_pretty(&sealed).unwrap()).unwrap();

    let out = repo
        .advent()
        .args(["verify", "--release"])
        .env("ADVENT_RELEASE_KEY", &repo.release_secret)
        .output()
        .unwrap();
    assert!(!out.status.success());
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(all.contains("day 3"), "{all}");
    // Day 3 is the company sample: none of its puzzle, answer or word may appear.
    for secret in ["Northwind", "Lumenville", "wick", "after"] {
        assert!(
            !all.contains(secret),
            "verify output leaked {secret:?}: {all}"
        );
    }
}

#[test]
fn verify_names_a_missing_day_without_any_key() {
    let repo = sample_event();
    fs::remove_file(repo.root.join("editors.key")).unwrap();
    fs::remove_file(repo.day_dir(4).join("sealed.json")).unwrap();
    repo.advent()
        .arg("verify")
        .assert()
        .failure()
        .stdout(contains("No editors key"))
        .stderr(contains("day 4 is not sealed"));
}

#[test]
fn verify_reports_a_stale_certificate() {
    let repo = sample_event();
    let day = repo.day_dir(2);
    let meta = fs::read_to_string(day.join("meta.yaml"))
        .unwrap()
        .replace("word: returns", "word: comes");
    fs::write(day.join("meta.yaml"), meta).unwrap();
    repo.advent()
        .args(["seal", "events/2026/day02"])
        .assert()
        .success();

    repo.advent()
        .arg("verify")
        .assert()
        .failure()
        .stderr(contains("certificate is stale"));
}

#[test]
fn verify_rejects_force_added_plaintext() {
    let repo = sample_event();
    git(&repo, &["add", "-f", "events/2026/day01/meta.yaml"]);
    repo.advent()
        .arg("verify")
        .assert()
        .failure()
        .stderr(contains("events/2026/day01/meta.yaml"));
}

#[test]
fn verify_reports_the_engine_that_sealed_an_unknown_schema() {
    let repo = sample_event();
    let path = repo.day_dir(5).join("sealed.json");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["schema"] = 2.into();
    value["sealed_by"] = "advent 9.0.0".into();
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();

    repo.advent()
        .arg("verify")
        .assert()
        .failure()
        .stderr(contains("advent 9.0.0").and(contains("day 5")));
}
