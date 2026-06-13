//! Thumbnail generation for image previews.
//!
//! The similar-media UI needs to *show* the images so the user can decide
//! which copy to keep. Rather than expose raw file paths to the webview (which
//! would require a broad asset-protocol scope over arbitrary scanned
//! directories), the backend decodes the image, shrinks it, and returns a
//! self-contained `data:` URL the frontend can drop straight into `<img src>`.

use anyhow::Result;
use base64::{engine::general_purpose::STANDARD, Engine};
use std::io::Cursor;
use std::path::Path;

/// Decode `path`, shrink it to fit within `max_size`×`max_size` (aspect ratio
/// preserved), and return a PNG `data:` URL. `max_size` is clamped to at least
/// 1px so a zero never produces an empty thumbnail.
pub fn thumbnail_data_url(path: &Path, max_size: u32) -> Result<String> {
    let max_size = max_size.max(1);
    let img = image::open(path)?;

    // `thumbnail` uses a fast filter and preserves aspect ratio
    let thumb = img.thumbnail(max_size, max_size);

    let mut buf = Cursor::new(Vec::new());
    thumb.write_to(&mut buf, image::ImageOutputFormat::Png)?;

    let encoded = STANDARD.encode(buf.get_ref());
    Ok(format!("data:image/png;base64,{encoded}"))
}

/// Read an image's pixel dimensions from its header only (no full decode).
/// Returns `None` when the file is missing or not a recognizable image.
pub fn image_dimensions(path: &Path) -> Option<(u32, u32)> {
    imagesize::size(path)
        .ok()
        .map(|s| (s.width as u32, s.height as u32))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};
    use tempfile::tempdir;

    fn save_png(path: &Path, width: u32, height: u32) {
        let img: image::RgbImage = ImageBuffer::from_fn(width, height, |x, y| {
            Rgb([(x % 256) as u8, (y % 256) as u8, 0])
        });
        img.save(path).unwrap();
    }

    #[test]
    fn thumbnail_returns_png_data_url() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("img.png");
        save_png(&path, 200, 100);

        let url = thumbnail_data_url(&path, 64).unwrap();
        assert!(url.starts_with("data:image/png;base64,"));
        // The payload must be non-trivial base64
        assert!(url.len() > "data:image/png;base64,".len() + 16);
    }

    #[test]
    fn thumbnail_clamps_zero_max_size() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("img.png");
        save_png(&path, 32, 32);

        // max_size 0 must not panic or yield an empty image
        let url = thumbnail_data_url(&path, 0).unwrap();
        assert!(url.starts_with("data:image/png;base64,"));
    }

    #[test]
    fn thumbnail_errors_on_missing_file() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("nope.png");
        assert!(thumbnail_data_url(&missing, 64).is_err());
    }

    #[test]
    fn thumbnail_errors_on_non_image() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("not-image.png");
        std::fs::write(&path, b"definitely not a png").unwrap();
        assert!(thumbnail_data_url(&path, 64).is_err());
    }

    #[test]
    fn dimensions_reads_header() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("img.png");
        save_png(&path, 120, 80);
        assert_eq!(image_dimensions(&path), Some((120, 80)));
    }

    #[test]
    fn dimensions_none_for_missing_or_invalid() {
        let dir = tempdir().unwrap();
        assert_eq!(image_dimensions(&dir.path().join("missing.png")), None);

        let bad = dir.path().join("bad.png");
        std::fs::write(&bad, b"not an image").unwrap();
        assert_eq!(image_dimensions(&bad), None);
    }
}
