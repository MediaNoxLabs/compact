---
id: RUST-ADR-0296
alias: ADR-0296
source_sha256: 71469e5393bd6592e66e2593c6f21d7b83a90ed118eaf1aab9a2a44154897ee2
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0296 — Reduce legacy recorded action recursion frames

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted isolated implementation, 2026-10-07. Root approval precedes code; production port requires separate release. Parent R030-17/#361 and R030-18/#362. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0296 — Reduce legacy recorded action recursion frames

Status: accepted isolated implementation, 2026-10-07. Root approval precedes code; production port requires separate release. Parent R030-17/#361 and R030-18/#362.

### Concrete failure and scope

ADR0292 fixed the pure and typed recorded expression dispatch frames and recovered the existing corpus. Subsequent finite ADR0291 calibration found a separate failure in the legacy recorded action owner. A valid acyclic chain of twelve Unit circuits (eleven calls ending in an assertion) aborts on an ordinary Rust worker; chains of ten and eleven succeed with `recorded=true`. A twelve-level repeated-call diamond also aborts; eleven levels succeeds. These are constructed typed IR controls, not a claim about frontend source limits.

LLDB places the exhausted stack in `recorded::render_recorded_item::append_steps`, recursively entered through its CircuitCall arm. The measured aarch64 Darwin debug prologue reserves 176,560 bytes (32 + 0x2b000 + 0x190). This is an observed frame size for the retained binary/toolchain, not a universal release/profile size. No stack-size override or catch_unwind was used. The renderer's native phase has completed before this failure.

Evidence: `/tmp/compact-adr291/calibration/{resource-calibrate,results.json,call-boundary.json,lldb-chain12.log,append-steps-prologue.txt}`. Driver source: `/tmp/compact-adr292/backend/src/bin/resource-calibrate.rs`. The binary includes the isolated ADR0292 repairs and experimental metrics; metrics are measured before the failing render, and no resource rejection is active.

### Exact owner and proposed change

Frozen source: `/tmp/compact-adr292/backend/src/recorded.rs:4107` through 6431. The owner is the nested `append_steps` function within `render_recorded_item`. It currently carries declarations, local substitutions, shared-callee membership, ordered statements, temporary numbering, cycle membership and diagnostic precision explicitly.

Make `append_steps` a thin match dispatcher and move unchanged complete arms into private same-owner helper families. Initial useful families are control/call (Sequence, If, CircuitCall), lexical Let, assertion/witness, scalar/collection writes, and Merkle writes. Preserve pattern alternatives, guards, exact arm bodies, and relative precedence. A token audit will determine the final grouping; it will not rewrite semantics to fit a predetermined module layout. Keep the existing explicit inputs instead of introducing a strategy framework or a new evaluator.

The large recursive arm is **Let**, starting at line 4307 and extending to the Assert arm at 5589. PureCall itself is the small preceding closed-assertion arm. The Let family recursively lowers continuations and therefore must also be measured; merely extracting the top-level match is insufficient evidence. If its retained recursive frame remains a problem, report the measurement and propose the smallest further control/leaf separation before changing arm semantics.

Preserve: left-to-right argument materialization, fresh callee-local maps, shared-helper dispatch, `visiting` insertion/removal and cycle errors, exact failure path and `LegacyActionPrecision`, selected branch statements, temporary identifiers, and generated source/capability bytes. No changes to shared Plan, composition admission, runtime, ABI50, schema20, or resource ceilings.

### Before / after contract

Before: a valid twelve-call graph reaches legacy recording and aborts the ordinary worker despite only 74 measured nodes, syntax depth 3, expanded depth 25 and expanded work 480.

After, if accepted and measured: the same graph completes recording on the ordinary worker with unchanged output and capability. The action dispatcher holds only dispatch state; each cohesive helper holds only its own existing lowering temporaries. This is not a promise of arbitrary recursion support, a throughput improvement, or a new accepted input domain.

### Retained tests and delivery gates

1. Preserve baseline chain12 and diamond12 subprocess abort evidence with core files disabled and a finite timeout. Add maintained ordinary-worker regressions requiring successful `recorded` capability, including a mixed call/control path.
2. Preserve malformed callee, arity, cycle, lexical scope, branch and path/precision diagnostics with focused existing tests plus a specific control if missing. Do not turn unknown/cyclic IR into a resource error.
3. Compare normalized syn tokens for every moved complete arm and relative order within each helper. Review all non-movement changes separately.
4. Measure dispatch/helper frames in the same debug profile; verify no remaining large recursive Let frame hides behind the new dispatcher.
5. Run backend unit/integration tests and strict all-target/all-feature Clippy. Compare complete Rust source and capability bytes for the accepted current corpus (at least the retained 184 controls, plus any newly integrated source fixtures). No generated fixture rewrite should be necessary. Repeat proof/consumer work only if emitted bytes unexpectedly change, which would first require investigation.
6. Recalibrate ADR0291 call/depth limits only after this repair passes. Do not adopt a call-depth limit of eleven to conceal the demonstrated failure.

### Ownership / dependencies

Own `tools/compact-rust-backend/src/recorded.rs` only in the legacy nested action-owner region, plus the maintained worker regression test file. Coordinate with ADR0295 before any shared-file port. No typed_plan/composition change is proposed. Start from signed ADR0292 plus current accepted dependencies once root authorizes implementation. ADR0291 preflight remains a separate pending change.

### Measurement limits

The existing calibration is a finite debugging experiment, not a benchmark or security assurance. Exact binary, source, logs, and toolchain provenance are retained alongside the ADR0291/0292 receipts. Numerical resource ceilings remain pending; the supported corpus must not be rejected to avoid repairing an observed renderer defect.


Tracking: https://github.com/MediaNoxLabs/compact/issues/420 (milestone 3). Created before candidate source changes. Frozen isolated source base 40047fb8; live port remains separately controlled.


### Isolated candidate validated — 2026-10-07

193/193 complete Rust/capability equality, 371 distinct backend tests across full + final focused runs, strict Clippy, and exact 27-arm token/order audit. Baseline chain12/diamond12/mixed12 abort; candidate records successfully on ordinary workers. Dispatcher 416 bytes; Let helper remains 93,600 bytes, explicitly retained for later resource calibration. No live port yet. See [ADR0296 isolated action frame repair — 2026-10-07](references-0.3.0.md#note-079). Receipt SHA256 `6dc8288fea270f8740c89468b45cf5650fd2dbc9372b6ad4d2b9a6aea201d608`.


### Approved live port validated — 2026-10-07

Root approved exactly two files on unchanged base `40047fb86cac845c5706ab404bea444d6ac71762`. Live ten ordinary-worker tests and strict all-target/all-feature Clippy pass. Both live files match the already audited isolated hashes. Source frozen for root review/signing; this agent made no commit. Live receipt `/tmp/compact-adr296/live-receipt.json`, SHA256 `da45dcd5318a8807167ede01dd77330681bc9a4103fa3f6db6ea05211dce66b6`. Existing isolated 193-source equality and 371 distinct tests remain bound to the identical source.


### Signed integration — 2026-10-07

Root integrated the approved two-file repair as `f6a87df1d247fcea4217cafd2a894b0f52624d7b` with verified GPG and DCO. Both signed source hashes match the live receipt; no runtime/admission/ABI change. Numerical resource calibration remains separately pending ADR0291.
