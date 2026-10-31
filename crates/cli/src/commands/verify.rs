//! `advent verify [event] [--release]`: is this event repo fit to deploy?
//!
//! Three depths, depending on the keys at hand:
//! - always: every day sealed, nothing extra, files readable in this
//!   engine's schema, the certificate sealed, no plaintext tracked by git;
//! - with the editors key: every day's sealed output matches its source
//!   (box, hints, word locks) and the certificate opens with the words;
//! - with `--release`: every wrapped key opens with the release key. This
//!   runs in CI logs, so its output names broken days and never content.
//!
//! With `--released`, completeness is only required for what is out: days
//! not yet released may still be unsealed, and the certificate is only
//! required once the last day is released. That is the deploy gate an event
//! repo's CI uses in November, while authors are still writing.

use std::fs;
use std::path::Path;
use std::process::Command;

use advent_core::format::{self, Bundle, FormatError, SealedDay};
use advent_core::model::Event;
use advent_core::primitives;
use advent_core::schedule::is_released;
use advent_core::wrap::{self, x25519};
use anyhow::{Context, bail};
use chrono::{DateTime, Utc};

use crate::commands::{keeper, reseal};
use crate::config::{EDITORS_KEY_FILE, editors_identities, find_repo_root, release_identities};
use crate::day::{self, Plaintext, SEALED_FILE, SOURCE_FILE};

pub fn run(event: Option<&str>, release: bool, released_only: bool) -> anyhow::Result<()> {
    let root = find_repo_root(Path::new("."))?;
    let editors = editors_key(&root)?;
    let release_ids = if release {
        Some(release_identities()?)
    } else {
        None
    };

    let mut problems = Vec::new();
    tracked_plaintext(&root, &mut problems);
    for event_dir in reseal::event_dirs(&root, event)? {
        let id = event_dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let mut report = |p: String| problems.push(format!("{id}: {p}"));
        let now = released_only.then(Utc::now);
        check_event(
            &event_dir,
            editors.as_deref(),
            release_ids.as_deref(),
            now,
            &mut report,
        )?;
    }

    if problems.is_empty() {
        println!("OK");
        return Ok(());
    }
    for problem in &problems {
        eprintln!("error: {problem}");
    }
    bail!("verify found {} problem(s)", problems.len())
}

/// The editors key if one is configured; verify degrades without it.
fn editors_key(root: &Path) -> anyhow::Result<Option<Vec<x25519::Identity>>> {
    let configured =
        std::env::var_os("ADVENT_EDITORS_KEY").is_some() || root.join(EDITORS_KEY_FILE).is_file();
    if !configured {
        println!(
            "No editors key: skipping the checks that compare sealed days with their sources."
        );
        return Ok(None);
    }
    editors_identities(root).map(Some)
}

fn check_event(
    event_dir: &Path,
    editors: Option<&[x25519::Identity]>,
    release: Option<&[x25519::Identity]>,
    released_as_of: Option<DateTime<Utc>>,
    report: &mut dyn FnMut(String),
) -> anyhow::Result<()> {
    let event_file = event_dir.join("event.yaml");
    let event = match fs::read_to_string(&event_file)
        .map_err(anyhow::Error::from)
        .and_then(|t| Event::from_yaml(&t).map_err(Into::into))
    {
        Ok(event) => event,
        Err(e) => {
            report(format!("event.yaml: {e}"));
            return Ok(());
        }
    };

    for extra in day_numbers(event_dir)?
        .into_iter()
        .filter(|n| *n == 0 || *n > event.days)
    {
        report(format!(
            "day{extra:02} is outside this {}-day event",
            event.days
        ));
    }

    let mut words = Some(Vec::new());
    for n in 1..=event.days {
        let dir = event_dir.join(format!("day{n:02}"));
        if !dir.join(SEALED_FILE).is_file() || !dir.join(SOURCE_FILE).is_file() {
            let pending = released_as_of.is_some_and(|now| !is_released(&event, n, now));
            if !pending {
                report(format!("day {n} is not sealed"));
            }
            words = None;
            continue;
        }
        let sealed = match format::read_sealed_day(&fs::read(dir.join(SEALED_FILE))?) {
            Ok(sealed) => sealed,
            Err(e @ FormatError::UnsupportedSchema { .. }) => {
                report(format!("day {n}: {e}"));
                words = None;
                continue;
            }
            Err(e) => {
                report(format!("day {n}: unreadable {SEALED_FILE}: {e}"));
                words = None;
                continue;
            }
        };
        if sealed.day != n {
            report(format!("day {n}: {SEALED_FILE} is for day {}", sealed.day));
        }
        if let Some(release) = release
            && !opens_with_release(&sealed, release)
        {
            report(format!("day {n}: does not open with the release key"));
        }
        if let Some(editors) = editors {
            match check_against_source(&dir, &sealed, editors) {
                Ok(word) => {
                    if let Some(w) = words.as_mut() {
                        w.push(word);
                    }
                }
                Err(e) => {
                    report(format!("day {n}: {e:#}"));
                    words = None;
                }
            }
        }
    }

    let final_sealed = event_dir.join(keeper::FINAL_DIR).join(SEALED_FILE);
    match fs::read(&final_sealed) {
        Err(_) => {
            let all_out = released_as_of.is_none_or(|now| is_released(&event, event.days, now));
            if all_out {
                report("the certificate is not sealed (run `advent keeper seal`)".into());
            }
        }
        Ok(bytes) => match format::read_sealed_final(&bytes) {
            Err(e) => report(format!("certificate: {e}")),
            Ok(sealed) => {
                if let (Some(_), Some(words)) = (editors, words) {
                    let words: Vec<&str> = words.iter().map(String::as_str).collect();
                    let opened = primitives::open_final(&words, sealed.salt, sealed.iv, &sealed.ct);
                    if let Ok(plain) = &opened {
                        check_rewards(event_dir, plain, report);
                    }
                    if opened.is_err() {
                        report(
                            "the certificate is stale: it does not open with the current words \
                             (run `advent keeper seal` again)"
                                .into(),
                        );
                    }
                }
            }
        },
    }
    Ok(())
}

/// Every reward file the certificate lists is present and opens with its key.
fn check_rewards(event_dir: &Path, certificate_json: &[u8], report: &mut dyn FnMut(String)) {
    let Ok(certificate) = serde_json::from_slice::<format::Certificate>(certificate_json) else {
        report("the certificate payload is not readable".into());
        return;
    };
    for file in certificate.rewards.iter().filter_map(|r| r.file.as_ref()) {
        let path = event_dir
            .join(keeper::FINAL_DIR)
            .join(keeper::reward_file_name(file.index));
        match fs::read(&path) {
            Err(_) => report(format!(
                "reward file {} ({}) is missing",
                path.display(),
                file.name
            )),
            Ok(ct) => {
                if primitives::open_bundle(&file.key, file.iv, &ct).is_err() {
                    report(format!(
                        "reward file {} does not open with its key",
                        file.name
                    ));
                }
            }
        }
    }
}

/// Numbers of every `dayNN` folder present, sealed or not.
fn day_numbers(event_dir: &Path) -> anyhow::Result<Vec<u32>> {
    Ok(fs::read_dir(event_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().to_str()?.strip_prefix("day")?.parse().ok())
        .collect())
}

fn opens_with_release(sealed: &SealedDay, release: &[x25519::Identity]) -> bool {
    let Ok(day_key) = wrap::unwrap_key(&sealed.day_key, release) else {
        return false;
    };
    primitives::open_bundle(&day_key, sealed.bundle.iv, &sealed.bundle.ct).is_ok()
        && sealed.hints.iter().all(|hint| {
            wrap::unwrap_key(&hint.key, release)
                .is_ok_and(|key| primitives::open_hint(&key, hint.iv, &hint.ct).is_ok())
        })
}

/// Open the day with the keys in its source copy and compare with the
/// source's plaintext. Returns the day's word for the certificate check.
fn check_against_source(
    dir: &Path,
    sealed: &SealedDay,
    editors: &[x25519::Identity],
) -> anyhow::Result<String> {
    let archive = wrap::decrypt_with(&fs::read(dir.join(SOURCE_FILE))?, editors)
        .context("source.age does not open with the editors key")?;
    let source = day::unpack_source(&archive)?;
    let keys = source.day_keys()?;
    let plain = Plaintext::from_files(source.files.clone())?;

    let bundle: Bundle = serde_json::from_slice(
        &primitives::open_bundle(&keys.day_key, sealed.bundle.iv, &sealed.bundle.ct)
            .context("the box does not open with the day key from the source")?,
    )?;
    if bundle.title != plain.meta.title || bundle.kind != plain.meta.kind {
        bail!("the sealed box does not match the source (re-seal the day)");
    }
    if sealed.hints.len() != plain.meta.hints.len() || keys.hint_keys.len() != sealed.hints.len() {
        bail!("hint count differs between the sealed day and its source");
    }
    for ((hint, key), text) in sealed
        .hints
        .iter()
        .zip(&keys.hint_keys)
        .zip(&plain.meta.hints)
    {
        let opened =
            primitives::open_hint(key, hint.iv, &hint.ct).context("a hint does not open")?;
        if opened != text.text {
            bail!("a sealed hint does not match the source (re-seal the day)");
        }
    }
    for answer in &plain.meta.answers {
        let opens = sealed
            .word_locks
            .iter()
            .any(|lock| primitives::open_word(answer, lock).is_ok_and(|w| w == plain.meta.word));
        if !opens {
            bail!("an accepted answer does not open the word (re-seal the day)");
        }
    }
    Ok(plain.meta.word)
}

/// Plaintext content tracked by git outside `examples/`. `.gitignore` keeps
/// it out by default, but `git add -f` bypasses that.
fn tracked_plaintext(root: &Path, problems: &mut Vec<String>) {
    let output = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(root)
        .output();
    let files = match output {
        Ok(out) if out.status.success() => out.stdout,
        _ => {
            println!("Not a git repository: skipping the tracked-plaintext check.");
            return;
        }
    };
    for path in files.split(|b| *b == 0).filter(|p| !p.is_empty()) {
        let path = String::from_utf8_lossy(path);
        let parts: Vec<&str> = path.split('/').collect();
        let in_sealed_folder = parts.len() >= 4
            && parts[0] == "events"
            && (parts[2].starts_with("day") || parts[2] == keeper::FINAL_DIR);
        let allowed = parts
            .last()
            .is_some_and(|name| keeper::is_sealed_output(name));
        if in_sealed_folder && !allowed {
            problems.push(format!(
                "plaintext is tracked by git: {path} (remove it with `git rm --cached {path}`)"
            ));
        }
    }
}
