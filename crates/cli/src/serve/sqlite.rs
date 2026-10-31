//! The server's [`Store`] in one SQLite file.
//!
//! rusqlite is synchronous, so every call runs on tokio's blocking pool
//! behind one connection. A calendar's write load (a guess, a sign-in) is
//! small enough that a single writer is not the bottleneck; the answer
//! check is, and it never holds the connection.

use std::path::Path;
use std::sync::{Arc, Mutex};

use advent_server::store::StoreResult;
use advent_server::{DayState, Player, Session, Solved, Store, StoreError};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, Row, params};

/// Schema steps, applied in order; `PRAGMA user_version` records how many
/// ran. Append only: a released step never changes.
const MIGRATIONS: &[&str] = &[
    "
    CREATE TABLE players (
        email TEXT PRIMARY KEY,
        display_email TEXT NOT NULL,
        name TEXT,
        link TEXT NOT NULL UNIQUE,
        created_at TEXT NOT NULL,
        invited_at TEXT,
        last_seen TEXT
    );
    CREATE TABLE sessions (
        id_hash TEXT PRIMARY KEY,
        email TEXT NOT NULL REFERENCES players(email) ON DELETE CASCADE,
        expires_at TEXT NOT NULL
    );
    CREATE INDEX sessions_email ON sessions(email);
    CREATE TABLE days (
        email TEXT NOT NULL REFERENCES players(email) ON DELETE CASCADE,
        event TEXT NOT NULL,
        day INTEGER NOT NULL,
        guesses INTEGER NOT NULL,
        window_start TEXT,
        window_count INTEGER NOT NULL,
        solved_word TEXT,
        solved_at TEXT,
        hint_out INTEGER,
        PRIMARY KEY (email, event, day)
    );
    CREATE INDEX days_event ON days(event);
",
    "
    CREATE TABLE admin_sessions (
        id_hash TEXT PRIMARY KEY,
        expires_at TEXT NOT NULL
    );
",
];

#[derive(Clone)]
pub struct SqliteStore {
    conn: Arc<Mutex<Connection>>,
}

fn err(e: impl std::fmt::Display) -> StoreError {
    StoreError(e.to_string())
}

impl SqliteStore {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        Self::init(conn)
    }

    #[cfg(test)]
    pub fn in_memory() -> anyhow::Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> anyhow::Result<Self> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        let version = usize::try_from(version).unwrap_or(usize::MAX);
        if version > MIGRATIONS.len() {
            anyhow::bail!(
                "the database was written by a newer advent (schema {version}, this one knows {})",
                MIGRATIONS.len()
            );
        }
        for (i, step) in MIGRATIONS.iter().enumerate().skip(version) {
            conn.execute_batch(&format!(
                "BEGIN; {step}; PRAGMA user_version = {}; COMMIT;",
                i + 1
            ))?;
        }
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    async fn call<T, F>(&self, f: F) -> StoreResult<T>
    where
        T: Send + 'static,
        F: FnOnce(&Connection) -> rusqlite::Result<T> + Send + 'static,
    {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().unwrap_or_else(|e| e.into_inner());
            f(&conn).map_err(err)
        })
        .await
        .map_err(err)?
    }
}

const PLAYER_COLUMNS: &str = "email, display_email, name, link, created_at, invited_at, last_seen";

fn player_row(row: &Row) -> rusqlite::Result<Player> {
    Ok(Player {
        email: row.get(0)?,
        display_email: row.get(1)?,
        name: row.get(2)?,
        link: row.get(3)?,
        created_at: row.get(4)?,
        invited_at: row.get(5)?,
        last_seen: row.get(6)?,
    })
}

const DAY_COLUMNS: &str = "guesses, window_start, window_count, solved_word, solved_at, hint_out";

/// A day state from the columns in [`DAY_COLUMNS`], starting at `at`.
fn day_row(row: &Row, at: usize) -> rusqlite::Result<DayState> {
    let word: Option<String> = row.get(at + 3)?;
    let solved = match word {
        Some(word) => Some(Solved {
            word,
            at: row.get(at + 4)?,
            hint_out: row.get::<_, Option<bool>>(at + 5)?.unwrap_or(false),
        }),
        None => None,
    };
    Ok(DayState {
        guesses: row.get(at)?,
        window_start: row.get(at + 1)?,
        window_count: row.get(at + 2)?,
        solved,
    })
}

impl Store for SqliteStore {
    async fn player(&self, email: &str) -> StoreResult<Option<Player>> {
        let email = email.to_string();
        self.call(move |c| {
            c.query_row(
                &format!("SELECT {PLAYER_COLUMNS} FROM players WHERE email = ?1"),
                [email],
                player_row,
            )
            .optional()
        })
        .await
    }

    async fn player_by_link(&self, token: &str) -> StoreResult<Option<Player>> {
        let token = token.to_string();
        self.call(move |c| {
            c.query_row(
                &format!("SELECT {PLAYER_COLUMNS} FROM players WHERE link = ?1"),
                [token],
                player_row,
            )
            .optional()
        })
        .await
    }

    async fn players(&self) -> StoreResult<Vec<Player>> {
        self.call(|c| {
            let mut stmt = c.prepare(&format!(
                "SELECT {PLAYER_COLUMNS} FROM players ORDER BY email"
            ))?;
            stmt.query_map([], player_row)?.collect()
        })
        .await
    }

    async fn put_player(&self, player: &Player) -> StoreResult<()> {
        let p = player.clone();
        self.call(move |c| {
            c.execute(
                "INSERT INTO players (email, display_email, name, link, created_at, invited_at, last_seen)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(email) DO UPDATE SET display_email = ?2, name = ?3, link = ?4,
                     created_at = ?5, invited_at = ?6, last_seen = ?7",
                params![
                    p.email,
                    p.display_email,
                    p.name,
                    p.link,
                    p.created_at,
                    p.invited_at,
                    p.last_seen
                ],
            )
            .map(drop)
        })
        .await
    }

    async fn remove_player(&self, email: &str) -> StoreResult<()> {
        let email = email.to_string();
        // Sessions and day states go with the player (ON DELETE CASCADE).
        self.call(move |c| {
            c.execute("DELETE FROM players WHERE email = ?1", [email])
                .map(drop)
        })
        .await
    }

    async fn session(&self, id_hash: &str) -> StoreResult<Option<Session>> {
        let id_hash = id_hash.to_string();
        self.call(move |c| {
            c.query_row(
                "SELECT id_hash, email, expires_at FROM sessions WHERE id_hash = ?1",
                [id_hash],
                |r| {
                    Ok(Session {
                        id_hash: r.get(0)?,
                        email: r.get(1)?,
                        expires_at: r.get(2)?,
                    })
                },
            )
            .optional()
        })
        .await
    }

    async fn put_session(&self, session: &Session) -> StoreResult<()> {
        let s = session.clone();
        self.call(move |c| {
            c.execute(
                "INSERT OR REPLACE INTO sessions (id_hash, email, expires_at) VALUES (?1, ?2, ?3)",
                params![s.id_hash, s.email, s.expires_at],
            )
            .map(drop)
        })
        .await
    }

    async fn remove_session(&self, id_hash: &str) -> StoreResult<()> {
        let id_hash = id_hash.to_string();
        self.call(move |c| {
            c.execute("DELETE FROM sessions WHERE id_hash = ?1", [id_hash])
                .map(drop)
        })
        .await
    }

    async fn remove_sessions(&self, email: &str) -> StoreResult<()> {
        let email = email.to_string();
        self.call(move |c| {
            c.execute("DELETE FROM sessions WHERE email = ?1", [email])
                .map(drop)
        })
        .await
    }

    async fn admin_session(&self, id_hash: &str) -> StoreResult<Option<DateTime<Utc>>> {
        let id_hash = id_hash.to_string();
        self.call(move |c| {
            c.query_row(
                "SELECT expires_at FROM admin_sessions WHERE id_hash = ?1",
                [id_hash],
                |r| r.get(0),
            )
            .optional()
        })
        .await
    }

    async fn put_admin_session(&self, id_hash: &str, expires_at: DateTime<Utc>) -> StoreResult<()> {
        let id_hash = id_hash.to_string();
        self.call(move |c| {
            c.execute(
                "INSERT OR REPLACE INTO admin_sessions (id_hash, expires_at) VALUES (?1, ?2)",
                params![id_hash, expires_at],
            )
            .map(drop)
        })
        .await
    }

    async fn remove_admin_session(&self, id_hash: &str) -> StoreResult<()> {
        let id_hash = id_hash.to_string();
        self.call(move |c| {
            c.execute("DELETE FROM admin_sessions WHERE id_hash = ?1", [id_hash])
                .map(drop)
        })
        .await
    }

    async fn day_state(&self, email: &str, event: &str, day: u32) -> StoreResult<Option<DayState>> {
        let (email, event) = (email.to_string(), event.to_string());
        self.call(move |c| {
            c.query_row(
                &format!(
                    "SELECT {DAY_COLUMNS} FROM days WHERE email = ?1 AND event = ?2 AND day = ?3"
                ),
                params![email, event, day],
                |r| day_row(r, 0),
            )
            .optional()
        })
        .await
    }

    async fn put_day_state(
        &self,
        email: &str,
        event: &str,
        day: u32,
        state: &DayState,
    ) -> StoreResult<()> {
        let (email, event, s) = (email.to_string(), event.to_string(), state.clone());
        self.call(move |c| {
            c.execute(
                "INSERT OR REPLACE INTO days
                     (email, event, day, guesses, window_start, window_count, solved_word, solved_at, hint_out)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    email,
                    event,
                    day,
                    s.guesses,
                    s.window_start,
                    s.window_count,
                    s.solved.as_ref().map(|x| x.word.clone()),
                    s.solved.as_ref().map(|x| x.at),
                    s.solved.as_ref().map(|x| x.hint_out),
                ],
            )
            .map(drop)
        })
        .await
    }

    async fn player_days(&self, email: &str, event: &str) -> StoreResult<Vec<(u32, DayState)>> {
        let (email, event) = (email.to_string(), event.to_string());
        self.call(move |c| {
            let mut stmt = c.prepare(&format!(
                "SELECT day, {DAY_COLUMNS} FROM days WHERE email = ?1 AND event = ?2 ORDER BY day"
            ))?;
            stmt.query_map(params![email, event], |r| Ok((r.get(0)?, day_row(r, 1)?)))?
                .collect()
        })
        .await
    }

    async fn event_days(&self, event: &str) -> StoreResult<Vec<(String, u32, DayState)>> {
        let event = event.to_string();
        self.call(move |c| {
            let mut stmt = c.prepare(&format!(
                "SELECT email, day, {DAY_COLUMNS} FROM days WHERE event = ?1 ORDER BY email, day"
            ))?;
            stmt.query_map([event], |r| Ok((r.get(0)?, r.get(1)?, day_row(r, 2)?)))?
                .collect()
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> DateTime<Utc> {
        text.parse().unwrap()
    }

    fn player(email: &str, link: &str) -> Player {
        Player {
            email: email.into(),
            display_email: email.to_uppercase(),
            name: Some("N".into()),
            link: link.into(),
            created_at: at("2026-11-20T00:00:00Z"),
            invited_at: None,
            last_seen: Some(at("2026-12-01T08:00:00Z")),
        }
    }

    #[tokio::test]
    async fn sqlite_store_round_trips_and_cascades() {
        let store = SqliteStore::in_memory().unwrap();
        store.put_player(&player("a@x.io", "L1")).await.unwrap();
        store.put_player(&player("b@x.io", "L2")).await.unwrap();
        assert_eq!(
            store.player_by_link("L2").await.unwrap().unwrap().email,
            "b@x.io"
        );
        let mut a = store.player("a@x.io").await.unwrap().unwrap();
        assert_eq!(a, player("a@x.io", "L1"));
        a.invited_at = Some(at("2026-11-21T00:00:00Z"));
        store.put_player(&a).await.unwrap();
        assert_eq!(store.player("a@x.io").await.unwrap().unwrap(), a);
        assert_eq!(store.players().await.unwrap().len(), 2);

        let session = Session {
            id_hash: "h".into(),
            email: "a@x.io".into(),
            expires_at: at("2027-01-01T00:00:00Z"),
        };
        store.put_session(&session).await.unwrap();
        assert_eq!(store.session("h").await.unwrap(), Some(session));

        let state = DayState {
            guesses: 2,
            window_start: Some(at("2026-12-01T09:00:00Z")),
            window_count: 2,
            solved: Some(Solved {
                word: "light".into(),
                at: at("2026-12-01T09:05:00Z"),
                hint_out: true,
            }),
        };
        store
            .put_day_state("a@x.io", "2026", 1, &state)
            .await
            .unwrap();
        store
            .put_day_state("a@x.io", "2026", 2, &DayState::default())
            .await
            .unwrap();
        assert_eq!(
            store.day_state("a@x.io", "2026", 1).await.unwrap(),
            Some(state.clone())
        );
        assert_eq!(store.player_days("a@x.io", "2026").await.unwrap().len(), 2);
        assert_eq!(store.event_days("2026").await.unwrap().len(), 2);
        assert!(store.event_days("2027").await.unwrap().is_empty());

        store.remove_player("a@x.io").await.unwrap();
        assert_eq!(store.session("h").await.unwrap(), None);
        assert!(store.event_days("2026").await.unwrap().is_empty());
        assert_eq!(store.players().await.unwrap().len(), 1);
    }

    #[test]
    fn sqlite_store_migrates_once() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("db/advent.db");
        SqliteStore::open(&path).unwrap();
        // Opening again finds the schema in place.
        SqliteStore::open(&path).unwrap();
    }
}
