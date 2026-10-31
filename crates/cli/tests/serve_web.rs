//! A player's whole visit to a server-hosted calendar, over HTTP as the
//! frontend makes it: the link page, signing in, playing every day on two
//! devices, and opening the certificate with the words the server returned.

mod common;

use advent_core::format::read_sealed_final;
use advent_core::primitives;
use common::server::{client, start};
use common::{SAMPLE_WORDS, sample_event};
use reqwest::StatusCode;
use serde_json::{Value, json};

const ANSWERS: [&str; 7] = [
    "keyboard",
    "55",
    "wick the owl",
    "snowflake",
    "alba cleo",
    "frost",
    "flows",
];

#[test]
fn serve_web_link_page_signs_in_only_on_request() {
    let repo = sample_event();
    let server = start(&repo, "2026-12-03T12:00:00Z", &[]);
    let link = server.invite("alice@acme.com");
    let c = client();

    // The link page is the app, marked as server-hosted.
    let page = server.get(&c, &format!("/signin/{link}"));
    assert_eq!(page.status(), StatusCode::OK);
    assert!(
        page.text()
            .unwrap()
            .contains(r#"<meta name="advent-mode" content="server">"#)
    );

    // Looking up whose link it is (as the page does, and as a mail
    // scanner would by opening it) creates no session.
    let owner: Value = server
        .get(&c, &format!("/api/signin/{link}"))
        .json()
        .unwrap();
    assert_eq!(owner["email"], "alice@acme.com");
    let db = rusqlite::Connection::open(server.data.join("advent.db")).unwrap();
    let sessions = || -> i64 {
        db.query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0))
            .unwrap()
    };
    assert_eq!(sessions(), 0);
    assert_eq!(
        server.get(&c, "/api/signin/not-a-link").status(),
        StatusCode::NOT_FOUND
    );

    server.signed_in(&link);
    assert_eq!(sessions(), 1);
}

#[test]
fn serve_web_player_finishes_across_devices_and_opens_the_certificate() {
    let repo = sample_event();
    // Every day of the sample event is open on the 7th.
    let server = start(&repo, "2026-12-07T12:00:00Z", &[]);
    let link = server.invite("alice@acme.com");
    let db = rusqlite::Connection::open(server.data.join("advent.db")).unwrap();
    db.execute(
        "UPDATE players SET name = 'Alice Martin' WHERE email = 'alice@acme.com'",
        [],
    )
    .unwrap();

    let laptop = server.signed_in(&link);
    let phone = server.signed_in(&link);
    for (i, answer) in ANSWERS.iter().enumerate() {
        let day = i + 1;
        // Alternate devices: progress is the server's, not the browser's.
        let device = if day % 2 == 0 { &phone } else { &laptop };
        let reply: Value = server
            .post(
                device,
                &format!("/api/events/2026/days/{day}/guess"),
                json!({ "answer": answer }),
            )
            .json()
            .unwrap();
        assert_eq!(reply["word"], SAMPLE_WORDS[i], "day {day}");
    }

    let progress: Value = server
        .get(&laptop, "/api/events/2026/progress")
        .json()
        .unwrap();
    assert_eq!(
        progress["name"], "Alice Martin",
        "the invited name prefills"
    );
    assert!(progress["finished_at"].is_string());
    let words: Vec<&str> = (1..=7)
        .map(|d| progress["words"][d.to_string()].as_str().unwrap())
        .collect();

    // The browser opens the certificate itself, with those words.
    let sealed = server.get(&laptop, "/2026/api/final.json").bytes().unwrap();
    let sealed = read_sealed_final(&sealed).unwrap();
    let certificate = primitives::open_final(&words, sealed.salt, sealed.iv, &sealed.ct);
    assert!(
        certificate.is_ok(),
        "the server's words open the certificate"
    );
}
