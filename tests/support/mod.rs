use tasktree::UrnScheme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct ExampleUrn;

impl UrnScheme for ExampleUrn {
    const TASK_PREFIX: &'static str = "urn:example:task:";
    const TREE_PREFIX: &'static str = "urn:example:task-tree:";
}
