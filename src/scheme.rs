//! The application-supplied URN scheme for task identities.

/// The URN namespace an application mints its task identities under.
///
/// This crate mints no namespace of its own: every [`crate::TaskId`]
/// and [`crate::TaskTreeId`] is parameterized by a scheme the
/// application defines once, so identities from different systems can
/// never be confused and no third party can mint another
/// application's URNs by accident.
///
/// Implement it on a unit struct with trivial derives:
///
/// ```rust
/// use tasktree::UrnScheme;
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
/// struct ExampleUrn;
///
/// impl UrnScheme for ExampleUrn {
///     const TASK_PREFIX: &'static str = "urn:example:task:";
///     const TREE_PREFIX: &'static str = "urn:example:task-tree:";
/// }
/// ```
///
/// Both prefixes are validated at compile time when the scheme is
/// first used: non-empty, ASCII graphic, ending in `:`, and distinct
/// from each other.
pub trait UrnScheme:
    Copy + Clone + PartialEq + Eq + std::hash::Hash + PartialOrd + Ord + std::fmt::Debug + 'static
{
    /// The prefix of a task URN, e.g. `urn:example:task:`.
    const TASK_PREFIX: &'static str;

    /// The prefix of a task-tree URN, e.g. `urn:example:task-tree:`.
    const TREE_PREFIX: &'static str;

    /// Compile-time prefix validation. Do not override; constructors
    /// evaluate it once per scheme.
    const VALIDATED: () = {
        assert!(
            !Self::TASK_PREFIX.is_empty(),
            "TASK_PREFIX must not be empty"
        );
        assert!(
            !Self::TREE_PREFIX.is_empty(),
            "TREE_PREFIX must not be empty"
        );
        validate_prefix(Self::TASK_PREFIX);
        validate_prefix(Self::TREE_PREFIX);
        assert!(
            !str_eq(Self::TASK_PREFIX, Self::TREE_PREFIX),
            "TASK_PREFIX and TREE_PREFIX must differ"
        );
    };
}

/// Panics (at compile time in `const` context) unless the prefix is
/// ASCII graphic and ends with `:`.
const fn validate_prefix(prefix: &str) {
    let bytes = prefix.as_bytes();
    assert!(
        bytes[bytes.len() - 1] == b':',
        "URN prefixes must end with ':'"
    );
    let mut index = 0;
    while index < bytes.len() {
        assert!(
            bytes[index].is_ascii_graphic(),
            "URN prefixes must be ASCII graphic characters"
        );
        index += 1;
    }
}

/// Const string equality.
const fn str_eq(left: &str, right: &str) -> bool {
    let (left, right) = (left.as_bytes(), right.as_bytes());
    if left.len() != right.len() {
        return false;
    }
    let mut index = 0;
    while index < left.len() {
        if left[index] != right[index] {
            return false;
        }
        index += 1;
    }
    true
}
