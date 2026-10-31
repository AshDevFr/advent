//! The static site's data contract: the JSON files `advent build` writes and
//! the frontend reads, and the theme CSS generated from brand and theme.
//! Paths (see PRD 8.1):
//!
//! - `/site.json`, `/theme.css`
//! - `/<event>/event.json`, `/<event>/theme.css`
//! - `/<event>/api/day/N.json`, `/<event>/api/day/N/<asset>`,
//!   `/<event>/api/day/N/hint.json`, `/<event>/api/day/N/locks.json`
//! - `/<event>/api/final.json`

use std::collections::BTreeMap;
use std::fmt::Write;

use serde::{Deserialize, Serialize};

use crate::model::{Brand, Scheme, Theme, resolve_scheme};
use crate::primitives::WordLock;

/// `/site.json`: who runs the calendar and which events it has.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteIndex {
    pub brand: SiteBrand,
    pub events: Vec<EventInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteBrand {
    pub name: String,
    /// Site path of the logo, e.g. `/brand/logo.svg`.
    pub logo: Option<String>,
}

/// One event, in `/site.json` and as `/<event>/event.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventInfo {
    pub id: String,
    pub title: String,
    pub timezone: String,
    pub start: String,
    pub days: u32,
    /// The edition's display headline, if the story sets one.
    pub headline: Option<String>,
    pub intro: Option<String>,
    /// When day 1 opens (RFC 3339, UTC), or null when it is already open
    /// regardless of the date. Drives the coming-soon countdown.
    pub opens_at: Option<String>,
    /// Opening date of every day (YYYY-MM-DD), released or not, so locked
    /// doors can say when they open.
    pub dates: Vec<String>,
    /// Days published in this build, in order.
    pub released: Vec<u32>,
    /// Type label of each released day (and of no other), for the door cards.
    pub types: BTreeMap<u32, String>,
    /// Announcements published so far, in the order of `event.yaml`.
    pub notices: Vec<SiteNotice>,
    /// Site paths of theme decoration, if the event's theme sets them.
    pub background: Option<String>,
    pub doors: Option<String>,
}

/// A published notice. `id` derives from the text, so players who dismissed
/// a notice see it again if its text is edited.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteNotice {
    pub id: String,
    pub text: String,
}

impl SiteNotice {
    pub fn new(text: &str) -> Self {
        use sha2::{Digest, Sha256};
        let digest = Sha256::digest(text.as_bytes());
        let id = digest[..6].iter().map(|b| format!("{b:02x}")).collect();
        SiteNotice {
            id,
            text: text.to_string(),
        }
    }
}

/// `/<event>/api/day/N.json`: a released day, in plain form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DayPage {
    pub day: u32,
    pub title: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub markdown: String,
}

/// `/<event>/api/day/N/hint.json`: the hints due so far.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HintsDue {
    pub hints: Vec<String>,
}

/// `/<event>/api/day/N/locks.json`: the day's word locks, still encrypted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Locks {
    pub word_locks: Vec<WordLock>,
}

/// Engine defaults; the names match `web/src/index.css` and the palette
/// keys authors use in `brand.yaml` and `theme.yaml`.
pub fn default_tokens(scheme: Scheme) -> BTreeMap<String, String> {
    let pairs: &[(&str, &str)] = match scheme {
        Scheme::Light => &[
            ("primary", "#a8621a"),
            ("on-primary", "#ffffff"),
            ("surface", "#f6f3ec"),
            ("surface-raised", "#ffffff"),
            ("text", "#1b1d24"),
            ("text-muted", "#5d6170"),
            ("accent", "#8a4f12"),
            ("border", "#e2dccf"),
        ],
        Scheme::Dark => &[
            ("primary", "#e3a54f"),
            ("on-primary", "#1c1408"),
            ("surface", "#10131a"),
            ("surface-raised", "#161b26"),
            ("text", "#e9ecf3"),
            ("text-muted", "#98a1b3"),
            ("accent", "#f0b65a"),
            ("border", "#262d3b"),
        ],
    };
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ThemeError {
    #[error("palette key {0:?} must use only a-z, 0-9 and -")]
    BadKey(String),
    #[error("palette value {value:?} for {key:?} contains a character CSS would misread")]
    BadValue { key: String, value: String },
}

fn rule(tokens: &BTreeMap<String, String>) -> Result<String, ThemeError> {
    let mut rule = String::new();
    for (key, value) in tokens {
        let key_ok = !key.is_empty()
            && key
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
        if !key_ok {
            return Err(ThemeError::BadKey(key.clone()));
        }
        if value.contains([';', '{', '}', '<', '>', '\n', '\\']) {
            return Err(ThemeError::BadValue {
                key: key.clone(),
                value: value.clone(),
            });
        }
        let _ = writeln!(rule, "  --{key}: {value};");
    }
    Ok(rule)
}

/// CSS custom properties for both schemes, layered defaults, brand, theme.
/// Light applies by default; dark applies when the system prefers it unless
/// the player picked light, and whenever the player picked dark (the
/// frontend sets `data-theme` on `<html>`). Keys and values are checked so a
/// palette cannot break out of its rule.
pub fn theme_css(brand: Option<&Brand>, theme: Option<&Theme>) -> Result<String, ThemeError> {
    let light = rule(&resolve_scheme(
        Scheme::Light,
        &default_tokens(Scheme::Light),
        brand,
        theme,
    ))?;
    let dark = rule(&resolve_scheme(
        Scheme::Dark,
        &default_tokens(Scheme::Dark),
        brand,
        theme,
    ))?;
    Ok(format!(
        ":root {{\n{light}}}\n\
         @media (prefers-color-scheme: dark) {{\n:root:not([data-theme=\"light\"]) {{\n{dark}}}\n}}\n\
         :root[data-theme=\"dark\"] {{\n{dark}}}\n"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Palettes;

    fn map(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn brand(light: &[(&str, &str)], dark: &[(&str, &str)]) -> Brand {
        Brand {
            name: "Acme".into(),
            logo: None,
            palette: Palettes {
                light: map(light),
                dark: map(dark),
            },
        }
    }

    #[test]
    fn site_theme_css_has_both_palettes_and_honours_the_toggle() {
        let b = brand(&[("primary", "#0055aa")], &[("primary", "#99ccff")]);
        let css = theme_css(Some(&b), None).unwrap();
        let (light, rest) = css
            .split_once("@media (prefers-color-scheme: dark)")
            .unwrap();
        let (system_dark, chosen_dark) = rest.split_once(":root[data-theme=\"dark\"]").unwrap();
        assert!(light.contains("--primary: #0055aa;"));
        assert!(
            light.contains("--surface: #f6f3ec;"),
            "defaults fill the gaps"
        );
        assert!(system_dark.contains(":root:not([data-theme=\"light\"])"));
        assert!(system_dark.contains("--primary: #99ccff;"));
        assert!(chosen_dark.contains("--primary: #99ccff;"));
        assert!(chosen_dark.contains("--surface: #10131a;"));
    }

    #[test]
    fn site_theme_css_theme_overrides_brand() {
        let theme = Theme {
            palette: Palettes {
                light: map(&[("primary", "#112233")]),
                dark: BTreeMap::new(),
            },
            ..Theme::default()
        };
        let css = theme_css(Some(&brand(&[("primary", "#0055aa")], &[])), Some(&theme)).unwrap();
        assert!(css.contains("--primary: #112233;"));
        assert!(!css.contains("#0055aa"));
    }

    #[test]
    fn site_theme_css_rejects_injection() {
        let bad = brand(&[("primary", "red; } body { display: none")], &[]);
        assert!(matches!(
            theme_css(Some(&bad), None),
            Err(ThemeError::BadValue { .. })
        ));
        let bad_key = brand(&[], &[("Primary Colour", "red")]);
        assert!(matches!(
            theme_css(Some(&bad_key), None),
            Err(ThemeError::BadKey(_))
        ));
    }

    #[test]
    fn site_default_tokens_match_the_frontend_stylesheet() {
        // web/src/index.css carries the same defaults so the dev server and
        // a build without theme.css look identical; keep them in step.
        let css = include_str!("../../../web/src/index.css");
        let (light, dark) = css
            .split_once("@media (prefers-color-scheme: dark)")
            .unwrap();
        for (scheme, block) in [(Scheme::Light, light), (Scheme::Dark, dark)] {
            for (key, value) in default_tokens(scheme) {
                assert!(
                    block.contains(&format!("--{key}: {value};")),
                    "index.css {scheme:?} block lacks --{key}: {value}"
                );
            }
        }
    }

    #[test]
    fn site_default_token_names_match_both_schemes() {
        let light: Vec<_> = default_tokens(Scheme::Light).into_keys().collect();
        let dark: Vec<_> = default_tokens(Scheme::Dark).into_keys().collect();
        assert_eq!(light, dark);
    }
}
