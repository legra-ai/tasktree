//! Unix-epoch lifecycle timestamps.

use std::time::SystemTime;

use serde::{
    Deserialize,
    Serialize,
};

/// Unix-epoch timestamps tracking the lifecycle of a task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskTimestamps {
    /// When the task was created (milliseconds since Unix epoch).
    pub created_at: u64,
    /// When the task started executing.
    pub started_at: Option<u64>,
    /// When the task reached a terminal state.
    pub completed_at: Option<u64>,
}

impl TaskTimestamps {
    /// Create timestamps with `created_at` set to the current time.
    #[must_use]
    pub fn now() -> Self {
        Self {
            created_at: epoch_ms(),
            started_at: None,
            completed_at: None,
        }
    }

    /// Returns the wall-clock duration in milliseconds between
    /// `started_at` and `completed_at`, if both are set.
    #[must_use]
    pub fn duration_ms(&self) -> Option<u64> {
        if let (Some(start), Some(end)) = (self.started_at, self.completed_at) {
            Some(end.saturating_sub(start))
        } else {
            None
        }
    }
}

fn epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("system clock is before Unix epoch")
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::TaskTimestamps;

    #[test]
    fn now_starts_an_unfinished_lifecycle() {
        let timestamps = TaskTimestamps::now();

        assert!(timestamps.created_at > 0);
        assert_eq!(timestamps.started_at, None);
        assert_eq!(timestamps.completed_at, None);
    }

    #[test]
    fn duration_requires_a_start() {
        let timestamps = TaskTimestamps {
            created_at: 1_000,
            started_at: None,
            completed_at: Some(5_000),
        };

        assert_eq!(timestamps.duration_ms(), None);
    }
}
