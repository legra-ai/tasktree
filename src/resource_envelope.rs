//! A task's resource envelope: its immutable budget, its planned
//! reservation, and its measured consumption.
//!
//! The three values compose differently by design. A budget is
//! authorization and is never added to anything. Reservations and actuals
//! compose bottom-up from children, and how they compose depends on the
//! resource: wall-clock span follows the execution structure (sequential
//! children sum, parallel children take the maximum), while tokens and
//! wall work are consumed by every child and always sum.

use serde::{
    Deserialize,
    Serialize,
};

use crate::budget_refusal::BudgetRefusal;
use crate::terminal_cause::BudgetResource;
use crate::token_budget::TokenBudget;
use crate::tokens::Tokens;

/// How one interior task schedules its direct children.
///
/// Parentage cannot reveal this distinction; the executor declares it so
/// wall-clock span can follow the critical path while consumptive
/// resources keep adding across every child.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceComposition {
    /// Children execute one after another.
    Sequential,
    /// Children execute concurrently without a tighter concurrency bound.
    Parallel,
}

impl ResourceComposition {
    fn compose_span(self, mut children: impl Iterator<Item = u64>) -> Result<u64, BudgetRefusal> {
        match self {
            Self::Sequential => children.try_fold(0_u64, |sum, child| {
                sum.checked_add(child).ok_or(BudgetRefusal::Overflow {
                    resource: BudgetResource::WallTime,
                })
            }),
            Self::Parallel => Ok(children.max().unwrap_or(0)),
        }
    }
}

fn checked_wall_ms(sum: u64, addend: u64) -> Result<u64, BudgetRefusal> {
    sum.checked_add(addend).ok_or(BudgetRefusal::Overflow {
        resource: BudgetResource::WallTime,
    })
}

fn checked_tokens(sum: Tokens, addend: Tokens) -> Result<Tokens, BudgetRefusal> {
    sum.checked_add(addend).ok_or(BudgetRefusal::Overflow {
        resource: BudgetResource::Tokens,
    })
}

/// Immutable hard authorization for one task's entire subtree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResourceBudget {
    wall_time_ms: u64,
    tokens: Tokens,
}

impl ResourceBudget {
    /// An immutable wall-time and token authorization.
    #[must_use]
    pub const fn new(wall_time_ms: u64, tokens: Tokens) -> Self {
        Self {
            wall_time_ms,
            tokens,
        }
    }

    /// Refuse a composed worst-case reservation that exceeds this budget.
    ///
    /// # Errors
    ///
    /// Returns [`BudgetRefusal::Exceeded`] for the first resource whose
    /// required reservation exceeds its authorization.
    pub fn authorize_reservation(
        self,
        reservation: ResourceReservation,
    ) -> Result<(), BudgetRefusal> {
        self.authorize(reservation.wall_time_ms, reservation.tokens)
    }

    /// Refuse measured subtree actuals that exceed this budget.
    ///
    /// Wall work is deliberately not compared to the wall-span budget:
    /// parallel work consumes more total worker time than elapsed time.
    ///
    /// # Errors
    ///
    /// Returns [`BudgetRefusal::Exceeded`] for the first resource whose
    /// actual consumption exceeds its authorization.
    pub fn authorize_actual(self, actual: ResourceActual) -> Result<(), BudgetRefusal> {
        self.authorize(actual.wall_span_ms, actual.tokens)
    }

    fn authorize(self, wall_time_ms: u64, tokens: Tokens) -> Result<(), BudgetRefusal> {
        if wall_time_ms > self.wall_time_ms {
            return Err(BudgetRefusal::Exceeded {
                resource: BudgetResource::WallTime,
                required: wall_time_ms,
                available: self.wall_time_ms,
            });
        }
        if tokens > self.tokens {
            return Err(BudgetRefusal::Exceeded {
                resource: BudgetResource::Tokens,
                required: tokens.get(),
                available: self.tokens.get(),
            });
        }
        Ok(())
    }

    /// Authorized wall-clock span in milliseconds.
    #[must_use]
    pub const fn wall_time_ms(self) -> u64 {
        self.wall_time_ms
    }

    /// Authorized tokens.
    #[must_use]
    pub const fn tokens(self) -> Tokens {
        self.tokens
    }

    /// The token half of this envelope as a budget that trickles down.
    #[must_use]
    pub const fn token_budget(self) -> TokenBudget {
        TokenBudget::new(self.tokens)
    }
}

/// Worst-case resources reserved by a task plan.
///
/// Direct child reservations compose bottom-up and the result must fit
/// the containing task's [`ResourceBudget`]. A budget is never added to
/// its descendants' reservations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResourceReservation {
    wall_time_ms: u64,
    tokens: Tokens,
}

impl ResourceReservation {
    /// The reservation for a task's exclusive local work.
    #[must_use]
    pub const fn new(wall_time_ms: u64, tokens: Tokens) -> Self {
        Self {
            wall_time_ms,
            tokens,
        }
    }

    /// Compose this task's exclusive reservation with its direct children.
    ///
    /// # Errors
    ///
    /// Returns [`BudgetRefusal::Overflow`] when a composed amount cannot
    /// be represented.
    pub fn compose_children(
        self,
        composition: ResourceComposition,
        children: &[Self],
    ) -> Result<Self, BudgetRefusal> {
        let child_wall_time_ms =
            composition.compose_span(children.iter().map(|child| child.wall_time_ms))?;
        let wall_time_ms = checked_wall_ms(self.wall_time_ms, child_wall_time_ms)?;
        let tokens = children
            .iter()
            .try_fold(self.tokens, |sum, child| checked_tokens(sum, child.tokens))?;
        Ok(Self::new(wall_time_ms, tokens))
    }

    /// Worst-case wall-clock span in milliseconds.
    #[must_use]
    pub const fn wall_time_ms(self) -> u64 {
        self.wall_time_ms
    }

    /// Worst-case reserved tokens.
    #[must_use]
    pub const fn tokens(self) -> Tokens {
        self.tokens
    }
}

/// Measured consumption for one task or composed subtree.
///
/// Wall span and wall work are deliberately separate: parallel children
/// take the maximum span, but every child's work remains consumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResourceActual {
    wall_span_ms: u64,
    wall_work_ms: u64,
    tokens: Tokens,
}

impl ResourceActual {
    /// Actuals for exclusive local work, whose span and work coincide.
    #[must_use]
    pub const fn new(wall_time_ms: u64, tokens: Tokens) -> Self {
        Self {
            wall_span_ms: wall_time_ms,
            wall_work_ms: wall_time_ms,
            tokens,
        }
    }

    /// Compose this task's exclusive local actuals with direct children.
    ///
    /// # Errors
    ///
    /// Returns [`BudgetRefusal::Overflow`] when a composed amount cannot
    /// be represented.
    pub fn compose_children(
        self,
        composition: ResourceComposition,
        children: &[Self],
    ) -> Result<Self, BudgetRefusal> {
        let child_span_ms =
            composition.compose_span(children.iter().map(|child| child.wall_span_ms))?;
        let wall_span_ms = checked_wall_ms(self.wall_span_ms, child_span_ms)?;
        let wall_work_ms = children.iter().try_fold(self.wall_work_ms, |sum, child| {
            checked_wall_ms(sum, child.wall_work_ms)
        })?;
        let tokens = children
            .iter()
            .try_fold(self.tokens, |sum, child| checked_tokens(sum, child.tokens))?;
        Ok(Self {
            wall_span_ms,
            wall_work_ms,
            tokens,
        })
    }

    /// Measured elapsed critical-path span in milliseconds.
    #[must_use]
    pub const fn wall_span_ms(self) -> u64 {
        self.wall_span_ms
    }

    /// Sum of measured wall work across every task in the subtree.
    #[must_use]
    pub const fn wall_work_ms(self) -> u64 {
        self.wall_work_ms
    }

    /// Tokens actually consumed.
    #[must_use]
    pub const fn tokens(self) -> Tokens {
        self.tokens
    }
}
