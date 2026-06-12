use crate::compress_plugins::{create_output_file, CompressionPlugin, CompressionResult};
use once_cell::sync::Lazy;
use std::path::Path;
use std::process::Command;
use tracing::{info, warn};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

/// External tool used for the conversion, detected once per process
static AVAILABLE_TOOL: Lazy<Option<&'static str>> = Lazy::new(|| {
    for tool in ["gif2webp", "ffmpeg"] {
        if new_command(tool).arg("-version").output().is_ok() {
            return Some(tool);
        }
    }
    None
});

fn new_command(program: &str) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(program);

    // On Windows, prevent opening a new terminal window
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

    cmd
}

pub struct AnimatedWebPConverterPlugin {
    quality: f32,
}

impl AnimatedWebPConverterPlugin {
    pub fn new() -> Self {
        Self { quality: 85.0 }
    }

    pub fn with_quality(mut self, quality: f32) -> Self {
        self.quality = quality.clamp(0.0, 100.0);
        self
    }
}

impl Default for AnimatedWebPConverterPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl CompressionPlugin for AnimatedWebPConverterPlugin {
    fn metadata(&self) -> crate::compress_plugins::PluginMetadata {
        crate::compress_plugins::PluginMetadata {
            name: "Animated WebP Converter".to_string(),
            description: "Convert GIF to Animated WebP with lossy compression for better file size"
                .to_string(),
            version: "1.0.0".to_string(),
        }
    }

    fn can_handle(&self, path: &Path) -> anyhow::Result<(bool, Option<String>)> {
        if let Some(ext) = path.extension() {
            let ext_lower = ext.to_string_lossy().to_lowercase();
            if ext_lower == "gif" {
                if AVAILABLE_TOOL.is_none() {
                    return Ok((
                        false,
                        Some("Requires gif2webp or ffmpeg in PATH; neither was found".to_string()),
                    ));
                }
                Ok((
                    true,
                    Some("GIF file for animated WebP conversion".to_string()),
                ))
            } else {
                Ok((
                    false,
                    Some(format!("Not a GIF file (extension: {})", ext_lower)),
                ))
            }
        } else {
            Ok((false, Some("No file extension".to_string())))
        }
    }

    fn estimate_ratio(&self, _path: &Path) -> anyhow::Result<Option<f32>> {
        // Animated WebP typically achieves 30-70% better compression than GIF
        Ok(Some(0.5))
    }

    fn process(&self, source: &Path, output_dir: &Path) -> anyhow::Result<CompressionResult> {
        info!(
            "Starting Animated WebP conversion for: {}",
            source.display()
        );

        // Check if file exists
        if !source.exists() {
            let err = format!("Source file does not exist: {}", source.display());
            return Err(anyhow::anyhow!(err));
        }

        let original_size = std::fs::metadata(source)?.len();
        info!("Original GIF size: {} bytes", original_size);

        let stem = source
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        let output_path = output_dir.join(format!("{}.animated.webp", stem));
        let temp_path = output_dir.join(format!("{}.animated_temp.webp", stem));

        // The external tools cannot create-exclusively, so reserve the final
        // output name atomically (create_new) before converting; a concurrent
        // writer targeting the same name fails here instead of overwriting
        create_output_file(&output_path)?;

        // Convert using gif2webp (best quality) or ffmpeg as fallback;
        // the manager handles size comparison, backup, and replacement
        let conversion_result = self
            .convert_with_gif2webp(source, &temp_path)
            .or_else(|_| self.convert_with_ffmpeg(source, &temp_path));

        let finish = || -> anyhow::Result<u64> {
            let compressed_size = std::fs::metadata(&temp_path)?.len();
            // Replaces our own empty placeholder with the real output
            std::fs::rename(&temp_path, &output_path)?;
            Ok(compressed_size)
        };

        match conversion_result.and_then(|()| finish()) {
            Ok(compressed_size) => {
                info!(
                    "Animated WebP conversion complete. Original: {} bytes, WebP: {} bytes",
                    original_size, compressed_size
                );

                Ok(CompressionResult {
                    output_path,
                    original_size,
                    compressed_size,
                    plugin_name: self.metadata().name,
                    files_processed: 1,
                    backup_path: None,
                    replace_source: false,
                })
            }
            Err(e) => {
                // Clean up the temp file and the reserved placeholder
                let _ = std::fs::remove_file(&temp_path);
                let _ = std::fs::remove_file(&output_path);
                Err(anyhow::anyhow!("Animated WebP conversion failed: {}", e))
            }
        }
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec!["gif"]
    }

    fn quality(&self) -> Option<f32> {
        Some(self.quality)
    }

    fn set_quality(&mut self, quality: f32) -> bool {
        self.quality = quality.clamp(0.0, 100.0);
        true
    }
}

impl AnimatedWebPConverterPlugin {
    /// Convert GIF to Animated WebP using gif2webp (recommended tool)
    fn convert_with_gif2webp(&self, input: &Path, output: &Path) -> anyhow::Result<()> {
        info!("Attempting GIF to Animated WebP conversion using gif2webp");

        let quality = format!("{}", self.quality.round() as u32);
        let mut cmd = new_command("gif2webp");
        cmd.args([
            "-q",
            &quality,
            "-m",
            "6", // Compression method 6 (best compression)
            "-lossy",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ]);

        let status = cmd.output()?;

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

        let quality = format!("{}", self.quality.round() as u32);
        let mut cmd = new_command("ffmpeg");
        cmd.args([
            "-i",
            input.to_str().unwrap(),
            "-c:v",
            "libwebp",
            "-lossless",
            "0", // Use lossy compression
            "-quality",
            &quality,
            "-loop",
            "0",  // Loop forever like GIF
            "-y", // Overwrite output file
            output.to_str().unwrap(),
        ]);

        let status = cmd.output()?;

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

    fn tool_available() -> bool {
        AVAILABLE_TOOL.is_some()
    }

    #[test]
    fn test_can_handle_gif() {
        let plugin = AnimatedWebPConverterPlugin::new();

        let (can_handle, reason) = plugin.can_handle(Path::new("test.gif")).unwrap();
        if tool_available() {
            assert!(can_handle);
            assert_eq!(
                reason,
                Some("GIF file for animated WebP conversion".to_string())
            );

            let (can_handle, _) = plugin.can_handle(Path::new("TEST.GIF")).unwrap();
            assert!(can_handle);
        } else {
            // Without gif2webp/ffmpeg installed, GIFs must be rejected up front
            assert!(!can_handle);
            assert!(reason.unwrap().contains("gif2webp"));
        }

        let (can_handle, reason) = plugin.can_handle(Path::new("test.png")).unwrap();
        assert!(!can_handle);
        assert!(reason.is_some());

        let (can_handle, reason) = plugin.can_handle(Path::new("test.jpg")).unwrap();
        assert!(!can_handle);
        assert!(reason.is_some());
    }

    #[test]
    fn test_metadata() {
        let plugin = AnimatedWebPConverterPlugin::new();
        let metadata = plugin.metadata();
        assert_eq!(metadata.name, "Animated WebP Converter");
    }

    #[test]
    fn test_supported_extensions() {
        let plugin = AnimatedWebPConverterPlugin::new();
        let extensions = plugin.supported_extensions();
        assert_eq!(extensions, vec!["gif"]);
    }

    #[test]
    fn test_quality() {
        let mut plugin = AnimatedWebPConverterPlugin::new();
        assert_eq!(CompressionPlugin::quality(&plugin), Some(85.0));
        assert!(plugin.set_quality(60.0));
        assert_eq!(CompressionPlugin::quality(&plugin), Some(60.0));
    }
}
