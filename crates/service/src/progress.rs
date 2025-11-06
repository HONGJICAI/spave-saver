use serde::{Deserialize, Serialize};

/// Progress update message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProgressUpdate {
    Started {
        task_type: String,
        total_items: usize,
    },
    Progress {
        current: usize,
        total: usize,
        message: String,
    },
    Completed {
        message: String,
    },
    Failed {
        error: String,
    },
    Cancelled,
}

/// Progress tracker
pub struct ProgressTracker {
    current: usize,
    total: usize,
    message: String,
}

impl ProgressTracker {
    pub fn new(total: usize) -> Self {
        Self {
            current: 0,
            total,
            message: String::new(),
        }
    }

    pub fn update(&mut self, current: usize, message: String) {
        self.current = current;
        self.message = message;
    }

    pub fn increment(&mut self) {
        self.current += 1;
    }

    pub fn set_message(&mut self, message: String) {
        self.message = message;
    }

    pub fn progress(&self) -> f32 {
        if self.total == 0 {
            return 0.0;
        }
        self.current as f32 / self.total as f32
    }

    pub fn percentage(&self) -> u8 {
        (self.progress() * 100.0) as u8
    }

    pub fn current(&self) -> usize {
        self.current
    }

    pub fn total(&self) -> usize {
        self.total
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn to_update(&self) -> ProgressUpdate {
        ProgressUpdate::Progress {
            current: self.current,
            total: self.total,
            message: self.message.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_tracker() {
        let mut tracker = ProgressTracker::new(100);
        assert_eq!(tracker.progress(), 0.0);
        assert_eq!(tracker.percentage(), 0);

        tracker.update(50, "Half done".to_string());
        assert_eq!(tracker.progress(), 0.5);
        assert_eq!(tracker.percentage(), 50);
        assert_eq!(tracker.message(), "Half done");

        tracker.increment();
        assert_eq!(tracker.current(), 51);
    }
}
