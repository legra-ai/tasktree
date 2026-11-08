//! Completion, reset and arithmetic boundaries preserve concrete observations.

use tasktree::{
    ProgressPublishDecision as Decision,
    ProgressPublishState,
};

#[test]
fn duplicate_completion_is_suppressed_until_it_qualifies_as_an_interval_report() {
    let mut state = ProgressPublishState::default();
    state.record_published(Some("work"), 8, Some(8), 0);
    assert_eq!(
        state.decide(Some("work"), 8, Some(8), 0),
        Decision::Suppressed
    );
    assert_eq!(
        state.decide(Some("work"), 8, Some(8), 999),
        Decision::Suppressed
    );
    assert_eq!(
        state.decide(Some("work"), 8, Some(8), 1_000),
        Decision::IntervalElapsed
    );
}

#[test]
fn backwards_counter_keeps_completion_history_when_label_and_scale_match() {
    let mut state = ProgressPublishState::default();
    state.record_published(Some("work"), 8, Some(8), 0);
    assert_eq!(
        state.decide(Some("work"), 4, Some(8), 10),
        Decision::CounterReset
    );
    state.record_published(Some("work"), 4, Some(8), 10);
    assert_eq!(
        state.decide(Some("work"), 8, Some(8), 11),
        Decision::Suppressed
    );
    assert_eq!(
        state.decide(Some("work"), 8, Some(8), 1_010),
        Decision::PercentChanged
    );
}

#[test]
fn a_new_label_or_scale_rearms_the_one_time_completion_observation() {
    let mut state = ProgressPublishState::default();
    state.record_published(Some("first"), 8, Some(8), 0);
    state.record_published(Some("next"), 1, Some(8), 1);
    assert_eq!(
        state.decide(Some("next"), 8, Some(8), 2),
        Decision::Completed
    );
    state.record_published(Some("next"), 8, Some(8), 2);
    state.record_published(Some("next"), 1, Some(9), 3);
    assert_eq!(
        state.decide(Some("next"), 9, Some(9), 4),
        Decision::Completed
    );
}

#[test]
fn passing_the_total_is_not_the_same_as_observing_exact_completion() {
    let mut state = ProgressPublishState::default();
    state.record_published(None, 9, Some(10), 0);
    assert_eq!(state.decide(None, 11, Some(10), 1), Decision::Suppressed);
    assert_eq!(
        state.decide(None, 11, Some(10), 1_000),
        Decision::PercentChanged
    );
}

#[test]
fn absent_or_zero_total_has_no_percentage_and_changing_between_them_resets() {
    let mut state = ProgressPublishState::default();
    state.record_published(None, 0, Some(0), 0);
    assert_eq!(
        state.decide(None, 1, Some(0), 1_000),
        Decision::IntervalElapsed
    );
    assert_eq!(state.decide(None, 0, None, 0), Decision::CounterReset);
    state.record_published(None, 0, None, 0);
    assert_eq!(state.decide(None, 0, Some(0), 0), Decision::CounterReset);
    assert_eq!(
        state.decide(None, u64::MAX, None, 1_000),
        Decision::IntervalElapsed
    );
}

#[test]
fn large_scale_movement_keeps_precise_percentages_without_intermediate_overflow() {
    let mut state = ProgressPublishState::default();
    state.record_published(None, u64::MAX / 100 + 1, Some(u64::MAX - 1), 0);
    assert_eq!(
        state.decide(None, u64::MAX - 2, Some(u64::MAX - 1), 1_000),
        Decision::PercentChanged
    );
    assert_eq!(
        state.decide(None, u64::MAX, Some(u64::MAX - 1), 1_000),
        Decision::PercentChanged
    );
}

#[test]
fn clock_rewind_and_epoch_maximum_use_saturating_elapsed_time() {
    let mut state = ProgressPublishState::default();
    state.record_published(None, 1, None, 5_000);
    assert_eq!(state.decide(None, 2, None, 0), Decision::Suppressed);
    assert_eq!(state.decide(None, 2, None, 5_999), Decision::Suppressed);
    assert_eq!(
        state.decide(None, 2, None, 6_000),
        Decision::IntervalElapsed
    );
    state.record_published(None, 1, None, u64::MAX - 500);
    assert_eq!(state.decide(None, 2, None, u64::MAX), Decision::Suppressed);
    assert_eq!(state.decide(None, 2, None, 0), Decision::Suppressed);
}

#[test]
fn valid_large_counters_preserve_percentage_movement_across_the_full_u64_range() {
    let mut state = ProgressPublishState::default();
    state.record_published(Some("work"), u64::MAX / 4, Some(u64::MAX), 0);
    assert_eq!(
        state.decide(Some("work"), u64::MAX / 2, Some(u64::MAX), 1_000),
        Decision::PercentChanged,
    );
}

#[test]
fn adjacent_large_counters_distinguish_a_real_percentage_boundary_from_a_stable_bucket() {
    let mut state = ProgressPublishState::default();
    state.record_published(None, u64::MAX / 2, Some(u64::MAX), 0);
    assert_eq!(
        state.decide(None, u64::MAX / 2 + 1, Some(u64::MAX), 1_000),
        Decision::PercentChanged
    );
    state.record_published(None, u64::MAX / 2 + 1, Some(u64::MAX), 1_000);
    assert_eq!(
        state.decide(None, u64::MAX / 2 + 2, Some(u64::MAX), 2_000),
        Decision::IntervalElapsed
    );
}

#[test]
fn above_total_counters_retain_percentage_movement_without_narrowing_the_result() {
    let mut state = ProgressPublishState::default();
    state.record_published(None, u64::MAX - 1, Some(1), 0);
    assert_eq!(
        state.decide(None, u64::MAX, Some(1), 1_000),
        Decision::PercentChanged
    );
}
