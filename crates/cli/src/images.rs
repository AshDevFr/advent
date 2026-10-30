//! Day images become WebP no wider or taller than `MAX_DIMENSION`, and the
//! puzzle's markdown is rewritten to reference the converted names.

use std::collections::BTreeMap;
use std::io::Cursor;

use anyhow::{Context, bail};
use image::ImageFormat;
use image::codecs::webp::WebPEncoder;
use image::imageops::FilterType;

pub const MAX_DIMENSION: u32 = 1600;
/// Above this many bytes of images, a day is slow to load on a phone.
pub const DAY_IMAGE_BUDGET: usize = 1_500_000;

const RASTER: [&str; 5] = ["png", "jpg", "jpeg", "gif", "webp"];

fn extension(name: &str) -> Option<String> {
    name.rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase())
}

pub fn is_image(name: &str) -> bool {
    extension(name).is_some_and(|e| RASTER.contains(&e.as_str()) || e == "svg")
}

/// Converted assets by their new name, and the markdown pointing at them.
pub struct Converted {
    pub assets: BTreeMap<String, Vec<u8>>,
    pub markdown: String,
    pub total_bytes: usize,
}

pub fn convert(files: &BTreeMap<String, Vec<u8>>, markdown: &str) -> anyhow::Result<Converted> {
    let mut assets = BTreeMap::new();
    let mut markdown = markdown.to_string();
    for (name, data) in files.iter().filter(|(n, _)| is_image(n)) {
        if extension(name).as_deref() == Some("svg") {
            bail!("{name}: SVG is not supported; export it to PNG first");
        }
        let webp = to_webp(data).with_context(|| format!("converting {name}"))?;
        let stem = name.rsplit_once('.').map_or(name.as_str(), |(s, _)| s);
        let new_name = format!("{stem}.webp");
        markdown = markdown.replace(&format!("]({name})"), &format!("]({new_name})"));
        assets.insert(new_name, webp);
    }
    let total_bytes = assets.values().map(Vec::len).sum();
    Ok(Converted {
        assets,
        markdown,
        total_bytes,
    })
}

pub fn to_webp(data: &[u8]) -> anyhow::Result<Vec<u8>> {
    let format = image::guess_format(data)?;
    let mut img = image::load_from_memory_with_format(data, format)?;
    if img.width() > MAX_DIMENSION || img.height() > MAX_DIMENSION {
        img = img.resize(MAX_DIMENSION, MAX_DIMENSION, FilterType::Lanczos3);
    }
    let rgba = img.to_rgba8();
    let mut out = Cursor::new(Vec::new());
    rgba.write_with_encoder(WebPEncoder::new_lossless(&mut out))?;
    debug_assert_eq!(
        image::guess_format(out.get_ref()).ok(),
        Some(ImageFormat::WebP)
    );
    Ok(out.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};

    fn png(width: u32, height: u32) -> Vec<u8> {
        let img = ImageBuffer::from_pixel(width, height, Rgb([200u8, 30, 30]));
        let mut out = Cursor::new(Vec::new());
        img.write_to(&mut out, ImageFormat::Png).unwrap();
        out.into_inner()
    }

    #[test]
    fn images_large_png_is_resized_webp_and_markdown_rewritten() {
        let files = BTreeMap::from([("map.png".to_string(), png(3000, 2000))]);
        let out = convert(&files, "Look: ![map](map.png)").unwrap();
        assert_eq!(out.markdown, "Look: ![map](map.webp)");
        let webp = &out.assets["map.webp"];
        let decoded = image::load_from_memory_with_format(webp, ImageFormat::WebP).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (1600, 1067));
    }

    #[test]
    fn images_svg_is_refused_with_advice() {
        let files = BTreeMap::from([("logo.svg".to_string(), b"<svg/>".to_vec())]);
        let Err(err) = convert(&files, "") else {
            panic!("svg must be refused");
        };
        assert!(err.to_string().contains("export it to PNG"));
    }
}
