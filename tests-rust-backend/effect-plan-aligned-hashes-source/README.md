# Aligned hashes: genuine ledger8 source acceptance

This fixture uses default ZKIRv2 and the official runtime's existing
`hash_to_curve` and `persistent_hash` primitives. Its source, retained whole IR,
and selected `hashes` proof artifacts have one frozen source identity. No ZKIRv3
flag or new cryptographic implementation is required.

## Four small contracts in one source

| Export | Purpose | Callback order and ledger view |
|---|---|---|
| `close(value)` | Local Unit helper checks point and digest | expectedPoint → expectedDigest, Counter 0 |
| `close_hash(value)` | Isolates nonbyte PersistentHash | expectedDigest, Counter 0 |
| `hashes(selected)` | Named result exposes point X and digest | Counter increment, curveInput → hashInput, Counter 1 |
| `hash_only(selected)` | Isolates named digest result | Counter increment, hashInput, Counter 1 |

At the frozen pre-#582 baseline, both Unit exports were recorded by
`selector.unit-composition`, but the checked candidate refused HashToCurve or
PersistentHash's byte-only input policy. Both named-return exports were not
recorded. The fixture build now requires all four recorded and observed APIs;
this does not claim retirement of the broad Unit selector.

## Test design

Three tests cover 24 uniquely identified fresh TypeScript cases: 15 successes and
nine refusals. Values zero, one and two exercise all four roots; a distinct
curve/hash input case checks order. Lazy branches suppress providers. Point and
digest mismatches stop at the exact callback prefix, as do first/second provider
errors. Every configuration is bound by ID, circuit, arguments, configured
witness values, selection, failure and mismatch choices.

The manual recorder reads point coordinates and digest bytes from frozen official
TypeScript answers. It does not call either Rust hash under test. The TypeScript
capture independently computes these answers using the official aligned APIs;
they are not claimed as externally published cryptographic vectors. Point
construction validates the captured coordinates through the official-backed
runtime before use.

Native, recorded, observed and manual execution must agree on result, ledger
state/effects, private state/transcript and metered execution gas. Recorded and
manual verifier programs and replay agree separately; replay gas is compared to
manual replay rather than execution gas. Every successful root has a Counter
effect, including lazy false branches, and prepares a nonempty call with the
original input and output FAB. Observed state remains immutable on failures.

`tests/common/answers.rs` owns the independent values, `witness.rs` owns callback
and manual semantics, and `state.rs` owns observed ledger setup. The main test
module contains the bounded scenario matrix and cross-path assertions.

## Source and proof boundaries

The selected `hashes(true)` circuit compiles using official default-ZKIRv2 tools
at k=13 / 2,746 rows. Key generation alone is not proof validation. Real proving,
changed-binding refusal and strict funded application belong to the separately
run proof harness, using the same retained source and key identity.

This fixture qualifies fixed Field inputs only. Opaque inputs require their own
source/runtime evidence; in particular the official frontend rejects opaque
PersistentHash inputs even though Rust carriers implement alignment traits.
Witness-result, public-signature and default-value policies remain independent.

## Reproduce

```sh
compactc --target rust --target ts --skip-zk \
  examples/rust_backend/effect_plan_aligned_hashes.compact /path/to/output
node tests-rust-backend/effect-plan-aligned-hashes-source/oracle/capture.mjs \
  examples/rust_backend/effect_plan_aligned_hashes.compact \
  /path/to/output/contract /path/to/official/compact-runtime \
  tests-rust-backend/effect-plan-aligned-hashes-source/oracle/typescript.json
cargo test --locked -p compact-rust-effect-plan-aligned-hashes-source-fixture \
  --features ledger-transaction
```

The capture hashes its exact source, IR, generated JavaScript, script, official
runtime build, lock and WASM files. The generated Rust is built unchanged from the
retained IR. Proof key files remain local artifacts.
