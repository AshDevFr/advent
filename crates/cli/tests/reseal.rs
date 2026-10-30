//! `advent unseal-source` and `advent reseal` against a real repo.

mod common;

use std::fs;

use advent_core::format::read_sealed_day;
use advent_core::model::RepoConfig;
use advent_core::primitives;
use advent_core::wrap;
use common::{Repo, examples};
use predicates::str::contains;

const PLAINTEXT: [&str; 3] = ["puzzle.md", "meta.yaml", "hidden.png"];

fn sealed_example(repo: &Repo) -> std::path::PathBuf {
    let day = repo.add_example(6, 1);
    repo.advent()
        .args(["seal", "events/2026/day01"])
        .assert()
        .success();
    day
}

fn set_keys(repo: &Repo, editors: Option<Vec<String>>, release: Option<Vec<String>>) {
    let path = repo.root.join("advent.yaml");
    let mut config = RepoConfig::from_yaml(&fs::read_to_string(&path).unwrap()).unwrap();
    if let Some(e) = editors {
        config.keys.editors = e;
    }
    if let Some(r) = release {
        config.keys.release = r;
    }
    fs::write(&path, serde_yaml::to_string(&config).unwrap()).unwrap();
}

fn config(repo: &Repo) -> RepoConfig {
    RepoConfig::from_yaml(&fs::read_to_string(repo.root.join("advent.yaml")).unwrap()).unwrap()
}

#[test]
fn reseal_unseal_source_restores_plaintext_byte_for_byte() {
    let repo = Repo::init();
    let day = sealed_example(&repo);
    for f in PLAINTEXT {
        fs::remove_file(day.join(f)).unwrap();
    }

    repo.advent()
        .args(["unseal-source", "events/2026/day01"])
        .assert()
        .success();

    for f in PLAINTEXT {
        assert_eq!(
            fs::read(day.join(f)).unwrap(),
            fs::read(examples().join("day06").join(f)).unwrap(),
            "{f} differs"
        );
    }
}

#[test]
fn reseal_unseal_source_refuses_to_overwrite_edits_without_force() {
    let repo = Repo::init();
    let day = sealed_example(&repo);
    fs::write(day.join("puzzle.md"), "my local edit").unwrap();
    fs::remove_file(day.join("meta.yaml")).unwrap();

    repo.advent()
        .args(["unseal-source", "events/2026/day01"])
        .assert()
        .failure()
        .stderr(contains("puzzle.md"));
    assert_eq!(
        fs::read_to_string(day.join("puzzle.md")).unwrap(),
        "my local edit"
    );
    assert!(!day.join("meta.yaml").exists(), "a refusal writes nothing");

    repo.advent()
        .args(["unseal-source", "events/2026/day01", "--force"])
        .assert()
        .success();
    assert_ne!(
        fs::read_to_string(day.join("puzzle.md")).unwrap(),
        "my local edit"
    );
}

#[test]
fn reseal_adds_a_new_editors_key() {
    let repo = Repo::init();
    let day = sealed_example(&repo);
    let (new_secret, new_public) = wrap::generate_identity();
    let mut editors = config(&repo).keys.editors;
    editors.push(new_public);
    set_keys(&repo, Some(editors), None);

    repo.advent().arg("reseal").assert().success();

    let new_id = wrap::parse_identities(&new_secret).unwrap();
    let source = fs::read(day.join("source.age")).unwrap();
    assert!(wrap::decrypt_with(&source, &new_id).is_ok());
    assert!(
        wrap::decrypt_with(&source, &repo.editors()).is_ok(),
        "old key still listed"
    );
}

#[test]
fn reseal_rotates_the_release_key() {
    let repo = Repo::init();
    let day = sealed_example(&repo);
    let (new_secret, new_public) = wrap::generate_identity();
    set_keys(&repo, None, Some(vec![new_public]));

    repo.advent().arg("reseal").assert().success();

    let sealed = read_sealed_day(&fs::read(day.join("sealed.json")).unwrap()).unwrap();
    let new_id = wrap::parse_identities(&new_secret).unwrap();
    assert!(wrap::unwrap_key(&sealed.day_key, &new_id).is_ok());
    for hint in &sealed.hints {
        assert!(wrap::unwrap_key(&hint.key, &new_id).is_ok());
        assert!(wrap::unwrap_key(&hint.key, &repo.release()).is_err());
    }
    assert!(wrap::unwrap_key(&sealed.day_key, &repo.release()).is_err());
}

#[test]
fn reseal_all_seals_again_with_fresh_keys() {
    let repo = Repo::init();
    let day = sealed_example(&repo);
    let before = read_sealed_day(&fs::read(day.join("sealed.json")).unwrap()).unwrap();
    let old_day_key = wrap::unwrap_key(&before.day_key, &repo.release()).unwrap();
    for f in PLAINTEXT {
        fs::remove_file(day.join(f)).unwrap();
    }

    repo.advent().args(["reseal", "--all"]).assert().success();

    let after = read_sealed_day(&fs::read(day.join("sealed.json")).unwrap()).unwrap();
    let new_day_key = wrap::unwrap_key(&after.day_key, &repo.release()).unwrap();
    assert_ne!(new_day_key, old_day_key);
    let bundle = primitives::open_bundle(&new_day_key, after.bundle.iv, &after.bundle.ct).unwrap();
    assert!(String::from_utf8_lossy(&bundle).contains("Look closer"));
    assert_eq!(
        primitives::open_word("frost", &after.word_locks[0]).unwrap(),
        "winter"
    );
}

#[test]
fn reseal_without_editors_key_explains_itself() {
    let repo = Repo::init();
    sealed_example(&repo);
    fs::remove_file(repo.root.join("editors.key")).unwrap();
    repo.advent()
        .arg("reseal")
        .assert()
        .failure()
        .stderr(contains("ADVENT_EDITORS_KEY"));
}
