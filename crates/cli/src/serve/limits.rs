//! Per-address request limits, to keep the server up under abuse. Fixed
//! windows in memory: simple, and plenty for one server process.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Request,
    Guess,
    SignIn,
}

/// Entries kept before expired windows are swept.
const SWEEP_AT: usize = 10_000;

pub struct Limiter {
    rules: HashMap<Kind, (u32, Duration)>,
    windows: Mutex<HashMap<(Kind, IpAddr), (Instant, u32)>>,
}

impl Limiter {
    pub fn new(rules: impl IntoIterator<Item = (Kind, u32, Duration)>) -> Self {
        Self {
            rules: rules.into_iter().map(|(k, n, w)| (k, (n, w))).collect(),
            windows: Mutex::new(HashMap::new()),
        }
    }

    /// Count a request; `Err` holds how long until the address may retry.
    pub fn check(&self, kind: Kind, ip: IpAddr, now: Instant) -> Result<(), Duration> {
        let Some(&(max, window)) = self.rules.get(&kind) else {
            return Ok(());
        };
        let mut windows = self.windows.lock().unwrap_or_else(|e| e.into_inner());
        if windows.len() >= SWEEP_AT {
            windows.retain(|(k, _), (start, _)| {
                let w = self.rules.get(k).map_or(Duration::ZERO, |r| r.1);
                now.duration_since(*start) < w
            });
        }
        let entry = windows.entry((kind, ip)).or_insert((now, 0));
        if now.duration_since(entry.0) >= window {
            *entry = (now, 0);
        }
        if entry.1 >= max {
            return Err(window - now.duration_since(entry.0));
        }
        entry.1 += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serve_limiter_counts_per_address_and_window() {
        let limiter = Limiter::new([(Kind::Guess, 2, Duration::from_secs(60))]);
        let a: IpAddr = "10.0.0.1".parse().unwrap();
        let b: IpAddr = "10.0.0.2".parse().unwrap();
        let t = Instant::now();
        assert!(limiter.check(Kind::Guess, a, t).is_ok());
        assert!(limiter.check(Kind::Guess, a, t).is_ok());
        assert_eq!(
            limiter.check(Kind::Guess, a, t + Duration::from_secs(20)),
            Err(Duration::from_secs(40))
        );
        assert!(limiter.check(Kind::Guess, b, t).is_ok(), "another address");
        assert!(
            limiter.check(Kind::Request, a, t).is_ok(),
            "no rule, no limit"
        );
        assert!(
            limiter
                .check(Kind::Guess, a, t + Duration::from_secs(60))
                .is_ok()
        );
    }
}
