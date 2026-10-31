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
    pub mail: MailConfig,
    pub admin: AdminConfig,
}

/// How organisers sign in to the admin section.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AdminConfig {
    pub auth: AdminAuth,
    /// The master password, from `ADVENT_ADMIN__PASSWORD`.
    pub password: Option<String>,
    /// The one-time code that creates the first admin in accounts mode,
    /// from `ADVENT_ADMIN__SETUP_CODE`; without it, one is generated and
    /// printed in the log at start.
    pub setup_code: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AdminAuth {
    /// One shared password, no admin accounts.
    #[default]
    Password,
    /// Admins are people who sign in with an email link.
    Accounts,
}

/// How emails are sent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MailConfig {
    pub sender: MailSender,
    /// The sender address, as `Name <address>` or a bare address. Required
    /// for SMTP and Resend.
    pub from: Option<String>,
    /// With the log sender: write each email as a JSON file in this folder
    /// instead of printing it.
    pub log_dir: Option<PathBuf>,
    pub smtp: SmtpConfig,
    pub resend: ResendConfig,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MailSender {
    /// Print emails to the log (or `log_dir`): development and tests.
    #[default]
    Log,
    Smtp,
    Resend,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SmtpConfig {
    pub host: Option<String>,
    pub port: u16,
    pub username: Option<String>,
    /// From `ADVENT_MAIL__SMTP__PASSWORD`.
    pub password: Option<String>,
    pub tls: SmtpTls,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SmtpTls {
    /// Upgrade a plain connection (usually port 587).
    #[default]
    Starttls,
    /// TLS from the start (usually port 465).
    Tls,
    /// No encryption: only for a relay on the same host or private network.
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ResendConfig {
    /// From `ADVENT_MAIL__RESEND__API_KEY`.
    pub api_key: Option<String>,
    /// The Resend API base address.
    pub endpoint: String,
}

impl Default for MailConfig {
    fn default() -> Self {
        Self {
            sender: MailSender::Log,
            from: None,
            log_dir: None,
            smtp: SmtpConfig::default(),
            resend: ResendConfig::default(),
        }
    }
}

impl Default for SmtpConfig {
    fn default() -> Self {
        Self {
            host: None,
            port: 587,
            username: None,
            password: None,
            tls: SmtpTls::Starttls,
        }
    }
}

impl Default for ResendConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            endpoint: "https://api.resend.com".into(),
        }
    }
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
    /// Sign-in links sent to one email address, whoever asks.
    pub links_per_hour: u32,
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
            mail: MailConfig::default(),
            admin: AdminConfig::default(),
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
            links_per_hour: 3,
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
        config.check_mail()?;
        if config.admin.auth == AdminAuth::Password
            && config.admin.password.as_deref().is_none_or(str::is_empty)
        {
            anyhow::bail!(
                "`admin.password` is required with `admin.auth: password`: set ADVENT_ADMIN__PASSWORD"
            );
        }
        Ok(config)
    }
}

impl ServerConfig {
    /// A real sender needs the details to send with, and `public_url` to
    /// build links players can open.
    fn check_mail(&self) -> anyhow::Result<()> {
        let mail = &self.mail;
        if mail.sender == MailSender::Log {
            return Ok(());
        }
        let need = |missing: bool, key: &str| -> anyhow::Result<()> {
            if missing {
                anyhow::bail!("`{key}` is required to send email (`mail.sender` is not `log`)");
            }
            Ok(())
        };
        need(self.public_url.is_none(), "public_url")?;
        need(mail.from.is_none(), "mail.from")?;
        match mail.sender {
            MailSender::Smtp => need(mail.smtp.host.is_none(), "mail.smtp.host"),
            MailSender::Resend => need(
                mail.resend.api_key.is_none(),
                "mail.resend.api_key (ADVENT_MAIL__RESEND__API_KEY)",
            ),
            MailSender::Log => Ok(()),
        }
    }

    /// Where links in emails point.
    pub fn link_base(&self) -> String {
        self.public_url
            .clone()
            .unwrap_or_else(|| format!("http://{}", self.listen))
            .trim_end_matches('/')
            .to_string()
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
            jail.set_env("ADVENT_ADMIN__PASSWORD", "admin");
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
    fn serve_config_mail_needs_its_details() {
        Jail::expect_with(|jail| {
            jail.set_env("ADVENT_ADMIN__PASSWORD", "admin");
            jail.set_env("ADVENT_MAIL__SENDER", "smtp");
            let err = ServerConfig::load(None, &Overrides::default()).unwrap_err();
            assert!(format!("{err:#}").contains("public_url"), "{err:#}");
            jail.set_env("ADVENT_PUBLIC_URL", "https://advent.acme.com/");
            jail.set_env("ADVENT_MAIL__FROM", "Advent <advent@acme.com>");
            let err = ServerConfig::load(None, &Overrides::default()).unwrap_err();
            assert!(format!("{err:#}").contains("mail.smtp.host"), "{err:#}");
            jail.set_env("ADVENT_MAIL__SMTP__HOST", "smtp.acme.com");
            jail.set_env("ADVENT_MAIL__SMTP__PASSWORD", "s3cret");
            let config = ServerConfig::load(None, &Overrides::default()).unwrap();
            assert_eq!(config.mail.smtp.password.as_deref(), Some("s3cret"));
            assert_eq!(config.mail.smtp.port, 587);
            assert_eq!(config.link_base(), "https://advent.acme.com");

            jail.set_env("ADVENT_MAIL__SENDER", "resend");
            let err = ServerConfig::load(None, &Overrides::default()).unwrap_err();
            assert!(format!("{err:#}").contains("api_key"), "{err:#}");
            Ok(())
        });
    }

    #[test]
    fn serve_config_password_mode_needs_a_password() {
        Jail::expect_with(|jail| {
            let err = ServerConfig::load(None, &Overrides::default()).unwrap_err();
            assert!(
                format!("{err:#}").contains("ADVENT_ADMIN__PASSWORD"),
                "{err:#}"
            );
            jail.set_env("ADVENT_ADMIN__PASSWORD", "s3cret");
            let config = ServerConfig::load(None, &Overrides::default()).unwrap();
            assert_eq!(config.admin.password.as_deref(), Some("s3cret"));
            Ok(())
        });
    }

    /// Every setting has a `### \`path\`` entry in the server reference.
    /// The literal is exhaustive on purpose: a new setting does not compile
    /// until it is added here, and then this fails until it is documented.
    #[test]
    fn serve_config_reference_documents_every_setting() {
        let config = ServerConfig {
            listen: "x".into(),
            public_url: Some("x".into()),
            data: PathBuf::from("x"),
            insecure_cookies: false,
            trust_proxy: false,
            limits: Limits {
                requests_per_minute: 1,
                guesses_per_minute: 1,
                signin_per_hour: 1,
                concurrent_checks: 1,
                links_per_hour: 1,
            },
            mail: MailConfig {
                sender: MailSender::Log,
                from: Some("x".into()),
                log_dir: Some(PathBuf::from("x")),
                smtp: SmtpConfig {
                    host: Some("x".into()),
                    port: 1,
                    username: Some("x".into()),
                    password: Some("x".into()),
                    tls: SmtpTls::None,
                },
                resend: ResendConfig {
                    api_key: Some("x".into()),
                    endpoint: "x".into(),
                },
            },
            admin: AdminConfig {
                auth: AdminAuth::Password,
                password: Some("x".into()),
                setup_code: Some("x".into()),
            },
        };
        fn paths(prefix: &str, value: &serde_json::Value, out: &mut Vec<String>) {
            match value {
                serde_json::Value::Object(map) => {
                    for (key, inner) in map {
                        let path = if prefix.is_empty() {
                            key.clone()
                        } else {
                            format!("{prefix}.{key}")
                        };
                        paths(&path, inner, out);
                    }
                }
                _ => out.push(prefix.to_string()),
            }
        }
        let mut settings = Vec::new();
        paths("", &serde_json::to_value(&config).unwrap(), &mut settings);
        let page = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/src/content/docs/reference/server.md"
        );
        let text = std::fs::read_to_string(page).unwrap();
        let missing: Vec<&String> = settings
            .iter()
            .filter(|path| !text.lines().any(|l| l.trim() == format!("### `{path}`")))
            .collect();
        assert!(missing.is_empty(), "undocumented in {page}: {missing:?}");
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
