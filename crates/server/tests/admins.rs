//! Admin accounts: the first-admin setup, single-use sign-in links, and
//! keeping at least one admin.

use std::sync::atomic::{AtomicU64, Ordering};

use advent_server::admin::{self, ADMIN_LINK_LIFETIME};
use advent_server::memory::MemoryStore;
use advent_server::{Entropy, ServerError, Store};
use chrono::{DateTime, Duration, Utc};
use pollster::block_on;

struct Counter(AtomicU64);

impl Entropy for Counter {
    fn fill(&self, buf: &mut [u8]) {
        let n = self.0.fetch_add(1, Ordering::Relaxed).to_le_bytes();
        for (i, b) in buf.iter_mut().enumerate() {
            *b = n[i % 8];
        }
    }
}

fn now() -> DateTime<Utc> {
    "2026-11-20T09:00:00Z".parse().unwrap()
}

#[test]
fn setup_needs_the_code_and_works_once() {
    let store = MemoryStore::new();
    assert!(block_on(admin::setup_needed(&store)).unwrap());
    assert_eq!(
        block_on(admin::setup(
            &store,
            "guess",
            "K7Q2-9XPL",
            "ana@acme.com",
            now()
        ))
        .unwrap_err(),
        ServerError::WrongSetupCode
    );
    let first = block_on(admin::setup(
        &store,
        " K7Q2-9XPL ",
        "K7Q2-9XPL",
        "Ana@Acme.com",
        now(),
    ))
    .unwrap();
    assert_eq!(first.email, "ana@acme.com");
    assert!(!block_on(admin::setup_needed(&store)).unwrap());
    assert_eq!(
        block_on(admin::setup(
            &store,
            "K7Q2-9XPL",
            "K7Q2-9XPL",
            "eve@acme.com",
            now()
        ))
        .unwrap_err(),
        ServerError::SetupDone,
        "the code stops working once an admin exists"
    );
}

#[test]
fn admin_links_work_once_and_expire() {
    let store = MemoryStore::new();
    let rng = Counter(AtomicU64::new(1));
    block_on(admin::add_admin(&store, "ana@acme.com", now())).unwrap();
    assert_eq!(
        block_on(admin::admin_link(&store, &rng, "eve@acme.com", now())).unwrap(),
        None,
        "no link for someone who is not an admin"
    );

    let link = block_on(admin::admin_link(&store, &rng, "ANA@acme.com", now()))
        .unwrap()
        .unwrap();
    assert_eq!(
        block_on(admin::admin_link_owner(&store, &link, now())).unwrap(),
        "ana@acme.com",
        "looking at a link does not use it"
    );
    assert_eq!(
        block_on(admin::use_admin_link(&store, &link, now())).unwrap(),
        "ana@acme.com"
    );
    assert_eq!(
        block_on(admin::use_admin_link(&store, &link, now())).unwrap_err(),
        ServerError::InvalidAdminLink,
        "a second use is refused"
    );

    let late = block_on(admin::admin_link(&store, &rng, "ana@acme.com", now()))
        .unwrap()
        .unwrap();
    let after = now() + ADMIN_LINK_LIFETIME + Duration::seconds(1);
    assert_eq!(
        block_on(admin::use_admin_link(&store, &late, after)).unwrap_err(),
        ServerError::InvalidAdminLink
    );
}

#[test]
fn the_last_admin_stays_and_removal_ends_sessions() {
    let store = MemoryStore::new();
    let rng = Counter(AtomicU64::new(1));
    block_on(admin::add_admin(&store, "ana@acme.com", now())).unwrap();
    assert_eq!(
        block_on(admin::remove_admin(&store, "ana@acme.com")).unwrap_err(),
        ServerError::LastAdmin
    );
    block_on(admin::add_admin(&store, "bo@acme.com", now())).unwrap();
    let session = block_on(admin::admin_sign_in(
        &store,
        &rng,
        Some("bo@acme.com".into()),
        now() + Duration::hours(1),
    ))
    .unwrap();
    let pending = block_on(admin::admin_link(&store, &rng, "bo@acme.com", now()))
        .unwrap()
        .unwrap();
    block_on(admin::remove_admin(&store, "bo@acme.com")).unwrap();
    assert_eq!(
        block_on(admin::admin_session(&store, &session, now())).unwrap(),
        None
    );
    assert!(block_on(admin::use_admin_link(&store, &pending, now())).is_err());
    assert_eq!(block_on(store.admins()).unwrap().len(), 1);
}
