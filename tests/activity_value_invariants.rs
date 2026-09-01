//! Generic activity-level and progress-update invariants.

use tasktree::{
    TaskEventLevel,
    TaskProgressUpdate,
};

#[test]
fn activity_levels_apply_a_stable_verbosity_order() {
    let info = TaskEventLevel::Info;

    assert!(TaskEventLevel::Error.visible_at(info));
    assert!(TaskEventLevel::Warn.visible_at(info));
    assert!(TaskEventLevel::Info.visible_at(info));
    assert!(!TaskEventLevel::Debug.visible_at(info));
    assert!(!TaskEventLevel::Trace.visible_at(info));
}

#[test]
fn progress_update_keeps_one_typed_observation() {
    let update = TaskProgressUpdate::new(47_104)
        .with_total(Some(71_392))
        .with_level(TaskEventLevel::Debug)
        .with_label(Some("persist block".to_owned()));

    assert_eq!(update.processed(), 47_104);
    assert_eq!(update.total(), Some(71_392));
    assert_eq!(update.level(), TaskEventLevel::Debug);
    assert_eq!(update.label(), Some("persist block"));
}

#[test]
fn activity_level_round_trip_preserves_the_tier() {
    let encoded = serde_json::to_string(&TaskEventLevel::Trace).expect("serialize level");
    let decoded = serde_json::from_str::<TaskEventLevel>(&encoded).expect("deserialize level");

    assert_eq!(decoded, TaskEventLevel::Trace);
}
