//! Task lifecycle status and validated transitions.

use serde::{
    Deserialize,
    Deserializer,
    Serialize,
};
use thiserror::Error;

use crate::terminal_cause::{
    OriginatingCause,
    TerminalCause,
};

/// Lifecycle status of a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Task has been admitted but not yet queued or started.
    Admitted,
    /// Task is waiting in the admission queue.
    Queued,
    /// Task is actively executing.
    Running,
    /// The task has entered its irreversible sealing region.
    Sealing,
    /// Task completed successfully.
    Done,
    /// Task failed with an error.
    Failed,
    /// Task was cancelled before completion.
    Cancelled,
}

impl TaskStatus {
    /// Every lifecycle status in declaration order.
    pub const ALL: [Self; 7] = [
        Self::Admitted,
        Self::Queued,
        Self::Running,
        Self::Sealing,
        Self::Done,
        Self::Failed,
        Self::Cancelled,
    ];

    /// Returns `true` if this status is terminal.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Done | Self::Failed | Self::Cancelled)
    }

    /// Returns a static string label for the status.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Admitted => "admitted",
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Sealing => "sealing",
            Self::Done => "done",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    /// Parse a status from its canonical label.
    ///
    /// # Errors
    ///
    /// Returns the unrecognized label when no variant matches.
    pub fn parse_label(label: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|status| status.as_str() == label)
            .ok_or_else(|| format!("unknown task status {label:?}"))
    }

    /// Whether a transition into this status must carry a
    /// [`TerminalCause`]: `Failed` and `Cancelled` do, nothing else may.
    #[must_use]
    pub const fn requires_cause(self) -> bool {
        matches!(self, Self::Failed | Self::Cancelled)
    }

    /// Validate the *edge* of a lifecycle transition, ignoring the
    /// cause a terminal edge must carry.
    ///
    /// Use this to ask whether a move is possible at all; building the
    /// move is [`TaskTransition::try_new`], which also demands the
    /// cause.
    ///
    /// # Errors
    ///
    /// Rejects every edge outside the task state machine.
    pub const fn validate_transition(self, next: Self) -> Result<(), TaskTransitionError> {
        if TaskTransition::edge_is_legal(self, next) {
            Ok(())
        } else {
            Err(TaskTransitionError::IllegalEdge {
                from: self,
                to: next,
            })
        }
    }
}

/// A refused task lifecycle transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum TaskTransitionError {
    /// The edge is outside the state machine.
    #[error("invalid task transition from {from} to {to}", from = .from.as_str(), to = .to.as_str())]
    IllegalEdge {
        /// The current status.
        from: TaskStatus,
        /// The refused target status.
        to: TaskStatus,
    },
    /// A terminal edge was built without saying why.
    #[error("terminal transition to {to} carries no cause", to = .to.as_str())]
    MissingCause {
        /// The terminal status reached without a cause.
        to: TaskStatus,
    },
    /// A non-terminal edge, or the one into `Done`, carried a cause.
    #[error("transition to {to} cannot carry a terminal cause", to = .to.as_str())]
    UnexpectedCause {
        /// The target status the cause was offered for.
        to: TaskStatus,
    },
    /// The cause ends a task in a different status than the edge names.
    #[error(
        "terminal cause {cause} ends a task in {expected}, not {to}",
        cause = .cause.as_str(),
        expected = .expected.as_str(),
        to = .to.as_str()
    )]
    CauseMismatch {
        /// The cause offered.
        cause: OriginatingCause,
        /// The status that cause ends in.
        expected: TaskStatus,
        /// The status the edge named.
        to: TaskStatus,
    },
}

/// One validated lifecycle transition: a legal edge, carrying the
/// [`TerminalCause`] exactly when it ends in `Failed` or `Cancelled`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TaskTransition {
    from: TaskStatus,
    to: TaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    cause: Option<TerminalCause>,
}

impl TaskTransition {
    /// Validate and construct one state-machine edge.
    ///
    /// A move into `Failed` or `Cancelled` must carry a cause whose own
    /// [`terminal_status`](TerminalCause::terminal_status) is that
    /// status; every other move must carry none.
    ///
    /// # Errors
    ///
    /// Rejects illegal edges, including every transition out of a
    /// terminal state, and every mismatch between the edge and its
    /// cause.
    pub const fn try_new(
        from: TaskStatus,
        to: TaskStatus,
        cause: Option<TerminalCause>,
    ) -> Result<Self, TaskTransitionError> {
        if !Self::edge_is_legal(from, to) {
            return Err(TaskTransitionError::IllegalEdge { from, to });
        }
        match (to.requires_cause(), cause) {
            (true, None) => Err(TaskTransitionError::MissingCause { to }),
            (false, Some(_)) => Err(TaskTransitionError::UnexpectedCause { to }),
            (true, Some(cause)) => {
                let expected = cause.terminal_status();
                if expected as u8 != to as u8 {
                    return Err(TaskTransitionError::CauseMismatch {
                        cause: cause.origin(),
                        expected,
                        to,
                    });
                }
                Ok(Self {
                    from,
                    to,
                    cause: Some(cause),
                })
            }
            (false, None) => Ok(Self {
                from,
                to,
                cause: None,
            }),
        }
    }

    /// The state machine's legal edges. A task may fail before it ever
    /// runs — an inherited deadline or an exhausted budget is a hard
    /// limit wherever it lands — but only a running task can succeed,
    /// and the irreversible sealing region refuses cancellation.
    pub(crate) const fn edge_is_legal(from: TaskStatus, to: TaskStatus) -> bool {
        match from {
            TaskStatus::Admitted => matches!(
                to,
                TaskStatus::Queued
                    | TaskStatus::Running
                    | TaskStatus::Failed
                    | TaskStatus::Cancelled
            ),
            TaskStatus::Queued => matches!(
                to,
                TaskStatus::Running | TaskStatus::Failed | TaskStatus::Cancelled
            ),
            TaskStatus::Running => matches!(
                to,
                TaskStatus::Sealing | TaskStatus::Done | TaskStatus::Failed | TaskStatus::Cancelled
            ),
            TaskStatus::Sealing => matches!(to, TaskStatus::Done | TaskStatus::Failed),
            TaskStatus::Done | TaskStatus::Failed | TaskStatus::Cancelled => false,
        }
    }

    /// Return the current status.
    #[must_use]
    pub const fn from(self) -> TaskStatus {
        self.from
    }

    /// Return the target status.
    #[must_use]
    pub const fn to(self) -> TaskStatus {
        self.to
    }

    /// Why the task ended, present exactly when `to` is terminal and
    /// not `Done`.
    #[must_use]
    pub const fn cause(self) -> Option<TerminalCause> {
        self.cause
    }
}

impl<'de> Deserialize<'de> for TaskTransition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct WireTransition {
            from: TaskStatus,
            to: TaskStatus,
            #[serde(default)]
            cause: Option<TerminalCause>,
        }

        let wire = WireTransition::deserialize(deserializer)?;
        Self::try_new(wire.from, wire.to, wire.cause).map_err(serde::de::Error::custom)
    }
}
