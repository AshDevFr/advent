//! `advent serve`: the company edition's self-hosted server. It serves the
//! calendar to invited, signed-in players, checks their guesses against the
//! sealed word locks, and keeps their progress in SQLite. The logic lives
//! in `advent-server`; this module is the axum, SQLite and tokio adapter.

mod api;
pub mod config;
mod content;
mod emails;
mod limits;
mod mail;
mod sqlite;

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use advent_core::primitives::WordLock;
use advent_core::wrap::x25519;
use advent_server::guess::check_locks;
use advent_server::{Checker, Entropy};
use anyhow::Context;
use chrono::{DateTime, Utc};
use tokio::sync::Semaphore;

use crate::config::{find_repo_root, load_repo_config, release_identities};
use config::{Overrides, ServerConfig};
use content::Content;
use limits::{Kind, Limiter};
use sqlite::SqliteStore;

pub struct Args {
    pub repo: Option<PathBuf>,
    pub config: Option<PathBuf>,
    pub data: Option<PathBuf>,
    pub listen: Option<String>,
}

/// The server's clock. `ADVENT_SERVE_NOW` (RFC 3339) starts it at another
/// instant, which then moves forward in real time: for tests and for
/// rehearsing a calendar before it starts. Not a setting to deploy with.
#[derive(Clone, Copy)]
pub struct ServeClock {
    offset: chrono::Duration,
}

impl ServeClock {
    fn from_env() -> anyhow::Result<Self> {
        let offset = match std::env::var("ADVENT_SERVE_NOW") {
            Ok(text) => {
                let start: DateTime<Utc> = text.parse().with_context(|| {
                    format!("ADVENT_SERVE_NOW={text:?} is not an RFC 3339 time")
                })?;
                eprintln!("note: clock set to {start} by ADVENT_SERVE_NOW");
                start - Utc::now()
            }
            Err(_) => chrono::Duration::zero(),
        };
        Ok(Self { offset })
    }

    pub fn now(&self) -> DateTime<Utc> {
        Utc::now() + self.offset
    }
}

/// Answer checks on the blocking pool, a bounded number at a time: each
/// is a PBKDF2 derivation per lock, too slow for the async executor.
pub struct PoolChecker {
    permits: Semaphore,
}

impl Checker for PoolChecker {
    async fn check(&self, answer: &str, locks: &[WordLock]) -> Option<String> {
        let _permit = self.permits.acquire().await.ok()?;
        let (answer, locks) = (answer.to_string(), locks.to_vec());
        tokio::task::spawn_blocking(move || check_locks(&answer, &locks))
            .await
            .ok()
            .flatten()
    }
}

pub struct OsEntropy;

impl Entropy for OsEntropy {
    fn fill(&self, buf: &mut [u8]) {
        getrandom::fill(buf).expect("the OS random number generator is unavailable");
    }
}

pub struct AppState {
    pub config: ServerConfig,
    pub store: SqliteStore,
    pub checker: PoolChecker,
    pub clock: ServeClock,
    pub limiter: Limiter,
    pub mailer: mail::Sender,
    content: RwLock<Arc<Content>>,
}

impl AppState {
    pub fn content(&self) -> Arc<Content> {
        self.content
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

pub fn run(args: Args) -> anyhow::Result<()> {
    let root = find_repo_root(args.repo.as_deref().unwrap_or(Path::new(".")))?;
    load_repo_config(&root)?;
    let config = ServerConfig::load(
        args.config.as_deref(),
        &Overrides {
            listen: args.listen,
            data: args.data,
        },
    )?;
    let identities = release_identities()?;
    let clock = ServeClock::from_env()?;
    // One TLS crypto provider for both email clients (SMTP and Resend).
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mailer = mail::Sender::new(&config.mail)?;

    content::validate(&root, &identities)?;
    let content = content::load(&root, &identities, clock.now())?;
    let store = SqliteStore::open(&config.data.join("advent.db"))?;

    let limits = &config.limits;
    let state = Arc::new(AppState {
        limiter: Limiter::new([
            (
                Kind::Request,
                limits.requests_per_minute,
                Duration::from_secs(60),
            ),
            (
                Kind::Guess,
                limits.guesses_per_minute,
                Duration::from_secs(60),
            ),
            (
                Kind::SignIn,
                limits.signin_per_hour,
                Duration::from_secs(3600),
            ),
            (
                Kind::LinkEmail,
                limits.links_per_hour,
                Duration::from_secs(3600),
            ),
        ]),
        mailer,
        checker: PoolChecker {
            permits: Semaphore::new(limits.concurrent_checks),
        },
        config,
        store,
        clock,
        content: RwLock::new(Arc::new(content)),
    });

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(serve(state, root, identities))
}

async fn serve(
    state: Arc<AppState>,
    root: PathBuf,
    identities: Vec<x25519::Identity>,
) -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind(&state.config.listen)
        .await
        .with_context(|| format!("cannot listen on {}", state.config.listen))?;
    println!(
        "Serving {} event(s) on http://{}/",
        state.content().events.len(),
        listener.local_addr()?
    );
    tokio::spawn(regenerate(state.clone(), root, identities));
    axum::serve(
        listener,
        api::router(state).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await?;
    Ok(())
}

/// Rebuild the site whenever something is released. A failed rebuild keeps
/// the current site.
async fn regenerate(state: Arc<AppState>, root: PathBuf, identities: Vec<x25519::Identity>) {
    // Woken at least this often, so a clock change cannot leave the site
    // stale for long.
    const MAX_SLEEP: Duration = Duration::from_secs(300);
    loop {
        let now = state.clock.now();
        let current = state.content();
        let next = content::next_moment(current.events.values().map(|e| e.as_ref()), now);
        drop(current);
        let Some(next) = next else {
            tokio::time::sleep(MAX_SLEEP).await;
            continue;
        };
        let wait = (next - now).to_std().unwrap_or_default();
        tokio::time::sleep(wait.min(MAX_SLEEP)).await;
        if state.clock.now() < next {
            continue;
        }
        // Retry every minute until it works: a day stays unpublished
        // otherwise, which is what the log is for.
        loop {
            let (root, identities, now) = (root.clone(), identities.clone(), state.clock.now());
            match tokio::task::spawn_blocking(move || content::load(&root, &identities, now)).await
            {
                Ok(Ok(fresh)) => {
                    *state.content.write().unwrap_or_else(|e| e.into_inner()) = Arc::new(fresh);
                    eprintln!("site updated for {now}");
                    break;
                }
                Ok(Err(e)) => {
                    eprintln!("error: could not update the site, keeping the current one: {e:#}")
                }
                Err(e) => eprintln!("error: site update failed: {e}"),
            }
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }
}
