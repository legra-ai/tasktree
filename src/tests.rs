//! Identity, scheme, lineage, and lifecycle coverage under an example
//! URN scheme.

use crate::{
    TaskId,
    TaskLineage,
    TaskLineageError,
    TaskStatus,
    TaskTreeId,
    UrnScheme,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct ExampleUrn;

impl UrnScheme for ExampleUrn {
    const TASK_PREFIX: &'static str = "urn:example:task:";
    const TREE_PREFIX: &'static str = "urn:example:task-tree:";
}

type Id = TaskId<ExampleUrn>;
type TreeId = TaskTreeId<ExampleUrn>;

#[test]
fn root_task_urn_round_trips_under_the_scheme() {
    let root = Id::generate();
    assert!(root.as_str().starts_with("urn:example:task:"));
    let parsed = Id::parse(root.as_str()).expect("round trip");
    assert_eq!(parsed, root);
    assert_eq!(parsed.node(), root.node());
}

#[test]
fn child_tasks_share_the_tree_and_round_trip() {
    let root = Id::generate();
    let child = root.child();
    assert_eq!(child.tree(), root.tree());
    assert_ne!(child, root);
    assert!(child.as_str().contains(':'));
    assert_eq!(Id::parse(child.as_str()).expect("round trip"), child);
}

#[test]
fn foreign_prefixes_are_rejected() {
    let root = Id::generate();
    let foreign = root.as_str().replace("urn:example:", "urn:other:");
    assert!(Id::parse(&foreign).is_err(), "{foreign}");
}

#[test]
fn tree_ids_parse_urn_bare_trace_and_traceparent_forms() {
    let tree = TreeId::generate();
    let trace = tree.as_w3c_trace_id();
    for raw in [
        tree.as_str().to_owned(),
        trace.to_owned(),
        format!("00-{trace}-00f067aa0ba902b7-01"),
    ] {
        assert_eq!(
            TreeId::new(&raw).expect("parse").as_str(),
            tree.as_str(),
            "{raw}"
        );
    }
}

#[test]
fn zero_and_malformed_identities_are_rejected() {
    for raw in [
        "urn:example:task:",
        "urn:example:task:00000000000000000000000000000000",
        "urn:example:task:zz",
        "urn:example:task:0af7651916cd43dd8448eb211c80319c:0000000000000000",
        "urn:example:task:0af7651916cd43dd8448eb211c80319c:0000000000000001",
        "urn:example:task:0af7651916cd43dd8448eb211c80319c:aa:bb",
    ] {
        assert!(Id::parse(raw).is_err(), "{raw}");
    }
}

#[test]
fn serde_uses_the_urn_string_form() {
    let root = Id::generate();
    let json = serde_json::to_string(&root).expect("serialize");
    assert_eq!(json, format!("{:?}", root.as_str()));
    let back: Id = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back, root);
}

#[test]
fn lineage_rejects_self_and_cross_tree_parentage() {
    let root = Id::generate();
    let child = root.child();
    assert!(TaskLineage::try_new(child.clone(), root.clone()).is_ok());
    assert!(matches!(
        TaskLineage::try_new(root.clone(), root.clone()),
        Err(TaskLineageError::SelfParent { .. })
    ));
    let stranger = Id::generate();
    assert!(matches!(
        TaskLineage::try_new(stranger, root),
        Err(TaskLineageError::TreeMismatch { .. })
    ));
}

#[test]
fn lifecycle_terminal_states_admit_no_exits() {
    for terminal in [TaskStatus::Done, TaskStatus::Failed, TaskStatus::Cancelled] {
        assert!(terminal.is_terminal());
        for next in TaskStatus::ALL {
            assert!(terminal.validate_transition(next).is_err());
        }
    }
    assert!(
        TaskStatus::Running
            .validate_transition(TaskStatus::Sealing)
            .is_ok()
    );
    assert!(
        TaskStatus::Sealing
            .validate_transition(TaskStatus::Done)
            .is_ok()
    );
    assert!(
        TaskStatus::Sealing
            .validate_transition(TaskStatus::Cancelled)
            .is_err()
    );
}

#[test]
fn tokens_use_checked_arithmetic_and_round_trip_as_text() {
    use crate::Tokens;

    assert_eq!(
        Tokens::new(2).checked_add(Tokens::new(3)),
        Some(Tokens::new(5))
    );
    assert_eq!(Tokens::new(u64::MAX).checked_add(Tokens::new(1)), None);
    assert_eq!(
        Tokens::new(3).checked_sub(Tokens::new(3)),
        Some(Tokens::ZERO)
    );
    assert_eq!(Tokens::new(2).checked_sub(Tokens::new(3)), None);
    assert_eq!(
        Tokens::checked_sum([Tokens::new(1), Tokens::new(2), Tokens::new(3)]),
        Some(Tokens::new(6))
    );
    assert_eq!(
        Tokens::checked_sum([Tokens::new(u64::MAX), Tokens::new(1)]),
        None
    );
    assert_eq!(Tokens::checked_sum([]), Some(Tokens::ZERO));
    assert!(Tokens::ZERO.is_zero());
    assert_eq!("42".parse::<Tokens>().expect("parses"), Tokens::new(42));
    assert!("-1".parse::<Tokens>().is_err());
    assert_eq!(Tokens::new(7).to_string(), "7");
    assert_eq!(
        serde_json::to_string(&Tokens::new(7)).expect("serialize"),
        "7"
    );
}
