//! Canonical task identities and their typed W3C trace components.

use std::marker::PhantomData;

use thiserror::Error;

use crate::scheme::UrnScheme;

mod traceparent;
mod traits;

/// Number of random bytes in a task-tree identifier.
const TRACE_BYTE_LEN: usize = 16;

/// Number of hex characters in a W3C trace identifier.
const TRACE_HEX_LEN: usize = TRACE_BYTE_LEN * 2;

/// Number of random bytes in a task-node identifier.
const SPAN_BYTE_LEN: usize = 8;

/// Number of hex characters in a W3C span identifier.
const SPAN_HEX_LEN: usize = SPAN_BYTE_LEN * 2;

/// Separator between the task-tree and descendant-node portions of a task ID.
const TASK_NODE_SEPARATOR: char = ':';

/// The all-zero W3C trace identifier is invalid.
const ZERO_TRACE_BYTES: [u8; TRACE_BYTE_LEN] = [0; TRACE_BYTE_LEN];

/// The all-zero W3C span identifier is invalid.
const ZERO_SPAN_BYTES: [u8; SPAN_BYTE_LEN] = [0; SPAN_BYTE_LEN];

/// Reserved W3C span identifier for every task-tree root.
const ROOT_TASK_NODE_BYTES: [u8; SPAN_BYTE_LEN] = [0, 0, 0, 0, 0, 0, 0, 1];

/// A malformed task identity.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid task identity {value:?}: {message}")]
pub struct TaskIdentityError {
    value: String,
    message: &'static str,
}

/// W3C trace-sized identity for one distributed task tree, minted
/// under the application's [`UrnScheme`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TaskTreeId<S: UrnScheme> {
    bytes: [u8; TRACE_BYTE_LEN],
    urn: String,
    scheme: PhantomData<S>,
}

/// W3C span-sized identity for one task node.
///
/// A task-node ID is only unique inside its [`TaskTreeId`]. Use [`TaskId`] at
/// process, protocol, journal, or persistence boundaries.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TaskNodeId {
    bytes: [u8; SPAN_BYTE_LEN],
    hex: String,
}

/// Canonical, globally usable identity of one task, minted under the
/// application's [`UrnScheme`].
///
/// This type is the sole authority for interpreting the task URN. A
/// root is encoded as `<TASK_PREFIX><tree>` and a descendant as
/// `<TASK_PREFIX><tree>:<node>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TaskId<S: UrnScheme> {
    tree: TaskTreeId<S>,
    node: TaskNodeId,
    urn: String,
}

impl<S: UrnScheme> TaskId<S> {
    /// Construct a task ID from its tree and node identities.
    #[must_use]
    pub fn new(tree: TaskTreeId<S>, node: TaskNodeId) -> Self {
        let () = S::VALIDATED;
        let urn = if node.is_root() {
            format!("{}{}", S::TASK_PREFIX, tree.as_w3c_trace_id())
        } else {
            format!(
                "{}{}{TASK_NODE_SEPARATOR}{}",
                S::TASK_PREFIX,
                tree.as_w3c_trace_id(),
                node.as_w3c_span_id()
            )
        };
        Self { tree, node, urn }
    }

    /// Construct the canonical root task for a task tree.
    #[must_use]
    pub fn root(tree: TaskTreeId<S>) -> Self {
        Self::new(tree, TaskNodeId::root())
    }

    /// Generate a new task tree and its root task node.
    #[must_use]
    pub fn generate() -> Self {
        Self::root(TaskTreeId::generate())
    }

    /// Generate a child task in the same task tree.
    #[must_use]
    pub fn child(&self) -> Self {
        Self::new(self.tree.clone(), TaskNodeId::generate())
    }

    /// Parse a canonical root or descendant task ID.
    ///
    /// # Errors
    ///
    /// Returns an error for a bare trace, bare span, or malformed task URN.
    pub fn parse(raw: &str) -> Result<Self, TaskIdentityError> {
        let suffix = raw.strip_prefix(S::TASK_PREFIX).ok_or_else(|| {
            invalid_format(raw, "URN does not start with the scheme's task prefix")
        })?;
        let Some((trace, span)) = suffix.split_once(TASK_NODE_SEPARATOR) else {
            return Ok(Self::root(TaskTreeId::from_hex(suffix, raw)?));
        };
        if span.contains(TASK_NODE_SEPARATOR) {
            return Err(invalid_format(raw, "task ID has too many fields"));
        }
        let node = TaskNodeId::from_hex(span, raw)?;
        if node.is_root() {
            return Err(invalid_format(
                raw,
                "root task URN must omit the reserved root node",
            ));
        }
        Ok(Self::new(TaskTreeId::from_hex(trace, raw)?, node))
    }

    /// Return the task-tree identity.
    #[must_use]
    pub fn tree(&self) -> &TaskTreeId<S> {
        &self.tree
    }

    /// Return the task-node identity.
    #[must_use]
    pub fn node(&self) -> &TaskNodeId {
        &self.node
    }

    /// Return the canonical task URN.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.urn
    }

    /// Return the canonical task URN without its namespace prefix.
    #[must_use]
    pub fn as_urn_suffix(&self) -> &str {
        &self.urn[S::TASK_PREFIX.len()..]
    }
}

impl<S: UrnScheme> TaskTreeId<S> {
    fn from_bytes(bytes: [u8; TRACE_BYTE_LEN]) -> Self {
        let () = S::VALIDATED;
        Self {
            urn: urn_from_bytes(S::TREE_PREFIX, &bytes),
            bytes,
            scheme: PhantomData,
        }
    }

    /// Generate a non-zero random task-tree identity.
    #[must_use]
    pub fn generate() -> Self {
        loop {
            let bytes = rand::random::<[u8; TRACE_BYTE_LEN]>();
            if bytes != ZERO_TRACE_BYTES {
                return Self::from_bytes(bytes);
            }
        }
    }

    /// Parse a task-tree URN, bare W3C trace ID, or W3C traceparent.
    ///
    /// # Errors
    ///
    /// Returns an error when the trace identity is malformed or all zero.
    pub fn new(raw: &str) -> Result<Self, TaskIdentityError> {
        if let Some(trace_hex) = traceparent::trace_hex(raw)? {
            return Self::from_hex(trace_hex, raw);
        }
        let hex = raw.strip_prefix(S::TREE_PREFIX).unwrap_or(raw);
        Self::from_hex(hex, raw)
    }

    /// Return the task-tree URN.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.urn
    }

    /// Return the W3C trace ID without its URN prefix.
    #[must_use]
    pub fn as_w3c_trace_id(&self) -> &str {
        &self.urn[S::TREE_PREFIX.len()..]
    }

    fn from_hex(hex: &str, original: &str) -> Result<Self, TaskIdentityError> {
        let bytes = decode_hex_array::<TRACE_BYTE_LEN, TRACE_HEX_LEN>(
            hex,
            original,
            "expected 32 trace hex characters",
        )?;
        if bytes == ZERO_TRACE_BYTES {
            return Err(invalid_format(original, "trace ID must not be all zero"));
        }
        Ok(Self::from_bytes(bytes))
    }
}

impl TaskNodeId {
    fn from_bytes(bytes: [u8; SPAN_BYTE_LEN]) -> Self {
        Self {
            hex: hex_from_bytes(&bytes),
            bytes,
        }
    }

    fn root() -> Self {
        Self::from_bytes(ROOT_TASK_NODE_BYTES)
    }

    fn is_root(&self) -> bool {
        self.bytes == ROOT_TASK_NODE_BYTES
    }

    /// Generate a non-zero random task-node identity.
    #[must_use]
    pub fn generate() -> Self {
        loop {
            let bytes = rand::random::<[u8; SPAN_BYTE_LEN]>();
            if bytes != ZERO_SPAN_BYTES && bytes != ROOT_TASK_NODE_BYTES {
                return Self::from_bytes(bytes);
            }
        }
    }

    /// Parse a bare W3C span ID.
    ///
    /// # Errors
    ///
    /// Returns an error when the span identity is malformed or all zero.
    pub fn new(raw: &str) -> Result<Self, TaskIdentityError> {
        Self::from_hex(raw, raw)
    }

    /// Return the canonical lowercase W3C span ID.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.hex
    }

    /// Return the canonical lowercase W3C span ID.
    #[must_use]
    pub fn as_w3c_span_id(&self) -> &str {
        &self.hex
    }

    fn from_hex(hex: &str, original: &str) -> Result<Self, TaskIdentityError> {
        let bytes = decode_hex_array::<SPAN_BYTE_LEN, SPAN_HEX_LEN>(
            hex,
            original,
            "expected 16 span hex characters",
        )?;
        if bytes == ZERO_SPAN_BYTES {
            return Err(invalid_format(original, "span ID must not be all zero"));
        }
        Ok(Self::from_bytes(bytes))
    }
}

fn decode_hex_array<const BYTE_LEN: usize, const HEX_LEN: usize>(
    hex: &str,
    original: &str,
    message: &'static str,
) -> Result<[u8; BYTE_LEN], TaskIdentityError> {
    if hex.len() != HEX_LEN
        || !hex
            .chars()
            .all(|character| character.is_ascii_hexdigit() && !character.is_ascii_uppercase())
    {
        return Err(invalid_format(original, message));
    }
    let mut bytes = [0u8; BYTE_LEN];
    for index in 0..BYTE_LEN {
        bytes[index] = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16)
            .map_err(|_| invalid_format(original, "invalid hex digit"))?;
    }
    Ok(bytes)
}

fn urn_from_bytes(prefix: &str, bytes: &[u8]) -> String {
    format!("{prefix}{}", hex_from_bytes(bytes))
}

fn hex_from_bytes(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}

fn invalid_format(value: &str, message: &'static str) -> TaskIdentityError {
    TaskIdentityError {
        value: value.to_owned(),
        message,
    }
}
