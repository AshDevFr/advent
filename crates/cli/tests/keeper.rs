//! `advent keeper seal` against a real repo with a short event.

mod common;

use std::fs;

use advent_core::format::{Certificate, read_sealed_final};
use advent_core::primitives;
use advent_core::wrap;
use common::Repo;
use predicates::str::contains;

const WORDS: [&str; 3] = ["light", "returns", "soon"];

/// A 3-day event with every day sealed (or all but `skip`) and a final.yaml.
fn short_event(skip: Option<u32>) -> Repo {
    let repo = Repo::init();
    let event_yaml = repo.event_dir().join("event.yaml");
    let text = fs::read_to_string(&event_yaml)
        .unwrap()
        .replace("days: 24", "days: 3");
    fs::write(&event_yaml, text).unwrap();

    for (i, word) in WORDS.iter().enumerate() {
        let n = i as u32 + 1;
        repo.add_day(
            n,
            &format!(
                "title: Day {n}\ntype: teaser\nanswers: [answer{n}x]\nword: {word}\nhints: [h]\n"
            ),
            "# t",
        );
        if Some(n) != skip {
            repo.advent()
                .args(["seal", &format!("events/2026/day{n:02}")])
                .assert()
                .success();
        }
    }
    let final_dir = repo.event_dir().join("final");
    fs::create_dir_all(&final_dir).unwrap();
    fs::write(
        final_dir.join("final.yaml"),
        "title: Well done\nmessage: You found every word.\n",
    )
    .unwrap();
    repo
}

fn open_certificate(repo: &Repo, words: &[&str]) -> Option<Certificate> {
    let sealed =
        read_sealed_final(&fs::read(repo.event_dir().join("final/sealed.json")).unwrap()).unwrap();
    let plain = primitives::open_final(words, sealed.salt, sealed.iv, &sealed.ct).ok()?;
    Some(serde_json::from_slice(&plain).unwrap())
}

#[test]
fn keeper_seals_the_certificate_with_the_words_in_order() {
    let repo = short_event(None);
    repo.advent()
        .args(["keeper", "seal", "2026"])
        .assert()
        .success();

    let certificate = open_certificate(&repo, &WORDS).expect("opens with the words in order");
    assert_eq!(certificate.title, "Well done");
    assert_eq!(certificate.message, "You found every word.");
    assert!(certificate.image.is_none());
    assert!(open_certificate(&repo, &["soon", "returns", "light"]).is_none());
}

#[test]
fn keeper_seals_every_certificate_option() {
    let repo = short_event(None);
    let final_dir = repo.event_dir().join("final");
    for name in ["medal.png", "paper.png"] {
        fs::copy(
            common::examples().join("day06/hidden.png"),
            final_dir.join(name),
        )
        .unwrap();
    }
    fs::write(
        final_dir.join("final.yaml"),
        "title: Well done\nmessage: You found every word.\nsubtitle: is hereby recognised for\nsignature: The Holiday Committee\nimage: medal.png\nimage_position: top\nbackground: paper.png\n",
    )
    .unwrap();
    repo.advent()
        .args(["keeper", "seal", "2026"])
        .assert()
        .success();

    let certificate = open_certificate(&repo, &WORDS).unwrap();
    assert_eq!(
        certificate.subtitle.as_deref(),
        Some("is hereby recognised for")
    );
    assert_eq!(
        certificate.signature.as_deref(),
        Some("The Holiday Committee")
    );
    assert_eq!(
        certificate.image_position,
        advent_core::model::ImagePosition::Top
    );
    for image in [certificate.image.unwrap(), certificate.background.unwrap()] {
        assert_eq!(&image[..4], b"RIFF", "converted to WebP");
    }
}

#[test]
fn keeper_rejects_an_unknown_image_position() {
    let repo = short_event(None);
    fs::write(
        repo.event_dir().join("final/final.yaml"),
        "title: t\nmessage: m\nimage_position: middle\n",
    )
    .unwrap();
    repo.advent()
        .args(["keeper", "seal", "2026"])
        .assert()
        .failure()
        .stderr(contains("image_position"));
}

#[test]
fn keeper_refuses_while_a_day_is_unsealed() {
    let repo = short_event(Some(2));
    repo.advent()
        .args(["keeper", "seal", "2026"])
        .assert()
        .failure()
        .stderr(contains("not sealed: 2"));
    assert!(!repo.event_dir().join("final/sealed.json").exists());
    assert!(!repo.event_dir().join("final/source.age").exists());
}

#[test]
fn keeper_source_restores_final_yaml() {
    let repo = short_event(None);
    repo.advent()
        .args(["keeper", "seal", "2026"])
        .assert()
        .success();
    let encrypted = fs::read(repo.event_dir().join("final/source.age")).unwrap();
    let archive = wrap::decrypt_with(&encrypted, &repo.editors()).unwrap();
    let text = String::from_utf8_lossy(&archive);
    assert!(text.contains("message: You found every word."));
}

#[test]
fn keeper_certificate_survives_reseal_all() {
    let repo = short_event(None);
    repo.advent()
        .args(["keeper", "seal", "2026"])
        .assert()
        .success();
    let before = fs::read(repo.event_dir().join("final/sealed.json")).unwrap();

    repo.advent().args(["reseal", "--all"]).assert().success();

    let after = fs::read(repo.event_dir().join("final/sealed.json")).unwrap();
    assert_ne!(before, after, "sealed again with a fresh salt and IV");
    assert_eq!(open_certificate(&repo, &WORDS).unwrap().title, "Well done");
}
