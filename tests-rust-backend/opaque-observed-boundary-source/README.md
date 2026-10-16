# Generated observed-call opaque boundaries (#556)

Each integration test includes a complete contract freshly emitted by the
current backend from retained full schema-20 IR. No generated methods are
rewritten or simulated. `bboard` uses maintained test-center source; `nested`
uses `examples/rust_backend/opaque_observed_nested.compact`. Root admission,
selector policies and low-level runtime conversion remain unchanged.

Four test functions cover fourteen cases:

- Three noncanonical bboard String inputs fail before `local_secret_key`.
- Four canonical bboard lifecycle rows (empty, Unicode, internal zero and long
  text) prepare exact public input/output atoms and run ordered callbacks.
- Three noncanonical nested String inputs fail before the Boolean witness or
  Counter effect. Direct native and recorded calls still accept these unused
  local values, demonstrating the narrower observed ABI policy.
- Four canonical nested String inputs prepare exact atoms, preserve a genuine
  nonempty transcript and record the witness after the Counter increment.

The nested argument is deliberately unused by the body: successful execution
alone cannot establish that its public input encoding preserves raw bytes.
Expected prepared atoms are compared directly with the original UTF-8 bytes,
not another potentially normalizing encoder.

## Scope

This fixture verifies real generated observed input guards and canonical output
preparation. Runtime rejection of an opaque result supplied by a witness is
**deferred**: the current root/witness policies do not admit a source circuit
that exposes that case, and the narrow boundary fix does not widen them.
Compiler structural tests can inspect emitted output guards independently.
The bboard lifecycle remains legacy-owned; it is not evidence of algebra
retirement or new public opaque root eligibility.

Deterministic verifier carriers are installed solely to exercise observed-call
preparation. No proof generation, cryptographic verification, funded transaction
application, network interaction or universal opaque payload claim is made.

```sh
cargo test -p compact-rust-opaque-observed-boundary-source-fixture --all-features --tests
```

## Larger tuple compatibility regression

`tuple.rs` is a separately labeled typed-IR compatibility variant of the full
maintained shielded-send receipt. The build appends two unused public parameters
to `send_to_user`: a nine-member tuple (OpaqueBytes plus eight Fields) and a
length-two vector of that tuple. It preserves every body action and return
expression, emits the full contract, and requires recorded/observed capability.
This is neither a new Compact source nor an algebra ownership claim.

One additional test checks three noncanonical positions (direct tuple, first
vector element, second vector element), then confirms canonical arguments reach
the unchanged body's locked-provisional-allocation error, also asserted through
direct generated recorded execution with the same context. Complete generated compilation
protects the existing public ABI from inadvertently acquiring the narrower
`CellValue` tuple-arity bound. No transaction/proof claim follows from this test.
