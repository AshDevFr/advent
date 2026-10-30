//! The binary carries the frontend: `advent build` writes it out unchanged.
//! Requires `web/dist` (built by `make frontend-build`, which `make ci` runs
//! before the Rust tests).

mod common;

use std::fs;
use std::path::PathBuf;

use common::sample_event;

fn web_dist() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../web/dist")
}

#[test]
fn embed_build_writes_the_frontend_at_the_root_and_in_each_event() {
    assert!(
        web_dist().join("index.html").is_file(),
        "web/dist missing: run `make frontend-build` first"
    );
    let repo = sample_event();
    repo.advent()
        .args(["build", "--until", "2026-12-03"])
        .env("ADVENT_RELEASE_KEY", &repo.release_secret)
        .assert()
        .success();
    let out = repo.root.join("dist");

    let index = fs::read(web_dist().join("index.html")).unwrap();
    assert_eq!(fs::read(out.join("index.html")).unwrap(), index);
    assert_eq!(fs::read(out.join("2026/index.html")).unwrap(), index);

    for entry in fs::read_dir(web_dist().join("assets")).unwrap() {
        let name = entry.unwrap().file_name();
        let built = out.join("assets").join(&name);
        assert_eq!(
            fs::read(&built).unwrap(),
            fs::read(web_dist().join("assets").join(&name)).unwrap(),
            "{name:?}"
        );
    }
    assert!(
        out.join("site.json").is_file(),
        "site data written alongside"
    );
}
