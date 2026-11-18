//! Incremental, fail-early assembly of a recovery checkpoint.

use std::collections::BTreeMap;

use super::{
    RecoveredTask,
    RecoveryCheckpoint,
    RecoveryCheckpointError,
};
use crate::{
    ActivitySeq,
    TaskId,
    TerminalTailLimit,
    UrnScheme,
};

/// Builds a [`RecoveryCheckpoint`] one task at a time, refusing the first
/// task that breaks an invariant before any later one is read.
///
/// The limit is a [`TerminalTailLimit`], never a bare count:
///
/// ```compile_fail
/// use tasktree::{ActivitySeq, RecoveryCheckpointBuilder, UrnScheme};
/// fn bare<S: UrnScheme, T: tasktree::RecoveredTask<S>>() {
///     let _ = RecoveryCheckpointBuilder::<S, T>::new(ActivitySeq::FIRST, 1_024);
/// }
/// ```
#[derive(Debug)]
pub struct RecoveryCheckpointBuilder<S: UrnScheme, T> {
    cursor: ActivitySeq,
    tail_limit: TerminalTailLimit,
    terminal_count: usize,
    // bounded: nonterminal entries are bounded by the live registry that
    // produced them; terminal entries by `tail_limit`, checked per insert.
    tasks: BTreeMap<TaskId<S>, T>,
}

impl<S: UrnScheme, T: RecoveredTask<S>> RecoveryCheckpointBuilder<S, T> {
    /// Start an empty checkpoint covering every record before `cursor`.
    #[must_use]
    pub const fn new(cursor: ActivitySeq, tail_limit: TerminalTailLimit) -> Self {
        Self {
            cursor,
            tail_limit,
            terminal_count: 0,
            tasks: BTreeMap::new(),
        }
    }

    /// Retain `task` under `key`.
    ///
    /// # Errors
    ///
    /// Refuses a key that is not the task's own identity, a task already
    /// retained, an invalid lifecycle, and the first terminal task beyond
    /// the tail limit. The builder is unchanged by a refusal.
    pub fn insert(&mut self, key: TaskId<S>, task: T) -> Result<(), RecoveryCheckpointError<S>> {
        if &key != task.id() {
            return Err(RecoveryCheckpointError::TaskIdentityMismatch {
                key,
                task: task.id().clone(),
            });
        }
        if self.tasks.contains_key(&key) {
            return Err(RecoveryCheckpointError::DuplicateTask(key));
        }
        task.validate_lifecycle()
            .map_err(|source| RecoveryCheckpointError::InvalidLifecycle {
                task: key.clone(),
                source,
            })?;
        let terminal_count = self.terminal_count + usize::from(task.is_terminal());
        if !self.tail_limit.admits(terminal_count) {
            return Err(RecoveryCheckpointError::TerminalLimitExceeded {
                limit: self.tail_limit,
            });
        }
        self.terminal_count = terminal_count;
        self.tasks.insert(key, task);
        Ok(())
    }

    /// Finish the checkpoint; every retained task is already validated.
    #[must_use]
    pub fn build(self) -> RecoveryCheckpoint<S, T> {
        RecoveryCheckpoint::from_validated(
            self.cursor,
            self.tail_limit,
            self.terminal_count,
            self.tasks,
        )
    }
}
