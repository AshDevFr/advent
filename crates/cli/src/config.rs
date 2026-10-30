//! Loading an event repo's configuration and private keys.
//!
//! `advent.yaml` is layered with `ADVENT_*` environment variables through
//! figment. Private keys never live in `advent.yaml`: the editors key comes
//! from `ADVENT_EDITORS_KEY` or the gitignored `editors.key`, the release key
//! only from `ADVENT_RELEASE_KEY`, since it belongs in CI secrets alone.

// Used by the commands that need a repo; init creates one instead.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use advent_core::model::RepoConfig;
use advent_core::wrap::{self, x25519};
use anyhow::{Context, bail};
use figment::Figment;
use figment::providers::{Env, Format, Yaml};
use serde::Deserialize;

pub const CONFIG_FILE: &str = "advent.yaml";
pub const EDITORS_KEY_FILE: &str = "editors.key";

/// The nearest directory at or above `start` holding `advent.yaml`.
pub fn find_repo_root(start: &Path) -> anyhow::Result<PathBuf> {
    let start = start
        .canonicalize()
        .with_context(|| format!("cannot resolve {}", start.display()))?;
    start
        .ancestors()
        .find(|dir| dir.join(CONFIG_FILE).is_file())
        .map(Path::to_path_buf)
        .with_context(|| {
            format!(
                "no {CONFIG_FILE} in {} or any parent: not inside an event repo (create one with `advent init`)",
                start.display()
            )
        })
}

/// `advent.yaml`, with `ADVENT_ENGINE` able to override the pinned version.
pub fn load_repo_config(root: &Path) -> anyhow::Result<RepoConfig> {
    let config: RepoConfig = Figment::new()
        .merge(Yaml::file_exact(root.join(CONFIG_FILE)))
        .merge(Env::prefixed("ADVENT_").only(&["engine"]))
        .extract()
        .with_context(|| format!("reading {}", root.join(CONFIG_FILE).display()))?;
    config.validate()?;
    Ok(config)
}

#[derive(Deserialize, Default)]
struct KeyEnv {
    editors_key: Option<String>,
    release_key: Option<String>,
}

fn key_env() -> anyhow::Result<KeyEnv> {
    Figment::new()
        .merge(Env::prefixed("ADVENT_").only(&["editors_key", "release_key"]))
        .extract()
        .context("reading ADVENT_EDITORS_KEY / ADVENT_RELEASE_KEY")
}

/// The editors identities: `ADVENT_EDITORS_KEY` if set, else `editors.key`.
pub fn editors_identities(root: &Path) -> anyhow::Result<Vec<x25519::Identity>> {
    let text = match key_env()?.editors_key {
        Some(key) => key,
        None => {
            let path = root.join(EDITORS_KEY_FILE);
            if !path.is_file() {
                bail!(
                    "no editors key: set ADVENT_EDITORS_KEY or put {EDITORS_KEY_FILE} at the repo root"
                );
            }
            std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?
        }
    };
    Ok(wrap::parse_identities(&text)?)
}

/// The release identities, from `ADVENT_RELEASE_KEY` only.
pub fn release_identities() -> anyhow::Result<Vec<x25519::Identity>> {
    match key_env()?.release_key {
        Some(key) => Ok(wrap::parse_identities(&key)?),
        None => bail!("no release key: set ADVENT_RELEASE_KEY (it belongs in CI secrets)"),
    }
}

#[cfg(test)]
// figment's Jail closures return figment::Error, which is large by design.
#[allow(clippy::result_large_err)]
mod tests {
    use super::*;
    use figment::Jail;

    fn identity_count(result: anyhow::Result<Vec<x25519::Identity>>) -> usize {
        result.map(|ids| ids.len()).unwrap_or(0)
    }

    #[test]
    fn config_env_editors_key_wins_over_file() {
        let (file_secret, _) = wrap::generate_identity();
        let (env_secret, env_public) = wrap::generate_identity();
        Jail::expect_with(|jail| {
            jail.create_file(EDITORS_KEY_FILE, &file_secret)?;
            jail.set_env("ADVENT_EDITORS_KEY", &env_secret);
            let ids = editors_identities(jail.directory()).unwrap();
            assert_eq!(ids.len(), 1);
            assert_eq!(ids[0].to_public().to_string(), env_public);
            Ok(())
        });
    }

    #[test]
    fn config_editors_key_file_is_used_without_env() {
        let (secret, public) = wrap::generate_identity();
        Jail::expect_with(|jail| {
            jail.create_file(EDITORS_KEY_FILE, &format!("# team key\n{secret}\n"))?;
            let ids = editors_identities(jail.directory()).unwrap();
            assert_eq!(ids[0].to_public().to_string(), public);
            Ok(())
        });
    }

    #[test]
    fn config_missing_keys_explain_where_to_put_them() {
        Jail::expect_with(|jail| {
            // Identities are secrets without Debug, so no unwrap_err.
            let Err(err) = editors_identities(jail.directory()) else {
                panic!("expected an error without any editors key");
            };
            assert!(err.to_string().contains("ADVENT_EDITORS_KEY"));
            assert_eq!(identity_count(release_identities()), 0);
            Ok(())
        });
    }

    #[test]
    fn config_env_overrides_engine_version() {
        Jail::expect_with(|jail| {
            jail.create_file(
                CONFIG_FILE,
                "engine: 0.1.0\nkeys:\n  editors: [age1a]\n  release: [age1b]\n",
            )?;
            jail.set_env("ADVENT_ENGINE", "0.2.0");
            assert_eq!(load_repo_config(jail.directory()).unwrap().engine, "0.2.0");
            Ok(())
        });
    }

    #[test]
    fn config_repo_root_found_from_a_subfolder() {
        Jail::expect_with(|jail| {
            jail.create_file(CONFIG_FILE, "")?;
            let day = jail.directory().join("events/2026/day01");
            std::fs::create_dir_all(&day).unwrap();
            let root = find_repo_root(&day).unwrap();
            assert_eq!(root, jail.directory().canonicalize().unwrap());
            Ok(())
        });
    }
}
