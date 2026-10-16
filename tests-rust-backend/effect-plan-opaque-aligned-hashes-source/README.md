# Exact opaque inputs for curve and transient hashes

This genuine default-ZKIRv2 source qualifies `hashToCurve`, `transientHash` and
`transientCommit` over opaque strings and bytes. Each of six exports increments
a Counter and returns `Output { x, y, value }`. Both curve coordinates are
observable; the other operations return their complete Field output. Commitment
uses opening seven. There are no witnesses or witness-domain changes.

Opaque **PersistentHash is excluded**: the official source frontend rejects its
opaque operands. Rust alignment traits alone do not establish Compact language
eligibility. This fixture does not change the separate fixed-input proof source.

## Twenty-four original TypeScript answers

The original fresh TS capture remains unchanged. It contains four strings and
four byte arrays for each operation: empty, ASCII/Unicode, interior zero and
trailing-zero controls. Every circuit/index ID is unique and binds its original
input. Independent official TS primitive calls check the generated source's
complete output.

After #583, **all 24 native and recorded executions must match the original TS
hashes exactly**. Tests also compare independent manual Counter recording,
state/effects, unchanged private state, empty private transcript, execution gas,
verifier program and replay. Replay gas is compared against manual replay.

The observed input boundary remains narrower:

- **15 canonical inputs** execute and prepare, including original input/output FAB
  and alignment comparisons.
- **Nine trailing-zero inputs** receive the established `InvalidLedgerCell`
  observed-input refusal. They are native/recorded hash parity successes but are
  not observed/prepared parity successes. The original capture's
  `rustObservedExpectation` is a Rust expectation label, not a TS refusal.

## Historical negative control

Before #583, ordinary Rust `Vec<u8> → ValueAtom` conversion stripped trailing
zeros before hashing. Native and recorded Rust hashed `"a\0"` like `"a"`, and `[0]`
like `[]`, while TypeScript preserved the exact byte length.

`normalized-reference.json` retains separately captured official TS answers for
those explicitly trimmed inputs. Tests now assert that the nine corrected hash
outputs **differ** from those historical normalized answers. They do not use the
normalized references as success expectations. `normalized-capture.mjs` remains
available to reproduce the historical comparison, separately from original-source
execution. No original TS result was rewritten to match Rust.

The additive exact hash-input conversion preserves bytes locally for hashing.
Ordinary ledger input validation remains unchanged, so observed calls still
reject ambiguous trailing-zero public inputs. Private witness handling is
qualified separately by the exact-hash-input sibling fixture.

## Provenance and limits

The capture binds the source, full retained IR, generated JavaScript, script,
runtime build/lock and WASM files. Build-time generation requires all six recorded
and observed APIs. This fixture does **not** claim opaque proof qualification.

```sh
compactc --target rust --target ts --skip-zk \
  examples/rust_backend/effect_plan_opaque_aligned_hashes.compact /path/to/output
node tests-rust-backend/effect-plan-opaque-aligned-hashes-source/oracle/capture.mjs \
  examples/rust_backend/effect_plan_opaque_aligned_hashes.compact \
  /path/to/output/contract /path/to/official/compact-runtime \
  tests-rust-backend/effect-plan-opaque-aligned-hashes-source/oracle/typescript.json
cargo test --locked -p compact-rust-effect-plan-opaque-aligned-hashes-source-fixture \
  --features ledger-transaction
```
