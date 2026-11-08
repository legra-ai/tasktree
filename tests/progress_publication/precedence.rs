//! Independent conflict cases define decision precedence and interval
//! boundaries.

use tasktree::{
    ProgressPublishDecision as Decision,
    ProgressPublishState,
};

#[test]
fn absent_history_precedes_even_an_immediately_complete_zero_total() {
    let state = ProgressPublishState::default();
    assert_eq!(state.decide(Some("empty"), 0, Some(0), 0), Decision::First);
}

#[test]
fn changed_label_precedes_reset_completion_and_clock_rewind() {
    let mut state = ProgressPublishState::default();
    state.record_published(Some("before"), 10, Some(20), 100);
    assert_eq!(
        state.decide(Some("after"), 2, Some(2), 0),
        Decision::StepChanged
    );
    assert_eq!(state.decide(None, 2, Some(2), 0), Decision::StepChanged);
}

#[test]
fn reset_precedes_completion_when_the_scale_changes() {
    let mut state = ProgressPublishState::default();
    state.record_published(Some("work"), 10, Some(20), 100);
    assert_eq!(
        state.decide(Some("work"), 2, Some(2), 100),
        Decision::CounterReset
    );
    assert_eq!(
        state.decide(Some("work"), 30, Some(30), 100),
        Decision::CounterReset
    );
    assert_eq!(
        state.decide(Some("work"), 9, Some(20), 100),
        Decision::CounterReset
    );
}

#[test]
fn first_completion_is_exempt_but_percent_changes_never_bypass_the_interval() {
    let mut state = ProgressPublishState::default();
    state.record_published(Some("work"), 10, Some(100), 5_000);
    assert_eq!(
        state.decide(Some("work"), 100, Some(100), 5_001),
        Decision::Completed
    );
    assert_eq!(
        state.decide(Some("work"), 99, Some(100), 5_999),
        Decision::Suppressed
    );
    assert_eq!(
        state.decide(Some("work"), 11, Some(100), 6_000),
        Decision::PercentChanged
    );
}

#[test]
fn unchanged_percent_reports_interval_elapsed_at_the_exact_boundary() {
    let mut state = ProgressPublishState::default();
    state.record_published(None, 10, Some(1_000), 40);
    assert_eq!(
        state.decide(None, 11, Some(1_000), 1_039),
        Decision::Suppressed
    );
    assert_eq!(
        state.decide(None, 11, Some(1_000), 1_040),
        Decision::IntervalElapsed
    );
}

#[test]
fn absent_and_empty_labels_are_distinct_and_borrowed_labels_are_owned_when_recorded() {
    let mut state = ProgressPublishState::default();
    state.record_published(None, 1, None, 0);
    assert_eq!(state.decide(Some(""), 1, None, 0), Decision::StepChanged);
    let mut label = String::from("stored");
    state.record_published(Some(&label), 1, None, 0);
    label.clear();
    assert_eq!(
        state.decide(Some("stored"), 2, None, 0),
        Decision::Suppressed
    );
    assert_eq!(
        state.decide(Some(&label), 2, None, 0),
        Decision::StepChanged
    );
}
