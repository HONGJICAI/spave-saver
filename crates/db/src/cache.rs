use anyhow::Result;
use sled::Db;
use std::path::Path;

/// Key-value cache using sled (embedded database)
pub struct Cache {
    db: Db,
}

impl Cache {
    /// Create a new cache at the specified path
    pub fn new(path: &Path) -> Result<Self> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    /// Create a temporary in-memory cache
    pub fn temporary() -> Result<Self> {
        let config = sled::Config::new().temporary(true);
        let db = config.open()?;
        Ok(Self { db })
    }

    /// Set a value in the cache
    pub fn set(&self, key: &[u8], value: &[u8]) -> Result<()> {
        self.db.insert(key, value)?;
        Ok(())
    }

    /// Get a value from the cache
    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        match self.db.get(key)? {
            Some(value) => Ok(Some(value.to_vec())),
            None => Ok(None),
        }
    }

    /// Check if a key exists
    pub fn contains(&self, key: &[u8]) -> Result<bool> {
        Ok(self.db.contains_key(key)?)
    }

    /// Delete a key
    pub fn delete(&self, key: &[u8]) -> Result<()> {
        self.db.remove(key)?;
        Ok(())
    }

    /// Clear all data
    pub fn clear(&self) -> Result<()> {
        self.db.clear()?;
        Ok(())
    }

    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.db.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.db.is_empty()
    }

    /// Flush data to disk
    pub fn flush(&self) -> Result<()> {
        self.db.flush()?;
        Ok(())
    }

    /// Set a string key-value pair
    pub fn set_string(&self, key: &str, value: &str) -> Result<()> {
        self.set(key.as_bytes(), value.as_bytes())
    }

    /// Get a string value
    pub fn get_string(&self, key: &str) -> Result<Option<String>> {
        match self.get(key.as_bytes())? {
            Some(bytes) => Ok(Some(String::from_utf8(bytes)?)),
            None => Ok(None),
        }
    }

    /// Set a serialized value (using bincode)
    pub fn set_serialized<T: serde::Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let bytes = bincode::serialize(value)?;
        self.set(key.as_bytes(), &bytes)
    }

    /// Get a deserialized value (using bincode)
    pub fn get_serialized<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        match self.get(key.as_bytes())? {
            Some(bytes) => {
                let value = bincode::deserialize(&bytes)?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }
}

/// File hash cache - specialized cache for file hashes
pub struct FileHashCache {
    cache: Cache,
}

impl FileHashCache {
    pub fn new(path: &Path) -> Result<Self> {
        let cache = Cache::new(path)?;
        Ok(Self { cache })
    }

    pub fn temporary() -> Result<Self> {
        let cache = Cache::temporary()?;
        Ok(Self { cache })
    }

    /// Get cached hash for a file
    /// Key format: "file_path:modified_timestamp"
    pub fn get_hash(&self, file_path: &str, modified: i64) -> Result<Option<String>> {
        let key = format!("{}:{}", file_path, modified);
        self.cache.get_string(&key)
    }

    /// Set cached hash for a file
    pub fn set_hash(&self, file_path: &str, modified: i64, hash: &str) -> Result<()> {
        let key = format!("{}:{}", file_path, modified);
        self.cache.set_string(&key, hash)
    }

    /// Check if file hash is cached
    pub fn has_hash(&self, file_path: &str, modified: i64) -> Result<bool> {
        let key = format!("{}:{}", file_path, modified);
        self.cache.contains(key.as_bytes())
    }

    /// Clear all cached hashes
    pub fn clear(&self) -> Result<()> {
        self.cache.clear()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_operations() {
        let cache = Cache::temporary().unwrap();

        // Test set and get
        cache.set(b"key1", b"value1").unwrap();
        let value = cache.get(b"key1").unwrap();
        assert_eq!(value, Some(b"value1".to_vec()));

        // Test contains
        assert!(cache.contains(b"key1").unwrap());
        assert!(!cache.contains(b"key2").unwrap());

        // Test delete
        cache.delete(b"key1").unwrap();
        assert!(!cache.contains(b"key1").unwrap());
    }

    #[test]
    fn test_string_operations() {
        let cache = Cache::temporary().unwrap();

        cache.set_string("name", "Alice").unwrap();
        let value = cache.get_string("name").unwrap();
        assert_eq!(value, Some("Alice".to_string()));
    }

    #[test]
    fn test_serialization() {
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct Person {
            name: String,
            age: u32,
        }

        let cache = Cache::temporary().unwrap();
        let person = Person {
            name: "Bob".to_string(),
            age: 30,
        };

        cache.set_serialized("person", &person).unwrap();
        let retrieved: Option<Person> = cache.get_serialized("person").unwrap();
        assert_eq!(retrieved, Some(person));
    }

    #[test]
    fn test_file_hash_cache() {
        let cache = FileHashCache::temporary().unwrap();

        cache.set_hash("/test/file.txt", 12345, "abc123").unwrap();

        let hash = cache.get_hash("/test/file.txt", 12345).unwrap();
        assert_eq!(hash, Some("abc123".to_string()));

        assert!(cache.has_hash("/test/file.txt", 12345).unwrap());
        assert!(!cache.has_hash("/test/file.txt", 99999).unwrap());
    }
}
