//! `advent serve` settings: defaults, then an optional YAML file, then
//! `ADVENT_*` environment variables (`__` separates nested keys, as in
//! `ADVENT_LIMITS__GUESSES_PER_MINUTE`), then command-line flags.
//!
//! These are deployment settings, kept apart from `advent.yaml`, which is
//! committed in the event repo. Secrets belong in the environment.

use std::path::{Path, PathBuf};

use anyhow::Context;
use figment::Figment;
use figment::providers::{Env, Format, Serialized, Yaml};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    /// Address and port to listen on.
    pub listen: String,
    /// The address players reach the calendar at, for links in emails
    /// (`https://advent.example.com`).
    pub public_url: Option<String>,
    /// Folder for the database.
    pub data: PathBuf,
    /// Send cookies without `Secure`, for trying the server over plain http
    /// on this machine. Never in production.
    pub insecure_cookies: bool,
    /// Take the client address from the last `X-Forwarded-For` entry, when
    /// the server runs behind a reverse proxy that sets it. Without a proxy
    /// this would let clients pick their own address, so it is off by default.
    pub trust_proxy: bool,
    pub limits: Limits,
}

/// Protection for the server itself, always on. Per client address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Limits {
    /// Any request.
    pub requests_per_minute: u32,
    /// Guesses: each one costs a key derivation.
    pub guesses_per_minute: u32,
    /// Sign-in requests: each can send an email.
    pub signin_per_hour: u32,
    /// Answer checks running at once, across all clients.
    pub concurrent_checks: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen: "0.0.0.0:8080".into(),
            public_url: None,
            data: PathBuf::from("data"),
            insecure_cookies: false,
            trust_proxy: false,
            limits: Limits::default(),
        }
    }
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            requests_per_minute: 600,
            guesses_per_minute: 10,
            signin_per_hour: 20,
            concurrent_checks: 4,
        }
    }
}

/// Values from the command line, which win over everything else.
#[derive(Debug, Default, Serialize)]
pub struct Overrides {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub listen: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<PathBuf>,
}

impl ServerConfig {
    pub fn load(file: Option<&Path>, overrides: &Overrides) -> anyhow::Result<Self> {
        let mut figment = Figment::from(Serialized::defaults(ServerConfig::default()));
        if let Some(file) = file {
            figment = figment.merge(Yaml::file_exact(file));
        }
        let config: ServerConfig = figment
            .merge(Env::prefixed("ADVENT_").split("__"))
            .merge(Serialized::defaults(overrides))
            .extract()
            .context("reading the server settings")?;
        if config.limits.concurrent_checks == 0 {
            anyhow::bail!("`limits.concurrent_checks` must be at least 1");
        }
        Ok(config)
    }
}

#[cfg(test)]
// figment's Jail closures return figment::Error, which is large by design.
#[allow(clippy::result_large_err)]
mod tests {
    use super::*;
    use figment::Jail;

    #[test]
    fn serve_config_layers_file_env_and_flags() {
        Jail::expect_with(|jail| {
            jail.create_file(
                "server.yaml",
                "listen: 127.0.0.1:9000\npublic_url: https://advent.acme.com\nlimits:\n  guesses_per_minute: 3\n",
            )?;
            jail.set_env("ADVENT_LIMITS__GUESSES_PER_MINUTE", "5");
            jail.set_env("ADVENT_RELEASE_KEY", "not a setting");
            let config = ServerConfig::load(
                Some(&jail.directory().join("server.yaml")),
                &Overrides::default(),
            )
            .unwrap();
            assert_eq!(config.listen, "127.0.0.1:9000");
            assert_eq!(
                config.public_url.as_deref(),
                Some("https://advent.acme.com")
            );
            assert_eq!(
                config.limits.guesses_per_minute, 5,
                "env wins over the file"
            );
            assert_eq!(config.limits.requests_per_minute, 600, "defaults remain");

            let config = ServerConfig::load(
                Some(&jail.directory().join("server.yaml")),
                &Overrides {
                    listen: Some("127.0.0.1:7000".into()),
                    data: None,
                },
            )
            .unwrap();
            assert_eq!(config.listen, "127.0.0.1:7000", "flags win");
            Ok(())
        });
    }

    #[test]
    fn serve_config_file_must_exist_when_given() {
        assert!(
            ServerConfig::load(
                Some(Path::new("/no/such/server.yaml")),
                &Overrides::default()
            )
            .is_err()
        );
    }
}
