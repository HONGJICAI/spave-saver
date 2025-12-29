use crate::progress::ProgressUpdate;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::sync::mpsc;

/// Task type enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    Scan(PathBuf),
    FindDuplicates(PathBuf),
    FindSimilarImages(PathBuf, f32), // path, threshold
    CleanEmpty(PathBuf),
    CompressFiles(Vec<PathBuf>),
    DeleteFiles(Vec<PathBuf>),
}

/// Task status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
    Cancelled,
}

/// Task trait for async execution
#[async_trait]
pub trait Task: Send + Sync {
    async fn run(&mut self, progress_tx: mpsc::Sender<ProgressUpdate>) -> Result<()>;
    fn task_type(&self) -> &TaskType;
    fn status(&self) -> &TaskStatus;
}

/// Scan task implementation
pub struct ScanTask {
    task_type: TaskType,
    status: TaskStatus,
}

impl ScanTask {
    pub fn new(path: PathBuf) -> Self {
        Self {
            task_type: TaskType::Scan(path),
            status: TaskStatus::Pending,
        }
    }
}

#[async_trait]
impl Task for ScanTask {
    async fn run(&mut self, progress_tx: mpsc::Sender<ProgressUpdate>) -> Result<()> {
        use space_saver_core::{scanner::DefaultFileScanner, FileScanner};

        self.status = TaskStatus::Running;

        let path = match &self.task_type {
            TaskType::Scan(p) => p.clone(),
            _ => unreachable!(),
        };

        let _ = progress_tx
            .send(ProgressUpdate::Started {
                task_type: format!("{:?}", self.task_type),
                total_items: 0,
            })
            .await;

        let scanner = DefaultFileScanner::new();
        let files = scanner.scan(&path)?;

        let _ = progress_tx
            .send(ProgressUpdate::Progress {
                current: files.len(),
                total: files.len(),
                message: format!("Scanned {} files", files.len()),
            })
            .await;

        self.status = TaskStatus::Completed;

        let _ = progress_tx
            .send(ProgressUpdate::Completed {
                message: format!("Scan completed. Found {} files", files.len()),
            })
            .await;

        Ok(())
    }

    fn task_type(&self) -> &TaskType {
        &self.task_type
    }

    fn status(&self) -> &TaskStatus {
        &self.status
    }
}

/// Find duplicates task
pub struct FindDuplicatesTask {
    task_type: TaskType,
    status: TaskStatus,
}

impl FindDuplicatesTask {
    pub fn new(path: PathBuf) -> Self {
        Self {
            task_type: TaskType::FindDuplicates(path),
            status: TaskStatus::Pending,
        }
    }
}

#[async_trait]
impl Task for FindDuplicatesTask {
    async fn run(&mut self, progress_tx: mpsc::Sender<ProgressUpdate>) -> Result<()> {
        use space_saver_core::{scanner::DefaultFileScanner, FileHasher, FileScanner};
        use std::collections::HashMap;

        self.status = TaskStatus::Running;

        let path = match &self.task_type {
            TaskType::FindDuplicates(p) => p.clone(),
            _ => unreachable!(),
        };

        let _ = progress_tx
            .send(ProgressUpdate::Started {
                task_type: "FindDuplicates".to_string(),
                total_items: 0,
            })
            .await;

        // Scan files
        let scanner = DefaultFileScanner::new();
        let files = scanner.scan(&path)?;

        // Hash files
        let hasher = FileHasher::new_blake3();
        let mut hash_map: HashMap<String, Vec<PathBuf>> = HashMap::new();

        for (idx, file) in files.iter().enumerate() {
            if let Ok(hash) = hasher.hash_file(&file.path) {
                hash_map
                    .entry(hash)
                    .or_default()
                    .push(file.path.clone());
            }

            if idx % 100 == 0 {
                let _ = progress_tx
                    .send(ProgressUpdate::Progress {
                        current: idx,
                        total: files.len(),
                        message: format!("Hashing files... {}/{}", idx, files.len()),
                    })
                    .await;
            }
        }

        // Count duplicates
        let duplicates: Vec<_> = hash_map
            .into_iter()
            .filter(|(_, paths)| paths.len() > 1)
            .collect();

        self.status = TaskStatus::Completed;

        let _ = progress_tx
            .send(ProgressUpdate::Completed {
                message: format!("Found {} groups of duplicate files", duplicates.len()),
            })
            .await;

        Ok(())
    }

    fn task_type(&self) -> &TaskType {
        &self.task_type
    }

    fn status(&self) -> &TaskStatus {
        &self.status
    }
}

/// Clean empty files task
pub struct CleanEmptyTask {
    task_type: TaskType,
    status: TaskStatus,
}

impl CleanEmptyTask {
    pub fn new(path: PathBuf) -> Self {
        Self {
            task_type: TaskType::CleanEmpty(path),
            status: TaskStatus::Pending,
        }
    }
}

#[async_trait]
impl Task for CleanEmptyTask {
    async fn run(&mut self, progress_tx: mpsc::Sender<ProgressUpdate>) -> Result<()> {
        use space_saver_core::{scanner::DefaultFileScanner, FileFilter, FileScanner};

        self.status = TaskStatus::Running;

        let path = match &self.task_type {
            TaskType::CleanEmpty(p) => p.clone(),
            _ => unreachable!(),
        };

        let _ = progress_tx
            .send(ProgressUpdate::Started {
                task_type: "CleanEmpty".to_string(),
                total_items: 0,
            })
            .await;

        // Scan and filter empty files
        let scanner = DefaultFileScanner::new();
        let files = scanner.scan(&path)?;
        let filter = FileFilter::empty_files();
        let empty_files = filter.filter_files(files);

        let _ = progress_tx
            .send(ProgressUpdate::Completed {
                message: format!("Found {} empty files", empty_files.len()),
            })
            .await;

        self.status = TaskStatus::Completed;
        Ok(())
    }

    fn task_type(&self) -> &TaskType {
        &self.task_type
    }

    fn status(&self) -> &TaskStatus {
        &self.status
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = ScanTask::new(PathBuf::from("/test"));
        assert_eq!(*task.status(), TaskStatus::Pending);
    }

    #[tokio::test]
    async fn test_scan_task() {
        use tempfile::tempdir;
        let dir = tempdir().unwrap();

        let (tx, mut rx) = mpsc::channel(10);
        let mut task = ScanTask::new(dir.path().to_path_buf());

        tokio::spawn(async move {
            let _ = task.run(tx).await;
        });

        // Collect progress updates
        while let Some(update) = rx.recv().await {
            if let ProgressUpdate::Completed { .. } = update {
                break;
            }
        }
    }
}
