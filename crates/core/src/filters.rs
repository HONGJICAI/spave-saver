use std::path::Path;
use std::collections::HashSet;
use crate::scanner::FileInfo;

/// File filter trait
pub trait Filter {
    fn apply(&self, file: &FileInfo) -> bool;
}

/// Filter by minimum file size
pub struct MinSizeFilter {
    min_size: u64,
}

impl MinSizeFilter {
    pub fn new(min_size: u64) -> Self {
        Self { min_size }
    }
}

impl Filter for MinSizeFilter {
    fn apply(&self, file: &FileInfo) -> bool {
        file.size >= self.min_size
    }
}

/// Filter by maximum file size
pub struct MaxSizeFilter {
    max_size: u64,
}

impl MaxSizeFilter {
    pub fn new(max_size: u64) -> Self {
        Self { max_size }
    }
}

impl Filter for MaxSizeFilter {
    fn apply(&self, file: &FileInfo) -> bool {
        file.size <= self.max_size
    }
}

/// Filter by file extension
pub struct ExtensionFilter {
    extensions: HashSet<String>,
}

impl ExtensionFilter {
    pub fn new(extensions: Vec<String>) -> Self {
        Self {
            extensions: extensions.iter().map(|s| s.to_lowercase()).collect(),
        }
    }
}

impl Filter for ExtensionFilter {
    fn apply(&self, file: &FileInfo) -> bool {
        if let Some(ext) = file.path.extension() {
            let ext = ext.to_string_lossy().to_lowercase();
            self.extensions.contains(&ext)
        } else {
            false
        }
    }
}

/// Filter by file name pattern
pub struct PatternFilter {
    pattern: String,
}

impl PatternFilter {
    pub fn new(pattern: String) -> Self {
        Self { pattern }
    }
}

impl Filter for PatternFilter {
    fn apply(&self, file: &FileInfo) -> bool {
        if let Some(name) = file.path.file_name() {
            name.to_string_lossy().contains(&self.pattern)
        } else {
            false
        }
    }
}

/// Filter to detect empty files
pub struct EmptyFileFilter;

impl Filter for EmptyFileFilter {
    fn apply(&self, file: &FileInfo) -> bool {
        file.size == 0
    }
}

/// Filter to detect hidden files (Unix-style)
pub struct HiddenFileFilter;

impl Filter for HiddenFileFilter {
    fn apply(&self, file: &FileInfo) -> bool {
        if let Some(name) = file.path.file_name() {
            name.to_string_lossy().starts_with('.')
        } else {
            false
        }
    }
}

/// Composite filter that combines multiple filters with AND logic
pub struct AndFilter {
    filters: Vec<Box<dyn Filter + Send + Sync>>,
}

impl AndFilter {
    pub fn new() -> Self {
        Self {
            filters: Vec::new(),
        }
    }

    pub fn add(mut self, filter: Box<dyn Filter + Send + Sync>) -> Self {
        self.filters.push(filter);
        self
    }
}

impl Default for AndFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl Filter for AndFilter {
    fn apply(&self, file: &FileInfo) -> bool {
        self.filters.iter().all(|f| f.apply(file))
    }
}

/// Composite filter that combines multiple filters with OR logic
pub struct OrFilter {
    filters: Vec<Box<dyn Filter + Send + Sync>>,
}

impl OrFilter {
    pub fn new() -> Self {
        Self {
            filters: Vec::new(),
        }
    }

    pub fn add(mut self, filter: Box<dyn Filter + Send + Sync>) -> Self {
        self.filters.push(filter);
        self
    }
}

impl Default for OrFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl Filter for OrFilter {
    fn apply(&self, file: &FileInfo) -> bool {
        self.filters.iter().any(|f| f.apply(file))
    }
}

/// Main file filter interface
pub struct FileFilter {
    filter: Box<dyn Filter + Send + Sync>,
}

impl FileFilter {
    pub fn new(filter: Box<dyn Filter + Send + Sync>) -> Self {
        Self { filter }
    }

    pub fn apply(&self, file: &FileInfo) -> bool {
        self.filter.apply(file)
    }

    pub fn filter_files(&self, files: Vec<FileInfo>) -> Vec<FileInfo> {
        files.into_iter().filter(|f| self.apply(f)).collect()
    }

    // Convenience constructors
    pub fn min_size(size: u64) -> Self {
        Self::new(Box::new(MinSizeFilter::new(size)))
    }

    pub fn max_size(size: u64) -> Self {
        Self::new(Box::new(MaxSizeFilter::new(size)))
    }

    pub fn extensions(exts: Vec<String>) -> Self {
        Self::new(Box::new(ExtensionFilter::new(exts)))
    }

    pub fn pattern(pattern: String) -> Self {
        Self::new(Box::new(PatternFilter::new(pattern)))
    }

    pub fn empty_files() -> Self {
        Self::new(Box::new(EmptyFileFilter))
    }

    pub fn hidden_files() -> Self {
        Self::new(Box::new(HiddenFileFilter))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use crate::scanner::FileType;

    fn create_test_file(path: &str, size: u64) -> FileInfo {
        FileInfo {
            path: PathBuf::from(path),
            size,
            modified: 0,
            file_type: FileType::Other,
            hash: None,
        }
    }

    #[test]
    fn test_min_size_filter() {
        let filter = MinSizeFilter::new(1000);
        let file1 = create_test_file("test1.txt", 500);
        let file2 = create_test_file("test2.txt", 1500);

        assert!(!filter.apply(&file1));
        assert!(filter.apply(&file2));
    }

    #[test]
    fn test_max_size_filter() {
        let filter = MaxSizeFilter::new(1000);
        let file1 = create_test_file("test1.txt", 500);
        let file2 = create_test_file("test2.txt", 1500);

        assert!(filter.apply(&file1));
        assert!(!filter.apply(&file2));
    }

    #[test]
    fn test_extension_filter() {
        let filter = ExtensionFilter::new(vec!["txt".to_string(), "doc".to_string()]);
        let file1 = create_test_file("test.txt", 100);
        let file2 = create_test_file("test.pdf", 100);

        assert!(filter.apply(&file1));
        assert!(!filter.apply(&file2));
    }

    #[test]
    fn test_pattern_filter() {
        let filter = PatternFilter::new("backup".to_string());
        let file1 = create_test_file("backup_2024.txt", 100);
        let file2 = create_test_file("document.txt", 100);

        assert!(filter.apply(&file1));
        assert!(!filter.apply(&file2));
    }

    #[test]
    fn test_empty_file_filter() {
        let filter = EmptyFileFilter;
        let file1 = create_test_file("empty.txt", 0);
        let file2 = create_test_file("nonempty.txt", 100);

        assert!(filter.apply(&file1));
        assert!(!filter.apply(&file2));
    }

    #[test]
    fn test_and_filter() {
        let filter = AndFilter::new()
            .add(Box::new(MinSizeFilter::new(100)))
            .add(Box::new(MaxSizeFilter::new(1000)));

        let file1 = create_test_file("test1.txt", 50);
        let file2 = create_test_file("test2.txt", 500);
        let file3 = create_test_file("test3.txt", 1500);

        assert!(!filter.apply(&file1)); // Too small
        assert!(filter.apply(&file2));  // Just right
        assert!(!filter.apply(&file3)); // Too large
    }
}
