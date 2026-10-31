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

#[test]
fn preview_serves_on_another_host_with_a_warning() {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpStream;
    use std::process::{Command, Stdio};

    let repo = sample_event();
    let mut child = Command::new(assert_cmd::cargo::cargo_bin("advent"))
        .args(["preview", "--host", "0.0.0.0", "--port", "0"])
        .current_dir(&repo.root)
        .env_remove("ADVENT_RELEASE_KEY")
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let port = line
        .split("0.0.0.0:")
        .nth(1)
        .and_then(|rest| rest.split('/').next())
        .unwrap_or_else(|| panic!("no address in {line:?}"))
        .to_string();
    assert!(line.contains("not released yet"), "{line}");

    let mut stream = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
    stream
        .write_all(b"GET /2026/event.json HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n")
        .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
}

#[test]
fn preview_with_days_out_has_no_countdown_whatever_the_clock_says() {
    let repo = sample_event();
    repo.advent()
        .args(["preview", "--out", "pv", "--all-open"])
        .assert()
        .success();
    let site: serde_json::Value =
        serde_json::from_slice(&fs::read(repo.root.join("pv/site.json")).unwrap()).unwrap();
    assert!(site["events"][0]["opens_at"].is_null(), "{site}");
    let event: serde_json::Value =
        serde_json::from_slice(&fs::read(repo.root.join("pv/2026/event.json")).unwrap()).unwrap();
    assert!(event["opens_at"].is_null());
}
