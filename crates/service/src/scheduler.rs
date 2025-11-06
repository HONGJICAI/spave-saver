use tokio::sync::{mpsc, RwLock};
use std::sync::Arc;
use anyhow::Result;
use crate::task::{Task, TaskStatus};
use crate::progress::ProgressUpdate;
use tracing::{info, error};

/// Task scheduler for managing concurrent tasks
pub struct Scheduler {
    task_queue: Arc<RwLock<Vec<Box<dyn Task>>>>,
    max_concurrent: usize,
    progress_tx: mpsc::Sender<ProgressUpdate>,
}

impl Scheduler {
    pub fn new(max_concurrent: usize) -> (Self, mpsc::Receiver<ProgressUpdate>) {
        let (progress_tx, progress_rx) = mpsc::channel(100);
        
        let scheduler = Self {
            task_queue: Arc::new(RwLock::new(Vec::new())),
            max_concurrent,
            progress_tx,
        };

        (scheduler, progress_rx)
    }

    /// Submit a task to the queue
    pub async fn submit(&self, task: Box<dyn Task>) -> Result<()> {
        let mut queue = self.task_queue.write().await;
        queue.push(task);
        info!("Task submitted. Queue length: {}", queue.len());
        Ok(())
    }

    /// Start the scheduler
    pub async fn start(&self) -> Result<()> {
        info!("Scheduler started with max_concurrent={}", self.max_concurrent);

        loop {
            let task = {
                let mut queue = self.task_queue.write().await;
                queue.pop()
            };

            match task {
                Some(mut task) => {
                    let progress_tx = self.progress_tx.clone();
                    
                    tokio::spawn(async move {
                        info!("Executing task: {:?}", task.task_type());
                        
                        match task.run(progress_tx).await {
                            Ok(_) => {
                                info!("Task completed successfully");
                            }
                            Err(e) => {
                                error!("Task failed: {}", e);
                            }
                        }
                    });
                }
                None => {
                    // No tasks in queue, wait a bit
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }
        }
    }

    /// Get the number of tasks in the queue
    pub async fn queue_length(&self) -> usize {
        let queue = self.task_queue.read().await;
        queue.len()
    }

    /// Clear all pending tasks
    pub async fn clear_queue(&self) {
        let mut queue = self.task_queue.write().await;
        queue.clear();
        info!("Task queue cleared");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::ScanTask;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_scheduler_submit() {
        let (scheduler, _rx) = Scheduler::new(4);
        let task = Box::new(ScanTask::new(PathBuf::from("/test")));
        
        scheduler.submit(task).await.unwrap();
        assert_eq!(scheduler.queue_length().await, 1);
    }

    #[tokio::test]
    async fn test_scheduler_clear() {
        let (scheduler, _rx) = Scheduler::new(4);
        let task = Box::new(ScanTask::new(PathBuf::from("/test")));
        
        scheduler.submit(task).await.unwrap();
        assert_eq!(scheduler.queue_length().await, 1);
        
        scheduler.clear_queue().await;
        assert_eq!(scheduler.queue_length().await, 0);
    }
}
