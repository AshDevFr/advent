//! The sample days shipped in the event template: one per puzzle type, valid,
//! and usable as a complete event (their words form the sample final key).

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use advent_core::model::DayMeta;
use advent_core::{GuessableReason, guessable, is_normalized_word, normalize};

const TYPES: [&str; 7] = [
    "teaser", "code", "company", "cipher", "logic", "image", "meta",
];

fn examples() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../templates/common/examples")
}

fn days() -> Vec<(u32, DayMeta, String)> {
    (1..=7)
        .map(|n| {
            let dir = examples().join(format!("day{n:02}"));
            let meta = DayMeta::from_yaml(&fs::read_to_string(dir.join("meta.yaml")).unwrap())
                .unwrap_or_else(|e| panic!("day{n:02}/meta.yaml: {e}"));
            let puzzle = fs::read_to_string(dir.join("puzzle.md")).unwrap();
            (n, meta, puzzle)
        })
        .collect()
}

#[test]
fn examples_cover_every_type_once_with_meta_last() {
    let days = days();
    let kinds: BTreeSet<&str> = days.iter().map(|(_, m, _)| m.kind.as_str()).collect();
    assert_eq!(kinds, BTreeSet::from(TYPES));
    assert_eq!(days.len(), TYPES.len(), "exactly one day per type");
    assert_eq!(days.last().unwrap().1.kind, "meta");
    assert!(!examples().join("day08").exists());
}

#[test]
fn examples_words_and_answers_are_valid() {
    for (n, meta, _) in days() {
        assert!(
            is_normalized_word(&meta.word),
            "day{n:02} word {:?}",
            meta.word
        );
        for answer in &meta.answers {
            assert!(!normalize(answer).is_empty(), "day{n:02} answer {answer:?}");
        }
    }
    let phrase: Vec<String> = days().into_iter().map(|(_, m, _)| m.word).collect();
    assert_eq!(
        phrase.join(" "),
        "light returns after the longest winter night"
    );
}

#[test]
fn examples_show_a_guessable_answer_and_alternates() {
    let days = days();
    assert!(
        days.iter()
            .flat_map(|(_, m, _)| &m.answers)
            .any(|a| guessable(a).contains(&GuessableReason::DigitsOnly)),
        "one sample should trigger the guessable warning"
    );
    assert!(days.iter().any(|(_, m, _)| m.answers.len() >= 2));
}

#[test]
fn examples_referenced_images_exist() {
    for (n, _, puzzle) in days() {
        let dir = examples().join(format!("day{n:02}"));
        for target in puzzle.split("](").skip(1) {
            let file = target.split(')').next().unwrap();
            assert!(
                dir.join(file).is_file(),
                "day{n:02} references missing {file}"
            );
        }
    }
}

#[test]
fn examples_meta_day_answer_follows_from_earlier_answers() {
    // Letters named in day 7's puzzle, taken from the first accepted answers.
    let first = |n: usize| normalize(&days()[n - 1].1.answers[0]);
    let pick = |n: usize, i: usize| first(n).chars().nth(i - 1).unwrap();
    let spelled: String = [pick(6, 1), pick(5, 2), pick(1, 5), pick(3, 1), pick(4, 1)]
        .into_iter()
        .collect();
    assert_eq!(spelled, normalize(&days()[6].1.answers[0]));
}

#[test]
fn examples_final_payload_parses() {
    // The typed payload lands with `keeper seal`; until then check the shape.
    let text = fs::read_to_string(examples().join("final/final.yaml")).unwrap();
    let value: serde_yaml::Value = serde_yaml::from_str(&text).unwrap();
    assert!(value["title"].as_str().is_some_and(|t| !t.is_empty()));
    assert!(value["message"].as_str().is_some_and(|m| !m.is_empty()));
}
