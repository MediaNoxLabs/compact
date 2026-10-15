# Typed Merkle composition

This fixture renders the retained full IR for `effect_plan_merkle.compact` using
the current backend. All seven exported circuits must expose recorded and
observed-call APIs. No witnesses or alternative Merkle implementation are used.

- `run` inserts a typed Uint8 leaf into a depth-three plain tree, conditionally
  calls a named helper for historic insertion, then calls observation helpers.
- `observe` reads both fullness and root membership. The source uses named
  root-return helpers because the pinned frontend rejects nested Merkle queries.
- `place`, `hashes`, and `defaults` compose indexed insertion, raw hash insertion,
  indexed hash insertion, and default insertion for plain and historic trees.
- `forget` and `reset` distinguish history pruning from full tree reset.

Runtime tests compare native, generated recorded, and independently assembled
`MerkleSlot`/`RecordingFrame` executions: result, state, effects, gas, private
state, transcript and replay. Reused contexts exercise history retention, pruning,
capacity, invalid indices and full-tree refusal. This is ledger8 qualification;
there are no ledger9/10 operations. Only `run` is intended for proof qualification;
other exports are runtime checks until separately qualified.

Run `cargo test -p compact-rust-effect-plan-merkle-source-fixture`.

A second source contract, `effect_plan_merkle_nested.compact`, places plain and
historic Field-leaf trees in different physical ledger segments (`[0,0]` and `[1,14]`). Its
retained IR is rendered into a test-only nested Rust module, with internal crate
references rebased solely to host both generated contracts in one fixture. A
runtime control repeatedly inserts into both trees and compares native/recorded
execution with independent slots, including full transcript, gas and replay.
This secondary contract has no proof qualification claim.

The nested control uses a Field parameter to stay within the preserved unit-root
admission policy. An earlier Uint8/unit variant was correctly refused by that
policy; its refusal is not evidence of a physical path defect.

This batch preserves the existing Cell/Set storage leaf domain and public/witness
interfaces. Top-level Unit, vector, tuple and wide unsigned Merkle leaves are not
qualified here. Successful retained voting composition does not retire its legacy
selector; the old accepted opaque-byte witness case still requires a separate
witness-domain change.
