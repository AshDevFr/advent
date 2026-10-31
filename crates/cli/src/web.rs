//! The frontend, embedded from `web/dist` at compile time, so an event
//! repo's CI needs only the pinned `advent` binary to build its site.

#[cfg(embedded_web)]
use std::fs;
use std::path::Path;

#[cfg(embedded_web)]
static WEB: include_dir::Dir<'_> = include_dir::include_dir!("$CARGO_MANIFEST_DIR/../../web/dist");

/// The `<base href>` the frontend ships with; `advent build` replaces it.
#[cfg(embedded_web)]
const DEFAULT_BASE: &str = r#"<base href="/" />"#;

/// Write the frontend at the site root, plus a copy of `index.html` in each
/// event folder so `<base>/<event>/` works on any static host without
/// rewrites. Every `index.html` gets `<base href="{base}">`: the frontend
/// uses relative URLs only, so the browser resolves them against it.
#[cfg(embedded_web)]
pub fn write(out: &Path, event_ids: &[String], base: &str) -> anyhow::Result<()> {
    WEB.extract(out)?;
    let index = WEB
        .get_file("index.html")
        .and_then(|f| f.contents_utf8())
        .ok_or_else(|| anyhow::anyhow!("embedded frontend has no index.html"))?;
    if !index.contains(DEFAULT_BASE) {
        anyhow::bail!("embedded index.html has no {DEFAULT_BASE} to set the base path in");
    }
    let index = index.replacen(DEFAULT_BASE, &format!(r#"<base href="{base}" />"#), 1);
    fs::write(out.join("index.html"), &index)?;
    for id in event_ids {
        let dir = out.join(id);
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("index.html"), &index)?;
    }
    Ok(())
}

#[cfg(not(embedded_web))]
pub fn write(_out: &Path, _event_ids: &[String], _base: &str) -> anyhow::Result<()> {
    anyhow::bail!(
        "this advent binary was compiled without the frontend; run `make frontend-build` \
         and rebuild it (release binaries always include it)"
    )
}
