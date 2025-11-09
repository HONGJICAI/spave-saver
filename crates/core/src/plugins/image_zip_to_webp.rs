use anyhow::{anyhow, Context, Result};
use image::{DynamicImage, ImageFormat};
use tokio::fs::rename;
use std::fs::{self, File};
use std::io::{BufWriter, Cursor, Read, Write};
use std::path::{Path, PathBuf};
use zip::{write::FileOptions, CompressionMethod, ZipArchive, ZipWriter};

use crate::compress_plugins::{
    generate_output_filename, get_file_size, has_extension, CompressionPlugin, CompressionResult,
    PluginMetadata,
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

    fn is_image_file(filename: &str) -> bool {
        let lower = filename.to_lowercase();
        lower.ends_with(".png")
            || lower.ends_with(".jpg")
            || lower.ends_with(".jpeg")
            || lower.ends_with(".bmp")
            || lower.ends_with(".tiff")
            || lower.ends_with(".tif")
    }

    fn is_webp(filename: &str) -> bool {
        filename.to_lowercase().ends_with(".webp")
    }

    fn has_convertible_images(&self, path: &Path) -> Result<bool> {
        let file = File::open(path)?;
        let mut archive = ZipArchive::new(file)?;

        let total_files = archive.len();
        if total_files == 0 {
            return Ok(false);
        }

        let mut image_count = 0;
        let mut webp_count = 0;

        for i in 0..total_files {
            let file = archive.by_index(i)?;
            let name = file.name();

            if Self::is_image_file(name) {
                image_count += 1;
                if Self::is_webp(name) {
                    webp_count += 1;
                }
            }
        }

        // Only process if:
        // 1. There are images in the ZIP
        // 2. Not all images are already WebP
        // 3. Images make up at least min_image_ratio of all files
        let image_ratio = image_count as f32 / total_files as f32;
        Ok(image_count > 0 && webp_count < image_count && image_ratio >= self.min_image_ratio)
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

    fn process_zip(&self, source: &Path, output: &Path) -> Result<(usize, u64, u64)> {
        let input_file = File::open(source)?;
        let mut input_archive = ZipArchive::new(input_file)?;

        let output_file = File::create(output)?;
        let mut output_archive = ZipWriter::new(output_file);

        let options = FileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .compression_level(Some(6));

        let mut files_processed = 0;
        let mut original_total = 0u64;
        let mut compressed_total = 0u64;

        for i in 0..input_archive.len() {
            let mut file = input_archive.by_index(i)?;
            let name = file.name().to_string();
            let original_size = file.size();

            // Read file contents
            let mut contents = Vec::new();
            file.read_to_end(&mut contents)?;
            drop(file); // Release the borrow

            original_total += original_size;

            if Self::is_image_file(&name) && !Self::is_webp(&name) {
                // Convert image to WebP
                match self.convert_image_to_webp(&contents, &name) {
                    Ok(webp_data) => {
                        // Change extension to .webp
                        let new_name = if let Some(idx) = name.rfind('.') {
                            format!("{}.webp", &name[..idx])
                        } else {
                            format!("{}.webp", name)
                        };

                        output_archive.start_file(new_name, options)?;
                        output_archive.write_all(&webp_data)?;

                        compressed_total += webp_data.len() as u64;
                        files_processed += 1;
                    }
                    Err(e) => {
                        // If conversion fails, copy original file
                        eprintln!(
                            "Warning: Failed to convert {}: {}. Copying original.",
                            name, e
                        );
                        output_archive.start_file(name, options)?;
                        output_archive.write_all(&contents)?;
                        compressed_total += contents.len() as u64;
                    }
                }
            } else {
                // Copy non-image files or already-WebP files as-is
                output_archive.start_file(name, options)?;
                output_archive.write_all(&contents)?;
                compressed_total += contents.len() as u64;
            }
        }

        output_archive.finish()?;

        Ok((files_processed, original_total, compressed_total))
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
            Ok((true, Some("ZIP file contains convertible images".to_string())))
        } else {
            Ok((false, Some("ZIP file contains no convertible images".to_string())))
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
        let backup_path = source.with_extension(".backup");

        // Ensure output directory exists
        fs::create_dir_all(output_dir)?;

        if (output_path.exists()) {
            return Err(anyhow!(
                "Output file {} already exists",
                output_path.display()
            ));
        }

        if (backup_path.exists()) {
            return Err(anyhow!(
                "Backup file {} already exists",
                backup_path.display()
            ));
        }

        // Process the ZIP file
        let (files_processed, _original_total, _compressed_total) = self
            .process_zip(source, &output_path)
            .with_context(|| format!("Failed to process ZIP file: {}", source.display()))?;

        let compressed_size = get_file_size(&output_path)?;
        
        // Rename the origin file as backup and move the new ZIP to original location
        fs::rename(source, backup_path)?;
        fs::rename(&output_path, source).with_context(|| format!(
            "Failed to move converted ZIP to original location: {}",
            source.display()
        ))?;

        Ok(CompressionResult {
            original_size,
            compressed_size,
            output_path,
            plugin_name: self.metadata().name,
            files_processed,
            backup_path: None,
        })
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec!["zip"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
