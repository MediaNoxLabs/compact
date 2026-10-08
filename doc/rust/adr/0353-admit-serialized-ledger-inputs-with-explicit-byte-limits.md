---
id: RUST-ADR-0353
alias: ADR-0353
source_sha256: 869fd4adcba3fe0bf0a15d07fd00b5808805db1b1ea32f320e27dc56c27e1764
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0353 — Admit serialized ledger inputs with explicit byte limits

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted for bounded implementation; total upstream heap/work assurance remains open. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0353 — Admit serialized ledger inputs with explicit byte limits

Date: 2026-10-08
Status: accepted for bounded implementation; total upstream heap/work assurance remains open
Parents: R03017/#361, R03013/#357, R03007/#351

### Problem

ObservedContractState::decode and decode_verifier_key delegate rawbytes to pinned ledger8 tagged_deserialize. They reject trailinginput, but expose no caller-selected encoded-size admission and lack direct owned malformed-tag/truncation corpus. Upstream recursion limits and32MiB initialVector capacity guard are real; they do not supply a total decoded allocation/work budget. No panic or denial-of-service was reproduced for these APIs, and we must not imply otherwise.

### Before / after

Before:
```rust
let state = ObservedContractState::decode(address, bytes, observation)?;
```
After (additive preferred boundary):
```rust
let limit = EncodedSizeLimit::new(max_indexer_response_bytes);
let state = ObservedContractState::decode_with_limit(address, bytes, observation, limit)?;
let key = decode_verifier_key_with_limit(key_bytes, limit)?;
```
The limit is selected by the integration, not invented from current fixtures. Zero is a valid deny-all byte policy. Check input length before upstream decoding and return io::ErrorKind::InvalidData on excess; disclose only length/budget, not payload. Exact limit admitted to normaldecoder; invalid content remains invalid. Existing decode APIs preserve signature and behavior for compatibility, with explicit inherited-limit documentation and pointers to limited counterparts. No silent retry from a limited refusal to legacyunlimited mode.

### Component ownership

Extract small typed wire-admission/decoder module under transaction/decoding.rs, reexporting existing verifierhelper and new limit/helper without changing callers. Keep upstream exact tagged schema and error handling; do not reimplement ledger codecs, alter pins, add globalalloc hooks or catchpanics. The bound owns encoded byte admission only. No change to emittedcode, ABI50, IR20 or packageversion is needed for additive handwritten consumer APIs.

### Tests and evidence

Use small real serialized ContractState and deterministic seededVerifierKey values. Cover validroundtrip/re-encoding, malformedtag, truncation, trailingbytes, emptyinput, zero/one-under/exact/over limit for both methods. Verify oversizedinput is refused before parsing (e.g. invalid-tag oversized input yields size-specific error). Testlegacyhelpers retain acceptance and refusal behavior. Do not run hugeallocation or stresspayloads. Focusedtransaction tests and formatting/Clippy; independentreview validates nofallback and precise resourceclaims.

### Explicit unresolved resource assumption

This change does NOT establish total upstream decoded heap, objectcount or CPU limits; a Read wrapper cannot intercept allocations before a read. Such a guarantee needs upstream budget support or process containment with its own portability/design work. Record the limitation in the source-first boundary crosswalk and independentaudit. Do not close#361 by silently turning this byteguard into a universal memory/work guarantee. The additional acceptance decision remains to be resolved from audit evidence; no scopechange is implied here.


### Local implementation delivered — 2026-10-08

Signed conventional GPG/DCO commit `16f5b55d`; combined source at `e68078846260e4c6b2d18f40ec9ec1a2571cae33`. Root and independent sibling source review found no actionable issues in this change. Focused test and strict Clippy receipts are retained in [ADR0353 — Implementation receipt](references-0.3.0.md#note-134). Shared runtime batch:76 tests; backend diagnostic cohort:14 focused tests. Counts are shared/attributed in each report, not additive test coverage. No version, ledger, IR or ABI change.

Evidence: [ADR0353 — Encoded byte admission.zip](references-0.3.0.md#note-133), SHA256 `642e7a78deaff10a8a6e52d287bb6d7ba835d2d5f4c659751c670b2937f1c877`; 6 archive members verified byte-for-byte. Source at commit matches tested/reviewed files. Independent external retest and changed-line instrumentation are underway on the combined source. Parent coverage/assurance/audit acceptance and final-candidate qualification remain open; decoded heap/CPU scope requires the pending owner decision.


### Independent retest and instrumentation complete

External read-only coding-agent retest at e6807884 found no actionable defect in these three fixes. Fresh local instrumentation passed681 package tests,198 renders and21 existing generated behavior tests; changed-line backend95.11%, runtime97.81%, with no added exclusions. [Independent audit-fix retest at e6807884](references-0.3.0.md#note-152) and [Current coverage at e6807884](references-0.3.0.md#note-149) retain exact scope, commands, source identity and limits. The bounded implementation issue is closed; parent assurance/audit and final qualification remain open. No total decoded heap/CPU assurance is inferred, and path encoding boundaries do not imply semantic support for every manual path depth.
