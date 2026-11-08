//! Publication decisions retain essential observations and limit steady
//! reports.

use tasktree::{
    ProgressPublishDecision,
    ProgressPublishState,
};

#[path = "progress_publication/counters.rs"]
mod counters;
#[path = "progress_publication/precedence.rs"]
mod precedence;
#[path = "progress_publication/rates.rs"]
mod rates;

#[test]
fn epoch_zero_first_observation_becomes_a_real_publication() {
    let mut state = ProgressPublishState::default();
    assert_eq!(
        state.decide(None, 1, None, 0),
        ProgressPublishDecision::First
    );
    state.record_published(None, 1, None, 0);
    assert_eq!(
        state.decide(None, 2, None, 0),
        ProgressPublishDecision::Suppressed
    );
}

#[test]
fn decision_classification_distinguishes_transitions_from_steady_reports() {
    // bounded: the complete seven-decision classification table.
    for (decision, publishes, steady) in [
        (ProgressPublishDecision::First, true, false),
        (ProgressPublishDecision::StepChanged, true, false),
        (ProgressPublishDecision::CounterReset, true, false),
        (ProgressPublishDecision::Completed, true, false),
        (ProgressPublishDecision::PercentChanged, true, true),
        (ProgressPublishDecision::IntervalElapsed, true, true),
        (ProgressPublishDecision::Suppressed, false, false),
    ] {
        assert_eq!(decision.publishes(), publishes);
        assert_eq!(decision.is_steady_state(), steady);
    }
}

#[test]
fn deciding_is_pure_and_only_explicit_recording_advances_the_cursor() {
    let mut state = ProgressPublishState::default();
    let before = state.clone();
    assert_eq!(
        state.decide(Some("work"), 7, Some(10), 8),
        ProgressPublishDecision::First
    );
    assert_eq!(
        state.decide(Some("work"), 7, Some(10), 8),
        ProgressPublishDecision::First
    );
    assert_eq!(state, before);
    state.record_published(Some("work"), 7, Some(10), 8);
    assert_ne!(state, before);
    assert_eq!(
        state.decide(Some("work"), 8, Some(10), 9),
        ProgressPublishDecision::Suppressed
    );
}

#[test]
fn cloned_state_retains_its_own_label_and_publication_cursor() {
    let mut state = ProgressPublishState::default();
    state.record_published(Some("first"), 1, None, 5);
    let cloned = state.clone();
    state.record_published(Some("second"), 2, None, 6);
    assert_eq!(
        cloned.decide(Some("first"), 2, None, 7),
        ProgressPublishDecision::Suppressed
    );
    assert_eq!(
        state.decide(Some("first"), 2, None, 7),
        ProgressPublishDecision::StepChanged
    );
}
