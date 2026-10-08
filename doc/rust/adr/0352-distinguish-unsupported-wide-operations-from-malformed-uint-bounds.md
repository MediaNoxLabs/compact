---
id: RUST-ADR-0352
alias: ADR-0352
source_sha256: 1c471a0ed2ffdb8d9f5d499dacdfc669f2a4b928ec7ff60c8860f967a3948b89
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0352 — Distinguish unsupported wide operations from malformed Uint bounds

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted for focused implementation. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0352 — Distinguish unsupported wide operations from malformed Uint bounds

Date: 2026-10-08
Status: accepted for focused implementation
Parents: R03002/#346 developer ergonomics, R03013/#357 audit

### Problem

Independent review F3 at4c8aebc3 found that wide subtraction/multiplication refusals use InvalidUnsignedMaximum even when the maximum is valid. Source review finds the same diagnostic misuse for ordered wide comparison, and the existing maximum error says canonicalu128 although Uint bounds up to248bits are admitted. This confuses supported type representation with unsupported operations. It does not demonstrate accepted wrong code or expand language support.

### Before / after

Before: a valid Uint<2^200> operand refused during multiplication is described as an unsupported maximum with expectedcanonicalu128.
After: an additive RenderError::UnsupportedWideUnsignedOperation names the actual operation and valid bound; genuinely malformed/noncanonical/out-of-range bounds retain InvalidUnsignedMaximum with a correct supported-bound description. Type parsing/errorprecedence and all acceptance/refusal domains remain unchanged. Display is already documented as human text, not a stable protocol; enum is non_exhaustive.

### Emitter/runtime ownership

Shared value lowering owns arithmetic refusal; comparison lowering uses the same diagnostic distinction. Runtime, ABI, privateIR, packageversion, scalar semantics and proof policy remain unchanged. Use ordinaryRust enum/match, no new wrapper/macro. Add focused positive controls for supportedwideidentity/addition and exact negative classes/messages for unsupportedwideoperations; preserve malformedbound tests and source-location wrappers.

### Acceptance

Confirm actual current diagnostic with a bounded test; then replace only misclassified refusals. Focusedbackend tests and formatting/scopedClippy pass, independent review agrees generatedvalidoutput/admission unchanged. Retain original/final output, exactsource and signedDCOcommit in midnight. No new conformancecampaign or fullfixtures sweep for a diagnostic-only change.

### Shared stateful diagnostic scope

The same validwidebound misuse exists in stateful/expression.rs left/right orderedcomparison guards. Include these two refusals while preserving their distinct validation order and existing source wrappers. No wider language support is introduced.


### Local implementation delivered — 2026-10-08

Signed conventional GPG/DCO commit `6aa345fd`; combined source at `e68078846260e4c6b2d18f40ec9ec1a2571cae33`. Root and independent sibling source review found no actionable issues in this change. Focused test and strict Clippy receipts are retained in [ADR0352 — Implementation receipt](references-0.3.0.md#note-131). Shared runtime batch:76 tests; backend diagnostic cohort:14 focused tests. Counts are shared/attributed in each report, not additive test coverage. No version, ledger, IR or ABI change.

Evidence: [ADR0352 — Wide operation diagnostics.zip](references-0.3.0.md#note-132), SHA256 `6be418f580ca9168809fca33ee329c70b24a6740ca2d378b0d318abd5d29c563`; 26 archive members verified byte-for-byte. Source at commit matches tested/reviewed files. Independent external retest and changed-line instrumentation are underway on the combined source. Parent coverage/assurance/audit acceptance and final-candidate qualification remain open; decoded heap/CPU scope requires the pending owner decision.


### Independent retest and instrumentation complete

External read-only coding-agent retest at e6807884 found no actionable defect in these three fixes. Fresh local instrumentation passed681 package tests,198 renders and21 existing generated behavior tests; changed-line backend95.11%, runtime97.81%, with no added exclusions. [Independent audit-fix retest at e6807884](references-0.3.0.md#note-152) and [Current coverage at e6807884](references-0.3.0.md#note-149) retain exact scope, commands, source identity and limits. The bounded implementation issue is closed; parent assurance/audit and final qualification remain open. No total decoded heap/CPU assurance is inferred, and path encoding boundaries do not imply semantic support for every manual path depth.
