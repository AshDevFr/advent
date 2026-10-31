//! Typed content of an event repo: `advent.yaml`, `brand/brand.yaml`, and
//! per event `event.yaml`, optional `theme.yaml`, and each day's `meta.yaml`.
//! These are data files, parsed with plain serde; validation that serde
//! cannot express (counts, ranges) happens in the `from_yaml` constructors.

use std::collections::BTreeMap;

use chrono::{Datelike, Days, NaiveDate, Weekday};
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
    /// A door every `every` days (counting only workdays with `weekdays`).
    #[serde(default = "default_every")]
    pub every: u32,
    /// Only Monday to Friday; `start` must then be a weekday.
    #[serde(default)]
    pub weekdays: bool,
    /// Explicit date of each day, instead of `every` / `weekdays`. Its
    /// length is `days`, it is strictly increasing, and it begins at `start`.
    pub dates: Option<Vec<NaiveDate>>,
    /// Default timing of each day's hints: days after the door opens, at
    /// local midnight (0 = with the door). One entry per possible hint.
    #[serde(default = "default_hint_after")]
    pub hint_after: Vec<u32>,
    /// Announcements shown at the top of the calendar.
    #[serde(default)]
    pub notices: Vec<Notice>,
    #[serde(default)]
    pub story: Story,
    /// Server mode only: how many guesses a player may make per day within
    /// a window that refills. Without it, guesses are unlimited.
    #[serde(default)]
    pub attempts: Option<Attempts>,
}

/// A guess limit that refills: at most `max` guesses per player and day in
/// any window of `per`, counted from the first guess of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Attempts {
    pub max: u32,
    pub per: Window,
}

/// A length of time written as a number and a unit: `30m`, `1h`, `2d`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Window {
    seconds: u64,
}

impl Window {
    pub fn seconds(self) -> u64 {
        self.seconds
    }

    pub fn duration(self) -> chrono::Duration {
        chrono::Duration::seconds(self.seconds as i64)
    }
}

impl std::str::FromStr for Window {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.trim();
        let (number, unit) = text.split_at(text.len().saturating_sub(1));
        let scale = match unit {
            "m" => 60,
            "h" => 3600,
            "d" => 86_400,
            _ => {
                return Err(format!(
                    "{text:?} needs a unit: m, h or d (as in 30m, 1h, 1d)"
                ));
            }
        };
        match number.parse::<u64>() {
            Ok(n) if n > 0 => Ok(Window { seconds: n * scale }),
            _ => Err(format!(
                "{text:?} must be a positive number with a unit, as in 1h"
            )),
        }
    }
}

impl std::fmt::Display for Window {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self.seconds;
        if s.is_multiple_of(86_400) {
            write!(f, "{}d", s / 86_400)
        } else if s.is_multiple_of(3600) {
            write!(f, "{}h", s / 3600)
        } else {
            write!(f, "{}m", s / 60)
        }
    }
}

impl Serialize for Window {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Window {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let text = String::deserialize(d)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

/// An announcement on the calendar page. With `day` it appears once that
/// day opens, with `date` from local midnight of that date, and with
/// neither from the start; it then stays until removed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Notice {
    pub text: String,
    pub day: Option<u32>,
    pub date: Option<NaiveDate>,
}

fn default_every() -> u32 {
    1
}

fn default_hint_after() -> Vec<u32> {
    vec![1, 2]
}

/// The most hints a day can have (PRD: 1 to 2).
pub const MAX_HINTS: usize = 2;

fn is_weekend(date: NaiveDate) -> bool {
    matches!(date.weekday(), Weekday::Sat | Weekday::Sun)
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
        event.validate_schedule()?;
        event.validate_notices()?;
        if event.attempts.is_some_and(|a| a.max == 0) {
            return Err(invalid(
                "attempts",
                "`max` must be at least 1 (remove `attempts` for no limit)",
            ));
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

    /// When each of a day's hints is due, in days after its door opens:
    /// the hint's own `after` if set, else the event default. Hints are
    /// revealed in order, so the result must not decrease.
    pub fn hint_days(&self, overrides: &[Option<u32>]) -> Result<Vec<u32>, ModelError> {
        let days: Vec<u32> = overrides
            .iter()
            .enumerate()
            .map(|(i, after)| after.unwrap_or(self.hint_after[i.min(self.hint_after.len() - 1)]))
            .collect();
        if days.windows(2).any(|w| w[1] < w[0]) {
            return Err(invalid(
                "hints",
                format!(
                    "come out of order ({days:?} days after the door); hints are revealed in order, so a later hint cannot be due sooner"
                ),
            ));
        }
        Ok(days)
    }

    fn validate_notices(&self) -> Result<(), ModelError> {
        for notice in &self.notices {
            if notice.text.trim().is_empty() {
                return Err(invalid("notices", "a notice has no text"));
            }
            if notice.day.is_some() && notice.date.is_some() {
                return Err(invalid(
                    "notices",
                    format!("{:?} sets both `day` and `date`; use one", notice.text),
                ));
            }
            if let Some(day) = notice.day
                && (day == 0 || day > self.days)
            {
                return Err(invalid(
                    "notices",
                    format!(
                        "{:?} is for day {day}, but the event has {} days",
                        notice.text, self.days
                    ),
                ));
            }
        }
        Ok(())
    }

    fn validate_schedule(&self) -> Result<(), ModelError> {
        if self.hint_after.len() != MAX_HINTS {
            return Err(invalid(
                "hint_after",
                format!(
                    "needs {MAX_HINTS} entries, one per hint, found {}",
                    self.hint_after.len()
                ),
            ));
        }
        if self.hint_after[1] < self.hint_after[0] {
            return Err(invalid(
                "hint_after",
                "must not decrease: hints are revealed in order",
            ));
        }
        if self.every == 0 {
            return Err(invalid("every", "must be at least 1"));
        }
        if let Some(dates) = &self.dates {
            if self.every != 1 || self.weekdays {
                return Err(invalid(
                    "dates",
                    "lists every date already, so it cannot be combined with `every` or `weekdays`",
                ));
            }
            if dates.len() != self.days as usize {
                return Err(invalid(
                    "dates",
                    format!(
                        "has {} dates but the event has {} days",
                        dates.len(),
                        self.days
                    ),
                ));
            }
            if dates.first() != Some(&self.start) {
                return Err(invalid("dates", "must begin with the `start` date"));
            }
            if let Some(pair) = dates.windows(2).find(|w| w[1] <= w[0]) {
                return Err(invalid(
                    "dates",
                    format!(
                        "must be in increasing order ({} is followed by {})",
                        pair[0], pair[1]
                    ),
                ));
            }
        }
        if self.weekdays && is_weekend(self.start) {
            return Err(invalid(
                "start",
                format!(
                    "is a {} but `weekdays` only opens doors Monday to Friday",
                    self.start.weekday()
                ),
            ));
        }
        Ok(())
    }

    /// The date day `n` (1-based) opens. Days outside the event fall back
    /// to the rule, which callers never ask for.
    pub fn day_date(&self, n: u32) -> NaiveDate {
        if let Some(dates) = &self.dates
            && let Some(date) = dates.get(n.saturating_sub(1) as usize)
        {
            return *date;
        }
        let mut date = self.start;
        for _ in 1..n.max(1) {
            for _ in 0..self.every {
                date = date + Days::new(1);
                while self.weekdays && is_weekend(date) {
                    date = date + Days::new(1);
                }
            }
        }
        date
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
    pub hints: Vec<Hint>,
}

/// One hint: plain text (timed by the event's `hint_after`) or
/// `{ text, after }` to set its own delay in days after the door opens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "HintRepr")]
pub struct Hint {
    pub text: String,
    pub after: Option<u32>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum HintRepr {
    Text(String),
    Timed(TimedHint),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimedHint {
    text: String,
    after: Option<u32>,
}

impl From<HintRepr> for Hint {
    fn from(repr: HintRepr) -> Self {
        match repr {
            HintRepr::Text(text) => Hint { text, after: None },
            HintRepr::Timed(t) => Hint {
                text: t.text,
                after: t.after,
            },
        }
    }
}

impl DayMeta {
    pub fn from_yaml(text: &str) -> Result<Self, ModelError> {
        let meta: Self = yaml(text)?;
        if meta.answers.is_empty() {
            return Err(invalid("answers", "needs at least one accepted answer"));
        }
        if !(1..=MAX_HINTS).contains(&meta.hints.len()) {
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
    /// Where `image` goes: `bottom` (under the message) or `top` (in place
    /// of the brand logo).
    #[serde(default)]
    pub image_position: ImagePosition,
    /// A line between the title and the player's name.
    pub subtitle: Option<String>,
    /// A sign-off above the footer.
    pub signature: Option<String>,
    /// Optional image file shown behind the whole certificate.
    pub background: Option<String>,
    /// What else finishing gives: files to download and links, revealed
    /// only once the certificate is unlocked.
    #[serde(default)]
    pub rewards: Vec<RewardSpec>,
}

/// One reward in `final.yaml`: a file next to it, or a link.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RewardSpec {
    pub label: String,
    pub file: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImagePosition {
    Top,
    #[default]
    Bottom,
}

impl FinalPayload {
    pub fn from_yaml(text: &str) -> Result<Self, ModelError> {
        let payload: Self = yaml(text)?;
        if payload.title.trim().is_empty() {
            return Err(invalid("title", "must not be empty"));
        }
        for reward in &payload.rewards {
            if reward.label.trim().is_empty() {
                return Err(invalid("rewards", "a reward has no label"));
            }
            if reward.file.is_some() == reward.url.is_some() {
                return Err(invalid(
                    "rewards",
                    format!("{:?} needs exactly one of `file` or `url`", reward.label),
                ));
            }
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
    /// Image shown behind the calendar, next to `theme.yaml`.
    pub background: Option<String>,
    /// Artwork shown on locked doors, next to `theme.yaml`.
    pub doors: Option<String>,
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
    fn event_attempts_parse_and_validate() {
        let base = "title: T\ntimezone: UTC\nstart: 2026-12-01\n";
        assert_eq!(Event::from_yaml(base).unwrap().attempts, None);

        let event = Event::from_yaml(&format!("{base}attempts:\n  max: 10\n  per: 1h\n")).unwrap();
        let attempts = event.attempts.unwrap();
        assert_eq!(attempts.max, 10);
        assert_eq!(attempts.per.seconds(), 3600);
        assert_eq!(attempts.per.to_string(), "1h");
        assert_eq!("30m".parse::<Window>().unwrap().seconds(), 1800);
        assert_eq!("2d".parse::<Window>().unwrap().to_string(), "2d");

        let zero = Event::from_yaml(&format!("{base}attempts: {{ max: 0, per: 1h }}\n"));
        assert!(matches!(
            zero,
            Err(ModelError::Invalid {
                field: "attempts",
                ..
            })
        ));
        for per in ["1", "0h", "1w", "h", "-1h"] {
            let bad = Event::from_yaml(&format!("{base}attempts: {{ max: 3, per: \"{per}\" }}\n"));
            assert!(bad.is_err(), "{per} should be refused");
        }
    }

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
        assert_eq!(day.hints[0].text, "Think big.");

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
        assert!(
            Theme::from_yaml("intro: x\n").is_err(),
            "the intro lives in event.yaml"
        );
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

    fn dates_of(yaml: &str) -> Vec<String> {
        let event = Event::from_yaml(yaml).unwrap();
        (1..=event.days)
            .map(|n| event.day_date(n).to_string())
            .collect()
    }

    const BASE: &str = "title: t\ntimezone: UTC\nstart: 2026-12-01\ndays: 12\n";

    #[test]
    fn model_days_are_consecutive_by_default() {
        let dates = dates_of(BASE);
        assert_eq!(dates.first().unwrap(), "2026-12-01");
        assert_eq!(dates.last().unwrap(), "2026-12-12");
    }

    #[test]
    fn model_every_spreads_days() {
        let dates = dates_of(&format!("{BASE}every: 2\n"));
        assert_eq!(dates[..3], ["2026-12-01", "2026-12-03", "2026-12-05"]);
        assert_eq!(dates.last().unwrap(), "2026-12-23");
    }

    #[test]
    fn model_weekdays_skip_weekends() {
        // 2026-12-01 is a Tuesday.
        let dates = dates_of(&format!("{BASE}weekdays: true\n"));
        assert_eq!(
            dates,
            [
                "2026-12-01",
                "2026-12-02",
                "2026-12-03",
                "2026-12-04",
                "2026-12-07",
                "2026-12-08",
                "2026-12-09",
                "2026-12-10",
                "2026-12-11",
                "2026-12-14",
                "2026-12-15",
                "2026-12-16"
            ]
        );
        let every_other = dates_of(&format!("{BASE}weekdays: true\nevery: 2\n"));
        assert_eq!(
            every_other[..4],
            ["2026-12-01", "2026-12-03", "2026-12-07", "2026-12-09"]
        );
    }

    #[test]
    fn model_explicit_dates_are_used_as_given() {
        let yaml = "title: t\ntimezone: UTC\nstart: 2026-12-01\ndays: 3\ndates: [2026-12-01, 2026-12-08, 2026-12-24]\n";
        assert_eq!(dates_of(yaml), ["2026-12-01", "2026-12-08", "2026-12-24"]);
    }

    #[test]
    fn model_schedule_options_are_validated() {
        let field = |yaml: &str| match Event::from_yaml(yaml) {
            Err(ModelError::Invalid { field, .. }) => field,
            other => panic!("expected a validation error, got {other:?}"),
        };
        let three = "title: t\ntimezone: UTC\nstart: 2026-12-01\ndays: 3\n";
        assert_eq!(
            field(&format!("{three}dates: [2026-12-01, 2026-12-02]\n")),
            "dates"
        );
        assert_eq!(
            field(&format!(
                "{three}dates: [2026-12-01, 2026-12-05, 2026-12-03]\n"
            )),
            "dates"
        );
        assert_eq!(
            field(&format!(
                "{three}dates: [2026-12-02, 2026-12-03, 2026-12-04]\n"
            )),
            "dates"
        );
        assert_eq!(
            field(&format!(
                "{three}every: 2\ndates: [2026-12-01, 2026-12-02, 2026-12-03]\n"
            )),
            "dates"
        );
        assert_eq!(field(&format!("{three}every: 0\n")), "every");
        // 2026-12-05 is a Saturday.
        let saturday = "title: t\ntimezone: UTC\nstart: 2026-12-05\ndays: 3\nweekdays: true\n";
        assert_eq!(field(saturday), "start");
    }

    #[test]
    fn model_hints_take_plain_text_or_their_own_timing() {
        let meta = DayMeta::from_yaml(
            "title: t\ntype: code\nanswers: [a]\nword: w\nhints:\n  - {text: Look again., after: 0}\n  - Think big.\n",
        )
        .unwrap();
        assert_eq!(meta.hints[0].text, "Look again.");
        assert_eq!(meta.hints[0].after, Some(0));
        assert_eq!(meta.hints[1].text, "Think big.");
        assert_eq!(meta.hints[1].after, None);

        let event = Event::from_yaml(BASE).unwrap();
        assert_eq!(event.hint_after, [1, 2], "default");
        assert_eq!(event.hint_days(&[Some(0), None]).unwrap(), [0, 2]);
        assert_eq!(event.hint_days(&[None]).unwrap(), [1]);
    }

    #[test]
    fn model_hint_timing_is_validated() {
        let field = |yaml: &str| match Event::from_yaml(yaml) {
            Err(ModelError::Invalid { field, .. }) => field,
            other => panic!("expected a validation error, got {other:?}"),
        };
        assert_eq!(field(&format!("{BASE}hint_after: [1]\n")), "hint_after");
        assert_eq!(field(&format!("{BASE}hint_after: [3, 1]\n")), "hint_after");

        let event = Event::from_yaml(&format!("{BASE}hint_after: [0, 3]\n")).unwrap();
        assert!(matches!(
            event.hint_days(&[Some(5), None]),
            Err(ModelError::Invalid { field: "hints", .. })
        ));
    }

    #[test]
    fn model_notices_are_validated() {
        let ok = Event::from_yaml(&format!(
            "{BASE}notices:\n  - text: Welcome\n  - text: Fixed\n    day: 3\n  - text: Last week\n    date: 2026-12-18\n"
        ))
        .unwrap();
        assert_eq!(ok.notices.len(), 3);
        assert_eq!(ok.notices[1].day, Some(3));

        let field = |yaml: &str| match Event::from_yaml(yaml) {
            Err(ModelError::Invalid { field, .. }) => field,
            other => panic!("expected a validation error, got {other:?}"),
        };
        assert_eq!(
            field(&format!(
                "{BASE}notices:\n  - text: x\n    day: 2\n    date: 2026-12-02\n"
            )),
            "notices"
        );
        assert_eq!(
            field(&format!("{BASE}notices:\n  - text: x\n    day: 13\n")),
            "notices"
        );
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
