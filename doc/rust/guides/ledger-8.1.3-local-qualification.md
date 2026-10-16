# Ledger8.1.3 compatibility and local qualification

## Current source profile

The native runtime uses official ledger8.1.3 revision
`b5d3e965a18e4cac17fac4f38446fc55c524439a`, runtime ABI52 and private Rust IR20.
Capability reports use schema3 and source-origin transport uses schema3.
The supported v2 cohort is ZKIR/transient-crypto2.1.1, base-crypto1.0.2,
serialization1.1.1, proofs0.7.1, circuits6.1.0, stdlib1.1.0 and curves0.2.0.
The runtime compatibility manifest and locked dependencies are authoritative.
Generated consumers declare Rust1.89 or newer. Earlier0.3.0 Rust1.88 receipts
retain their original graph; they do not establish this graph's minimum version.

## Evidence scopes

The source profile above is the current dependency selection. Original selected
proof/application and public-consumer receipts retain their exact source, tool
and dependency epochs. They are not relabeled as fresh execution of this
publication cohort. Current local compiler and finite consumer-role qualification
is recorded separately with its frozen inputs; it does not rerun all retained
proof calls or establish general performance or specification coverage.

This focused core extraction preserves compiler/runtime semantics and the
ordinary proof gates. Its local validation and independent review are listed in
the PR. No complete new historical qualification, proof-qualified release, MSRV,
platform/network deployment or package publication follows from this document.
The original source-policy JSON preserves its dated receipt descriptions.

## Ordinary local tests

Use the repository toolchain and affected tests first. Actual circuit tests still
perform their normal key generation, proofs, verification, assertions and negative
controls. No external cache is required by the compiler or default test entry
points. An optional local artifact-cache adapter is a separate delivery slice.

The shared guarded-Jubjub compiler correction changes affected v2 circuit bytes;
regenerate matching prover/verifier artifacts and qualify them before replacing
any deployed verifier identity. ZKIR-v3 native/Keccak qualification remains separate
from v2 ledger proof/application support.

The separate authenticated historical qualification has an accepted 974-case
receipt for its frozen `ab132650` epoch and external inputs. Its Git metadata
incident remains disclosed; continuous metadata invariance is not claimed.
It is not current language coverage or a formal proof. The [local #508 closure report](../recording-effect-algebra/closure-report.md)
records the composed source/evidence joins, meaningful signed contribution
history and separately recorded final acceptance. The [historical results summary](../../../tools/compact-rust-backend/tests/performance/historical-path-validation-benchmark.md)
records the three-case measurement and its limits. The experimental patch,
measurement driver and detailed evidence remain archived outside this checkout;
no path-validation optimization is enabled in the current runner.

The ordinary full gate selects [explicit retained conformance baseline inputs](retained-conformance-baseline.md)
for its one original repository-baseline law. Its historical positive result does
not relabel the current implementation as conformant or replace that separate
historical qualification.

Full qualification requires the [retained guarded-point graph inputs](../../../tools/compact-rust-backend/tests/proof-regressions/guarded-point-read/README.md#retained-inputs-during-ordinary-test-discovery).
The full gate selects them with `--guarded-point-artifacts`; those ten mutation
controls retain their original graph/source epoch, independently of fresh
compiler emission and proof qualification.

Without selected graph artifacts, ordinary discovery explicitly skips manual cases. The [portable-fixture guide](retained-python-fixtures.md) explains the default public baseline, invalid-input refusals and exact manual-input selection. Compiler0.31.135 / JSruntime0.16.102 source identities and scoped Rust1.89 checks are recorded in the closure report; this is not a whole-workspace MSRV or new proof claim.
