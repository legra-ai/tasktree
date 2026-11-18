//! The bound on terminal tasks a registry or checkpoint retains.

use std::num::NonZeroUsize;

use serde::{
    Deserialize,
    Serialize,
};
use thiserror::Error;

/// How many terminal tasks may be retained beside the nonterminal ones.
///
/// Recovery state is all nonterminal work plus a fixed terminal tail;
/// complete history lives behind a paged interface, never in memory. The
/// limit is never zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TerminalTailLimit(NonZeroUsize);

impl TerminalTailLimit {
    /// Wrap a non-zero limit.
    #[must_use]
    pub const fn new(limit: NonZeroUsize) -> Self {
        Self(limit)
    }

    /// Validate a raw limit.
    ///
    /// # Errors
    ///
    /// Refuses zero.
    pub const fn try_new(limit: usize) -> Result<Self, TerminalTailLimitError> {
        match NonZeroUsize::new(limit) {
            Some(limit) => Ok(Self(limit)),
            None => Err(TerminalTailLimitError::Zero),
        }
    }

    /// The maximum number of retained terminal tasks.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0.get()
    }

    /// Whether `terminal_count` retained terminal tasks fit the limit.
    #[must_use]
    pub const fn admits(self, terminal_count: usize) -> bool {
        terminal_count <= self.0.get()
    }
}

/// A refused terminal tail limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum TerminalTailLimitError {
    /// A zero limit would retain no terminal outcome at all.
    #[error("terminal tail limit must be at least one")]
    Zero,
}
