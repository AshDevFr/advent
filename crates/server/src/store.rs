//! What the server keeps, and the storage interface each adapter implements.

use std::future::Future;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{MaybeSend, MaybeSync};

/// A storage failure, as text: each backend has its own error type, and the
/// core only reports them.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
#[error("storage: {0}")]
pub struct StoreError(pub String);

pub type StoreResult<T> = Result<T, StoreError>;

/// An invited player. The normalized email is the identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Player {
    /// Trimmed and lowercased; the key.
    pub email: String,
    /// The address as it was imported, for display and for sending.
    pub display_email: String,
    pub name: Option<String>,
    /// The personal sign-in link token. Kept as is rather than hashed:
    /// the same link is sent again when a player loses it, so the server
    /// must be able to produce it.
    pub link: String,
    pub created_at: DateTime<Utc>,
    /// When the invitation email was sent, if it was.
    pub invited_at: Option<DateTime<Utc>>,
    /// Last time the player used the calendar; `None` until they sign in.
    pub last_seen: Option<DateTime<Utc>>,
}

/// A signed-in browser. Only a hash of the session id is stored, so a copy
/// of the database does not let anyone take over a session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    pub id_hash: String,
    pub email: String,
    pub expires_at: DateTime<Utc>,
}

/// One player's state on one day of one event.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DayState {
    /// Every guess checked, the right one included.
    pub guesses: u32,
    /// Start of the current attempt window, with the attempt limit.
    pub window_start: Option<DateTime<Utc>>,
    /// Guesses in the current window.
    pub window_count: u32,
    pub solved: Option<Solved>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Solved {
    pub word: String,
    pub at: DateTime<Utc>,
    /// Whether at least one hint was out when the day was solved. Hints are
    /// released on a schedule rather than requested, so this is what "used
    /// a hint" can mean.
    pub hint_out: bool,
}

/// Storage for players, sessions and day states.
///
/// Methods return futures bounded by [`MaybeSend`] rather than being plain
/// `async fn`, so that the native adapter gets `Send` futures and the wasm
/// one does not have to provide them. Implementations can still write
/// `async fn`.
pub trait Store: MaybeSend + MaybeSync {
    fn player(&self, email: &str) -> impl Future<Output = StoreResult<Option<Player>>> + MaybeSend;
    fn player_by_link(
        &self,
        token: &str,
    ) -> impl Future<Output = StoreResult<Option<Player>>> + MaybeSend;
    fn players(&self) -> impl Future<Output = StoreResult<Vec<Player>>> + MaybeSend;
    /// Insert or replace the player with this email.
    fn put_player(&self, player: &Player) -> impl Future<Output = StoreResult<()>> + MaybeSend;
    /// Remove the player with their sessions and day states.
    fn remove_player(&self, email: &str) -> impl Future<Output = StoreResult<()>> + MaybeSend;

    fn session(
        &self,
        id_hash: &str,
    ) -> impl Future<Output = StoreResult<Option<Session>>> + MaybeSend;
    fn put_session(&self, session: &Session) -> impl Future<Output = StoreResult<()>> + MaybeSend;
    fn remove_session(&self, id_hash: &str) -> impl Future<Output = StoreResult<()>> + MaybeSend;
    /// End every session of a player.
    fn remove_sessions(&self, email: &str) -> impl Future<Output = StoreResult<()>> + MaybeSend;

    /// When an admin session (by hash) expires, if it exists.
    fn admin_session(
        &self,
        id_hash: &str,
    ) -> impl Future<Output = StoreResult<Option<DateTime<Utc>>>> + MaybeSend;
    fn put_admin_session(
        &self,
        id_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> impl Future<Output = StoreResult<()>> + MaybeSend;
    fn remove_admin_session(
        &self,
        id_hash: &str,
    ) -> impl Future<Output = StoreResult<()>> + MaybeSend;

    fn day_state(
        &self,
        email: &str,
        event: &str,
        day: u32,
    ) -> impl Future<Output = StoreResult<Option<DayState>>> + MaybeSend;
    fn put_day_state(
        &self,
        email: &str,
        event: &str,
        day: u32,
        state: &DayState,
    ) -> impl Future<Output = StoreResult<()>> + MaybeSend;
    /// A player's states on an event, by day.
    fn player_days(
        &self,
        email: &str,
        event: &str,
    ) -> impl Future<Output = StoreResult<Vec<(u32, DayState)>>> + MaybeSend;
    /// Every player's states on an event, as (email, day, state).
    fn event_days(
        &self,
        event: &str,
    ) -> impl Future<Output = StoreResult<Vec<(String, u32, DayState)>>> + MaybeSend;
}
