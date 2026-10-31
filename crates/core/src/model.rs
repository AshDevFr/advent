//! Typed content of an event repo: `advent.yaml`, `brand/brand.yaml`, and
//! per event `event.yaml`, optional `theme.yaml`, and each day's `meta.yaml`.
//! These are data files, parsed with plain serde; validation that serde
//! cannot express (counts, ranges) happens in the `from_yaml` constructors.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ModelError {
    #[error("invalid YAML: {0}")]
    Yaml(String),
    #[error("`{field}`: {reason}")]
    Invalid { field: &'static str, reason: String },
}

fn yaml<T: for<'de> Deserialize<'de>>(text: &str) -> Result<T, ModelError> {
    serde_yaml::from_str(text).map_err(|e| ModelError::Yaml(e.to_string()))
}

fn invalid(field: &'static str, reason: impl Into<String>) -> ModelError {
    ModelError::Invalid {
        field,
        reason: reason.into(),
    }
}

/// `advent.yaml` at the root of an event repo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepoConfig {
    /// Engine version the repo's CI downloads and builds with.
    pub engine: String,
    pub keys: Keys,
}

/// Public halves of the two key pairs, as lists so keys can be rotated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Keys {
    pub editors: Vec<String>,
    pub release: Vec<String>,
}

impl RepoConfig {
    pub fn from_yaml(text: &str) -> Result<Self, ModelError> {
        let config: Self = yaml(text)?;
        config.validate()?;
        Ok(config)
    }

    /// Checks for a config built some other way (the CLI layers env vars
    /// over the file with figment).
    pub fn validate(&self) -> Result<(), ModelError> {
        if self.keys.editors.is_empty() {
            return Err(invalid("keys.editors", "needs at least one public key"));
        }
        if self.keys.release.is_empty() {
            return Err(invalid("keys.release", "needs at least one public key"));
        }
        Ok(())
    }
}

/// `events/<id>/event.yaml`. The event id is the folder name, not a field,
/// so the two cannot disagree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub title: String,
    pub timezone: Tz,
    /// Date of day 1, released at 00:00 in `timezone`.
    pub start: NaiveDate,
    #[serde(default = "default_days")]
    pub days: u32,
    /// Days 1 to `unlocked` are open regardless of the date (the public demo
    /// opens every day this way).
    #[serde(default)]
    pub unlocked: u32,
    #[serde(default)]
    pub story: Story,
}

fn default_days() -> u32 {
    24
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Story {
    /// Display headline for the edition (e.g. "The Longest Night Hunt");
    /// the event title is used when absent.
    pub title: Option<String>,
    pub intro: Option<String>,
}

impl Event {
    pub fn from_yaml(text: &str) -> Result<Self, ModelError> {
        let event: Self = yaml(text)?;
        if event.days == 0 {
            return Err(invalid("days", "must be at least 1"));
        }
        if event.unlocked > event.days {
            return Err(invalid(
                "unlocked",
                format!(
                    "is {} but the event has {} days",
                    event.unlocked, event.days
                ),
            ));
        }
        Ok(event)
    }
}

/// A day's `meta.yaml`. Plaintext: never committed, only sealed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DayMeta {
    pub title: String,
    /// A label only (`teaser`, `code`, ...); the engine treats all types alike.
    #[serde(rename = "type")]
    pub kind: String,
    /// Accepted answers; the word is locked once per answer.
    pub answers: Vec<String>,
    pub word: String,
    pub hints: Vec<String>,
}

impl DayMeta {
    pub fn from_yaml(text: &str) -> Result<Self, ModelError> {
        let meta: Self = yaml(text)?;
        if meta.answers.is_empty() {
            return Err(invalid("answers", "needs at least one accepted answer"));
        }
        if !(1..=2).contains(&meta.hints.len()) {
            return Err(invalid(
                "hints",
                format!("needs 1 or 2 hints, found {}", meta.hints.len()),
            ));
        }
        Ok(meta)
    }
}

/// `events/<id>/final/final.yaml`: the certificate the keeper writes.
/// Plaintext, never committed; sealed with the key built from every word.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinalPayload {
    pub title: String,
    pub message: String,
    /// Optional image file next to `final.yaml`.
    pub image: Option<String>,
}

impl FinalPayload {
    pub fn from_yaml(text: &str) -> Result<Self, ModelError> {
        let payload: Self = yaml(text)?;
        if payload.title.trim().is_empty() {
            return Err(invalid("title", "must not be empty"));
        }
        Ok(payload)
    }
}

/// `brand/brand.yaml`: who runs the event repo. Shared by every event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Brand {
    pub name: String,
    pub logo: Option<String>,
    #[serde(default)]
    pub palette: Palettes,
}

/// Colour tokens for each scheme. The site follows the player's system
/// setting unless they pick one, so both are always in play; a key missing
/// from one scheme keeps the layer below for that scheme only.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Palettes {
    #[serde(default)]
    pub light: BTreeMap<String, String>,
    #[serde(default)]
    pub dark: BTreeMap<String, String>,
}

impl Palettes {
    pub fn get(&self, scheme: Scheme) -> &BTreeMap<String, String> {
        match scheme {
            Scheme::Light => &self.light,
            Scheme::Dark => &self.dark,
        }
    }
}

impl Brand {
    pub fn from_yaml(text: &str) -> Result<Self, ModelError> {
        yaml(text)
    }
}

/// `events/<id>/theme.yaml`: one edition's look, layered over the brand.
/// It has no `name` or `logo` field, so a theme cannot rebrand the event.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Theme {
    #[serde(default)]
    pub palette: Palettes,
    pub background: Option<String>,
    pub doors: Option<String>,
    pub intro: Option<String>,
}

impl Theme {
    pub fn from_yaml(text: &str) -> Result<Self, ModelError> {
        yaml(text)
    }
}

/// Colour scheme a set of tokens is resolved for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    Light,
    Dark,
}

/// Merge palette tokens for one scheme: engine defaults, then the brand,
/// then the theme, later layers winning. A missing layer is skipped, and
/// each scheme reads only its own palette.
pub fn resolve_scheme(
    scheme: Scheme,
    defaults: &BTreeMap<String, String>,
    brand: Option<&Brand>,
    theme: Option<&Theme>,
) -> BTreeMap<String, String> {
    let mut tokens = defaults.clone();
    for palettes in [brand.map(|b| &b.palette), theme.map(|t| &t.palette)]
        .into_iter()
        .flatten()
    {
        tokens.extend(palettes.get(scheme).clone());
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_parses_each_file_type() {
        let repo = RepoConfig::from_yaml(
            "engine: 0.1.0\nkeys:\n  editors: [age1aaa]\n  release: [age1bbb, age1ccc]\n",
        )
        .unwrap();
        assert_eq!(repo.keys.release, ["age1bbb", "age1ccc"]);

        let event = Event::from_yaml(
            "title: Advent 2026\ntimezone: Europe/Paris\nstart: 2026-12-01\nstory:\n  intro: Mission control...\n",
        )
        .unwrap();
        assert_eq!(event.days, 24);
        assert_eq!(event.unlocked, 0);
        assert_eq!(event.timezone, chrono_tz::Europe::Paris);
        assert_eq!(event.story.intro.as_deref(), Some("Mission control..."));

        let day = DayMeta::from_yaml(
            "title: Capitals\ntype: teaser\nanswers: [Paris, Lutece]\nword: light\nhints: [Think big.]\n",
        )
        .unwrap();
        assert_eq!(day.kind, "teaser");
        assert_eq!(day.answers.len(), 2);

        let brand = Brand::from_yaml(
            "name: Acme Advent\nlogo: logo.svg\npalette:\n  light:\n    primary: '#05a'\n  dark:\n    primary: '#8cf'\n",
        )
        .unwrap();
        assert_eq!(brand.palette.light["primary"], "#05a");
        assert_eq!(brand.palette.dark["primary"], "#8cf");

        let theme = Theme::from_yaml("background: stars.webp\n").unwrap();
        assert_eq!(theme.background.as_deref(), Some("stars.webp"));
    }

    #[test]
    fn model_theme_cannot_set_name_or_logo() {
        assert!(Theme::from_yaml("name: X\n").is_err());
        assert!(Theme::from_yaml("logo: other.svg\n").is_err());
    }

    #[test]
    fn model_tokens_layer_defaults_brand_theme_per_scheme() {
        let map = |pairs: &[(&str, &str)]| -> BTreeMap<String, String> {
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect()
        };
        let defaults = map(&[("primary", "#000"), ("surface", "#fff")]);
        let brand = Brand {
            name: "Acme".into(),
            logo: None,
            palette: Palettes {
                light: map(&[("primary", "#05a")]),
                dark: map(&[("primary", "#8cf")]),
            },
        };
        let theme = Theme {
            palette: Palettes {
                light: map(&[("surface", "#0b1026")]),
                dark: BTreeMap::new(),
            },
            ..Theme::default()
        };

        let light = resolve_scheme(Scheme::Light, &defaults, Some(&brand), Some(&theme));
        assert_eq!(light["primary"], "#05a");
        assert_eq!(light["surface"], "#0b1026");

        let dark = resolve_scheme(Scheme::Dark, &defaults, Some(&brand), Some(&theme));
        assert_eq!(dark["primary"], "#8cf");
        assert_eq!(
            dark["surface"], "#fff",
            "the theme set surface for light only"
        );

        let no_theme = resolve_scheme(Scheme::Light, &defaults, Some(&brand), None);
        assert_eq!(no_theme["surface"], "#fff");
    }

    #[test]
    fn model_palette_needs_the_light_and_dark_shape() {
        assert!(Brand::from_yaml("name: X\npalette:\n  primary: '#000'\n").is_err());
    }

    #[test]
    fn model_day_needs_answers_and_one_or_two_hints() {
        let no_answers =
            DayMeta::from_yaml("title: t\ntype: code\nanswers: []\nword: w\nhints: [h]\n");
        assert!(matches!(
            no_answers,
            Err(ModelError::Invalid {
                field: "answers",
                ..
            })
        ));
        let three_hints =
            DayMeta::from_yaml("title: t\ntype: code\nanswers: [a]\nword: w\nhints: [a, b, c]\n");
        assert!(matches!(
            three_hints,
            Err(ModelError::Invalid { field: "hints", .. })
        ));
    }

    #[test]
    fn model_unlocked_cannot_exceed_days() {
        let result =
            Event::from_yaml("title: t\ntimezone: UTC\nstart: 2026-12-01\ndays: 7\nunlocked: 8\n");
        assert!(matches!(
            result,
            Err(ModelError::Invalid {
                field: "unlocked",
                ..
            })
        ));
    }

    #[test]
    fn model_repo_config_needs_keys() {
        let result =
            RepoConfig::from_yaml("engine: 0.1.0\nkeys:\n  editors: []\n  release: [age1x]\n");
        assert!(matches!(
            result,
            Err(ModelError::Invalid {
                field: "keys.editors",
                ..
            })
        ));
    }
}
