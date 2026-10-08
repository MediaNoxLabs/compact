---
id: RUST-ADR-0333
alias: ADR-0333
source_sha256: c688fcd724dce133c4a122d3c48e7ef75754c33e8d244b62c75cb6e137b32dc6
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0333 — Preserve historical passport separately from current adoption

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted source-role clarification; 2026-10-07; R03007/#351 and approved R03010 target policy; milestone rust-backend-v0.3.0. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0333 — Preserve the historical passport fixture separately from current adoption

Status: accepted source-role clarification; 2026-10-07; R03007/#351 and approved R03010 target policy; milestone rust-backend-v0.3.0.

### Problem
The full fixture census found48uninvoked public functions in passport-dogfood. That fixture is a predecessor pinned in milestone2, with credential-compact0.1.0-rc3; the user-approved0.3.0 target is midnight-vc-passport and credential-compact0.2.0. The approved targets explicitly prohibit substituting the old prototype for the selected application. Treating both source trees as one adoption would hide their materially different policies and miscount scope.

### Decision
Keep passport-dogfood unchanged as a historical regression fixture: include it in release regeneration, retain its five existing integration tests and independent observations, and preserve all original provenance. The current mandatory application adoption is vc-passport-adoption at its approved pinned closure, with75exports/202cases. The historical fixture's48direct gaps remain documented as untested old-application behavior, not passed, removed or diagnostic coverage.

The required source/export matrix must distinguish those roles. No old source, test or generated library is deleted. No runtime/backend primitive is excluded from milestone acceptance. Comparison found the same14runtime intrinsic names in both families and no old-only witness/ledger requirement; distinct old holder-binding/pseudonym policies remain explicitly unqualified. This source observation is not a security-equivalence proof.

### Before / after
```text
Before: two different passport families risk being counted as one required application
After:  current approved passport -> complete current adoption obligations
        historical predecessor -> retained rendering and five sampled regression tests
        48 old direct-call gaps -> explicit coverage limitation, no substitution claim
```
Historical/current public surfaces have77/75exports,49same names,28old-only and26new-only. Old source has18reachable Compact files versus17current. Equal names do not authorize evidence substitution. The decision follows approved target selection; it does not redefine instrumented coverage denominators or waive any current required primitive.

### Engineer evidence and consequences
Read the detailed historical-passport scope report for exact source/export hashes,152export-to-source mappings, primitive comparison, five test owners and fallback test plan if historical protocol certification is requested later. Existing historical suites remain part of final applicable regression execution. No new old-protocol test execution is claimed by this decision. Emitter/runtime/code/ABI unchanged. Parent#351 still needs execution/semantic completeness for its required cohort; stoppedADR0285 and audit gates remain open.


### Delivery record

Tracked in [MediaNoxLabs/compact#457](https://github.com/MediaNoxLabs/compact/issues/457), milestone `rust-backend-v0.3.0`. Accepted source-role clarification only; no code change or new execution claim. Detailed evidence: [Historical passport source-role decision](references-0.3.0.md#note-151). Current approved adoption and all primitive obligations remain required.
