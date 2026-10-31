//! `advent preview`: the calendar as players will see it, on this machine,
//! for authors and the keeper. Days are opened with the editors key (each
//! source copy holds its day's keys), so no release key is needed. The site
//! is written to a temporary folder outside the repository and served on
//! 127.0.0.1 by default: a preview can show days that are not released yet,
//! so it must never be deployable, and is reachable from elsewhere only when
//! asked with `--host` (inside a container, where 127.0.0.1 is the
//! container's own and unreachable from the host).

use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{IpAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};

use advent_core::model::DayMeta;
use advent_core::wrap::{self, x25519};
use anyhow::Context;
use chrono::NaiveDate;

use crate::commands::build::{self, Moment, Opener, Options};
use crate::commands::reseal;
use crate::config::{editors_identities, find_repo_root, load_repo_config};
use crate::day::{self, SOURCE_FILE};

pub struct Args {
    pub until: Option<NaiveDate>,
    pub all_open: bool,
    pub solved: bool,
    pub port: u16,
    pub host: IpAddr,
    pub out: Option<PathBuf>,
}

pub fn run(args: Args) -> anyhow::Result<()> {
    let root = find_repo_root(Path::new("."))?;
    load_repo_config(&root)?;
    let editors =
        editors_identities(&root).context("advent preview opens days with the editors key")?;

    let head_html = if args.solved {
        Some(solved_script(&root, &editors)?)
    } else {
        None
    };
    let options = Options {
        moment: match (args.all_open, args.until) {
            (true, _) => Moment::AllOpen,
            (false, Some(date)) => Moment::Until(date),
            (false, None) => Moment::Now,
        },
        base: "/",
        opener: Opener::Editors(editors),
        skip_unsealed: true,
        head_html,
        server: false,
    };

    if let Some(out) = &args.out {
        build::build_site(&root, out, &options)?;
        println!("Preview written to {}.", out.display());
        return Ok(());
    }

    let dir = tempfile::Builder::new()
        .prefix("advent-preview-")
        .tempdir()?;
    let site = dir.path().join("site");
    build::build_site(&root, &site, &options)?;

    // The preview may hold unreleased days: remove it on Ctrl-C too.
    let cleanup = dir.path().to_path_buf();
    ctrlc::set_handler(move || {
        let _ = fs::remove_dir_all(&cleanup);
        std::process::exit(0);
    })?;

    let listener = TcpListener::bind((args.host, args.port))
        .with_context(|| format!("port {} is busy; pick another with --port", args.port))?;
    let addr = listener.local_addr()?;
    if args.host.is_loopback() {
        println!("Preview at http://{addr}/ (this machine only). Ctrl-C to stop.");
    } else {
        println!(
            "Preview at http://{addr}/, reachable from other machines and showing days not released yet. Ctrl-C to stop."
        );
    }
    for stream in listener.incoming().flatten() {
        let _ = handle(stream, &site);
    }
    Ok(())
}

/// A script that stores every sealed day's word in the player's progress,
/// so the certificate can be unlocked straight away in the preview.
fn solved_script(root: &Path, editors: &[x25519::Identity]) -> anyhow::Result<String> {
    let mut words: BTreeMap<String, BTreeMap<u32, String>> = BTreeMap::new();
    for event_dir in reseal::event_dirs(root, None)? {
        let id = event_dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let event_words = words.entry(id).or_default();
        for day_dir in reseal::sealed_days(&event_dir)? {
            let number: u32 = day_dir
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_prefix("day"))
                .and_then(|n| n.parse().ok())
                .context("unexpected day folder name")?;
            let source = day::unpack_source(&wrap::decrypt_with(
                &fs::read(day_dir.join(SOURCE_FILE))?,
                editors,
            )?)?;
            let meta = source
                .files
                .get("meta.yaml")
                .context("source has no meta.yaml")?;
            let meta = DayMeta::from_yaml(std::str::from_utf8(meta)?)?;
            event_words.insert(number, meta.word);
        }
    }
    let json = serde_json::to_string(&words)?;
    Ok(format!(
        "<script>/* advent preview --solved */(()=>{{try{{const W={json};const k=\"advent-progress\";\
const s=JSON.parse(localStorage.getItem(k)||'{{\"state\":{{\"events\":{{}}}},\"version\":1}}');\
for(const[id,words]of Object.entries(W)){{const e=s.state.events[id]||{{words:{{}},hintsUsed:{{}},attempts:{{}},name:\"\"}};\
e.words={{...e.words,...words}};s.state.events[id]=e;}}localStorage.setItem(k,JSON.stringify(s));}}catch{{}}}})()</script>"
    ))
}

fn handle(stream: TcpStream, site: &Path) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    // Drain the headers; the preview needs none of them.
    let mut line = String::new();
    while reader.read_line(&mut line)? > 2 {
        line.clear();
    }
    let path = request_line.split_whitespace().nth(1).unwrap_or("/");
    let response = respond(site, path);
    let mut stream = stream;
    write!(
        stream,
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        response.status,
        if response.status == 200 {
            "OK"
        } else {
            "Not Found"
        },
        response.content_type,
        response.body.len()
    )?;
    stream.write_all(&response.body)
}

#[derive(Debug, PartialEq)]
pub(crate) struct Response {
    pub status: u16,
    pub content_type: &'static str,
    pub body: Vec<u8>,
}

fn not_found() -> Response {
    Response {
        status: 404,
        content_type: "text/plain; charset=utf-8",
        body: b"not found".to_vec(),
    }
}

/// The file for a request path: `/` and folders map to their
/// `index.html`, and nothing outside `site` can be reached.
pub(crate) fn respond(site: &Path, target: &str) -> Response {
    let path = target.split(['?', '#']).next().unwrap_or("/");
    let Some(decoded) = percent_decode(path) else {
        return not_found();
    };
    let mut file = site.to_path_buf();
    for part in decoded.split('/').filter(|p| !p.is_empty()) {
        if part == ".." || part == "." || part.contains('\\') {
            return not_found();
        }
        file.push(part);
    }
    if file.is_dir() {
        file.push("index.html");
    }
    match fs::read(&file) {
        Ok(body) => Response {
            status: 200,
            content_type: content_type(&file),
            body,
        },
        Err(_) => not_found(),
    }
}

fn content_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
    {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "woff2" => "font/woff2",
        "ico" => "image/x-icon",
        _ => "application/octet-stream",
    }
}

fn percent_decode(path: &str) -> Option<String> {
    let bytes = path.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = std::str::from_utf8(bytes.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("index.html"), "<html>root</html>").unwrap();
        fs::create_dir_all(dir.path().join("2026/api/day")).unwrap();
        fs::write(dir.path().join("2026/index.html"), "<html>event</html>").unwrap();
        fs::write(dir.path().join("2026/api/day/1.json"), "{}").unwrap();
        fs::write(dir.path().join("my image.webp"), "RIFF").unwrap();
        dir
    }

    #[test]
    fn preview_serves_index_pages_for_folders() {
        let dir = site();
        let root = respond(dir.path(), "/");
        assert_eq!(
            (root.status, root.body.as_slice()),
            (200, &b"<html>root</html>"[..])
        );
        assert_eq!(root.content_type, "text/html; charset=utf-8");
        let event = respond(dir.path(), "/2026/?x=1");
        assert_eq!(event.body, b"<html>event</html>");
    }

    #[test]
    fn preview_serves_files_with_their_type() {
        let dir = site();
        assert_eq!(
            respond(dir.path(), "/2026/api/day/1.json").content_type,
            "application/json"
        );
        let image = respond(dir.path(), "/my%20image.webp");
        assert_eq!((image.status, image.content_type), (200, "image/webp"));
    }

    #[test]
    fn preview_refuses_paths_outside_the_site() {
        let dir = site();
        for path in [
            "/../secret",
            "/2026/../../etc/passwd",
            "/%2e%2e/secret",
            "/a\\b",
            "/%zz",
        ] {
            assert_eq!(respond(dir.path(), path).status, 404, "{path}");
        }
        assert_eq!(respond(dir.path(), "/missing.json").status, 404);
    }
}
