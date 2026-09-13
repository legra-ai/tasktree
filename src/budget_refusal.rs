//! Why a budget could not authorize what was asked of it.

use thiserror::Error;

use crate::terminal_cause::BudgetResource;

/// A budget refused a reservation, a release, or a composed amount.
///
/// Both variants name the resource, so a refusal can be attributed and
/// settled without re-deriving which envelope it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Error)]
pub enum BudgetRefusal {
    /// More was asked than the budget holds.
    #[error("{resource} budget exceeded: required {required}, available {available}")]
    Exceeded {
        /// The resource whose budget was exceeded.
        resource: BudgetResource,
        /// What was asked.
        required: u64,
        /// What the budget had left.
        available: u64,
    },
    /// A composed amount cannot be represented.
    #[error("{resource} amount overflowed while composing")]
    Overflow {
        /// The resource whose amount overflowed.
        resource: BudgetResource,
    },
}
