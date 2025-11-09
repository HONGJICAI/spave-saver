use std::collections::HashSet;
use std::path::PathBuf;

use space_saver_service::ServiceApi;
use space_saver_service::api::{ScanResult, DuplicateGroup, SimilarGroup, StorageStats, FilterConfig};
use space_saver_service::FileOperations;
use tracing::{debug, info};
use tracing::field::debug;

/// Scan multiple directories
#[tauri::command]
pub async fn scan(paths: Vec<String>, filter: Option<FilterConfig>) -> Result<Vec<ScanResult>, String> {
    let api = ServiceApi::new();
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    
    api.scan_directories(paths, filter)
        .await
        .map_err(|e| e.to_string())
}

/// Find duplicate files across multiple paths
#[tauri::command]
pub async fn duplicate_file_check(paths: Vec<String>, filter: Option<FilterConfig>) -> Result<Vec<DuplicateGroup>, String> {
    let api = ServiceApi::new();
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    
    api.find_duplicates_in_paths(paths, filter)
        .await
        .map_err(|e| e.to_string())
}

/// Find similar images across multiple paths
#[tauri::command]
pub async fn similar_file_check(paths: Vec<String>, threshold: f32, filter: Option<FilterConfig>) -> Result<Vec<SimilarGroup>, String> {
    let api = ServiceApi::new();
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    
    api.find_similar_images_in_paths(paths, threshold, filter)
        .await
        .map_err(|e| e.to_string())
}

/// Check for empty folders and files across multiple paths
#[tauri::command]
pub async fn empty_folder_check(paths: Vec<String>, filter: Option<FilterConfig>) -> Result<Vec<String>, String> {
    use space_saver_core::{FileScanner, scanner::DefaultFileScanner};
    
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
    let empty_files: Vec<_> = all_files
        .into_iter()
        .filter(|f| f.size == 0)
        .collect();
    
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
    
    ops.delete_files(&paths)
        .map_err(|e| e.to_string())
}

/// Get storage statistics across multiple paths
#[tauri::command]
pub async fn get_storage_stats(paths: Vec<String>, filter: Option<FilterConfig>) -> Result<StorageStats, String> {
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
    
    Ok(plugins.iter().map(|p| serde_json::json!({
        "name": p.name,
        "description": p.description,
        "version": p.version,
    })).collect())
}

/// Scan paths and find compressible files with estimates
#[tauri::command]
pub async fn scan_compressible_files(
    paths: Vec<String>,
    active_plugins: Vec<String>,
    filter: Option<FilterConfig>,
) -> Result<serde_json::Value, String> {
    use space_saver_core::{FileScanner, scanner::DefaultFileScanner};
    use std::path::PathBuf;
    
    // Get the global plugin manager
    let manager = space_saver_core::compress_plugins::global_plugin_manager();
    let manager = manager.read().map_err(|e| e.to_string())?;
    
    // Step 1: Validate active plugins
    let all_plugin_names: Vec<String> = manager.get_plugins().iter().map(|p| p.name.clone()).collect();
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
    
    // Step 3: Try to apply plugins on each file
    let mut compressible_files = Vec::new();
    let mut rejected_files = Vec::new();
    
    for file_info in all_files {
        match check_file_compressibility(&file_info.path, &*manager, &active_plugins)? {
            Some(compress_info) => {
                compressible_files.push(compress_info);
            }
            None => {
                // File was rejected by all plugins, collect rejection reasons
                if let Some(rejection_info) = get_file_rejection_reasons(&file_info.path, &*manager, &active_plugins)? {
                    rejected_files.push(rejection_info);
                }
            }
        }
    }
    
    Ok(serde_json::json!({
        "compressible": compressible_files,
        "rejected": rejected_files,
    }))
}

fn check_file_compressibility(
    path: &PathBuf,
    manager: &space_saver_core::PluginManager,
    active_plugins: &Vec<String>,
) -> Result<Option<serde_json::Value>, String> {
    use std::fs;
    
    // Try each candidate plugin in order
    for plugin_name in active_plugins {
        // Check if this plugin can handle the file
        match manager.check_plugin_capability(path, &plugin_name) {
            Ok(Some((metadata, can_handle, reason, estimate_ratio))) => {
                if can_handle {
                    // Plugin can handle this file
                    // Get file size
                    let file_size = fs::metadata(path).map_err(|e| e.to_string())?.len();
                    
                    // Estimate compressed size
                    let ratio = estimate_ratio.unwrap_or(0.0);
                    let estimated_compressed = (file_size as f64 * (1.0 - ratio as f64)) as u64;
                    let estimated_savings = file_size.saturating_sub(estimated_compressed);
                    
                    return Ok(Some(serde_json::json!({
                        "path": path.to_string_lossy(),
                        "original_size": file_size,
                        "estimated_compressed_size": estimated_compressed,
                        "estimated_savings": estimated_savings,
                        "plugin_name": metadata.name,
                        "can_handle": true,
                        "reason": reason,
                    })));
                } else {
                    // Plugin cannot handle this file, continue to next plugin
                    // We could log the reason here for debugging
                    continue;
                }
            }
            Ok(None) => {
                // Plugin not found, skip
                continue;
            }
            Err(e) => {
                // Error checking plugin capability
                return Err(e.to_string());
            }
        }
    }
    
    // No plugin could handle this file
    Ok(None)
}

fn get_file_rejection_reasons(
    path: &PathBuf,
    manager: &space_saver_core::PluginManager,
    active_plugins: &Vec<String>,
) -> Result<Option<serde_json::Value>, String> {
    use std::fs;
    
    // Get file extension
    let extension = path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");
    
    // Get all plugins in registration order
    let all_plugins = manager.get_plugins();
    
    // Get plugins by extension and filter to only active ones
    let extension_plugins = manager.get_plugins_by_extension(extension);
    let mut candidate_plugins: Vec<String> = extension_plugins.iter()
        .filter(|p| active_plugins.contains(&p.name))
        .map(|p| p.name.clone())
        .collect();
    
    // If no plugins match by extension, try all active plugins in order
    if candidate_plugins.is_empty() {
        candidate_plugins = all_plugins.iter()
            .filter(|p| active_plugins.contains(&p.name))
            .map(|p| p.name.clone())
            .collect();
    }
    
    // Collect rejection reasons from all plugins
    let mut rejection_reasons = Vec::new();
    
    for plugin_name in candidate_plugins {
        // Check if this plugin can handle the file
        match manager.check_plugin_capability(path, &plugin_name) {
            Ok(Some((metadata, can_handle, reason, _))) => {
                if !can_handle {
                    rejection_reasons.push(serde_json::json!({
                        "plugin_name": metadata.name,
                        "reason": reason.unwrap_or_else(|| "Unknown reason".to_string()),
                    }));
                }
            }
            Ok(None) => {
                // Plugin not found, skip
                continue;
            }
            Err(e) => {
                rejection_reasons.push(serde_json::json!({
                    "plugin_name": plugin_name,
                    "reason": format!("Error: {}", e),
                }));
            }
        }
    }
    
    if rejection_reasons.is_empty() {
        return Ok(None);
    }
    
    // Get file size
    let file_size = fs::metadata(path).map_err(|e| e.to_string())?.len();
    
    Ok(Some(serde_json::json!({
        "path": path.to_string_lossy(),
        "size": file_size,
        "extension": extension,
        "rejection_reasons": rejection_reasons,
    })))
}

/// Compress files in place (rename original to .backup, create compressed with original name)
#[tauri::command]
pub async fn compress_files_in_place(
    file_paths: Vec<String>,
    plugin_orders: Vec<String>, // Ordered list of active plugin names
) -> Result<Vec<serde_json::Value>, String> {
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
                "success": false,
                "path": path_str,
                "error": "File not found",
            }));
            continue;
        }

        let source_dir = source.parent().ok_or("Failed to get parent directory")?;
        
        // Process file in-place using plugin's built-in backup logic with plugin order preference
        match manager.process_file(&source, source_dir, orders) {
            Ok(compress_result) => {
                results.push(serde_json::json!({
                    "success": true,
                    "path": compress_result.output_path.to_string_lossy(),
                    "backup_path": compress_result.backup_path.as_ref().map(|p| p.to_string_lossy()),
                    "original_size": compress_result.original_size,
                    "compressed_size": compress_result.compressed_size,
                    "savings": compress_result.original_size.saturating_sub(compress_result.compressed_size),
                    "plugin_name": compress_result.plugin_name,
                }));
            }
            Err(e) => {
                results.push(serde_json::json!({
                    "success": false,
                    "path": path_str,
                    "error": e.to_string(),
                }));
            }
        }
    }
    
    Ok(results)
}
