//! One sequenced activity record about one task.

use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    ActivityDurability,
    ActivityPayload,
    ActivitySeq,
    TaskId,
    UrnScheme,
};

/// One application activity payload, positioned in an ordered stream and
/// attributed to one task.
///
/// `at` is the observation time in milliseconds since the Unix epoch. It is
/// informational: wall clocks regress, so ordering is the sequence's job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(serialize = "P: Serialize", deserialize = "P: Deserialize<'de>"))]
pub struct OrderedActivity<S: UrnScheme, P> {
    task: TaskId<S>,
    seq: ActivitySeq,
    at: u64,
    payload: P,
}

impl<S: UrnScheme, P> OrderedActivity<S, P> {
    /// Attribute `payload` to `task` at stream position `seq`.
    #[must_use]
    pub const fn new(task: TaskId<S>, seq: ActivitySeq, at: u64, payload: P) -> Self {
        Self {
            task,
            seq,
            at,
            payload,
        }
    }

    /// The task this record is about.
    #[must_use]
    pub const fn task(&self) -> &TaskId<S> {
        &self.task
    }

    /// The record's stream position.
    #[must_use]
    pub const fn seq(&self) -> ActivitySeq {
        self.seq
    }

    /// The observation time, in milliseconds since the Unix epoch.
    #[must_use]
    pub const fn at(&self) -> u64 {
        self.at
    }

    /// The application payload.
    #[must_use]
    pub const fn payload(&self) -> &P {
        &self.payload
    }

    /// Give up the record, keeping only its payload.
    #[must_use]
    pub fn into_payload(self) -> P {
        self.payload
    }

    /// Whether this record comes strictly before `other` in the stream.
    #[must_use]
    pub fn precedes<Q>(&self, other: &OrderedActivity<S, Q>) -> bool {
        self.seq < other.seq
    }
}

impl<S: UrnScheme, P: ActivityPayload> OrderedActivity<S, P> {
    /// The durability class the payload declares.
    #[must_use]
    pub fn durability(&self) -> ActivityDurability {
        self.payload.durability()
    }
}
