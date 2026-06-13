use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

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

    /// Default delete mode for delete actions ("trash" or "permanent").
    /// Consumed by the frontend as the default for delete dialogs.
    #[serde(default = "default_delete_mode")]
    pub default_delete_mode: String,

    /// Whether in-place compression keeps a `.bak` of the original by default.
    /// Consumed by the frontend as the default for the compress confirm step.
    #[serde(default = "default_compress_backup")]
    pub default_compress_backup: bool,

    /// Scan settings
    pub scan: ScanConfig,
}

fn default_delete_mode() -> String {
    "trash".to_string()
}

fn default_compress_backup() -> bool {
    true
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
        let _config_dir = directories::ProjectDirs::from("com", "spacesaver", "Space-Saver")
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
            default_delete_mode: default_delete_mode(),
            default_compress_backup: default_compress_backup(),
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

    /// Validate the configuration, rejecting values the app cannot honour.
    /// Called before persisting an edited config so bad input fails loudly
    /// instead of silently corrupting behaviour.
    pub fn validate(&self) -> Result<()> {
        if !(0.0..=1.0).contains(&self.image_similarity_threshold) {
            anyhow::bail!(
                "image_similarity_threshold must be between 0.0 and 1.0, got {}",
                self.image_similarity_threshold
            );
        }
        if self.max_concurrent_tasks == 0 {
            anyhow::bail!("max_concurrent_tasks must be at least 1");
        }
        const LEVELS: [&str; 5] = ["error", "warn", "info", "debug", "trace"];
        if !LEVELS.contains(&self.log_level.as_str()) {
            anyhow::bail!(
                "log_level must be one of error, warn, info, debug, trace, got '{}'",
                self.log_level
            );
        }
        if self.default_delete_mode != "trash" && self.default_delete_mode != "permanent" {
            anyhow::bail!(
                "default_delete_mode must be 'trash' or 'permanent', got '{}'",
                self.default_delete_mode
            );
        }
        Ok(())
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
        assert!(!scan.exclude_patterns.is_empty());
    }

    #[test]
    fn test_new_defaults() {
        let config = Config::default();
        assert_eq!(config.default_delete_mode, "trash");
        assert!(config.default_compress_backup);
    }

    #[test]
    fn test_validate_accepts_default() {
        assert!(Config::default().validate().is_ok());
    }

    #[test]
    fn test_validate_rejects_out_of_range_threshold() {
        let high = Config {
            image_similarity_threshold: 1.5,
            ..Default::default()
        };
        assert!(high.validate().is_err());
        let low = Config {
            image_similarity_threshold: -0.1,
            ..Default::default()
        };
        assert!(low.validate().is_err());
    }

    #[test]
    fn test_validate_threshold_boundaries() {
        let zero = Config {
            image_similarity_threshold: 0.0,
            ..Default::default()
        };
        assert!(zero.validate().is_ok());
        let one = Config {
            image_similarity_threshold: 1.0,
            ..Default::default()
        };
        assert!(one.validate().is_ok());
    }

    #[test]
    fn test_validate_rejects_zero_concurrency() {
        let config = Config {
            max_concurrent_tasks: 0,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_rejects_unknown_log_level() {
        let config = Config {
            log_level: "verbose".to_string(),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_rejects_unknown_delete_mode() {
        let config = Config {
            default_delete_mode: "shred".to_string(),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_load_old_config_without_new_fields() {
        // A config file written before the new fields existed must still load,
        // falling back to the serde defaults rather than failing to parse.
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        let legacy = r#"
database_path = "/tmp/db.sqlite"
cache_dir = "/tmp/cache"
log_level = "info"
max_concurrent_tasks = 4
hash_algorithm = "Blake3"
image_similarity_threshold = 0.9

[scan]
follow_links = false
min_file_size = 0
exclude_patterns = ["*.tmp"]
"#;
        fs::write(&config_path, legacy).unwrap();

        let loaded = Config::load(&config_path).unwrap();
        assert_eq!(loaded.default_delete_mode, "trash");
        assert!(loaded.default_compress_backup);
    }
}
