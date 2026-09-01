//! Task lifecycle status and validated transitions.

use serde::{
    Deserialize,
    Deserializer,
    Serialize,
};
use thiserror::Error;

/// Lifecycle status of a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Task has been admitted but not yet queued or started.
    Admitted,
    /// Task is waiting in the admission queue.
    Queued,
    /// Task is actively executing.
    Running,
    /// The task has entered its irreversible sealing region.
    Sealing,
    /// Task completed successfully.
    Done,
    /// Task failed with an error.
    Failed,
    /// Task was cancelled before completion.
    Cancelled,
}

impl TaskStatus {
    /// Every lifecycle status in declaration order.
    pub const ALL: [Self; 7] = [
        Self::Admitted,
        Self::Queued,
        Self::Running,
        Self::Sealing,
        Self::Done,
        Self::Failed,
        Self::Cancelled,
    ];

    /// Returns `true` if this status is terminal.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Done | Self::Failed | Self::Cancelled)
    }

    /// Returns a static string label for the status.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Admitted => "admitted",
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Sealing => "sealing",
            Self::Done => "done",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    /// Parse a status from its canonical label.
    ///
    /// # Errors
    ///
    /// Returns the unrecognized label when no variant matches.
    pub fn parse_label(label: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|status| status.as_str() == label)
            .ok_or_else(|| format!("unknown task status {label:?}"))
    }

    /// Validate a lifecycle transition.
    ///
    /// # Errors
    ///
    /// Rejects every edge outside the task state machine.
    pub fn validate_transition(self, next: Self) -> Result<(), TaskTransitionError> {
        TaskTransition::try_new(self, next).map(|_| ())
    }
}

/// An illegal task lifecycle transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("invalid task transition from {from} to {to}", from = .from.as_str(), to = .to.as_str())]
pub struct TaskTransitionError {
    from: TaskStatus,
    to: TaskStatus,
}

impl TaskTransitionError {
    /// Return the current status.
    #[must_use]
    pub const fn from(self) -> TaskStatus {
        self.from
    }

    /// Return the refused target status.
    #[must_use]
    pub const fn to(self) -> TaskStatus {
        self.to
    }
}

/// One validated lifecycle transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TaskTransition {
    from: TaskStatus,
    to: TaskStatus,
}

impl TaskTransition {
    /// Validate and construct one state-machine edge.
    ///
    /// # Errors
    ///
    /// Rejects illegal transitions, including every transition out of a
    /// terminal state.
    pub fn try_new(from: TaskStatus, to: TaskStatus) -> Result<Self, TaskTransitionError> {
        let valid = match from {
            TaskStatus::Admitted => matches!(
                to,
                TaskStatus::Queued | TaskStatus::Running | TaskStatus::Cancelled
            ),
            TaskStatus::Queued => matches!(to, TaskStatus::Running | TaskStatus::Cancelled),
            TaskStatus::Running => matches!(
                to,
                TaskStatus::Sealing | TaskStatus::Done | TaskStatus::Failed | TaskStatus::Cancelled
            ),
            TaskStatus::Sealing => matches!(to, TaskStatus::Done | TaskStatus::Failed),
            TaskStatus::Done | TaskStatus::Failed | TaskStatus::Cancelled => false,
        };
        if !valid {
            return Err(TaskTransitionError { from, to });
        }
        Ok(Self { from, to })
    }

    /// Return the current status.
    #[must_use]
    pub const fn from(self) -> TaskStatus {
        self.from
    }

    /// Return the target status.
    #[must_use]
    pub const fn to(self) -> TaskStatus {
        self.to
    }
}

impl<'de> Deserialize<'de> for TaskTransition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct WireTransition {
            from: TaskStatus,
            to: TaskStatus,
        }

        let wire = WireTransition::deserialize(deserializer)?;
        Self::try_new(wire.from, wire.to).map_err(serde::de::Error::custom)
    }
}
