use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use tracing::warn;

/// Cheap fingerprint of a file's state: one stat call, no content read.
/// Size + mtime is enough to detect changes for a personal tool; hashing
/// would re-read every file and defeat the purpose of the cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileFingerprint {
    pub size: u64,
    /// Modification time in seconds since UNIX epoch (matches scanner::FileInfo)
    pub mtime: i64,
}

impl FileFingerprint {
    pub fn of(path: &Path) -> Result<Self> {
        let meta = fs::metadata(path)?;
        Ok(Self::from_metadata(&meta))
    }

    pub fn from_metadata(meta: &fs::Metadata) -> Self {
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        Self {
            size: meta.len(),
            mtime,
        }
    }
}

/// One remembered "compression produced no size reduction" result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkipEntry {
    pub plugin_name: String,
    /// Plugin quality at the time of the attempt; a different quality
    /// invalidates the conclusion (the file may compress smaller now)
    pub quality: Option<f32>,
    pub fingerprint: FileFingerprint,
    /// Seconds since UNIX epoch when this was recorded
    pub recorded_at: i64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct SkipCacheData {
    version: u32,
    /// file path -> one entry per (plugin, quality) combination
    entries: HashMap<String, Vec<SkipEntry>>,
}

/// Remembers files that a plugin already failed to shrink, keyed by
/// (path, plugin, quality) and guarded by a size+mtime fingerprint, so the
/// next scan can exclude them instead of re-running the trial compression.
#[derive(Debug, Default)]
pub struct SkipCache {
    data: SkipCacheData,
    storage_path: Option<PathBuf>,
    dirty: bool,
}

impl SkipCache {
    /// Load the cache from `path`. A missing or unreadable file yields an
    /// empty cache (this is an optimization, never a hard dependency).
    pub fn load(path: PathBuf) -> Self {
        let data = match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|e| {
                warn!(path = %path.display(), error = %e, "Corrupt skip cache; starting empty");
                SkipCacheData::default()
            }),
            Err(_) => SkipCacheData::default(),
        };
        Self {
            data,
            storage_path: Some(path),
            dirty: false,
        }
    }

    /// In-memory cache without persistence (for tests)
    pub fn in_memory() -> Self {
        Self::default()
    }

    /// Persist to disk if anything changed. Atomic: writes a temp file in the
    /// same directory, then renames it over the target.
    pub fn save(&mut self) -> Result<()> {
        if !self.dirty {
            return Ok(());
        }
        let Some(path) = &self.storage_path else {
            return Ok(());
        };

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_vec_pretty(&self.data)?)?;
        fs::rename(&tmp, path)?;
        self.dirty = false;
        Ok(())
    }

    /// True if this exact file state was already tried by this plugin at this
    /// quality and produced no size reduction.
    pub fn is_known_skip(
        &self,
        path: &str,
        fingerprint: &FileFingerprint,
        plugin_name: &str,
        quality: Option<f32>,
    ) -> bool {
        self.data.entries.get(path).is_some_and(|entries| {
            entries.iter().any(|e| {
                e.plugin_name == plugin_name
                    && e.quality == quality
                    && e.fingerprint == *fingerprint
            })
        })
    }

    /// Look up the entry (regardless of fingerprint) for diagnostics
    pub fn entry_for(
        &self,
        path: &str,
        plugin_name: &str,
        quality: Option<f32>,
    ) -> Option<&SkipEntry> {
        self.data
            .entries
            .get(path)?
            .iter()
            .find(|e| e.plugin_name == plugin_name && e.quality == quality)
    }

    /// Record a no-size-reduction result, replacing any previous entry for
    /// the same (plugin, quality) pair.
    pub fn record_skip(
        &mut self,
        path: &str,
        fingerprint: FileFingerprint,
        plugin_name: &str,
        quality: Option<f32>,
    ) {
        let recorded_at = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let entries = self.data.entries.entry(path.to_string()).or_default();
        entries.retain(|e| !(e.plugin_name == plugin_name && e.quality == quality));
        entries.push(SkipEntry {
            plugin_name: plugin_name.to_string(),
            quality,
            fingerprint,
            recorded_at,
        });
        self.dirty = true;
    }

    /// Drop all entries for a path (e.g. the file was compressed or replaced)
    pub fn invalidate_path(&mut self, path: &str) {
        if self.data.entries.remove(path).is_some() {
            self.dirty = true;
        }
    }

    /// Number of remembered (path, plugin, quality) results
    pub fn len(&self) -> usize {
        self.data.entries.values().map(|v| v.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Forget everything; returns how many entries were removed
    pub fn clear(&mut self) -> usize {
        let removed = self.len();
        if removed > 0 {
            self.data.entries.clear();
            self.dirty = true;
        }
        removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fp(size: u64, mtime: i64) -> FileFingerprint {
        FileFingerprint { size, mtime }
    }

    #[test]
    fn test_record_and_lookup() {
        let mut cache = SkipCache::in_memory();
        cache.record_skip("/a/photo.png", fp(1000, 42), "WebP Converter", Some(85.0));

        assert!(cache.is_known_skip("/a/photo.png", &fp(1000, 42), "WebP Converter", Some(85.0)));

        // Any changed dimension is a miss
        assert!(!cache.is_known_skip("/a/photo.png", &fp(1001, 42), "WebP Converter", Some(85.0)));
        assert!(!cache.is_known_skip("/a/photo.png", &fp(1000, 43), "WebP Converter", Some(85.0)));
        assert!(!cache.is_known_skip("/a/photo.png", &fp(1000, 42), "Other Plugin", Some(85.0)));
        assert!(!cache.is_known_skip("/a/photo.png", &fp(1000, 42), "WebP Converter", Some(60.0)));
        assert!(!cache.is_known_skip("/a/other.png", &fp(1000, 42), "WebP Converter", Some(85.0)));
    }

    #[test]
    fn test_quality_switch_keeps_both_entries() {
        let mut cache = SkipCache::in_memory();
        cache.record_skip("/a.png", fp(1, 1), "P", Some(85.0));
        cache.record_skip("/a.png", fp(1, 1), "P", Some(60.0));

        // Switching quality back and forth reuses the remembered results
        assert!(cache.is_known_skip("/a.png", &fp(1, 1), "P", Some(85.0)));
        assert!(cache.is_known_skip("/a.png", &fp(1, 1), "P", Some(60.0)));
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_record_replaces_same_plugin_quality() {
        let mut cache = SkipCache::in_memory();
        cache.record_skip("/a.png", fp(1, 1), "P", Some(85.0));
        cache.record_skip("/a.png", fp(2, 2), "P", Some(85.0));

        assert_eq!(cache.len(), 1);
        assert!(!cache.is_known_skip("/a.png", &fp(1, 1), "P", Some(85.0)));
        assert!(cache.is_known_skip("/a.png", &fp(2, 2), "P", Some(85.0)));
    }

    #[test]
    fn test_invalidate_and_clear() {
        let mut cache = SkipCache::in_memory();
        cache.record_skip("/a.png", fp(1, 1), "P", None);
        cache.record_skip("/b.png", fp(1, 1), "P", None);

        cache.invalidate_path("/a.png");
        assert!(!cache.is_known_skip("/a.png", &fp(1, 1), "P", None));
        assert_eq!(cache.len(), 1);

        assert_eq!(cache.clear(), 1);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_persistence_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("skip_cache.json");

        let mut cache = SkipCache::load(path.clone());
        assert!(cache.is_empty());
        cache.record_skip("/a/photo.png", fp(1000, 42), "WebP Converter", Some(85.0));
        cache.save().unwrap();

        let reloaded = SkipCache::load(path.clone());
        assert_eq!(reloaded.len(), 1);
        assert!(reloaded.is_known_skip(
            "/a/photo.png",
            &fp(1000, 42),
            "WebP Converter",
            Some(85.0)
        ));
    }

    #[test]
    fn test_corrupt_file_starts_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("skip_cache.json");
        fs::write(&path, b"not json at all").unwrap();

        let cache = SkipCache::load(path);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_fingerprint_of_real_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("f.txt");
        fs::write(&file, b"hello").unwrap();

        let fp = FileFingerprint::of(&file).unwrap();
        assert_eq!(fp.size, 5);
        assert!(fp.mtime > 0);
    }
}
