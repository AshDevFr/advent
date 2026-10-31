//! `advent build --base`: a calendar served under a sub-path.

mod common;

use std::fs;
use std::path::Path;

use common::sample_event;
use predicates::str::contains;

fn index_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            found.extend(index_files(&path));
        } else if path.file_name().is_some_and(|n| n == "index.html") {
            found.push(path);
        }
    }
    found
}

#[test]
fn base_path_is_written_into_every_index_html() {
    let repo = sample_event();
    for (base, out) in [("/demo/", "demo-out"), ("/", "root-out")] {
        repo.advent()
            .args([
                "build",
                "--until",
                "2026-12-03",
                "--out",
                out,
                "--base",
                base,
            ])
            .env("ADVENT_RELEASE_KEY", &repo.release_secret)
            .assert()
            .success();
        let pages = index_files(&repo.root.join(out));
        assert_eq!(pages.len(), 2, "site root and the event folder");
        for page in pages {
            let html = fs::read_to_string(&page).unwrap();
            assert!(
                html.contains(&format!(r#"<base href="{base}" />"#)),
                "{} lacks base {base}",
                page.display()
            );
            assert!(
                !html.contains(r#"src="/assets"#),
                "asset URLs must be relative"
            );
        }
    }
}

#[test]
fn base_path_must_start_and_end_with_a_slash() {
    let repo = sample_event();
    repo.advent()
        .args(["build", "--until", "2026-12-03", "--base", "demo"])
        .env("ADVENT_RELEASE_KEY", &repo.release_secret)
        .assert()
        .failure()
        .stderr(contains("such as / or /demo/"));
}

#[test]
fn base_path_site_data_uses_relative_urls() {
    let repo = sample_event();
    repo.advent()
        .args(["build", "--until", "2026-12-03", "--base", "/demo/"])
        .env("ADVENT_RELEASE_KEY", &repo.release_secret)
        .assert()
        .success();
    let out = repo.root.join("dist");
    for file in ["site.json", "2026/event.json"] {
        let value: serde_json::Value =
            serde_json::from_slice(&fs::read(out.join(file)).unwrap()).unwrap();
        let mut stack = vec![value];
        while let Some(v) = stack.pop() {
            match v {
                serde_json::Value::String(s) => {
                    assert!(!s.starts_with('/'), "{file} has an absolute path {s:?}")
                }
                serde_json::Value::Array(a) => stack.extend(a),
                serde_json::Value::Object(o) => stack.extend(o.into_values()),
                _ => {}
            }
        }
    }
}

#[test]
fn base_path_can_come_from_the_environment() {
    let repo = sample_event();
    repo.advent()
        .args(["build", "--until", "2026-12-03", "--out", "env-out"])
        .env("ADVENT_RELEASE_KEY", &repo.release_secret)
        .env("ADVENT_BASE", "/calendar/")
        .assert()
        .success();
    let html = fs::read_to_string(repo.root.join("env-out/index.html")).unwrap();
    assert!(html.contains(r#"<base href="/calendar/" />"#));
}
