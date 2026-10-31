//! `advent init` end to end, running the real binary in temp directories.

use std::fs;
use std::path::Path;
use std::process::Command as Std;

use advent_core::model::{Brand, Event, RepoConfig};
use advent_core::wrap;
use assert_cmd::Command;

fn advent() -> Command {
    Command::cargo_bin("advent").unwrap()
}

fn init(dir: &Path) -> String {
    let out = advent()
        .args(["init", dir.to_str().unwrap(), "--event", "2026"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn init_writes_a_complete_repo() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("calendar");
    init(&repo);

    for file in [
        "advent.yaml",
        "brand/brand.yaml",
        "brand/logo.svg",
        "events/2026/event.yaml",
        ".gitignore",
        "README.md",
        "editors.key",
    ] {
        assert!(repo.join(file).is_file(), "missing {file}");
    }

    let config =
        RepoConfig::from_yaml(&fs::read_to_string(repo.join("advent.yaml")).unwrap()).unwrap();
    assert_eq!(config.engine, env!("CARGO_PKG_VERSION"));
    assert_eq!(config.keys.editors.len(), 1);
    assert_eq!(config.keys.release.len(), 1);
    assert!(config.keys.editors[0].starts_with("age1"));
    assert!(config.keys.release[0].starts_with("age1"));

    let event = Event::from_yaml(&fs::read_to_string(repo.join("events/2026/event.yaml")).unwrap())
        .unwrap();
    assert_eq!(event.start.to_string(), "2026-12-01");
    assert_eq!(event.title, "Advent 2026");
    Brand::from_yaml(&fs::read_to_string(repo.join("brand/brand.yaml")).unwrap()).unwrap();

    let editors =
        wrap::parse_identities(&fs::read_to_string(repo.join("editors.key")).unwrap()).unwrap();
    assert_eq!(editors[0].to_public().to_string(), config.keys.editors[0]);
}

#[test]
fn init_prints_the_release_secret_once_and_never_stores_it() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("calendar");
    let stdout = init(&repo);

    let secret = stdout
        .lines()
        .find(|l| l.starts_with("AGE-SECRET-KEY-1"))
        .expect("release secret printed");
    let identity = wrap::parse_identities(secret).unwrap().remove(0);
    let config =
        RepoConfig::from_yaml(&fs::read_to_string(repo.join("advent.yaml")).unwrap()).unwrap();
    assert_eq!(identity.to_public().to_string(), config.keys.release[0]);

    for entry in walk(&repo) {
        let content = fs::read(&entry).unwrap();
        let text = String::from_utf8_lossy(&content);
        assert!(
            !text.contains(secret),
            "release secret found in {}",
            entry.display()
        );
    }
}

#[test]
fn init_gitignore_keeps_plaintext_out_and_sealed_files_in() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("calendar");
    init(&repo);
    assert!(
        Std::new("git")
            .args(["init", "-q"])
            .current_dir(&repo)
            .status()
            .unwrap()
            .success()
    );

    let ignored = |path: &str| {
        Std::new("git")
            .args(["check-ignore", "-q", "--no-index", path])
            .current_dir(&repo)
            .status()
            .unwrap()
            .success()
    };
    assert!(ignored("events/2026/day01/puzzle.md"));
    assert!(ignored("events/2026/day01/meta.yaml"));
    assert!(ignored("events/2026/day01/map.png"));
    assert!(ignored("events/2026/final/final.yaml"));
    assert!(ignored("editors.key"));
    assert!(ignored(".preview/2026/api/day/1.json"));
    assert!(ignored("advent"));
    assert!(!ignored("brand/advent.svg"));
    assert!(!ignored("events/2026/day01/sealed.json"));
    assert!(!ignored("events/2026/day01/source.age"));
    assert!(!ignored("events/2026/final/sealed.json"));
}

#[test]
fn init_static_edition_deploys_to_pages_without_docker_files() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("calendar");
    init(&repo);
    assert!(repo.join(".github/workflows/build.yml").is_file());
    for file in ["Dockerfile", ".dockerignore", "docker-compose.example.yml"] {
        assert!(
            !repo.join(file).exists(),
            "{file} belongs to the company edition"
        );
    }
    let readme = fs::read_to_string(repo.join("README.md")).unwrap();
    assert!(readme.contains("## Getting started"));
    assert!(readme.contains(&format!(
        "releases/download/v{}/",
        env!("CARGO_PKG_VERSION")
    )));
    assert!(
        readme.contains("events/2026/day01"),
        "the event id is filled in"
    );
    assert!(readme.contains(&format!(
        "ghcr.io/ashdevfr/advent:{}",
        env!("CARGO_PKG_VERSION")
    )));
}

#[test]
fn init_company_edition_builds_an_image_named_after_the_git_remote() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("calendar");
    // A freshly cloned, empty repository: only .git is there.
    fs::create_dir_all(&repo).unwrap();
    let git = |args: &[&str]| {
        assert!(
            Std::new("git")
                .args(args)
                .current_dir(&repo)
                .status()
                .unwrap()
                .success()
        )
    };
    git(&["init", "-q"]);
    git(&[
        "remote",
        "add",
        "origin",
        "git@github.com:Acme/Advent-Calendar.git",
    ]);
    let out = advent()
        .args(["init", repo.to_str().unwrap(), "--event", "2026"])
        .args(["--edition", "company"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    for file in [
        "Dockerfile",
        ".dockerignore",
        ".github/workflows/image.yml",
        "advent.yaml",
        "events/2026/event.yaml",
    ] {
        assert!(repo.join(file).is_file(), "missing {file}");
    }
    assert!(!repo.join(".github/workflows/build.yml").exists());
    let compose = fs::read_to_string(repo.join("docker-compose.example.yml")).unwrap();
    assert!(
        compose.contains("image: ghcr.io/acme/advent-calendar:latest"),
        "{compose}"
    );
    let readme = fs::read_to_string(repo.join("README.md")).unwrap();
    assert!(readme.contains("advent serve"));
}

#[test]
fn init_company_edition_marks_the_image_to_fill_in_without_a_remote() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("calendar");
    advent()
        .args(["init", repo.to_str().unwrap(), "--edition", "company"])
        .assert()
        .success();
    let compose = fs::read_to_string(repo.join("docker-compose.example.yml")).unwrap();
    assert!(
        compose.contains("image: ghcr.io/OWNER/REPO:latest"),
        "{compose}"
    );
}

#[test]
fn init_refuses_a_non_empty_directory() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("existing.txt"), "keep me").unwrap();
    advent()
        .args(["init", tmp.path().to_str().unwrap()])
        .assert()
        .failure();
    let entries: Vec<_> = fs::read_dir(tmp.path()).unwrap().collect();
    assert_eq!(entries.len(), 1, "init must write nothing");
}

fn walk(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(walk(&path));
        } else {
            files.push(path);
        }
    }
    files
}
