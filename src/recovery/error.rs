//! Refusals of a recovery checkpoint.

use thiserror::Error;

use crate::{
    TaskId,
    TaskSnapshotError,
    TerminalTailLimit,
    UrnScheme,
};

/// A recovery checkpoint that violates its bound or its task invariants.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RecoveryCheckpointError<S: UrnScheme> {
    /// A retained task's lifecycle fields disagree.
    #[error("task {task} has an invalid lifecycle: {source}")]
    InvalidLifecycle {
        /// The offending task.
        task: TaskId<S>,
        /// The lifecycle disagreement.
        source: TaskSnapshotError,
    },
    /// More terminal tasks than the checkpoint's tail limit.
    #[error("checkpoint exceeds its terminal tail limit of {limit}", limit = .limit.get())]
    TerminalLimitExceeded {
        /// The exceeded limit.
        limit: TerminalTailLimit,
    },
    /// A task is stored under another task's identity.
    #[error("task {task} is keyed as {key}")]
    TaskIdentityMismatch {
        /// The identity the entry was stored under.
        key: TaskId<S>,
        /// The identity the task itself carries.
        task: TaskId<S>,
    },
    /// The same task appears more than once.
    #[error("task {0} appears more than once")]
    DuplicateTask(TaskId<S>),
}
