//! Every configuration field has an entry in the docs reference.
//!
//! Each config type is built here with every optional field set and
//! serialized, which yields the field names authors actually write. The
//! struct literals are exhaustive on purpose: adding a field to a config
//! type fails to compile until it is added here, and then this test fails
//! until the field is documented as a `### \`name\`` entry on its page.

use std::collections::{BTreeMap, BTreeSet};

use advent_core::model::{
    Brand, DayMeta, Event, FinalPayload, Hint, Keys, Notice, Palettes, RepoConfig, Story, Theme,
};
use serde::Serialize;

/// Palettes with no tokens: their keys are colour names, not fields.
fn palettes() -> Palettes {
    Palettes {
        light: BTreeMap::new(),
        dark: BTreeMap::new(),
    }
}

/// Field names of `value`, nested structs and lists included.
fn fields<T: Serialize>(value: &T) -> BTreeSet<String> {
    fn walk(value: &serde_yaml::Value, out: &mut BTreeSet<String>) {
        match value {
            serde_yaml::Value::Mapping(map) => {
                for (key, inner) in map {
                    if let Some(name) = key.as_str() {
                        out.insert(name.to_string());
                    }
                    walk(inner, out);
                }
            }
            serde_yaml::Value::Sequence(items) => items.iter().for_each(|i| walk(i, out)),
            _ => {}
        }
    }
    let mut out = BTreeSet::new();
    walk(&serde_yaml::to_value(value).unwrap(), &mut out);
    out
}

fn reference(page: &str) -> String {
    let path = format!(
        "{}/../../docs/src/content/docs/reference/{page}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn assert_documented(page: &str, names: BTreeSet<String>) {
    let text = reference(page);
    let missing: Vec<&String> = names
        .iter()
        .filter(|name| !text.lines().any(|l| l.trim() == format!("### `{name}`")))
        .collect();
    assert!(
        missing.is_empty(),
        "docs/src/content/docs/reference/{page} has no `### \\`name\\`` entry for: {missing:?}"
    );
}

#[test]
fn reference_documents_advent_yaml() {
    let config = RepoConfig {
        engine: "1.0.0".into(),
        keys: Keys {
            editors: vec!["age1x".into()],
            release: vec!["age1y".into()],
        },
    };
    assert_documented("advent-yaml.md", fields(&config));
}

#[test]
fn reference_documents_brand_yaml() {
    let brand = Brand {
        name: "n".into(),
        logo: Some("logo.svg".into()),
        palette: palettes(),
    };
    assert_documented("brand-yaml.md", fields(&brand));
}

#[test]
fn reference_documents_theme_yaml() {
    let theme = Theme {
        palette: palettes(),
        background: Some("bg.webp".into()),
        doors: Some("doors.webp".into()),
        intro: Some("i".into()),
    };
    assert_documented("theme-yaml.md", fields(&theme));
}

#[test]
fn reference_documents_event_yaml() {
    let event = Event {
        title: "t".into(),
        timezone: chrono_tz::UTC,
        start: "2026-12-01".parse().unwrap(),
        days: 2,
        unlocked: 1,
        every: 1,
        weekdays: false,
        dates: Some(vec!["2026-12-01".parse().unwrap()]),
        hint_after: vec![1, 2],
        notices: vec![Notice {
            text: "n".into(),
            day: Some(1),
            date: Some("2026-12-01".parse().unwrap()),
        }],
        story: Story {
            title: Some("t".into()),
            intro: Some("i".into()),
        },
    };
    assert_documented("event-yaml.md", fields(&event));
}

#[test]
fn reference_documents_meta_yaml() {
    let meta = DayMeta {
        title: "t".into(),
        kind: "teaser".into(),
        answers: vec!["a".into()],
        word: "w".into(),
        hints: vec![Hint {
            text: "h".into(),
            after: Some(0),
        }],
    };
    assert_documented("meta-yaml.md", fields(&meta));
}

#[test]
fn reference_documents_final_yaml() {
    let payload = FinalPayload {
        title: "t".into(),
        message: "m".into(),
        image: Some("i.png".into()),
    };
    assert_documented("final-yaml.md", fields(&payload));
}
