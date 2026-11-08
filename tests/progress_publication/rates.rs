//! Long sequences bound steady reports while keeping essential transitions.

use tasktree::{
    ProgressPublishDecision as Decision,
    ProgressPublishState,
};

#[test]
fn fast_unknown_total_sequence_emits_only_its_first_observation() {
    let mut state = ProgressPublishState::default();
    let mut count = 0;
    for processed in 0..=10_000 {
        if state.decide(Some("work"), processed, None, 17).publishes() {
            state.record_published(Some("work"), processed, None, 17);
            count += 1;
        }
    }
    assert_eq!(count, 1);
}

#[test]
fn fast_known_total_sequence_keeps_first_and_exact_completion_only() {
    let mut state = ProgressPublishState::default();
    let mut count = 0;
    for processed in 0..=200 {
        if state.decide(None, processed, Some(200), 0).publishes() {
            state.record_published(None, processed, Some(200), 0);
            count += 1;
        }
    }
    assert_eq!(count, 2);
}

#[test]
fn a_long_unknown_total_sequence_reports_once_per_elapsed_interval() {
    let mut state = ProgressPublishState::default();
    let mut count = 0;
    for now in 0..=5_500 {
        if state.decide(None, now, None, now).publishes() {
            state.record_published(None, now, None, now);
            count += 1;
        }
    }
    assert_eq!(count, 6);
}

#[test]
fn long_known_total_sequence_has_five_steady_reports_plus_first_and_completion() {
    let mut state = ProgressPublishState::default();
    let mut steady = 0;
    let mut transitions = 0;
    for now in 0..=5_500 {
        let decision = state.decide(None, now, Some(5_500), now);
        if !decision.publishes() {
            continue;
        }
        state.record_published(None, now, Some(5_500), now);
        if decision.is_steady_state() {
            steady += 1;
        } else {
            transitions += 1;
        }
    }
    assert_eq!(steady, 5);
    assert_eq!(transitions, 2);
}

#[test]
fn duplicate_completions_do_not_spend_another_transition_publish() {
    let mut state = ProgressPublishState::default();
    let mut count = 0;
    for _ in 0..1_000 {
        if state.decide(None, 1, Some(1), 0).publishes() {
            state.record_published(None, 1, Some(1), 0);
            count += 1;
        }
    }
    assert_eq!(count, 1);
    assert_eq!(
        state.decide(None, 1, Some(1), 1_000),
        Decision::IntervalElapsed
    );
}
