---
id: RUST-ADR-0332
alias: ADR-0332
source_sha256: cae52255213d9b1f2631111be12ca8aabfc138c8de020cdb6d91adb11e1b66a3
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0332 — Reject recursive pure calls at the private IR admission boundary

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** planned remediation; 2026-10-07; independent audit ADR0327/#452 and R03013/#357; milestone rust-backend-v0.3.0. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0332 — Reject recursive pure calls at the private IR admission boundary

Status: planned remediation; 2026-10-07; independent audit ADR0327/#452 and R03013/#357; milestone rust-backend-v0.3.0.

### Problem
ADR0327 finding5 is confirmed only for direct private schema20 IR: pure self/mutual call cycles render successfully. The normal Compact frontend already rejects recursive source contracts. No recursive generated function was executed. Resource preflight notices a cycle but deliberately defers semantic rejection, and pure emission currently has no rejecting owner.

### Before / after
```text
Before: recursive pure IR -> resource cycle status -> emitted recursive Rust
After:  recursive pure IR -> structured semantic refusal before output publication
        acyclic shared/helper calls -> unchanged emitted Rust
```
Add an explicit bounded pure-call semantic validation using the existing graph facts/owner where possible. Preserve established malformed-schema/resource/unknown-call diagnostics and allow DAG sharing. Do not execute recursion, introduce new arbitrary depth restrictions, or turn resource metrics into a second conflicting semantic model. Choose a clear existing error variant if appropriate, otherwise document a non-exhaustive backend diagnostic addition. Runtime/IR/output ABI unchanged for valid contracts.

### Verification
Direct IR selfcycle/mutualcycle refuse, acyclic repeated helpers pass, ordinary frontend recursion refusals remain. Verify known undefined/helper error precedence and bounded ordinary-worker behavior. Focused tests, strict lint/format; validfixture outputs unchanged. Signed GPG/DCO and independent external retest. This is compiler input admission, outside stoppedADR0285.


### Final delivery

Locally delivered in signed GPG/DCO `dbfd7dc2`, pushed as part of dbfd7dc2. [Joined compiler audit remediation — 2026-10-07](references-0.3.0.md#note-155) records the 701-test final core gate, 197 fresh fixtures, 95.11% changed-backend coverage, actual consumer/MSRV checks, independent retest and exact source-to-commit binding. Earlier attempt receipts retain their original source scope.
