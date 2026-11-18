//! Bounded recovery checkpoints: all nonterminal work plus a fixed terminal
//! tail, validated on construction and while streaming in.

mod builder;
mod checkpoint;
mod error;
mod recovered_task;
mod wire;

pub use builder::RecoveryCheckpointBuilder;
pub use checkpoint::RecoveryCheckpoint;
pub use error::RecoveryCheckpointError;
pub use recovered_task::RecoveredTask;
