//! Running `advent serve` for a test and talking to it over HTTP.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use reqwest::StatusCode;
use reqwest::blocking::Client;
use serde_json::{Value, json};

use super::Repo;

pub const ADMIN_PASSWORD: &str = "admin-test-password";

pub struct Server {
    pub child: Child,
    pub url: String,
    pub data: PathBuf,
    _data_dir: tempfile::TempDir,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub fn command(repo: &Repo, now: &str, data: &Path) -> Command {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin("advent"));
    cmd.args(["serve", "--listen", "127.0.0.1:0", "--data"])
        .arg(data)
        .arg("--repo")
        .arg(&repo.root)
        .env("ADVENT_RELEASE_KEY", &repo.release_secret)
        .env("ADVENT_SERVE_NOW", now)
        .env("ADVENT_INSECURE_COOKIES", "true")
        .env("ADVENT_ADMIN__PASSWORD", ADMIN_PASSWORD)
        .env_remove("ADVENT_EDITORS_KEY")
        .env_remove("ADVENT_BASE");
    cmd
}

/// Start the server with its clock at `now` and wait until it listens.
pub fn start(repo: &Repo, now: &str, env: &[(&str, &str)]) -> Server {
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
    pub fn get(&self, client: &Client, path: &str) -> reqwest::blocking::Response {
        client.get(format!("{}{path}", self.url)).send().unwrap()
    }

    pub fn post(&self, client: &Client, path: &str, body: Value) -> reqwest::blocking::Response {
        client
            .post(format!("{}{path}", self.url))
            .json(&body)
            .send()
            .unwrap()
    }

    /// Invite a player straight into the database (the admin section does
    /// this in real use); returns their link.
    pub fn invite(&self, email: &str) -> String {
        let db = rusqlite::Connection::open(self.data.join("advent.db")).unwrap();
        let link = format!("link-{email}");
        db.execute(
            "INSERT INTO players (email, display_email, link, created_at) VALUES (?1, ?1, ?2, '2026-11-20T00:00:00Z')",
            [email, &link],
        )
        .unwrap();
        link
    }

    pub fn signed_in(&self, link: &str) -> Client {
        let client = cookie_client();
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

pub fn client() -> Client {
    install_crypto();
    Client::new()
}

/// A client that keeps cookies, like a browser.
pub fn cookie_client() -> Client {
    install_crypto();
    Client::builder().cookie_store(true).build().unwrap()
}

/// reqwest is built without a bundled TLS provider (the server picks
/// ring for its email clients), so the test client needs one too.
pub fn install_crypto() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}
