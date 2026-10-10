# Generated OwnPublicKey recording fixture (#568)

Genuine ledger-8 Compact source: `examples/rust_backend/effect_plan_own_public_key.compact`. The retained full schema-20 IR is emitted by the pinned frontend without Rust rewriting. Build-time generation must expose both recorded and observed `run`; the compiler test independently checks actual algebra ownership.

The source increments a Counter, then calls `mark(1)`, a lazy first `ownPublicKey`, `mark(2)`, a lazy second `ownPublicKey`, and `mark(3)`. The named result retains both keys and all witness values. The false branch returns default keys and must work without an execution key.

Three tests cover five successful input/branch rows; five exact failure-prefix rows; and three observed identity binding rows. Native, generated recorded, generated observed, and independent official RecordingFrame execution are compared for output, state, effects, gas, private state/FAB, public verify program and replay. Configured all-zero keys are valid and distinct from absent keys. Literal prepared atoms qualify fixed-key normalization and field order. The identity binding test wraps a real generated recording in the public adapter and expects its existing exact mismatch error.

Callback bodies do not perform ledger reads, so callback metering introduces no separate TS view-read cost. No new same-source TS execution or proof claim is made. Existing official native key vectors are background evidence only. The Counter supplies the nonempty public transcript needed to test ordinary call preparation; no key generation/proof/network fixture is needed for this primitive.

The baseline frontend capability report has `run.recorded = false`. Source rendering success does not establish unchanged Rust compilation. Parent runs the generated fixture after integrating the primitive.
