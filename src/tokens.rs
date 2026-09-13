//! The abstract token unit a task tree is budgeted in.
//!
//! What a token buys — compute, bytes, a priced factor, a currency — is
//! the application's business. This crate only counts them: non-negative
//! integers with checked arithmetic, because an amount that cannot be
//! represented is an admission error, never a saturated total.

use std::fmt;
use std::num::ParseIntError;
use std::str::FromStr;

use serde::{
    Deserialize,
    Serialize,
};

/// A non-negative count of the application's token unit.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Tokens(u64);

impl Tokens {
    /// The explicit zero amount: a resolved budget of nothing, which is
    /// still a budget.
    pub const ZERO: Self = Self(0);

    /// An amount of `count` tokens.
    #[must_use]
    pub const fn new(count: u64) -> Self {
        Self(count)
    }

    /// The raw count.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Whether this is the zero amount.
    #[must_use]
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    /// The sum, or `None` when it cannot be represented.
    #[must_use]
    pub const fn checked_add(self, other: Self) -> Option<Self> {
        match self.0.checked_add(other.0) {
            Some(sum) => Some(Self(sum)),
            None => None,
        }
    }

    /// The difference, or `None` when `other` exceeds this amount.
    #[must_use]
    pub const fn checked_sub(self, other: Self) -> Option<Self> {
        match self.0.checked_sub(other.0) {
            Some(difference) => Some(Self(difference)),
            None => None,
        }
    }

    /// The sum of every amount, or `None` when it cannot be represented.
    ///
    /// There is deliberately no `Sum` implementation: a summing iterator
    /// that silently wrapped or saturated would hide the overflow this
    /// type exists to surface.
    #[must_use]
    pub fn checked_sum(amounts: impl IntoIterator<Item = Self>) -> Option<Self> {
        amounts.into_iter().try_fold(Self::ZERO, Self::checked_add)
    }
}

impl fmt::Display for Tokens {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl FromStr for Tokens {
    type Err = ParseIntError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        text.parse().map(Self)
    }
}
