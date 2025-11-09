use crate::compress_plugins::{CompressionPlugin, CompressionResult};
use std::path::Path;
use std::process::Command;
use tracing::{error, info, warn};

pub struct AnimatedWebPConverterPlugin;

impl CompressionPlugin for AnimatedWebPConverterPlugin {
    fn metadata(&self) -> crate::compress_plugins::PluginMetadata {
        crate::compress_plugins::PluginMetadata {
            name: "Animated WebP Converter".to_string(),
            description: "Convert GIF to Animated WebP with lossy compression for better file size".to_string(),
            version: "1.0.0".to_string(),
        }
    }

    fn can_handle(&self, path: &Path) -> anyhow::Result<(bool, Option<String>)> {
        if let Some(ext) = path.extension() {
            let ext_lower = ext.to_string_lossy().to_lowercase();
            if ext_lower == "gif" {
                Ok((true, Some("GIF file for animated WebP conversion".to_string())))
            } else {
                Ok((false, Some(format!("Not a GIF file (extension: {})", ext_lower))))
            }
        } else {
            Ok((false, Some("No file extension".to_string())))
        }
    }

    fn estimate_ratio(&self, _path: &Path) -> anyhow::Result<Option<f32>> {
        // Animated WebP typically achieves 30-70% better compression than GIF
        Ok(Some(0.5))
    }

    fn process(&self, source: &Path, _output_dir: &Path) -> anyhow::Result<CompressionResult> {
        info!("Starting Animated WebP conversion for: {}", source.display());

        // Check if file exists
        if !source.exists() {
            let err = format!("Source file does not exist: {}", source.display());
            error!("{}", err);
            return Err(anyhow::anyhow!(err));
        }

        let original_size = std::fs::metadata(source)?.len();
        info!("Original GIF size: {} bytes", original_size);

        // Keep .gif extension to indicate it's animated
        // The file will be WebP format but with .gif extension for easy identification
        let output_path = source.with_extension("gif");
        let temp_path = source.with_extension("gif.tmp");

        // Convert using gif2webp (best quality) or ffmpeg as fallback
        let conversion_result = self.convert_with_gif2webp(source, &temp_path)
            .or_else(|_| self.convert_with_ffmpeg(source, &temp_path));

        match conversion_result {
            Ok(()) => {
                let compressed_size = std::fs::metadata(&temp_path)?.len();
                info!(
                    "Animated WebP conversion complete. Original: {} bytes, WebP: {} bytes",
                    original_size, compressed_size
                );

                // Compare sizes and decide which to keep
                if compressed_size >= original_size {
                    warn!(
                        "WebP size ({} bytes) is not smaller than GIF ({} bytes), removing WebP",
                        compressed_size, original_size
                    );
                    std::fs::remove_file(&temp_path)?;
                    return Err(anyhow::anyhow!(
                        "WebP conversion did not reduce file size"
                    ));
                }

                // WebP is smaller, remove original GIF and rename temp to final
                info!("WebP is smaller, replacing original GIF");
                std::fs::remove_file(source)?;
                std::fs::rename(&temp_path, &output_path)?;

                let savings = original_size.saturating_sub(compressed_size);
                info!("Space saved: {} bytes", savings);

                Ok(CompressionResult {
                    output_path: output_path.clone(),
                    original_size,
                    compressed_size,
                    plugin_name: self.metadata().name,
                    files_processed: 1,
                    backup_path: Some(source.to_path_buf()),
                })
            }
            Err(e) => {
                error!("Failed to convert GIF to Animated WebP: {}", e);
                // Clean up temp file if it exists
                let _ = std::fs::remove_file(&temp_path);
                Err(anyhow::anyhow!("Animated WebP conversion failed: {}", e))
            }
        }
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec!["gif"]
    }
}

impl AnimatedWebPConverterPlugin {
    /// Convert GIF to Animated WebP using gif2webp (recommended tool)
    fn convert_with_gif2webp(&self, input: &Path, output: &Path) -> anyhow::Result<()> {
        info!("Attempting GIF to Animated WebP conversion using gif2webp");

        let status = Command::new("gif2webp")
            .args(&[
                "-q",
                "85", // Quality 85
                "-m",
                "6", // Compression method 6 (best compression)
                "-lossy",
                input.to_str().unwrap(),
                "-o",
                output.to_str().unwrap(),
            ])
            .output()?;

        if status.status.success() {
            info!("gif2webp conversion successful");
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&status.stderr);
            warn!("gif2webp conversion failed: {}", stderr);
            Err(anyhow::anyhow!("gif2webp conversion failed: {}", stderr))
        }
    }

    /// Convert GIF to Animated WebP using FFmpeg (fallback)
    fn convert_with_ffmpeg(&self, input: &Path, output: &Path) -> anyhow::Result<()> {
        info!("Attempting GIF to Animated WebP conversion using FFmpeg");

        let status = Command::new("ffmpeg")
            .args(&[
                "-i",
                input.to_str().unwrap(),
                "-c:v",
                "libwebp",
                "-lossless",
                "0", // Use lossy compression
                "-quality",
                "75", // Quality setting
                "-loop",
                "0", // Loop forever like GIF
                "-y", // Overwrite output file
                output.to_str().unwrap(),
            ])
            .output()?;

        if status.status.success() {
            info!("FFmpeg conversion successful");
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&status.stderr);
            warn!("FFmpeg conversion failed: {}", stderr);
            Err(anyhow::anyhow!("FFmpeg conversion failed: {}", stderr))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_handle_gif() {
        let plugin = AnimatedWebPConverterPlugin;
        let (can_handle, reason) = plugin.can_handle(Path::new("test.gif")).unwrap();
        assert!(can_handle);
        assert_eq!(reason, Some("GIF file for animated WebP conversion".to_string()));
        
        let (can_handle, _) = plugin.can_handle(Path::new("TEST.GIF")).unwrap();
        assert!(can_handle);
        
        let (can_handle, reason) = plugin.can_handle(Path::new("test.png")).unwrap();
        assert!(!can_handle);
        assert!(reason.is_some());
        
        let (can_handle, reason) = plugin.can_handle(Path::new("test.jpg")).unwrap();
        assert!(!can_handle);
        assert!(reason.is_some());
    }

    #[test]
    fn test_metadata() {
        let plugin = AnimatedWebPConverterPlugin;
        let metadata = plugin.metadata();
        assert_eq!(metadata.name, "Animated WebP Converter");
    }

    #[test]
    fn test_supported_extensions() {
        let plugin = AnimatedWebPConverterPlugin;
        let extensions = plugin.supported_extensions();
        assert_eq!(extensions, vec!["gif"]);
    }
}
