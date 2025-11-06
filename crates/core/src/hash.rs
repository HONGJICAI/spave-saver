use std::path::Path;
use std::fs::File;
use std::io::{BufReader, Read};
use anyhow::Result;
use blake3::Hasher as Blake3Hasher;
use sha2::{Sha256, Digest};

/// Hash algorithm trait
pub trait HashAlgorithm {
    fn hash_file(&self, path: &Path) -> Result<String>;
    fn hash_bytes(&self, data: &[u8]) -> String;
}

/// BLAKE3 hasher (fast, recommended for large files)
pub struct Blake3Hash;

impl HashAlgorithm for Blake3Hash {
    fn hash_file(&self, path: &Path) -> Result<String> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut hasher = Blake3Hasher::new();
        let mut buffer = vec![0u8; 8192];

        loop {
            let count = reader.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            hasher.update(&buffer[..count]);
        }

        Ok(hasher.finalize().to_hex().to_string())
    }

    fn hash_bytes(&self, data: &[u8]) -> String {
        blake3::hash(data).to_hex().to_string()
    }
}

/// SHA256 hasher (standard, widely compatible)
pub struct Sha256Hash;

impl HashAlgorithm for Sha256Hash {
    fn hash_file(&self, path: &Path) -> Result<String> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 8192];

        loop {
            let count = reader.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            hasher.update(&buffer[..count]);
        }

        Ok(format!("{:x}", hasher.finalize()))
    }

    fn hash_bytes(&self, data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }
}

/// File hasher with configurable algorithm
pub struct FileHasher {
    algorithm: Box<dyn HashAlgorithm + Send + Sync>,
}

impl FileHasher {
    pub fn new_blake3() -> Self {
        Self {
            algorithm: Box::new(Blake3Hash),
        }
    }

    pub fn new_sha256() -> Self {
        Self {
            algorithm: Box::new(Sha256Hash),
        }
    }

    pub fn hash_file(&self, path: &Path) -> Result<String> {
        self.algorithm.hash_file(path)
    }

    pub fn hash_bytes(&self, data: &[u8]) -> String {
        self.algorithm.hash_bytes(data)
    }
}

impl Default for FileHasher {
    fn default() -> Self {
        Self::new_blake3()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_blake3_hash() {
        let hasher = Blake3Hash;
        let data = b"test data";
        let hash = hasher.hash_bytes(data);
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // BLAKE3 produces 32-byte hash (64 hex chars)
    }

    #[test]
    fn test_sha256_hash() {
        let hasher = Sha256Hash;
        let data = b"test data";
        let hash = hasher.hash_bytes(data);
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // SHA256 produces 32-byte hash (64 hex chars)
    }

    #[test]
    fn test_file_hasher() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "test content").unwrap();

        let hasher = FileHasher::new_blake3();
        let hash = hasher.hash_file(&file_path).unwrap();
        assert!(!hash.is_empty());
    }

    #[test]
    fn test_consistent_hashing() {
        let data = b"consistent data";
        let hasher = Blake3Hash;
        
        let hash1 = hasher.hash_bytes(data);
        let hash2 = hasher.hash_bytes(data);
        
        assert_eq!(hash1, hash2);
    }
}
