//! Generic task timestamp invariants.

use tasktree::TaskTimestamps;

#[test]
fn lifecycle_duration_requires_both_terminal_boundaries() {
    let complete = TaskTimestamps {
        created_at: 1_000,
        started_at: Some(2_000),
        completed_at: Some(5_000),
    };
    let running = TaskTimestamps {
        completed_at: None,
        ..complete.clone()
    };

    assert_eq!(complete.duration_ms(), Some(3_000));
    assert_eq!(running.duration_ms(), None);
}

#[test]
fn lifecycle_duration_saturates_a_regressed_wall_clock() {
    let timestamps = TaskTimestamps {
        created_at: 1_000,
        started_at: Some(5_000),
        completed_at: Some(2_000),
    };

    assert_eq!(timestamps.duration_ms(), Some(0));
}

#[test]
fn timestamp_round_trip_preserves_each_boundary() {
    let timestamps = TaskTimestamps {
        created_at: 1_000,
        started_at: Some(2_000),
        completed_at: Some(3_000),
    };
    let encoded = serde_json::to_string(&timestamps).expect("serialize timestamps");
    let decoded = serde_json::from_str::<TaskTimestamps>(&encoded).expect("deserialize timestamps");

    assert_eq!(decoded, timestamps);
}
