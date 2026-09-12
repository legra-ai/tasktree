//! Terminal-cause invariants: every terminal edge says why, the cause
//! binds the terminal status and the fault side, and a cascade never
//! nests.

use tasktree::{
    BudgetResource,
    DeadlineAuthority,
    FaultSide,
    OriginatingCause,
    TaskStatus,
    TaskTransition,
    TaskTransitionError,
    TerminalCause,
};

#[test]
fn every_originating_cause_binds_one_terminal_status_and_one_fault_side() {
    let expected = [
        (
            OriginatingCause::CallerCancelled,
            TaskStatus::Cancelled,
            FaultSide::Caller,
        ),
        (
            OriginatingCause::SessionClosed,
            TaskStatus::Cancelled,
            FaultSide::Caller,
        ),
        (
            OriginatingCause::Deadline {
                imposed_by: DeadlineAuthority::Caller,
            },
            TaskStatus::Failed,
            FaultSide::Caller,
        ),
        (
            OriginatingCause::Deadline {
                imposed_by: DeadlineAuthority::Provider,
            },
            TaskStatus::Failed,
            FaultSide::Provider,
        ),
        (
            OriginatingCause::BudgetExhausted {
                resource: BudgetResource::WallTime,
            },
            TaskStatus::Failed,
            FaultSide::Caller,
        ),
        (
            OriginatingCause::BudgetExhausted {
                resource: BudgetResource::Tokens,
            },
            TaskStatus::Failed,
            FaultSide::Caller,
        ),
        (
            OriginatingCause::ExecutorFailed {
                fault: FaultSide::Caller,
            },
            TaskStatus::Failed,
            FaultSide::Caller,
        ),
        (
            OriginatingCause::ExecutorFailed {
                fault: FaultSide::Provider,
            },
            TaskStatus::Failed,
            FaultSide::Provider,
        ),
        (
            OriginatingCause::ProgressStalled,
            TaskStatus::Cancelled,
            FaultSide::Provider,
        ),
        (
            OriginatingCause::NodeLost,
            TaskStatus::Cancelled,
            FaultSide::Provider,
        ),
        (
            OriginatingCause::HostEvicted,
            TaskStatus::Cancelled,
            FaultSide::Provider,
        ),
        (
            OriginatingCause::HostRestarted,
            TaskStatus::Failed,
            FaultSide::Provider,
        ),
    ];
    assert_eq!(expected.len(), OriginatingCause::ALL.len());
    for (cause, status, fault) in expected {
        assert!(OriginatingCause::ALL.contains(&cause));
        assert_eq!(cause.terminal_status(), status, "{cause:?}");
        assert_eq!(cause.fault_side(), fault, "{cause:?}");
        assert_eq!(cause.direct().terminal_status(), status);
        assert_eq!(cause.direct().fault_side(), fault);
    }
}

#[test]
fn a_cascade_is_collateral_cancelled_under_the_origin_attribution() {
    for origin in OriginatingCause::ALL {
        let cascaded = origin.cascaded();
        assert!(cascaded.is_cascaded());
        assert_eq!(cascaded.origin(), origin);
        assert_eq!(cascaded.terminal_status(), TaskStatus::Cancelled);
        assert_eq!(cascaded.fault_side(), origin.fault_side());
        // Propagating further changes nothing: a cascade never nests.
        assert_eq!(cascaded.for_descendant(), cascaded);
        assert_eq!(origin.direct().for_descendant(), cascaded);
    }
}

#[test]
fn a_terminal_edge_demands_a_cause_that_ends_where_the_edge_does() {
    let failed = OriginatingCause::HostRestarted.direct();
    let cancelled = OriginatingCause::CallerCancelled.direct();

    assert_eq!(
        TaskTransition::try_new(TaskStatus::Running, TaskStatus::Failed, None),
        Err(TaskTransitionError::MissingCause {
            to: TaskStatus::Failed
        })
    );
    assert_eq!(
        TaskTransition::try_new(TaskStatus::Running, TaskStatus::Done, Some(failed)),
        Err(TaskTransitionError::UnexpectedCause {
            to: TaskStatus::Done
        })
    );
    assert_eq!(
        TaskTransition::try_new(TaskStatus::Running, TaskStatus::Sealing, Some(cancelled)),
        Err(TaskTransitionError::UnexpectedCause {
            to: TaskStatus::Sealing
        })
    );
    assert_eq!(
        TaskTransition::try_new(TaskStatus::Running, TaskStatus::Cancelled, Some(failed)),
        Err(TaskTransitionError::CauseMismatch {
            cause: OriginatingCause::HostRestarted,
            expected: TaskStatus::Failed,
            to: TaskStatus::Cancelled,
        })
    );
    let ok = TaskTransition::try_new(TaskStatus::Running, TaskStatus::Failed, Some(failed))
        .expect("a failure with its cause");
    assert_eq!(ok.cause(), Some(failed));
    assert_eq!(
        TaskTransition::try_new(TaskStatus::Running, TaskStatus::Done, None)
            .expect("success carries no cause")
            .cause(),
        None
    );
}

#[test]
fn a_hard_limit_fails_a_task_that_never_ran_but_sealing_still_refuses_cancellation() {
    let budget = OriginatingCause::BudgetExhausted {
        resource: BudgetResource::Tokens,
    }
    .direct();
    assert!(
        TaskTransition::try_new(TaskStatus::Admitted, TaskStatus::Failed, Some(budget)).is_ok()
    );
    assert!(TaskTransition::try_new(TaskStatus::Queued, TaskStatus::Failed, Some(budget)).is_ok());
    assert_eq!(
        TaskTransition::try_new(
            TaskStatus::Sealing,
            TaskStatus::Cancelled,
            Some(OriginatingCause::CallerCancelled.direct())
        ),
        Err(TaskTransitionError::IllegalEdge {
            from: TaskStatus::Sealing,
            to: TaskStatus::Cancelled,
        })
    );
}

#[test]
fn causes_and_caused_transitions_round_trip_and_revalidate_on_deserialization() {
    for origin in OriginatingCause::ALL {
        for cause in [origin.direct(), origin.cascaded()] {
            let encoded = serde_json::to_string(&cause).unwrap();
            let decoded: TerminalCause = serde_json::from_str(&encoded).unwrap();
            assert_eq!(decoded, cause);
        }
    }
    let transition = TaskTransition::try_new(
        TaskStatus::Running,
        TaskStatus::Cancelled,
        Some(OriginatingCause::SessionClosed.cascaded()),
    )
    .unwrap();
    let encoded = serde_json::to_string(&transition).unwrap();
    assert_eq!(
        serde_json::from_str::<TaskTransition>(&encoded).unwrap(),
        transition
    );
    assert!(serde_json::from_str::<TaskTransition>(r#"{"from":"running","to":"failed"}"#).is_err());
    assert!(
        serde_json::from_str::<TaskTransition>(
            r#"{"from":"running","to":"failed","cause":{"origin":{"cause":"caller_cancelled"},"cascaded":false}}"#
        )
        .is_err()
    );
}

#[test]
fn cause_labels_are_distinct_and_stable() {
    let mut labels: Vec<&str> = OriginatingCause::ALL.iter().map(|c| c.as_str()).collect();
    labels.sort_unstable();
    labels.dedup();
    assert_eq!(labels.len(), 9, "one label per cause kind");
    assert_eq!(FaultSide::Caller.as_str(), "caller");
    assert_eq!(FaultSide::Provider.as_str(), "provider");
}

#[test]
fn causes_display_their_label_and_cascade() {
    assert_eq!(FaultSide::Provider.to_string(), "provider");
    assert_eq!(OriginatingCause::NodeLost.to_string(), "node_lost");
    assert_eq!(
        OriginatingCause::CallerCancelled.direct().to_string(),
        "caller_cancelled"
    );
    assert_eq!(
        OriginatingCause::CallerCancelled.cascaded().to_string(),
        "cascaded from caller_cancelled"
    );
}
