//! The frontend, embedded from `web/dist` at compile time, so an event
//! repo's CI needs only the pinned `advent` binary to build its site.

#[cfg(embedded_web)]
use std::fs;
use std::path::Path;

#[cfg(embedded_web)]
static WEB: include_dir::Dir<'_> = include_dir::include_dir!("$CARGO_MANIFEST_DIR/../../web/dist");

/// Write the frontend at the site root, plus a copy of `index.html` in each
/// event folder so `/<event>/` works on any static host without rewrites.
#[cfg(embedded_web)]
pub fn write(out: &Path, event_ids: &[String]) -> anyhow::Result<()> {
    WEB.extract(out)?;
    let index = WEB
        .get_file("index.html")
        .ok_or_else(|| anyhow::anyhow!("embedded frontend has no index.html"))?;
    for id in event_ids {
        let dir = out.join(id);
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("index.html"), index.contents())?;
    }
    Ok(())
}

#[cfg(not(embedded_web))]
pub fn write(_out: &Path, _event_ids: &[String]) -> anyhow::Result<()> {
    anyhow::bail!(
        "this advent binary was compiled without the frontend; run `make frontend-build` \
         and rebuild it (release binaries always include it)"
    )
}
