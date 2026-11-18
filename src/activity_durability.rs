//! Durability classes of activity payloads.

use serde::{
    Deserialize,
    Serialize,
};

/// Whether an activity record must survive backpressure and restart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityDurability {
    /// The record is evidence (lifecycle, lineage, terminal outcome): it is
    /// never dropped or merged, only delivered or refused loudly.
    Retained,
    /// The record is a diagnostic observation (intermediate progress, a
    /// sample): a newer record for the same task may replace it.
    Coalescible,
}

impl ActivityDurability {
    /// Whether a newer record may replace this one before delivery.
    #[must_use]
    pub const fn may_coalesce(self) -> bool {
        matches!(self, Self::Coalescible)
    }
}

/// An application activity payload that knows its own durability class.
///
/// The classification is the application's policy; the ordering and
/// delivery machinery only reads it.
pub trait ActivityPayload {
    /// The durability class of this payload.
    fn durability(&self) -> ActivityDurability;
}
