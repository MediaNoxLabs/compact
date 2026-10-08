---
id: RUST-ADR-0343
alias: ADR-0343
source_sha256: 3601dad336c07e2f03987d2c620bc1a55a78ce8d387ebc9c32c672d36f9977ca
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0343 — Preserve previous compiler output when publication fails

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** not separately declared. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0343 — preserve prior compiler output after final rename failure

### Problem / owned requirement

`compactc --target rust` stages a complete directory and holds the output-name lock before replacement. Its `StagedOutput::publish` implementation (`tools/compact-rust-backend/src/bin/compactc.rs:584–640`) promises to restore the previous complete directory when moving the staged directory into place fails after the old directory has already moved to backup. The restore-success/error-return branch at615–623 currently lacks a direct regression. Existing tests cover successful replacement, interrupted-backup recovery and pre-publication failure; `check_rejections.py:164–233` does not force this particular rename failure.

This is output preservation during a live compiler operation. It does not promise power-loss durability, fsync semantics, arbitrary filesystem races, remote registry publication or runtime state rollback.

### Before / after

Before: old→backup, stage→output failure, backup→output restoration is implemented but not directly asserted.
After: the same production algorithm, plus a deterministic unit test proving restoration and original I/O error propagation.

### Exact component / test seam

Own only the existing `#[cfg(test)]` module of `src/bin/compactc.rs`. Add `failed_stage_rename_restores_previous_output_and_releases_lock` adjacent to completed-stage replacement test. No fault-injection production hook.

1. Create TempRoot and existing output with a nested sentinel containing independently pinned bytes.
2. Create `StagedOutput::new(&output)` and put an incomplete-new marker in its stage; retain stage and derived backup paths.
3. Remove only that test-owned stage directory before invoking `publish`. Existing output and lock remain intact. The missing stage deterministically causes stage→output rename to return `io::ErrorKind::NotFound`, after old→backup succeeds.
4. Call publish and downcast returned boxed error to `io::Error`; assert NotFound, rather than generic is_err.
5. Assert original nested sentinel bytes/path and exact old directory contents restored; new marker absent; backup and stage absent. No partial successful publication is reported.
6. Open lock sidecar and `try_lock` successfully after failed publish returns; unlock or drop. This verifies RAII releases the held publication lock without entering a potentially blocking second constructor.

Keep existing completed-stage positive test as paired success control. No concurrent filesystem mutation needed. This does not test the distinct restore-itself-fails branch, which remains explicit limited fault coverage.

### Alternatives / gates

Permission-based or racing failures are host-dependent and unnecessary. General injectable filesystem abstraction would add production complexity for a unit-testable branch.

Focused Rust1.99 offline/locked binary tests: new failure test, completed-stage success, existing interruption recovery/unsafe backup/lock tests (or compactc binary test module, small existing set). Strict Clippy for compactc binary and scoped rustfmt. Preserve source/lock/toolchain hashes, test log, exact error and before/after scoped diff. No proof, generated output, Cargo dependency, runtime, capture or frontend changes. Root records ADR/issue before implementation.


### Root decision — 2026-10-07

Accepted for bounded test-only implementation. Parent R03017/#361 and R03007/#351 remain open. Preserve historical receipts and the stopped ADR0285 boundary. No runtime or production behavior change is authorized by this decision.


### Delivery — 2026-10-07

Signed pushed commit1374c7ed;19compiler-driver tests and strict Clippy/format pass. Production unchanged. [Compiler publication and Field boundaries — 2026-10-07](references-0.3.0.md#note-146) contains exact receipts, archive and limits.
