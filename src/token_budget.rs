//! One task's token ceiling and the shares it has handed to delegated
//! children.
//!
//! A budget trickles down a tree in two ways. A child that runs inside
//! the parent's own process **attenuates**: it sees everything the parent
//! has left and the parent's pool is untouched, because the parent
//! measures the child's consumption itself. A child that runs elsewhere
//! **reserves**: the parent carves a share out of its pool for it, since
//! nothing else bounds what the remote side may consume. Shares compose
//! by checked sum, so siblings running in parallel split the pool rather
//! than each claiming all of it.

use std::fmt;

use serde::{
    Deserialize,
    Deserializer,
    Serialize,
};

use crate::budget_refusal::BudgetRefusal;
use crate::terminal_cause::BudgetResource;
use crate::tokens::Tokens;

/// An immutable token ceiling for one task's whole subtree, with the
/// running sum of shares reserved by delegated children.
///
/// `reserved` never exceeds `ceiling`; the type refuses to construct or
/// deserialize a value where it would.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct TokenBudget {
    ceiling: Tokens,
    reserved: Tokens,
}

impl TokenBudget {
    /// A fresh budget of `ceiling` tokens with nothing reserved. Zero is
    /// a valid ceiling: zero-priced work is budgeted at zero, not
    /// unbudgeted.
    #[must_use]
    pub const fn new(ceiling: Tokens) -> Self {
        Self {
            ceiling,
            reserved: Tokens::ZERO,
        }
    }

    fn try_from_parts(ceiling: Tokens, reserved: Tokens) -> Result<Self, BudgetRefusal> {
        if reserved > ceiling {
            return Err(BudgetRefusal::Exceeded {
                resource: BudgetResource::Tokens,
                required: reserved.get(),
                available: ceiling.get(),
            });
        }
        Ok(Self { ceiling, reserved })
    }

    /// The immutable ceiling.
    #[must_use]
    pub const fn ceiling(self) -> Tokens {
        self.ceiling
    }

    /// The sum of shares currently reserved by delegated children.
    #[must_use]
    pub const fn reserved(self) -> Tokens {
        self.reserved
    }

    /// What is left to hand out: the ceiling less every reserved share.
    #[must_use]
    pub const fn remaining(self) -> Tokens {
        match self.ceiling.checked_sub(self.reserved) {
            Some(remaining) => remaining,
            // Unreachable by construction: `reserved` never exceeds
            // `ceiling`. Zero is the only answer that keeps a budget a
            // budget.
            None => Tokens::ZERO,
        }
    }

    /// The budget a local child runs under: everything this task has
    /// left. The pool is not reduced; the parent measures the child.
    #[must_use]
    pub const fn attenuate(self) -> Self {
        Self::new(self.remaining())
    }

    /// Carve `share` out of the pool for a delegated child.
    ///
    /// Returns this budget with the share reserved, and the child's own
    /// budget of exactly that share.
    ///
    /// # Errors
    ///
    /// Refuses a share larger than what remains.
    pub fn reserve(self, share: Tokens) -> Result<(Self, Self), BudgetRefusal> {
        let remaining = self.remaining();
        let reserved = self
            .reserved
            .checked_add(share)
            .filter(|reserved| *reserved <= self.ceiling)
            .ok_or(BudgetRefusal::Exceeded {
                resource: BudgetResource::Tokens,
                required: share.get(),
                available: remaining.get(),
            })?;
        Ok((
            Self {
                ceiling: self.ceiling,
                reserved,
            },
            Self::new(share),
        ))
    }

    /// Reserve everything that remains for one delegated child. The
    /// pool is then empty until that share is released.
    #[must_use]
    pub const fn reserve_remaining(self) -> (Self, Self) {
        let share = self.remaining();
        (
            Self {
                ceiling: self.ceiling,
                reserved: self.ceiling,
            },
            Self::new(share),
        )
    }

    /// Hand back a share a delegated child did not use.
    ///
    /// # Errors
    ///
    /// Refuses to release more than is reserved: that would credit the
    /// pool with tokens it never held.
    pub fn release(self, share: Tokens) -> Result<Self, BudgetRefusal> {
        let reserved = self
            .reserved
            .checked_sub(share)
            .ok_or(BudgetRefusal::Exceeded {
                resource: BudgetResource::Tokens,
                required: share.get(),
                available: self.reserved.get(),
            })?;
        Ok(Self {
            ceiling: self.ceiling,
            reserved,
        })
    }
}

impl fmt::Display for TokenBudget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} of {} tokens remaining",
            self.remaining(),
            self.ceiling
        )
    }
}

impl<'de> Deserialize<'de> for TokenBudget {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct WireBudget {
            ceiling: Tokens,
            reserved: Tokens,
        }

        let wire = WireBudget::deserialize(deserializer)?;
        Self::try_from_parts(wire.ceiling, wire.reserved).map_err(serde::de::Error::custom)
    }
}
