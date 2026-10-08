---
id: RUST-ADR-0362
alias: ADR-0362
source_sha256: c0721b7d2cf8f769f4aef4e1312926f51f624eebaed0572ab6bc92159355f317
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0362 — Test output recovery after runtime compatibility admission

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted, implementation in progress. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.

**Current bounded disposition:** the dated local-delivery amendment below records signed delivery at `679be285d91baf87ae55ae3161140e371d259b62`. The original opening status remains historical. This corrects qualification-harness/source-anchor expectations; it does not establish successful full candidate qualification or broaden conformance/proof acceptance. Parent #364 remains open.

## Original decision and amendments

## ADR0362 — Test output recovery after runtime compatibility admission

Date: 2026-10-08
Status: accepted, implementation in progress
Parent: #364 final candidate qualification

### Problem statement

The full candidate gate at 2c798d2d fails two output-publication assertions in check_rejections.py. Both cascade from a stale ordering assumption: the interrupted-backup test selects an invalid runtime and expects output recovery first. ADR0261/987b56b1 deliberately validates the selected runtime before staging or frontend execution. The original recovery test predates that admission rule. The production ordering is correct; invalid runtime selection must not mutate output.

### Decision

Test admission refusal and recovery separately. Preserve all existing byte-preservation, unsafe-root, cleanup and successful replacement assertions. When a complete output has been moved to the interrupted backup, assert that invalid runtime admission leaves the backup and absent output untouched. Then select a valid runtime and a source with an exact frontend type error: staging must recover the previous output before the frontend refuses the rebuild.

### Before / after

```python
# Before: invalid runtime fails before recovery can run.
output.rename(interrupted_backup)
rejected = run_compiler(invalid_environment)
assert output.is_dir()
```

```python
# After: separate the two ordered behaviors (illustrative).
output.rename(interrupted_backup)
rejected = run_compiler(invalid_environment)
assert not output.exists()
assert snapshot(interrupted_backup) == preserved
rejected = run_compiler(valid_environment, invalid_source)
assert output.is_dir() and snapshot(output) == preserved
assert not interrupted_backup.exists()
```

### Emitter, runtime and domain impact

Only the compiler integration harness changes. No production compiler ordering, generated ABI, runtime API or domain objects change. This finding is in the Rust target test expectation; no TS or upstream ledger8 decoder/runtime defect is asserted. No new CoPS is needed for a stale branch-local test assumption.

### Validation and delivery

Retain the failed full-gate receipt. Run the corrected rejection gate against the current candidate compiler and newly built Scheme frontend. The exact Boolean diagnostic must prove failure after admission, while byte snapshots prove recovery and unchanged prior output. Then qualify the signed successor with the maintained full gate. Previous source/test receipts keep their original revisions.

Tracking issue: [#499](https://github.com/MediaNoxLabs/compact/issues/499), rust-backend-v0.3.0.

### Local delivery

Signed/DCO commit `679be285d91baf87ae55ae3161140e371d259b62` verifies Good GPG and is pushed. Exact rejection gate passes with full frontend diagnostic equality, backup/output byte checks and existing unsafe-root assertions. [Candidate consumers coverage and ADR0362 — 2026-10-08](references-0.3.0.md#note-140) retains original failed gate and corrected focused pass. Full successor qualification is running; parent #364 remains open.
