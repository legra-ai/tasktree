//! Position of one record in an ordered activity stream.

use serde::{
    Deserialize,
    Serialize,
};
use thiserror::Error;

/// The position of one activity record in an ordered stream.
///
/// Sequences are totally ordered and only ever advance by checked
/// increments; a stream that runs out of positions is refused rather than
/// wrapped. Whoever allocates them (a sequencer, a journal) decides the
/// namespace a sequence belongs to; this type only carries the order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ActivitySeq(u64);

impl ActivitySeq {
    /// The first position of every stream.
    pub const FIRST: Self = Self(0);

    /// Wrap a raw stream position.
    #[must_use]
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// The raw stream position.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// The position immediately after this one.
    ///
    /// # Errors
    ///
    /// Refuses to advance past the last representable position.
    pub const fn next(self) -> Result<Self, ActivitySeqError> {
        match self.0.checked_add(1) {
            Some(next) => Ok(Self(next)),
            None => Err(ActivitySeqError::Exhausted),
        }
    }

    /// Check that `next` may follow this position in one ordered stream.
    ///
    /// Gaps are allowed (a stream may be filtered); repeats and
    /// regressions are not.
    ///
    /// # Errors
    ///
    /// Refuses a `next` that does not strictly follow this position.
    pub const fn validate_successor(self, next: Self) -> Result<(), ActivitySeqError> {
        if next.0 > self.0 {
            Ok(())
        } else {
            Err(ActivitySeqError::OutOfOrder {
                previous: self,
                next,
            })
        }
    }

    /// Fold an observed position into this watermark, never moving it
    /// backwards.
    #[must_use]
    pub const fn observe(self, observed: Self) -> Self {
        if observed.0 > self.0 { observed } else { self }
    }
}

/// A refused activity sequence operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ActivitySeqError {
    /// The stream has no position left after `u64::MAX`.
    #[error("activity sequence is exhausted")]
    Exhausted,
    /// A record did not strictly follow its predecessor.
    #[error(
        "activity sequence {next} does not follow {previous}",
        next = .next.get(),
        previous = .previous.get()
    )]
    OutOfOrder {
        /// The position already seen.
        previous: ActivitySeq,
        /// The refused position.
        next: ActivitySeq,
    },
}
