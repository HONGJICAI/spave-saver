//! Detection of invalid or corrupted files.
//!
//! A file is reported as broken when we can *prove* it is unusable:
//! - its bytes do not match its extension (e.g. a `.jpg` that is really a
//!   PDF) — [`BrokenCategory::ExtensionMismatch`], or
//! - it claims a known format but cannot be parsed as that format because it
//!   is corrupted or truncated — [`BrokenCategory::Corrupted`].
//!
//! Files we cannot prove broken are never reported. Unknown extensions, files
//! we cannot read (permission errors), and formats without a deep validator
//! (beyond their signature) are left alone, so the result is safe to offer for
//! deletion without false positives.

use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::Path;
use tracing::debug;

/// Why a file is considered broken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrokenCategory {
    /// The content is corrupted or truncated and cannot be parsed as its
    /// declared format (e.g. a JPEG whose pixel data is cut off).
    Corrupted,
    /// The content does not match the extension (e.g. a `.jpg` whose bytes are
    /// actually a PDF). The file may be perfectly valid — just misnamed.
    ExtensionMismatch,
}

/// The reason a file failed validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrokenReason {
    pub category: BrokenCategory,
    /// Human-readable explanation, worded close to the underlying error so it
    /// can be shown directly in the UI.
    pub detail: String,
    /// For `ExtensionMismatch`, the extension matching the actual content
    /// (e.g. `pdf` for a PDF named `.jpg`), so the UI can offer to rename
    /// instead of delete. `None` for corruption.
    pub suggested_extension: Option<String>,
}

impl BrokenReason {
    fn corrupted(detail: impl Into<String>) -> Self {
        Self {
            category: BrokenCategory::Corrupted,
            detail: detail.into(),
            suggested_extension: None,
        }
    }

    /// `actual` is the canonical format name sniffed from the content;
    /// `ext` is the (wrong) extension the file currently carries.
    fn extension_mismatch(actual: &'static str, ext: &str) -> Self {
        Self {
            category: BrokenCategory::ExtensionMismatch,
            detail: format!("Content looks like {actual} but the extension is .{ext}"),
            suggested_extension: Some(canonical_extension(actual).to_string()),
        }
    }
}

/// Validates files for corruption or content/extension mismatch.
pub struct BrokenFileChecker;

impl BrokenFileChecker {
    pub fn new() -> Self {
        Self
    }

    /// Inspect a single file. Returns `Some` only when the file is provably
    /// broken; `None` means valid, unknown, or impossible to verify.
    pub fn check_file(&self, path: &Path) -> Option<BrokenReason> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Only formats we know how to verify are candidates. An unknown
        // extension carries no expectation, so we never flag it.
        let declared = declared_format(&ext)?;

        // A file we cannot read cannot be proven broken; leave it alone.
        let header = match read_header(path, 32) {
            Some(header) => header,
            None => {
                debug!("Skipping unreadable file: {}", path.display());
                return None;
            }
        };

        match sniff_format(&header) {
            // The bytes match a known format other than the one the extension
            // claims: the file is misnamed (it may still be valid content).
            Some(actual) if actual != declared => {
                return Some(BrokenReason::extension_mismatch(actual, &ext));
            }
            // Signature matches the extension: fall through to deep validation.
            Some(_) => {}
            // Every format we declare has a required signature, so its absence
            // means the header is garbage or the file is truncated.
            None => {
                return Some(BrokenReason::corrupted(format!(
                    "Missing or invalid {declared} file signature"
                )));
            }
        }

        // The signature is right; parse the body where we have a validator.
        match declared {
            "jpeg" | "png" | "gif" | "bmp" | "webp" => deep_validate_image(path, declared),
            "zip" => deep_validate_zip(path),
            "gzip" => deep_validate_gzip(path),
            // pdf, mp4: signature-level check only (no bundled deep parser).
            _ => None,
        }
    }
}

impl Default for BrokenFileChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Map a lowercase extension to the canonical format name we validate against.
/// Returns `None` for extensions we make no claims about.
fn declared_format(ext: &str) -> Option<&'static str> {
    match ext {
        "jpg" | "jpeg" => Some("jpeg"),
        "png" => Some("png"),
        "gif" => Some("gif"),
        "bmp" => Some("bmp"),
        "webp" => Some("webp"),
        "pdf" => Some("pdf"),
        "zip" => Some("zip"),
        "gz" | "gzip" => Some("gzip"),
        "mp4" | "m4v" | "mov" => Some("mp4"),
        _ => None,
    }
}

/// The preferred file extension for a canonical format name. Takes a
/// `'static` name because every caller passes a sniffed/declared format
/// literal, letting the unknown-format arm fall back to the name itself.
fn canonical_extension(format: &'static str) -> &'static str {
    match format {
        "jpeg" => "jpg",
        "png" => "png",
        "gif" => "gif",
        "bmp" => "bmp",
        "webp" => "webp",
        "pdf" => "pdf",
        "zip" => "zip",
        "gzip" => "gz",
        "mp4" => "mp4",
        // Unknown format names are returned unchanged.
        other => other,
    }
}

/// The extension matching a file's actual content, sniffed from its leading
/// bytes. Returns `None` when the content is not a format we recognize.
pub fn detected_extension(path: &Path) -> Option<&'static str> {
    let header = read_header(path, 32)?;
    sniff_format(&header).map(canonical_extension)
}

/// The extension a misnamed file *should* have, or `None` when there is
/// nothing to fix — either the content is unrecognized, or the current
/// extension already matches it (after normalizing aliases like `jpeg`/`jpg`).
/// Used to safely rename a file to match its real content.
pub fn extension_fix_for(path: &Path) -> Option<&'static str> {
    let detected = detected_extension(path)?;
    let current = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    // Normalize the current extension to its canonical format before comparing
    // so e.g. a `.jpeg` holding JPEG bytes is not treated as needing a fix.
    let current_canonical = declared_format(&current).map(canonical_extension);
    if current_canonical == Some(detected) {
        None
    } else {
        Some(detected)
    }
}

/// Identify a file's actual format from its leading bytes (magic numbers).
/// Returns `None` when no known signature matches.
fn sniff_format(header: &[u8]) -> Option<&'static str> {
    if header.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("jpeg");
    }
    if header.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Some("png");
    }
    if header.starts_with(b"GIF87a") || header.starts_with(b"GIF89a") {
        return Some("gif");
    }
    if header.starts_with(b"BM") {
        return Some("bmp");
    }
    if header.len() >= 12 && &header[0..4] == b"RIFF" && &header[8..12] == b"WEBP" {
        return Some("webp");
    }
    if header.starts_with(b"%PDF") {
        return Some("pdf");
    }
    // The three ZIP local/central/spanned signatures.
    if header.starts_with(b"PK\x03\x04")
        || header.starts_with(b"PK\x05\x06")
        || header.starts_with(b"PK\x07\x08")
    {
        return Some("zip");
    }
    if header.starts_with(&[0x1F, 0x8B]) {
        return Some("gzip");
    }
    // ISO base media (mp4/mov/m4v): "ftyp" box at offset 4.
    if header.len() >= 8 && &header[4..8] == b"ftyp" {
        return Some("mp4");
    }
    None
}

/// Read up to `n` leading bytes. Returns `None` if the file cannot be opened
/// or read (treated as "cannot verify", never as broken).
fn read_header(path: &Path, n: usize) -> Option<Vec<u8>> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut buf = vec![0u8; n];
    let read = file.read(&mut buf).ok()?;
    buf.truncate(read);
    Some(buf)
}

/// Validate an image beyond its signature. The header is parsed for every
/// format (catches truncated/garbage headers, including WebP); the body is
/// additionally fully decoded for formats the `image` crate reliably supports,
/// catching corruption past the header.
fn deep_validate_image(path: &Path, declared: &str) -> Option<BrokenReason> {
    if let Err(e) = imagesize::size(path) {
        return Some(BrokenReason::corrupted(format!(
            "Image header is invalid: {e}"
        )));
    }

    // WebP decoding is not guaranteed to be compiled in, so a decode failure
    // there could be a false positive; trust the header parse for it.
    if declared != "webp" {
        if let Err(e) = image::open(path) {
            return Some(BrokenReason::corrupted(format!(
                "Image cannot be decoded: {e}"
            )));
        }
    }

    None
}

/// Validate a ZIP archive by opening its central directory.
fn deep_validate_zip(path: &Path) -> Option<BrokenReason> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(e) => return Some(BrokenReason::corrupted(format!("Cannot open file: {e}"))),
    };
    match zip::ZipArchive::new(file) {
        Ok(_) => None,
        Err(e) => Some(BrokenReason::corrupted(format!("Invalid ZIP archive: {e}"))),
    }
}

/// Validate a gzip stream by decompressing it end to end (also checks the CRC).
fn deep_validate_gzip(path: &Path) -> Option<BrokenReason> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(e) => return Some(BrokenReason::corrupted(format!("Cannot open file: {e}"))),
    };
    let mut decoder = flate2::read::GzDecoder::new(file);
    let mut buf = [0u8; 8192];
    loop {
        match decoder.read(&mut buf) {
            Ok(0) => return None,
            Ok(_) => continue,
            Err(e) => return Some(BrokenReason::corrupted(format!("Invalid gzip stream: {e}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    fn check(path: &Path) -> Option<BrokenReason> {
        BrokenFileChecker::new().check_file(path)
    }

    #[test]
    fn valid_png_is_not_broken() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("ok.png");
        let img: image::RgbImage = ImageBuffer::from_fn(8, 8, |_, _| Rgb([10, 20, 30]));
        img.save(&path).unwrap();

        assert_eq!(check(&path), None);
    }

    #[test]
    fn valid_jpeg_is_not_broken() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("ok.jpg");
        let img: image::RgbImage =
            ImageBuffer::from_fn(8, 8, |x, y| Rgb([(x * 8) as u8, (y * 8) as u8, 128]));
        img.save(&path).unwrap();

        assert_eq!(check(&path), None);
    }

    #[test]
    fn truncated_jpeg_is_corrupted() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("truncated.jpg");
        // Valid JPEG signature followed by nothing usable.
        fs::write(&path, [0xFF, 0xD8, 0xFF, 0xE0]).unwrap();

        let reason = check(&path).expect("truncated jpeg must be flagged");
        assert_eq!(reason.category, BrokenCategory::Corrupted);
    }

    #[test]
    fn png_with_garbage_body_is_corrupted() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bad.png");
        // No PNG signature at all.
        fs::write(&path, b"this is definitely not a png").unwrap();

        let reason = check(&path).expect("garbage png must be flagged");
        assert_eq!(reason.category, BrokenCategory::Corrupted);
        assert!(reason.detail.contains("signature"));
    }

    #[test]
    fn pdf_content_named_jpg_is_extension_mismatch() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("photo.jpg");
        fs::write(&path, b"%PDF-1.7\n%fake pdf body").unwrap();

        let reason = check(&path).expect("misnamed file must be flagged");
        assert_eq!(reason.category, BrokenCategory::ExtensionMismatch);
        assert!(reason.detail.contains("pdf"));
    }

    #[test]
    fn pdf_content_named_webp_is_extension_mismatch() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("image.webp");
        fs::write(&path, b"%PDF-1.7\n%not really a webp").unwrap();

        let reason = check(&path).expect("misnamed webp must be flagged");
        assert_eq!(reason.category, BrokenCategory::ExtensionMismatch);
    }

    #[test]
    fn mismatch_suggests_the_correct_extension() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("photo.jpg");
        fs::write(&path, b"%PDF-1.7\n%fake pdf body").unwrap();

        let reason = check(&path).unwrap();
        assert_eq!(reason.suggested_extension.as_deref(), Some("pdf"));
    }

    #[test]
    fn corrupted_has_no_suggested_extension() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("truncated.jpg");
        fs::write(&path, [0xFF, 0xD8, 0xFF, 0xE0]).unwrap();

        let reason = check(&path).unwrap();
        assert_eq!(reason.category, BrokenCategory::Corrupted);
        assert_eq!(reason.suggested_extension, None);
    }

    #[test]
    fn detected_extension_reads_real_format() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("misnamed.jpg");
        fs::write(&path, b"%PDF-1.7\nbody").unwrap();

        assert_eq!(detected_extension(&path), Some("pdf"));
    }

    #[test]
    fn detected_extension_none_for_unrecognized_content() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("mystery.bin");
        fs::write(&path, b"\x00\x01\x02not a known signature").unwrap();

        assert_eq!(detected_extension(&path), None);
    }

    #[test]
    fn extension_fix_for_suggests_rename_target() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("scan.jpg");
        fs::write(&path, b"%PDF-1.7\nbody").unwrap();

        assert_eq!(extension_fix_for(&path), Some("pdf"));
    }

    #[test]
    fn extension_fix_for_none_when_already_correct() {
        // A valid JPEG named .jpeg must not be "fixed" to .jpg: jpeg/jpg are
        // the same canonical format.
        let dir = tempdir().unwrap();
        let path = dir.path().join("photo.jpeg");
        let img: image::RgbImage = ImageBuffer::from_fn(8, 8, |_, _| Rgb([1, 2, 3]));
        img.save(&path).unwrap();

        assert_eq!(extension_fix_for(&path), None);
    }

    #[test]
    fn extension_fix_for_none_for_unrecognized_content() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("mystery.dat");
        fs::write(&path, b"\x00\x01\x02nothing known").unwrap();

        assert_eq!(extension_fix_for(&path), None);
    }

    #[test]
    fn valid_zip_is_not_broken() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("ok.zip");
        let file = fs::File::create(&path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file("hello.txt", zip::write::FileOptions::default())
            .unwrap();
        zip.write_all(b"hello").unwrap();
        zip.finish().unwrap();

        assert_eq!(check(&path), None);
    }

    #[test]
    fn corrupt_zip_is_corrupted() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bad.zip");
        // Right signature, but no valid central directory follows.
        fs::write(&path, b"PK\x03\x04 not a real archive").unwrap();

        let reason = check(&path).expect("corrupt zip must be flagged");
        assert_eq!(reason.category, BrokenCategory::Corrupted);
        assert!(reason.detail.contains("ZIP"));
    }

    #[test]
    fn valid_gzip_is_not_broken() {
        use flate2::write::GzEncoder;
        use flate2::Compression;

        let dir = tempdir().unwrap();
        let path = dir.path().join("ok.gz");
        let mut encoder = GzEncoder::new(fs::File::create(&path).unwrap(), Compression::default());
        encoder.write_all(b"some compressible content").unwrap();
        encoder.finish().unwrap();

        assert_eq!(check(&path), None);
    }

    #[test]
    fn corrupt_gzip_is_corrupted() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bad.gz");
        // gzip magic followed by an invalid stream.
        fs::write(&path, [0x1F, 0x8B, 0x08, 0x00, 0xDE, 0xAD, 0xBE, 0xEF]).unwrap();

        let reason = check(&path).expect("corrupt gzip must be flagged");
        assert_eq!(reason.category, BrokenCategory::Corrupted);
    }

    #[test]
    fn unknown_extension_is_never_flagged() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("notes.xyz");
        fs::write(&path, b"arbitrary bytes \x00\x01\x02").unwrap();

        assert_eq!(check(&path), None);
    }

    #[test]
    fn file_without_extension_is_never_flagged() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("README");
        fs::write(&path, b"plain text").unwrap();

        assert_eq!(check(&path), None);
    }

    #[test]
    fn nonexistent_file_is_not_flagged() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("missing.png");
        assert_eq!(check(&path), None);
    }

    #[test]
    fn valid_pdf_signature_passes() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("doc.pdf");
        fs::write(&path, b"%PDF-1.7\n1 0 obj\n<<>>\nendobj\n").unwrap();

        // Signature-level only: a real PDF header is accepted.
        assert_eq!(check(&path), None);
    }

    #[test]
    fn empty_known_format_reports_missing_signature() {
        // The checker itself flags a 0-byte known-format file as corrupted
        // (no signature); the service layer is what excludes empty files so
        // they stay the Empty Files feature's responsibility.
        let dir = tempdir().unwrap();
        let path = dir.path().join("empty.png");
        fs::write(&path, b"").unwrap();

        let reason = check(&path).expect("empty png has no signature");
        assert_eq!(reason.category, BrokenCategory::Corrupted);
    }
}
