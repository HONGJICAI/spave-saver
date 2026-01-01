use anyhow::Result;
use serde::{Deserialize, Serialize};
use space_saver_core::{scanner::DefaultFileScanner, FileFilter, FileInfo, FileScanner};
use std::path::PathBuf;

/// Filter configuration for file operations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FilterConfig {
    /// Minimum file size in bytes
    pub min_size: Option<u64>,
    /// Maximum file size in bytes
    pub max_size: Option<u64>,
    /// File extensions to include (e.g., ["jpg", "png"])
    pub extensions: Option<Vec<String>>,
    /// Pattern to match in filename
    pub file_pattern: Option<String>,
}

impl FilterConfig {
    /// Apply filters to a list of files
    pub fn apply(&self, files: Vec<FileInfo>) -> Vec<FileInfo> {
        let mut filtered = files;

        // Apply min size filter
        if let Some(min_size) = self.min_size {
            let filter = FileFilter::min_size(min_size);
            filtered = filter.filter_files(filtered);
        }

        // Apply max size filter
        if let Some(max_size) = self.max_size {
            let filter = FileFilter::max_size(max_size);
            filtered = filter.filter_files(filtered);
        }

        // Apply extensions filter
        if let Some(ref extensions) = self.extensions {
            if !extensions.is_empty() {
                let filter = FileFilter::extensions(extensions.clone());
                filtered = filter.filter_files(filtered);
            }
        }

        // Apply pattern filter
        if let Some(ref pattern) = self.file_pattern {
            if !pattern.is_empty() {
                let filter = FileFilter::pattern(pattern.clone());
                filtered = filter.filter_files(filtered);
            }
        }

        filtered
    }
}

/// Service API for external interfaces (Tauri, CLI, etc.)
pub struct ServiceApi {
    scanner: DefaultFileScanner,
}

impl ServiceApi {
    pub fn new() -> Self {
        Self {
            scanner: DefaultFileScanner::new(),
        }
    }

    /// Scan multiple directories (primary method)
    pub async fn scan_directories(
        &self,
        paths: Vec<PathBuf>,
        filter: Option<FilterConfig>,
    ) -> Result<Vec<ScanResult>> {
        let mut results = Vec::new();

        for path in paths {
            let mut files = self.scanner.scan(&path)?;

            // Apply filters if provided
            if let Some(ref filter_config) = filter {
                files = filter_config.apply(files);
            }

            let total_size: u64 = files.iter().map(|f| f.size).sum();
            let file_count = files.len();

            results.push(ScanResult {
                path,
                file_count,
                total_size,
                files,
            });
        }

        Ok(results)
    }

    /// Scan a single directory (delegates to scan_directories)
    pub async fn scan_directory(
        &self,
        path: PathBuf,
        filter: Option<FilterConfig>,
    ) -> Result<ScanResult> {
        let results = self.scan_directories(vec![path], filter).await?;
        results
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No scan results returned"))
    }

    /// Find duplicate files across multiple directories (primary method)
    pub async fn find_duplicates_in_paths(
        &self,
        paths: Vec<PathBuf>,
        filter: Option<FilterConfig>,
    ) -> Result<Vec<DuplicateGroup>> {
        use space_saver_core::FileHasher;
        use std::collections::HashMap;

        // Collect files from all paths
        let mut all_files = Vec::new();
        for path in paths {
            let mut files = self.scanner.scan(&path)?;

            // Apply filters if provided
            if let Some(ref filter_config) = filter {
                files = filter_config.apply(files);
            }

            all_files.extend(files);
        }

        // Step 1: Group files by size first
        let mut size_map: HashMap<u64, Vec<FileInfo>> = HashMap::new();
        for file in all_files {
            size_map.entry(file.size).or_default().push(file);
        }

        // Step 2: Only calculate hashes for files with the same size (potential duplicates)
        let hasher = FileHasher::new_blake3();
        let mut hash_map: HashMap<String, Vec<FileInfo>> = HashMap::new();

        for (_, files) in size_map {
            // Skip if only one file with this size
            if files.len() == 1 {
                continue;
            }

            // Calculate hashes only for files that might be duplicates
            for file in files {
                if let Ok(hash) = hasher.hash_file(&file.path) {
                    hash_map.entry(hash).or_default().push(file);
                }
            }
        }

        // Step 3: Build duplicate groups
        let duplicates: Vec<DuplicateGroup> = hash_map
            .into_iter()
            .filter(|(_, files)| files.len() > 1)
            .map(|(hash, files)| {
                let total_size: u64 = files.iter().map(|f| f.size).sum();
                let wasted_space = total_size - files[0].size;
                let count = files.len();

                DuplicateGroup {
                    hash,
                    files,
                    count,
                    total_size,
                    wasted_space,
                }
            })
            .collect();

        Ok(duplicates)
    }

    /// Find duplicate files in a single directory (delegates to find_duplicates_in_paths)
    pub async fn find_duplicates(
        &self,
        path: PathBuf,
        filter: Option<FilterConfig>,
    ) -> Result<Vec<DuplicateGroup>> {
        self.find_duplicates_in_paths(vec![path], filter).await
    }

    /// Find similar images across multiple directories (primary method)
    pub async fn find_similar_images_in_paths(
        &self,
        paths: Vec<PathBuf>,
        threshold: f32,
        filter: Option<FilterConfig>,
    ) -> Result<Vec<SimilarGroup>> {
        use space_saver_core::{
            image_sim::SimilarityAlgorithm, scanner::FileType, ImageSimilarity,
        };

        // Collect image files from all paths
        let mut image_files = Vec::new();
        for path in paths {
            let mut files = self.scanner.scan(&path)?;

            // Apply filters if provided
            if let Some(ref filter_config) = filter {
                files = filter_config.apply(files);
            }

            let mut images: Vec<_> = files
                .into_iter()
                .filter(|f| matches!(f.file_type, FileType::Image))
                .collect();
            image_files.append(&mut images);
        }

        let similarity = ImageSimilarity::new();
        let mut similar_groups = Vec::new();

        // Simple pairwise comparison (can be optimized)
        for i in 0..image_files.len() {
            for j in (i + 1)..image_files.len() {
                if let Ok(score) = similarity.compare(&image_files[i].path, &image_files[j].path) {
                    if score >= threshold {
                        similar_groups.push(SimilarGroup {
                            files: vec![image_files[i].clone(), image_files[j].clone()],
                            similarity_score: score,
                        });
                    }
                }
            }
        }

        Ok(similar_groups)
    }

    /// Find similar images in a single directory (delegates to find_similar_images_in_paths)
    pub async fn find_similar_images(
        &self,
        path: PathBuf,
        threshold: f32,
        filter: Option<FilterConfig>,
    ) -> Result<Vec<SimilarGroup>> {
        self.find_similar_images_in_paths(vec![path], threshold, filter)
            .await
    }

    /// Get storage statistics across multiple directories (primary method)
    pub async fn get_storage_stats_for_paths(
        &self,
        paths: Vec<PathBuf>,
        filter: Option<FilterConfig>,
    ) -> Result<StorageStats> {
        use space_saver_core::scanner::FileType;

        // Collect files from all paths
        let mut all_files = Vec::new();
        for path in paths {
            let mut files = self.scanner.scan(&path)?;

            // Apply filters if provided
            if let Some(ref filter_config) = filter {
                files = filter_config.apply(files);
            }

            all_files.extend(files);
        }

        let mut stats = StorageStats {
            total_files: all_files.len(),
            total_size: 0,
            images: 0,
            videos: 0,
            documents: 0,
            archives: 0,
            others: 0,
            empty_files: 0,
        };

        for file in all_files {
            stats.total_size += file.size;

            if file.size == 0 {
                stats.empty_files += 1;
            }

            match file.file_type {
                FileType::Image => stats.images += 1,
                FileType::Video => stats.videos += 1,
                FileType::Document => stats.documents += 1,
                FileType::Archive => stats.archives += 1,
                FileType::Other => stats.others += 1,
            }
        }

        Ok(stats)
    }

    /// Get storage statistics for a single directory (delegates to get_storage_stats_for_paths)
    pub async fn get_storage_stats(
        &self,
        path: PathBuf,
        filter: Option<FilterConfig>,
    ) -> Result<StorageStats> {
        self.get_storage_stats_for_paths(vec![path], filter).await
    }
}

impl Default for ServiceApi {
    fn default() -> Self {
        Self::new()
    }
}

/// Scan result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub path: PathBuf,
    pub file_count: usize,
    pub total_size: u64,
    pub files: Vec<FileInfo>,
}

/// Duplicate group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateGroup {
    pub hash: String,
    pub files: Vec<FileInfo>,
    pub count: usize,
    pub total_size: u64,
    pub wasted_space: u64,
}

/// Similar image group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarGroup {
    pub files: Vec<FileInfo>,
    pub similarity_score: f32,
}

/// Storage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_files: usize,
    pub total_size: u64,
    pub images: usize,
    pub videos: usize,
    pub documents: usize,
    pub archives: usize,
    pub others: usize,
    pub empty_files: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_service_api_creation() {
        let _api = ServiceApi::new();
        // Just ensure it can be created
    }

    #[tokio::test]
    async fn test_find_duplicates_without_filter() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        // Create duplicate files
        let content = b"Hello, World!";
        let mut file1 = fs::File::create(dir_path.join("file1.txt")).unwrap();
        file1.write_all(content).unwrap();

        let mut file2 = fs::File::create(dir_path.join("file2.txt")).unwrap();
        file2.write_all(content).unwrap();

        // Create a unique file
        let mut file3 = fs::File::create(dir_path.join("file3.txt")).unwrap();
        file3.write_all(b"Different content").unwrap();

        // Create a large duplicate
        let large_content = vec![b'A'; 2_000_000]; // 2MB
        let mut file4 = fs::File::create(dir_path.join("large1.bin")).unwrap();
        file4.write_all(&large_content).unwrap();

        let mut file5 = fs::File::create(dir_path.join("large2.bin")).unwrap();
        file5.write_all(&large_content).unwrap();

        let api = ServiceApi::new();
        let duplicates = api
            .find_duplicates_in_paths(vec![dir_path.to_path_buf()], None)
            .await
            .unwrap();

        // Should find 2 duplicate groups (txt files and large files)
        assert_eq!(duplicates.len(), 2, "Should find 2 duplicate groups");

        // Check that each group has 2 files
        for group in &duplicates {
            assert_eq!(group.count, 2, "Each group should have 2 files");
            assert_eq!(group.files.len(), 2);
        }
    }

    #[tokio::test]
    async fn test_find_duplicates_with_min_size_filter() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        // Create small duplicate files
        let small_content = b"Small";
        let mut file1 = fs::File::create(dir_path.join("small1.txt")).unwrap();
        file1.write_all(small_content).unwrap();
        drop(file1); // Ensure file is flushed

        let mut file2 = fs::File::create(dir_path.join("small2.txt")).unwrap();
        file2.write_all(small_content).unwrap();
        drop(file2);

        // Create large duplicate files (1MB each)
        let large_content = vec![b'A'; 1_000_000];
        let mut file3 = fs::File::create(dir_path.join("large1.bin")).unwrap();
        file3.write_all(&large_content).unwrap();
        drop(file3);

        let mut file4 = fs::File::create(dir_path.join("large2.bin")).unwrap();
        file4.write_all(&large_content).unwrap();
        drop(file4);

        let api = ServiceApi::new();

        // Filter: only files >= 100KB
        let filter = FilterConfig {
            min_size: Some(100_000),
            max_size: None,
            extensions: None,
            file_pattern: None,
        };

        let duplicates = api
            .find_duplicates_in_paths(vec![dir_path.to_path_buf()], Some(filter))
            .await
            .unwrap();

        // Should only find the large duplicates, not the small ones
        assert_eq!(
            duplicates.len(),
            1,
            "Should find only 1 duplicate group (large files)"
        );
        assert_eq!(duplicates[0].count, 2);

        // Verify the files are the large ones
        for file in &duplicates[0].files {
            assert!(file.size >= 100_000, "All files should be >= 100KB");
        }
    }

    #[tokio::test]
    async fn test_find_duplicates_with_max_size_filter() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        // Create small duplicate files
        let small_content = b"Small";
        let mut file1 = fs::File::create(dir_path.join("small1.txt")).unwrap();
        file1.write_all(small_content).unwrap();
        drop(file1);

        let mut file2 = fs::File::create(dir_path.join("small2.txt")).unwrap();
        file2.write_all(small_content).unwrap();
        drop(file2);

        // Create large duplicate files (1MB each)
        let large_content = vec![b'A'; 1_000_000];
        let mut file3 = fs::File::create(dir_path.join("large1.bin")).unwrap();
        file3.write_all(&large_content).unwrap();
        drop(file3);

        let mut file4 = fs::File::create(dir_path.join("large2.bin")).unwrap();
        file4.write_all(&large_content).unwrap();
        drop(file4);

        let api = ServiceApi::new();

        // Filter: only files <= 1KB
        let filter = FilterConfig {
            min_size: None,
            max_size: Some(1_000),
            extensions: None,
            file_pattern: None,
        };

        let duplicates = api
            .find_duplicates_in_paths(vec![dir_path.to_path_buf()], Some(filter))
            .await
            .unwrap();

        // Should only find the small duplicates
        assert_eq!(
            duplicates.len(),
            1,
            "Should find only 1 duplicate group (small files)"
        );
        assert_eq!(duplicates[0].count, 2);

        // Verify the files are the small ones
        for file in &duplicates[0].files {
            assert!(file.size <= 1_000, "All files should be <= 1KB");
        }
    }

    #[tokio::test]
    async fn test_find_duplicates_with_extension_filter() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        // Create duplicate .txt files
        let txt_content = b"Text content";
        let mut file1 = fs::File::create(dir_path.join("doc1.txt")).unwrap();
        file1.write_all(txt_content).unwrap();

        let mut file2 = fs::File::create(dir_path.join("doc2.txt")).unwrap();
        file2.write_all(txt_content).unwrap();

        // Create duplicate .bin files
        let bin_content = b"Binary content";
        let mut file3 = fs::File::create(dir_path.join("data1.bin")).unwrap();
        file3.write_all(bin_content).unwrap();

        let mut file4 = fs::File::create(dir_path.join("data2.bin")).unwrap();
        file4.write_all(bin_content).unwrap();

        let api = ServiceApi::new();

        // Filter: only .txt files
        let filter = FilterConfig {
            min_size: None,
            max_size: None,
            extensions: Some(vec!["txt".to_string()]),
            file_pattern: None,
        };

        let duplicates = api
            .find_duplicates_in_paths(vec![dir_path.to_path_buf()], Some(filter))
            .await
            .unwrap();

        // Should only find txt duplicates
        assert_eq!(
            duplicates.len(),
            1,
            "Should find only 1 duplicate group (.txt files)"
        );
        assert_eq!(duplicates[0].count, 2);

        // Verify all files have .txt extension
        for file in &duplicates[0].files {
            assert!(
                file.path.extension().unwrap() == "txt",
                "All files should be .txt"
            );
        }
    }

    #[tokio::test]
    async fn test_find_duplicates_with_pattern_filter() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        // Create duplicate files with "report" in name
        let content1 = b"Report content";
        let mut file1 = fs::File::create(dir_path.join("report_2024.txt")).unwrap();
        file1.write_all(content1).unwrap();

        let mut file2 = fs::File::create(dir_path.join("report_2025.txt")).unwrap();
        file2.write_all(content1).unwrap();

        // Create duplicate files with "data" in name
        let content2 = b"Data content";
        let mut file3 = fs::File::create(dir_path.join("data_old.txt")).unwrap();
        file3.write_all(content2).unwrap();

        let mut file4 = fs::File::create(dir_path.join("data_new.txt")).unwrap();
        file4.write_all(content2).unwrap();

        let api = ServiceApi::new();

        // Filter: only files with "report" in name
        let filter = FilterConfig {
            min_size: None,
            max_size: None,
            extensions: None,
            file_pattern: Some("report".to_string()),
        };

        let duplicates = api
            .find_duplicates_in_paths(vec![dir_path.to_path_buf()], Some(filter))
            .await
            .unwrap();

        // Should only find report duplicates
        assert_eq!(
            duplicates.len(),
            1,
            "Should find only 1 duplicate group (report files)"
        );
        assert_eq!(duplicates[0].count, 2);

        // Verify all files contain "report" in filename
        for file in &duplicates[0].files {
            let filename = file.path.file_name().unwrap().to_str().unwrap();
            assert!(
                filename.contains("report"),
                "All files should contain 'report' in name"
            );
        }
    }

    #[tokio::test]
    async fn test_find_duplicates_with_combined_filters() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        // Create small .txt file
        let mut file1 = fs::File::create(dir_path.join("small.txt")).unwrap();
        file1.write_all(b"Small").unwrap();
        drop(file1);

        // Create duplicate large .txt files (200KB each)
        let large_txt_content = vec![b'A'; 200_000];
        let mut file2 = fs::File::create(dir_path.join("large1.txt")).unwrap();
        file2.write_all(&large_txt_content).unwrap();
        drop(file2);

        let mut file3 = fs::File::create(dir_path.join("large2.txt")).unwrap();
        file3.write_all(&large_txt_content).unwrap();
        drop(file3);

        // Create duplicate large .bin files (200KB each)
        let mut file4 = fs::File::create(dir_path.join("large1.bin")).unwrap();
        file4.write_all(&large_txt_content).unwrap();
        drop(file4);

        let mut file5 = fs::File::create(dir_path.join("large2.bin")).unwrap();
        file5.write_all(&large_txt_content).unwrap();
        drop(file5);

        let api = ServiceApi::new();

        // Filter: .txt files AND size >= 100KB
        let filter = FilterConfig {
            min_size: Some(100_000),
            max_size: None,
            extensions: Some(vec!["txt".to_string()]),
            file_pattern: None,
        };

        let duplicates = api
            .find_duplicates_in_paths(vec![dir_path.to_path_buf()], Some(filter))
            .await
            .unwrap();

        // Should only find large .txt duplicates
        assert_eq!(
            duplicates.len(),
            1,
            "Should find only 1 duplicate group (large .txt files)"
        );
        assert_eq!(duplicates[0].count, 2);

        // Verify files match both criteria
        for file in &duplicates[0].files {
            assert!(file.size >= 100_000, "Files should be >= 100KB");
            assert!(
                file.path.extension().unwrap() == "txt",
                "Files should be .txt"
            );
        }
    }

    #[tokio::test]
    async fn test_find_duplicates_multiple_paths() {
        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();
        let dir1_path = temp_dir1.path();
        let dir2_path = temp_dir2.path();

        // Create duplicate files across two directories
        let content = b"Shared content";
        let mut file1 = fs::File::create(dir1_path.join("file1.txt")).unwrap();
        file1.write_all(content).unwrap();

        let mut file2 = fs::File::create(dir2_path.join("file2.txt")).unwrap();
        file2.write_all(content).unwrap();

        let api = ServiceApi::new();
        let duplicates = api
            .find_duplicates_in_paths(vec![dir1_path.to_path_buf(), dir2_path.to_path_buf()], None)
            .await
            .unwrap();

        // Should find duplicates across both directories
        assert_eq!(
            duplicates.len(),
            1,
            "Should find 1 duplicate group across directories"
        );
        assert_eq!(duplicates[0].count, 2);
    }
}
