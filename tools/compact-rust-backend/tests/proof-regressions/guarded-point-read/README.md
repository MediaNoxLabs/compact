# Guarded point reads can feed zero coordinates into unselected curve operations

This is the exact original source for the #517 qualification failure, retained
before the proof-compatible source hoisted its shared point read. See CoPS-004 /
issue #518 for required shared-compiler follow-up.

Native and recorded execution pass. `compose` proves and applies. Proving
`read_branch(false, generator(7), 3)` reaches an official subgroup-conversion
panic: the unselected branch's guarded public inputs become `(0,0)` and its
unguarded `ec_add` still executes. `(0,0)` is not Compact's identity `(0,1)`.
`midnight-zkir 2.1.0` `ir_vm.rs:325–327` inserts zero for inactive public inputs;
`EcAdd`/`EcMul` evaluation at lines 450/460 is unconditional. Conversion panics
at `midnight-circuits 6.0.0` `ecc/curves.rs:114–117`.

The exact source compiled with the same Scheme frontend for TS and Rust emitted
byte-identical `read_branch.zkir`. This is evidence of a shared failing proof
artifact, **not an executed TS proof test**.

To reproduce artifact generation, compile this source with the pinned ledger8
Scheme frontend and SDK 0.29.0 (zkir 2.1.0), for each target:

```sh
COMPACTC_SCHEME=/path/to/compactc-scheme compactc --target rust --skip-zk \
  tools/compact-rust-backend/tests/proof-regressions/guarded-point-read/effect_plan_jubjub.compact /tmp/guarded-point-rust
COMPACTC_SCHEME=/path/to/compactc-scheme compactc --target ts --skip-zk \
  tools/compact-rust-backend/tests/proof-regressions/guarded-point-read/effect_plan_jubjub.compact /tmp/guarded-point-ts
```

For the original failure receipt, the then-current proof adapter was generated
from this exact IR and ran `--effect-plan-jubjub` with matching keys. The current
adapter deliberately rejects this old IR before loading keys, because its
source-bound fixture now uses the documented hoisted-read mitigation. Reproduce
the original proving case with an isolated fixture generated from this source;
do not bypass the current adapter's input pairing guard.

No proof keys are committed. The local failure receipt retains source, IR,
backtrace and key/ZKIR hashes. Hoisting the shared read is a source-level
mitigation for the positive qualification fixture, not a compiler repair or
proof support claim for this original form.
