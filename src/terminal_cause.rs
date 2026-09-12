//! Why a task ended: the typed cause every terminal transition carries.
//!
//! A status says *where* a task ended; the cause says *why*, and which
//! party that is attributed to. Two tasks are equally `Failed` whether a
//! budget ran out or an executor panicked — what differs is who answers
//! for the work consumed, so the cause is a fact of the transition, not
//! a second status.

use std::fmt;

use serde::{
    Deserialize,
    Serialize,
};

use crate::lifecycle::TaskStatus;

/// The party a terminal outcome is attributed to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FaultSide {
    /// The principal that asked for the work.
    Caller,
    /// The operator that ran it.
    Provider,
}

impl FaultSide {
    /// Stable label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Caller => "caller",
            Self::Provider => "provider",
        }
    }
}

/// Who imposed the wall-clock deadline a task ran past.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeadlineAuthority {
    /// The caller set the deadline on the request.
    Caller,
    /// The provider's own ceiling for that kind of work applied.
    Provider,
}

/// The resource whose hard budget a task exhausted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetResource {
    /// The subtree's wall-clock budget.
    WallTime,
    /// The subtree's token budget.
    Tokens,
}

/// The event that first ended a task, before any cascade.
///
/// Every variant knows the terminal status it ends in and the party it
/// is attributed to, so those two facts can never be recorded
/// inconsistently with the cause.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "cause", rename_all = "snake_case")]
pub enum OriginatingCause {
    /// The caller cancelled the work explicitly.
    CallerCancelled,
    /// The owning session closed, taking its trees with it.
    SessionClosed,
    /// A wall-clock deadline was reached.
    Deadline {
        /// Who set it.
        imposed_by: DeadlineAuthority,
    },
    /// A hard subtree budget was exhausted.
    BudgetExhausted {
        /// Which one.
        resource: BudgetResource,
    },
    /// The executor reported a failure.
    ExecutorFailed {
        /// Whose fault the executor classified it as.
        fault: FaultSide,
    },
    /// The progress watchdog saw no attributed progress in time.
    ProgressStalled,
    /// The node executing the task became unreachable.
    NodeLost,
    /// The host withdrew the resource the task was running on — it
    /// unloaded the workspace, moved the shard, or shut the executor
    /// down — while the task was still live.
    HostEvicted,
    /// The host restarted before the task completed.
    HostRestarted,
}

impl OriginatingCause {
    /// Every originating cause, for exhaustive tests.
    pub const ALL: [Self; 12] = [
        Self::CallerCancelled,
        Self::SessionClosed,
        Self::Deadline {
            imposed_by: DeadlineAuthority::Caller,
        },
        Self::Deadline {
            imposed_by: DeadlineAuthority::Provider,
        },
        Self::BudgetExhausted {
            resource: BudgetResource::WallTime,
        },
        Self::BudgetExhausted {
            resource: BudgetResource::Tokens,
        },
        Self::ExecutorFailed {
            fault: FaultSide::Caller,
        },
        Self::ExecutorFailed {
            fault: FaultSide::Provider,
        },
        Self::ProgressStalled,
        Self::NodeLost,
        Self::HostEvicted,
        Self::HostRestarted,
    ];

    /// The terminal status this cause ends the task in.
    ///
    /// Hard limits — a deadline, an exhausted budget, an executor
    /// failure, a lost host — end in `Failed`: the work did not
    /// complete and nothing chose to stop it. An explicit stop, a
    /// closed session, a stalled watchdog, a lost node, or a host
    /// withdrawing the task's resource end in `Cancelled`.
    #[must_use]
    pub const fn terminal_status(self) -> TaskStatus {
        match self {
            Self::Deadline { .. }
            | Self::BudgetExhausted { .. }
            | Self::ExecutorFailed { .. }
            | Self::HostRestarted => TaskStatus::Failed,
            Self::CallerCancelled
            | Self::SessionClosed
            | Self::ProgressStalled
            | Self::NodeLost
            | Self::HostEvicted => TaskStatus::Cancelled,
        }
    }

    /// The party this cause is attributed to.
    ///
    /// A caller pays for what it asked for and then stopped, capped, or
    /// mis-specified; a provider answers for what it failed to deliver.
    #[must_use]
    pub const fn fault_side(self) -> FaultSide {
        match self {
            Self::CallerCancelled | Self::SessionClosed | Self::BudgetExhausted { .. } => {
                FaultSide::Caller
            }
            Self::Deadline { imposed_by } => match imposed_by {
                DeadlineAuthority::Caller => FaultSide::Caller,
                DeadlineAuthority::Provider => FaultSide::Provider,
            },
            Self::ExecutorFailed { fault } => fault,
            Self::ProgressStalled | Self::NodeLost | Self::HostEvicted | Self::HostRestarted => {
                FaultSide::Provider
            }
        }
    }

    /// Stable label of the cause kind.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CallerCancelled => "caller_cancelled",
            Self::SessionClosed => "session_closed",
            Self::Deadline { .. } => "deadline",
            Self::BudgetExhausted { .. } => "budget_exhausted",
            Self::ExecutorFailed { .. } => "executor_failed",
            Self::ProgressStalled => "progress_stalled",
            Self::NodeLost => "node_lost",
            Self::HostEvicted => "host_evicted",
            Self::HostRestarted => "host_restarted",
        }
    }

    /// This cause as the task's own, direct terminal cause.
    #[must_use]
    pub const fn direct(self) -> TerminalCause {
        TerminalCause {
            origin: self,
            cascaded: false,
        }
    }

    /// This cause as inherited by a descendant the cascade stopped.
    #[must_use]
    pub const fn cascaded(self) -> TerminalCause {
        TerminalCause {
            origin: self,
            cascaded: true,
        }
    }
}

/// Why one task ended: the originating event, and whether this task
/// was its subject or collateral stopped by the cascade from it.
///
/// A cascade never nests: a descendant stopped because its parent was
/// stopped records the *originating* cause, so the whole tree settles
/// under one attribution however deep the stop propagated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TerminalCause {
    origin: OriginatingCause,
    cascaded: bool,
}

impl TerminalCause {
    /// The event that ended this task or the tree it belongs to.
    #[must_use]
    pub const fn origin(self) -> OriginatingCause {
        self.origin
    }

    /// Whether this task was stopped by the cascade rather than being
    /// the cause's subject.
    #[must_use]
    pub const fn is_cascaded(self) -> bool {
        self.cascaded
    }

    /// The cause a descendant stopped by this task's cascade records.
    #[must_use]
    pub const fn for_descendant(self) -> Self {
        self.origin.cascaded()
    }

    /// The terminal status this cause ends the task in.
    ///
    /// Collateral is always `Cancelled`: the descendant did nothing
    /// wrong and was not the hard limit's subject.
    #[must_use]
    pub const fn terminal_status(self) -> TaskStatus {
        if self.cascaded {
            TaskStatus::Cancelled
        } else {
            self.origin.terminal_status()
        }
    }

    /// The party the outcome is attributed to — the origin's, cascade
    /// or not.
    #[must_use]
    pub const fn fault_side(self) -> FaultSide {
        self.origin.fault_side()
    }
}

impl fmt::Display for FaultSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for OriginatingCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for TerminalCause {
    /// The origin label, prefixed with `cascaded from` when this task
    /// was collateral of the cascade.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.cascaded {
            write!(f, "cascaded from {}", self.origin)
        } else {
            fmt::Display::fmt(&self.origin, f)
        }
    }
}
