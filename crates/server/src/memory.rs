//! A [`Store`] kept in memory: for tests, and as the reference for what
//! the database adapters must do.

use std::collections::BTreeMap;
use std::sync::Mutex;

use crate::store::{DayState, Player, Session, Store, StoreResult};

#[derive(Default)]
struct Inner {
    players: BTreeMap<String, Player>,
    sessions: BTreeMap<String, Session>,
    days: BTreeMap<(String, String, u32), DayState>,
}

#[derive(Default)]
pub struct MemoryStore {
    inner: Mutex<Inner>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl Store for MemoryStore {
    async fn player(&self, email: &str) -> StoreResult<Option<Player>> {
        Ok(self.lock().players.get(email).cloned())
    }

    async fn player_by_link(&self, token: &str) -> StoreResult<Option<Player>> {
        Ok(self
            .lock()
            .players
            .values()
            .find(|p| p.link == token)
            .cloned())
    }

    async fn players(&self) -> StoreResult<Vec<Player>> {
        Ok(self.lock().players.values().cloned().collect())
    }

    async fn put_player(&self, player: &Player) -> StoreResult<()> {
        self.lock()
            .players
            .insert(player.email.clone(), player.clone());
        Ok(())
    }

    async fn remove_player(&self, email: &str) -> StoreResult<()> {
        let mut inner = self.lock();
        inner.players.remove(email);
        inner.sessions.retain(|_, s| s.email != email);
        inner.days.retain(|(e, _, _), _| e != email);
        Ok(())
    }

    async fn session(&self, id_hash: &str) -> StoreResult<Option<Session>> {
        Ok(self.lock().sessions.get(id_hash).cloned())
    }

    async fn put_session(&self, session: &Session) -> StoreResult<()> {
        self.lock()
            .sessions
            .insert(session.id_hash.clone(), session.clone());
        Ok(())
    }

    async fn remove_session(&self, id_hash: &str) -> StoreResult<()> {
        self.lock().sessions.remove(id_hash);
        Ok(())
    }

    async fn remove_sessions(&self, email: &str) -> StoreResult<()> {
        self.lock().sessions.retain(|_, s| s.email != email);
        Ok(())
    }

    async fn day_state(&self, email: &str, event: &str, day: u32) -> StoreResult<Option<DayState>> {
        Ok(self
            .lock()
            .days
            .get(&(email.to_string(), event.to_string(), day))
            .cloned())
    }

    async fn put_day_state(
        &self,
        email: &str,
        event: &str,
        day: u32,
        state: &DayState,
    ) -> StoreResult<()> {
        self.lock()
            .days
            .insert((email.to_string(), event.to_string(), day), state.clone());
        Ok(())
    }

    async fn player_days(&self, email: &str, event: &str) -> StoreResult<Vec<(u32, DayState)>> {
        Ok(self
            .lock()
            .days
            .iter()
            .filter(|((e, ev, _), _)| e == email && ev == event)
            .map(|((_, _, day), state)| (*day, state.clone()))
            .collect())
    }

    async fn event_days(&self, event: &str) -> StoreResult<Vec<(String, u32, DayState)>> {
        Ok(self
            .lock()
            .days
            .iter()
            .filter(|((_, ev, _), _)| ev == event)
            .map(|((email, _, day), state)| (email.clone(), *day, state.clone()))
            .collect())
    }
}
