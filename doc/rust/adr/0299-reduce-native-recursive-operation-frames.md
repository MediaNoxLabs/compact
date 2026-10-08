---
id: RUST-ADR-0299
alias: ADR-0299
source_sha256: cd47e452c1ed3573c416921e037a9aba651a56e70943a67de5f46bd7a55651ab
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0299 — Reduce native recursive operation frames

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted isolated implementation, 2026-10-07. Parent R030-17/#361 and R030-18/#362; ADR0291 numerical defaults remain pending. This record precedes candidate source edits. Production port requires separate root review. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0299 — Reduce native recursive operation frames

Status: accepted isolated implementation, 2026-10-07. Parent R030-17/#361 and R030-18/#362; ADR0291 numerical defaults remain pending. This record precedes candidate source edits. Production port requires separate root review.

### Problem and evidence

After signed ADR0292 and ADR0296, finite ordinary-worker calibration exposes a distinct native-expression failure. A stateful circuit returning Field with sixteen nested transient hashes aborts; sixteen nested Field additions also abort. Depth twelve succeeds. The corresponding pure expressions pass through depth forty-eight. Inputs are valid constructed typed IR with no public reads/witnesses, one callable and only 29 nodes for hash16. This is a native rendering failure, not a recording-admission claim.

The debugger stops in `stateful::expression::render_operation_expression`'s stack probe. Its measured aarch64 Darwin debug frame is 141,040 bytes, plus a 192-byte dispatcher; the current control and aggregate helper frames are 16,560 and 15,424 bytes. Evidence: `/tmp/compact-adr291/joined/stateful-hash16-backtrace.log`, `additional-frames.json`, `stateful-expression-controls.json`, and exact inputs/logs under `controls/`. Frozen joined source is f6a87df1; toolchain Rust1.99.0, Apple M2 Max, incremental disabled. No thread-stack override or caught panic.

### Decision and ownership

Keep `render_state_expression`'s existing structural routing. Make the remaining `render_operation_expression` selection thin and move complete unchanged arms into cohesive private helpers in the same `src/stateful/expression.rs` owner. Proposed groups: ledger observations; local calls and witnesses; Kernel/Zswap effects; crypto primitives; numeric conversions/arithmetic/comparison; assertion. A small fallback preserves the existing pure-expression delegation exactly. Measure each helper after extraction; revise grouping only with exact arm-body/order evidence if a recursive helper remains unnecessarily large.

Retain every match pattern and alternative, left-to-right operand materialization, typed result checks, `statements` order, `next_temp`, witness/query effects, context ownership and errors. Helpers receive only the existing explicit inputs they use. No new evaluator, memoization, stateful-result profile, recording admission, crypto primitive, runtime behavior, ABI50 or schema20 change. Do not touch recorded.rs, composition, relation-owned paths or resource preflight.

Before: all native operation recursion reserves the entire operation-match frame and the finite hash/add16 controls abort.
After required acceptance: the same controls complete on ordinary workers, pure controls remain unchanged, and legitimately supported recorded controls retain their actual capabilities. No forced recorded support for a native-only shape. Numerical limits are not used to conceal the demonstrated failure.

### Validation

1. Retain baseline hash/add16 failures with core files disabled and finite subprocess timeouts. Add maintained native hash/add/mixed recursion tests alongside pure and existing recorded controls. Assert exact legitimate capabilities separately.
2. Audit all moved complete arms using normalized syn tokens and original within-helper order. Guard, error and fallback precedence must remain unchanged.
3. Measure dispatcher and helper prologues with the same compiler/profile, including simultaneous dispatcher/helper frames. Finite tests define measured support, not arbitrary-depth or release-stack guarantees.
4. Full current 193-source ordinary-worker Rust/capability byte equality, focused/full backend tests and strict all-target/all-feature Clippy. No generated fixture rewrite is intended. Proofs need not repeat if every emitted byte is unchanged.
5. Root reviews isolated evidence before live port. Resume ADR0291 cross-family and cross-call calibration only on the accepted repaired source; no weighted defaults are adopted by this ADR.

### Limits

This is a bounded engineering repair, not a universal resource-exhaustion guarantee. The existing 93,600-byte legacy Let helper remains measured and visible. Neither a stack-size increase, catch_unwind, lowered source acceptance ceiling nor performance claim is part of this slice.


Tracking: https://github.com/MediaNoxLabs/compact/issues/423, created before candidate code.

### Candidate and approved live port validated — 2026-10-07

374 backend tests, 13 live ordinary-worker tests, strict Clippy, 193 complete Rust/capability comparisons and exact 30-arm token/order audit pass. Native-only profiles remain native-only. Root reviewed and authorized only the two owned files; source is frozen for signing. See [ADR0299 native operation frame repair — 2026-10-07](references-0.3.0.md#note-085). Live receipt SHA256 `5e655f667fb04b3690858b0d720fd3a83f359cb34898036b271da9fc02d3c50a`.


### Signed integration — 2026-10-07

Root integrated the reviewed repair as `ca97a69fe9f1ebbb9ccf5de1acacedd26a30b6ee`, verified GPG and DCO. The signed two-file hashes match the live receipt. Resource limits remain separate and unadopted.
