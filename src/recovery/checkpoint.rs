//! The validated recovery checkpoint value.

use std::collections::BTreeMap;

use super::{
    RecoveredTask,
    RecoveryCheckpointBuilder,
    RecoveryCheckpointError,
};
use crate::{
    ActivitySeq,
    TaskId,
    TerminalTailLimit,
    UrnScheme,
};

/// A typed, bounded recovery snapshot: every retained task plus the stream
/// position from which incremental records must be replayed.
///
/// Invariants, enforced by [`try_new`](Self::try_new), the
/// [`RecoveryCheckpointBuilder`] and deserialization alike: every task is
/// keyed by its own identity and appears once, every lifecycle passes
/// [`TaskStatus::validate_snapshot`](crate::TaskStatus::validate_snapshot),
/// and at most [`TerminalTailLimit`] tasks are terminal. Deserialization
/// streams task entries and fails on the first offending one, so a corrupt
/// or oversized checkpoint is refused without being read in full.
///
/// The wire form is `{ "cursor", "tail_limit", "tasks": { id: task } }`, in
/// that order: the limit must be known before the first task arrives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryCheckpoint<S: UrnScheme, T> {
    cursor: ActivitySeq,
    tail_limit: TerminalTailLimit,
    terminal_count: usize,
    // bounded: see `RecoveryCheckpointBuilder::tasks`.
    tasks: BTreeMap<TaskId<S>, T>,
}

impl<S: UrnScheme, T: RecoveredTask<S>> RecoveryCheckpoint<S, T> {
    /// Validate `tasks` into a checkpoint covering every record before
    /// `cursor`.
    ///
    /// # Errors
    ///
    /// Returns the first invariant violation; see
    /// [`RecoveryCheckpointBuilder::insert`].
    pub fn try_new(
        cursor: ActivitySeq,
        tail_limit: TerminalTailLimit,
        tasks: impl IntoIterator<Item = T>,
    ) -> Result<Self, RecoveryCheckpointError<S>> {
        let mut builder = RecoveryCheckpointBuilder::new(cursor, tail_limit);
        for task in tasks {
            builder.insert(task.id().clone(), task)?;
        }
        Ok(builder.build())
    }
}

impl<S: UrnScheme, T> RecoveryCheckpoint<S, T> {
    pub(super) const fn from_validated(
        cursor: ActivitySeq,
        tail_limit: TerminalTailLimit,
        terminal_count: usize,
        tasks: BTreeMap<TaskId<S>, T>,
    ) -> Self {
        Self {
            cursor,
            tail_limit,
            terminal_count,
            tasks,
        }
    }

    /// The first stream position *not* reflected in this checkpoint;
    /// replay resumes here.
    #[must_use]
    pub const fn cursor(&self) -> ActivitySeq {
        self.cursor
    }

    /// Whether the record at `seq` is already reflected in this checkpoint.
    #[must_use]
    pub fn covers(&self, seq: ActivitySeq) -> bool {
        seq < self.cursor
    }

    /// The terminal tail limit this checkpoint was validated against.
    #[must_use]
    pub const fn tail_limit(&self) -> TerminalTailLimit {
        self.tail_limit
    }

    /// The number of retained terminal tasks.
    #[must_use]
    pub const fn terminal_count(&self) -> usize {
        self.terminal_count
    }

    /// The number of retained tasks.
    #[must_use]
    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    /// Whether no task is retained.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// The retained task with identity `id`.
    #[must_use]
    pub fn get(&self, id: &TaskId<S>) -> Option<&T> {
        self.tasks.get(id)
    }

    /// The retained tasks in identity order.
    pub fn tasks(&self) -> impl ExactSizeIterator<Item = &T> {
        self.tasks.values()
    }

    /// Give up the checkpoint, yielding its tasks in identity order.
    pub fn into_tasks(self) -> impl ExactSizeIterator<Item = T> {
        self.tasks.into_values()
    }

    pub(super) const fn entries(&self) -> &BTreeMap<TaskId<S>, T> {
        &self.tasks
    }
}
