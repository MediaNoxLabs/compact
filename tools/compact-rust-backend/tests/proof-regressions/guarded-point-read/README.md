# Guarded point operands in ZKIR v2

This is the exact original source for the #517 qualification failure, retained
before the proof-compatible source hoisted its shared point read. Issue #518 /
CoPS-004 track the shared compiler correction below.

## Historical failure (before #518)

With the pre-fix compiler, native and recorded execution pass. `compose` proves
and applies. Proving
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
from this exact IR and ran `--effect-plan-jubjub` with matching keys. The `--effect-plan-jubjub`
adapter deliberately rejects this IR before loading keys, because its
source-bound fixture uses the documented hoisted-read mitigation. The separate
`--guarded-point-read` adapter uses the exact original source retained here.
Neither adapter accepts the other source's IR; do not bypass this pairing check.

No proof keys are committed. The local failure receipt retains source, IR,
backtrace and key/ZKIR hashes. Hoisting the shared read is a source-level
mitigation for the positive qualification fixture, not a compiler repair or
proof support claim for this original form.

## Shared compiler correction

At a guarded, resolved `ecAdd`/`ecMul` call, the v2 emitter selects `(0, 1)`
for each inactive point operand immediately before the curve gate. Active
coordinates and multiplication scalars remain unchanged. Public/private input
zero constraints and transcript operations retain their original behavior.
Constant-true calls keep their previous artifacts. Affected ZKIR requires fresh
keys; key compatibility is not claimed.

## Local regression gates

Run these against distinct isolated pre-fix and candidate Scheme builds:

```sh
python3 tools/compact-rust-backend/guarded_point_zkir.py \
  --baseline-compiler /path/to/baseline/compactc \
  --candidate-compiler /path/to/candidate/compactc \
  --output /tmp/guarded-point-emission
python3 tools/compact-rust-backend/test_guarded_point_zkir.py \
  --artifacts /tmp/guarded-point-emission -v
```

The gate checks nine finite circuit graphs, complete Boolean guard tables for
these fixtures, input/transcript preservation, and byte equality for unaffected
controls. Mutation tests reject wrong guards, wrong identity coordinates,
changed active operands/scalars, and escaped transcript substitutions. These
are emission checks, not proof execution.

### Retained inputs during ordinary test discovery

The existing replay kit's six-file `graph/` can supply the original nested pair
and four authority files during ordinary unittest discovery:

```sh
COMPACT_GUARDED_POINT_ROOT=/absolute/path/to/retained-graph \
  python3 -B -m unittest discover -s tools/compact-rust-backend \
  -p test_guarded_point_zkir.py
```

This mode authenticates the exact original graph/authority hashes and retained
nested source before and after each case. Every original mutation assertion
executes against a newly loaded in-memory graph; inputs are never modified or
replaced by a cached result. Missing, changed or escaped inputs fail. The full
gate requires `--guarded-point-artifacts /absolute/path/to/retained-graph` and
passes the same explicit selection to discovery.

The ten original test IDs (including the historical "fresh" case name) are
preserved. In this input mode they are **retained nested graph mutation controls**,
not fresh current-compiler emission, all nine producer fixtures, official VM or
proof execution, or the separate complete historical qualification. The exact
original receipts can mention other historical logs: authenticating the six-file
closure does not assert those other files were supplied or checked. Direct
`--artifacts OUTPUT` keeps the original producer-output workflow above; no
retained-input fallback is applied to that explicit selection.

The Rust fixture packages `compact-rust-guarded-point-read-source-fixture` and
`compact-rust-guarded-point-nested-source-fixture` bind the exact retained IR.
The proof smoke commands `--guarded-point-read <artifacts>` and
`--guarded-point-nested <artifacts>` require fresh keys and matching source IR.
They exercise the original composition and both branch choices, plus all four
nested guard combinations, through native execution, recording, the official
preimage checker, proving, verification and ledger application. The smoke
harness uses its existing unbalanced transaction policy; this is not production
transaction qualification. No proof keys are committed.

Current ordinary discovery reports manual graph cases as skipped when no artifacts are selected. Explicit invalid inputs fail. Full qualification requires authenticated graph inputs and executes the original controls; see the [portable-fixture guide](../../../../../doc/rust/guides/retained-python-fixtures.md). Retained graph evidence is not fresh current compiler emission or proof execution.
