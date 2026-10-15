# Owned local values and pure interfaces

Builds the current backend from the exact retained IR for
`examples/rust_backend/effect_plan_owned_values.compact`. Both exports return a
supported scalar-only Struct and use only a Field Cell/scalar witness interface.
Owned payloads stay inside locals and pure helper boundaries.

The source covers recursive Struct defaults containing opaque strings/bytes and
fixed vectors, construction/projection, a branch returning an owned Struct,
repeated ownership-preserving helper calls, vector equality, and aggregate
conversions through pure interfaces. The independent oracle uses official slot
and recording-frame operations. Tests compare native/recorded/manual results,
state/effects, execution gas, private state/output ordering and independently
replayed gas. The scalar witness reads the previous write through its metered
ledger view, making the ordering assertion observable even on failure.

```sh
cargo test -p compact-rust-effect-plan-owned-values-source-fixture
```

## Qualification limits

- The source's opaque payloads are defaults. A separate direct native pure-API
  test roundtrips nonempty opaque data; this is not a nonempty recorded ABI test.
- Aggregate widening and conversion to Field use pure helper return boundaries.
  An explicit aggregate cast in a stateful typed local
  assignment is rejected by the current native backend; the same pure-circuit
  local cast is accepted. Vector indexing also needs an
  explicit Vector-to-Tuple conversion in the pure helper. These source emission
  limitations are separate from checked algebra coercion support.
- The initial single-export draft exceeded the existing legacy render-path
  budget. Splitting conversions into a second export preserves all tested type
  domains without changing limits; the original source is retained in the local
  qualification evidence.
- Storage, root/stateful-helper and witness interfaces are not expanded. There
  is no new proof qualification or receive-family retirement in this fixture.
