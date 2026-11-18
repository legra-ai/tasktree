//! The lifecycle view a checkpoint needs of an application task record.

use crate::{
    TaskId,
    TaskSnapshotError,
    TaskStatus,
    TaskTimestamps,
    TerminalCause,
    UrnScheme,
};

/// An application task record that can be retained in a
/// [`RecoveryCheckpoint`](crate::RecoveryCheckpoint).
///
/// The application owns the record's shape (metadata, payload,
/// transport form); the checkpoint reads only identity and lifecycle.
pub trait RecoveredTask<S: UrnScheme> {
    /// The task's identity.
    fn id(&self) -> &TaskId<S>;

    /// The task's lifecycle status.
    fn status(&self) -> TaskStatus;

    /// Why the task ended, for failed and cancelled tasks.
    fn terminal_cause(&self) -> Option<TerminalCause>;

    /// The task's lifecycle timestamps.
    fn timestamps(&self) -> &TaskTimestamps;

    /// Whether the task has reached a terminal status.
    fn is_terminal(&self) -> bool {
        self.status().is_terminal()
    }

    /// Validate the record's lifecycle fields with
    /// [`TaskStatus::validate_snapshot`].
    ///
    /// # Errors
    ///
    /// Returns the first lifecycle disagreement.
    fn validate_lifecycle(&self) -> Result<(), TaskSnapshotError> {
        self.status()
            .validate_snapshot(self.terminal_cause(), self.timestamps())
    }
}
