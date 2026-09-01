//! Generic progress value and composition invariants.

use tasktree::{
    ProgressUnit,
    TaskProgress,
};

#[test]
fn same_unit_progress_composes_without_losing_unknown_totals() {
    let known = TaskProgress {
        processed: 4,
        total: Some(10),
        unit: ProgressUnit::Items,
    };
    let unknown = TaskProgress {
        processed: 7,
        total: None,
        unit: ProgressUnit::Items,
    };

    let aggregate = TaskProgress::aggregate([&known, &unknown]).expect("same unit");

    assert_eq!(aggregate.processed, 11);
    assert_eq!(aggregate.total, None);
    assert_eq!(aggregate.unit, ProgressUnit::Items);
}

#[test]
fn different_progress_units_cannot_be_composed() {
    let bytes = TaskProgress {
        processed: 1,
        total: None,
        unit: ProgressUnit::Bytes,
    };
    let rows = TaskProgress {
        processed: 1,
        total: None,
        unit: ProgressUnit::Rows,
    };

    assert!(TaskProgress::aggregate([&bytes, &rows]).is_none());
}

#[test]
fn progress_deserialization_preserves_the_typed_unit() {
    let progress = TaskProgress {
        processed: 5,
        total: Some(8),
        unit: ProgressUnit::Quads,
    };
    let encoded = serde_json::to_string(&progress).unwrap();

    assert_eq!(
        serde_json::from_str::<TaskProgress>(&encoded).unwrap(),
        progress
    );
}
