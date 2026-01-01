use serde::{Deserialize, Serialize};

/// File record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    pub id: i64,
    pub path: String,
    pub size: u64,
    pub hash: Option<String>,
    pub file_type: String,
    pub modified: i64,
    pub created_at: i64,
}

/// Scan record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanRecord {
    pub id: i64,
    pub path: String,
    pub file_count: usize,
    pub total_size: u64,
    pub scan_time: i64,
    pub created_at: i64,
}

/// Duplicate record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateRecord {
    pub id: i64,
    pub hash: String,
    pub file_paths: Vec<String>,
    pub file_count: usize,
    pub total_size: u64,
    pub wasted_space: u64,
    pub created_at: i64,
}

/// Image similarity record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarityRecord {
    pub id: i64,
    pub file_a: String,
    pub file_b: String,
    pub similarity_score: f32,
    pub created_at: i64,
}

impl FileRecord {
    pub fn new(path: String, size: u64, file_type: String, modified: i64) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: 0,
            path,
            size,
            hash: None,
            file_type,
            modified,
            created_at: now,
        }
    }
}

impl ScanRecord {
    pub fn new(path: String, file_count: usize, total_size: u64, scan_time: i64) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: 0,
            path,
            file_count,
            total_size,
            scan_time,
            created_at: now,
        }
    }
}

impl DuplicateRecord {
    pub fn new(
        hash: String,
        file_paths: Vec<String>,
        file_count: usize,
        total_size: u64,
        wasted_space: u64,
    ) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: 0,
            hash,
            file_paths,
            file_count,
            total_size,
            wasted_space,
            created_at: now,
        }
    }
}
