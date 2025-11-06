use std::path::{Path, PathBuf};
use std::fs;
use anyhow::Result;

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
            if let Ok(_) = self.delete_file(path) {
                count += 1;
            }
        }
        Ok(count)
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
