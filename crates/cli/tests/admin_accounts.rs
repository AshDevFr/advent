//! Admin accounts: the setup wizard with its one-time code, email sign-in
//! links, and adding and removing admins.

mod common;

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, Instant};

use common::sample_event;
use common::server::{Server, client, command, cookie_client, start};
use reqwest::StatusCode;
use reqwest::blocking::Client;
use serde_json::{Value, json};

const NOW: &str = "2026-11-20T09:00:00Z";
const CODE: &str = "TEST-CODE";

fn accounts(repo: &common::Repo, outbox: &Path) -> Server {
    start(
        repo,
        NOW,
        &[
            ("ADVENT_ADMIN__AUTH", "accounts"),
            ("ADVENT_ADMIN__SETUP_CODE", CODE),
            ("ADVENT_MAIL__LOG_DIR", outbox.to_str().unwrap()),
            ("ADVENT_PUBLIC_URL", "https://advent.acme.com"),
        ],
    )
}

/// The first admin, created through the wizard and signed in.
fn first_admin(server: &Server) -> Client {
    let c = cookie_client();
    let response = server.post(
        &c,
        "/api/admin/setup",
        json!({ "code": CODE, "email": "ana@acme.com" }),
    );
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    c
}

/// The admin sign-in links emailed so far, newest last.
fn links(outbox: &Path) -> Vec<String> {
    let mut files: Vec<_> = std::fs::read_dir(outbox)
        .map(|d| d.filter_map(|e| e.ok().map(|e| e.path())).collect())
        .unwrap_or_default();
    files.sort();
    files
        .iter()
        .filter_map(|p| {
            let email: Value = serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap();
            let text = email["text"].as_str()?.to_string();
            let start = text.find("/admin/signin/")? + "/admin/signin/".len();
            Some(text[start..].split_whitespace().next()?.to_string())
        })
        .collect()
}

fn wait_for_links(outbox: &Path, count: usize) -> Vec<String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let found = links(outbox);
        if found.len() >= count {
            return found;
        }
        assert!(Instant::now() < deadline, "no sign-in email");
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[test]
fn admin_accounts_log_a_setup_code_until_an_admin_exists() {
    let repo = sample_event();
    let data = tempfile::tempdir().unwrap();
    let mut child = command(&repo, NOW, data.path())
        .env("ADVENT_ADMIN__AUTH", "accounts")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut line = String::new();
    let mut reader = BufReader::new(child.stderr.take().unwrap());
    let code = loop {
        line.clear();
        assert!(
            reader.read_line(&mut line).unwrap() > 0,
            "no setup code logged"
        );
        if let Some(code) = line.trim().split("setup code: ").nth(1) {
            break code.to_string();
        }
    };
    let _ = child.kill();
    let _ = child.wait();
    assert_eq!(code.len(), 9, "{code}");
    assert_eq!(&code[4..5], "-");
}

#[test]
fn admin_accounts_wizard_needs_the_code_and_works_once() {
    let repo = sample_event();
    let outbox = tempfile::tempdir().unwrap();
    let server = accounts(&repo, outbox.path());
    let c = client();
    let mode: Value = server.get(&c, "/api/admin/mode").json().unwrap();
    assert_eq!(
        mode,
        json!({ "auth": "accounts", "setup_needed": true, "oidc": null })
    );

    let wrong = server.post(
        &c,
        "/api/admin/setup",
        json!({ "code": "GUESS-ME", "email": "eve@acme.com" }),
    );
    assert_eq!(wrong.status(), StatusCode::FORBIDDEN);

    let ana = first_admin(&server);
    assert_eq!(
        server.get(&ana, "/api/admin/session").status(),
        StatusCode::NO_CONTENT
    );
    let again = server.post(
        &c,
        "/api/admin/setup",
        json!({ "code": CODE, "email": "eve@acme.com" }),
    );
    assert_eq!(again.status(), StatusCode::CONFLICT);
    let mode: Value = server.get(&c, "/api/admin/mode").json().unwrap();
    assert_eq!(mode["setup_needed"], false);

    // The master password does not open accounts mode.
    let password = server.post(
        &c,
        "/api/admin/login",
        json!({ "password": common::server::ADMIN_PASSWORD }),
    );
    assert_eq!(password.status(), StatusCode::ACCEPTED);
    assert_eq!(
        server.get(&c, "/api/admin/session").status(),
        StatusCode::UNAUTHORIZED
    );
}

#[test]
fn admin_accounts_sign_in_links_work_once() {
    let repo = sample_event();
    let outbox = tempfile::tempdir().unwrap();
    let server = accounts(&repo, outbox.path());
    first_admin(&server);
    let c = client();

    let unknown = server.post(&c, "/api/admin/login", json!({ "email": "eve@acme.com" }));
    let known = server.post(&c, "/api/admin/login", json!({ "email": "ANA@acme.com" }));
    assert_eq!(unknown.status(), StatusCode::ACCEPTED);
    assert_eq!(unknown.text().unwrap(), known.text().unwrap());
    let link = wait_for_links(outbox.path(), 1).pop().unwrap();
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(links(outbox.path()).len(), 1, "nothing sent to eve");

    // Looking at the link (as the page and mail scanners do) does not use it.
    let owner: Value = server
        .get(&c, &format!("/api/admin/signin/{link}"))
        .json()
        .unwrap();
    assert_eq!(owner["email"], "ana@acme.com");

    let browser = cookie_client();
    let used = server.post(&browser, "/api/admin/signin", json!({ "link": link }));
    assert_eq!(used.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        server.get(&browser, "/api/admin/session").status(),
        StatusCode::NO_CONTENT
    );

    let reused = server.post(
        &cookie_client(),
        "/api/admin/signin",
        json!({ "link": link }),
    );
    assert_eq!(reused.status(), StatusCode::NOT_FOUND);
}

#[test]
fn admin_accounts_keep_at_least_one_admin() {
    let repo = sample_event();
    let outbox = tempfile::tempdir().unwrap();
    let server = accounts(&repo, outbox.path());
    let ana = first_admin(&server);
    let delete = |c: &Client, email: &str| {
        c.delete(format!("{}/api/admin/admins/{email}", server.url))
            .send()
            .unwrap()
            .status()
    };
    assert_eq!(delete(&ana, "ana@acme.com"), StatusCode::CONFLICT);

    let added = server.post(&ana, "/api/admin/admins", json!({ "email": "bo@acme.com" }));
    assert_eq!(added.status(), StatusCode::CREATED);
    let link = wait_for_links(outbox.path(), 1).pop().unwrap();
    let bo = cookie_client();
    server.post(&bo, "/api/admin/signin", json!({ "link": link }));
    let admins: Value = server.get(&bo, "/api/admin/admins").json().unwrap();
    assert_eq!(admins.as_array().unwrap().len(), 2);

    assert_eq!(delete(&bo, "ana@acme.com"), StatusCode::NO_CONTENT);
    assert_eq!(
        server.get(&ana, "/api/admin/session").status(),
        StatusCode::UNAUTHORIZED,
        "a removed admin is signed out"
    );
    assert_eq!(delete(&bo, "bo@acme.com"), StatusCode::CONFLICT);
}

#[test]
fn admin_accounts_routes_are_absent_in_password_mode() {
    let repo = sample_event();
    let server = start(&repo, NOW, &[]);
    let c = cookie_client();
    server.post(
        &c,
        "/api/admin/login",
        json!({ "password": common::server::ADMIN_PASSWORD }),
    );
    assert_eq!(
        server.get(&c, "/api/admin/admins").status(),
        StatusCode::NOT_FOUND
    );
    let setup = server.post(
        &client(),
        "/api/admin/setup",
        json!({ "code": "x", "email": "a@b.c" }),
    );
    assert_eq!(setup.status(), StatusCode::NOT_FOUND);
    let mode: Value = server.get(&c, "/api/admin/mode").json().unwrap();
    assert_eq!(
        mode,
        json!({ "auth": "password", "setup_needed": false, "oidc": null })
    );
}
