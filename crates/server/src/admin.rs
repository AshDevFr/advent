//! What the organisers work with: importing an invite list, the players
//! table, one player's days, and CSV exports.

use std::collections::BTreeMap;

use chrono::{DateTime, SecondsFormat, Utc};
use serde::Serialize;

use crate::content::EventData;
use crate::players::{Invite, normalize_email};
use crate::players::{hash_token, token};
use crate::stats::finishers;
use crate::store::{DayState, Player, Store};
use crate::{Entropy, ServerError};
use sha2::{Digest, Sha256};

/// An invite list as pasted or uploaded: one address per line, optionally
/// with a name, separated by a comma, semicolon or tab. A header line
/// (one without an `@`) is skipped, and if it names the columns, their
/// order is taken from it. Returns the entries and the lines that are not
/// an address.
pub fn parse_invites(text: &str) -> (Vec<Invite>, Vec<String>) {
    let mut invites = Vec::new();
    let mut invalid = Vec::new();
    // Column of the email and of the name; by default the first two.
    let mut columns = (0usize, Some(1usize));
    for (n, line) in text.lines().enumerate() {
        let line = line.trim().trim_start_matches('\u{feff}');
        if line.is_empty() {
            continue;
        }
        let fields = split_fields(line);
        if n == 0 && !line.contains('@') {
            let lower: Vec<String> = fields.iter().map(|f| f.to_lowercase()).collect();
            if let Some(email) = lower.iter().position(|f| f.contains("mail")) {
                columns = (email, lower.iter().position(|f| f.contains("name")));
            }
            continue;
        }
        let email = fields.get(columns.0).map(|f| f.trim()).unwrap_or_default();
        if normalize_email(email).is_none() {
            invalid.push(line.to_string());
            continue;
        }
        let name = columns
            .1
            .and_then(|i| fields.get(i))
            .map(|f| f.trim().to_string())
            .filter(|n| !n.is_empty());
        invites.push(Invite {
            email: email.to_string(),
            name,
        });
    }
    (invites, invalid)
}

/// The fields of one CSV-like line: comma, semicolon or tab separated,
/// with double quotes around fields that contain a separator.
fn split_fields(line: &str) -> Vec<String> {
    let separator = [',', ';', '\t']
        .into_iter()
        .find(|s| line.contains(*s))
        .unwrap_or(',');
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            c if c == separator && !quoted => fields.push(std::mem::take(&mut field)),
            c => field.push(c),
        }
    }
    fields.push(field);
    fields
}

/// Whether `given` is the admin password, in time that does not depend on
/// where they differ: both are hashed first, then compared in full.
pub fn password_matches(given: &str, expected: &str) -> bool {
    let (a, b) = (Sha256::digest(given), Sha256::digest(expected));
    a.iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

/// Start an admin session; returns the token for the cookie (only its
/// hash is stored).
pub async fn admin_sign_in<S: Store>(
    store: &S,
    entropy: &impl Entropy,
    expires_at: DateTime<Utc>,
) -> Result<String, ServerError> {
    let session = token(entropy);
    store
        .put_admin_session(&hash_token(&session), expires_at)
        .await?;
    Ok(session)
}

/// Whether an admin session token is valid at `now`.
pub async fn admin_signed_in<S: Store>(
    store: &S,
    session: &str,
    now: DateTime<Utc>,
) -> Result<bool, ServerError> {
    let hash = hash_token(session);
    match store.admin_session(&hash).await? {
        Some(expires) if expires > now => Ok(true),
        Some(_) => {
            store.remove_admin_session(&hash).await?;
            Ok(false)
        }
        None => Ok(false),
    }
}

pub async fn admin_sign_out<S: Store>(store: &S, session: &str) -> Result<(), ServerError> {
    store.remove_admin_session(&hash_token(session)).await?;
    Ok(())
}

/// What an import would do, before doing it.
#[derive(Debug, Default, PartialEq, Eq, Serialize)]
pub struct ImportPreview {
    pub new: Vec<Invite>,
    pub existing: Vec<Invite>,
    pub invalid: Vec<String>,
}

pub async fn preview<S: Store>(store: &S, text: &str) -> Result<ImportPreview, ServerError> {
    let (invites, invalid) = parse_invites(text);
    let mut preview = ImportPreview {
        invalid,
        ..Default::default()
    };
    // Normalized email -> (is new, index in its list), for repeated lines.
    let mut seen: BTreeMap<String, (bool, usize)> = BTreeMap::new();
    for invite in invites {
        let email = normalize_email(&invite.email).unwrap_or_default();
        if let Some(&(is_new, index)) = seen.get(&email) {
            // A repeated address: keep the name if this line has one.
            let list = if is_new {
                &mut preview.new
            } else {
                &mut preview.existing
            };
            if invite.name.is_some() {
                list[index].name = invite.name;
            }
            continue;
        }
        let is_new = store.player(&email).await?.is_none();
        let list = if is_new {
            &mut preview.new
        } else {
            &mut preview.existing
        };
        seen.insert(email, (is_new, list.len()));
        list.push(invite);
    }
    Ok(preview)
}

/// One row of the players table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlayerRow {
    pub email: String,
    pub name: Option<String>,
    pub invited_at: Option<DateTime<Utc>>,
    pub last_seen: Option<DateTime<Utc>>,
    pub solved: u32,
    pub finished_at: Option<DateTime<Utc>>,
}

pub async fn player_rows<S: Store>(
    store: &S,
    content: &EventData,
) -> Result<Vec<PlayerRow>, ServerError> {
    let players = store.players().await?;
    let rows = store.event_days(&content.id).await?;
    let mut by_player: BTreeMap<&str, Vec<(u32, &DayState)>> = BTreeMap::new();
    for (email, day, state) in &rows {
        by_player
            .entry(email.as_str())
            .or_default()
            .push((*day, state));
    }
    Ok(players
        .iter()
        .map(|p| {
            let days = by_player.get(p.email.as_str()).cloned().unwrap_or_default();
            row(p, content, &days)
        })
        .collect())
}

fn row(player: &Player, content: &EventData, days: &[(u32, &DayState)]) -> PlayerRow {
    let solved: Vec<DateTime<Utc>> = days
        .iter()
        .filter_map(|(_, s)| s.solved.as_ref().map(|x| x.at))
        .collect();
    let finished = solved.len() as u32 >= content.event.days;
    PlayerRow {
        email: player.display_email.clone(),
        name: player.name.clone(),
        invited_at: player.invited_at,
        last_seen: player.last_seen,
        solved: solved.len() as u32,
        finished_at: if finished {
            solved.iter().max().copied()
        } else {
            None
        },
    }
}

/// One player's days, for the player page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlayerDay {
    pub day: u32,
    pub guesses: u32,
    pub solved_at: Option<DateTime<Utc>>,
    pub hint_out: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlayerDetail {
    #[serde(flatten)]
    pub row: PlayerRow,
    pub days: Vec<PlayerDay>,
}

pub async fn player_detail<S: Store>(
    store: &S,
    content: &EventData,
    email: &str,
) -> Result<PlayerDetail, ServerError> {
    let email = normalize_email(email).ok_or(ServerError::UnknownPlayer)?;
    let player = store
        .player(&email)
        .await?
        .ok_or(ServerError::UnknownPlayer)?;
    let days = store.player_days(&email, &content.id).await?;
    let refs: Vec<(u32, &DayState)> = days.iter().map(|(d, s)| (*d, s)).collect();
    Ok(PlayerDetail {
        row: row(&player, content, &refs),
        days: days
            .iter()
            .map(|(day, s)| PlayerDay {
                day: *day,
                guesses: s.guesses,
                solved_at: s.solved.as_ref().map(|x| x.at),
                hint_out: s.solved.as_ref().is_some_and(|x| x.hint_out),
            })
            .collect(),
    })
}

/// `finishers.csv`: who solved every day, earliest first.
pub async fn finishers_csv<S: Store>(
    store: &S,
    content: &EventData,
) -> Result<String, ServerError> {
    let mut out = csv_line(&["name", "email", "finished_at"]);
    for f in finishers(store, content).await? {
        out += &csv_line(&[
            f.name.as_deref().unwrap_or(""),
            &f.email,
            &time(Some(f.finished_at)),
        ]);
    }
    Ok(out)
}

/// `players.csv`: everyone, with their progress on this event.
pub async fn players_csv<S: Store>(store: &S, content: &EventData) -> Result<String, ServerError> {
    let mut out = csv_line(&[
        "name",
        "email",
        "invited_at",
        "last_seen",
        "days_solved",
        "finished_at",
    ]);
    for r in player_rows(store, content).await? {
        out += &csv_line(&[
            r.name.as_deref().unwrap_or(""),
            &r.email,
            &time(r.invited_at),
            &time(r.last_seen),
            &r.solved.to_string(),
            &time(r.finished_at),
        ]);
    }
    Ok(out)
}

fn time(t: Option<DateTime<Utc>>) -> String {
    t.map(|t| t.to_rfc3339_opts(SecondsFormat::Secs, true))
        .unwrap_or_default()
}

/// One CSV line (RFC 4180). A field a spreadsheet would read as a formula
/// (starting with `=`, `+`, `-`, `@`, tab or carriage return) gets a
/// leading apostrophe, so a player's chosen name cannot run in the
/// organiser's spreadsheet.
fn csv_line(fields: &[&str]) -> String {
    let cells: Vec<String> = fields
        .iter()
        .map(|f| {
            let f = if f.starts_with(['=', '+', '-', '@', '\t', '\r']) {
                format!("'{f}")
            } else {
                (*f).to_string()
            };
            if f.contains([',', '"', '\n', '\r']) {
                format!("\"{}\"", f.replace('"', "\"\""))
            } else {
                f
            }
        })
        .collect();
    cells.join(",") + "\r\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invites_parse_lists_and_csv() {
        let (invites, invalid) = parse_invites(
            "bob@acme.com\nAlice@acme.com,Alice\nnot-an-email\n\n  carol@acme.com ; Carol C  \n",
        );
        assert_eq!(
            invites,
            [
                Invite {
                    email: "bob@acme.com".into(),
                    name: None
                },
                Invite {
                    email: "Alice@acme.com".into(),
                    name: Some("Alice".into())
                },
                Invite {
                    email: "carol@acme.com".into(),
                    name: Some("Carol C".into())
                },
            ]
        );
        assert_eq!(invalid, ["not-an-email"]);

        let (invites, invalid) =
            parse_invites("\u{feff}Name,E-mail\n\"Martin, Alice\",alice@acme.com\n");
        assert!(invalid.is_empty());
        assert_eq!(invites[0].email, "alice@acme.com");
        assert_eq!(invites[0].name.as_deref(), Some("Martin, Alice"));
    }

    #[test]
    fn admin_password_comparison() {
        assert!(password_matches("correct horse", "correct horse"));
        assert!(!password_matches("correct hors", "correct horse"));
        assert!(!password_matches("", "x"));
    }

    #[test]
    fn csv_lines_quote_and_neutralize() {
        assert_eq!(
            csv_line(&["a", "b,c", "say \"hi\""]),
            "a,\"b,c\",\"say \"\"hi\"\"\"\r\n"
        );
        assert_eq!(
            csv_line(&["=HYPERLINK(1)", "-2", "@x"]),
            "'=HYPERLINK(1),'-2,'@x\r\n"
        );
    }
}
