use anyhow::{Context, Result};
use image::DynamicImage;
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tracing::warn;
use zip::{write::FileOptions, CompressionMethod, ZipArchive, ZipWriter};

use crate::compress_plugins::{
    create_output_file, get_file_size, has_extension, CompressionError, CompressionErrorKind,
    CompressionPlugin, CompressionResult, PluginMetadata,
};

/// Plugin for converting ZIP files containing images to WebP format
/// Reads ZIP, converts all images to WebP, and creates a new ZIP
pub struct ImageZipToWebpZipPlugin {
    quality: f32,
    min_image_ratio: f32, // Minimum ratio of images to total files to process
}

impl ImageZipToWebpZipPlugin {
    pub fn new() -> Self {
        Self {
            quality: 85.0,
            min_image_ratio: 1.0, // At least 100% of files should be images
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

    /// Images this plugin converts to WebP
    fn is_image_file(filename: &str) -> bool {
        let lower = filename.to_lowercase();
        lower.ends_with(".png")
            || lower.ends_with(".jpg")
            || lower.ends_with(".jpeg")
            || lower.ends_with(".bmp")
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

    fn has_convertible_images(&self, path: &Path) -> Result<bool> {
        let file = File::open(path)?;
        let mut archive = ZipArchive::new(file).map_err(|e| {
            anyhow::Error::new(CompressionError::new(
                CompressionErrorKind::CorruptFile,
                format!("Failed to read ZIP archive {}: {}", path.display(), e),
            ))
        })?;

        let mut counted_files = 0;
        let mut image_count = 0;
        let mut convertible_count = 0;

        for i in 0..archive.len() {
            let file = archive.by_index_raw(i)?;
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

        if counted_files == 0 {
            return Ok(false);
        }

        // Only process if:
        // 1. There are images that are not WebP yet
        // 2. Images make up at least min_image_ratio of the counted files
        let image_ratio = image_count as f32 / counted_files as f32;
        Ok(convertible_count > 0 && image_ratio >= self.min_image_ratio)
    }

    fn convert_image_to_webp(&self, data: &[u8], original_name: &str) -> Result<Vec<u8>> {
        // Load image from bytes
        let img = image::load_from_memory(data)
            .with_context(|| format!("Failed to decode image: {}", original_name))?;

        // Encode as WebP
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

    /// Write a copy of `source` to `output` with its images converted to
    /// WebP. Returns the number of images converted. An image is kept as is
    /// when it fails to decode, its WebP is not smaller, or no free name is
    /// left for the WebP entry. Every other entry is copied byte for byte
    /// (compressed data, timestamp, permissions) under its decoded name.
    fn process_zip(&self, source: &Path, output: &Path) -> Result<usize> {
        let input_file = File::open(source)?;
        let mut input_archive = ZipArchive::new(input_file).map_err(|e| {
            anyhow::Error::new(CompressionError::new(
                CompressionErrorKind::CorruptFile,
                format!("Failed to read ZIP archive {}: {}", source.display(), e),
            ))
        })?;

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

        for (i, name) in names.iter().enumerate() {
            let mut file = input_archive.by_index(i)?;

            if file.is_dir() {
                let options = FileOptions::default().last_modified_time(file.last_modified());
                output_archive.add_directory(name.as_str(), options)?;
                continue;
            }

            if Self::is_image_file(name) {
                let last_modified = file.last_modified();
                let mut contents = Vec::new();
                file.read_to_end(&mut contents)?;
                drop(file); // Release the borrow

                match self.convert_image_to_webp(&contents, name) {
                    Ok(webp_data) if webp_data.len() < contents.len() => {
                        if let Some(new_name) = Self::webp_name_for(name, &taken) {
                            taken.insert(new_name.to_lowercase());
                            // WebP is already compressed; deflating it again
                            // only costs CPU
                            let options = FileOptions::default()
                                .compression_method(CompressionMethod::Stored)
                                .last_modified_time(last_modified);
                            output_archive.start_file(new_name, options)?;
                            output_archive.write_all(&webp_data)?;
                            files_processed += 1;
                            continue;
                        }
                        warn!(entry = %name, "No free name for the WebP entry; keeping original");
                    }
                    // WebP is not smaller (e.g. an already well-compressed JPEG)
                    Ok(_) => {}
                    Err(e) => {
                        warn!(entry = %name, error = %e, "Failed to convert image; keeping original");
                    }
                }

                // Reopen the entry to copy the original bytes
                file = input_archive.by_index(i)?;
            }

            output_archive.raw_copy_file_rename(file, name.as_str())?;
        }

        output_archive.finish()?;

        Ok(files_processed)
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
            description: "Converts images inside ZIP archives to WebP format".to_string(),
            version: "1.0.0".to_string(),
        }
    }

    fn can_handle(&self, path: &Path) -> Result<(bool, Option<String>)> {
        if !path.is_file() {
            return Ok((false, Some("Not a file".to_string())));
        }

        if !has_extension(path, &["zip"]) {
            return Ok((false, Some("Not a ZIP file".to_string())));
        }

        let has_images = self.has_convertible_images(path)?;
        if has_images {
            Ok((
                true,
                Some("ZIP file contains convertible images".to_string()),
            ))
        } else {
            Ok((
                false,
                Some("ZIP file contains no convertible images".to_string()),
            ))
        }
    }

    fn estimate_ratio(&self, path: &Path) -> Result<Option<f32>> {
        // Try to estimate based on the types of images in the ZIP
        let file = File::open(path)?;
        let mut archive = ZipArchive::new(file)?;

        let mut total_size = 0u64;
        let mut image_size = 0u64;

        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            let size = file.size();
            total_size += size;

            if Self::is_image_file(file.name()) && !Self::is_webp(file.name()) {
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

        // Generate output filename
        let output_filename = if let Some(stem) = source.file_stem() {
            PathBuf::from(format!("{}_webp.zip", stem.to_string_lossy()))
        } else {
            PathBuf::from("converted_webp.zip")
        };

        let output_path = output_dir.join(&output_filename);

        // Ensure output directory exists
        fs::create_dir_all(output_dir)?;

        // Process the ZIP file (the output is created with create_new, so an
        // existing file fails the operation); the manager backs up the
        // original and moves the output over the source path (replace_source)
        let files_processed = self
            .process_zip(source, &output_path)
            .with_context(|| format!("Failed to process ZIP file: {}", source.display()))?;

        let compressed_size = get_file_size(&output_path)?;

        Ok(CompressionResult {
            original_size,
            compressed_size,
            output_path,
            plugin_name: self.metadata().name,
            files_processed,
            backup_path: None,
            replace_source: true,
        })
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec!["zip"]
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
}
