//! A day folder on disk: locating it in its event, loading its plaintext,
//! and packing that plaintext (plus the day key) into the source archive
//! that `source.age` encrypts.

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use advent_core::model::{DayMeta, Event};
use advent_core::primitives::{KEY_LEN, Key};
use anyhow::{Context, bail};

pub const SEALED_FILE: &str = "sealed.json";
pub const SOURCE_FILE: &str = "source.age";
const META_FILE: &str = "meta.yaml";
const PUZZLE_FILE: &str = "puzzle.md";
const EVENT_FILE: &str = "event.yaml";

/// Archive entries holding the day key and hint keys, next to `files/...`.
const DAY_KEY_ENTRY: &str = "day_key";
const HINT_KEY_PREFIX: &str = "hint_key/";
const FILES_PREFIX: &str = "files/";

pub struct DayDir {
    pub path: PathBuf,
    pub number: u32,
    pub event: Event,
}

/// Resolve `events/<id>/dayNN` and check `NN` fits the event.
pub fn locate(day_dir: &Path) -> anyhow::Result<DayDir> {
    let path = day_dir
        .canonicalize()
        .with_context(|| format!("no such day folder: {}", day_dir.display()))?;
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let number: u32 = name
        .strip_prefix("day")
        .and_then(|n| n.parse().ok())
        .with_context(|| format!("{name:?} is not a day folder: expected a name like day05"))?;
    let event_dir = path
        .parent()
        .context("day folder has no parent")?
        .to_path_buf();
    let event_file = event_dir.join(EVENT_FILE);
    let event = Event::from_yaml(
        &fs::read_to_string(&event_file)
            .with_context(|| format!("reading {}", event_file.display()))?,
    )
    .with_context(|| format!("in {}", event_file.display()))?;
    if number == 0 || number > event.days {
        bail!(
            "day {number} is outside this event, which has {} days",
            event.days
        );
    }
    Ok(DayDir {
        path,
        number,
        event,
    })
}

/// The plaintext of a day: its meta, its puzzle, and every file in the folder.
pub struct Plaintext {
    pub meta: DayMeta,
    pub puzzle: String,
    /// Every plaintext file by path relative to the day folder.
    pub files: BTreeMap<String, Vec<u8>>,
}

pub fn load_plaintext(dir: &Path) -> anyhow::Result<Plaintext> {
    let mut files = BTreeMap::new();
    collect(dir, dir, &mut files)?;
    Plaintext::from_files(files).with_context(|| format!("in {}", dir.display()))
}

impl Plaintext {
    /// Build from a day's files, whether read from disk or from `source.age`.
    pub fn from_files(files: BTreeMap<String, Vec<u8>>) -> anyhow::Result<Plaintext> {
        let meta_text = files
            .get(META_FILE)
            .with_context(|| format!("no {META_FILE}"))?;
        let meta = DayMeta::from_yaml(std::str::from_utf8(meta_text)?)
            .with_context(|| format!("in {META_FILE}"))?;
        let puzzle = String::from_utf8(
            files
                .get(PUZZLE_FILE)
                .with_context(|| format!("no {PUZZLE_FILE}"))?
                .clone(),
        )?;
        Ok(Plaintext {
            meta,
            puzzle,
            files,
        })
    }
}

fn collect(root: &Path, dir: &Path, files: &mut BTreeMap<String, Vec<u8>>) -> anyhow::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let relative = path
            .strip_prefix(root)?
            .to_string_lossy()
            .replace('\\', "/");
        if path.is_dir() {
            collect(root, &path, files)?;
        } else if relative != SEALED_FILE && relative != SOURCE_FILE && !relative.ends_with(".tmp")
        {
            files.insert(relative, fs::read(&path)?);
        }
    }
    Ok(())
}

/// The keys a day was sealed with, kept in its source copy so an editor can
/// re-wrap them for new release recipients and verify the day without the
/// release key.
pub struct DayKeys {
    pub day_key: Key,
    pub hint_keys: Vec<Key>,
}

/// Tar the plaintext files and the day's keys together. Original files are
/// kept as they were (not the converted WebP), so restoring is exact.
pub fn pack_source(
    files: &BTreeMap<String, Vec<u8>>,
    keys: Option<&DayKeys>,
) -> anyhow::Result<Vec<u8>> {
    let mut builder = tar::Builder::new(Vec::new());
    let mut append = |name: &str, data: &[u8]| -> anyhow::Result<()> {
        let mut header = tar::Header::new_gnu();
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder.append_data(&mut header, name, data)?;
        Ok(())
    };
    if let Some(keys) = keys {
        append(DAY_KEY_ENTRY, &keys.day_key)?;
        for (i, key) in keys.hint_keys.iter().enumerate() {
            append(&format!("{HINT_KEY_PREFIX}{i}"), key)?;
        }
    }
    for (name, data) in files {
        append(&format!("{FILES_PREFIX}{name}"), data)?;
    }
    Ok(builder.into_inner()?)
}

pub struct Source {
    /// Present for a day's source; a final payload's source has no keys.
    pub keys: Option<DayKeys>,
    pub files: BTreeMap<String, Vec<u8>>,
}

impl Source {
    pub fn day_keys(&self) -> anyhow::Result<&DayKeys> {
        self.keys.as_ref().context("source archive has no day key")
    }
}

fn key_from(name: &str, data: Vec<u8>) -> anyhow::Result<Key> {
    let len = data.len();
    <[u8; KEY_LEN]>::try_from(data).map_err(|_| anyhow::anyhow!("{name} in source has {len} bytes"))
}

pub fn unpack_source(archive: &[u8]) -> anyhow::Result<Source> {
    let mut day_key = None;
    let mut hint_keys = BTreeMap::new();
    let mut files = BTreeMap::new();
    for entry in tar::Archive::new(archive).entries()? {
        let mut entry = entry?;
        let name = entry.path()?.to_string_lossy().into_owned();
        let mut data = Vec::new();
        entry.read_to_end(&mut data)?;
        if name == DAY_KEY_ENTRY {
            day_key = Some(key_from(&name, data)?);
        } else if let Some(index) = name.strip_prefix(HINT_KEY_PREFIX) {
            let index: usize = index
                .parse()
                .with_context(|| format!("bad entry {name:?}"))?;
            hint_keys.insert(index, key_from(&name, data)?);
        } else if let Some(file) = name.strip_prefix(FILES_PREFIX) {
            if file.split('/').any(|part| part == ".." || part.is_empty()) {
                bail!("unsafe path {file:?} in source archive");
            }
            files.insert(file.to_string(), data);
        }
    }
    let hint_keys: Vec<Key> = hint_keys.into_values().collect();
    Ok(Source {
        keys: day_key.map(|day_key| DayKeys { day_key, hint_keys }),
        files,
    })
}

/// Write via a temp file and rename, so an interrupted run never leaves a
/// half-written sealed file behind.
pub fn write_atomic(path: &Path, data: &[u8]) -> anyhow::Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, data).with_context(|| format!("writing {}", tmp.display()))?;
    fs::rename(&tmp, path).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_source_archive_round_trips() {
        let files = BTreeMap::from([
            ("puzzle.md".to_string(), b"# Door".to_vec()),
            ("img/map.png".to_string(), vec![0, 1, 2, 255]),
        ]);
        let keys = DayKeys {
            day_key: [9u8; KEY_LEN],
            hint_keys: vec![[1u8; KEY_LEN], [2u8; KEY_LEN]],
        };
        let source = unpack_source(&pack_source(&files, Some(&keys)).unwrap()).unwrap();
        let unpacked = source.day_keys().unwrap();
        assert_eq!(unpacked.day_key, keys.day_key);
        assert_eq!(unpacked.hint_keys, keys.hint_keys);
        assert_eq!(source.files, files);

        let without_keys = unpack_source(&pack_source(&files, None).unwrap()).unwrap();
        assert!(without_keys.keys.is_none());
    }
}
