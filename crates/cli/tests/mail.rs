//! Sign-in links by email from `advent serve`: the log sender, SMTP and
//! Resend, each against a local stand-in.

mod common;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use common::sample_event;
use common::server::{client, command, start};
use reqwest::StatusCode;
use serde_json::{Value, json};

const NOW: &str = "2026-12-03T12:00:00Z";

/// The JSON files the log sender wrote, in order.
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

/// Wait until `check` holds; sending happens after the response.
fn eventually(what: &str, mut check: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !check() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[test]
fn mail_link_goes_to_invited_addresses_only() {
    let repo = sample_event();
    let outbox = tempfile::tempdir().unwrap();
    let dir = outbox.path().to_str().unwrap();
    let server = start(
        &repo,
        NOW,
        &[
            ("ADVENT_MAIL__LOG_DIR", dir),
            ("ADVENT_PUBLIC_URL", "https://advent.acme.com"),
        ],
    );
    let link = server.invite("alice@acme.com");
    let c = client();

    let unknown = server.post(
        &c,
        "/api/signin/request",
        json!({ "email": "eve@acme.com" }),
    );
    let invited = server.post(
        &c,
        "/api/signin/request",
        json!({ "email": " ALICE@acme.com" }),
    );
    assert_eq!(unknown.status(), StatusCode::ACCEPTED);
    assert_eq!(invited.status(), StatusCode::ACCEPTED);
    assert_eq!(unknown.text().unwrap(), invited.text().unwrap());

    eventually("the email", || !logged(outbox.path()).is_empty());
    std::thread::sleep(Duration::from_millis(300));
    let emails = logged(outbox.path());
    assert_eq!(
        emails.len(),
        1,
        "nothing for the unknown address: {emails:?}"
    );
    assert_eq!(emails[0]["to"], "alice@acme.com");
    let url = format!("https://advent.acme.com/signin/{link}");
    assert!(emails[0]["text"].as_str().unwrap().contains(&url));
    assert!(emails[0]["html"].as_str().unwrap().contains(&url));
    assert!(
        emails[0]["subject"]
            .as_str()
            .unwrap()
            .contains("Advent Calendar")
    );
}

#[test]
fn mail_links_per_address_are_limited() {
    let repo = sample_event();
    let outbox = tempfile::tempdir().unwrap();
    let server = start(
        &repo,
        NOW,
        &[("ADVENT_MAIL__LOG_DIR", outbox.path().to_str().unwrap())],
    );
    server.invite("alice@acme.com");
    let c = client();
    for _ in 0..4 {
        let response = server.post(
            &c,
            "/api/signin/request",
            json!({ "email": "alice@acme.com" }),
        );
        assert_eq!(response.status(), StatusCode::ACCEPTED);
    }
    eventually("three emails", || logged(outbox.path()).len() >= 3);
    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(logged(outbox.path()).len(), 3);
}

/// The smallest SMTP server that accepts one message, sent back whole.
fn smtp_sink() -> (u16, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut out = stream.try_clone().unwrap();
        let mut reader = BufReader::new(stream);
        let mut transcript = String::new();
        out.write_all(b"220 sink ESMTP\r\n").unwrap();
        let mut line = String::new();
        while reader.read_line(&mut line).unwrap_or(0) > 0 {
            transcript.push_str(&line);
            let command = line.to_uppercase();
            let reply: &[u8] = if command.starts_with("EHLO") || command.starts_with("HELO") {
                b"250-sink\r\n250 OK\r\n"
            } else if command.starts_with("DATA") {
                out.write_all(b"354 go on\r\n").unwrap();
                let mut data = String::new();
                loop {
                    let mut l = String::new();
                    reader.read_line(&mut l).unwrap();
                    if l == ".\r\n" {
                        break;
                    }
                    data.push_str(&l);
                }
                transcript.push_str(&data);
                b"250 queued\r\n"
            } else if command.starts_with("QUIT") {
                out.write_all(b"221 bye\r\n").unwrap();
                break;
            } else {
                b"250 OK\r\n"
            };
            out.write_all(reply).unwrap();
            line.clear();
        }
        let _ = tx.send(transcript);
    });
    (port, rx)
}

#[test]
fn mail_smtp_delivers_through_the_relay() {
    let repo = sample_event();
    let (port, received) = smtp_sink();
    let port = port.to_string();
    let server = start(
        &repo,
        NOW,
        &[
            ("ADVENT_MAIL__SENDER", "smtp"),
            ("ADVENT_MAIL__FROM", "Advent <advent@acme.com>"),
            ("ADVENT_MAIL__SMTP__HOST", "127.0.0.1"),
            ("ADVENT_MAIL__SMTP__PORT", &port),
            ("ADVENT_MAIL__SMTP__TLS", "none"),
            ("ADVENT_PUBLIC_URL", "https://advent.acme.com"),
        ],
    );
    let link = server.invite("alice@acme.com");
    server.post(
        &client(),
        "/api/signin/request",
        json!({ "email": "alice@acme.com" }),
    );
    let transcript = received.recv_timeout(Duration::from_secs(10)).unwrap();
    assert!(
        transcript.contains("MAIL FROM:<advent@acme.com>"),
        "{transcript}"
    );
    assert!(
        transcript.contains("RCPT TO:<alice@acme.com>"),
        "{transcript}"
    );
    assert!(
        transcript.contains("From: Advent <advent@acme.com>"),
        "{transcript}"
    );
    // Quoted-printable may wrap long lines; the token itself is intact.
    assert!(transcript.contains(&link), "{transcript}");
}

/// A stand-in for the Resend API: answers one request, sends it back.
fn resend_mock() -> (String, mpsc::Receiver<(String, Value)>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut out = stream.try_clone().unwrap();
        let mut reader = BufReader::new(stream);
        let mut head = String::new();
        let mut length = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if let Some(v) = line.to_lowercase().strip_prefix("content-length:") {
                length = v.trim().parse().unwrap();
            }
            if line == "\r\n" {
                break;
            }
            head.push_str(&line);
        }
        let mut body = vec![0; length];
        reader.read_exact(&mut body).unwrap();
        out.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 11\r\n\r\n{\"id\":\"1\"}\n")
            .unwrap();
        let _ = tx.send((head, serde_json::from_slice(&body).unwrap()));
    });
    (url, rx)
}

#[test]
fn mail_resend_sends_with_the_api_key() {
    let repo = sample_event();
    let (endpoint, received) = resend_mock();
    let server = start(
        &repo,
        NOW,
        &[
            ("ADVENT_MAIL__SENDER", "resend"),
            ("ADVENT_MAIL__FROM", "Advent <advent@acme.com>"),
            ("ADVENT_MAIL__RESEND__API_KEY", "re_test_key"),
            ("ADVENT_MAIL__RESEND__ENDPOINT", &endpoint),
            ("ADVENT_PUBLIC_URL", "https://advent.acme.com"),
        ],
    );
    let link = server.invite("alice@acme.com");
    server.post(
        &client(),
        "/api/signin/request",
        json!({ "email": "alice@acme.com" }),
    );
    let (head, body) = received.recv_timeout(Duration::from_secs(10)).unwrap();
    assert!(head.starts_with("POST /emails "), "{head}");
    assert!(
        head.to_lowercase()
            .contains("authorization: bearer re_test_key"),
        "{head}"
    );
    assert_eq!(body["from"], "Advent <advent@acme.com>");
    assert_eq!(body["to"], json!(["alice@acme.com"]));
    assert!(body["text"].as_str().unwrap().contains(&link));
}

#[test]
fn mail_sender_needs_a_public_url() {
    let repo = sample_event();
    let data = tempfile::tempdir().unwrap();
    let out = command(&repo, NOW, data.path())
        .env("ADVENT_MAIL__SENDER", "smtp")
        .env("ADVENT_MAIL__FROM", "advent@acme.com")
        .env("ADVENT_MAIL__SMTP__HOST", "127.0.0.1")
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("public_url"), "{stderr}");
}
