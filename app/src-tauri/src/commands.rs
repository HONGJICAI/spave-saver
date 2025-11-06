use std::path::PathBuf;

use space_saver_service::ServiceApi;
use space_saver_service::api::{ScanResult, DuplicateGroup, SimilarGroup, StorageStats, FilterConfig};
use space_saver_service::FileOperations;

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

/// Compress files
#[tauri::command]
pub async fn compress_files(
    paths: Vec<String>,
    output_path: String,
) -> Result<u64, String> {
    use space_saver_core::Compressor;
    use std::path::Path;
    
    let compressor = Compressor::new_zip();
    
    if paths.len() == 1 {
        let source = Path::new(&paths[0]);
        let dest = Path::new(&output_path);
        
        if source.is_dir() {
            compressor.compress_directory(source, dest)
        } else {
            compressor.compress_file(source, dest)
        }
        .map_err(|e| e.to_string())
    } else {
        Err("Multiple file compression not yet implemented".to_string())
    }
}
