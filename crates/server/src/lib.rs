//! Server logic for the company edition: invited players, personal sign-in
//! links, sessions, guesses checked against the sealed word locks with an
//! optional attempt limit, progress and stats.
//!
//! Everything here is independent of the runtime. Storage, the answer check
//! and randomness come in through traits, and the current time is passed in,
//! so the self-hosted server (axum and SQLite) and a Cloudflare Worker (D1)
//! are thin adapters over the same code. The crate builds for
//! wasm32-unknown-unknown for that reason.

pub mod admin;
pub mod content;
pub mod guess;
pub mod mail;
pub mod memory;
pub mod players;
pub mod stats;
pub mod store;

pub use content::EventData;
pub use guess::{Checker, InlineChecker, Outcome, guess};
pub use store::{DayState, Player, Session, Solved, Store, StoreError};

/// `Send` on native targets, nothing on wasm32.
///
/// The axum adapter runs these futures on a multi-threaded executor, which
/// needs them to be `Send`; a Worker's storage futures are not `Send` and do
/// not need to be. Bounding the traits by this marker gives each adapter
/// what it needs from one set of traits.
#[cfg(not(target_arch = "wasm32"))]
pub trait MaybeSend: Send {}
#[cfg(not(target_arch = "wasm32"))]
impl<T: Send + ?Sized> MaybeSend for T {}
#[cfg(target_arch = "wasm32")]
pub trait MaybeSend {}
#[cfg(target_arch = "wasm32")]
impl<T: ?Sized> MaybeSend for T {}

/// `Sync` on native targets, nothing on wasm32 (see [`MaybeSend`]).
#[cfg(not(target_arch = "wasm32"))]
pub trait MaybeSync: Sync {}
#[cfg(not(target_arch = "wasm32"))]
impl<T: Sync + ?Sized> MaybeSync for T {}
#[cfg(target_arch = "wasm32")]
pub trait MaybeSync {}
#[cfg(target_arch = "wasm32")]
impl<T: ?Sized> MaybeSync for T {}

/// Source of random bytes for tokens. The crate cannot draw randomness
/// itself on wasm32-unknown-unknown; each adapter supplies the platform's.
pub trait Entropy: MaybeSend + MaybeSync {
    fn fill(&self, buf: &mut [u8]);
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ServerError {
    #[error("not signed in")]
    NoSession,
    #[error("this sign-in link is not valid")]
    InvalidLink,
    #[error("no player with this email")]
    UnknownPlayer,
    #[error("day {0} is not part of this event")]
    UnknownDay(u32),
    #[error("day {0} is not open yet")]
    NotReleased(u32),
    #[error("invalid name: {0}")]
    InvalidName(String),
    #[error(transparent)]
    Store(#[from] StoreError),
}
