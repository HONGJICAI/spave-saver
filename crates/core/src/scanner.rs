use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{debug, info};
use walkdir::WalkDir;

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
        let ext = path
            .extension()
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

        let mut walker = WalkDir::new(path).follow_links(self.follow_links);

        if let Some(depth) = self.max_depth {
            walker = walker.max_depth(depth);
        }

        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(e) => {
                    debug!(
                        "Failed to read metadata for {}: {}",
                        entry.path().display(),
                        e
                    );
                    continue;
                }
            };

            if metadata.is_file() {
                let modified = metadata
                    .modified()
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

/// Find the topmost empty directories beneath `path`. A directory counts as
/// empty when its subtree contains no files (it may contain other empty
/// directories, which are subsumed by their topmost empty ancestor). The scan
/// root itself is never reported, so callers can always offer the results for
/// deletion without risking the path the user asked to scan.
pub fn find_empty_dirs(path: &Path) -> Result<Vec<PathBuf>> {
    /// Returns whether `dir`'s subtree contains no files, appending the
    /// topmost empty directories found inside non-empty subtrees to `out`.
    /// Unreadable directories are treated as non-empty: a directory we cannot
    /// inspect must never be offered for deletion.
    fn collect(dir: &Path, out: &mut Vec<PathBuf>) -> bool {
        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) => {
                debug!("Failed to read directory {}: {}", dir.display(), e);
                return false;
            }
        };

        let mut is_empty = true;
        let mut empty_subdirs = Vec::new();
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => {
                    is_empty = false;
                    continue;
                }
            };
            // Symlinks count as content, never followed (matching the scanner)
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            if is_dir {
                let mut nested = Vec::new();
                if collect(&entry.path(), &mut nested) {
                    empty_subdirs.push(entry.path());
                } else {
                    is_empty = false;
                    out.append(&mut nested);
                }
            } else {
                is_empty = false;
            }
        }

        if !is_empty {
            out.append(&mut empty_subdirs);
        }
        is_empty
    }

    // A missing or unreadable scan root is the caller's error, unlike
    // unreadable directories encountered mid-walk
    let entries = std::fs::read_dir(path)?;

    let mut out = Vec::new();
    for entry in entries {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let mut nested = Vec::new();
            if collect(&entry.path(), &mut nested) {
                out.push(entry.path());
            } else {
                out.append(&mut nested);
            }
        }
    }
    out.sort();
    Ok(out)
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
    fn test_find_empty_dirs_reports_topmost_only() {
        let dir = tempdir().unwrap();
        // a/ contains a file plus an empty chain b/c -> report a/b only
        fs::create_dir_all(dir.path().join("a/b/c")).unwrap();
        fs::write(dir.path().join("a/file.txt"), "content").unwrap();
        // d/ contains only the empty e/ -> d itself is the topmost empty dir
        fs::create_dir_all(dir.path().join("d/e")).unwrap();

        let result = find_empty_dirs(dir.path()).unwrap();
        assert_eq!(result, vec![dir.path().join("a/b"), dir.path().join("d")]);
    }

    #[test]
    fn test_find_empty_dirs_never_reports_scan_root() {
        let dir = tempdir().unwrap();
        // A fully empty scan root yields nothing, not the root itself
        assert_eq!(find_empty_dirs(dir.path()).unwrap(), Vec::<PathBuf>::new());
    }

    #[test]
    fn test_find_empty_dirs_with_no_empty_dirs() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("sub")).unwrap();
        fs::write(dir.path().join("sub/file.txt"), "content").unwrap();

        assert_eq!(find_empty_dirs(dir.path()).unwrap(), Vec::<PathBuf>::new());
    }

    #[test]
    fn test_find_empty_dirs_nonexistent_path_errors() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("does-not-exist");
        assert!(find_empty_dirs(&missing).is_err());
    }

    #[test]
    fn test_find_empty_dirs_file_as_root_errors() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("file.txt");
        fs::write(&file, "content").unwrap();
        assert!(find_empty_dirs(&file).is_err());
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
