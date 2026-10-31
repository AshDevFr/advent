//! Email senders: SMTP (the company's relay), Resend (an HTTP API), and a
//! log for development and tests.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use advent_server::mail::{Email, MailError, Mailer};
use anyhow::Context;
use lettre::message::{Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use serde_json::json;

use super::config::{MailConfig, MailSender, SmtpTls};

pub enum Sender {
    Log {
        dir: Option<PathBuf>,
        count: AtomicU64,
    },
    Smtp {
        transport: AsyncSmtpTransport<Tokio1Executor>,
        from: Mailbox,
    },
    Resend {
        client: reqwest::Client,
        endpoint: String,
        api_key: String,
        from: String,
    },
}

impl Sender {
    pub fn new(config: &MailConfig) -> anyhow::Result<Self> {
        let from = || config.from.clone().context("`mail.from` is required");
        Ok(match config.sender {
            MailSender::Log => {
                if let Some(dir) = &config.log_dir {
                    std::fs::create_dir_all(dir)
                        .with_context(|| format!("creating {}", dir.display()))?;
                }
                Sender::Log {
                    dir: config.log_dir.clone(),
                    count: AtomicU64::new(0),
                }
            }
            MailSender::Smtp => {
                let smtp = &config.smtp;
                let host = smtp
                    .host
                    .as_deref()
                    .context("`mail.smtp.host` is required")?;
                let mut builder = match smtp.tls {
                    SmtpTls::Starttls => {
                        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host)?
                    }
                    SmtpTls::Tls => AsyncSmtpTransport::<Tokio1Executor>::relay(host)?,
                    SmtpTls::None => AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host),
                }
                .port(smtp.port);
                if let Some(user) = &smtp.username {
                    builder = builder.credentials(Credentials::new(
                        user.clone(),
                        smtp.password.clone().unwrap_or_default(),
                    ));
                }
                Sender::Smtp {
                    transport: builder.build(),
                    from: from()?
                        .parse()
                        .context("`mail.from` is not an email address")?,
                }
            }
            MailSender::Resend => Sender::Resend {
                client: reqwest::Client::new(),
                endpoint: config.resend.endpoint.trim_end_matches('/').to_string(),
                api_key: config
                    .resend
                    .api_key
                    .clone()
                    .context("`mail.resend.api_key` is required")?,
                from: from()?,
            },
        })
    }
}

fn fail(e: impl std::fmt::Display) -> MailError {
    MailError(e.to_string())
}

impl Mailer for Sender {
    async fn send(&self, email: &Email) -> Result<(), MailError> {
        match self {
            Sender::Log { dir, count } => {
                let value = json!({
                    "to": email.to,
                    "subject": email.subject,
                    "text": email.text,
                    "html": email.html,
                });
                match dir {
                    Some(dir) => {
                        let n = count.fetch_add(1, Ordering::Relaxed) + 1;
                        let path = dir.join(format!("{n:04}.json"));
                        std::fs::write(&path, serde_json::to_vec_pretty(&value).map_err(fail)?)
                            .map_err(fail)?;
                    }
                    None => eprintln!(
                        "email to {} ({}):\n{}\n",
                        email.to, email.subject, email.text
                    ),
                }
                Ok(())
            }
            Sender::Smtp { transport, from } => {
                let to: Mailbox = email.to.parse().map_err(fail)?;
                let builder = Message::builder()
                    .from(from.clone())
                    .to(to)
                    .subject(&email.subject);
                let message = match &email.html {
                    Some(html) => builder.multipart(MultiPart::alternative_plain_html(
                        email.text.clone(),
                        html.clone(),
                    )),
                    None => builder.singlepart(SinglePart::plain(email.text.clone())),
                }
                .map_err(fail)?;
                transport.send(message).await.map_err(fail)?;
                Ok(())
            }
            Sender::Resend {
                client,
                endpoint,
                api_key,
                from,
            } => {
                let response = client
                    .post(format!("{endpoint}/emails"))
                    .bearer_auth(api_key)
                    .json(&json!({
                        "from": from,
                        "to": [email.to],
                        "subject": email.subject,
                        "text": email.text,
                        "html": email.html,
                    }))
                    .send()
                    .await
                    .map_err(fail)?;
                if !response.status().is_success() {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    return Err(MailError(format!("Resend answered {status}: {body}")));
                }
                Ok(())
            }
        }
    }
}
