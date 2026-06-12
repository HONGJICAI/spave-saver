use crate::skip_cache::FileFingerprint;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::warn;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HashEntry {
    fingerprint: FileFingerprint,
    hash: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct HashCacheData {
    version: u32,
    entries: HashMap<String, HashEntry>,
}

/// Remembers content hashes keyed by path and guarded by a size+mtime
/// fingerprint, so repeated duplicate scans skip re-reading unchanged files.
/// A stale fingerprint simply misses; the entry is replaced on insert.
#[derive(Debug, Default)]
pub struct HashCache {
    data: HashCacheData,
    storage_path: Option<PathBuf>,
    dirty: bool,
}

impl HashCache {
    /// Load from `path`; missing or corrupt files yield an empty cache
    /// (this is an optimization, never a hard dependency)
    pub fn load(path: PathBuf) -> Self {
        let data = match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|e| {
                warn!(path = %path.display(), error = %e, "Corrupt hash cache; starting empty");
                HashCacheData::default()
            }),
            Err(_) => HashCacheData::default(),
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

    /// Persist to disk if anything changed (atomic: temp file + rename)
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
        fs::write(&tmp, serde_json::to_vec(&self.data)?)?;
        fs::rename(&tmp, path)?;
        self.dirty = false;
        Ok(())
    }

    /// The cached hash, if the file state still matches
    pub fn get(&self, path: &str, fingerprint: &FileFingerprint) -> Option<&str> {
        self.data
            .entries
            .get(path)
            .filter(|e| e.fingerprint == *fingerprint)
            .map(|e| e.hash.as_str())
    }

    pub fn insert(&mut self, path: &str, fingerprint: FileFingerprint, hash: String) {
        self.data.entries.insert(
            path.to_string(),
            HashEntry { fingerprint, hash },
        );
        self.dirty = true;
    }

    pub fn len(&self) -> usize {
        self.data.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.entries.is_empty()
    }

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
    fn test_get_requires_matching_fingerprint() {
        let mut cache = HashCache::in_memory();
        cache.insert("/a.bin", fp(100, 7), "abc123".to_string());

        assert_eq!(cache.get("/a.bin", &fp(100, 7)), Some("abc123"));
        assert_eq!(cache.get("/a.bin", &fp(101, 7)), None);
        assert_eq!(cache.get("/a.bin", &fp(100, 8)), None);
        assert_eq!(cache.get("/b.bin", &fp(100, 7)), None);
    }

    #[test]
    fn test_insert_replaces() {
        let mut cache = HashCache::in_memory();
        cache.insert("/a.bin", fp(100, 7), "old".to_string());
        cache.insert("/a.bin", fp(200, 8), "new".to_string());

        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get("/a.bin", &fp(100, 7)), None);
        assert_eq!(cache.get("/a.bin", &fp(200, 8)), Some("new"));
    }

    #[test]
    fn test_persistence_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hash_cache.json");

        let mut cache = HashCache::load(path.clone());
        cache.insert("/a.bin", fp(100, 7), "abc".to_string());
        cache.save().unwrap();

        let reloaded = HashCache::load(path);
        assert_eq!(reloaded.get("/a.bin", &fp(100, 7)), Some("abc"));
    }

    #[test]
    fn test_corrupt_file_starts_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hash_cache.json");
        fs::write(&path, b"garbage").unwrap();

        assert!(HashCache::load(path).is_empty());
    }
}
