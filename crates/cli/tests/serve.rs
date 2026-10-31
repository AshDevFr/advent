//! `advent serve` over the sealed sample event, driven over HTTP.

mod common;

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use common::{Repo, sample_event};
use reqwest::StatusCode;
use reqwest::blocking::Client;
use serde_json::{Value, json};

struct Server {
    child: Child,
    url: String,
    data: PathBuf,
    _data_dir: tempfile::TempDir,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn command(repo: &Repo, now: &str, data: &Path) -> Command {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin("advent"));
    cmd.args(["serve", "--listen", "127.0.0.1:0", "--data"])
        .arg(data)
        .arg("--repo")
        .arg(&repo.root)
        .env("ADVENT_RELEASE_KEY", &repo.release_secret)
        .env("ADVENT_SERVE_NOW", now)
        .env("ADVENT_INSECURE_COOKIES", "true")
        .env_remove("ADVENT_EDITORS_KEY")
        .env_remove("ADVENT_BASE");
    cmd
}

/// Start the server with its clock at `now` and wait until it listens.
fn start(repo: &Repo, now: &str, env: &[(&str, &str)]) -> Server {
    let data_dir = tempfile::tempdir().unwrap();
    let data = data_dir.path().to_path_buf();
    let mut cmd = command(repo, now, &data);
    for (k, v) in env {
        cmd.env(k, v);
    }
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut line = String::new();
    BufReader::new(stdout).read_line(&mut line).unwrap();
    let url = line
        .split_whitespace()
        .find(|w| w.starts_with("http://"))
        .unwrap_or_else(|| panic!("server did not start: {line:?}"))
        .trim_end_matches('/')
        .to_string();
    Server {
        child,
        url,
        data,
        _data_dir: data_dir,
    }
}

impl Server {
    fn get(&self, client: &Client, path: &str) -> reqwest::blocking::Response {
        client.get(format!("{}{path}", self.url)).send().unwrap()
    }

    fn post(&self, client: &Client, path: &str, body: Value) -> reqwest::blocking::Response {
        client
            .post(format!("{}{path}", self.url))
            .json(&body)
            .send()
            .unwrap()
    }

    /// Invite a player straight into the database (the admin section does
    /// this in real use); returns their link.
    fn invite(&self, email: &str) -> String {
        let db = rusqlite::Connection::open(self.data.join("advent.db")).unwrap();
        let link = format!("link-{email}");
        db.execute(
            "INSERT INTO players (email, display_email, link, created_at) VALUES (?1, ?1, ?2, '2026-11-20T00:00:00Z')",
            [email, &link],
        )
        .unwrap();
        link
    }

    fn signed_in(&self, link: &str) -> Client {
        let client = Client::builder().cookie_store(true).build().unwrap();
        let response = self.post(&client, "/api/signin", json!({ "link": link }));
        assert_eq!(response.status(), StatusCode::OK);
        let cookie = response
            .headers()
            .get("set-cookie")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert!(cookie.contains("HttpOnly") && cookie.contains("SameSite=Lax"));
        client
    }
}

fn client() -> Client {
    Client::new()
}

#[test]
fn serve_hosts_released_days_without_locks() {
    let repo = sample_event();
    let server = start(&repo, "2026-12-03T12:00:00Z", &[]);
    let c = client();

    assert_eq!(
        server.get(&c, "/2026/api/day/3.json").status(),
        StatusCode::OK
    );
    assert_eq!(
        server.get(&c, "/2026/api/day/4.json").status(),
        StatusCode::NOT_FOUND
    );
    for n in 1..=7 {
        let path = format!("/2026/api/day/{n}/locks.json");
        assert_eq!(
            server.get(&c, &path).status(),
            StatusCode::NOT_FOUND,
            "{path}"
        );
    }
    let site: Value = server.get(&c, "/site.json").json().unwrap();
    assert_eq!(site["mode"], "server");
    let page = server.get(&c, "/");
    assert_eq!(page.status(), StatusCode::OK);
    assert!(page.text().unwrap().contains("<html"));
    assert_eq!(
        server.get(&c, "/api/nothing").status(),
        StatusCode::NOT_FOUND
    );
}

#[test]
fn serve_checks_guesses_and_keeps_progress_per_player() {
    let repo = sample_event();
    let server = start(&repo, "2026-12-03T12:00:00Z", &[]);
    let anonymous = client();
    assert_eq!(
        server
            .post(
                &anonymous,
                "/api/events/2026/days/2/guess",
                json!({ "answer": "55" })
            )
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        server.get(&anonymous, "/api/events/2026/progress").status(),
        StatusCode::UNAUTHORIZED
    );

    let link = server.invite("alice@acme.com");
    assert_eq!(
        server
            .post(&anonymous, "/api/signin", json!({ "link": "nope" }))
            .status(),
        StatusCode::NOT_FOUND
    );
    let laptop = server.signed_in(&link);
    let me: Value = server.get(&laptop, "/api/session").json().unwrap();
    assert_eq!(me["email"], "alice@acme.com");

    let wrong: Value = server
        .post(
            &laptop,
            "/api/events/2026/days/2/guess",
            json!({ "answer": "54" }),
        )
        .json()
        .unwrap();
    assert_eq!(wrong, json!({ "wrong": true, "remaining": null }));
    let right: Value = server
        .post(
            &laptop,
            "/api/events/2026/days/2/guess",
            json!({ "answer": "55" }),
        )
        .json()
        .unwrap();
    assert_eq!(right, json!({ "word": "returns" }));
    assert_eq!(
        server
            .post(
                &laptop,
                "/api/events/2026/days/4/guess",
                json!({ "answer": "x" })
            )
            .status(),
        StatusCode::FORBIDDEN,
        "day 4 is not open"
    );

    let phone = server.signed_in(&link);
    let progress: Value = server
        .get(&phone, "/api/events/2026/progress")
        .json()
        .unwrap();
    assert_eq!(progress["words"]["2"], "returns");
    assert_eq!(progress["guesses"]["2"], 2);

    let named = server.client_put(&phone, "/api/name", json!({ "name": "Alice" }));
    assert_eq!(named["name"], "Alice");

    assert_eq!(
        server.post(&phone, "/api/signout", json!({})).status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        server.get(&phone, "/api/session").status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(server.get(&laptop, "/api/session").status(), StatusCode::OK);
}

impl Server {
    fn client_put(&self, client: &Client, path: &str, body: Value) -> Value {
        client
            .put(format!("{}{path}", self.url))
            .json(&body)
            .send()
            .unwrap()
            .json()
            .unwrap()
    }
}

#[test]
fn serve_link_requests_answer_the_same_for_anyone() {
    let repo = sample_event();
    let server = start(&repo, "2026-12-03T12:00:00Z", &[]);
    server.invite("alice@acme.com");
    let c = client();
    let invited = server.post(
        &c,
        "/api/signin/request",
        json!({ "email": "Alice@acme.com" }),
    );
    let unknown = server.post(
        &c,
        "/api/signin/request",
        json!({ "email": "eve@acme.com" }),
    );
    assert_eq!(invited.status(), StatusCode::ACCEPTED);
    assert_eq!(unknown.status(), StatusCode::ACCEPTED);
    assert_eq!(invited.text().unwrap(), unknown.text().unwrap());
}

#[test]
fn serve_refuses_to_start_on_broken_content() {
    let repo = sample_event();
    let data = tempfile::tempdir().unwrap();

    let sealed = repo.day_dir(3).join("sealed.json");
    let good = fs::read(&sealed).unwrap();
    fs::write(&sealed, b"{ not json").unwrap();
    let out = command(&repo, "2026-12-03T12:00:00Z", data.path())
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("day03") || stderr.contains("day 3"),
        "{stderr}"
    );
    fs::write(&sealed, good).unwrap();

    let (other, _) = advent_core::wrap::generate_identity();
    let out = command(&repo, "2026-12-03T12:00:00Z", data.path())
        .env("ADVENT_RELEASE_KEY", other)
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("ADVENT_RELEASE_KEY"), "{stderr}");
}

#[test]
fn serve_publishes_days_and_hints_when_their_moment_comes() {
    let repo = sample_event();
    let server = start(&repo, "2026-12-02T23:59:58Z", &[]);
    let c = client();
    let hints = |n: u32| -> usize {
        let v: Value = server
            .get(&c, &format!("/2026/api/day/{n}/hint.json"))
            .json()
            .unwrap();
        v["hints"].as_array().unwrap().len()
    };
    assert_eq!(hints(2), 0);
    assert_eq!(
        server.get(&c, "/2026/api/day/3.json").status(),
        StatusCode::NOT_FOUND
    );

    let deadline = Instant::now() + Duration::from_secs(20);
    while server.get(&c, "/2026/api/day/3.json").status() != StatusCode::OK {
        assert!(Instant::now() < deadline, "day 3 never opened");
        std::thread::sleep(Duration::from_millis(250));
    }
    assert_eq!(
        hints(2),
        1,
        "day 2's first hint came out at the same midnight"
    );
}

#[test]
fn serve_limits_guesses_per_address() {
    let repo = sample_event();
    let server = start(
        &repo,
        "2026-12-03T12:00:00Z",
        &[("ADVENT_LIMITS__GUESSES_PER_MINUTE", "5")],
    );
    let player = server.signed_in(&server.invite("alice@acme.com"));
    for i in 0..5 {
        let status = server
            .post(
                &player,
                "/api/events/2026/days/1/guess",
                json!({ "answer": format!("no{i}") }),
            )
            .status();
        assert_eq!(status, StatusCode::OK, "guess {i}");
    }
    assert_eq!(
        server
            .post(
                &player,
                "/api/events/2026/days/1/guess",
                json!({ "answer": "keyboard" })
            )
            .status(),
        StatusCode::TOO_MANY_REQUESTS
    );
}

#[test]
fn serve_applies_the_event_attempt_limit() {
    let repo = sample_event();
    let event = repo.event_dir().join("event.yaml");
    let text = fs::read_to_string(&event).unwrap() + "attempts:\n  max: 2\n  per: 1h\n";
    fs::write(&event, text).unwrap();
    let server = start(&repo, "2026-12-03T12:00:00Z", &[]);
    let player = server.signed_in(&server.invite("alice@acme.com"));
    let guess = |answer: &str| {
        server.post(
            &player,
            "/api/events/2026/days/1/guess",
            json!({ "answer": answer }),
        )
    };
    let first: Value = guess("no").json().unwrap();
    assert_eq!(first["remaining"], 1);
    guess("nope");
    let limited = guess("keyboard");
    assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);
    let body: Value = limited.json().unwrap();
    // An hour after the first guess, which came moments after 12:00.
    assert!(
        body["retry_at"]
            .as_str()
            .unwrap()
            .starts_with("2026-12-03T13:00"),
        "{body}"
    );
}
