//! Shared helpers for the CLI integration tests: an initialized event repo
//! in a temp dir, with both private keys captured for opening sealed output.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use advent_core::wrap::{self, x25519};
use assert_cmd::Command;

pub fn advent() -> Command {
    Command::cargo_bin("advent").unwrap()
}

pub struct Repo {
    _tmp: tempfile::TempDir,
    pub root: PathBuf,
    pub release_secret: String,
}

impl Repo {
    pub fn init() -> Repo {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("calendar");
        let out = advent()
            .args(["init", root.to_str().unwrap(), "--event", "2026"])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let release_secret = String::from_utf8(out.stdout)
            .unwrap()
            .lines()
            .find(|l| l.starts_with("AGE-SECRET-KEY-1"))
            .unwrap()
            .to_string();
        Repo {
            _tmp: tmp,
            root,
            release_secret,
        }
    }

    pub fn event_dir(&self) -> PathBuf {
        self.root.join("events/2026")
    }

    pub fn day_dir(&self, n: u32) -> PathBuf {
        self.event_dir().join(format!("day{n:02}"))
    }

    /// Copy a sample day from the template into the event as day `n`.
    pub fn add_example(&self, example: u32, n: u32) -> PathBuf {
        let from = examples().join(format!("day{example:02}"));
        let to = self.day_dir(n);
        copy_dir(&from, &to);
        to
    }

    /// Write a day from scratch.
    pub fn add_day(&self, n: u32, meta: &str, puzzle: &str) -> PathBuf {
        let dir = self.day_dir(n);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("meta.yaml"), meta).unwrap();
        fs::write(dir.join("puzzle.md"), puzzle).unwrap();
        dir
    }

    pub fn editors(&self) -> Vec<x25519::Identity> {
        wrap::parse_identities(&fs::read_to_string(self.root.join("editors.key")).unwrap()).unwrap()
    }

    pub fn release(&self) -> Vec<x25519::Identity> {
        wrap::parse_identities(&self.release_secret).unwrap()
    }

    pub fn advent(&self) -> Command {
        let mut cmd = advent();
        cmd.current_dir(&self.root)
            .env_remove("ADVENT_EDITORS_KEY")
            .env_remove("ADVENT_RELEASE_KEY");
        cmd
    }
}

pub fn examples() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../templates/event/examples")
}

pub fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let path = entry.unwrap().path();
        let target = to.join(path.file_name().unwrap());
        if path.is_dir() {
            copy_dir(&path, &target);
        } else {
            fs::copy(&path, &target).unwrap();
        }
    }
}

/// The template's sample event: 7 days sealed and the certificate sealed.
pub fn sample_event() -> Repo {
    let repo = Repo::init();
    let event_yaml = repo.event_dir().join("event.yaml");
    let text = fs::read_to_string(&event_yaml)
        .unwrap()
        .replace("days: 24", "days: 7");
    fs::write(&event_yaml, text).unwrap();

    for n in 1..=7 {
        repo.add_example(n, n);
        repo.advent()
            .args(["seal", &format!("events/2026/day{n:02}")])
            .assert()
            .success();
    }
    let final_dir = repo.event_dir().join("final");
    fs::create_dir_all(&final_dir).unwrap();
    fs::copy(
        examples().join("final/final.yaml"),
        final_dir.join("final.yaml"),
    )
    .unwrap();
    repo.advent()
        .args(["keeper", "seal", "2026"])
        .assert()
        .success();
    repo
}

pub const SAMPLE_WORDS: [&str; 7] = [
    "light", "returns", "after", "the", "longest", "winter", "night",
];

pub const SIMPLE_META: &str =
    "title: Simple\ntype: teaser\nanswers: [lantern]\nword: light\nhints: [It glows.]\n";
