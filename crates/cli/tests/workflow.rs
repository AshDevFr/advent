//! The event repo's build-and-deploy workflow, shipped in the template.

mod common;

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use common::Repo;

fn workflow_text() -> String {
    fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../templates/event/.github/workflows/build.yml"),
    )
    .unwrap()
}

fn workflow() -> serde_yaml::Value {
    serde_yaml::from_str(&workflow_text()).unwrap()
}

/// Every `${{ <kind>.NAME }}` reference in the workflow.
fn references(kind: &str) -> BTreeSet<String> {
    let text = workflow_text();
    let marker = format!("{kind}.");
    text.match_indices(&marker)
        .map(|(i, _)| {
            text[i + marker.len()..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect()
        })
        .collect()
}

#[test]
fn workflow_runs_on_schedule_push_and_demand() {
    let on = &workflow()["on"];
    assert!(!on["schedule"].as_sequence().unwrap().is_empty());
    assert_eq!(on["push"]["branches"][0], "main");
    assert!(on.get("workflow_dispatch").is_some());
}

#[test]
fn workflow_verifies_builds_then_deploys() {
    let steps = workflow()["jobs"]["deploy"]["steps"]
        .as_sequence()
        .unwrap()
        .clone();
    let text: Vec<String> = steps
        .iter()
        .map(|s| {
            format!(
                "{} {} {}",
                s["run"].as_str().unwrap_or_default(),
                s["uses"].as_str().unwrap_or_default(),
                s["with"]["command"].as_str().unwrap_or_default()
            )
        })
        .collect();
    let position = |needle: &str| {
        text.iter()
            .position(|t| t.contains(needle))
            .unwrap_or_else(|| panic!("no step with {needle:?}"))
    };
    let download = position("releases/download/v${version}");
    let verify = position("advent verify --release --released");
    let build = position("advent build");
    let create = position("pages project create");
    let deploy = position("pages deploy dist");
    assert!(download < verify && verify < build && build < create && create < deploy);
}

#[test]
fn workflow_uses_only_the_expected_secrets() {
    assert_eq!(
        references("secrets"),
        BTreeSet::from([
            "ADVENT_RELEASE_KEY".to_string(),
            "CLOUDFLARE_ACCOUNT_ID".to_string(),
            "CLOUDFLARE_API_TOKEN".to_string(),
        ])
    );
    assert!(references("vars").contains("CLOUDFLARE_PAGES_PROJECT"));
}

#[test]
fn workflow_is_written_by_init() {
    let repo = Repo::init();
    let written = repo.root.join(".github/workflows/build.yml");
    assert_eq!(fs::read_to_string(written).unwrap(), workflow_text());
}
