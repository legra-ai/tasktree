//! The lifecycle rule every restored task snapshot must satisfy.

use thiserror::Error;

use crate::{
    TaskStatus,
    TaskTimestamps,
    TaskTransitionError,
    TerminalCause,
};

impl TaskStatus {
    /// Validate the lifecycle fields of a restored task snapshot.
    ///
    /// Sparse observations may omit a start timestamp, and Unix-epoch
    /// clocks can regress, so neither start presence nor numeric ordering
    /// is checked.
    ///
    /// # Errors
    ///
    /// `Failed` and `Cancelled` require a cause ending in that status and
    /// every other status refuses a cause; a completion timestamp must be
    /// present exactly when the status is terminal.
    pub const fn validate_snapshot(
        self,
        cause: Option<TerminalCause>,
        timestamps: &TaskTimestamps,
    ) -> Result<(), TaskSnapshotError> {
        if let Err(error) = self.validate_cause(cause) {
            return Err(TaskSnapshotError::InvalidCause(error));
        }
        match (self.is_terminal(), timestamps.completed_at.is_some()) {
            (true, false) => Err(TaskSnapshotError::MissingCompletion { status: self }),
            (false, true) => Err(TaskSnapshotError::UnexpectedCompletion { status: self }),
            _ => Ok(()),
        }
    }

    const fn validate_cause(self, cause: Option<TerminalCause>) -> Result<(), TaskTransitionError> {
        match (self.requires_cause(), cause) {
            (true, None) => Err(TaskTransitionError::MissingCause { to: self }),
            (false, Some(_)) => Err(TaskTransitionError::UnexpectedCause { to: self }),
            (true, Some(cause)) if cause.terminal_status() as u8 != self as u8 => {
                Err(TaskTransitionError::CauseMismatch {
                    cause: cause.origin(),
                    expected: cause.terminal_status(),
                    to: self,
                })
            }
            _ => Ok(()),
        }
    }
}

/// A restored task snapshot whose lifecycle fields disagree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum TaskSnapshotError {
    /// The terminal cause does not fit the status.
    #[error(transparent)]
    InvalidCause(TaskTransitionError),
    /// A terminal status without a completion timestamp.
    #[error("terminal status {status} has no completion timestamp", status = .status.as_str())]
    MissingCompletion {
        /// The terminal status.
        status: TaskStatus,
    },
    /// A nonterminal status with a completion timestamp.
    #[error("nonterminal status {status} has a completion timestamp", status = .status.as_str())]
    UnexpectedCompletion {
        /// The nonterminal status.
        status: TaskStatus,
    },
}
