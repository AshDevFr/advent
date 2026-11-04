//! The binary carries the frontend: `advent build` writes it out unchanged,
//! but for each page's title and link preview tags.
//! Requires `web/dist` (built by `make frontend-build`, which `make ci` runs
//! before the Rust tests).

mod common;

use std::fs;
use std::path::PathBuf;

use common::sample_event;

/// A page without its `<title>` and the preview tags that follow it, which
/// the build writes for each page.
fn without_preview(page: &str) -> String {
    let start = page.find("<title>").unwrap();
    let last = page
        .find(r#"name="twitter:card""#)
        .unwrap_or_else(|| page.find("</title>").unwrap());
    let end = last + page[last..].find('>').unwrap() + 1;
    format!("{}{}", &page[..start], &page[end..])
}

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

    let index = without_preview(&fs::read_to_string(web_dist().join("index.html")).unwrap());
    for page in ["index.html", "2026/index.html"] {
        let built = fs::read_to_string(out.join(page)).unwrap();
        assert!(
            built.contains(r#"name="twitter:card""#),
            "{page} has preview tags"
        );
        assert_eq!(without_preview(&built), index, "{page}");
    }

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
