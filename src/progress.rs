//! Progress counters carried on a running task.

use serde::{
    Deserialize,
    Serialize,
};

use crate::ProgressUnit;

/// Progress counters for a running task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskProgress {
    /// Number of items processed so far.
    pub processed: u64,
    /// Total number of items, if known.
    pub total: Option<u64>,
    /// The unit of measurement.
    pub unit: ProgressUnit,
}

impl TaskProgress {
    /// Create a new progress tracker with zero items processed.
    #[must_use]
    pub fn new(unit: ProgressUnit) -> Self {
        Self {
            processed: 0,
            total: None,
            unit,
        }
    }

    /// Returns the completion fraction in `[0.0, 1.0]`, or `None` if
    /// the total is unknown or zero.
    ///
    /// Counters above 2^52 lose precision in the conversion, which is
    /// acceptable for a display fraction.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn fraction(&self) -> Option<f64> {
        self.total.and_then(|t| {
            if t == 0 {
                None
            } else {
                Some(self.processed as f64 / t as f64)
            }
        })
    }

    /// Aggregate progress counters with the same unit.
    ///
    /// Returns `None` when the iterator is empty or when any item
    /// uses a different unit. Unknown totals are preserved: if any
    /// child total is unknown then the aggregate total is unknown.
    pub fn aggregate<'a>(progress: impl IntoIterator<Item = &'a TaskProgress>) -> Option<Self> {
        let mut iter = progress.into_iter();
        let first = iter.next()?;
        let mut aggregate = first.clone();
        for next in iter {
            if next.unit != aggregate.unit {
                return None;
            }
            aggregate.processed = aggregate.processed.saturating_add(next.processed);
            aggregate.total = match (aggregate.total, next.total) {
                (Some(left), Some(right)) => Some(left.saturating_add(right)),
                _ => None,
            };
        }
        Some(aggregate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_fraction_with_total() {
        let mut p = TaskProgress::new(ProgressUnit::Quads);
        p.processed = 50;
        p.total = Some(100);
        let f = p.fraction().unwrap();
        assert!((f - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn progress_fraction_no_total() {
        let p = TaskProgress::new(ProgressUnit::Bytes);
        assert!(p.fraction().is_none());
    }

    #[test]
    fn progress_fraction_zero_total() {
        let mut p = TaskProgress::new(ProgressUnit::Rows);
        p.total = Some(0);
        assert!(p.fraction().is_none());
    }

    #[test]
    fn aggregate_sums_same_unit_progress() {
        let left = TaskProgress {
            processed: 2,
            total: Some(5),
            unit: ProgressUnit::Quads,
        };
        let right = TaskProgress {
            processed: 3,
            total: Some(7),
            unit: ProgressUnit::Quads,
        };

        let aggregated = TaskProgress::aggregate([&left, &right]).expect("same unit");

        assert_eq!(aggregated.processed, 5);
        assert_eq!(aggregated.total, Some(12));
        assert_eq!(aggregated.unit, ProgressUnit::Quads);
    }

    #[test]
    fn aggregate_rejects_mixed_units() {
        let rows = TaskProgress {
            processed: 2,
            total: Some(5),
            unit: ProgressUnit::Rows,
        };
        let bytes = TaskProgress {
            processed: 3,
            total: Some(7),
            unit: ProgressUnit::Bytes,
        };

        assert!(TaskProgress::aggregate([&rows, &bytes]).is_none());
    }

    #[test]
    fn progress_serde_round_trip() {
        let mut p = TaskProgress::new(ProgressUnit::Items);
        p.processed = 42;
        p.total = Some(100);
        let json = serde_json::to_string(&p).unwrap();
        let parsed: TaskProgress = serde_json::from_str(&json).unwrap();
        assert_eq!(p, parsed);
    }
}
