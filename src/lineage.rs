//! Validated parent-child task lineage.

use serde::{
    Deserialize,
    Deserializer,
    Serialize,
};
use thiserror::Error;

use crate::TaskId;
use crate::scheme::UrnScheme;

/// A structurally invalid task lineage.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TaskLineageError<S: UrnScheme> {
    /// The task and parent belong to different task trees.
    #[error("task {task} and parent {parent} belong to different task trees")]
    TreeMismatch {
        /// Child task ID.
        task: TaskId<S>,
        /// Parent task ID.
        parent: TaskId<S>,
    },
    /// A task names itself as its parent.
    #[error("task cannot be its own parent: {task}")]
    SelfParent {
        /// Self-parented task ID.
        task: TaskId<S>,
    },
}

/// A validated child and its exact parent in one distributed task tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TaskLineage<S: UrnScheme> {
    task: TaskId<S>,
    parent: TaskId<S>,
}

impl<S: UrnScheme> TaskLineage<S> {
    /// Validate and construct one parent-child edge.
    ///
    /// # Errors
    ///
    /// Rejects self-parentage and cross-tree parentage.
    pub fn try_new(task: TaskId<S>, parent: TaskId<S>) -> Result<Self, TaskLineageError<S>> {
        if task == parent {
            return Err(TaskLineageError::SelfParent { task });
        }
        if task.tree() != parent.tree() {
            return Err(TaskLineageError::TreeMismatch { task, parent });
        }
        Ok(Self { task, parent })
    }

    /// Return the child task identity.
    #[must_use]
    pub fn task(&self) -> &TaskId<S> {
        &self.task
    }

    /// Return the exact parent task identity.
    #[must_use]
    pub fn parent(&self) -> &TaskId<S> {
        &self.parent
    }

    /// Consume the edge into its child and parent identities.
    #[must_use]
    pub fn into_parts(self) -> (TaskId<S>, TaskId<S>) {
        (self.task, self.parent)
    }
}

impl<'de, S: UrnScheme> Deserialize<'de> for TaskLineage<S> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(bound = "")]
        struct WireLineage<S: UrnScheme> {
            task: TaskId<S>,
            parent: TaskId<S>,
        }

        let wire = WireLineage::<S>::deserialize(deserializer)?;
        Self::try_new(wire.task, wire.parent).map_err(serde::de::Error::custom)
    }
}
