//! The email interface. Senders (SMTP, Resend, a log for development) live
//! in the adapters; the core only says what to send.

use std::future::Future;

use crate::{MaybeSend, MaybeSync};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Email {
    pub to: String,
    pub subject: String,
    pub text: String,
    pub html: Option<String>,
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
#[error("mail: {0}")]
pub struct MailError(pub String);

pub trait Mailer: MaybeSend + MaybeSync {
    fn send(&self, email: &Email) -> impl Future<Output = Result<(), MailError>> + MaybeSend;
}
