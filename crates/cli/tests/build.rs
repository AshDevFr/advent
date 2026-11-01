//! `advent build` over the sealed sample event.

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use advent_core::format::read_sealed_final;
use advent_core::primitives;
use common::{Repo, SAMPLE_WORDS, examples, sample_event};
use predicates::str::contains;

fn build(repo: &Repo, until: &str, out: &str) -> PathBuf {
    repo.advent()
        .args(["build", "--until", until, "--out", out])
        .env("ADVENT_RELEASE_KEY", &repo.release_secret)
        .assert()
        .success();
    repo.root.join(out)
}

fn tree(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut files = BTreeMap::new();
    fn walk(root: &Path, dir: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, files);
            } else {
                let rel = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                files.insert(rel, fs::read(&path).unwrap());
            }
        }
    }
    walk(dir, dir, &mut files);
    files
}

#[test]
fn build_publishes_only_released_days() {
    let repo = sample_event();
    let out = build(&repo, "2026-12-03", "dist");
    let api = out.join("2026/api/day");
    for n in 1..=3 {
        assert!(api.join(format!("{n}.json")).is_file(), "day {n} json");
        assert!(
            api.join(format!("{n}/locks.json")).is_file(),
            "day {n} locks"
        );
    }
    for n in 4..=7 {
        assert!(
            !api.join(format!("{n}.json")).exists(),
            "day {n} must not exist"
        );
        assert!(
            !api.join(n.to_string()).exists(),
            "day {n} folder must not exist"
        );
    }
    let event: serde_json::Value =
        serde_json::from_slice(&fs::read(out.join("2026/event.json")).unwrap()).unwrap();
    assert_eq!(event["released"], serde_json::json!([1, 2, 3]));
    assert_eq!(
        event["types"],
        serde_json::json!({"1": "teaser", "2": "code", "3": "company"}),
        "types of released days only"
    );
    let dates = event["dates"].as_array().unwrap();
    assert_eq!(dates.len(), 7, "every day's date, released or not");
    assert_eq!(dates[6], "2026-12-07");
}

#[test]
fn build_publishes_hints_on_schedule() {
    let repo = sample_event();
    let out = build(&repo, "2026-12-03", "dist");
    let hints = |n: u32| -> usize {
        let path = out.join(format!("2026/api/day/{n}/hint.json"));
        let v: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        v["hints"].as_array().unwrap().len()
    };
    assert_eq!((hints(1), hints(2), hints(3)), (2, 1, 0));
}

#[test]
fn build_leaks_no_answer_and_no_unreleased_text() {
    let repo = sample_event();
    let out = build(&repo, "2026-12-03", "dist");

    // (day, text that must not appear). Answers of unreleased days must not
    // appear anywhere; a released day's answer may appear only in that day's
    // own puzzle text, where the author wrote it (the company sample quotes
    // its handbook). Unreleased titles must not appear at all.
    let mut secrets: Vec<(u32, String)> = Vec::new();
    for n in 1..=7u32 {
        let meta = fs::read_to_string(examples().join(format!("day{n:02}/meta.yaml"))).unwrap();
        let meta: serde_yaml::Value = serde_yaml::from_str(&meta).unwrap();
        for answer in meta["answers"].as_sequence().unwrap() {
            secrets.push((n, answer.as_str().unwrap().to_string()));
        }
        if n > 3 {
            secrets.push((n, meta["title"].as_str().unwrap().to_string()));
        }
    }

    for (path, bytes) in tree(&out) {
        // Ciphertext is base64 and can contain any short string by chance.
        if path.ends_with("locks.json") || path.ends_with("final.json") {
            continue;
        }
        // The embedded frontend is compiled before any event exists, so it
        // cannot carry event data; its minified code matches short answers
        // like "55" by coincidence.
        if path.starts_with("assets/") || path.ends_with("index.html") {
            continue;
        }
        let text = String::from_utf8_lossy(&bytes);
        assert!(!text.contains("\"answers\""), "{path} has an answers field");
        for (day, secret) in &secrets {
            if path == format!("2026/api/day/{day}.json") {
                continue;
            }
            let as_word = text
                .split(|c: char| !c.is_alphanumeric())
                .any(|w| w.eq_ignore_ascii_case(secret));
            let as_phrase = secret.contains(' ') && text.contains(secret.as_str());
            assert!(
                !as_word && !as_phrase,
                "{path} contains {secret:?} from day {day}"
            );
        }
    }
    assert!(!tree(&out).keys().any(|p| p.ends_with("source.age")));
}

#[test]
fn build_certificate_opens_with_the_sample_words() {
    let repo = sample_event();
    let out = build(&repo, "2026-12-03", "dist");
    let sealed = read_sealed_final(&fs::read(out.join("2026/api/final.json")).unwrap()).unwrap();
    let plain = primitives::open_final(&SAMPLE_WORDS, sealed.salt, sealed.iv, &sealed.ct).unwrap();
    let certificate: serde_json::Value = serde_json::from_slice(&plain).unwrap();
    assert!(
        certificate["title"]
            .as_str()
            .unwrap()
            .contains("longest night")
    );
}

#[test]
fn build_is_identical_for_the_same_date() {
    let repo = sample_event();
    let first = tree(&build(&repo, "2026-12-03", "one"));
    let second = tree(&build(&repo, "2026-12-03", "two"));
    assert_eq!(first, second);
    let again = tree(&build(&repo, "2026-12-03", "one"));
    assert_eq!(first, again, "rebuilding into the same folder replaces it");
}

#[test]
fn build_publishes_a_logo_and_background_for_each_scheme() {
    let repo = sample_event();
    let brand = repo.root.join("brand");
    fs::copy(brand.join("logo.svg"), brand.join("logo-dark.svg")).unwrap();
    let yaml = fs::read_to_string(brand.join("brand.yaml"))
        .unwrap()
        .replace(
            "logo: logo.svg",
            "logo:\n  light: logo.svg\n  dark: logo-dark.svg",
        );
    fs::write(brand.join("brand.yaml"), yaml).unwrap();
    for file in ["day.png", "night.png"] {
        fs::copy(
            repo.root.join("examples/day06/hidden.png"),
            repo.event_dir().join(file),
        )
        .unwrap();
    }
    fs::write(
        repo.event_dir().join("theme.yaml"),
        "background:\n  light: day.png\n  dark: night.png\n",
    )
    .unwrap();
    let out = build(&repo, "2026-12-03", "dist");
    let site: serde_json::Value =
        serde_json::from_slice(&fs::read(out.join("site.json")).unwrap()).unwrap();
    assert_eq!(site["brand"]["logo"], "brand/logo.svg");
    assert_eq!(site["brand"]["logo_dark"], "brand/logo-dark.svg");
    assert!(out.join("brand/logo-dark.svg").is_file());
    assert_eq!(site["events"][0]["background"], "2026/theme/day.png");
    assert_eq!(site["events"][0]["background_dark"], "2026/theme/night.png");
    assert!(out.join("2026/theme/night.png").is_file());
}

#[test]
fn build_publishes_a_list_of_door_artwork_in_order() {
    let repo = sample_event();
    for file in ["a.png", "b.png"] {
        fs::copy(
            repo.root.join("examples/day06/hidden.png"),
            repo.event_dir().join(file),
        )
        .unwrap();
    }
    fs::write(
        repo.event_dir().join("theme.yaml"),
        "doors: [b.png, a.png]\n",
    )
    .unwrap();
    let out = build(&repo, "2026-12-03", "dist");
    let event: serde_json::Value =
        serde_json::from_slice(&fs::read(out.join("2026/event.json")).unwrap()).unwrap();
    assert_eq!(
        event["doors"],
        serde_json::json!(["2026/theme/b.png", "2026/theme/a.png"])
    );
    assert!(out.join("2026/theme/a.png").is_file());
}

#[test]
fn build_passes_the_theme_columns_on_only_when_set() {
    let repo = sample_event();
    let out = build(&repo, "2026-12-03", "dist");
    let event: serde_json::Value =
        serde_json::from_slice(&fs::read(out.join("2026/event.json")).unwrap()).unwrap();
    assert!(
        event.get("columns").is_none(),
        "the calendar picks one itself"
    );

    fs::write(repo.event_dir().join("theme.yaml"), "columns: 5\n").unwrap();
    let out = build(&repo, "2026-12-03", "dist2");
    let event: serde_json::Value =
        serde_json::from_slice(&fs::read(out.join("2026/event.json")).unwrap()).unwrap();
    assert_eq!(event["columns"], 5);
}

#[test]
fn build_writes_brand_and_theme() {
    let repo = sample_event();
    fs::write(
        repo.event_dir().join("theme.yaml"),
        "palette:\n  light:\n    primary: \"#112233\"\n  dark:\n    primary: \"#ccddee\"\n",
    )
    .unwrap();
    let out = build(&repo, "2026-12-03", "dist");

    let site: serde_json::Value =
        serde_json::from_slice(&fs::read(out.join("site.json")).unwrap()).unwrap();
    assert_eq!(site["brand"]["name"], "Advent Calendar");
    assert_eq!(site["brand"]["logo"], "brand/logo.svg");
    assert_eq!(site["events"][0]["title"], "Advent 2026");
    assert!(out.join("brand/logo.svg").is_file());

    let root_css = fs::read_to_string(out.join("theme.css")).unwrap();
    assert!(
        root_css.contains("--primary: #a8621a;"),
        "brand light primary"
    );
    assert!(
        root_css.contains("--primary: #e3a54f;"),
        "brand dark primary"
    );
    // The landing page lists every event, each in its own colours.
    let (root_rules, scoped) = root_css.split_once("[data-event=\"2026\"] {").unwrap();
    assert!(!root_rules.contains("#112233"), "the page keeps the brand");
    assert!(scoped.contains("--primary: #112233;"));
    assert!(scoped.contains("--primary: #ccddee;"));
    let event_css = fs::read_to_string(out.join("2026/theme.css")).unwrap();
    assert!(
        event_css.contains("--primary: #112233;"),
        "theme light overrides"
    );
    assert!(
        event_css.contains("--primary: #ccddee;"),
        "theme dark overrides"
    );
}

#[test]
fn build_publishes_notices_once_their_moment_comes() {
    let repo = sample_event();
    let path = repo.event_dir().join("event.yaml");
    let text = fs::read_to_string(&path).unwrap()
        + "notices:\n  - text: Always\n  - text: Day three\n    day: 3\n  - text: Early December\n    date: 2026-12-02\n  - text: Day four\n    day: 4\n  - text: Mid December\n    date: 2026-12-10\n";
    fs::write(path, text).unwrap();

    let out = build(&repo, "2026-12-03", "dist");
    let event: serde_json::Value =
        serde_json::from_slice(&fs::read(out.join("2026/event.json")).unwrap()).unwrap();
    let texts: Vec<&str> = event["notices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["text"].as_str().unwrap())
        .collect();
    assert_eq!(texts, ["Always", "Day three", "Early December"]);
    assert_eq!(event["notices"][0]["id"].as_str().unwrap().len(), 12);
}

#[test]
fn build_requires_the_release_key() {
    let repo = sample_event();
    repo.advent()
        .args(["build", "--until", "2026-12-03"])
        .assert()
        .failure()
        .stderr(contains("ADVENT_RELEASE_KEY"));
}

#[test]
fn build_refuses_to_replace_a_folder_it_did_not_write() {
    let repo = sample_event();
    repo.advent()
        .args(["build", "--until", "2026-12-03", "--out", "events"])
        .env("ADVENT_RELEASE_KEY", &repo.release_secret)
        .assert()
        .failure()
        .stderr(contains("not written by `advent build`"));
    assert!(repo.day_dir(1).join("sealed.json").is_file());
}

#[test]
fn build_counts_down_only_until_the_first_day_is_out() {
    let repo = sample_event();
    let before = build(&repo, "2026-11-30", "before");
    let event: serde_json::Value =
        serde_json::from_slice(&fs::read(before.join("2026/event.json")).unwrap()).unwrap();
    assert!(event["opens_at"].as_str().unwrap().starts_with("2026-1"));
    let after = build(&repo, "2026-12-01", "after");
    let event: serde_json::Value =
        serde_json::from_slice(&fs::read(after.join("2026/event.json")).unwrap()).unwrap();
    assert!(event["opens_at"].is_null());
}
