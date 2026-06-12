use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tracing::warn;

/// Result of a compression operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionResult {
    pub original_size: u64,
    pub compressed_size: u64,
    pub output_path: PathBuf,
    pub plugin_name: String,
    pub files_processed: usize,
    /// Set by the manager after it backs up the original file
    pub backup_path: Option<PathBuf>,
    /// When true, the manager moves the output over the source path after
    /// backing up the original (e.g. ZIP-to-ZIP conversion keeps the name)
    #[serde(default)]
    pub replace_source: bool,
}

/// Outcome of running a plugin through the manager
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CompressionOutcome {
    /// The file was compressed; the original was renamed to `backup_path`
    Compressed(CompressionResult),
    /// The plugin ran but the output was not smaller; the original was kept untouched
    Skipped { plugin_name: String, reason: String },
}

/// Metadata about a compression plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub name: String,
    pub description: String,
    pub version: String,
}

/// Trait that all compression plugins must implement
///
/// Plugins must NOT delete, rename, or otherwise modify the source file.
/// They only read the source and write a new file into `output_dir`.
/// Backups, size comparison, and in-place replacement are handled by
/// [`PluginManager`].
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

    /// Perform the compression/conversion, writing the result into `output_dir`
    fn process(&self, source: &Path, output_dir: &Path) -> Result<CompressionResult>;

    /// Get supported file extensions (e.g., ["png", "jpg", "jpeg"])
    fn supported_extensions(&self) -> Vec<&str>;

    /// Current quality setting (0-100), or None if not applicable
    fn quality(&self) -> Option<f32> {
        None
    }

    /// Update the quality setting; returns false if the plugin has no such setting
    fn set_quality(&mut self, _quality: f32) -> bool {
        false
    }
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

    /// Get the quality setting of a plugin, if it has one
    pub fn get_plugin_quality(&self, plugin_name: &str) -> Option<f32> {
        self.plugins
            .iter()
            .find(|p| p.metadata().name == plugin_name)
            .and_then(|p| p.quality())
    }

    /// Set the quality of a plugin (0-100)
    pub fn set_plugin_quality(&mut self, plugin_name: &str, quality: f32) -> Result<()> {
        let plugin = self
            .plugins
            .iter_mut()
            .find(|p| p.metadata().name == plugin_name)
            .ok_or_else(|| anyhow!("Plugin not found: {}", plugin_name))?;

        if plugin.set_quality(quality.clamp(0.0, 100.0)) {
            Ok(())
        } else {
            Err(anyhow!(
                "Plugin '{}' does not support a quality setting",
                plugin_name
            ))
        }
    }

    /// Process a file with the best available plugin.
    ///
    /// If `plugin_orders` is provided, ONLY those plugins are considered, in
    /// that order. If none of them can handle the file, an error is returned
    /// (a plugin the caller did not list is never used).
    ///
    /// When `keep_backup` is false, the original is still renamed aside during
    /// processing (so a failure can never lose it), but it is deleted once the
    /// compression has fully succeeded and `backup_path` will be None.
    pub fn process_file(
        &self,
        source: &Path,
        output_dir: &Path,
        plugin_orders: Option<&[String]>,
        keep_backup: bool,
    ) -> Result<CompressionOutcome> {
        let plugin = match plugin_orders {
            Some(orders) => {
                let mut selected = None;
                for plugin_name in orders {
                    if let Some(plugin) = self
                        .plugins
                        .iter()
                        .find(|p| &p.metadata().name == plugin_name)
                    {
                        let (can_handle, _reason) = plugin.can_handle(source)?;
                        if can_handle {
                            selected = Some(plugin.as_ref());
                            break;
                        }
                    }
                }
                selected.ok_or_else(|| {
                    anyhow!(
                        "No active plugin can handle file: {}",
                        source.display()
                    )
                })?
            }
            None => self.find_plugin(source)?.ok_or_else(|| {
                anyhow!("No suitable plugin found for file: {}", source.display())
            })?,
        };

        self.execute_plugin(plugin, source, output_dir, keep_backup)
    }

    /// Process a file with a specific plugin by name
    pub fn process_with_plugin(
        &self,
        source: &Path,
        output_dir: &Path,
        plugin_name: &str,
        keep_backup: bool,
    ) -> Result<CompressionOutcome> {
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

        self.execute_plugin(plugin.as_ref(), source, output_dir, keep_backup)
    }

    /// Run a plugin and apply the shared backup / size-check / replace logic:
    /// 1. The plugin writes its output into `output_dir` (source untouched).
    /// 2. If the output is not smaller, it is deleted and the file is skipped.
    /// 3. Otherwise the original is renamed to `<name>.bak` (the backup), and
    ///    if the plugin requested `replace_source`, the output takes over the
    ///    original path.
    /// 4. With `keep_backup` false, the backup is deleted only after every
    ///    step above succeeded, so a failure can never lose the original.
    fn execute_plugin(
        &self,
        plugin: &dyn CompressionPlugin,
        source: &Path,
        output_dir: &Path,
        keep_backup: bool,
    ) -> Result<CompressionOutcome> {
        let mut result = plugin.process(source, output_dir)?;

        if result.compressed_size >= result.original_size {
            if result.output_path != source {
                let _ = fs::remove_file(&result.output_path);
            }
            return Ok(CompressionOutcome::Skipped {
                plugin_name: result.plugin_name,
                reason: format!(
                    "Compressed output ({} bytes) is not smaller than the original ({} bytes); original kept",
                    result.compressed_size, result.original_size
                ),
            });
        }

        let backup_path = backup_path_for(source);
        if let Err(e) = fs::rename(source, &backup_path) {
            let _ = fs::remove_file(&result.output_path);
            return Err(anyhow!(
                "Failed to back up original file {}: {}",
                source.display(),
                e
            ));
        }

        if result.replace_source {
            if let Err(e) = fs::rename(&result.output_path, source) {
                // Restore the original so the user is never left without the file
                let _ = fs::remove_file(&result.output_path);
                let _ = fs::rename(&backup_path, source);
                return Err(anyhow!(
                    "Failed to move compressed output over {}: {}",
                    source.display(),
                    e
                ));
            }
            result.output_path = source.to_path_buf();
        }

        if keep_backup {
            result.backup_path = Some(backup_path);
        } else {
            // Compression fully succeeded; the user opted out of backups
            match fs::remove_file(&backup_path) {
                Ok(()) => result.backup_path = None,
                Err(e) => {
                    // Keep the backup rather than fail a successful compression
                    warn!(
                        backup = %backup_path.display(),
                        error = %e,
                        "Failed to remove backup after compression; keeping it"
                    );
                    result.backup_path = Some(backup_path);
                }
            }
        }
        Ok(CompressionOutcome::Compressed(result))
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
        keep_backup: bool,
    ) -> Result<Vec<Result<CompressionOutcome>>> {
        fs::create_dir_all(output_dir)?;

        let results: Vec<Result<CompressionOutcome>> = sources
            .iter()
            .map(|source| self.process_file(source, output_dir, plugin_orders, keep_backup))
            .collect();

        Ok(results)
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Pick a backup path next to the source that does not exist yet:
/// `foo.png` -> `foo.png.bak`, then `foo.png.bak.1`, `foo.png.bak.2`, ...
fn backup_path_for(source: &Path) -> PathBuf {
    let file_name = source
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());
    let parent = source.parent().unwrap_or_else(|| Path::new(""));

    let mut candidate = parent.join(format!("{}.bak", file_name));
    let mut counter = 1;
    while candidate.exists() {
        candidate = parent.join(format!("{}.bak.{}", file_name, counter));
        counter += 1;
    }
    candidate
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
    manager.register(Box::new(AnimatedWebPConverterPlugin::new()));

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

/// Atomically create a new output file, failing if it already exists.
/// Check and creation are a single syscall (O_EXCL), so two concurrent
/// writers can never silently overwrite each other's output — e.g. when
/// `photo.jpg` and `photo.png` in the same directory both target
/// `photo.webp`, the second one fails cleanly instead.
pub fn create_output_file(path: &Path) -> Result<fs::File> {
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                anyhow!("Output file already exists: {}", path.display())
            } else {
                anyhow!("Failed to create output file {}: {}", path.display(), e)
            }
        })
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
        /// Bytes written as the "compressed" output file
        output_content: Vec<u8>,
        replace_source: bool,
        quality: Option<f32>,
    }

    impl MockPlugin {
        fn new(name: &str, extensions: &[&str]) -> Self {
            Self {
                name: name.to_string(),
                extensions: extensions.iter().map(|s| s.to_string()).collect(),
                output_content: b"c".to_vec(),
                replace_source: false,
                quality: None,
            }
        }
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

        fn process(&self, source: &Path, output_dir: &Path) -> Result<CompressionResult> {
            let original_size = get_file_size(source)?;
            let output_path = output_dir.join(generate_output_filename(source, "mock"));
            fs::write(&output_path, &self.output_content)?;

            Ok(CompressionResult {
                original_size,
                compressed_size: self.output_content.len() as u64,
                output_path,
                plugin_name: self.name.clone(),
                files_processed: 1,
                backup_path: None,
                replace_source: self.replace_source,
            })
        }

        fn supported_extensions(&self) -> Vec<&str> {
            self.extensions.iter().map(|s| s.as_str()).collect()
        }

        fn quality(&self) -> Option<f32> {
            self.quality
        }

        fn set_quality(&mut self, quality: f32) -> bool {
            if self.quality.is_some() {
                self.quality = Some(quality);
                true
            } else {
                false
            }
        }
    }

    fn temp_source(dir: &Path, name: &str, content: &[u8]) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_plugin_registration() {
        let mut manager = PluginManager::new();
        manager.register(Box::new(MockPlugin::new("Plugin1", &["txt"])));
        manager.register(Box::new(MockPlugin::new("Plugin2", &["txt"])));

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
        let mock_plugins: Vec<Box<dyn CompressionPlugin>> =
            vec![Box::new(MockPlugin::new("Test Plugin", &["test"]))];

        let manager = init_plugin_manager_with(mock_plugins);
        let manager = manager.read().unwrap();
        let plugins = manager.get_plugins();

        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "Test Plugin");
    }

    #[test]
    fn test_process_creates_backup_and_keeps_output() {
        let dir = tempfile::tempdir().unwrap();
        let source = temp_source(dir.path(), "test.txt", b"original content");

        let mut manager = PluginManager::new();
        manager.register(Box::new(MockPlugin::new("Plugin1", &["txt"])));

        let outcome = manager.process_file(&source, dir.path(), None, true).unwrap();
        match outcome {
            CompressionOutcome::Compressed(result) => {
                let backup = result.backup_path.expect("backup path must be set");
                assert!(backup.exists(), "backup file must exist");
                assert_eq!(fs::read(&backup).unwrap(), b"original content");
                assert!(!source.exists(), "source was renamed to backup");
                assert!(result.output_path.exists(), "output must exist");
            }
            other => panic!("expected Compressed, got {:?}", other),
        }
    }

    #[test]
    fn test_process_skips_when_output_not_smaller() {
        let dir = tempfile::tempdir().unwrap();
        let source = temp_source(dir.path(), "small.txt", b"x");

        let mut plugin = MockPlugin::new("Plugin1", &["txt"]);
        plugin.output_content = b"way bigger than the original".to_vec();

        let mut manager = PluginManager::new();
        manager.register(Box::new(plugin));

        let outcome = manager.process_file(&source, dir.path(), None, true).unwrap();
        match outcome {
            CompressionOutcome::Skipped { plugin_name, .. } => {
                assert_eq!(plugin_name, "Plugin1");
                assert!(source.exists(), "original must be kept untouched");
                assert!(
                    !dir.path().join("small.mock").exists(),
                    "larger output must be removed"
                );
            }
            other => panic!("expected Skipped, got {:?}", other),
        }
    }

    #[test]
    fn test_replace_source_takes_over_original_path() {
        let dir = tempfile::tempdir().unwrap();
        let source = temp_source(dir.path(), "archive.zip", b"original zip content");

        let mut plugin = MockPlugin::new("ZipPlugin", &["zip"]);
        plugin.replace_source = true;

        let mut manager = PluginManager::new();
        manager.register(Box::new(plugin));

        let outcome = manager.process_file(&source, dir.path(), None, true).unwrap();
        match outcome {
            CompressionOutcome::Compressed(result) => {
                assert_eq!(result.output_path, source);
                assert_eq!(fs::read(&source).unwrap(), b"c");
                let backup = result.backup_path.unwrap();
                assert_eq!(fs::read(backup).unwrap(), b"original zip content");
            }
            other => panic!("expected Compressed, got {:?}", other),
        }
    }

    #[test]
    fn test_backup_does_not_overwrite_existing_backup() {
        let dir = tempfile::tempdir().unwrap();
        let source = temp_source(dir.path(), "test.txt", b"original content");
        temp_source(dir.path(), "test.txt.bak", b"older backup");

        let mut manager = PluginManager::new();
        manager.register(Box::new(MockPlugin::new("Plugin1", &["txt"])));

        let outcome = manager.process_file(&source, dir.path(), None, true).unwrap();
        match outcome {
            CompressionOutcome::Compressed(result) => {
                let backup = result.backup_path.unwrap();
                assert_eq!(backup, dir.path().join("test.txt.bak.1"));
                assert_eq!(
                    fs::read(dir.path().join("test.txt.bak")).unwrap(),
                    b"older backup"
                );
            }
            other => panic!("expected Compressed, got {:?}", other),
        }
    }

    #[test]
    fn test_process_without_backup_removes_original() {
        let dir = tempfile::tempdir().unwrap();
        let source = temp_source(dir.path(), "test.txt", b"original content");

        let mut manager = PluginManager::new();
        manager.register(Box::new(MockPlugin::new("Plugin1", &["txt"])));

        let outcome = manager.process_file(&source, dir.path(), None, false).unwrap();
        match outcome {
            CompressionOutcome::Compressed(result) => {
                assert!(result.backup_path.is_none(), "no backup path when disabled");
                assert!(!source.exists(), "original removed after success");
                assert!(
                    !dir.path().join("test.txt.bak").exists(),
                    "no .bak file left behind"
                );
                assert!(result.output_path.exists());
            }
            other => panic!("expected Compressed, got {:?}", other),
        }
    }

    #[test]
    fn test_process_without_backup_keeps_original_on_skip() {
        let dir = tempfile::tempdir().unwrap();
        let source = temp_source(dir.path(), "small.txt", b"x");

        let mut plugin = MockPlugin::new("Plugin1", &["txt"]);
        plugin.output_content = b"way bigger than the original".to_vec();

        let mut manager = PluginManager::new();
        manager.register(Box::new(plugin));

        // Even with backups disabled, a skip must never touch the original
        let outcome = manager.process_file(&source, dir.path(), None, false).unwrap();
        assert!(matches!(outcome, CompressionOutcome::Skipped { .. }));
        assert_eq!(fs::read(&source).unwrap(), b"x");
    }

    #[test]
    fn test_replace_source_without_backup() {
        let dir = tempfile::tempdir().unwrap();
        let source = temp_source(dir.path(), "archive.zip", b"original zip content");

        let mut plugin = MockPlugin::new("ZipPlugin", &["zip"]);
        plugin.replace_source = true;

        let mut manager = PluginManager::new();
        manager.register(Box::new(plugin));

        let outcome = manager.process_file(&source, dir.path(), None, false).unwrap();
        match outcome {
            CompressionOutcome::Compressed(result) => {
                assert_eq!(result.output_path, source);
                assert_eq!(fs::read(&source).unwrap(), b"c");
                assert!(result.backup_path.is_none());
                assert!(!dir.path().join("archive.zip.bak").exists());
            }
            other => panic!("expected Compressed, got {:?}", other),
        }
    }

    #[test]
    fn test_plugin_orders() {
        let dir = tempfile::tempdir().unwrap();

        let mut manager = PluginManager::new();
        manager.register(Box::new(MockPlugin::new("Plugin1", &["txt"])));
        manager.register(Box::new(MockPlugin::new("Plugin2", &["txt"])));

        // Without plugin_orders, should use first registered plugin
        let source = temp_source(dir.path(), "a.txt", b"original content");
        match manager.process_file(&source, dir.path(), None, true).unwrap() {
            CompressionOutcome::Compressed(result) => assert_eq!(result.plugin_name, "Plugin1"),
            other => panic!("expected Compressed, got {:?}", other),
        }

        // With plugin_orders, should use specified order
        let source = temp_source(dir.path(), "b.txt", b"original content");
        let orders = vec!["Plugin2".to_string()];
        match manager
            .process_file(&source, dir.path(), Some(&orders), true)
            .unwrap()
        {
            CompressionOutcome::Compressed(result) => assert_eq!(result.plugin_name, "Plugin2"),
            other => panic!("expected Compressed, got {:?}", other),
        }
    }

    #[test]
    fn test_plugin_orders_never_falls_back_to_unlisted_plugins() {
        let dir = tempfile::tempdir().unwrap();
        let source = temp_source(dir.path(), "test.txt", b"original content");

        let mut manager = PluginManager::new();
        manager.register(Box::new(MockPlugin::new("Plugin1", &["txt"])));

        // Plugin1 could handle the file, but it is not in the orders list,
        // so it must NOT be used (the user deactivated it)
        let orders = vec!["Nonexistent Plugin".to_string()];
        let result = manager.process_file(&source, dir.path(), Some(&orders), true);
        assert!(result.is_err());
        assert!(source.exists(), "source must be untouched");
    }

    #[test]
    fn test_plugin_quality() {
        let mut manager = PluginManager::new();

        let mut with_quality = MockPlugin::new("Quality Plugin", &["txt"]);
        with_quality.quality = Some(85.0);
        manager.register(Box::new(with_quality));
        manager.register(Box::new(MockPlugin::new("No Quality Plugin", &["txt"])));

        assert_eq!(manager.get_plugin_quality("Quality Plugin"), Some(85.0));
        assert_eq!(manager.get_plugin_quality("No Quality Plugin"), None);

        manager.set_plugin_quality("Quality Plugin", 60.0).unwrap();
        assert_eq!(manager.get_plugin_quality("Quality Plugin"), Some(60.0));

        // Out-of-range values are clamped
        manager.set_plugin_quality("Quality Plugin", 150.0).unwrap();
        assert_eq!(manager.get_plugin_quality("Quality Plugin"), Some(100.0));

        assert!(manager
            .set_plugin_quality("No Quality Plugin", 60.0)
            .is_err());
        assert!(manager.set_plugin_quality("Missing Plugin", 60.0).is_err());
    }

    #[test]
    fn test_get_plugins_by_extension() {
        let mut manager = PluginManager::new();
        manager.register(Box::new(MockPlugin::new("PNG Handler", &["png", "bmp"])));
        manager.register(Box::new(MockPlugin::new("JPEG Handler", &["jpg", "jpeg"])));
        manager.register(Box::new(MockPlugin::new(
            "Multi Format Handler",
            &["png", "jpg", "gif"],
        )));

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
