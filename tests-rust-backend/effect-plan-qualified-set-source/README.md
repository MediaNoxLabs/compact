# Qualified coin Set: checked recording from real source

The fixture compiles the complete retained schema-20 IR from
`examples/rust_backend/effect_plan_qualified_set_source.compact` through the
current Rust emitter in `build.rs`. It requires both public circuits to expose
recorded execution and observed-call preparation. Sixteen ledger declarations
place `coins` at the frontend-generated path `[1, 14]`, asserted during the build.
The source and retained IR are unchanged by the tests.

`insert_once` inserts one qualified coin. `insert_selected` first inserts the
public coin, then optionally calls a helper with two ordered witness results,
then inserts the public coin again. The helper reuses its coin and recipient
twice. This exercises stateful helper composition, nested paths, branches and
owned operand reuse. Compiler selection tests separately identify the actual
algebra owner; capability booleans alone do not prove ownership.

## Runtime evidence

Four test functions cover:

- Both user and contract recipient variants, with the branch selected and
  unselected. Duplicate insertions preserve Set cardinality; qualified entries
  retain the allocated index. Padding fields remain unchanged.
- Coin and recipient witness failures, plus lazy unselected branches. Each
  callback observes the Set after the first insertion. The selected branch
  calls each witness once, in source order, despite helper operand reuse.
- Missing root allocation (no callback), missing helper allocation, allocation
  for the wrong recipient variant and allocation for another contract address.
  Callback order and exact native/recorded/manual errors are compared. Retained
  immutable state snapshots remain unchanged on failure.
- The inherited ledger8 `Concat { n: 91 }` boundary at serialized sizes 91, 92,
  94 and 102, including both recipients. Numeric maxima with short normalized
  nonce/color payloads also succeed. This preserves the official runtime
  behavior; it does not fix or narrow the bound (CoPS-014 / issue #552).

Successful execution compares generated native execution, generated recorded
execution and a manually assembled `RecordingFrame` using official slot APIs:
result, private state, public state, effects, private FAB outputs, sequential
execution gas, full public verification transcript and replayed state/effects.
Batched replay gas is compared against the manual batched replay, since a single
batched query and multiple execution queries charge update overhead differently.
Set insertion preserves both the existing intent plan and commitment-index map.

Allocation is installed explicitly through the official commitment-index map.
These are runtime tests, not funded-transaction or proof-verification evidence.
The manual reference shares the official slot runtime with generated execution;
it independently checks emitter sequencing and wiring, not the implementation
of the ledger VM itself. The separate maintained-source proof fixture qualifies
funded proof/application behavior.

## Run locally

```sh
cargo test -p compact-rust-effect-plan-qualified-set-source-fixture --all-features --test coins
```

Only two-component source paths are exercised here. Runtime path limits and
arbitrary contract/recipient schemas are not inferred from this fixture.
