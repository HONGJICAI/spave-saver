use anyhow::{Context, Result};
use image::{DynamicImage, GenericImageView};
use std::fs;
use std::path::Path;
use tracing::{debug, error, info, warn};

use crate::compress_plugins::{
    generate_output_filename, get_file_size, has_extension, CompressionPlugin, CompressionResult,
    PluginMetadata,
};

/// Plugin for converting images to WebP format
pub struct WebPConverterPlugin {
    quality: f32,
}

impl WebPConverterPlugin {
    pub fn new() -> Self {
        Self { quality: 85.0 }
    }

    pub fn with_quality(mut self, quality: f32) -> Self {
        self.quality = quality.clamp(0.0, 100.0);
        self
    }

    fn is_webp(path: &Path) -> bool {
        has_extension(path, &["webp"])
    }

    fn is_supported_image(path: &Path) -> bool {
        has_extension(path, &["png", "jpg", "jpeg", "bmp", "tiff", "tif"])
    }

    /// Calculate bits per pixel (BPP) for an image file
    /// Returns the BPP value, or None if it cannot be calculated
    fn calculate_bpp(path: &Path) -> Option<f64> {
        // Get file size in bytes
        let file_size = match fs::metadata(path) {
            Ok(metadata) => metadata.len(),
            Err(e) => {
                debug!(
                    path = %path.display(),
                    error = %e,
                    "Failed to get file metadata for BPP calculation"
                );
                return None;
            }
        };

        // Get image dimensions without decoding the entire image (much faster!)
        let dimensions = match imagesize::size(path) {
            Ok(size) => size,
            Err(e) => {
                debug!(
                    path = %path.display(),
                    error = %e,
                    "Failed to read image dimensions for BPP calculation"
                );
                return None;
            }
        };

        let (width, height) = (dimensions.width as u64, dimensions.height as u64);
        let total_pixels = width * height;

        if total_pixels == 0 {
            debug!(
                path = %path.display(),
                "Image has zero pixels"
            );
            return None;
        }

        // Calculate BPP: (file_size_in_bytes * 8) / total_pixels
        let bpp = (file_size as f64 * 8.0) / total_pixels as f64;

        debug!(
            path = %path.display(),
            file_size = file_size,
            width = width,
            height = height,
            bpp = format!("{:.2}", bpp),
            "Calculated BPP for image"
        );

        Some(bpp)
    }

    /// Check if an image file has high BPP (bits per pixel)
    /// Returns true if BPP is above threshold (indicating potential for compression)
    fn has_high_bpp(path: &Path, threshold: f64) -> bool {
        match Self::calculate_bpp(path) {
            Some(bpp) => {
                let has_high = bpp > threshold;
                debug!(
                    path = %path.display(),
                    bpp = format!("{:.2}", bpp),
                    threshold = threshold,
                    has_high_bpp = has_high,
                    "Checked if image has high BPP"
                );
                has_high
            }
            None => false,
        }
    }

    fn convert_to_webp(&self, source: &Path, output: &Path) -> Result<()> {
        // Load the image
        let img = match image::open(source) {
            Ok(img) => img,
            Err(e) => {
                error!(
                    source = %source.display(),
                    error = %e,
                    "Failed to open image for WebP conversion"
                );
                return Err(
                    anyhow::anyhow!("Failed to open image: {}", source.display()).context(e),
                );
            }
        };

        // Create output directory if it doesn't exist
        if let Some(parent) = output.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                error!(
                    parent_dir = %parent.display(),
                    error = %e,
                    "Failed to create output directory for WebP conversion"
                );
                return Err(e.into());
            }
        }

        // Check if output file already exists
        if output.exists() {
            warn!(
                output = %output.display(),
                "Output file already exists, skipping WebP conversion"
            );
            return Err(anyhow::anyhow!(
                "Output file already exists: {}",
                output.display()
            ));
        }

        match self.encode_webp(&img, output) {
            Ok(_) => Ok(()),
            Err(e) => {
                error!(
                    source = %source.display(),
                    output = %output.display(),
                    quality = self.quality,
                    error = %e,
                    "Failed to encode image to WebP format"
                );
                Err(
                    anyhow::anyhow!("Failed to encode image to WebP: {}", source.display())
                        .context(e),
                )
            }
        }
    }

    fn encode_webp(&self, img: &DynamicImage, output: &Path) -> Result<()> {
        // Without webp feature, use external webp crate
        use webp::Encoder;

        let (width, height) = img.dimensions();
        let rgba = img.to_rgba8();

        let encoder = Encoder::from_rgba(&rgba, width, height);
        let encoded = encoder.encode(self.quality);

        std::fs::write(output, &*encoded).with_context(|| {
            error!(
                output = %output.display(),
                width = width,
                height = height,
                quality = self.quality,
                "Failed to write WebP encoded data to file"
            );
            format!("Failed to write WebP file: {}", output.display())
        })?;

        Ok(())
    }
}

impl Default for WebPConverterPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl CompressionPlugin for WebPConverterPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "WebP Converter".to_string(),
            description: "Converts PNG, JPEG, and other image formats to WebP".to_string(),
            version: "1.0.0".to_string(),
        }
    }

    fn can_handle(&self, path: &Path) -> Result<(bool, Option<String>)> {
        // Only handle non-WebP images
        if !path.is_file() {
            return Ok((false, Some("Not a file".to_string())));
        }

        if !Self::is_supported_image(path) {
            return Ok((false, Some("File extension not supported".to_string())));
        }

        if Self::is_webp(path) {
            return Ok((false, Some("Already a WebP file".to_string())));
        }

        // For JPEG files, only process if they have high BPP (bits per pixel)
        // This indicates the file is not heavily compressed and can benefit from WebP conversion
        if has_extension(path, &["jpg", "jpeg"]) {
            const BPP_THRESHOLD: f64 = 0.5;
            let has_high = Self::has_high_bpp(path, BPP_THRESHOLD);
            if !has_high {
                debug!(
                    path = %path.display(),
                    threshold = BPP_THRESHOLD,
                    "Skipping JPEG file: BPP too low (already well compressed)"
                );
                return Ok((
                    false,
                    Some(format!("JPEG BPP below threshold ({})", BPP_THRESHOLD)),
                ));
            }
            return Ok((
                true,
                Some(format!("JPEG with high BPP (above {})", BPP_THRESHOLD)),
            ));
        }

        // Process all other supported image formats
        Ok((true, None))
    }

    fn estimate_ratio(&self, path: &Path) -> Result<Option<f32>> {
        // WebP typically achieves 25-35% better compression than JPEG
        // and 26% better than PNG on average
        if has_extension(path, &["png"]) {
            Ok(Some(0.26))
        } else if has_extension(path, &["jpg", "jpeg"]) {
            Ok(Some(0.30))
        } else {
            Ok(Some(0.25))
        }
    }

    fn process(&self, source: &Path, output_dir: &Path) -> Result<CompressionResult> {
        let original_size = get_file_size(source)?;

        // Generate output filename
        let output_filename = generate_output_filename(source, "webp");
        let output_path = output_dir.join(&output_filename);

        // Convert to WebP; the manager handles size comparison and backups
        self.convert_to_webp(source, &output_path)
            .with_context(|| format!("Failed to convert {} to WebP", source.display()))?;

        let compressed_size = get_file_size(&output_path)?;

        info!(
            source = %source.display(),
            original_size = original_size,
            webp_size = compressed_size,
            "Converted image to WebP"
        );

        Ok(CompressionResult {
            original_size,
            compressed_size,
            output_path,
            plugin_name: self.metadata().name,
            files_processed: 1,
            backup_path: None,
            replace_source: false,
        })
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec!["png", "jpg", "jpeg", "bmp", "tiff", "tif"]
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
    use image::{ImageBuffer, Rgb, RgbImage};
    use std::path::PathBuf;

    /// Deterministic pseudo-random noise image. PNG stores noise poorly,
    /// so a lossy WebP of the same image is reliably much smaller.
    fn noise_image(width: u32, height: u32) -> RgbImage {
        let mut seed = 0x2545F491u32;
        ImageBuffer::from_fn(width, height, |_, _| {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            Rgb([
                (seed & 0xFF) as u8,
                ((seed >> 8) & 0xFF) as u8,
                ((seed >> 16) & 0xFF) as u8,
            ])
        })
    }

    fn save_noise_png(dir: &Path, name: &str, width: u32, height: u32) -> PathBuf {
        let path = dir.join(name);
        noise_image(width, height).save(&path).unwrap();
        path
    }

    fn save_jpeg(img: &RgbImage, path: &Path, quality: u8) {
        let mut file = fs::File::create(path).unwrap();
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut file, quality);
        encoder.encode_image(img).unwrap();
    }

    #[test]
    fn test_can_handle_png() {
        let plugin = WebPConverterPlugin::new();
        let path = Path::new("test.png");

        // Note: This will return false because the file doesn't exist
        // In a real test, you'd create a temporary PNG file
        let result = plugin.can_handle(path);
        assert!(result.is_ok());
        let (can_handle, reason) = result.unwrap();
        assert!(!can_handle); // False because file doesn't exist
        assert!(reason.is_some());
    }

    #[test]
    fn test_can_handle_real_png() {
        let dir = tempfile::tempdir().unwrap();
        let source = save_noise_png(dir.path(), "noise.png", 32, 32);

        let plugin = WebPConverterPlugin::new();
        let (can_handle, _) = plugin.can_handle(&source).unwrap();
        assert!(can_handle);
    }

    #[test]
    fn test_cannot_handle_webp() {
        let plugin = WebPConverterPlugin::new();
        let path = Path::new("test.webp");

        // WebP files should not be handled
        let result = plugin.can_handle(path);
        assert!(result.is_ok());
        let (can_handle, reason) = result.unwrap();
        assert!(!can_handle);
        assert_eq!(reason, Some("Not a file".to_string()));
    }

    #[test]
    fn test_jpeg_bpp_gate() {
        let dir = tempfile::tempdir().unwrap();
        let plugin = WebPConverterPlugin::new();

        // High-quality noise JPEG has high bits-per-pixel: worth converting
        let high_bpp = dir.path().join("noise.jpg");
        save_jpeg(&noise_image(100, 100), &high_bpp, 100);
        let (can_handle, _) = plugin.can_handle(&high_bpp).unwrap();
        assert!(can_handle, "high-BPP JPEG should be convertible");

        // Heavily compressed solid-color JPEG has low BPP: already optimal
        let solid = ImageBuffer::from_pixel(200, 200, Rgb([120u8, 130, 140]));
        let low_bpp = dir.path().join("solid.jpg");
        save_jpeg(&solid, &low_bpp, 10);
        let (can_handle, reason) = plugin.can_handle(&low_bpp).unwrap();
        assert!(!can_handle, "low-BPP JPEG should be skipped at scan time");
        assert!(reason.unwrap().contains("BPP"));
    }

    #[test]
    fn test_process_converts_to_smaller_webp_and_keeps_source() {
        let dir = tempfile::tempdir().unwrap();
        let source = save_noise_png(dir.path(), "noise.png", 128, 128);

        let plugin = WebPConverterPlugin::new();
        let result = plugin.process(&source, dir.path()).unwrap();

        // The plugin itself must never touch the source; that's the manager's job
        assert!(source.exists(), "plugin must not delete or rename the source");
        assert!(result.output_path.exists());
        assert_eq!(result.output_path, dir.path().join("noise.webp"));
        assert!(
            result.compressed_size < result.original_size,
            "lossy WebP of noise must be smaller than PNG ({} vs {})",
            result.compressed_size,
            result.original_size
        );
        assert!(!result.replace_source);
    }

    #[test]
    fn test_end_to_end_manager_creates_backup() {
        let dir = tempfile::tempdir().unwrap();
        let source = save_noise_png(dir.path(), "photo.png", 128, 128);
        let original_bytes = fs::read(&source).unwrap();

        let mut manager = PluginManager::new();
        manager.register(Box::new(WebPConverterPlugin::new()));

        let outcome = manager.process_file(&source, dir.path(), None, true).unwrap();
        match outcome {
            CompressionOutcome::Compressed(result) => {
                assert!(!source.exists(), "original renamed to backup");
                let backup = result.backup_path.unwrap();
                assert_eq!(backup, dir.path().join("photo.png.bak"));
                assert_eq!(fs::read(&backup).unwrap(), original_bytes);
                assert!(dir.path().join("photo.webp").exists());
            }
            other => panic!("expected Compressed, got {:?}", other),
        }
    }

    #[test]
    fn test_supported_extensions() {
        let plugin = WebPConverterPlugin::new();
        let extensions = plugin.supported_extensions();

        assert!(extensions.contains(&"png"));
        assert!(extensions.contains(&"jpg"));
        assert!(extensions.contains(&"jpeg"));
    }
}
