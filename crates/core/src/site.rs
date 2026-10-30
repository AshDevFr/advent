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
    pub intro: Option<String>,
    /// When day 1 opens (RFC 3339, UTC), or null when it is already open
    /// regardless of the date. Drives the coming-soon countdown.
    pub opens_at: Option<String>,
    /// Days published in this build, in order.
    pub released: Vec<u32>,
    /// Site paths of theme decoration, if the event's theme sets them.
    pub background: Option<String>,
    pub doors: Option<String>,
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
            ("primary", "#b3261e"),
            ("on-primary", "#ffffff"),
            ("surface", "#fffaf3"),
            ("surface-raised", "#ffffff"),
            ("text", "#1f1b16"),
            ("text-muted", "#5c544b"),
            ("accent", "#2e7d32"),
            ("border", "#e3dccf"),
        ],
        Scheme::Dark => &[
            ("primary", "#ffb4a9"),
            ("on-primary", "#561e18"),
            ("surface", "#1a1612"),
            ("surface-raised", "#26211c"),
            ("text", "#efe6dc"),
            ("text-muted", "#b8ada1"),
            ("accent", "#8fd694"),
            ("border", "#3d362f"),
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

/// CSS custom properties for both schemes, layered defaults, brand, theme.
/// Keys and values are checked so a palette cannot break out of its rule.
pub fn theme_css(brand: Option<&Brand>, theme: Option<&Theme>) -> Result<String, ThemeError> {
    let mut css = String::new();
    for scheme in [Scheme::Light, Scheme::Dark] {
        let tokens = resolve_scheme(scheme, &default_tokens(scheme), brand, theme);
        let mut rule = String::new();
        for (key, value) in &tokens {
            if key.is_empty()
                || !key
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            {
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
        match scheme {
            Scheme::Light => {
                let _ = write!(css, ":root {{\n{rule}}}\n");
            }
            Scheme::Dark => {
                let _ = write!(
                    css,
                    "@media (prefers-color-scheme: dark) {{\n:root {{\n{rule}}}\n}}\n"
                );
            }
        }
    }
    Ok(css)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn brand(palette: &[(&str, &str)]) -> Brand {
        Brand {
            name: "Acme".into(),
            logo: None,
            palette: palette
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            palette_dark: BTreeMap::new(),
        }
    }

    #[test]
    fn site_theme_css_layers_brand_over_defaults() {
        let css = theme_css(Some(&brand(&[("primary", "#0055aa")])), None).unwrap();
        let (light, dark) = css.split_once("@media").unwrap();
        assert!(light.contains("--primary: #0055aa;"));
        assert!(light.contains("--surface: #fffaf3;"));
        assert!(dark.contains("--primary: #ffb4a9;"));
    }

    #[test]
    fn site_theme_css_theme_overrides_brand() {
        let theme = Theme {
            palette: BTreeMap::from([("primary".to_string(), "#112233".to_string())]),
            ..Theme::default()
        };
        let css = theme_css(Some(&brand(&[("primary", "#0055aa")])), Some(&theme)).unwrap();
        assert!(css.contains("--primary: #112233;"));
        assert!(!css.contains("#0055aa"));
    }

    #[test]
    fn site_theme_css_rejects_injection() {
        let bad = brand(&[("primary", "red; } body { display: none")]);
        assert!(matches!(
            theme_css(Some(&bad), None),
            Err(ThemeError::BadValue { .. })
        ));
        let bad_key = brand(&[("Primary Colour", "red")]);
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
