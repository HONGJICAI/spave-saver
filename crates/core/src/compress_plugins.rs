use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Result of a compression operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionResult {
    pub original_size: u64,
    pub compressed_size: u64,
    pub output_path: PathBuf,
    pub plugin_name: String,
    pub files_processed: usize,
    pub backup_path: Option<PathBuf>,
}

/// Metadata about a compression plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub name: String,
    pub description: String,
    pub version: String,
}

/// Trait that all compression plugins must implement
pub trait CompressionPlugin: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> PluginMetadata;

    /// Check if this plugin can handle the given file
    /// Returns `(can_handle: bool, reason: Option<String>)`
    /// The reason should explain why the file can or cannot be handled
    fn can_handle(&self, path: &Path) -> Result<(bool, Option<String>)>;

    /// Estimate the potential compression ratio (0.0 to 1.0)
    /// Returns None if estimation is not possible
    fn estimate_ratio(&self, _path: &Path) -> Result<Option<f32>> {
        Ok(None)
    }

    /// Perform the compression/conversion
    fn process(&self, source: &Path, output_dir: &Path) -> Result<CompressionResult>;

    /// Get supported file extensions (e.g., ["png", "jpg", "jpeg"])
    fn supported_extensions(&self) -> Vec<&str>;
}

/// Plugin registry and manager
pub struct PluginManager {
    plugins: Vec<Box<dyn CompressionPlugin>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// Register a plugin
    pub fn register(&mut self, plugin: Box<dyn CompressionPlugin>) {
        self.plugins.push(plugin);
    }

    /// Get all registered plugins
    pub fn get_plugins(&self) -> Vec<PluginMetadata> {
        self.plugins.iter().map(|p| p.metadata()).collect()
    }

    /// Find the best plugin for a file
    pub fn find_plugin(&self, path: &Path) -> Result<Option<&dyn CompressionPlugin>> {
        for plugin in &self.plugins {
            let (can_handle, _reason) = plugin.can_handle(path)?;
            if can_handle {
                return Ok(Some(plugin.as_ref()));
            }
        }
        Ok(None)
    }

    /// Find all plugins that can handle a file
    pub fn find_all_plugins(&self, path: &Path) -> Result<Vec<&dyn CompressionPlugin>> {
        let mut suitable_plugins = Vec::new();
        for plugin in &self.plugins {
            let (can_handle, _reason) = plugin.can_handle(path)?;
            if can_handle {
                suitable_plugins.push(plugin.as_ref());
            }
        }
        Ok(suitable_plugins)
    }

    /// Get all plugins that support a specific file extension
    ///
    /// # Arguments
    /// * `extension` - The file extension to search for (without the dot, e.g., "png", "jpg")
    ///
    /// # Returns
    /// A vector of plugin metadata for plugins that support the extension
    ///
    /// # Example
    /// ```
    /// use space_saver_core::compress_plugins::PluginManager;
    ///
    /// let manager = PluginManager::new();
    /// let png_plugins = manager.get_plugins_by_extension("png");
    /// for plugin in png_plugins {
    ///     println!("Plugin {} supports PNG files", plugin.name);
    /// }
    /// ```
    pub fn get_plugins_by_extension(&self, extension: &str) -> Vec<PluginMetadata> {
        self.plugins
            .iter()
            .filter(|plugin| {
                plugin
                    .supported_extensions()
                    .iter()
                    .any(|ext| ext.eq_ignore_ascii_case(extension))
            })
            .map(|plugin| plugin.metadata())
            .collect()
    }

    /// Get supported extensions for a specific plugin by name
    ///
    /// # Arguments
    /// * `plugin_name` - The name of the plugin
    ///
    /// # Returns
    /// A vector of supported file extensions (without the dot)
    pub fn get_supported_extensions(&self, plugin_name: &str) -> Vec<String> {
        self.plugins
            .iter()
            .find(|plugin| plugin.metadata().name == plugin_name)
            .map(|plugin| {
                plugin
                    .supported_extensions()
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Process a file with the best available plugin
    ///
    /// If `plugin_orders` is provided, plugins will be tried in that order.
    /// Otherwise, plugins are tried in registration order.
    pub fn process_file(
        &self,
        source: &Path,
        output_dir: &Path,
        plugin_orders: Option<&[String]>,
    ) -> Result<CompressionResult> {
        if let Some(orders) = plugin_orders {
            // Try plugins in the specified order
            for plugin_name in orders {
                if let Some(plugin) = self
                    .plugins
                    .iter()
                    .find(|p| &p.metadata().name == plugin_name)
                {
                    let (can_handle, _reason) = plugin.can_handle(source)?;
                    if can_handle {
                        return plugin.process(source, output_dir);
                    }
                }
            }
            // If no ordered plugin matched, fall back to default behavior
        }

        // Default behavior: use first available plugin
        let plugin = self
            .find_plugin(source)?
            .ok_or_else(|| anyhow!("No suitable plugin found for file: {}", source.display()))?;

        plugin.process(source, output_dir)
    }

    /// Process a file with a specific plugin by name
    pub fn process_with_plugin(
        &self,
        source: &Path,
        output_dir: &Path,
        plugin_name: &str,
    ) -> Result<CompressionResult> {
        let plugin = self
            .plugins
            .iter()
            .find(|p| p.metadata().name == plugin_name)
            .ok_or_else(|| anyhow!("Plugin not found: {}", plugin_name))?;

        let (can_handle, reason) = plugin.can_handle(source)?;
        if !can_handle {
            let reason_msg = reason.unwrap_or_else(|| "Unknown reason".to_string());
            return Err(anyhow!(
                "Plugin '{}' cannot handle file: {} (Reason: {})",
                plugin_name,
                source.display(),
                reason_msg
            ));
        }

        plugin.process(source, output_dir)
    }

    /// Check if a specific plugin can handle a file and get the reason
    /// Returns (plugin_metadata, can_handle, reason, estimate_ratio)
    #[allow(clippy::type_complexity)]
    pub fn check_plugin_capability(
        &self,
        path: &Path,
        plugin_name: &str,
    ) -> Result<Option<(PluginMetadata, bool, Option<String>, Option<f32>)>> {
        let plugin = self
            .plugins
            .iter()
            .find(|p| p.metadata().name == plugin_name);

        if let Some(plugin) = plugin {
            let metadata = plugin.metadata();
            let (can_handle, reason) = plugin.can_handle(path)?;
            let estimate_ratio = if can_handle {
                plugin.estimate_ratio(path).ok().flatten()
            } else {
                None
            };

            Ok(Some((metadata, can_handle, reason, estimate_ratio)))
        } else {
            Ok(None)
        }
    }

    /// Batch process multiple files
    pub fn process_batch(
        &self,
        sources: &[PathBuf],
        output_dir: &Path,
        plugin_orders: Option<&[String]>,
    ) -> Result<Vec<Result<CompressionResult>>> {
        fs::create_dir_all(output_dir)?;

        let results: Vec<Result<CompressionResult>> = sources
            .iter()
            .map(|source| self.process_file(source, output_dir, plugin_orders))
            .collect();

        Ok(results)
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Global plugin manager instance
static GLOBAL_PLUGIN_MANAGER: Lazy<Arc<RwLock<PluginManager>>> = Lazy::new(|| {
    let mut manager = PluginManager::new();

    // Register default plugins
    use crate::plugins::{
        AnimatedWebPConverterPlugin, ImageZipToWebpZipPlugin, WebPConverterPlugin,
    };
    manager.register(Box::new(ImageZipToWebpZipPlugin::new()));
    manager.register(Box::new(WebPConverterPlugin::new()));
    manager.register(Box::new(AnimatedWebPConverterPlugin));

    Arc::new(RwLock::new(manager))
});

/// Get the global plugin manager instance
pub fn global_plugin_manager() -> Arc<RwLock<PluginManager>> {
    Arc::clone(&GLOBAL_PLUGIN_MANAGER)
}

/// Initialize the global plugin manager with custom plugins (for testing)
/// This will replace all existing plugins
pub fn init_plugin_manager_with(
    plugins: Vec<Box<dyn CompressionPlugin>>,
) -> Arc<RwLock<PluginManager>> {
    let mut manager = PluginManager::new();
    for plugin in plugins {
        manager.register(plugin);
    }
    Arc::new(RwLock::new(manager))
}

/// Helper function to check if file has one of the given extensions
pub fn has_extension(path: &Path, extensions: &[&str]) -> bool {
    if let Some(ext) = path.extension() {
        if let Some(ext_str) = ext.to_str() {
            return extensions.iter().any(|e| e.eq_ignore_ascii_case(ext_str));
        }
    }
    false
}

/// Helper to get file size
pub fn get_file_size(path: &Path) -> Result<u64> {
    Ok(fs::metadata(path)?.len())
}

/// Helper to generate output filename with new extension
pub fn generate_output_filename(source: &Path, new_ext: &str) -> PathBuf {
    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    PathBuf::from(format!("{}.{}", stem, new_ext))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockPlugin {
        name: String,
        extensions: Vec<String>,
    }

    impl CompressionPlugin for MockPlugin {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata {
                name: self.name.clone(),
                description: "Mock plugin".to_string(),
                version: "1.0.0".to_string(),
            }
        }

        fn can_handle(&self, path: &Path) -> Result<(bool, Option<String>)> {
            let can_handle = has_extension(
                path,
                &self
                    .extensions
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>(),
            );
            let reason = if can_handle {
                Some("File extension matches supported extensions".to_string())
            } else {
                Some("File extension not supported by this plugin".to_string())
            };
            Ok((can_handle, reason))
        }

        fn process(&self, source: &Path, _output_dir: &Path) -> Result<CompressionResult> {
            Ok(CompressionResult {
                original_size: 1000,
                compressed_size: 500,
                output_path: source.to_path_buf(),
                plugin_name: self.name.clone(),
                files_processed: 1,
                backup_path: None,
            })
        }

        fn supported_extensions(&self) -> Vec<&str> {
            self.extensions.iter().map(|s| s.as_str()).collect()
        }
    }

    #[test]
    fn test_plugin_registration() {
        let mut manager = PluginManager::new();

        manager.register(Box::new(MockPlugin {
            name: "Plugin1".to_string(),
            extensions: vec!["txt".to_string()],
        }));

        manager.register(Box::new(MockPlugin {
            name: "Plugin2".to_string(),
            extensions: vec!["txt".to_string()],
        }));

        let path = Path::new("test.txt");
        let plugin = manager.find_plugin(path).unwrap().unwrap();
        // Should return first registered plugin that can handle
        assert_eq!(plugin.metadata().name, "Plugin1");
    }

    #[test]
    fn test_global_plugin_manager() {
        // Test that global manager is initialized
        let manager = global_plugin_manager();
        let manager = manager.read().unwrap();
        let plugins = manager.get_plugins();

        // Should have all 3 default plugins
        assert_eq!(plugins.len(), 3);

        // Check plugin names
        let plugin_names: Vec<_> = plugins.iter().map(|p| p.name.as_str()).collect();
        assert!(plugin_names.contains(&"Image ZIP to WebP ZIP"));
        assert!(plugin_names.contains(&"WebP Converter"));
        assert!(plugin_names.contains(&"Animated WebP Converter"));
    }

    #[test]
    fn test_custom_plugin_manager_for_testing() {
        // Example: Create a custom manager with mock plugins for testing
        let mock_plugins: Vec<Box<dyn CompressionPlugin>> = vec![Box::new(MockPlugin {
            name: "Test Plugin".to_string(),
            extensions: vec!["test".to_string()],
        })];

        let manager = init_plugin_manager_with(mock_plugins);
        let manager = manager.read().unwrap();
        let plugins = manager.get_plugins();

        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "Test Plugin");
    }

    #[test]
    fn test_plugin_orders() {
        let mut manager = PluginManager::new();

        // Register two plugins
        manager.register(Box::new(MockPlugin {
            name: "Plugin1".to_string(),
            extensions: vec!["txt".to_string()],
        }));

        manager.register(Box::new(MockPlugin {
            name: "Plugin2".to_string(),
            extensions: vec!["txt".to_string()],
        }));

        let path = Path::new("test.txt");
        let temp_dir = std::env::temp_dir();

        // Without plugin_orders, should use first registered plugin
        let result = manager.process_file(path, &temp_dir, None).unwrap();
        assert_eq!(result.plugin_name, "Plugin1");

        // With plugin_orders, should use specified order
        let orders = vec!["Plugin2".to_string()];
        let result = manager
            .process_file(path, &temp_dir, Some(&orders))
            .unwrap();
        assert_eq!(result.plugin_name, "Plugin2");

        // If specified plugin doesn't match, should fall back to first available
        let orders = vec!["Nonexistent Plugin".to_string()];
        let result = manager
            .process_file(path, &temp_dir, Some(&orders))
            .unwrap();
        assert_eq!(result.plugin_name, "Plugin1");
    }

    #[test]
    fn test_get_plugins_by_extension() {
        let mut manager = PluginManager::new();

        // Register plugins with different extensions
        manager.register(Box::new(MockPlugin {
            name: "PNG Handler".to_string(),
            extensions: vec!["png".to_string(), "bmp".to_string()],
        }));

        manager.register(Box::new(MockPlugin {
            name: "JPEG Handler".to_string(),
            extensions: vec!["jpg".to_string(), "jpeg".to_string()],
        }));

        manager.register(Box::new(MockPlugin {
            name: "Multi Format Handler".to_string(),
            extensions: vec!["png".to_string(), "jpg".to_string(), "gif".to_string()],
        }));

        // Test finding plugins by extension
        let png_plugins = manager.get_plugins_by_extension("png");
        assert_eq!(png_plugins.len(), 2);
        let png_names: Vec<_> = png_plugins.iter().map(|p| p.name.as_str()).collect();
        assert!(png_names.contains(&"PNG Handler"));
        assert!(png_names.contains(&"Multi Format Handler"));

        let jpg_plugins = manager.get_plugins_by_extension("jpg");
        assert_eq!(jpg_plugins.len(), 2);
        let jpg_names: Vec<_> = jpg_plugins.iter().map(|p| p.name.as_str()).collect();
        assert!(jpg_names.contains(&"JPEG Handler"));
        assert!(jpg_names.contains(&"Multi Format Handler"));

        let jpeg_plugins = manager.get_plugins_by_extension("jpeg");
        assert_eq!(jpeg_plugins.len(), 1);
        assert_eq!(jpeg_plugins[0].name, "JPEG Handler");

        // Test case-insensitive matching
        let png_plugins_upper = manager.get_plugins_by_extension("PNG");
        assert_eq!(png_plugins_upper.len(), 2);

        // Test non-existent extension
        let unknown_plugins = manager.get_plugins_by_extension("xyz");
        assert_eq!(unknown_plugins.len(), 0);
    }
}
