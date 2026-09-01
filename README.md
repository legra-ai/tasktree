# tasktree

[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![CI][ci-badge]][ci-url]
[![License][license-badge]][license-url]
[![Downloads][downloads-badge]][downloads-url]

Distributed task-tree values: W3C trace-compatible task identities under
an application-supplied URN scheme, validated lifecycle transitions,
parent-child lineage, progress, and timestamps.

This crate is deliberately **runtime-independent** — no async, no I/O, no
executor. It is the value layer a distributed task engine is built on;
the engine (registries, admission, execution, event streams) is yours.

- **Identity** — a `TaskTreeId` is W3C trace-sized (16 bytes) and a
  `TaskNodeId` span-sized (8 bytes), so every task maps directly onto
  Trace Context: a tree id parses from its URN, a bare trace id, *or* a
  full `traceparent` header. Every tree has one reserved root node, and
  `TaskId::child` mints siblings inside the tree.
- **Your namespace, not ours** — identities are minted under a
  [`UrnScheme`] the application defines; this crate ships no namespace of
  its own, so no third party can mint another system's URNs and two
  systems' identities can never be confused, even at the type level.
  Prefixes are validated at compile time.
- **Lifecycle** — `TaskStatus` with a validated transition machine
  (`admitted → queued → running → sealing → done/failed/cancelled`);
  terminal states admit no exits, and the irreversible sealing region
  refuses cancellation. Illegal edges are unrepresentable as
  `TaskTransition` values.
- **Lineage** — `TaskLineage` proves a parent-child edge is within one
  tree and never self-referential, enforced again on deserialization.
- **Progress** — typed units, bounded aggregation, display fractions,
  and lifecycle timestamps.

```rust
use tasktree::{TaskId, TaskLineage, TaskStatus, UrnScheme};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct MyUrn;

impl UrnScheme for MyUrn {
    const TASK_PREFIX: &'static str = "urn:my-app:task:";
    const TREE_PREFIX: &'static str = "urn:my-app:task-tree:";
}

let root: TaskId<MyUrn> = TaskId::generate();
let child = root.child();
assert!(root.as_str().starts_with("urn:my-app:task:"));

let edge = TaskLineage::try_new(child, root).expect("same tree, not self");
assert!(TaskStatus::Running.validate_transition(TaskStatus::Sealing).is_ok());
# let _ = edge;
```

Serde uses the URN string form throughout, and every deserialization
re-validates: a foreign prefix, a zero identity, or an illegal
transition never becomes a value.

[`UrnScheme`]: https://docs.rs/tasktree/latest/tasktree/trait.UrnScheme.html

## License

Licensed under either of:

- Apache License, Version 2.0 ([`LICENSE-APACHE`](LICENSE-APACHE));
- MIT License ([`LICENSE-MIT`](LICENSE-MIT)).

## Links

[crates-badge]: https://img.shields.io/crates/v/tasktree.svg
[crates-url]: https://crates.io/crates/tasktree
[docs-badge]: https://docs.rs/tasktree/badge.svg
[docs-url]: https://docs.rs/tasktree
[ci-badge]: https://github.com/legra-ai/tasktree/actions/workflows/ci.yml/badge.svg
[ci-url]: https://github.com/legra-ai/tasktree/actions/workflows/ci.yml
[license-badge]: https://img.shields.io/crates/l/tasktree.svg
[license-url]: https://github.com/legra-ai/tasktree/blob/main/LICENSE-APACHE
[downloads-badge]: https://img.shields.io/crates/d/tasktree.svg
[downloads-url]: https://crates.io/crates/tasktree
