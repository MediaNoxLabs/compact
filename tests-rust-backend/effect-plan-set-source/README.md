# Ordinary Set effect composition

`build.rs` emits the current backend from the full source-derived IR of
`examples/rust_backend/effect_plan_set.compact`. Four exports must remain recorded
and observed: `run`, `lifecycle`, `scalarSize`, `scalarEmpty`. The last two use
specialized Set return nodes in the retained IR. `lifecycle` enters the existing
checked Unit route after its return domain was widened to ordinary Unit returns.
Direct query exports retain their existing public routing in this slice.

The actual 16-field source puts the scalar Set at nested path `[0,0]`; its second
chunk contains owned-Struct and opaque-string Sets, a canonical owned-key Cell,
a result Cell, and untouched neighboring fields. Owned keys come from a selected
Cell read or pure/default producer. Root parameters stay Field/Boolean; opaque
root and witness interfaces are not expanded.

Four tests cover eight successive source `run` calls with duplicate and distinct
scalar/owned/string keys, selected/unselected insertion, repeated non-Copy reuse,
and independent expected set contents. `lifecycle` covers reset, isEmpty,
duplicate insert, size, present and absent remove, then reset again; direct
size/empty return entries are checked for empty and populated state. Wrong Set
container and selected malformed UTF-8 Cell reads must fail identically; the
unselected malformed Cell is never read and the call succeeds. Actual constructor
state and nested physical paths are checked against the source IR.

Native and recorded execution are compared to independent programs assembled
from official Set/Cell slots: result, state/effects, execution gas, private state,
public query order and replay gas. Explicit set contents/counts provide a second
semantic oracle. Failure checks do not claim recovery of partial frames. No
proof or full TS parity is claimed by these runtime tests.

## Frontend limitation retained separately

A first lifecycle source saved `isEmpty()`/`size()` into locals, then performed
more Set operations and returned a Struct. The pinned frontend rejected a nested
`Set.insert` before Rust lowering. The accepted lifecycle expresses the same
checks as assertions with a Unit result; direct query exports retain explicit
result-value coverage. The original failing source and diagnostic were retained
in the delivery evidence. This fixture does not claim that frontend shape works.

```sh
cargo test --offline --locked -p compact-rust-effect-plan-set-source-fixture --all-features
```
