# Source-bound Jubjub algebra qualification

The build script renders the exact retained IR from
`examples/rust_backend/effect_plan_jubjub.compact` with the current default
backend. It requires recorded and observed-call APIs for all three exports.

- `compose`: the legacy family's four value operations, nested lexical bindings,
  one point Cell write, and a returned point distinct from the stored point.
- `read_branch`: explicitly new composition of two point Cells, point reads and
  a Unit-valued conditional. Its shared point read is explicitly hoisted before
  the branch because inactive guarded reads otherwise feed `(0,0)` into
  unguarded ZKIR curve operations (CoPS-004 / #518). No point-valued
  conditional/equality or proof support for the original guarded-read form is implied.
- `raw_after_write`: a raw canonical-scalar check after an earlier state write.

The runtime tests execute 56 combinations: identity/nonidentity point × seven
retained scalar boundaries × four scenarios. Fifty succeed and six refuse with
`InvalidJubjubScalar`. Native and recorded execution are compared with an
independent source-order program assembled from official runtime crypto and slot
adapters. Checks cover result, state, effects, execution gas, private state,
public transcript and independent batched replay gas. ContractLab rollback after
an executed write prefix has a separate negative control.

The original `jubjub-scalar-cell` fixture retains its TS/oracle provenance and
boundary tests; its generated code must be regenerated during retirement. These
finite scenarios complement the backend's independent legacy-grammar inclusion
checks. Neither source counts nor successful proofs establish complete language
coverage.

`compact-rust-proof-smoke --effect-plan-jubjub <matching compiler output>` first
checks exact retained IR before loading keys. It qualifies `compose` and both
`read_branch` cases using the shared unbalanced smoke policy, with separate
native/recorded, trace preparation, proof/verification and ledger-application
checks. Raw scalar failure is tested without claiming a proof of a failing call.

```sh
cargo +1.99.0 test -p compact-rust-effect-plan-jubjub-source-fixture --locked
compact-rust-proof-smoke --effect-plan-jubjub /path/to/matching-compiler-output
```

## Qualification boundaries discovered during retirement

The original guarded-read source is retained under
`tools/compact-rust-backend/tests/proof-regressions/guarded-point-read/`, with
reproduction instructions. Native/recorded acceptance for that form is separate
from proof compatibility; its false-branch proof currently panics in the shared
official circuit stack. The positive proof adapter uses only the exact mitigated
source/IR and rejects the original IR before loading keys.

The semantic point-result routing also makes the existing
`witness_jubjub::generator` recordable (one Field witness, reduction, generator).
Its witness order/private transcript and unchanged TS oracle result have focused
fixture tests. Its public program is empty, so ordinary call preparation still
returns `EmptyTranscript`; recording availability does not claim a provable
ledger call.

A separate `effect-plan-jubjub-nested-layout` fixture crate is generated from explicitly synthetic IR with
16 declared point Cells using the native layout `[0,0]`, `[1,0]` … `[1,14]`.
Its tests cover nested query/state behavior, neighbor preservation and rejection
of a flat runtime state. The source-bound top-level module and proof adapter are
unchanged by that layout control; no Compact-source or proof claim is made for
the synthetic crate.
