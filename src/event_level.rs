//! Verbosity level carried by task activity.

use serde::{
    Deserialize,
    Serialize,
};

/// Verbosity tier of a task activity event.
///
/// This is orthogonal to audience visibility: visibility controls who may see
/// an event, while level controls how verbose that event is. Subscriptions can
/// carry a minimum level and drop more verbose events before transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskEventLevel {
    /// The event reports a failure.
    Error,
    /// The event reports something suspicious but non-fatal.
    Warn,
    /// Normal narrative tier (default).
    #[default]
    Info,
    /// High-frequency diagnostic tier.
    Debug,
    /// Maximum-detail tier.
    Trace,
}

impl TaskEventLevel {
    /// Stable string form used in durable records and display flags.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
            Self::Trace => "trace",
        }
    }

    /// Whether this level demands attention regardless of verbosity.
    #[must_use]
    pub fn is_at_least_warn(self) -> bool {
        matches!(self, Self::Error | Self::Warn)
    }

    /// Whether this event should be delivered at the requested verbosity.
    ///
    /// `Error` is always delivered; `Info` includes error, warning, and normal
    /// narrative events; `Trace` includes every level.
    #[must_use]
    pub fn visible_at(self, minimum: Self) -> bool {
        self.verbosity_rank() <= minimum.verbosity_rank()
    }

    fn verbosity_rank(self) -> u8 {
        match self {
            Self::Error => 0,
            Self::Warn => 1,
            Self::Info => 2,
            Self::Debug => 3,
            Self::Trace => 4,
        }
    }
}

impl std::str::FromStr for TaskEventLevel {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "error" => Ok(Self::Error),
            "warn" => Ok(Self::Warn),
            "info" => Ok(Self::Info),
            "debug" => Ok(Self::Debug),
            "trace" => Ok(Self::Trace),
            other => Err(format!("unknown task event level: {other}")),
        }
    }
}

impl std::fmt::Display for TaskEventLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::TaskEventLevel;

    #[test]
    fn default_is_info() {
        assert_eq!(TaskEventLevel::default(), TaskEventLevel::Info);
    }

    #[test]
    fn trace_subscriber_sees_everything() {
        for level in [
            TaskEventLevel::Error,
            TaskEventLevel::Warn,
            TaskEventLevel::Info,
            TaskEventLevel::Debug,
            TaskEventLevel::Trace,
        ] {
            assert!(level.visible_at(TaskEventLevel::Trace));
        }
    }

    #[test]
    fn only_warn_and_error_demand_attention() {
        assert!(TaskEventLevel::Error.is_at_least_warn());
        assert!(TaskEventLevel::Warn.is_at_least_warn());
        assert!(!TaskEventLevel::Info.is_at_least_warn());
        assert!(!TaskEventLevel::Debug.is_at_least_warn());
        assert!(!TaskEventLevel::Trace.is_at_least_warn());
    }

    #[test]
    fn stable_string_form_round_trips() {
        for level in [
            TaskEventLevel::Error,
            TaskEventLevel::Warn,
            TaskEventLevel::Info,
            TaskEventLevel::Debug,
            TaskEventLevel::Trace,
        ] {
            assert_eq!(TaskEventLevel::from_str(level.as_str()), Ok(level));
            assert_eq!(level.to_string(), level.as_str());
        }
    }

    #[test]
    fn unknown_string_is_rejected() {
        assert!(TaskEventLevel::from_str("verbose").is_err());
    }
}
