//! Bounded recovery checkpoint validation, both constructed and streamed.

mod support;

use serde::{
    Deserialize,
    Serialize,
};
use support::ExampleUrn;
use tasktree::{
    ActivitySeq,
    OriginatingCause,
    RecoveredTask,
    RecoveryCheckpoint,
    RecoveryCheckpointError,
    TaskId,
    TaskSnapshotError,
    TaskStatus,
    TaskTimestamps,
    TaskTransitionError,
    TerminalCause,
    TerminalTailLimit,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ExampleTask {
    id: TaskId<ExampleUrn>,
    status: TaskStatus,
    cause: Option<TerminalCause>,
    timestamps: TaskTimestamps,
}

impl ExampleTask {
    fn new(status: TaskStatus) -> Self {
        let cause = match status {
            TaskStatus::Failed => Some(OriginatingCause::HostRestarted.direct()),
            TaskStatus::Cancelled => Some(OriginatingCause::CallerCancelled.direct()),
            _ => None,
        };
        Self {
            id: TaskId::generate(),
            status,
            cause,
            timestamps: TaskTimestamps {
                created_at: 1,
                started_at: None,
                completed_at: status.is_terminal().then_some(2),
            },
        }
    }
}

impl RecoveredTask<ExampleUrn> for ExampleTask {
    fn id(&self) -> &TaskId<ExampleUrn> {
        &self.id
    }

    fn status(&self) -> TaskStatus {
        self.status
    }

    fn terminal_cause(&self) -> Option<TerminalCause> {
        self.cause
    }

    fn timestamps(&self) -> &TaskTimestamps {
        &self.timestamps
    }
}

type Checkpoint = RecoveryCheckpoint<ExampleUrn, ExampleTask>;

fn limit(n: usize) -> TerminalTailLimit {
    TerminalTailLimit::try_new(n).expect("non-zero limit")
}

fn encoded_task(task: &ExampleTask) -> String {
    serde_json::to_string(task).expect("serialize task")
}

fn wire(limit: usize, entries: &[(String, String)]) -> String {
    let tasks = entries
        .iter()
        .map(|(key, task)| format!("\"{key}\":{task}"))
        .collect::<Vec<_>>()
        .join(",");
    format!("{{\"cursor\":0,\"tail_limit\":{limit},\"tasks\":{{{tasks}}}}}")
}

#[test]
fn snapshot_rule_requires_completion_exactly_for_terminal_status() {
    let open = TaskTimestamps {
        created_at: 1,
        started_at: None,
        completed_at: None,
    };
    let closed = TaskTimestamps {
        completed_at: Some(2),
        ..open.clone()
    };

    assert_eq!(TaskStatus::Running.validate_snapshot(None, &open), Ok(()));
    assert_eq!(TaskStatus::Done.validate_snapshot(None, &closed), Ok(()));
    assert_eq!(
        TaskStatus::Done.validate_snapshot(None, &open),
        Err(TaskSnapshotError::MissingCompletion {
            status: TaskStatus::Done
        })
    );
    assert_eq!(
        TaskStatus::Queued.validate_snapshot(None, &closed),
        Err(TaskSnapshotError::UnexpectedCompletion {
            status: TaskStatus::Queued
        })
    );
}

#[test]
fn snapshot_rule_requires_the_matching_cause() {
    let closed = TaskTimestamps {
        created_at: 1,
        started_at: Some(1),
        completed_at: Some(2),
    };
    let restarted = OriginatingCause::HostRestarted.direct();

    assert_eq!(
        TaskStatus::Failed.validate_snapshot(None, &closed),
        Err(TaskSnapshotError::InvalidCause(
            TaskTransitionError::MissingCause {
                to: TaskStatus::Failed
            }
        ))
    );
    assert_eq!(
        TaskStatus::Done.validate_snapshot(Some(restarted), &closed),
        Err(TaskSnapshotError::InvalidCause(
            TaskTransitionError::UnexpectedCause {
                to: TaskStatus::Done
            }
        ))
    );
    assert!(matches!(
        TaskStatus::Cancelled.validate_snapshot(Some(restarted), &closed),
        Err(TaskSnapshotError::InvalidCause(
            TaskTransitionError::CauseMismatch { .. }
        ))
    ));
    assert_eq!(
        TaskStatus::Failed.validate_snapshot(Some(restarted), &closed),
        Ok(())
    );
}

#[test]
fn checkpoint_keeps_nonterminal_work_and_a_bounded_terminal_tail() {
    let tasks = [
        ExampleTask::new(TaskStatus::Running),
        ExampleTask::new(TaskStatus::Queued),
        ExampleTask::new(TaskStatus::Done),
        ExampleTask::new(TaskStatus::Cancelled),
    ];
    let checkpoint =
        Checkpoint::try_new(ActivitySeq::new(9), limit(2), tasks.clone()).expect("valid");

    assert_eq!(checkpoint.cursor(), ActivitySeq::new(9));
    assert_eq!(checkpoint.tail_limit(), limit(2));
    assert_eq!(checkpoint.len(), 4);
    assert!(!checkpoint.is_empty());
    assert_eq!(checkpoint.terminal_count(), 2);
    assert!(checkpoint.covers(ActivitySeq::new(8)));
    assert!(!checkpoint.covers(ActivitySeq::new(9)));
    assert_eq!(checkpoint.get(&tasks[2].id), Some(&tasks[2]));
    assert_eq!(checkpoint.tasks().count(), 4);
    assert_eq!(checkpoint.into_tasks().count(), 4);
}

#[test]
fn checkpoint_refuses_a_terminal_beyond_the_tail_limit() {
    let tasks = (0..3).map(|_| ExampleTask::new(TaskStatus::Done));

    assert_eq!(
        Checkpoint::try_new(ActivitySeq::FIRST, limit(2), tasks),
        Err(RecoveryCheckpointError::TerminalLimitExceeded { limit: limit(2) })
    );
}

#[test]
fn checkpoint_refuses_duplicate_tasks() {
    let task = ExampleTask::new(TaskStatus::Running);

    assert_eq!(
        Checkpoint::try_new(ActivitySeq::FIRST, limit(1), [task.clone(), task.clone()]),
        Err(RecoveryCheckpointError::DuplicateTask(task.id))
    );
}

#[test]
fn checkpoint_refuses_an_invalid_lifecycle() {
    let mut task = ExampleTask::new(TaskStatus::Running);
    task.timestamps.completed_at = Some(5);

    assert_eq!(
        Checkpoint::try_new(ActivitySeq::FIRST, limit(1), [task.clone()]),
        Err(RecoveryCheckpointError::InvalidLifecycle {
            task: task.id,
            source: TaskSnapshotError::UnexpectedCompletion {
                status: TaskStatus::Running
            },
        })
    );
}

#[test]
fn checkpoint_round_trips_through_its_streaming_validator() {
    let checkpoint = Checkpoint::try_new(
        ActivitySeq::new(4),
        limit(1),
        [
            ExampleTask::new(TaskStatus::Running),
            ExampleTask::new(TaskStatus::Done),
        ],
    )
    .expect("valid");
    let encoded = serde_json::to_string(&checkpoint).expect("serialize");

    assert_eq!(
        serde_json::from_str::<Checkpoint>(&encoded).expect("deserialize"),
        checkpoint
    );
}

#[test]
fn streaming_validator_fails_on_the_first_terminal_past_the_limit() {
    let mut entries = (0..3)
        .map(|_| ExampleTask::new(TaskStatus::Done))
        .map(|task| (task.id.to_string(), encoded_task(&task)))
        .collect::<Vec<_>>();
    // A corrupt entry after the overflowing one must never be reached.
    entries.push(("not-a-task".to_owned(), "{".to_owned()));

    let error = serde_json::from_str::<Checkpoint>(&wire(2, &entries)).expect_err("overflow");

    assert!(
        error.to_string().contains("terminal tail limit of 2"),
        "{error}"
    );
}

#[test]
fn streaming_validator_refuses_key_identity_mismatch() {
    let task = ExampleTask::new(TaskStatus::Running);
    let other = TaskId::<ExampleUrn>::generate();
    let error =
        serde_json::from_str::<Checkpoint>(&wire(1, &[(other.to_string(), encoded_task(&task))]))
            .expect_err("mismatch");

    assert!(error.to_string().contains("is keyed as"), "{error}");
}

#[test]
fn streaming_validator_refuses_duplicate_keys() {
    let task = ExampleTask::new(TaskStatus::Running);
    let entry = (task.id.to_string(), encoded_task(&task));
    let error = serde_json::from_str::<Checkpoint>(&wire(1, &[entry.clone(), entry]))
        .expect_err("duplicate");

    assert!(error.to_string().contains("more than once"), "{error}");
}

#[test]
fn streaming_validator_refuses_corrupt_and_misordered_input() {
    let task = ExampleTask::new(TaskStatus::Running);
    let tasks_first = format!(
        "{{\"tasks\":{{\"{}\":{}}},\"cursor\":0,\"tail_limit\":1}}",
        task.id,
        encoded_task(&task)
    );
    let corrupt = [
        tasks_first.as_str(),
        "{\"cursor\":0,\"tail_limit\":0,\"tasks\":{}}",
        "{\"cursor\":-1,\"tail_limit\":1,\"tasks\":{}}",
        "{\"cursor\":0,\"tail_limit\":1}",
        "{\"cursor\":0,\"tail_limit\":1,\"tasks\":{},\"extra\":1}",
        "{\"cursor\":0,\"tail_limit\":1,\"tasks\":{\"urn:example:task:bogus\":{}}}",
    ];

    for input in corrupt {
        assert!(
            serde_json::from_str::<Checkpoint>(input).is_err(),
            "accepted {input}"
        );
    }
}
