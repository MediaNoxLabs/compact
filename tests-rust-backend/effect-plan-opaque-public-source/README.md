# Opaque public interfaces through the recording algebra (#555)

`build.rs` emits the complete retained frontend IR for
`examples/rust_backend/effect_plan_opaque_public.compact` through the current
Rust backend. All three public circuits must expose recorded execution and
observed-call preparation. Compiler selection tests separately assert their
actual algebra owner; capability flags alone do not establish that attribution.
The generated library is included unchanged and exposes `RETAINED_IR` for
source-bound downstream qualification.

The source offers String and Bytes roundtrips returning named products, plus
`run(Envelope, Boolean) -> Result`. `run` passes a nested owned argument through
a stateful helper that increments a Counter, optionally replaces one Bytes leaf
with a zero-argument witness result, then returns both the chosen and original
values. The source exercises ownership reuse and lazy branching without widening
witness argument policy.

## Runtime cases

Four tests cover 27 rows:

- Seven String rows: empty, Unicode, internal zero, long text and three terminal
  zero cases.
- Eight Bytes rows: empty, invalid UTF-8, leading/internal zeros, long payload and
  three terminal zero cases.
- Six nested product/branch rows: original data is reused, selected callbacks
  occur once after the Counter increment, and unselected callbacks remain lazy
  even when configured with a noncanonical result.
- Three invalid nested public inputs are refused before callbacks. Three
  canonical inputs with invalid witness-derived public outputs are refused
  after the callback observes the Counter at one. Direct native/recorded
  execution still returns the original unnormalized Bytes values.

Native/recorded success comparisons include typed result, private state, public
state, effects, gas and private outputs; the public transcript is nonempty and
replays to the same state/effects. Generated observed methods prepare canonical
public input/output atoms and compare them directly with known original byte
sequences. These comparisons do not use a second potentially lossy conversion
as an oracle.

The public input guard executes before the body, and the public output guard
executes after local recording but before constructing a RecordedCall. Witness
private FAB conversion remains the existing low-level policy and can normalize
terminal zeros; this fixture does not claim it is lossless or changed. Likewise
raw runtime `Into<AlignedValue>` and generic `prepare_call` are unchanged.

## Boundaries

These are runtime and preparation tests. Deterministic installed verifier
carriers are sufficient for preparation but provide no proof verification or
funded-transaction evidence. Any real source-bound proof/application evidence
is recorded separately by its adapter. There is no witness argument expansion,
selector priority change, membership-family retirement, network support claim
or bound on dynamic opaque payload size in this fixture.

```sh
cargo test -p compact-rust-effect-plan-opaque-public-source-fixture --all-features --test boundaries
```
