//! Generic task-tree core construction and deserialization invariants.

mod support;

use support::ExampleUrn;
use tasktree::{
    TaskId as GenericTaskId,
    TaskLineage as GenericTaskLineage,
    TaskLineageError,
    TaskNodeId,
    TaskStatus,
    TaskTransition,
    TaskTreeId as GenericTaskTreeId,
};

type TaskId = GenericTaskId<ExampleUrn>;
type TaskTreeId = GenericTaskTreeId<ExampleUrn>;
type TaskLineage = GenericTaskLineage<ExampleUrn>;

const TRACE: &str = "0102030405060708090a0b0c0d0e0f10";
const OTHER_TRACE: &str = "1112131415161718191a1b1c1d1e1f20";
const SPAN: &str = "1112131415161718";

#[test]
fn task_identity_round_trips_canonical_root_and_descendant_urns() {
    let tree = TaskTreeId::new(TRACE).unwrap();
    let root = TaskId::root(tree.clone());
    let descendant = TaskId::new(tree, TaskNodeId::new(SPAN).unwrap());

    assert_eq!(root.as_str(), format!("urn:example:task:{TRACE}"));
    assert_eq!(root.node().as_w3c_span_id(), "0000000000000001");
    assert_eq!(TaskId::parse(root.as_str()).unwrap(), root);
    assert_eq!(
        descendant.as_str(),
        format!("urn:example:task:{TRACE}:{SPAN}")
    );
    assert_eq!(TaskId::parse(descendant.as_str()).unwrap(), descendant);
}

#[test]
fn task_identity_rejects_noncanonical_and_ambiguous_forms() {
    assert!(TaskId::parse(TRACE).is_err());
    assert!(TaskId::parse(SPAN).is_err());
    assert!(TaskId::parse(&format!("urn:example:task-tree:{TRACE}")).is_err());
    assert!(TaskId::parse(&format!("urn:example:task-ref:{TRACE}:{SPAN}")).is_err());
    assert!(TaskId::parse(&format!("urn:example:task:{TRACE}:0000000000000001")).is_err());
}

#[test]
fn task_identity_components_enforce_w3c_width_and_nonzero_rules() {
    assert!(TaskTreeId::new("00000000000000000000000000000000").is_err());
    assert!(TaskNodeId::new("0000000000000000").is_err());
    assert!(TaskTreeId::new("0102").is_err());
    assert!(TaskNodeId::new("0102").is_err());

    let tree = TaskTreeId::new(&format!("urn:example:task-tree:{TRACE}")).unwrap();
    assert_eq!(tree.as_w3c_trace_id(), TRACE);
    let from_traceparent = TaskTreeId::new(&format!("00-{TRACE}-{SPAN}-01")).unwrap();
    assert_eq!(from_traceparent, tree);
}

#[test]
fn task_identity_serde_cannot_bypass_canonical_parsing() {
    let task = TaskId::generate().child();
    let encoded = serde_json::to_string(&task).unwrap();

    assert_eq!(serde_json::from_str::<TaskId>(&encoded).unwrap(), task);
    assert!(serde_json::from_str::<TaskId>(&format!(r#""{TRACE}""#)).is_err());
}

#[test]
fn task_nodes_are_only_unique_within_their_tree() {
    let node = TaskNodeId::new(SPAN).unwrap();
    let first = TaskId::new(TaskTreeId::new(TRACE).unwrap(), node.clone());
    let second = TaskId::new(TaskTreeId::new(OTHER_TRACE).unwrap(), node);

    assert_ne!(first, second);
}

#[test]
fn lineage_rejects_self_and_cross_tree_parentage() {
    let parent = TaskId::generate();
    let child = parent.child();
    let lineage = TaskLineage::try_new(child.clone(), parent.clone()).unwrap();

    assert_eq!(lineage.task(), &child);
    assert_eq!(lineage.parent(), &parent);
    assert!(matches!(
        TaskLineage::try_new(parent.clone(), parent),
        Err(TaskLineageError::SelfParent { .. })
    ));
    assert!(matches!(
        TaskLineage::try_new(child, TaskId::generate()),
        Err(TaskLineageError::TreeMismatch { .. })
    ));
}

#[test]
fn lineage_deserialization_runs_the_constructor_invariants() {
    let task = TaskId::generate();
    let encoded = format!(
        r#"{{"task":"{}","parent":"{}"}}"#,
        task.as_str(),
        task.as_str()
    );

    assert!(serde_json::from_str::<TaskLineage>(&encoded).is_err());
}

#[test]
fn every_status_label_round_trips() {
    for status in TaskStatus::ALL {
        assert_eq!(TaskStatus::parse_label(status.as_str()).unwrap(), status);
    }
}

#[test]
fn lifecycle_rejects_terminal_reversal_at_construction_and_deserialization() {
    assert!(TaskTransition::try_new(TaskStatus::Done, TaskStatus::Running).is_err());
    assert!(TaskTransition::try_new(TaskStatus::Cancelled, TaskStatus::Failed).is_err());
    assert!(serde_json::from_str::<TaskTransition>(r#"{"from":"done","to":"running"}"#).is_err());
}

#[test]
fn lifecycle_accepts_the_complete_legal_transition_set() {
    let legal = [
        (TaskStatus::Admitted, TaskStatus::Queued),
        (TaskStatus::Admitted, TaskStatus::Running),
        (TaskStatus::Admitted, TaskStatus::Cancelled),
        (TaskStatus::Queued, TaskStatus::Running),
        (TaskStatus::Queued, TaskStatus::Cancelled),
        (TaskStatus::Running, TaskStatus::Sealing),
        (TaskStatus::Running, TaskStatus::Done),
        (TaskStatus::Running, TaskStatus::Failed),
        (TaskStatus::Running, TaskStatus::Cancelled),
        (TaskStatus::Sealing, TaskStatus::Done),
        (TaskStatus::Sealing, TaskStatus::Failed),
    ];

    for from in TaskStatus::ALL {
        for to in TaskStatus::ALL {
            let expected = legal.contains(&(from, to));
            assert_eq!(
                TaskTransition::try_new(from, to).is_ok(),
                expected,
                "unexpected lifecycle result for {from:?} -> {to:?}"
            );
        }
    }
}
