use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;
use anyhow::Result;
use tracing::{debug, info};

/// File information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: PathBuf,
    pub size: u64,
    pub modified: i64,
    pub file_type: FileType,
    pub hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileType {
    Image,
    Video,
    Document,
    Archive,
    Other,
}

/// File scanner trait
pub trait FileScanner {
    fn scan(&self, path: &Path) -> Result<Vec<FileInfo>>;
}

/// Default file scanner implementation
pub struct DefaultFileScanner {
    max_depth: Option<usize>,
    follow_links: bool,
}

impl DefaultFileScanner {
    pub fn new() -> Self {
        Self {
            max_depth: None,
            follow_links: false,
        }
    }

    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    pub fn follow_links(mut self, follow: bool) -> Self {
        self.follow_links = follow;
        self
    }

    fn determine_file_type(path: &Path) -> FileType {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "svg" => FileType::Image,
            "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" => FileType::Video,
            "pdf" | "doc" | "docx" | "txt" | "rtf" | "odt" => FileType::Document,
            "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" => FileType::Archive,
            _ => FileType::Other,
        }
    }
}

impl Default for DefaultFileScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl FileScanner for DefaultFileScanner {
    fn scan(&self, path: &Path) -> Result<Vec<FileInfo>> {
        info!("Starting scan of: {}", path.display());
        let mut results = Vec::new();

        let mut walker = WalkDir::new(path)
            .follow_links(self.follow_links);

        if let Some(depth) = self.max_depth {
            walker = walker.max_depth(depth);
        }

        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(e) => {
                    debug!("Failed to read metadata for {}: {}", entry.path().display(), e);
                    continue;
                }
            };

            if metadata.is_file() {
                let modified = metadata.modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);

                results.push(FileInfo {
                    path: entry.path().to_path_buf(),
                    size: metadata.len(),
                    modified,
                    file_type: Self::determine_file_type(entry.path()),
                    hash: None,
                });
            }
        }

        info!("Scan completed. Found {} files", results.len());
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_scan_directory() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "test content").unwrap();

        let scanner = DefaultFileScanner::new();
        let results = scanner.scan(dir.path()).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, file_path);
        assert_eq!(results[0].size, 12);
    }

    #[test]
    fn test_file_type_detection() {
        let img_path = Path::new("test.jpg");
        let video_path = Path::new("test.mp4");
        let doc_path = Path::new("test.pdf");

        assert!(matches!(
            DefaultFileScanner::determine_file_type(img_path),
            FileType::Image
        ));
        assert!(matches!(
            DefaultFileScanner::determine_file_type(video_path),
            FileType::Video
        ));
        assert!(matches!(
            DefaultFileScanner::determine_file_type(doc_path),
            FileType::Document
        ));
    }
}
