//! The admin section's API in password mode.

mod common;

use std::path::Path;
use std::time::{Duration, Instant};

use common::sample_event;
use common::server::{ADMIN_PASSWORD, Server, client, command, cookie_client, start};
use reqwest::StatusCode;
use reqwest::blocking::Client;
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

fn admin(server: &Server) -> Client {
    let c = cookie_client();
    let response = server.post(
        &c,
        "/api/admin/login",
        json!({ "password": ADMIN_PASSWORD }),
    );
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let cookie = response.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .to_string();
    assert!(cookie.contains("HttpOnly") && cookie.contains("SameSite=Strict"));
    c
}

fn import(server: &Server, c: &Client, text: &str) -> Value {
    server
        .post(c, "/api/admin/import", json!({ "text": text }))
        .json()
        .unwrap()
}

/// A player's current link, as the invitation email would carry it.
fn link_of(server: &Server, email: &str) -> String {
    let db = rusqlite::Connection::open(server.data.join("advent.db")).unwrap();
    db.query_row("SELECT link FROM players WHERE email = ?1", [email], |r| {
        r.get(0)
    })
    .unwrap()
}

fn solve(server: &Server, player: &Client, days: std::ops::RangeInclusive<usize>) {
    for day in days {
        let reply: Value = server
            .post(
                player,
                &format!("/api/events/2026/days/{day}/guess"),
                json!({ "answer": ANSWERS[day - 1] }),
            )
            .json()
            .unwrap();
        assert!(reply["word"].is_string(), "day {day}: {reply}");
    }
}

fn logged(dir: &Path) -> Vec<Value> {
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .map(|d| d.filter_map(|e| e.ok().map(|e| e.path())).collect())
        .unwrap_or_default();
    files.sort();
    files
        .iter()
        .map(|p| serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap())
        .collect()
}

#[test]
fn admin_password_is_required_and_limited() {
    let repo = sample_event();
    let data = tempfile::tempdir().unwrap();
    let out = command(&repo, "2026-12-03T12:00:00Z", data.path())
        .env_remove("ADVENT_ADMIN__PASSWORD")
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("ADVENT_ADMIN__PASSWORD"));

    let server = start(&repo, "2026-12-03T12:00:00Z", &[]);
    let c = client();
    assert_eq!(
        server.get(&c, "/api/admin/session").status(),
        StatusCode::UNAUTHORIZED
    );
    for i in 0..5 {
        let status = server
            .post(
                &c,
                "/api/admin/login",
                json!({ "password": format!("guess{i}") }),
            )
            .status();
        assert_eq!(status, StatusCode::UNAUTHORIZED, "attempt {i}");
    }
    assert_eq!(
        server
            .post(
                &c,
                "/api/admin/login",
                json!({ "password": ADMIN_PASSWORD })
            )
            .status(),
        StatusCode::TOO_MANY_REQUESTS,
        "even the right password waits once the limit is reached"
    );
}

#[test]
fn admin_imports_and_invites() {
    let repo = sample_event();
    let outbox = tempfile::tempdir().unwrap();
    let server = start(
        &repo,
        "2026-12-03T12:00:00Z",
        &[
            ("ADVENT_MAIL__LOG_DIR", outbox.path().to_str().unwrap()),
            ("ADVENT_PUBLIC_URL", "https://advent.acme.com"),
        ],
    );
    let c = admin(&server);
    let paste = "bob@acme.com\nAlice@acme.com,Alice\nnot-an-email\n";

    let preview: Value = server
        .post(&c, "/api/admin/import/preview", json!({ "text": paste }))
        .json()
        .unwrap();
    assert_eq!(preview["new"].as_array().unwrap().len(), 2);
    assert_eq!(preview["invalid"], json!(["not-an-email"]));
    assert!(
        server
            .get(&c, "/api/admin/events/2026/players")
            .json::<Value>()
            .unwrap()
            .as_array()
            .unwrap()
            .is_empty(),
        "a preview changes nothing"
    );

    let report = import(&server, &c, paste);
    assert_eq!(report["created"].as_array().unwrap().len(), 2);

    let started = server.post(&c, "/api/admin/invitations", json!({}));
    assert_eq!(started.status(), StatusCode::ACCEPTED);
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let job: Value = server.get(&c, "/api/admin/invitations").json().unwrap();
        if job["running"] == false {
            assert_eq!(job["sent"], 2);
            assert!(job["failed"].as_array().unwrap().is_empty());
            break;
        }
        assert!(Instant::now() < deadline, "invitations never finished");
        std::thread::sleep(Duration::from_millis(100));
    }
    let emails = logged(outbox.path());
    assert_eq!(emails.len(), 2);
    let texts: Vec<&str> = emails.iter().map(|e| e["text"].as_str().unwrap()).collect();
    assert!(texts[0].contains(&link_of(
        &server,
        &emails[0]["to"].as_str().unwrap().to_lowercase()
    )));
    assert_ne!(texts[0], texts[1], "each player gets their own link");

    let players: Value = server
        .get(&c, "/api/admin/events/2026/players")
        .json()
        .unwrap();
    assert!(
        players
            .as_array()
            .unwrap()
            .iter()
            .all(|p| p["invited_at"].is_string())
    );

    // Sending again only goes to players not invited yet: nobody.
    server.post(&c, "/api/admin/invitations", json!({}));
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(logged(outbox.path()).len(), 2);
}

#[test]
fn admin_sees_progress_stats_and_exports() {
    let repo = sample_event();
    // Every day of the sample event is open on the 7th.
    let server = start(&repo, "2026-12-07T12:00:00Z", &[]);
    let c = admin(&server);
    import(
        &server,
        &c,
        "alice@acme.com\nbob@acme.com\ncarol@acme.com\ndave@acme.com\n",
    );
    let alice = server.signed_in(&link_of(&server, "alice@acme.com"));
    let bob = server.signed_in(&link_of(&server, "bob@acme.com"));
    let _carol = server.signed_in(&link_of(&server, "carol@acme.com"));
    solve(&server, &alice, 1..=7);
    solve(&server, &bob, 1..=1);
    // A name a spreadsheet would run as a formula.
    alice
        .put(format!("{}/api/name", server.url))
        .json(&json!({ "name": "=HYPERLINK(\"x\")" }))
        .send()
        .unwrap();

    let stats: Value = server
        .get(&c, "/api/admin/events/2026/stats")
        .json()
        .unwrap();
    assert_eq!(stats["players"], 4);
    assert_eq!(stats["signed_in"], 3);
    assert_eq!(stats["finished"], 1);
    assert_eq!(stats["days"][0]["solved"], 2);
    let completion = stats["days"][0]["completion"].as_f64().unwrap();
    assert!((completion - 2.0 / 3.0).abs() < 1e-9, "{completion}");

    let rows: Value = server
        .get(&c, "/api/admin/events/2026/players")
        .json()
        .unwrap();
    let bob_row = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["email"] == "bob@acme.com")
        .unwrap();
    assert_eq!(bob_row["solved"], 1);
    let detail: Value = server
        .get(&c, "/api/admin/events/2026/players/alice@acme.com")
        .json()
        .unwrap();
    assert_eq!(detail["days"].as_array().unwrap().len(), 7);
    assert_eq!(detail["days"][0]["guesses"], 1);

    let events: Value = server.get(&c, "/api/admin/events").json().unwrap();
    assert_eq!(events[0]["days"][0]["date"], "2026-12-01");
    assert!(
        events[0]["days"][0]["title"].is_string(),
        "released days show their title"
    );

    let finishers = server.get(&c, "/api/admin/events/2026/finishers.csv");
    assert!(
        finishers.headers()["content-type"]
            .to_str()
            .unwrap()
            .starts_with("text/csv")
    );
    let text = finishers.text().unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2, "{text}");
    assert_eq!(lines[0], "name,email,finished_at");
    assert!(
        lines[1].starts_with("\"'=HYPERLINK(\"\"x\"\")\",alice@acme.com,2026-12-07T"),
        "{text}"
    );

    let players_csv = server
        .get(&c, "/api/admin/events/2026/players.csv")
        .text()
        .unwrap();
    assert_eq!(players_csv.lines().count(), 5);
}

#[test]
fn admin_resets_a_link_and_removes_a_player() {
    let repo = sample_event();
    let server = start(&repo, "2026-12-03T12:00:00Z", &[]);
    let c = admin(&server);
    import(&server, &c, "alice@acme.com\n");
    let old = link_of(&server, "alice@acme.com");
    let player = server.signed_in(&old);

    let reset = server.post(&c, "/api/admin/players/alice@acme.com/reset", json!({}));
    assert_eq!(reset.status(), StatusCode::OK);
    assert_eq!(
        server.get(&player, "/api/session").status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        server
            .get(&client(), &format!("/api/signin/{old}"))
            .status(),
        StatusCode::NOT_FOUND
    );
    let fresh = link_of(&server, "alice@acme.com");
    assert_ne!(fresh, old);
    server.signed_in(&fresh);

    let removed = c
        .delete(format!("{}/api/admin/players/alice@acme.com", server.url))
        .send()
        .unwrap();
    assert_eq!(removed.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        server
            .get(&client(), &format!("/api/signin/{fresh}"))
            .status(),
        StatusCode::NOT_FOUND
    );
}

#[test]
fn admin_and_player_sessions_are_separate() {
    let repo = sample_event();
    let server = start(&repo, "2026-12-03T12:00:00Z", &[]);
    let c = admin(&server);
    import(&server, &c, "alice@acme.com\n");
    let player = server.signed_in(&link_of(&server, "alice@acme.com"));

    assert_eq!(
        server.get(&player, "/api/admin/events").status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        server.get(&c, "/api/session").status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        server.get(&c, "/api/admin/session").status(),
        StatusCode::NO_CONTENT
    );

    // The admin pages are the app.
    let page = server.get(&client(), "/admin/players");
    assert_eq!(page.status(), StatusCode::OK);
    assert!(page.text().unwrap().contains("<html"));

    server.post(&c, "/api/admin/logout", json!({}));
    assert_eq!(
        server.get(&c, "/api/admin/session").status(),
        StatusCode::UNAUTHORIZED
    );
}
