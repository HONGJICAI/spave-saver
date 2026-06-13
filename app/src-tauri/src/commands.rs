use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use once_cell::sync::Lazy;
use space_saver_core::hash_cache::HashCache;
use space_saver_core::skip_cache::{FileFingerprint, SkipCache};
use space_saver_service::api::{
    BrokenFile, DuplicateGroup, EmptyScanResult, FilterConfig, ScanResult, SimilarGroup,
    StorageStats,
};
use space_saver_service::ServiceApi;
use space_saver_service::{DeleteMode, DeleteResult, FileOperations, FixExtensionResult};

/// Remembers files a plugin already failed to shrink at a given quality so
/// scans can exclude them. Keyed by (path, plugin, quality), guarded by a
/// size+mtime fingerprint — no hashing, so lookups stay one stat call.
static SKIP_CACHE: Lazy<Arc<RwLock<SkipCache>>> = Lazy::new(|| {
    let cache = SkipCache::load(skip_cache_path());
    Arc::new(RwLock::new(cache))
});

#[cfg(not(test))]
fn skip_cache_path() -> PathBuf {
    space_saver_utils::Config::load_or_default()
        .cache_dir
        .join("compress_skip_cache.json")
}

/// Tests must not touch the real user cache; give each test process its own file
#[cfg(test)]
fn skip_cache_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "space-saver-test-skip-cache-{}.json",
        std::process::id()
    ))
}

/// Content-hash cache for duplicate scans: unchanged files (same size+mtime)
/// are not re-read on subsequent scans
static HASH_CACHE: Lazy<Arc<RwLock<HashCache>>> = Lazy::new(|| {
    let cache = HashCache::load(hash_cache_path());
    Arc::new(RwLock::new(cache))
});

#[cfg(not(test))]
fn hash_cache_path() -> PathBuf {
    space_saver_utils::Config::load_or_default()
        .cache_dir
        .join("duplicate_hash_cache.json")
}

#[cfg(test)]
fn hash_cache_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "space-saver-test-hash-cache-{}.json",
        std::process::id()
    ))
}

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
    let api = ServiceApi::new().with_hash_cache(Arc::clone(&HASH_CACHE));
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();

    let result = api
        .find_duplicates_in_paths(paths, filter)
        .await
        .map_err(|e| e.to_string())?;

    // Persist newly computed hashes; cache failures must not fail the scan
    if let Ok(mut cache) = HASH_CACHE.write() {
        if let Err(e) = cache.save() {
            tracing::warn!(error = %e, "Failed to persist duplicate hash cache");
        }
    }

    Ok(result)
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

/// Find empty files (0 bytes) and empty folders (no files anywhere beneath
/// them, reported topmost-only) across multiple paths. `filter` applies to
/// files only.
#[tauri::command]
pub async fn empty_folder_check(
    paths: Vec<String>,
    filter: Option<FilterConfig>,
) -> Result<EmptyScanResult, String> {
    let api = ServiceApi::new();
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();

    api.find_empty_in_paths(paths, filter)
        .await
        .map_err(|e| e.to_string())
}

/// Find broken (invalid or corrupted) files across multiple paths. Reports
/// only files that are provably unusable — corrupted/truncated content, or
/// content that does not match its extension. Empty files are excluded.
#[tauri::command]
pub async fn broken_file_check(
    paths: Vec<String>,
    filter: Option<FilterConfig>,
) -> Result<Vec<BrokenFile>, String> {
    let api = ServiceApi::new();
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();

    api.find_broken_files_in_paths(paths, filter)
        .await
        .map_err(|e| e.to_string())
}

/// Rename misnamed files (whose content does not match their extension) to the
/// extension matching their real content, reporting a per-file outcome. This
/// is the safe action for `extension_mismatch` results from `broken_file_check`
/// — the file is valid, just named wrong, so it is renamed rather than deleted.
#[tauri::command]
pub async fn fix_file_extensions(paths: Vec<String>) -> Result<Vec<FixExtensionResult>, String> {
    let ops = FileOperations::new();
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();

    Ok(ops.fix_extensions(&paths))
}

/// Delete files, reporting a per-file outcome. `mode` defaults to "trash"
/// (recoverable); "permanent" removes from disk immediately.
#[tauri::command]
pub async fn delete_files(
    paths: Vec<String>,
    mode: Option<DeleteMode>,
) -> Result<Vec<DeleteResult>, String> {
    let ops = FileOperations::new();
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    let mode = mode.unwrap_or(DeleteMode::Trash);

    Ok(ops.delete_files_with_mode(&paths, mode))
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

    let skip_cache = SKIP_CACHE.read().map_err(|e| e.to_string())?;

    for file_info in all_files {
        let mut rejection_reasons = Vec::new();
        let mut accepted = None;

        // The scanner already stat'ed the file; reuse size + mtime
        let fingerprint = FileFingerprint {
            size: file_info.size,
            mtime: file_info.modified,
        };
        let path_str = file_info.path.to_string_lossy().to_string();

        for plugin_name in &active_plugins {
            match manager.check_plugin_capability(&file_info.path, plugin_name) {
                Ok(Some((metadata, can_handle, reason, estimate_ratio))) => {
                    if can_handle {
                        // Skip-cache: this exact file state already produced no
                        // size reduction with this plugin at this quality
                        let quality = manager.get_plugin_quality(plugin_name);
                        if skip_cache.is_known_skip(&path_str, &fingerprint, plugin_name, quality) {
                            rejection_reasons.push(serde_json::json!({
                                "plugin_name": metadata.name,
                                "reason": format!(
                                    "Previously produced no size reduction{} (cached result; file unchanged)",
                                    quality.map(|q| format!(" at quality {}", q)).unwrap_or_default()
                                ),
                            }));
                            continue;
                        }

                        let ratio = estimate_ratio.unwrap_or(0.0);
                        let estimated_compressed =
                            (file_info.size as f64 * (1.0 - ratio as f64)) as u64;
                        let estimated_savings = file_info.size.saturating_sub(estimated_compressed);

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

/// Compress files in place. With `create_backup` the original is kept as
/// `<name>.bak` next to the output; without it the original is deleted once
/// compression fully succeeds (failures and skips never touch it). Each file
/// ends up in one of three states: "compressed", "skipped" (output was not
/// smaller, original kept untouched), or "failed".
#[tauri::command]
pub async fn compress_files_in_place(
    file_paths: Vec<String>,
    plugin_orders: Vec<String>, // Ordered list of active plugin names
    create_backup: bool,        // false: delete the original once compression succeeds
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
        match manager.process_file(&source, source_dir, orders, create_backup) {
            Ok(CompressionOutcome::Compressed(compress_result)) => {
                // Any remembered no-reduction results for this path are stale
                // (the file at this path was replaced or renamed away)
                if let Ok(mut cache) = SKIP_CACHE.write() {
                    cache.invalidate_path(&path_str);
                }
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
                // Remember this so the next scan excludes the file instead of
                // re-running the trial compression (skip leaves it untouched)
                if let Ok(fingerprint) = FileFingerprint::of(&source) {
                    let quality = manager.get_plugin_quality(&plugin_name);
                    if let Ok(mut cache) = SKIP_CACHE.write() {
                        cache.record_skip(&path_str, fingerprint, &plugin_name, quality);
                    }
                }
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

    // Persist new skip-cache entries; the cache is an optimization, so a
    // failed save must not fail the compression that already happened
    if let Ok(mut cache) = SKIP_CACHE.write() {
        if let Err(e) = cache.save() {
            tracing::warn!(error = %e, "Failed to persist compression skip cache");
        }
    }

    Ok(results)
}

/// Number of remembered no-size-reduction results
#[tauri::command]
pub async fn get_skip_cache_info() -> Result<serde_json::Value, String> {
    let cache = SKIP_CACHE.read().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "entries": cache.len() }))
}

/// Forget all remembered no-size-reduction results; returns how many were removed
#[tauri::command]
pub async fn clear_skip_cache() -> Result<usize, String> {
    let mut cache = SKIP_CACHE.write().map_err(|e| e.to_string())?;
    let removed = cache.clear();
    cache.save().map_err(|e| e.to_string())?;
    Ok(removed)
}

/// Location of the on-disk config file (the single source of truth for settings)
fn config_path() -> PathBuf {
    space_saver_utils::Config::default_path()
}

/// Load config from a path, falling back to defaults when the file is absent.
/// Split from the command so it can be tested against a temp path.
fn load_config_from(path: &std::path::Path) -> Result<space_saver_utils::Config, String> {
    if path.exists() {
        space_saver_utils::Config::load(path).map_err(|e| e.to_string())
    } else {
        Ok(space_saver_utils::Config::default())
    }
}

/// Validate then persist config to a path. Split from the command so it can be
/// tested against a temp path without touching the real user config.
fn save_config_to(
    path: &std::path::Path,
    config: &space_saver_utils::Config,
) -> Result<(), String> {
    config.validate().map_err(|e| e.to_string())?;
    config.save(path).map_err(|e| e.to_string())
}

/// Get the current application configuration (or defaults if none saved yet)
#[tauri::command]
pub async fn get_config() -> Result<space_saver_utils::Config, String> {
    load_config_from(&config_path())
}

/// Validate and persist the application configuration, returning what was saved
#[tauri::command]
pub async fn set_config(
    config: space_saver_utils::Config,
) -> Result<space_saver_utils::Config, String> {
    save_config_to(&config_path(), &config)?;
    Ok(config)
}

/// Detect optional external tools (ffmpeg etc.) on PATH. Runs the (blocking)
/// PATH lookup + version queries off the async runtime.
#[tauri::command]
pub async fn detect_tools() -> Result<Vec<space_saver_service::ToolStatus>, String> {
    tokio::task::spawn_blocking(space_saver_service::detect_tools)
        .await
        .map_err(|e| e.to_string())
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

    /// Tests touching the shared SKIP_CACHE must not run concurrently
    /// (clear_skip_cache would wipe another test's entries mid-flight).
    /// Async-aware so the guard may be held across await points.
    static CACHE_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

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
            scan_compressible_files(paths_of(&dir), vec!["No Such Plugin".to_string()], None).await;
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
            true,
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
            true,
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
    async fn compress_in_place_without_backup_leaves_no_bak_file() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("noise.png");
        save_noise_png(&source, 128, 128);

        let results = compress_files_in_place(
            vec![source.to_string_lossy().to_string()],
            vec!["WebP Converter".to_string()],
            false,
        )
        .await
        .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0]["status"], "compressed");
        assert!(results[0]["backup_path"].is_null());
        assert!(!source.exists(), "original deleted after success");
        assert!(
            !dir.path().join("noise.png.bak").exists(),
            "no .bak file when backups are disabled"
        );
        assert!(dir.path().join("noise.webp").exists());
    }

    #[tokio::test]
    async fn skip_cache_excludes_unchanged_files_from_scan() {
        let _guard = CACHE_TEST_LOCK.lock().await;
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("noise.png");
        save_noise_png(&source, 64, 64);
        let path_str = source.to_string_lossy().to_string();

        let active = vec!["WebP Converter".to_string()];

        // First scan: compressible
        let result = scan_compressible_files(paths_of(&dir), active.clone(), None)
            .await
            .unwrap();
        assert_eq!(result["compressible"].as_array().unwrap().len(), 1);

        // Simulate a remembered "no size reduction" result for this exact state
        {
            let manager = space_saver_core::compress_plugins::global_plugin_manager();
            let quality = manager.read().unwrap().get_plugin_quality("WebP Converter");
            let fp = FileFingerprint::of(&source).unwrap();
            SKIP_CACHE
                .write()
                .unwrap()
                .record_skip(&path_str, fp, "WebP Converter", quality);
        }

        // Second scan: excluded, with a cached-result rejection reason
        let result = scan_compressible_files(paths_of(&dir), active.clone(), None)
            .await
            .unwrap();
        assert_eq!(result["compressible"].as_array().unwrap().len(), 0);
        let rejected = result["rejected"].as_array().unwrap();
        assert_eq!(rejected.len(), 1);
        let reason = rejected[0]["rejection_reasons"][0]["reason"]
            .as_str()
            .unwrap();
        assert!(reason.contains("cached"), "reason: {reason}");

        // Touch the file (content change bumps size): cache entry no longer matches
        std::fs::write(&source, b"changed").unwrap();
        let result = scan_compressible_files(paths_of(&dir), active.clone(), None)
            .await
            .unwrap();
        // The png is no longer a valid image but it must not be cache-rejected;
        // it should not appear with a "cached" reason anymore
        let all = serde_json::to_string(&result).unwrap();
        assert!(
            !all.contains("cached result"),
            "stale fingerprint must miss: {all}"
        );

        SKIP_CACHE.write().unwrap().invalidate_path(&path_str);
    }

    #[tokio::test]
    async fn skip_cache_clear_restores_files() {
        let _guard = CACHE_TEST_LOCK.lock().await;
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("photo.png");
        save_noise_png(&source, 32, 32);
        let path_str = source.to_string_lossy().to_string();

        let fp = FileFingerprint::of(&source).unwrap();
        let manager = space_saver_core::compress_plugins::global_plugin_manager();
        let quality = manager.read().unwrap().get_plugin_quality("WebP Converter");
        SKIP_CACHE
            .write()
            .unwrap()
            .record_skip(&path_str, fp, "WebP Converter", quality);

        let info = get_skip_cache_info().await.unwrap();
        assert!(info["entries"].as_u64().unwrap() >= 1);

        let removed = clear_skip_cache().await.unwrap();
        assert!(removed >= 1);

        let result =
            scan_compressible_files(paths_of(&dir), vec!["WebP Converter".to_string()], None)
                .await
                .unwrap();
        assert_eq!(result["compressible"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn successful_compression_invalidates_skip_entries() {
        let _guard = CACHE_TEST_LOCK.lock().await;
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("invalidate-me.png");
        save_noise_png(&source, 64, 64);
        let path_str = source.to_string_lossy().to_string();

        // A (stale) skip entry exists for the path
        let fp = FileFingerprint::of(&source).unwrap();
        SKIP_CACHE
            .write()
            .unwrap()
            .record_skip(&path_str, fp, "Some Old Plugin", None);

        let results = compress_files_in_place(
            vec![path_str.clone()],
            vec!["WebP Converter".to_string()],
            true,
        )
        .await
        .unwrap();
        assert_eq!(results[0]["status"], "compressed");

        let cache = SKIP_CACHE.read().unwrap();
        assert!(
            !cache.is_known_skip(&path_str, &fp, "Some Old Plugin", None),
            "entries for a compressed path must be invalidated"
        );
    }

    #[tokio::test]
    async fn empty_check_finds_files_and_folders() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("zero.txt"), b"").unwrap();
        fs::write(dir.path().join("full.txt"), b"content").unwrap();
        fs::create_dir_all(dir.path().join("hollow/nested")).unwrap();

        let result = empty_folder_check(paths_of(&dir), None).await.unwrap();

        assert_eq!(
            result.empty_files,
            vec![dir.path().join("zero.txt").to_string_lossy().to_string()]
        );
        // Only the topmost empty folder is reported
        assert_eq!(
            result.empty_folders,
            vec![dir.path().join("hollow").to_string_lossy().to_string()]
        );
    }

    #[tokio::test]
    async fn empty_check_errors_on_nonexistent_path() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nope").to_string_lossy().to_string();
        assert!(empty_folder_check(vec![missing], None).await.is_err());
    }

    #[tokio::test]
    async fn empty_check_with_no_paths_returns_empty_result() {
        let result = empty_folder_check(vec![], None).await.unwrap();
        assert!(result.empty_files.is_empty());
        assert!(result.empty_folders.is_empty());
    }

    #[tokio::test]
    async fn delete_files_removes_empty_directories() {
        let dir = tempfile::tempdir().unwrap();
        let hollow = dir.path().join("hollow");
        fs::create_dir(&hollow).unwrap();
        let occupied = dir.path().join("occupied");
        fs::create_dir(&occupied).unwrap();
        fs::write(occupied.join("file.txt"), b"data").unwrap();

        let results = delete_files(
            vec![
                hollow.to_string_lossy().to_string(),
                occupied.to_string_lossy().to_string(),
            ],
            Some(space_saver_service::DeleteMode::Permanent),
        )
        .await
        .unwrap();

        assert!(results[0].success);
        assert!(!hollow.exists());
        // A folder that gained content after the scan must be refused
        assert!(!results[1].success);
        assert!(occupied.join("file.txt").exists());
    }

    #[tokio::test]
    async fn delete_files_reports_per_file_results() {
        let dir = tempfile::tempdir().unwrap();
        let existing = dir.path().join("delete-me.txt");
        std::fs::write(&existing, b"x").unwrap();
        let missing = dir.path().join("not-there.txt");

        let results = delete_files(
            vec![
                existing.to_string_lossy().to_string(),
                missing.to_string_lossy().to_string(),
            ],
            Some(space_saver_service::DeleteMode::Permanent),
        )
        .await
        .unwrap();

        assert_eq!(results.len(), 2);
        assert!(results[0].success);
        assert!(!existing.exists());
        assert!(
            !results[1].success,
            "missing file must be reported as failed"
        );
        assert!(results[1].error.is_some());
    }

    #[tokio::test]
    async fn broken_check_finds_corrupted_and_mismatched_files() {
        let dir = tempfile::tempdir().unwrap();
        // Truncated JPEG: valid signature, unparseable body
        fs::write(dir.path().join("truncated.jpg"), [0xFF, 0xD8, 0xFF, 0xE0]).unwrap();
        // PDF bytes wearing a .png extension
        fs::write(dir.path().join("fake.png"), b"%PDF-1.7\nnot a png").unwrap();
        // A healthy noise PNG must not be flagged
        save_noise_png(&dir.path().join("ok.png"), 32, 32);

        let broken = broken_file_check(paths_of(&dir), None).await.unwrap();

        assert_eq!(broken.len(), 2);
        assert!(broken.iter().any(|b| b.path.ends_with("truncated.jpg")));
        assert!(broken.iter().any(|b| b.path.ends_with("fake.png")));
        assert!(broken.iter().all(|b| !b.reason.is_empty()));
    }

    #[tokio::test]
    async fn fix_file_extensions_renames_misnamed_file() {
        let dir = tempfile::tempdir().unwrap();
        // PDF bytes wearing a .jpg extension
        let path = dir.path().join("scan.jpg");
        fs::write(&path, b"%PDF-1.7\nbody").unwrap();

        let results = fix_file_extensions(vec![path.to_string_lossy().to_string()])
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert!(results[0].success);
        assert!(results[0].new_path.as_ref().unwrap().ends_with("scan.pdf"));
        assert!(!path.exists());
        assert!(dir.path().join("scan.pdf").exists());
    }

    #[tokio::test]
    async fn fix_file_extensions_reports_failure_for_unrecognized_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("junk.jpg");
        fs::write(&path, b"not a recognizable format").unwrap();

        let results = fix_file_extensions(vec![path.to_string_lossy().to_string()])
            .await
            .unwrap();

        assert!(!results[0].success);
        assert!(results[0].error.is_some());
        assert!(path.exists(), "file untouched when nothing to fix");
    }

    #[tokio::test]
    async fn broken_check_with_no_paths_returns_empty() {
        let broken = broken_file_check(vec![], None).await.unwrap();
        assert!(broken.is_empty());
    }

    #[tokio::test]
    async fn broken_check_nonexistent_path_yields_no_results() {
        // Matches the sibling scan-based commands: a missing root is empty,
        // not an error.
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nope").to_string_lossy().to_string();
        let broken = broken_file_check(vec![missing], None).await.unwrap();
        assert!(broken.is_empty());
    }

    #[tokio::test]
    async fn duplicate_check_finds_groups_and_populates_hash_cache() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.bin"), b"identical bytes").unwrap();
        std::fs::write(dir.path().join("b.bin"), b"identical bytes").unwrap();
        std::fs::write(dir.path().join("unique.bin"), b"something else!!").unwrap();

        let groups = duplicate_file_check(paths_of(&dir), None).await.unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].count, 2);

        // Second scan resolves from the cache and agrees
        let groups = duplicate_file_check(paths_of(&dir), None).await.unwrap();
        assert_eq!(groups.len(), 1);
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

    #[test]
    fn load_config_returns_default_when_file_absent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("does-not-exist.toml");
        let config = load_config_from(&path).unwrap();
        assert_eq!(config.log_level, "info");
        assert_eq!(config.default_delete_mode, "trash");
    }

    #[test]
    fn save_then_load_config_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let config = space_saver_utils::Config {
            image_similarity_threshold: 0.75,
            default_delete_mode: "permanent".to_string(),
            default_compress_backup: false,
            ..Default::default()
        };

        save_config_to(&path, &config).unwrap();

        let loaded = load_config_from(&path).unwrap();
        assert_eq!(loaded.image_similarity_threshold, 0.75);
        assert_eq!(loaded.default_delete_mode, "permanent");
        assert!(!loaded.default_compress_backup);
    }

    #[test]
    fn save_config_rejects_invalid_values() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let bad_threshold = space_saver_utils::Config {
            image_similarity_threshold: 2.0,
            ..Default::default()
        };
        assert!(save_config_to(&path, &bad_threshold).is_err());
        // An invalid config must not have been written to disk
        assert!(!path.exists());

        let bad_mode = space_saver_utils::Config {
            default_delete_mode: "obliterate".to_string(),
            ..Default::default()
        };
        assert!(save_config_to(&path, &bad_mode).is_err());
    }

    #[tokio::test]
    async fn detect_tools_command_lists_known_tools() {
        let tools = detect_tools().await.unwrap();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"ffmpeg"));
        assert!(names.contains(&"cwebp"));
    }
}
