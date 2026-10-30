//! `advent seal` against a real initialized repo.

mod common;

use std::fs;
use std::io::Cursor;

use advent_core::format::{Bundle, read_sealed_day};
use advent_core::primitives;
use advent_core::wrap;
use common::{Repo, SIMPLE_META};
use image::{ImageBuffer, ImageFormat, Rgb, RgbImage};
use predicates::str::contains;

fn png(img: &RgbImage) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, ImageFormat::Png).unwrap();
    out.into_inner()
}

#[test]
fn seal_writes_sealed_files_and_leaves_plaintext_alone() {
    let repo = Repo::init();
    let day = repo.add_example(1, 1);
    let puzzle_before = fs::read(day.join("puzzle.md")).unwrap();

    repo.advent()
        .args(["seal", "events/2026/day01"])
        .assert()
        .success();

    let sealed = read_sealed_day(&fs::read(day.join("sealed.json")).unwrap()).unwrap();
    assert_eq!(sealed.schema, 1);
    assert_eq!(sealed.day, 1);
    assert!(day.join("source.age").is_file());
    assert_eq!(fs::read(day.join("puzzle.md")).unwrap(), puzzle_before);
}

#[test]
fn seal_rejects_a_word_that_is_not_normalized() {
    let repo = Repo::init();
    let day = repo.add_day(
        1,
        "title: t\ntype: teaser\nanswers: [lantern]\nword: Light\nhints: [h]\n",
        "# t",
    );
    repo.advent()
        .args(["seal", "events/2026/day01"])
        .assert()
        .failure()
        .stderr(contains("normalized form"));
    assert!(!day.join("sealed.json").exists());
    assert!(!day.join("source.age").exists());
}

#[test]
fn seal_warns_about_a_guessable_answer_but_seals() {
    let repo = Repo::init();
    let day = repo.add_day(
        1,
        "title: t\ntype: code\nanswers: ['2014']\nword: light\nhints: [h]\n",
        "# t",
    );
    repo.advent()
        .args(["seal", "events/2026/day01"])
        .assert()
        .success()
        .stderr(contains("easy to guess"));
    assert!(day.join("sealed.json").is_file());
}

#[test]
fn seal_converts_large_images_to_webp_and_rewrites_markdown() {
    let repo = Repo::init();
    let day = repo.add_day(1, SIMPLE_META, "# Map\n\n![map](map.png)\n");
    let img = ImageBuffer::from_pixel(3000, 2000, Rgb([10u8, 120, 200]));
    fs::write(day.join("map.png"), png(&img)).unwrap();

    repo.advent()
        .args(["seal", "events/2026/day01"])
        .assert()
        .success();

    let bundle = open_bundle(&repo, 1);
    assert_eq!(bundle.markdown, "# Map\n\n![map](map.webp)\n");
    let webp =
        image::load_from_memory_with_format(&bundle.assets["map.webp"], ImageFormat::WebP).unwrap();
    assert!(webp.width() <= 1600 && webp.height() <= 1600);
    assert!(!bundle.assets.contains_key("map.png"));
}

#[test]
fn seal_warns_when_images_exceed_the_budget() {
    let repo = Repo::init();
    let day = repo.add_day(1, SIMPLE_META, "![noise](noise.png)\n");
    // Noise does not compress, so lossless WebP stays far above 1.5 MB.
    let mut state = 0x2545_f491_u32;
    let img = ImageBuffer::from_fn(1500, 1000, |_, _| {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        Rgb([state as u8, (state >> 8) as u8, (state >> 16) as u8])
    });
    fs::write(day.join("noise.png"), png(&img)).unwrap();

    repo.advent()
        .args(["seal", "events/2026/day01"])
        .assert()
        .success()
        .stderr(contains("MB of images"));
}

#[test]
fn seal_output_opens_with_release_and_editors_keys() {
    let repo = Repo::init();
    let day = repo.add_example(6, 1);
    repo.advent()
        .args(["seal", "events/2026/day01"])
        .assert()
        .success();

    let bundle = open_bundle(&repo, 1);
    assert_eq!(bundle.kind, "image");
    assert!(bundle.assets.contains_key("hidden.webp"));

    let sealed = read_sealed_day(&fs::read(day.join("sealed.json")).unwrap()).unwrap();
    let hint_key = wrap::unwrap_key(&sealed.hints[0].key, &repo.release()).unwrap();
    let hint = primitives::open_hint(&hint_key, sealed.hints[0].iv, &sealed.hints[0].ct).unwrap();
    assert!(hint.contains("darker"));
    assert_eq!(
        primitives::open_word("Frost", &sealed.word_locks[0]).unwrap(),
        "winter"
    );

    let source = wrap::decrypt_with(&fs::read(day.join("source.age")).unwrap(), &repo.editors());
    assert!(source.is_ok(), "source.age opens with the editors key");
    assert!(
        wrap::decrypt_with(&fs::read(day.join("source.age")).unwrap(), &repo.release()).is_err()
    );
}

#[test]
fn seal_refuses_a_day_beyond_the_event() {
    let repo = Repo::init();
    let day = repo.add_day(30, SIMPLE_META, "# t");
    repo.advent()
        .args(["seal", "events/2026/day30"])
        .assert()
        .failure()
        .stderr(contains("24 days"));
    assert!(!day.join("sealed.json").exists());
}

#[test]
fn seal_works_from_inside_the_day_folder() {
    let repo = Repo::init();
    let day = repo.add_day(2, SIMPLE_META, "# t");
    common::advent()
        .current_dir(&day)
        .args(["seal", "."])
        .assert()
        .success();
    assert!(day.join("sealed.json").is_file());
}

fn open_bundle(repo: &Repo, n: u32) -> Bundle {
    let sealed = read_sealed_day(&fs::read(repo.day_dir(n).join("sealed.json")).unwrap()).unwrap();
    let day_key = wrap::unwrap_key(&sealed.day_key, &repo.release()).unwrap();
    let plain = primitives::open_bundle(&day_key, sealed.bundle.iv, &sealed.bundle.ct).unwrap();
    serde_json::from_slice(&plain).unwrap()
}
