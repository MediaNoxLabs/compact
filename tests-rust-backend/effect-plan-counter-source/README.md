# Counter effect source fixture

This crate freshly generates the exact schema20 IR for
`examples/rust_backend/effect_plan_counter.compact`. It covers increment, a
queried Counter threshold, lessThan, selected decrement/reset and a Field
outcome. No synthetic exports are added.

The current frontend chooses Effectful return plans only when a terminal branch
contains a Cell write. This source genuinely writes its post-operation outcome
to a Field Cell. A Counter-only version is still refused during Rust IR emission;
this fixture does not claim that frontend limitation is fixed. Ordinary-root
recording policy remains separate.

The generated execution tests use 108 finite boundary cases with Counter states
and thresholds at zero, one and u64::MAX, Uint16 amounts including u16::MAX,
official underflow/overflow refusals, and both selected branches. An independent
program assembled from official slot adapters fixes the query order. Execution
gas compares native/recorded/manual execution; batched replay gas compares two
batched replays, separately. No witness or network execution is involved.

The default proof command is:

```sh
cargo run --offline -p compact-rust-proof-smoke -- \
  --effect-plan-counter /path/to/counter-source-artifacts
```

Fresh matching ledger8 keys/ZKIR and `contract/compact-rust-ir.json` are required.
The runner checks IR identity before loading keys, then proves both successful
branches with the existing official ledger checks and shared **unbalanced**
smoke policy. The reset case skips a decrement that would underflow. This is a
local identity check, not authenticated key provenance or funded network evidence.
Keep source, IR, compiler and key fingerprints with each proof receipt.
