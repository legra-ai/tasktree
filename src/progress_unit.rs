//! Typed units for task progress counters.

use serde::{
    Deserialize,
    Serialize,
};

/// The unit of measurement for progress counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgressUnit {
    /// Progress measured in RDF quads.
    Quads,
    /// Progress measured in bytes.
    Bytes,
    /// Progress measured in result rows.
    Rows,
    /// Progress measured in generic items.
    Items,
}

impl ProgressUnit {
    /// Stable lowercase string form (the inverse of
    /// [`FromStr`](std::str::FromStr)).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Quads => "quads",
            Self::Bytes => "bytes",
            Self::Rows => "rows",
            Self::Items => "items",
        }
    }
}

impl std::fmt::Display for ProgressUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ProgressUnit {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "quads" => Ok(Self::Quads),
            "bytes" => Ok(Self::Bytes),
            "rows" => Ok(Self::Rows),
            "items" => Ok(Self::Items),
            _ => Err(()),
        }
    }
}
