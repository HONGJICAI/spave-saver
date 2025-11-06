use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use anyhow::Result;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Database path
    pub database_path: PathBuf,
    
    /// Cache directory
    pub cache_dir: PathBuf,
    
    /// Log level
    pub log_level: String,
    
    /// Maximum concurrent tasks
    pub max_concurrent_tasks: usize,
    
    /// Default hash algorithm
    pub hash_algorithm: HashAlgorithm,
    
    /// Image similarity threshold
    pub image_similarity_threshold: f32,
    
    /// Scan settings
    pub scan: ScanConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    /// Follow symbolic links
    pub follow_links: bool,
    
    /// Maximum scan depth
    pub max_depth: Option<usize>,
    
    /// Minimum file size to include (bytes)
    pub min_file_size: u64,
    
    /// File patterns to exclude
    pub exclude_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HashAlgorithm {
    Blake3,
    Sha256,
}

impl Default for Config {
    fn default() -> Self {
        let config_dir = directories::ProjectDirs::from("com", "spacesaver", "Space-Saver")
            .map(|dirs| dirs.config_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));

        let data_dir = directories::ProjectDirs::from("com", "spacesaver", "Space-Saver")
            .map(|dirs| dirs.data_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));

        Self {
            database_path: data_dir.join("spacesaver.db"),
            cache_dir: data_dir.join("cache"),
            log_level: "info".to_string(),
            max_concurrent_tasks: 4,
            hash_algorithm: HashAlgorithm::Blake3,
            image_similarity_threshold: 0.9,
            scan: ScanConfig::default(),
        }
    }
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            follow_links: false,
            max_depth: None,
            min_file_size: 0,
            exclude_patterns: vec![
                "*.tmp".to_string(),
                "*.cache".to_string(),
                ".git/*".to_string(),
                "node_modules/*".to_string(),
            ],
        }
    }
}

impl Config {
    /// Load configuration from a file
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to a file
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        fs::write(path, content)?;
        Ok(())
    }

    /// Get default config file path
    pub fn default_path() -> PathBuf {
        directories::ProjectDirs::from("com", "spacesaver", "Space-Saver")
            .map(|dirs| dirs.config_dir().join("config.toml"))
            .unwrap_or_else(|| PathBuf::from("config.toml"))
    }

    /// Load or create default configuration
    pub fn load_or_default() -> Self {
        let path = Self::default_path();
        
        if path.exists() {
            Self::load(&path).unwrap_or_default()
        } else {
            let config = Self::default();
            let _ = config.save(&path);
            config
        }
    }

    /// Ensure directories exist
    pub fn ensure_directories(&self) -> Result<()> {
        if let Some(parent) = self.database_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::create_dir_all(&self.cache_dir)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.log_level, "info");
        assert_eq!(config.max_concurrent_tasks, 4);
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        
        let config = Config::default();
        config.save(&config_path).unwrap();
        
        let loaded = Config::load(&config_path).unwrap();
        assert_eq!(loaded.log_level, config.log_level);
    }

    #[test]
    fn test_scan_config_default() {
        let scan = ScanConfig::default();
        assert!(!scan.follow_links);
        assert!(scan.exclude_patterns.len() > 0);
    }
}
