//! Invited players, their personal sign-in links, and sessions.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{DateTime, Duration, Utc};
use sha2::{Digest, Sha256};

use crate::store::{Player, Session, Store};
use crate::{Entropy, ServerError};

/// The longest name kept; it has to fit on a certificate.
pub const MAX_NAME: usize = 80;

/// How stale `last_seen` may get before a request updates it, so reading
/// a session does not write on every request.
const SEEN_GRANULARITY: Duration = Duration::minutes(5);

/// One line of an invite list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invite {
    pub email: String,
    pub name: Option<String>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ImportReport {
    /// Normalized emails of new players.
    pub created: Vec<String>,
    /// Existing players whose name changed.
    pub updated: Vec<String>,
    /// Existing players left as they were.
    pub unchanged: Vec<String>,
    /// Lines that are not an email address, as given.
    pub invalid: Vec<String>,
}

/// The identity form of an email (trimmed, lowercased), or `None` if it
/// does not look like an address. Deliberately loose: the invitation email
/// is what proves an address works.
pub fn normalize_email(email: &str) -> Option<String> {
    let email = email.trim().to_lowercase();
    let (local, domain) = email.split_once('@')?;
    let valid = !local.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !domain.contains('@')
        && !email.chars().any(|c| c.is_whitespace() || c.is_control());
    valid.then_some(email)
}

fn clean_name(name: &str) -> Result<String, ServerError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(ServerError::InvalidName("it is empty".into()));
    }
    if name.chars().count() > MAX_NAME {
        return Err(ServerError::InvalidName(format!(
            "it is longer than {MAX_NAME} characters"
        )));
    }
    if name.chars().any(char::is_control) {
        return Err(ServerError::InvalidName(
            "it contains control characters".into(),
        ));
    }
    Ok(name.to_string())
}

/// A new random token: 32 bytes, URL-safe base64.
pub fn token(entropy: &impl Entropy) -> String {
    let mut bytes = [0u8; 32];
    entropy.fill(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// What is stored for a session id.
pub fn hash_token(token: &str) -> String {
    Sha256::digest(token.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Add or update players. An address is one player however it is written
/// and however often it appears; a name updates an existing player, and a
/// missing name leaves theirs alone.
pub async fn import<S: Store>(
    store: &S,
    entropy: &impl Entropy,
    invites: &[Invite],
    now: DateTime<Utc>,
) -> Result<ImportReport, ServerError> {
    let mut report = ImportReport::default();
    for invite in invites {
        let Some(email) = normalize_email(&invite.email) else {
            report.invalid.push(invite.email.clone());
            continue;
        };
        let name = invite
            .name
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .map(clean_name)
            .transpose()?;
        match store.player(&email).await? {
            None => {
                store
                    .put_player(&Player {
                        email: email.clone(),
                        display_email: invite.email.trim().to_string(),
                        name,
                        link: token(entropy),
                        created_at: now,
                        invited_at: None,
                        last_seen: None,
                    })
                    .await?;
                report.created.push(email);
            }
            Some(mut player) => {
                if name.is_some() && name != player.name {
                    player.name = name;
                    store.put_player(&player).await?;
                    if !report.created.contains(&email) && !report.updated.contains(&email) {
                        report.updated.push(email);
                    }
                } else if !report.created.contains(&email)
                    && !report.updated.contains(&email)
                    && !report.unchanged.contains(&email)
                {
                    report.unchanged.push(email);
                }
            }
        }
    }
    Ok(report)
}

/// Give a player a new link: the old one stops working and every session
/// they have ends. Returns the new link token.
pub async fn reset_link<S: Store>(
    store: &S,
    entropy: &impl Entropy,
    email: &str,
) -> Result<String, ServerError> {
    let mut player = store
        .player(email)
        .await?
        .ok_or(ServerError::UnknownPlayer)?;
    player.link = token(entropy);
    store.put_player(&player).await?;
    store.remove_sessions(email).await?;
    Ok(player.link)
}

/// The player a link belongs to, without signing in: the link page shows
/// who is about to sign in before anything changes.
pub async fn link_player<S: Store>(store: &S, link: &str) -> Result<Player, ServerError> {
    if link.is_empty() {
        return Err(ServerError::InvalidLink);
    }
    store
        .player_by_link(link)
        .await?
        .ok_or(ServerError::InvalidLink)
}

/// Exchange a link for a new session. Returns the session token for the
/// cookie, which is never stored.
pub async fn sign_in<S: Store>(
    store: &S,
    entropy: &impl Entropy,
    link: &str,
    now: DateTime<Utc>,
    expires_at: DateTime<Utc>,
) -> Result<(String, Player), ServerError> {
    let mut player = link_player(store, link).await?;
    let session = token(entropy);
    store
        .put_session(&Session {
            id_hash: hash_token(&session),
            email: player.email.clone(),
            expires_at,
        })
        .await?;
    player.last_seen = Some(now);
    store.put_player(&player).await?;
    Ok((session, player))
}

/// The player signed in with this session token.
pub async fn session_player<S: Store>(
    store: &S,
    session: &str,
    now: DateTime<Utc>,
) -> Result<Player, ServerError> {
    let hash = hash_token(session);
    let Some(record) = store.session(&hash).await? else {
        return Err(ServerError::NoSession);
    };
    if record.expires_at <= now {
        store.remove_session(&hash).await?;
        return Err(ServerError::NoSession);
    }
    let mut player = store
        .player(&record.email)
        .await?
        .ok_or(ServerError::NoSession)?;
    if player
        .last_seen
        .is_none_or(|seen| now - seen >= SEEN_GRANULARITY)
    {
        player.last_seen = Some(now);
        store.put_player(&player).await?;
    }
    Ok(player)
}

pub async fn sign_out<S: Store>(store: &S, session: &str) -> Result<(), ServerError> {
    store.remove_session(&hash_token(session)).await?;
    Ok(())
}

/// Set the name shown on the player's certificate.
pub async fn set_name<S: Store>(store: &S, email: &str, name: &str) -> Result<Player, ServerError> {
    let name = clean_name(name)?;
    let mut player = store
        .player(email)
        .await?
        .ok_or(ServerError::UnknownPlayer)?;
    player.name = Some(name);
    store.put_player(&player).await?;
    Ok(player)
}

/// Record that the invitation email went out.
pub async fn mark_invited<S: Store>(
    store: &S,
    email: &str,
    now: DateTime<Utc>,
) -> Result<(), ServerError> {
    let mut player = store
        .player(email)
        .await?
        .ok_or(ServerError::UnknownPlayer)?;
    player.invited_at = Some(now);
    store.put_player(&player).await?;
    Ok(())
}

pub async fn remove<S: Store>(store: &S, email: &str) -> Result<(), ServerError> {
    store.remove_player(email).await?;
    Ok(())
}
