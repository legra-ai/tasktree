//! Ordered activity sequence, durability class and terminal-tail invariants.

mod support;

use std::num::NonZeroUsize;

use support::ExampleUrn;
use tasktree::{
    ActivityDurability,
    ActivityPayload,
    ActivitySeq,
    ActivitySeqError,
    OrderedActivity,
    TaskId,
    TerminalTailLimit,
    TerminalTailLimitError,
};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum ExamplePayload {
    Progress,
    Finished,
}

impl ActivityPayload for ExamplePayload {
    fn durability(&self) -> ActivityDurability {
        match self {
            Self::Progress => ActivityDurability::Coalescible,
            Self::Finished => ActivityDurability::Retained,
        }
    }
}

#[test]
fn sequence_advances_strictly_and_refuses_overflow() {
    let first = ActivitySeq::FIRST;
    let second = first.next().expect("advance");

    assert_eq!(first.get(), 0);
    assert_eq!(second.get(), 1);
    assert!(first < second);
    assert_eq!(
        ActivitySeq::new(u64::MAX).next(),
        Err(ActivitySeqError::Exhausted)
    );
}

#[test]
fn successor_validation_refuses_repeats_and_regressions() {
    let five = ActivitySeq::new(5);

    assert_eq!(five.validate_successor(ActivitySeq::new(9)), Ok(()));
    assert_eq!(
        five.validate_successor(five),
        Err(ActivitySeqError::OutOfOrder {
            previous: five,
            next: five,
        })
    );
    assert_eq!(
        five.validate_successor(ActivitySeq::new(4)),
        Err(ActivitySeqError::OutOfOrder {
            previous: five,
            next: ActivitySeq::new(4),
        })
    );
}

#[test]
fn observing_a_watermark_never_moves_backwards() {
    let seven = ActivitySeq::new(7);

    assert_eq!(seven.observe(ActivitySeq::new(3)), seven);
    assert_eq!(seven.observe(ActivitySeq::new(11)), ActivitySeq::new(11));
}

#[test]
fn sequence_serializes_as_a_bare_integer() {
    let encoded = serde_json::to_string(&ActivitySeq::new(42)).expect("serialize");

    assert_eq!(encoded, "42");
    assert_eq!(
        serde_json::from_str::<ActivitySeq>(&encoded).expect("deserialize"),
        ActivitySeq::new(42)
    );
}

#[test]
fn durability_classes_say_what_may_be_coalesced() {
    assert!(!ActivityDurability::Retained.may_coalesce());
    assert!(ActivityDurability::Coalescible.may_coalesce());
}

#[test]
fn ordered_activity_carries_its_payload_durability_and_order() {
    let task = TaskId::<ExampleUrn>::generate();
    let earlier = OrderedActivity::new(
        task.clone(),
        ActivitySeq::new(1),
        1_000,
        ExamplePayload::Progress,
    );
    let later = OrderedActivity::new(
        task.clone(),
        ActivitySeq::new(2),
        999,
        ExamplePayload::Finished,
    );

    assert_eq!(earlier.task(), &task);
    assert_eq!(earlier.seq(), ActivitySeq::new(1));
    assert_eq!(earlier.at(), 1_000);
    assert_eq!(earlier.payload(), &ExamplePayload::Progress);
    assert_eq!(earlier.durability(), ActivityDurability::Coalescible);
    assert_eq!(later.durability(), ActivityDurability::Retained);
    assert!(earlier.precedes(&later));
    assert!(!later.precedes(&earlier));
    assert_eq!(later.into_payload(), ExamplePayload::Finished);
}

#[test]
fn ordered_activity_round_trips() {
    let activity = OrderedActivity::new(
        TaskId::<ExampleUrn>::generate(),
        ActivitySeq::new(3),
        7,
        ExamplePayload::Finished,
    );
    let encoded = serde_json::to_string(&activity).expect("serialize");
    let decoded: OrderedActivity<ExampleUrn, ExamplePayload> =
        serde_json::from_str(&encoded).expect("deserialize");

    assert_eq!(decoded, activity);
}

#[test]
fn terminal_tail_limit_is_never_zero_and_admits_up_to_its_bound() {
    let limit = TerminalTailLimit::try_new(2).expect("non-zero");

    assert_eq!(
        TerminalTailLimit::try_new(0),
        Err(TerminalTailLimitError::Zero)
    );
    assert_eq!(limit.get(), 2);
    assert!(limit.admits(2));
    assert!(!limit.admits(3));
    assert_eq!(TerminalTailLimit::new(NonZeroUsize::MIN).get(), 1);
    assert!(serde_json::from_str::<TerminalTailLimit>("0").is_err());
}
