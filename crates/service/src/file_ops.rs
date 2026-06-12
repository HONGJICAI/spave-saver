use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// How files should be removed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeleteMode {
    /// Move to the system trash / recycle bin (recoverable)
    Trash,
    /// Remove from disk immediately (unrecoverable)
    Permanent,
}

/// Per-file outcome of a delete operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteResult {
    pub path: String,
    pub success: bool,
    pub error: Option<String>,
}

/// File operations (delete, move, copy, etc.)
pub struct FileOperations;

impl FileOperations {
    pub fn new() -> Self {
        Self
    }

    /// Delete a file
    pub fn delete_file(&self, path: &Path) -> Result<()> {
        fs::remove_file(path)?;
        Ok(())
    }

    /// Delete multiple files
    pub fn delete_files(&self, paths: &[PathBuf]) -> Result<usize> {
        let mut count = 0;
        for path in paths {
            if self.delete_file(path).is_ok() {
                count += 1;
            }
        }
        Ok(count)
    }

    /// Delete files or empty directories reporting a per-file outcome instead
    /// of swallowing failures. Trash mode can fail on some mounts (e.g.
    /// network drives without a trash directory); those files are reported,
    /// not deleted. Directories are refused in every mode unless their
    /// subtree contains no files (empty-subfolder scaffolding is removed with
    /// them) — this operation backs the cleanup UI and must never take real
    /// data along with a "empty" folder that gained content after the scan.
    pub fn delete_files_with_mode(&self, paths: &[PathBuf], mode: DeleteMode) -> Vec<DeleteResult> {
        paths
            .iter()
            .map(|path| {
                let outcome = self.delete_path_with_mode(path, mode);
                match outcome {
                    Ok(()) => DeleteResult {
                        path: path.to_string_lossy().to_string(),
                        success: true,
                        error: None,
                    },
                    Err(e) => DeleteResult {
                        path: path.to_string_lossy().to_string(),
                        success: false,
                        error: Some(e),
                    },
                }
            })
            .collect()
    }

    fn delete_path_with_mode(
        &self,
        path: &Path,
        mode: DeleteMode,
    ) -> std::result::Result<(), String> {
        let is_dir = path.is_dir();
        if is_dir {
            match self.count_files(path) {
                Ok(0) => {}
                Ok(n) => return Err(format!("Directory is not empty ({} file(s) inside)", n)),
                Err(e) => return Err(e.to_string()),
            }
        }
        match mode {
            DeleteMode::Trash => trash::delete(path).map_err(|e| e.to_string()),
            DeleteMode::Permanent if is_dir => fs::remove_dir_all(path).map_err(|e| e.to_string()),
            DeleteMode::Permanent => fs::remove_file(path).map_err(|e| e.to_string()),
        }
    }

    /// Move a file
    pub fn move_file(&self, source: &Path, dest: &Path) -> Result<()> {
        fs::rename(source, dest)?;
        Ok(())
    }

    /// Copy a file
    pub fn copy_file(&self, source: &Path, dest: &Path) -> Result<u64> {
        let bytes = fs::copy(source, dest)?;
        Ok(bytes)
    }

    /// Create a directory
    pub fn create_dir(&self, path: &Path) -> Result<()> {
        fs::create_dir_all(path)?;
        Ok(())
    }

    /// Get file size
    pub fn file_size(&self, path: &Path) -> Result<u64> {
        let metadata = fs::metadata(path)?;
        Ok(metadata.len())
    }

    /// Check if file exists
    pub fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    /// Get directory size (recursive)
    #[allow(clippy::only_used_in_recursion)]
    pub fn dir_size(&self, path: &Path) -> Result<u64> {
        let mut total_size = 0u64;

        if path.is_dir() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_file() {
                    total_size += entry.metadata()?.len();
                } else if path.is_dir() {
                    total_size += self.dir_size(&path)?;
                }
            }
        }

        Ok(total_size)
    }

    /// Count files in directory (recursive)
    #[allow(clippy::only_used_in_recursion)]
    pub fn count_files(&self, path: &Path) -> Result<usize> {
        let mut count = 0;

        if path.is_dir() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_file() {
                    count += 1;
                } else if path.is_dir() {
                    count += self.count_files(&path)?;
                }
            }
        }

        Ok(count)
    }
}

impl Default for FileOperations {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_file_operations() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");

        fs::write(&file_path, "test content").unwrap();

        let ops = FileOperations::new();

        // Test file exists
        assert!(ops.exists(&file_path));

        // Test file size
        let size = ops.file_size(&file_path).unwrap();
        assert_eq!(size, 12);

        // Test copy
        let copy_path = dir.path().join("copy.txt");
        ops.copy_file(&file_path, &copy_path).unwrap();
        assert!(ops.exists(&copy_path));

        // Test delete
        ops.delete_file(&copy_path).unwrap();
        assert!(!ops.exists(&copy_path));
    }

    #[test]
    fn test_delete_with_mode_reports_per_file_results() {
        let dir = tempdir().unwrap();
        let existing = dir.path().join("existing.txt");
        fs::write(&existing, "content").unwrap();
        let missing = dir.path().join("missing.txt");

        let ops = FileOperations::new();
        let results =
            ops.delete_files_with_mode(&[existing.clone(), missing], DeleteMode::Permanent);

        assert_eq!(results.len(), 2);
        assert!(results[0].success);
        assert!(results[0].error.is_none());
        assert!(!existing.exists());

        // The failure is reported with its reason, not swallowed
        assert!(!results[1].success);
        assert!(results[1].error.is_some());
    }

    #[test]
    fn test_delete_empty_directory_permanently() {
        let dir = tempdir().unwrap();
        // Empty-subfolder scaffolding is removed together with the target
        let target = dir.path().join("hollow");
        fs::create_dir_all(target.join("nested/deeper")).unwrap();

        let ops = FileOperations::new();
        let results =
            ops.delete_files_with_mode(std::slice::from_ref(&target), DeleteMode::Permanent);

        assert_eq!(results.len(), 1);
        assert!(results[0].success, "error: {:?}", results[0].error);
        assert!(!target.exists());
    }

    #[test]
    fn test_delete_refuses_non_empty_directory_in_both_modes() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("occupied");
        fs::create_dir_all(target.join("nested")).unwrap();
        fs::write(target.join("nested/precious.txt"), "data").unwrap();

        let ops = FileOperations::new();
        for mode in [DeleteMode::Permanent, DeleteMode::Trash] {
            let results = ops.delete_files_with_mode(std::slice::from_ref(&target), mode);
            assert!(!results[0].success, "non-empty dir must be refused");
            assert!(results[0].error.as_deref().unwrap().contains("not empty"));
            assert!(target.join("nested/precious.txt").exists());
        }
    }

    #[test]
    fn test_delete_to_trash() {
        // Trash availability depends on the environment (e.g. tmpfs mounts
        // may have no trash directory), so accept either outcome but require
        // the report to be consistent with the filesystem state
        let dir = tempdir().unwrap();
        let file = dir.path().join("trash-me.txt");
        fs::write(&file, "content").unwrap();

        let ops = FileOperations::new();
        let results = ops.delete_files_with_mode(std::slice::from_ref(&file), DeleteMode::Trash);

        assert_eq!(results.len(), 1);
        if results[0].success {
            assert!(!file.exists(), "trashed file must be gone from its path");
        } else {
            assert!(file.exists(), "failed trash must leave the file in place");
            assert!(results[0].error.is_some());
        }
    }

    #[test]
    fn test_dir_operations() {
        let dir = tempdir().unwrap();

        fs::write(dir.path().join("file1.txt"), "content1").unwrap();
        fs::write(dir.path().join("file2.txt"), "content2").unwrap();

        let ops = FileOperations::new();

        // Test count files
        let count = ops.count_files(dir.path()).unwrap();
        assert_eq!(count, 2);

        // Test directory size
        let size = ops.dir_size(dir.path()).unwrap();
        assert!(size > 0);
    }
}
