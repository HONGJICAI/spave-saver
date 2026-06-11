use std::collections::HashSet;
use std::path::PathBuf;

use space_saver_service::api::{
    DuplicateGroup, FilterConfig, ScanResult, SimilarGroup, StorageStats,
};
use space_saver_service::FileOperations;
use space_saver_service::ServiceApi;

/// Scan multiple directories
#[tauri::command]
pub async fn scan(
    paths: Vec<String>,
    filter: Option<FilterConfig>,
) -> Result<Vec<ScanResult>, String> {
    let api = ServiceApi::new();
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();

    api.scan_directories(paths, filter)
        .await
        .map_err(|e| e.to_string())
}

/// Find duplicate files across multiple paths
#[tauri::command]
pub async fn duplicate_file_check(
    paths: Vec<String>,
    filter: Option<FilterConfig>,
) -> Result<Vec<DuplicateGroup>, String> {
    let api = ServiceApi::new();
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();

    api.find_duplicates_in_paths(paths, filter)
        .await
        .map_err(|e| e.to_string())
}

/// Find similar images across multiple paths
#[tauri::command]
pub async fn similar_file_check(
    paths: Vec<String>,
    threshold: f32,
    filter: Option<FilterConfig>,
) -> Result<Vec<SimilarGroup>, String> {
    let api = ServiceApi::new();
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();

    api.find_similar_images_in_paths(paths, threshold, filter)
        .await
        .map_err(|e| e.to_string())
}

/// Check for empty folders and files across multiple paths
#[tauri::command]
pub async fn empty_folder_check(
    paths: Vec<String>,
    filter: Option<FilterConfig>,
) -> Result<Vec<String>, String> {
    use space_saver_core::{scanner::DefaultFileScanner, FileScanner};

    let scanner = DefaultFileScanner::new();
    let mut all_files = Vec::new();

    // Collect files from all paths
    for path_str in paths {
        let path = PathBuf::from(path_str);
        let mut files = scanner.scan(&path).map_err(|e| e.to_string())?;

        // Apply filters if provided
        if let Some(ref filter_config) = filter {
            files = filter_config.apply(files);
        }

        all_files.extend(files);
    }

    // Filter for empty files
    let empty_files: Vec<_> = all_files.into_iter().filter(|f| f.size == 0).collect();

    let result_paths: Vec<String> = empty_files
        .into_iter()
        .map(|f| f.path.to_string_lossy().to_string())
        .collect();

    Ok(result_paths)
}

/// Delete files
#[tauri::command]
pub async fn delete_files(paths: Vec<String>) -> Result<usize, String> {
    let ops = FileOperations::new();
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();

    ops.delete_files(&paths).map_err(|e| e.to_string())
}

/// Get storage statistics across multiple paths
#[tauri::command]
pub async fn get_storage_stats(
    paths: Vec<String>,
    filter: Option<FilterConfig>,
) -> Result<StorageStats, String> {
    let api = ServiceApi::new();
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();

    api.get_storage_stats_for_paths(paths, filter)
        .await
        .map_err(|e| e.to_string())
}

/// Get available compression plugins
#[tauri::command]
pub async fn get_compression_plugins() -> Result<Vec<serde_json::Value>, String> {
    let manager = space_saver_core::compress_plugins::global_plugin_manager();
    let manager = manager.read().map_err(|e| e.to_string())?;
    let plugins = manager.get_plugins();

    Ok(plugins
        .iter()
        .map(|p| {
            serde_json::json!({
                "name": p.name,
                "description": p.description,
                "version": p.version,
                "quality": manager.get_plugin_quality(&p.name),
            })
        })
        .collect())
}

/// Set the quality (0-100) of a compression plugin
#[tauri::command]
pub async fn set_plugin_quality(plugin_name: String, quality: f32) -> Result<(), String> {
    let manager = space_saver_core::compress_plugins::global_plugin_manager();
    let mut manager = manager.write().map_err(|e| e.to_string())?;
    manager
        .set_plugin_quality(&plugin_name, quality)
        .map_err(|e| e.to_string())
}

/// Scan paths and find compressible files with estimates
#[tauri::command]
pub async fn scan_compressible_files(
    paths: Vec<String>,
    active_plugins: Vec<String>,
    filter: Option<FilterConfig>,
) -> Result<serde_json::Value, String> {
    use space_saver_core::{scanner::DefaultFileScanner, FileScanner};
    use std::path::PathBuf;

    // Get the global plugin manager
    let manager = space_saver_core::compress_plugins::global_plugin_manager();
    let manager = manager.read().map_err(|e| e.to_string())?;

    // Step 1: Validate active plugins
    let all_plugin_names: Vec<String> = manager
        .get_plugins()
        .iter()
        .map(|p| p.name.clone())
        .collect();
    for plugin_name in &active_plugins {
        if !all_plugin_names.contains(plugin_name) {
            return Err(format!("Active plugin not found: {}", plugin_name));
        }
    }

    // Step 2: Merge plugin supported extensions with filterConfig
    let mut supported_extensions = HashSet::new();
    for plugin_name in &active_plugins {
        let exts = manager.get_supported_extensions(plugin_name);
        supported_extensions.extend(exts);
    }

    // Create or update filter config with plugin extensions
    let mut merged_filter = filter.unwrap_or_default();
    if !supported_extensions.is_empty() {
        // If filter already has extensions, intersect them with plugin extensions
        // Otherwise, use plugin extensions
        if let Some(existing_exts) = &merged_filter.extensions {
            let existing_set: HashSet<String> = existing_exts.iter().cloned().collect();
            let intersection: Vec<String> = supported_extensions
                .intersection(&existing_set)
                .cloned()
                .collect();
            merged_filter.extensions = if intersection.is_empty() {
                // If no intersection, use plugin extensions (plugin extensions take precedence)
                Some(supported_extensions.into_iter().collect())
            } else {
                Some(intersection)
            };
        } else {
            // No existing extensions, use plugin extensions
            merged_filter.extensions = Some(supported_extensions.into_iter().collect());
        }
    }

    // Use the global scanner to scan all paths
    let scanner = DefaultFileScanner::new();
    let mut all_files = Vec::new();

    for path_str in paths {
        let path = PathBuf::from(path_str);
        let mut files = scanner.scan(&path).map_err(|e| e.to_string())?;

        // Apply merged filters
        files = merged_filter.apply(files);

        all_files.extend(files);
    }

    // Step 3: Try each active plugin (in order) on each file, collecting
    // rejection reasons along the way in a single pass
    let mut compressible_files = Vec::new();
    let mut rejected_files = Vec::new();

    for file_info in all_files {
        let mut rejection_reasons = Vec::new();
        let mut accepted = None;

        for plugin_name in &active_plugins {
            match manager.check_plugin_capability(&file_info.path, plugin_name) {
                Ok(Some((metadata, can_handle, reason, estimate_ratio))) => {
                    if can_handle {
                        let ratio = estimate_ratio.unwrap_or(0.0);
                        let estimated_compressed =
                            (file_info.size as f64 * (1.0 - ratio as f64)) as u64;
                        let estimated_savings =
                            file_info.size.saturating_sub(estimated_compressed);

                        accepted = Some(serde_json::json!({
                            "path": file_info.path.to_string_lossy(),
                            "original_size": file_info.size,
                            "estimated_compressed_size": estimated_compressed,
                            "estimated_savings": estimated_savings,
                            "plugin_name": metadata.name,
                            "can_handle": true,
                            "reason": reason,
                        }));
                        break;
                    }

                    rejection_reasons.push(serde_json::json!({
                        "plugin_name": metadata.name,
                        "reason": reason.unwrap_or_else(|| "Unknown reason".to_string()),
                    }));
                }
                // Plugin not found (already validated above), skip
                Ok(None) => continue,
                // A plugin failing on one file (e.g. a corrupt archive) must
                // not abort the whole scan; record it as a rejection reason
                Err(e) => {
                    rejection_reasons.push(serde_json::json!({
                        "plugin_name": plugin_name,
                        "reason": format!("Error: {}", e),
                    }));
                }
            }
        }

        match accepted {
            Some(compress_info) => compressible_files.push(compress_info),
            None => {
                if !rejection_reasons.is_empty() {
                    let extension = file_info
                        .path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .unwrap_or("");
                    rejected_files.push(serde_json::json!({
                        "path": file_info.path.to_string_lossy(),
                        "size": file_info.size,
                        "extension": extension,
                        "rejection_reasons": rejection_reasons,
                    }));
                }
            }
        }
    }

    Ok(serde_json::json!({
        "compressible": compressible_files,
        "rejected": rejected_files,
    }))
}

/// Compress files in place. The manager renames the original to `<name>.bak`
/// (the backup) and keeps the compressed output next to it. Each file ends up
/// in one of three states: "compressed", "skipped" (output was not smaller,
/// original kept untouched), or "failed".
#[tauri::command]
pub async fn compress_files_in_place(
    file_paths: Vec<String>,
    plugin_orders: Vec<String>, // Ordered list of active plugin names
) -> Result<Vec<serde_json::Value>, String> {
    use space_saver_core::CompressionOutcome;
    use std::path::PathBuf;

    // Get the global plugin manager (all plugins pre-registered with priorities)
    let manager = space_saver_core::compress_plugins::global_plugin_manager();
    let manager = manager.read().map_err(|e| e.to_string())?;

    let mut results = Vec::new();

    // Convert plugin_orders to Option for process_file
    let orders = if plugin_orders.is_empty() {
        None
    } else {
        Some(plugin_orders.as_slice())
    };

    for path_str in file_paths {
        let source = PathBuf::from(&path_str);

        if !source.exists() {
            results.push(serde_json::json!({
                "status": "failed",
                "success": false,
                "path": path_str,
                "error": "File not found",
            }));
            continue;
        }

        let source_dir = source.parent().ok_or("Failed to get parent directory")?;

        // Only the plugins listed in plugin_orders are considered; the
        // manager performs the backup before replacing anything
        match manager.process_file(&source, source_dir, orders) {
            Ok(CompressionOutcome::Compressed(compress_result)) => {
                results.push(serde_json::json!({
                    "status": "compressed",
                    "success": true,
                    "path": compress_result.output_path.to_string_lossy(),
                    "backup_path": compress_result.backup_path.as_ref().map(|p| p.to_string_lossy()),
                    "original_size": compress_result.original_size,
                    "compressed_size": compress_result.compressed_size,
                    "savings": compress_result.original_size.saturating_sub(compress_result.compressed_size),
                    "plugin_name": compress_result.plugin_name,
                }));
            }
            Ok(CompressionOutcome::Skipped {
                plugin_name,
                reason,
            }) => {
                results.push(serde_json::json!({
                    "status": "skipped",
                    "success": true,
                    "path": path_str,
                    "plugin_name": plugin_name,
                    "reason": reason,
                }));
            }
            Err(e) => {
                results.push(serde_json::json!({
                    "status": "failed",
                    "success": false,
                    "path": path_str,
                    "error": e.to_string(),
                }));
            }
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};
    use std::fs;
    use std::path::Path;

    /// Noise PNG: stores poorly as PNG, so WebP conversion reliably shrinks it
    fn save_noise_png(path: &Path, width: u32, height: u32) {
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
        img.save(path).unwrap();
    }

    fn paths_of(dir: &tempfile::TempDir) -> Vec<String> {
        vec![dir.path().to_string_lossy().to_string()]
    }

    #[tokio::test]
    async fn scan_finds_compressible_and_rejected_files() {
        let dir = tempfile::tempdir().unwrap();
        save_noise_png(&dir.path().join("noise.png"), 64, 64);
        // A corrupt "ZIP" must land in rejected with a reason, not abort the scan
        fs::write(dir.path().join("fake.zip"), b"this is not a zip archive").unwrap();

        let result = scan_compressible_files(
            paths_of(&dir),
            vec![
                "Image ZIP to WebP ZIP".to_string(),
                "WebP Converter".to_string(),
            ],
            None,
        )
        .await
        .unwrap();

        let compressible = result["compressible"].as_array().unwrap();
        assert_eq!(compressible.len(), 1);
        assert!(compressible[0]["path"]
            .as_str()
            .unwrap()
            .ends_with("noise.png"));
        assert_eq!(compressible[0]["plugin_name"], "WebP Converter");
        assert!(compressible[0]["original_size"].as_u64().unwrap() > 0);

        let rejected = result["rejected"].as_array().unwrap();
        assert_eq!(rejected.len(), 1);
        assert!(rejected[0]["path"].as_str().unwrap().ends_with("fake.zip"));
        let reasons = rejected[0]["rejection_reasons"].as_array().unwrap();
        assert!(!reasons.is_empty());
        assert!(reasons.iter().all(|r| r["plugin_name"].is_string()));
    }

    #[tokio::test]
    async fn scan_rejects_unknown_plugin_name() {
        let dir = tempfile::tempdir().unwrap();
        let result =
            scan_compressible_files(paths_of(&dir), vec!["No Such Plugin".to_string()], None)
                .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn compress_in_place_reports_compressed_with_backup() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("noise.png");
        save_noise_png(&source, 128, 128);

        let results = compress_files_in_place(
            vec![source.to_string_lossy().to_string()],
            vec!["WebP Converter".to_string()],
        )
        .await
        .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0]["status"], "compressed");
        assert_eq!(results[0]["success"], true);
        assert!(results[0]["path"].as_str().unwrap().ends_with("noise.webp"));

        let backup = results[0]["backup_path"].as_str().unwrap().to_string();
        assert!(backup.ends_with("noise.png.bak"));
        assert!(Path::new(&backup).exists(), "backup file must exist");
        assert!(!source.exists(), "original renamed to backup");
        assert!(
            results[0]["savings"].as_u64().unwrap() > 0,
            "noise PNG must shrink as WebP"
        );
    }

    #[tokio::test]
    async fn compress_in_place_reports_failures() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("noise.png");
        save_noise_png(&source, 64, 64);

        let results = compress_files_in_place(
            vec![
                // No active plugin can handle a PNG when only the ZIP plugin is active
                source.to_string_lossy().to_string(),
                // Nonexistent file
                dir.path().join("missing.png").to_string_lossy().to_string(),
            ],
            vec!["Image ZIP to WebP ZIP".to_string()],
        )
        .await
        .unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0]["status"], "failed");
        assert!(results[0]["error"]
            .as_str()
            .unwrap()
            .contains("No active plugin"));
        assert!(source.exists(), "file must be untouched on failure");

        assert_eq!(results[1]["status"], "failed");
        assert_eq!(results[1]["error"], "File not found");
    }

    #[tokio::test]
    async fn plugin_quality_roundtrip() {
        let plugins = get_compression_plugins().await.unwrap();
        assert_eq!(plugins.len(), 3);
        assert!(plugins.iter().all(|p| p["quality"].is_number()));

        // Use the ZIP plugin here so parallel WebP-Converter tests are unaffected
        let name = "Image ZIP to WebP ZIP".to_string();
        set_plugin_quality(name.clone(), 60.0).await.unwrap();
        let plugins = get_compression_plugins().await.unwrap();
        let zip_plugin = plugins.iter().find(|p| p["name"] == name).unwrap();
        assert_eq!(zip_plugin["quality"], 60.0);

        // Restore the default so other tests see the expected state
        set_plugin_quality(name, 85.0).await.unwrap();

        assert!(set_plugin_quality("No Such Plugin".to_string(), 50.0)
            .await
            .is_err());
    }
}
