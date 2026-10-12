# Opaque inputs for aligned curve and transient hashes

This separate default-ZKIRv2 source qualifies `hashToCurve`, `transientHash` and
`transientCommit` over opaque strings and bytes. Each of six exports increments
a Counter and returns a named `Output { x, y, value }`. Both curve coordinates are
observable; the other primitives return their complete Field output. The commit
opening is seven. There are no witnesses or unrelated witness-domain changes.

Opaque **PersistentHash is excluded**: the official source frontend rejects its
opaque operands. Rust alignment traits alone do not establish Compact language
eligibility. This fixture does not widen persistent hashing or ledger input
validation, and it does not change the separate fixed-input proof source.

## Exact test surfaces

The fresh TypeScript capture contains 24 successful executions: four strings and
four byte arrays for each of three operations. Strings cover empty, ASCII,
Unicode with an interior NUL, and trailing NUL. Bytes cover empty, zero-only,
interior zero, and trailing zero. Each row has a unique circuit/index ID and
asserted input. Its output is checked against a separate call to the official
TypeScript primitive, including both curve coordinates.

Two Rust tests compare native and recorded execution with a manual Counter trace.
The 15 canonical-input cases compare the original frozen TS output. The nine
trailing-zero cases explicitly assert divergence from that original output and
use a separate official TS primitive reference for the normalized bytes. Those
nine are characterization tests, not native or recorded parity successes.

All cases compare native versus recorded result, state/effects, private state,
empty private transcript, execution gas, verifier instructions and manual replay.
Replay gas is compared against manual replay, independently of execution gas.

The observed API has a narrower existing input boundary:

- **15 canonical-input cases** execute and prepare successfully, including original
  input/output FAB and exact alignment comparisons against TypeScript.
- **Nine trailing-zero cases** are explicit refusal controls. TypeScript succeeds;
  Rust observed calls must return `InvalidLedgerCell` with the established
  trailing-zero message before changing the borrowed observed state. They are
  not counted as observed/prepared parity successes. The capture's
  `rustObservedExpectation` is a Rust expectation label, not a TS refusal.

The cause is the existing conversion from Rust opaque carriers through official
`Vec<u8> → ValueAtom`: it strips trailing zeros. The TS opaque type descriptors
preserve the original bytes, whose length contributes to aligned hashing. Thus
Rust hashes `"a\0"` like `"a"`, and `[0]` like `[]`, while TypeScript distinguishes
them. Native and recorded Rust agree with each other but differ from TypeScript
on these nine values. Observed validation blocks the ambiguous public input.

`oracle/typescript.json` remains the original 24-case source execution capture.
`normalized-capture.mjs` produces the separately labeled
`normalized-reference.json`, calling the official TS primitives on explicitly
trimmed inputs. Tests verify exact row identity, original/normalized input, the
expected divergence, and the independently captured normalized answer. They
never call Rust hashing to manufacture expected answers. No runtime behavior is
changed by this fixture.

## Boundaries and provenance

The capture hashes the exact licensed source, full retained IR, generated
JavaScript, capture script, runtime build/lock and official WASM package. The
source was freshly generated after adding its repository filename and license
header; the earlier research revisions remain separate evidence. Build-time
rendering requires all six recorded and observed APIs.

This is a native/recording/observed qualification fixture, **not an opaque proof
qualification**. Proof support remains a separate gate. The sibling fixed-input
fixture and its selected strict proof have their own source and artifact identity.

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
