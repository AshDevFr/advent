//! Rewards unlocked with the certificate: files and links.

mod common;

use std::fs;
use std::process::Command as Std;

use advent_core::format::{Certificate, read_sealed_final};
use advent_core::primitives;
use common::{Repo, SAMPLE_WORDS, sample_event};
use predicates::prelude::*;
use predicates::str::contains;

const PAYLOAD: &[u8] = b"wallpaper bytes \x00\x01\x02 not an image";

fn with_rewards(repo: &Repo, rewards_yaml: &str) {
    let final_dir = repo.event_dir().join("final");
    fs::write(final_dir.join("wallpaper.bin.txt"), PAYLOAD).unwrap();
    let base = fs::read_to_string(common::examples().join("final/final.yaml")).unwrap();
    fs::write(
        final_dir.join("final.yaml"),
        format!("{base}rewards:\n{rewards_yaml}"),
    )
    .unwrap();
}

fn certificate(repo: &Repo) -> Certificate {
    let sealed =
        read_sealed_final(&fs::read(repo.event_dir().join("final/sealed.json")).unwrap()).unwrap();
    let plain = primitives::open_final(&SAMPLE_WORDS, sealed.salt, sealed.iv, &sealed.ct).unwrap();
    serde_json::from_slice(&plain).unwrap()
}

const BOTH_KINDS: &str = "  - label: Wallpaper\n    file: wallpaper.bin.txt\n  - label: Bonus puzzle\n    url: https://example.com/bonus\n";

#[test]
fn rewards_are_sealed_with_the_certificate() {
    let repo = sample_event();
    with_rewards(&repo, BOTH_KINDS);
    repo.advent()
        .args(["keeper", "seal", "2026"])
        .assert()
        .success();

    let certificate = certificate(&repo);
    assert_eq!(certificate.rewards.len(), 2);
    let link = &certificate.rewards[1];
    assert_eq!(link.url.as_deref(), Some("https://example.com/bonus"));
    let file = certificate.rewards[0].file.as_ref().unwrap();
    assert_eq!(
        (file.name.as_str(), file.size, file.index),
        ("wallpaper.bin.txt", PAYLOAD.len() as u64, 1)
    );

    let ct = fs::read(repo.event_dir().join("final/reward-1.bin")).unwrap();
    assert_ne!(ct, PAYLOAD, "stored encrypted");
    assert_eq!(
        primitives::open_bundle(&file.key, file.iv, &ct).unwrap(),
        PAYLOAD
    );
}

#[test]
fn rewards_need_exactly_one_of_file_or_url() {
    let repo = sample_event();
    with_rewards(
        &repo,
        "  - label: Odd\n    file: wallpaper.bin.txt\n    url: https://x.test\n",
    );
    repo.advent()
        .args(["keeper", "seal", "2026"])
        .assert()
        .failure()
        .stderr(contains("rewards"));
    with_rewards(&repo, "  - label: Empty\n");
    repo.advent()
        .args(["keeper", "seal", "2026"])
        .assert()
        .failure()
        .stderr(contains("exactly one of `file` or `url`"));
}

#[test]
fn rewards_over_25_mb_are_refused() {
    let repo = sample_event();
    with_rewards(&repo, "  - label: Huge\n    file: huge.zip\n");
    fs::write(
        repo.event_dir().join("final/huge.zip"),
        vec![0u8; 26 * 1024 * 1024],
    )
    .unwrap();
    repo.advent()
        .args(["keeper", "seal", "2026"])
        .assert()
        .failure()
        .stderr(contains("the limit is 25 MB"));
}

#[test]
fn rewards_are_published_and_verified() {
    let repo = sample_event();
    with_rewards(&repo, BOTH_KINDS);
    repo.advent()
        .args(["keeper", "seal", "2026"])
        .assert()
        .success();

    repo.advent()
        .args(["build", "--until", "2026-12-03"])
        .env("ADVENT_RELEASE_KEY", &repo.release_secret)
        .assert()
        .success();
    assert_eq!(
        fs::read(repo.root.join("dist/2026/api/rewards/1.bin")).unwrap(),
        fs::read(repo.event_dir().join("final/reward-1.bin")).unwrap()
    );

    repo.advent().arg("verify").assert().success();
    fs::remove_file(repo.event_dir().join("final/reward-1.bin")).unwrap();
    repo.advent()
        .arg("verify")
        .assert()
        .failure()
        .stderr(contains("reward file").and(contains("is missing")));
}

#[test]
fn rewards_sealed_files_are_committed_and_plaintext_is_not() {
    let repo = Repo::init();
    assert!(
        Std::new("git")
            .args(["init", "-q"])
            .current_dir(&repo.root)
            .status()
            .unwrap()
            .success()
    );
    let ignored = |path: &str| {
        Std::new("git")
            .args(["check-ignore", "-q", "--no-index", path])
            .current_dir(&repo.root)
            .status()
            .unwrap()
            .success()
    };
    assert!(!ignored("events/2026/final/reward-1.bin"));
    assert!(ignored("events/2026/final/wallpaper.zip"));
}
