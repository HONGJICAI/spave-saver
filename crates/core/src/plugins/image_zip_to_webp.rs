use anyhow::{Context, Result};
use image::DynamicImage;
use rayon::prelude::*;
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use tracing::warn;
use zip::result::ZipError;
use zip::{write::FileOptions, CompressionMethod, ZipArchive, ZipWriter};

use crate::compress_plugins::{
    create_output_file, get_file_size, has_extension, CompressionError, CompressionErrorKind,
    CompressionPlugin, CompressionResult, PluginMetadata,
};

/// Archive extensions this plugin handles (`.cbz` is a ZIP of comic pages)
const ARCHIVE_EXTENSIONS: &[&str] = &["zip", "cbz"];

/// WebP cannot encode an image wider or taller than this
const WEBP_MAX_DIMENSION: u32 = 16383;

/// Image entries larger than this (uncompressed) are copied as is instead of
/// being read into memory
const DEFAULT_MAX_IMAGE_ENTRY_BYTES: u64 = 256 * 1024 * 1024;

/// Images are read and converted in parallel batches; a batch closes once it
/// holds this many images or bytes, which bounds memory use
const BATCH_MAX_IMAGES: usize = 64;
const BATCH_MAX_BYTES: u64 = 256 * 1024 * 1024;

/// JPEGs below this many bits per pixel are already compressed well enough
/// that re-encoding them (lossy again) gains little and costs quality.
/// Typical photos at quality ~85 sit around 1-2 bpp; quality 95-100 or no
/// chroma subsampling pushes them to 3-8 bpp.
const DEFAULT_MIN_JPEG_BPP: f32 = 3.0;

/// Plugin for converting ZIP files containing images to WebP format
/// Reads ZIP, converts all images to WebP, and creates a new ZIP
pub struct ImageZipToWebpZipPlugin {
    quality: f32,
    min_image_ratio: f32, // Minimum ratio of images to total files to process
    min_jpeg_bpp: f32,
    max_image_entry_bytes: u64,
}

/// What happened to one image entry
enum ImageOutcome {
    /// The WebP is smaller; write it in place of the image
    Converted(Vec<u8>),
    /// Not worth converting (WebP not smaller, or a well-compressed JPEG)
    Kept,
    /// Could not be converted; the reason is reported as a warning
    Failed(String),
}

/// What converting an archive produced
struct ZipConversion {
    /// Images written as WebP
    converted: usize,
    /// Images that failed to decode or encode
    failed: usize,
    /// One message per image left unconverted for a reason the user should
    /// know about (failed, too large, no free name)
    warnings: Vec<String>,
}

/// An image entry read from the source archive, waiting to be converted
struct PendingImage {
    index: usize,
    data: Vec<u8>,
}

impl ImageZipToWebpZipPlugin {
    pub fn new() -> Self {
        Self {
            quality: 85.0,
            min_image_ratio: 1.0, // At least 100% of files should be images
            min_jpeg_bpp: DEFAULT_MIN_JPEG_BPP,
            max_image_entry_bytes: DEFAULT_MAX_IMAGE_ENTRY_BYTES,
        }
    }

    pub fn with_quality(mut self, quality: f32) -> Self {
        self.quality = quality.clamp(0.0, 100.0);
        self
    }

    pub fn with_min_image_ratio(mut self, ratio: f32) -> Self {
        self.min_image_ratio = ratio.clamp(0.0, 1.0);
        self
    }

    /// JPEGs below `bpp` bits per pixel are kept as is (0 converts every JPEG)
    pub fn with_min_jpeg_bpp(mut self, bpp: f32) -> Self {
        self.min_jpeg_bpp = bpp.max(0.0);
        self
    }

    /// Lets tests exercise the size cap without building huge archives
    #[cfg(test)]
    fn with_max_image_entry_bytes(mut self, bytes: u64) -> Self {
        self.max_image_entry_bytes = bytes;
        self
    }

    /// Images this plugin converts to WebP
    fn is_image_file(filename: &str) -> bool {
        let lower = filename.to_lowercase();
        lower.ends_with(".png")
            || lower.ends_with(".jpg")
            || lower.ends_with(".jpeg")
            || lower.ends_with(".bmp")
    }

    fn is_jpeg(filename: &str) -> bool {
        let lower = filename.to_lowercase();
        lower.ends_with(".jpg") || lower.ends_with(".jpeg")
    }

    fn is_webp(filename: &str) -> bool {
        filename.to_lowercase().ends_with(".webp")
    }

    /// Images that count toward the image ratio but are copied as is
    /// (already WebP, or animated formats a still WebP would flatten)
    fn is_passthrough_image(filename: &str) -> bool {
        Self::is_webp(filename) || filename.to_lowercase().ends_with(".gif")
    }

    /// Files that archivers and operating systems drop into image archives.
    /// They do not count against `min_image_ratio` and are copied as is.
    fn is_incidental_file(filename: &str) -> bool {
        let lower = filename.to_lowercase();
        if lower.starts_with("__macosx/") || lower.contains("/__macosx/") {
            return true;
        }
        let base = lower.rsplit('/').next().unwrap_or(&lower);
        matches!(
            base,
            "comicinfo.xml" | "thumbs.db" | ".ds_store" | "desktop.ini"
        )
    }

    /// Decode an entry name from its raw bytes. zip 0.6 decodes names without
    /// the UTF-8 flag as CP437, which mangles UTF-8 names from tools that do
    /// not set the flag and GBK names from Chinese Windows. The writer always
    /// stores names as UTF-8, so a name decoded wrongly here would be
    /// corrupted in the output. Raw bytes that are neither valid UTF-8 nor
    /// valid GBK fall back to the reader's CP437 decoding (the ZIP default).
    fn decode_entry_name(raw: &[u8], fallback: &str) -> String {
        if let Ok(name) = std::str::from_utf8(raw) {
            return name.to_string();
        }
        if let Some(name) =
            encoding_rs::GBK.decode_without_bom_handling_and_without_replacement(raw)
        {
            return name.into_owned();
        }
        fallback.to_string()
    }

    /// Pick the output name for a converted image: `a.png` -> `a.webp`, or
    /// `a.png.webp` when `a.webp` is already taken by another entry or an
    /// earlier conversion. None when both are taken, so the image is kept.
    /// `taken` holds lowercased names: extracting on Windows/macOS is
    /// case-insensitive, so `A.webp` and `a.webp` would overwrite each other.
    fn webp_name_for(name: &str, taken: &HashSet<String>) -> Option<String> {
        let replaced = match name.rfind('.') {
            Some(idx) => format!("{}.webp", &name[..idx]),
            None => format!("{}.webp", name),
        };
        [replaced, format!("{}.webp", name)]
            .into_iter()
            .find(|candidate| !taken.contains(&candidate.to_lowercase()))
    }

    fn open_archive(path: &Path) -> Result<ZipArchive<File>> {
        let file = File::open(path)?;
        ZipArchive::new(file).map_err(|e| {
            anyhow::Error::new(CompressionError::new(
                CompressionErrorKind::CorruptFile,
                format!("Failed to read ZIP archive {}: {}", path.display(), e),
            ))
        })
    }

    /// Whether the archive is worth converting, with the reason shown to the
    /// user when it is not
    fn inspect_archive(&self, path: &Path) -> Result<(bool, String)> {
        let mut archive = Self::open_archive(path)?;

        let mut counted_files = 0;
        let mut image_count = 0;
        let mut convertible_count = 0;

        for i in 0..archive.len() {
            // by_index (unlike by_index_raw) refuses encrypted entries, which
            // could be neither converted nor copied without the password
            let file = match archive.by_index(i) {
                Ok(file) => file,
                Err(ZipError::UnsupportedArchive(msg)) if msg == ZipError::PASSWORD_REQUIRED => {
                    return Ok((false, "ZIP archive contains encrypted entries".to_string()));
                }
                Err(e) => return Err(e.into()),
            };
            let name = file.name();

            // Directory entries and OS clutter say nothing about the content
            if file.is_dir() || Self::is_incidental_file(name) {
                continue;
            }
            counted_files += 1;

            if Self::is_image_file(name) {
                image_count += 1;
                convertible_count += 1;
            } else if Self::is_passthrough_image(name) {
                image_count += 1;
            }
        }

        // Only process if:
        // 1. There are images that are not WebP yet
        // 2. Images make up at least min_image_ratio of the counted files
        if convertible_count == 0 {
            return Ok((false, "ZIP file contains no convertible images".to_string()));
        }
        let image_ratio = image_count as f32 / counted_files as f32;
        if image_ratio < self.min_image_ratio {
            return Ok((
                false,
                format!(
                    "ZIP file is not an image archive ({} of {} files are images)",
                    image_count, counted_files
                ),
            ));
        }
        Ok((true, "ZIP file contains convertible images".to_string()))
    }

    /// Whether a JPEG holds enough bits per pixel to be worth re-encoding.
    /// Reads only the header; an unreadable header goes on to the full
    /// decode, which reports the actual error.
    fn jpeg_worth_converting(&self, data: &[u8]) -> bool {
        match imagesize::blob_size(data) {
            Ok(size) if size.width > 0 && size.height > 0 => {
                let bpp = data.len() as f64 * 8.0 / (size.width as f64 * size.height as f64);
                bpp >= self.min_jpeg_bpp as f64
            }
            _ => true,
        }
    }

    /// EXIF orientation (1-8) of a JPEG, or 1 when it has none
    fn jpeg_orientation(data: &[u8]) -> u32 {
        exif::Reader::new()
            .read_from_container(&mut Cursor::new(data))
            .ok()
            .and_then(|exif| {
                exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY)
                    .and_then(|field| field.value.get_uint(0))
            })
            .unwrap_or(1)
    }

    /// Rotate/flip pixels so they display upright without the EXIF tag,
    /// which the WebP output does not carry
    fn apply_orientation(img: DynamicImage, orientation: u32) -> DynamicImage {
        match orientation {
            2 => img.fliph(),
            3 => img.rotate180(),
            4 => img.flipv(),
            5 => img.rotate90().fliph(),
            6 => img.rotate90(),
            7 => img.rotate270().fliph(),
            8 => img.rotate270(),
            _ => img,
        }
    }

    /// Decode with dimension limits (WebP's maximum) so a huge or malicious
    /// image fails fast instead of allocating gigabytes
    fn decode_image(data: &[u8]) -> Result<DynamicImage> {
        let mut reader = image::io::Reader::new(Cursor::new(data)).with_guessed_format()?;
        let mut limits = image::io::Limits::default();
        limits.max_image_width = Some(WEBP_MAX_DIMENSION);
        limits.max_image_height = Some(WEBP_MAX_DIMENSION);
        reader.limits(limits);
        Ok(reader.decode()?)
    }

    fn convert_image_to_webp(&self, data: &[u8], original_name: &str) -> Result<Vec<u8>> {
        let mut img = Self::decode_image(data)
            .with_context(|| format!("Failed to decode image: {}", original_name))?;

        // Viewers honor EXIF orientation in JPEGs (phone photos rely on it),
        // so bake it into the pixels before the tag is lost
        if Self::is_jpeg(original_name) {
            img = Self::apply_orientation(img, Self::jpeg_orientation(data));
        }

        self.encode_webp(&img)
    }

    fn encode_webp(&self, img: &DynamicImage) -> Result<Vec<u8>> {
        use image::GenericImageView;
        use webp::Encoder;

        let (width, height) = img.dimensions();
        let rgba = img.to_rgba8();

        let encoder = Encoder::from_rgba(&rgba, width, height);
        let encoded = encoder.encode(self.quality);

        Ok(encoded.to_vec())
    }

    fn convert_entry(&self, name: &str, data: &[u8]) -> ImageOutcome {
        if Self::is_jpeg(name) && !self.jpeg_worth_converting(data) {
            return ImageOutcome::Kept;
        }
        match self.convert_image_to_webp(data, name) {
            Ok(webp_data) if webp_data.len() < data.len() => ImageOutcome::Converted(webp_data),
            // WebP is not smaller (e.g. an already well-compressed image)
            Ok(_) => ImageOutcome::Kept,
            Err(e) => ImageOutcome::Failed(format!("{:#}", e)),
        }
    }

    /// Write a copy of `source` to `output` with its images converted to
    /// WebP. An image that cannot be converted (undecodable, too large, no
    /// free name) is kept as is with a warning; one not worth converting is
    /// kept silently. Every other entry is copied byte for byte (compressed
    /// data, timestamp, permissions) under its decoded name.
    ///
    /// Images are read in batches and converted in parallel; entries are
    /// written in their original order.
    fn process_zip(&self, source: &Path, output: &Path) -> Result<ZipConversion> {
        let mut input_archive = Self::open_archive(source)?;

        let mut names = Vec::with_capacity(input_archive.len());
        for i in 0..input_archive.len() {
            let file = input_archive.by_index_raw(i)?;
            names.push(Self::decode_entry_name(file.name_raw(), file.name()));
        }
        // Reserve every existing name so a converted image never shadows one
        let mut taken: HashSet<String> = names.iter().map(|n| n.to_lowercase()).collect();

        // create_new (O_EXCL): fails instead of overwriting a concurrent
        // writer's output with the same name
        let output_file = create_output_file(output)?;
        let mut output_archive = ZipWriter::new(output_file);

        let mut files_processed = 0;
        let mut failed = 0;
        let mut warnings = Vec::new();

        let mut start = 0;
        while start < names.len() {
            // Read the next batch of images
            let mut batch: Vec<PendingImage> = Vec::new();
            let mut batch_bytes = 0u64;
            let mut end = start;
            while end < names.len()
                && batch.len() < BATCH_MAX_IMAGES
                && batch_bytes < BATCH_MAX_BYTES
            {
                let index = end;
                end += 1;
                if !Self::is_image_file(&names[index]) {
                    continue;
                }
                let file = input_archive.by_index(index)?;
                if file.is_dir() {
                    continue;
                }
                if file.size() > self.max_image_entry_bytes {
                    warnings.push(format!(
                        "{}: image is too large to convert ({} bytes); kept as is",
                        names[index],
                        file.size()
                    ));
                    continue;
                }
                let mut data = Vec::with_capacity(file.size() as usize);
                // take(): the size header is not trusted to bound the read
                file.take(self.max_image_entry_bytes + 1)
                    .read_to_end(&mut data)?;
                if data.len() as u64 > self.max_image_entry_bytes {
                    warnings.push(format!(
                        "{}: image is too large to convert; kept as is",
                        names[index]
                    ));
                    continue;
                }
                batch_bytes += data.len() as u64;
                batch.push(PendingImage { index, data });
            }

            let outcomes: Vec<ImageOutcome> = batch
                .par_iter()
                .map(|pending| self.convert_entry(&names[pending.index], &pending.data))
                .collect();
            let mut converted = batch
                .iter()
                .map(|pending| pending.index)
                .zip(outcomes)
                .peekable();

            // Write the batch's entries in order
            for (i, name) in names.iter().enumerate().take(end).skip(start) {
                let file = input_archive.by_index(i)?;

                if file.is_dir() {
                    let options = FileOptions::default().last_modified_time(file.last_modified());
                    output_archive.add_directory(name.as_str(), options)?;
                    continue;
                }

                if let Some((_, outcome)) = converted.next_if(|(index, _)| *index == i) {
                    match outcome {
                        ImageOutcome::Converted(webp_data) => {
                            if let Some(new_name) = Self::webp_name_for(name, &taken) {
                                taken.insert(new_name.to_lowercase());
                                // WebP is already compressed; deflating it
                                // again only costs CPU
                                let options = FileOptions::default()
                                    .compression_method(CompressionMethod::Stored)
                                    .last_modified_time(file.last_modified());
                                drop(file); // Release the borrow
                                output_archive.start_file(new_name, options)?;
                                output_archive.write_all(&webp_data)?;
                                files_processed += 1;
                                continue;
                            }
                            warnings.push(format!(
                                "{}: no free name for the WebP entry; kept as is",
                                name
                            ));
                        }
                        ImageOutcome::Kept => {}
                        ImageOutcome::Failed(reason) => {
                            failed += 1;
                            warn!(entry = %name, error = %reason, "Failed to convert image; keeping original");
                            warnings.push(format!("{}: {}; kept as is", name, reason));
                        }
                    }
                }

                output_archive.raw_copy_file_rename(file, name.as_str())?;
            }

            start = end;
        }

        output_archive.finish()?;

        Ok(ZipConversion {
            converted: files_processed,
            failed,
            warnings,
        })
    }
}

impl Default for ImageZipToWebpZipPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl CompressionPlugin for ImageZipToWebpZipPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "Image ZIP to WebP ZIP".to_string(),
            description: "Converts images inside ZIP/CBZ archives to WebP format".to_string(),
            version: "1.1.0".to_string(),
        }
    }

    fn can_handle(&self, path: &Path) -> Result<(bool, Option<String>)> {
        if !path.is_file() {
            return Ok((false, Some("Not a file".to_string())));
        }

        if !has_extension(path, ARCHIVE_EXTENSIONS) {
            return Ok((false, Some("Not a ZIP/CBZ file".to_string())));
        }

        let (can_handle, reason) = self.inspect_archive(path)?;
        Ok((can_handle, Some(reason)))
    }

    fn estimate_ratio(&self, path: &Path) -> Result<Option<f32>> {
        // Try to estimate based on the types of images in the ZIP
        let mut archive = Self::open_archive(path)?;

        let mut total_size = 0u64;
        let mut image_size = 0u64;

        for i in 0..archive.len() {
            let file = archive.by_index_raw(i)?;
            let size = file.size();
            total_size += size;

            if Self::is_image_file(file.name()) {
                image_size += size;
            }
        }

        if image_size == 0 {
            return Ok(None);
        }

        // Estimate 25-30% savings on average for WebP conversion
        let image_ratio = image_size as f32 / total_size as f32;
        let estimated_savings = image_ratio * 0.28;

        Ok(Some(estimated_savings))
    }

    fn process(&self, source: &Path, output_dir: &Path) -> Result<CompressionResult> {
        let original_size = get_file_size(source)?;

        // Generate output filename, keeping the archive's extension
        let extension = source
            .extension()
            .map(|e| e.to_string_lossy().into_owned())
            .unwrap_or_else(|| "zip".to_string());
        let output_filename = if let Some(stem) = source.file_stem() {
            PathBuf::from(format!("{}_webp.{}", stem.to_string_lossy(), extension))
        } else {
            PathBuf::from(format!("converted_webp.{}", extension))
        };

        let output_path = output_dir.join(&output_filename);

        // Ensure output directory exists
        fs::create_dir_all(output_dir)?;

        // Process the ZIP file (the output is created with create_new, so an
        // existing file fails the operation); the manager backs up the
        // original and moves the output over the source path (replace_source)
        let ZipConversion {
            converted: files_processed,
            failed,
            warnings,
        } = self
            .process_zip(source, &output_path)
            .with_context(|| format!("Failed to process ZIP file: {}", source.display()))?;

        if files_processed == 0 {
            let _ = fs::remove_file(&output_path);
            if failed > 0 {
                // Nothing converted and images failed to decode: report the
                // failures rather than a plain "no reduction" skip
                return Err(anyhow::Error::new(CompressionError::new(
                    CompressionErrorKind::CorruptFile,
                    format!(
                        "No image in {} could be converted: {}",
                        source.display(),
                        warnings.join("; ")
                    ),
                )));
            }
            // Nothing was worth converting. Report no reduction so the manager
            // skips the archive (and remembers the skip) instead of replacing
            // it with a re-packed copy that differs only in ZIP overhead.
            return Ok(CompressionResult {
                original_size,
                compressed_size: original_size,
                output_path: source.to_path_buf(),
                plugin_name: self.metadata().name,
                files_processed: 0,
                backup_path: None,
                replace_source: false,
                warnings,
            });
        }

        let compressed_size = get_file_size(&output_path)?;

        Ok(CompressionResult {
            original_size,
            compressed_size,
            output_path,
            plugin_name: self.metadata().name,
            files_processed,
            backup_path: None,
            replace_source: true,
            warnings,
        })
    }

    fn supported_extensions(&self) -> Vec<&str> {
        ARCHIVE_EXTENSIONS.to_vec()
    }

    fn quality(&self) -> Option<f32> {
        Some(self.quality)
    }

    fn set_quality(&mut self, quality: f32) -> bool {
        self.quality = quality.clamp(0.0, 100.0);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compress_plugins::{CompressionOutcome, PluginManager};
    use image::{ImageBuffer, Rgb};
    use std::io::Cursor;

    /// PNG bytes of deterministic noise; PNG stores noise poorly, so the
    /// WebP conversion inside the ZIP reliably shrinks it
    fn noise_png_bytes(width: u32, height: u32) -> Vec<u8> {
        let mut seed = 0x2545F491u32;
        let img: image::RgbImage = ImageBuffer::from_fn(width, height, |_, _| {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            Rgb([
                (seed & 0xFF) as u8,
                ((seed >> 8) & 0xFF) as u8,
                ((seed >> 16) & 0xFF) as u8,
            ])
        });
        let mut buf = Vec::new();
        img.write_to(&mut Cursor::new(&mut buf), image::ImageOutputFormat::Png)
            .unwrap();
        buf
    }

    fn build_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let file = File::create(path).unwrap();
        let mut writer = ZipWriter::new(file);
        // Stored, so the noise PNG stays large and conversion shows savings
        let options = FileOptions::default().compression_method(CompressionMethod::Stored);
        for (name, data) in entries {
            writer.start_file(*name, options).unwrap();
            writer.write_all(data).unwrap();
        }
        writer.finish().unwrap();
    }

    #[test]
    fn test_is_image_file() {
        assert!(ImageZipToWebpZipPlugin::is_image_file("photo.png"));
        assert!(ImageZipToWebpZipPlugin::is_image_file("image.jpg"));
        assert!(ImageZipToWebpZipPlugin::is_image_file("PHOTO.JPEG"));
        assert!(!ImageZipToWebpZipPlugin::is_image_file("document.pdf"));
        assert!(!ImageZipToWebpZipPlugin::is_image_file("video.mp4"));
    }

    #[test]
    fn test_is_webp() {
        assert!(ImageZipToWebpZipPlugin::is_webp("photo.webp"));
        assert!(ImageZipToWebpZipPlugin::is_webp("PHOTO.WEBP"));
        assert!(!ImageZipToWebpZipPlugin::is_webp("photo.png"));
    }

    #[test]
    fn test_can_handle_zip_variants() {
        let dir = tempfile::tempdir().unwrap();
        let plugin = ImageZipToWebpZipPlugin::new();
        let png = noise_png_bytes(32, 32);

        // ZIP made only of convertible images
        let images_zip = dir.path().join("images.zip");
        build_zip(&images_zip, &[("a.png", &png), ("b.png", &png)]);
        let (can_handle, _) = plugin.can_handle(&images_zip).unwrap();
        assert!(can_handle);

        // ZIP without images
        let text_zip = dir.path().join("text.zip");
        build_zip(&text_zip, &[("readme.txt", b"hello")]);
        let (can_handle, _) = plugin.can_handle(&text_zip).unwrap();
        assert!(!can_handle);

        // Mixed ZIP below the default min_image_ratio (1.0)
        let mixed_zip = dir.path().join("mixed.zip");
        build_zip(&mixed_zip, &[("a.png", &png), ("readme.txt", b"hello")]);
        let (can_handle, _) = plugin.can_handle(&mixed_zip).unwrap();
        assert!(!can_handle);

        // A corrupt "ZIP" must surface as an error, not a panic
        let fake_zip = dir.path().join("fake.zip");
        fs::write(&fake_zip, b"this is not a zip archive").unwrap();
        let err = plugin.can_handle(&fake_zip).unwrap_err();
        assert_eq!(
            crate::compress_plugins::classify_error(&err),
            crate::compress_plugins::CompressionErrorKind::CorruptFile
        );
    }

    #[test]
    fn test_end_to_end_manager_replaces_zip_in_place() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("photos.zip");
        let png = noise_png_bytes(128, 128);
        build_zip(&source, &[("a.png", &png), ("b.png", &png)]);
        let original_bytes = fs::read(&source).unwrap();

        let mut manager = PluginManager::new();
        manager.register(Box::new(ImageZipToWebpZipPlugin::new()));

        let outcome = manager
            .process_file(&source, dir.path(), None, true)
            .unwrap();
        match outcome {
            CompressionOutcome::Compressed(result) => {
                // The converted ZIP takes over the original path (replace_source)
                assert_eq!(result.output_path, source);
                assert!(result.compressed_size < result.original_size);

                let backup = result.backup_path.unwrap();
                assert_eq!(backup, dir.path().join("photos.zip.bak"));
                assert_eq!(fs::read(&backup).unwrap(), original_bytes);

                // All image entries inside the new ZIP are WebP now
                let file = File::open(&source).unwrap();
                let mut archive = ZipArchive::new(file).unwrap();
                let names: Vec<String> = (0..archive.len())
                    .map(|i| archive.by_index(i).unwrap().name().to_string())
                    .collect();
                assert_eq!(names.len(), 2);
                assert!(
                    names.iter().all(|n| n.ends_with(".webp")),
                    "entries: {:?}",
                    names
                );
            }
            other => panic!("expected Compressed, got {:?}", other),
        }
    }

    /// JPEG of noise at low quality: the WebP of its decoded pixels is
    /// reliably larger than the JPEG itself
    fn low_quality_noise_jpeg() -> Vec<u8> {
        let img = image::load_from_memory(&noise_png_bytes(64, 64)).unwrap();
        let mut buf = Vec::new();
        img.write_to(
            &mut Cursor::new(&mut buf),
            image::ImageOutputFormat::Jpeg(20),
        )
        .unwrap();
        buf
    }

    /// Build a ZIP from entries; a name ending in '/' becomes a directory
    fn build_zip_with_dirs(path: &Path, entries: &[(&str, &[u8])]) {
        let file = File::create(path).unwrap();
        let mut writer = ZipWriter::new(file);
        let options = FileOptions::default().compression_method(CompressionMethod::Stored);
        for (name, data) in entries {
            if name.ends_with('/') {
                writer.add_directory(*name, options).unwrap();
            } else {
                writer.start_file(*name, options).unwrap();
                writer.write_all(data).unwrap();
            }
        }
        writer.finish().unwrap();
    }

    /// Run the plugin directly and open the converted ZIP
    fn convert(source: &Path, out_dir: &Path) -> (CompressionResult, ZipArchive<File>) {
        let result = ImageZipToWebpZipPlugin::new()
            .process(source, out_dir)
            .unwrap();
        let archive = ZipArchive::new(File::open(&result.output_path).unwrap()).unwrap();
        (result, archive)
    }

    fn entry_names(archive: &mut ZipArchive<File>) -> Vec<String> {
        (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_string())
            .collect()
    }

    fn entry_bytes(archive: &mut ZipArchive<File>, name: &str) -> Vec<u8> {
        let mut data = Vec::new();
        archive
            .by_name(name)
            .unwrap()
            .read_to_end(&mut data)
            .unwrap();
        data
    }

    fn is_webp_data(data: &[u8]) -> bool {
        data.len() > 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP"
    }

    #[test]
    fn test_is_incidental_file() {
        assert!(ImageZipToWebpZipPlugin::is_incidental_file(
            "__MACOSX/._a.png"
        ));
        assert!(ImageZipToWebpZipPlugin::is_incidental_file(
            "ch01/__MACOSX/._a.png"
        ));
        assert!(ImageZipToWebpZipPlugin::is_incidental_file("ComicInfo.xml"));
        assert!(ImageZipToWebpZipPlugin::is_incidental_file(
            "ch01/Thumbs.db"
        ));
        assert!(ImageZipToWebpZipPlugin::is_incidental_file(".DS_Store"));
        assert!(ImageZipToWebpZipPlugin::is_incidental_file("desktop.ini"));
        assert!(!ImageZipToWebpZipPlugin::is_incidental_file("readme.txt"));
        assert!(!ImageZipToWebpZipPlugin::is_incidental_file("a.png"));
        // Only the exact file name counts, not a suffix of it
        assert!(!ImageZipToWebpZipPlugin::is_incidental_file("myThumbs.db"));
    }

    #[test]
    fn test_decode_entry_name() {
        let utf8 = "第一话/01.png";
        assert_eq!(
            ImageZipToWebpZipPlugin::decode_entry_name(utf8.as_bytes(), "mangled"),
            utf8
        );

        let (gbk, _, _) = encoding_rs::GBK.encode(utf8);
        assert_eq!(
            ImageZipToWebpZipPlugin::decode_entry_name(&gbk, "mangled"),
            utf8
        );

        // Neither UTF-8 nor GBK: keep the reader's (CP437) decoding
        assert_eq!(
            ImageZipToWebpZipPlugin::decode_entry_name(&[0xFF, 0xFF], "cp437"),
            "cp437"
        );

        assert_eq!(ImageZipToWebpZipPlugin::decode_entry_name(b"", ""), "");
    }

    #[test]
    fn test_webp_name_for() {
        let taken = |names: &[&str]| -> HashSet<String> {
            names.iter().map(|n| n.to_lowercase()).collect()
        };

        assert_eq!(
            ImageZipToWebpZipPlugin::webp_name_for("ch01/a.png", &taken(&["ch01/a.png"])),
            Some("ch01/a.webp".to_string())
        );
        // a.webp already exists: fall back to keeping the old extension
        assert_eq!(
            ImageZipToWebpZipPlugin::webp_name_for("a.jpg", &taken(&["a.jpg", "a.webp"])),
            Some("a.jpg.webp".to_string())
        );
        // Collisions are case-insensitive
        assert_eq!(
            ImageZipToWebpZipPlugin::webp_name_for("a.PNG", &taken(&["A.WEBP"])),
            Some("a.PNG.webp".to_string())
        );
        // Both candidates taken: no name, the image is kept as is
        assert_eq!(
            ImageZipToWebpZipPlugin::webp_name_for("a.png", &taken(&["a.webp", "a.png.webp"])),
            None
        );
    }

    #[test]
    fn test_can_handle_ignores_directory_entries() {
        let dir = tempfile::tempdir().unwrap();
        let plugin = ImageZipToWebpZipPlugin::new();
        let png = noise_png_bytes(32, 32);

        let zip = dir.path().join("chapters.zip");
        build_zip_with_dirs(
            &zip,
            &[
                ("ch01/", b""),
                ("ch01/a.png", &png),
                ("ch01/sub/", b""),
                ("ch01/sub/b.png", &png),
            ],
        );
        let (can_handle, reason) = plugin.can_handle(&zip).unwrap();
        assert!(can_handle, "reason: {:?}", reason);

        // A ZIP of nothing but directories has no images
        let dirs_only = dir.path().join("dirs.zip");
        build_zip_with_dirs(&dirs_only, &[("ch01/", b""), ("ch02/", b"")]);
        let (can_handle, _) = plugin.can_handle(&dirs_only).unwrap();
        assert!(!can_handle);
    }

    #[test]
    fn test_can_handle_ignores_incidental_files() {
        let dir = tempfile::tempdir().unwrap();
        let plugin = ImageZipToWebpZipPlugin::new();
        let png = noise_png_bytes(32, 32);

        let zip = dir.path().join("comic.zip");
        build_zip(
            &zip,
            &[
                ("a.png", &png),
                ("ComicInfo.xml", b"<ComicInfo/>"),
                ("Thumbs.db", b"thumbs"),
                ("__MACOSX/._a.png", b"resource fork"),
            ],
        );
        let (can_handle, reason) = plugin.can_handle(&zip).unwrap();
        assert!(can_handle, "reason: {:?}", reason);

        // Only clutter, no images
        let clutter = dir.path().join("clutter.zip");
        build_zip(&clutter, &[("Thumbs.db", b"thumbs"), (".DS_Store", b"x")]);
        let (can_handle, _) = plugin.can_handle(&clutter).unwrap();
        assert!(!can_handle);
    }

    #[test]
    fn test_can_handle_counts_webp_and_gif_as_images() {
        let dir = tempfile::tempdir().unwrap();
        let plugin = ImageZipToWebpZipPlugin::new();
        let png = noise_png_bytes(32, 32);

        // Partly converted archive: still all images, one left to convert
        let partial = dir.path().join("partial.zip");
        build_zip(
            &partial,
            &[("a.webp", b"webp"), ("b.gif", b"gif"), ("c.png", &png)],
        );
        let (can_handle, _) = plugin.can_handle(&partial).unwrap();
        assert!(can_handle);

        // Nothing left to convert
        let done = dir.path().join("done.zip");
        build_zip(&done, &[("a.webp", b"webp"), ("b.gif", b"gif")]);
        let (can_handle, _) = plugin.can_handle(&done).unwrap();
        assert!(!can_handle);

        // Empty archive
        let empty = dir.path().join("empty.zip");
        build_zip(&empty, &[]);
        let (can_handle, _) = plugin.can_handle(&empty).unwrap();
        assert!(!can_handle);
    }

    #[test]
    fn test_process_resolves_name_collisions() {
        let dir = tempfile::tempdir().unwrap();
        let png = noise_png_bytes(64, 64);
        let source = dir.path().join("photos.zip");
        // a.png and a.jpg both want a.webp; b.png must not shadow B.WEBP
        build_zip(
            &source,
            &[
                ("a.png", &png),
                ("a.jpg", &png),
                ("B.WEBP", b"existing webp"),
                ("b.png", &png),
            ],
        );

        let (result, mut archive) = convert(&source, &dir.path().join("out"));
        assert_eq!(result.files_processed, 3);

        let names = entry_names(&mut archive);
        assert_eq!(names, vec!["a.webp", "a.jpg.webp", "B.WEBP", "b.png.webp"]);
        assert!(is_webp_data(&entry_bytes(&mut archive, "a.webp")));
        assert!(is_webp_data(&entry_bytes(&mut archive, "a.jpg.webp")));
        assert!(is_webp_data(&entry_bytes(&mut archive, "b.png.webp")));
        assert_eq!(entry_bytes(&mut archive, "B.WEBP"), b"existing webp");
    }

    #[test]
    fn test_process_keeps_image_when_no_name_is_free() {
        let dir = tempfile::tempdir().unwrap();
        let png = noise_png_bytes(64, 64);
        let source = dir.path().join("photos.zip");
        build_zip(
            &source,
            &[("a.png", &png), ("a.webp", b"one"), ("a.png.webp", b"two")],
        );

        let (result, mut archive) = convert(&source, &dir.path().join("out"));
        assert_eq!(result.files_processed, 0);
        // Nothing converted: reported as no reduction on the untouched source
        assert_eq!(result.output_path, source);
        assert_eq!(result.compressed_size, result.original_size);
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].starts_with("a.png: no free name"));
        assert_eq!(
            entry_names(&mut archive),
            vec!["a.png", "a.webp", "a.png.webp"]
        );
        assert_eq!(entry_bytes(&mut archive, "a.png"), png);
    }

    #[test]
    fn test_process_keeps_image_whose_webp_is_not_smaller() {
        let dir = tempfile::tempdir().unwrap();
        let png = noise_png_bytes(64, 64);
        let jpeg = low_quality_noise_jpeg();
        let source = dir.path().join("photos.zip");
        build_zip(&source, &[("big.png", &png), ("small.jpg", &jpeg)]);

        let (result, mut archive) = convert(&source, &dir.path().join("out"));
        assert_eq!(result.files_processed, 1);
        assert_eq!(entry_names(&mut archive), vec!["big.webp", "small.jpg"]);
        assert_eq!(entry_bytes(&mut archive, "small.jpg"), jpeg);
    }

    #[test]
    fn test_process_keeps_undecodable_image() {
        let dir = tempfile::tempdir().unwrap();
        let png = noise_png_bytes(64, 64);
        let source = dir.path().join("photos.zip");
        build_zip(&source, &[("good.png", &png), ("broken.png", b"not a png")]);

        let (result, mut archive) = convert(&source, &dir.path().join("out"));
        assert_eq!(result.files_processed, 1);
        assert_eq!(result.warnings.len(), 1);
        assert!(
            result.warnings[0].starts_with("broken.png: Failed to decode image"),
            "{:?}",
            result.warnings
        );
        assert_eq!(entry_names(&mut archive), vec!["good.webp", "broken.png"]);
        assert_eq!(entry_bytes(&mut archive, "broken.png"), b"not a png");
    }

    #[test]
    fn test_process_preserves_directories_and_entry_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let png = noise_png_bytes(64, 64);
        let source = dir.path().join("chapters.zip");
        let mtime = zip::DateTime::from_date_and_time(2020, 5, 17, 10, 30, 0).unwrap();
        {
            let mut writer = ZipWriter::new(File::create(&source).unwrap());
            let stored = FileOptions::default()
                .compression_method(CompressionMethod::Stored)
                .last_modified_time(mtime);
            let deflated = FileOptions::default()
                .compression_method(CompressionMethod::Deflated)
                .last_modified_time(mtime);
            writer.add_directory("ch01/", stored).unwrap();
            writer.start_file("ch01/a.png", stored).unwrap();
            writer.write_all(&png).unwrap();
            writer.start_file("ComicInfo.xml", deflated).unwrap();
            writer.write_all(&b"<ComicInfo/>".repeat(50)).unwrap();
            writer.finish().unwrap();
        }

        let (_, mut archive) = convert(&source, &dir.path().join("out"));
        assert_eq!(
            entry_names(&mut archive),
            vec!["ch01/", "ch01/a.webp", "ComicInfo.xml"]
        );

        assert!(archive.by_name("ch01/").unwrap().is_dir());

        {
            let webp = archive.by_name("ch01/a.webp").unwrap();
            // Already-compressed WebP is stored, not deflated again
            assert_eq!(webp.compression(), CompressionMethod::Stored);
            assert_eq!(webp.last_modified().datepart(), mtime.datepart());
            assert_eq!(webp.last_modified().timepart(), mtime.timepart());
        }

        {
            // Other entries are copied with their compression and timestamp
            let info = archive.by_name("ComicInfo.xml").unwrap();
            assert_eq!(info.compression(), CompressionMethod::Deflated);
            assert_eq!(info.last_modified().datepart(), mtime.datepart());
            assert_eq!(info.last_modified().timepart(), mtime.timepart());
        }
        assert_eq!(
            entry_bytes(&mut archive, "ComicInfo.xml"),
            b"<ComicInfo/>".repeat(50)
        );
    }

    /// Build a ZIP whose entry names are stored as `raw_prefix` bytes without
    /// the UTF-8 flag, as legacy Windows archivers do. zip 0.6 always writes
    /// UTF-8 names, so the archive is written with an ASCII placeholder of
    /// the same length (flag unset) and the bytes are patched afterwards.
    fn build_zip_with_raw_names(path: &Path, raw_prefix: &[u8], png: &[u8]) {
        let placeholder = "X".repeat(raw_prefix.len());
        build_zip_with_dirs(
            path,
            &[
                (&format!("{}/", placeholder), b""),
                (&format!("{}/01.png", placeholder), png),
                (&format!("{}/readme.txt", placeholder), b"hi"),
            ],
        );
        let mut bytes = fs::read(path).unwrap();
        let needle = placeholder.as_bytes();
        let mut i = 0;
        let mut patched = 0;
        while i + needle.len() <= bytes.len() {
            if &bytes[i..i + needle.len()] == needle {
                bytes[i..i + needle.len()].copy_from_slice(raw_prefix);
                patched += 1;
                i += needle.len();
            } else {
                i += 1;
            }
        }
        // 3 entries, each named in the local header and the central directory
        assert_eq!(patched, 6);
        fs::write(path, bytes).unwrap();
    }

    #[test]
    fn test_process_decodes_legacy_entry_names() {
        let dir = tempfile::tempdir().unwrap();
        let png = noise_png_bytes(64, 64);
        let folder = "第一话";

        let (gbk, _, _) = encoding_rs::GBK.encode(folder);
        for (label, raw) in [("gbk", gbk.to_vec()), ("utf8", folder.as_bytes().to_vec())] {
            let source = dir.path().join(format!("{}.zip", label));
            build_zip_with_raw_names(&source, &raw, &png);

            let (result, mut archive) = convert(&source, &dir.path().join(label));
            assert_eq!(result.files_processed, 1, "{}", label);
            assert_eq!(
                entry_names(&mut archive),
                vec![
                    format!("{}/", folder),
                    format!("{}/01.webp", folder),
                    format!("{}/readme.txt", folder),
                ],
                "{}",
                label
            );
            assert_eq!(
                entry_bytes(&mut archive, &format!("{}/readme.txt", folder)),
                b"hi"
            );
        }
    }

    fn jpeg_bytes(img: &DynamicImage, quality: u8) -> Vec<u8> {
        let mut buf = Vec::new();
        img.write_to(
            &mut Cursor::new(&mut buf),
            image::ImageOutputFormat::Jpeg(quality),
        )
        .unwrap();
        buf
    }

    /// Smooth gradient JPEG: under 1 bpp, yet its WebP is far smaller, so
    /// keeping it can only be the bpp threshold's doing
    fn low_bpp_jpeg() -> Vec<u8> {
        let img: image::RgbImage = ImageBuffer::from_fn(256, 256, |x, y| {
            Rgb([x as u8, y as u8, ((x + y) / 2) as u8])
        });
        jpeg_bytes(&DynamicImage::ImageRgb8(img), 90)
    }

    /// Noise JPEG at high quality: well over the bpp threshold, and its WebP
    /// is smaller
    fn high_bpp_jpeg(width: u32, height: u32) -> Vec<u8> {
        let img = image::load_from_memory(&noise_png_bytes(width, height)).unwrap();
        jpeg_bytes(&img, 90)
    }

    /// Insert an EXIF APP1 segment carrying `orientation` right after SOI
    fn with_exif_orientation(jpeg: &[u8], orientation: u16) -> Vec<u8> {
        let mut tiff = Vec::new();
        tiff.extend_from_slice(b"MM\x00\x2a\x00\x00\x00\x08"); // big-endian, IFD0 at 8
        tiff.extend_from_slice(&1u16.to_be_bytes()); // one entry
        tiff.extend_from_slice(&0x0112u16.to_be_bytes()); // Orientation
        tiff.extend_from_slice(&3u16.to_be_bytes()); // SHORT
        tiff.extend_from_slice(&1u32.to_be_bytes()); // count
        tiff.extend_from_slice(&orientation.to_be_bytes());
        tiff.extend_from_slice(&[0, 0]); // value padding
        tiff.extend_from_slice(&0u32.to_be_bytes()); // no next IFD

        let mut payload = b"Exif\x00\x00".to_vec();
        payload.extend_from_slice(&tiff);

        let mut out = jpeg[..2].to_vec(); // SOI
        out.extend_from_slice(&[0xFF, 0xE1]);
        out.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
        out.extend_from_slice(&payload);
        out.extend_from_slice(&jpeg[2..]);
        out
    }

    /// Set the "encrypted" flag on every entry. zip 0.6 cannot write
    /// encrypted entries, but readers decide from this flag alone.
    fn mark_entries_encrypted(path: &Path) {
        let mut bytes = fs::read(path).unwrap();
        let mut patched = 0;
        for i in 0..bytes.len().saturating_sub(10) {
            // Local header: flags at +6; central directory header: flags at +8
            let flags_at = match &bytes[i..i + 4] {
                [0x50, 0x4b, 0x03, 0x04] => i + 6,
                [0x50, 0x4b, 0x01, 0x02] => i + 8,
                _ => continue,
            };
            bytes[flags_at] |= 1;
            patched += 1;
        }
        assert!(patched >= 2);
        fs::write(path, bytes).unwrap();
    }

    #[test]
    fn test_apply_orientation_all_values() {
        // 3x2 image where every pixel is unique: value = 10 * y + x
        let img = DynamicImage::ImageLuma8(ImageBuffer::from_fn(3, 2, |x, y| {
            image::Luma([(10 * y + x) as u8])
        }));
        // (orientation, expected dimensions, expected top-left pixel)
        let cases = [
            (1, (3, 2), 0),  // as is
            (2, (3, 2), 2),  // mirrored: top-right comes first
            (3, (3, 2), 12), // rotated 180: bottom-right comes first
            (4, (3, 2), 10), // flipped: bottom-left comes first
            (5, (2, 3), 0),  // transposed
            (6, (2, 3), 10), // rotated 90 CW: bottom-left comes first
            (7, (2, 3), 12), // transversed
            (8, (2, 3), 2),  // rotated 270 CW: top-right comes first
            (0, (3, 2), 0),  // invalid values leave the image alone
            (9, (3, 2), 0),
        ];
        for (orientation, (w, h), top_left) in cases {
            let out =
                ImageZipToWebpZipPlugin::apply_orientation(img.clone(), orientation).to_luma8();
            assert_eq!(out.dimensions(), (w, h), "orientation {}", orientation);
            assert_eq!(
                out.get_pixel(0, 0)[0],
                top_left,
                "orientation {}",
                orientation
            );
        }
    }

    #[test]
    fn test_jpeg_orientation_reads_exif() {
        let jpeg = high_bpp_jpeg(16, 8);
        assert_eq!(ImageZipToWebpZipPlugin::jpeg_orientation(&jpeg), 1);
        assert_eq!(
            ImageZipToWebpZipPlugin::jpeg_orientation(&with_exif_orientation(&jpeg, 6)),
            6
        );
        // Not a JPEG at all
        assert_eq!(ImageZipToWebpZipPlugin::jpeg_orientation(b"garbage"), 1);
    }

    #[test]
    fn test_process_applies_jpeg_exif_orientation() {
        let dir = tempfile::tempdir().unwrap();
        // Stored 64x32 but tagged "rotate 90° CW": it displays as 32x64
        let rotated = with_exif_orientation(&high_bpp_jpeg(64, 32), 6);
        let source = dir.path().join("photos.zip");
        build_zip(&source, &[("portrait.jpg", &rotated)]);

        let (result, mut archive) = convert(&source, &dir.path().join("out"));
        assert_eq!(result.files_processed, 1);
        let webp = entry_bytes(&mut archive, "portrait.webp");
        let decoded = image::load_from_memory(&webp).unwrap();
        assert_eq!(
            (decoded.width(), decoded.height()),
            (32, 64),
            "the WebP carries no EXIF, so the rotation must be in the pixels"
        );
    }

    #[test]
    fn test_jpeg_bpp_threshold() {
        let dir = tempfile::tempdir().unwrap();
        let low = low_bpp_jpeg();
        let high = high_bpp_jpeg(64, 64);
        let source = dir.path().join("photos.zip");
        build_zip(&source, &[("low.jpg", &low), ("high.jpg", &high)]);

        // Default threshold: the well-compressed JPEG is kept untouched
        let (result, mut archive) = convert(&source, &dir.path().join("default"));
        assert_eq!(result.files_processed, 1);
        assert!(result.warnings.is_empty(), "{:?}", result.warnings);
        assert_eq!(entry_names(&mut archive), vec!["low.jpg", "high.webp"]);
        assert_eq!(entry_bytes(&mut archive, "low.jpg"), low);

        // Threshold 0: every JPEG is converted, so the threshold alone kept it
        let out_dir = dir.path().join("all");
        let result = ImageZipToWebpZipPlugin::new()
            .with_min_jpeg_bpp(0.0)
            .process(&source, &out_dir)
            .unwrap();
        assert_eq!(result.files_processed, 2);
        let mut archive = ZipArchive::new(File::open(&result.output_path).unwrap()).unwrap();
        assert_eq!(entry_names(&mut archive), vec!["low.webp", "high.webp"]);
    }

    #[test]
    fn test_jpeg_worth_converting_edges() {
        let plugin = ImageZipToWebpZipPlugin::new();
        assert!(!plugin.jpeg_worth_converting(&low_bpp_jpeg()));
        assert!(plugin.jpeg_worth_converting(&high_bpp_jpeg(64, 64)));
        // Unreadable header: let the full decode report the real error
        assert!(plugin.jpeg_worth_converting(b"not a jpeg"));
        assert!(plugin.jpeg_worth_converting(b""));
        // The threshold is inclusive and cannot go negative
        let plugin = ImageZipToWebpZipPlugin::new().with_min_jpeg_bpp(-5.0);
        assert!(plugin.jpeg_worth_converting(&low_bpp_jpeg()));
    }

    #[test]
    fn test_process_skips_archive_with_nothing_worth_converting() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("photos.zip");
        build_zip(&source, &[("low.jpg", &low_bpp_jpeg())]);
        let original_bytes = fs::read(&source).unwrap();

        let mut manager = PluginManager::new();
        manager.register(Box::new(ImageZipToWebpZipPlugin::new()));
        let outcome = manager
            .process_file(&source, dir.path(), None, true)
            .unwrap();
        assert!(
            matches!(outcome, CompressionOutcome::Skipped { .. }),
            "{:?}",
            outcome
        );
        assert_eq!(fs::read(&source).unwrap(), original_bytes);
        // No backup and no leftover output next to the source
        let mut files: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        files.sort();
        assert_eq!(files, vec!["photos.zip"]);
    }

    #[test]
    fn test_process_fails_when_no_image_can_be_decoded() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("photos.zip");
        build_zip(&source, &[("a.png", b"not a png"), ("b.png", b"nor this")]);

        let out_dir = dir.path().join("out");
        let err = ImageZipToWebpZipPlugin::new()
            .process(&source, &out_dir)
            .unwrap_err();
        assert_eq!(
            crate::compress_plugins::classify_error(&err),
            CompressionErrorKind::CorruptFile
        );
        let message = format!("{:#}", err);
        assert!(
            message.contains("a.png") && message.contains("b.png"),
            "{}",
            message
        );
        // The partial output is cleaned up
        assert_eq!(fs::read_dir(&out_dir).unwrap().count(), 0);
    }

    #[test]
    fn test_partial_conversion_reports_warnings_through_manager() {
        let dir = tempfile::tempdir().unwrap();
        let png = noise_png_bytes(64, 64);
        let source = dir.path().join("photos.zip");
        build_zip(&source, &[("good.png", &png), ("broken.png", b"not a png")]);

        let mut manager = PluginManager::new();
        manager.register(Box::new(ImageZipToWebpZipPlugin::new()));
        match manager
            .process_file(&source, dir.path(), None, true)
            .unwrap()
        {
            CompressionOutcome::Compressed(result) => {
                assert_eq!(result.files_processed, 1);
                assert_eq!(result.warnings.len(), 1);
                assert!(result.warnings[0].contains("broken.png"));
            }
            other => panic!("expected Compressed, got {:?}", other),
        }
    }

    #[test]
    fn test_process_keeps_image_over_entry_size_cap() {
        let dir = tempfile::tempdir().unwrap();
        let small = noise_png_bytes(16, 16);
        let big = noise_png_bytes(64, 64);
        let source = dir.path().join("photos.zip");
        build_zip(&source, &[("small.png", &small), ("big.png", &big)]);

        // Cap exactly at the small image: at the limit is allowed, above is not
        let result = ImageZipToWebpZipPlugin::new()
            .with_max_image_entry_bytes(small.len() as u64)
            .process(&source, &dir.path().join("out"))
            .unwrap();
        assert_eq!(result.files_processed, 1);
        assert_eq!(result.warnings.len(), 1);
        assert!(
            result.warnings[0].starts_with("big.png: image is too large"),
            "{:?}",
            result.warnings
        );
        let mut archive = ZipArchive::new(File::open(&result.output_path).unwrap()).unwrap();
        assert_eq!(entry_names(&mut archive), vec!["small.webp", "big.png"]);
        assert_eq!(entry_bytes(&mut archive, "big.png"), big);
    }

    #[test]
    fn test_process_keeps_image_beyond_webp_dimensions() {
        let dir = tempfile::tempdir().unwrap();
        let png = noise_png_bytes(64, 64);
        // One pixel wider than WebP can encode; solid, so the PNG stays tiny
        let wide: image::GrayImage = ImageBuffer::new(WEBP_MAX_DIMENSION + 1, 1);
        let mut wide_png = Vec::new();
        wide.write_to(
            &mut Cursor::new(&mut wide_png),
            image::ImageOutputFormat::Png,
        )
        .unwrap();
        let source = dir.path().join("photos.zip");
        build_zip(&source, &[("good.png", &png), ("wide.png", &wide_png)]);

        let (result, mut archive) = convert(&source, &dir.path().join("out"));
        assert_eq!(result.files_processed, 1);
        assert_eq!(result.warnings.len(), 1);
        assert!(
            result.warnings[0].starts_with("wide.png:"),
            "{:?}",
            result.warnings
        );
        assert_eq!(entry_names(&mut archive), vec!["good.webp", "wide.png"]);
        assert_eq!(entry_bytes(&mut archive, "wide.png"), wide_png);
    }

    #[test]
    fn test_process_keeps_entry_order_across_batches() {
        let dir = tempfile::tempdir().unwrap();
        let png = noise_png_bytes(32, 32);
        let count = BATCH_MAX_IMAGES * 2 + 3;
        // Interleave non-image entries so batch boundaries fall around them
        let mut owned: Vec<(String, &[u8])> = Vec::new();
        for i in 0..count {
            owned.push((format!("p{:03}.png", i), &png));
            if i % 50 == 0 {
                owned.push((format!("n{:03}.txt", i), b"n"));
            }
        }
        let entries: Vec<(&str, &[u8])> = owned.iter().map(|(n, d)| (n.as_str(), *d)).collect();
        let source = dir.path().join("many.zip");
        build_zip(&source, &entries);

        let (result, mut archive) = convert(&source, &dir.path().join("out"));
        assert_eq!(result.files_processed, count);
        let expected: Vec<String> = entries
            .iter()
            .map(|(name, _)| name.replace(".png", ".webp"))
            .collect();
        assert_eq!(entry_names(&mut archive), expected);
    }

    #[test]
    fn test_cbz_archives() {
        let dir = tempfile::tempdir().unwrap();
        let plugin = ImageZipToWebpZipPlugin::new();
        assert_eq!(plugin.supported_extensions(), vec!["zip", "cbz"]);

        let png = noise_png_bytes(64, 64);
        let source = dir.path().join("Volume 1.CBZ");
        build_zip(&source, &[("001.png", &png), ("002.png", &png)]);
        let (can_handle, reason) = plugin.can_handle(&source).unwrap();
        assert!(can_handle, "{:?}", reason);

        let mut manager = PluginManager::new();
        manager.register(Box::new(ImageZipToWebpZipPlugin::new()));
        match manager
            .process_file(&source, dir.path(), None, true)
            .unwrap()
        {
            CompressionOutcome::Compressed(result) => {
                // Replaced in place, keeping the .CBZ name
                assert_eq!(result.output_path, source);
                assert_eq!(
                    result.backup_path,
                    Some(dir.path().join("Volume 1.CBZ.bak"))
                );
            }
            other => panic!("expected Compressed, got {:?}", other),
        }
        let mut archive = ZipArchive::new(File::open(&source).unwrap()).unwrap();
        assert_eq!(entry_names(&mut archive), vec!["001.webp", "002.webp"]);

        // Other extensions are still refused
        let rar = dir.path().join("comic.cbr");
        fs::write(&rar, b"Rar!").unwrap();
        let (can_handle, _) = plugin.can_handle(&rar).unwrap();
        assert!(!can_handle);
    }

    #[test]
    fn test_can_handle_rejects_encrypted_archives() {
        let dir = tempfile::tempdir().unwrap();
        let png = noise_png_bytes(32, 32);
        let source = dir.path().join("secret.zip");
        build_zip(&source, &[("a.png", &png)]);
        mark_entries_encrypted(&source);

        let (can_handle, reason) = ImageZipToWebpZipPlugin::new().can_handle(&source).unwrap();
        assert!(!can_handle);
        assert_eq!(
            reason.as_deref(),
            Some("ZIP archive contains encrypted entries")
        );
    }

    #[test]
    fn test_can_handle_reason_names_image_share() {
        let dir = tempfile::tempdir().unwrap();
        let png = noise_png_bytes(32, 32);
        let source = dir.path().join("mixed.zip");
        build_zip(&source, &[("a.png", &png), ("readme.txt", b"hi")]);
        let (can_handle, reason) = ImageZipToWebpZipPlugin::new().can_handle(&source).unwrap();
        assert!(!can_handle);
        assert_eq!(
            reason.as_deref(),
            Some("ZIP file is not an image archive (1 of 2 files are images)")
        );
        // A lower ratio accepts it
        let (can_handle, _) = ImageZipToWebpZipPlugin::new()
            .with_min_image_ratio(0.5)
            .can_handle(&source)
            .unwrap();
        assert!(can_handle);
    }
}
