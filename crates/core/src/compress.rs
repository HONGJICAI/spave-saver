use std::path::{Path, PathBuf};
use std::fs::{File, self};
use std::io::{self, Write, Read};
use anyhow::Result;
use zip::write::FileOptions;
use zip::{ZipWriter, CompressionMethod};
use flate2::Compression;
use flate2::write::GzEncoder;

/// Compression trait
pub trait CompressionAlgorithm {
    fn compress_file(&self, source: &Path, dest: &Path) -> Result<u64>;
    fn compress_directory(&self, source: &Path, dest: &Path) -> Result<u64>;
}

/// ZIP compression
pub struct ZipCompressor {
    compression_level: i32,
}

impl ZipCompressor {
    pub fn new() -> Self {
        Self {
            compression_level: 6, // Default compression level
        }
    }

    pub fn with_compression_level(mut self, level: i32) -> Self {
        self.compression_level = level.clamp(0, 9);
        self
    }

    fn add_directory_to_zip(
        &self,
        zip: &mut ZipWriter<File>,
        dir: &Path,
        prefix: &Path,
    ) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = path.strip_prefix(prefix)?;

            if path.is_file() {
                let options = FileOptions::default()
                    .compression_method(CompressionMethod::Deflated)
                    .compression_level(Some(self.compression_level));

                zip.start_file(name.to_string_lossy().to_string(), options)?;
                let mut file = File::open(&path)?;
                io::copy(&mut file, zip)?;
            } else if path.is_dir() {
                zip.add_directory(name.to_string_lossy().to_string(), FileOptions::default())?;
                self.add_directory_to_zip(zip, &path, prefix)?;
            }
        }
        Ok(())
    }
}

impl Default for ZipCompressor {
    fn default() -> Self {
        Self::new()
    }
}

impl CompressionAlgorithm for ZipCompressor {
    fn compress_file(&self, source: &Path, dest: &Path) -> Result<u64> {
        let file = File::create(dest)?;
        let mut zip = ZipWriter::new(file);

        let options = FileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .compression_level(Some(self.compression_level));

        let filename = source.file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?
            .to_string_lossy();

        zip.start_file(filename.to_string(), options)?;

        let mut file = File::open(source)?;
        io::copy(&mut file, &mut zip)?;

        let result = zip.finish()?;
        let compressed_size = result.metadata()?.len();

        Ok(compressed_size)
    }

    fn compress_directory(&self, source: &Path, dest: &Path) -> Result<u64> {
        let file = File::create(dest)?;
        let mut zip = ZipWriter::new(file);

        self.add_directory_to_zip(&mut zip, source, source)?;

        let result = zip.finish()?;
        let compressed_size = result.metadata()?.len();

        Ok(compressed_size)
    }
}

/// GZIP compression
pub struct GzipCompressor {
    compression_level: u32,
}

impl GzipCompressor {
    pub fn new() -> Self {
        Self {
            compression_level: 6,
        }
    }

    pub fn with_compression_level(mut self, level: u32) -> Self {
        self.compression_level = level.clamp(0, 9);
        self
    }
}

impl Default for GzipCompressor {
    fn default() -> Self {
        Self::new()
    }
}

impl CompressionAlgorithm for GzipCompressor {
    fn compress_file(&self, source: &Path, dest: &Path) -> Result<u64> {
        let mut input = File::open(source)?;
        let output = File::create(dest)?;
        let mut encoder = GzEncoder::new(output, Compression::new(self.compression_level));

        io::copy(&mut input, &mut encoder)?;
        let result = encoder.finish()?;
        let compressed_size = result.metadata()?.len();

        Ok(compressed_size)
    }

    fn compress_directory(&self, _source: &Path, _dest: &Path) -> Result<u64> {
        Err(anyhow::anyhow!("GZIP does not support directory compression directly. Use tar+gzip instead."))
    }
}

/// Main compressor interface
pub struct Compressor {
    algorithm: Box<dyn CompressionAlgorithm + Send + Sync>,
}

impl Compressor {
    pub fn new_zip() -> Self {
        Self {
            algorithm: Box::new(ZipCompressor::new()),
        }
    }

    pub fn new_gzip() -> Self {
        Self {
            algorithm: Box::new(GzipCompressor::new()),
        }
    }

    pub fn compress_file(&self, source: &Path, dest: &Path) -> Result<u64> {
        self.algorithm.compress_file(source, dest)
    }

    pub fn compress_directory(&self, source: &Path, dest: &Path) -> Result<u64> {
        self.algorithm.compress_directory(source, dest)
    }

    /// Calculate compression ratio
    pub fn compression_ratio(original_size: u64, compressed_size: u64) -> f32 {
        if original_size == 0 {
            return 0.0;
        }
        1.0 - (compressed_size as f32 / original_size as f32)
    }
}

impl Default for Compressor {
    fn default() -> Self {
        Self::new_zip()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[test]
    fn test_zip_compress_file() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("test.txt");
        let dest = dir.path().join("test.zip");

        fs::write(&source, "test content for compression").unwrap();

        let compressor = Compressor::new_zip();
        let compressed_size = compressor.compress_file(&source, &dest).unwrap();

        assert!(compressed_size > 0);
        assert!(dest.exists());
    }

    #[test]
    fn test_gzip_compress_file() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("test.txt");
        let dest = dir.path().join("test.gz");

        fs::write(&source, "test content for compression").unwrap();

        let compressor = Compressor::new_gzip();
        let compressed_size = compressor.compress_file(&source, &dest).unwrap();

        assert!(compressed_size > 0);
        assert!(dest.exists());
    }

    #[test]
    fn test_compression_ratio() {
        let ratio = Compressor::compression_ratio(1000, 500);
        assert_eq!(ratio, 0.5);

        let ratio = Compressor::compression_ratio(1000, 100);
        assert_eq!(ratio, 0.9);
    }
}
