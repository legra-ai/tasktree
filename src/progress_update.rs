//! A single typed task-progress observation.

use crate::TaskEventLevel;

/// A single progress observation for a task.
///
/// Bundles counters with the optional step label that produced them, so one
/// progress emission stays one event rather than duplicating its label as a
/// separate step record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskProgressUpdate {
    processed: u64,
    total: Option<u64>,
    level: TaskEventLevel,
    label: Option<String>,
}

impl TaskProgressUpdate {
    /// Create a bare observation with no total or label at the default level.
    #[must_use]
    pub fn new(processed: u64) -> Self {
        Self {
            processed,
            total: None,
            level: TaskEventLevel::default(),
            label: None,
        }
    }

    /// Attach the total, when the producing stage knows one.
    #[must_use]
    pub fn with_total(mut self, total: Option<u64>) -> Self {
        self.total = total;
        self
    }

    /// Set the verbosity tier of the producing call site.
    #[must_use]
    pub fn with_level(mut self, level: TaskEventLevel) -> Self {
        self.level = level;
        self
    }

    /// Attach the step label describing the counted work.
    #[must_use]
    pub fn with_label(mut self, label: Option<String>) -> Self {
        self.label = label;
        self
    }

    /// Items processed so far.
    #[must_use]
    pub fn processed(&self) -> u64 {
        self.processed
    }

    /// Total items, when known.
    #[must_use]
    pub fn total(&self) -> Option<u64> {
        self.total
    }

    /// Verbosity tier of the producing call site.
    #[must_use]
    pub fn level(&self) -> TaskEventLevel {
        self.level
    }

    /// Step label describing the counted work.
    #[must_use]
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Consume the update, yielding its label.
    #[must_use]
    pub fn into_label(self) -> Option<String> {
        self.label
    }
}

#[cfg(test)]
mod tests {
    use super::TaskProgressUpdate;
    use crate::TaskEventLevel;

    #[test]
    fn builder_defaults_are_bare() {
        let update = TaskProgressUpdate::new(7);

        assert_eq!(update.processed(), 7);
        assert_eq!(update.total(), None);
        assert_eq!(update.level(), TaskEventLevel::Info);
        assert_eq!(update.label(), None);
    }

    #[test]
    fn consuming_an_update_yields_its_label() {
        let update = TaskProgressUpdate::new(1).with_label(Some("hydrate".to_owned()));

        assert_eq!(update.into_label().as_deref(), Some("hydrate"));
    }
}
