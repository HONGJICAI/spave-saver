use anyhow::Result;
use image::{imageops::FilterType, DynamicImage};
use std::path::Path;

/// Image similarity algorithm trait
pub trait SimilarityAlgorithm {
    fn compare(&self, a: &Path, b: &Path) -> Result<f32>;
}

/// Perceptual hash (pHash) based similarity
pub struct ImageSimilarity {
    hash_size: u32,
}

impl ImageSimilarity {
    pub fn new() -> Self {
        Self { hash_size: 8 }
    }

    pub fn with_hash_size(mut self, size: u32) -> Self {
        self.hash_size = size;
        self
    }

    /// Compute perceptual hash for an image
    fn compute_phash(&self, path: &Path) -> Result<Vec<u8>> {
        let img = image::open(path)?;
        let img = img.resize_exact(self.hash_size, self.hash_size, FilterType::Lanczos3);
        let img = img.to_luma8();

        // Calculate average pixel value
        let pixels: Vec<u8> = img.as_raw().clone();
        let sum: u32 = pixels.iter().map(|&p| p as u32).sum();
        let avg = sum / (self.hash_size * self.hash_size);

        // Create hash based on whether each pixel is above or below average
        let hash: Vec<u8> = pixels
            .iter()
            .map(|&p| if p as u32 >= avg { 1 } else { 0 })
            .collect();

        Ok(hash)
    }

    /// Calculate hamming distance between two hashes
    fn hamming_distance(&self, hash1: &[u8], hash2: &[u8]) -> u32 {
        hash1
            .iter()
            .zip(hash2.iter())
            .filter(|(a, b)| a != b)
            .count() as u32
    }

    /// Convert hamming distance to similarity score (0.0 to 1.0)
    fn distance_to_similarity(&self, distance: u32, hash_length: u32) -> f32 {
        1.0 - (distance as f32 / hash_length as f32)
    }
}

impl Default for ImageSimilarity {
    fn default() -> Self {
        Self::new()
    }
}

impl SimilarityAlgorithm for ImageSimilarity {
    fn compare(&self, a: &Path, b: &Path) -> Result<f32> {
        let hash_a = self.compute_phash(a)?;
        let hash_b = self.compute_phash(b)?;

        let distance = self.hamming_distance(&hash_a, &hash_b);
        let hash_length = self.hash_size * self.hash_size;

        Ok(self.distance_to_similarity(distance, hash_length))
    }
}

/// Alternative: Histogram-based similarity
pub struct HistogramSimilarity;

impl HistogramSimilarity {
    pub fn new() -> Self {
        Self
    }

    fn load_and_resize(path: &Path) -> Result<DynamicImage> {
        let img = image::open(path)?;
        Ok(img.resize(256, 256, FilterType::Lanczos3))
    }

    fn compute_histogram(img: &DynamicImage) -> Vec<u32> {
        let rgb = img.to_rgb8();
        let mut histogram = vec![0u32; 256];

        for pixel in rgb.pixels() {
            let gray = (pixel[0] as u32 + pixel[1] as u32 + pixel[2] as u32) / 3;
            histogram[gray as usize] += 1;
        }

        histogram
    }

    fn histogram_correlation(hist1: &[u32], hist2: &[u32]) -> f32 {
        let sum1: u32 = hist1.iter().sum();
        let sum2: u32 = hist2.iter().sum();

        if sum1 == 0 || sum2 == 0 {
            return 0.0;
        }

        let mut correlation = 0.0;
        for (v1, v2) in hist1.iter().zip(hist2.iter()) {
            let p1 = *v1 as f32 / sum1 as f32;
            let p2 = *v2 as f32 / sum2 as f32;
            correlation += (p1 * p2).sqrt();
        }

        correlation
    }
}

impl Default for HistogramSimilarity {
    fn default() -> Self {
        Self::new()
    }
}

impl SimilarityAlgorithm for HistogramSimilarity {
    fn compare(&self, a: &Path, b: &Path) -> Result<f32> {
        let img_a = Self::load_and_resize(a)?;
        let img_b = Self::load_and_resize(b)?;

        let hist_a = Self::compute_histogram(&img_a);
        let hist_b = Self::compute_histogram(&img_b);

        Ok(Self::histogram_correlation(&hist_a, &hist_b))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hamming_distance() {
        let similarity = ImageSimilarity::new();
        let hash1 = vec![1, 0, 1, 0];
        let hash2 = vec![1, 1, 1, 0];

        let distance = similarity.hamming_distance(&hash1, &hash2);
        assert_eq!(distance, 1);
    }

    #[test]
    fn test_distance_to_similarity() {
        let similarity = ImageSimilarity::new();

        // Identical hashes (distance 0)
        let sim = similarity.distance_to_similarity(0, 64);
        assert_eq!(sim, 1.0);

        // Half different (distance 32)
        let sim = similarity.distance_to_similarity(32, 64);
        assert_eq!(sim, 0.5);
    }
}
