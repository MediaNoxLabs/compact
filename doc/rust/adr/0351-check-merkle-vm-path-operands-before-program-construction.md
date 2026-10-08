---
id: RUST-ADR-0351
alias: ADR-0351
source_sha256: b343357811c704cdddf7d782b9dfe5c628ad1f351b0bb56c1180a8d9872a8cbb
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0351 — Check Merkle VM path operands before program construction

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted for bounded implementation; not yet delivered. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0351 — Check Merkle VM path operands before program construction

Date: 2026-10-08
Status: accepted for bounded implementation; not yet delivered
Parents: R03017/#361, R03007/#351, R03013/#357

### Problem

The runtime trust-boundary inventory found that public plain/historic reset functions accept Into<LedgerPath>, whose slice conversion permits an empty path. Their shared program builders call split_last().expect, so malformed input panics before the Result-returning API can reject it. Other Merkle program builders narrow path length with as u8 and add VM bookkeeping offsets; overwidth can truncate or overflow. This is a source-confirmed boundary defect; no allocation-stress or deployed-system experiment is required.

### Before / after

Before (illustrative current shape):
```rust
let (field, parent) = path.as_slice().split_last().expect("ledger path contains a field index");
let n = path.as_slice().len() as u8 + 2;
```
After (intended shape):
```rust
let (field, parent) = checked_reset_path(&path)?;
let n = checked_path_operand(&path, 2)?;
```
Actual helper names remain implementation choices. Derive accepted path lengths from the u8 VM operand plus each operation's exact bookkeeping offset, not an arbitrary global cap. Empty paths remain legal where the original non-reset program deliberately addresses the root; reset needs a field index. Preserve valid emitted opcode sequences.

### Ownership and compatibility

The shared Merkle program owner validates before constructing operations. Internal builders become fallible; public ledger APIs retain their existing Result<QueryResults,TranscriptRejected> signatures using upstream InvalidArgs or BoundsExceeded as appropriate. Context and RecordingFrame propagate the same rejection as existing CompactError::LedgerQueryRejected. Do not add panic-catching, silently skip a mutation, truncate paths or convert failure to an empty program. No new runtime API/error variant, ledger pin, IR or compiler output change is intended.

### Verification

First reproduce empty reset and overwidth operand handling using small literal paths; require intended errors rather than merely no panic. Cover public native/context/recorded routes as applicable, exact inclusive operand boundaries and first-invalid values, valid plain/historic single/nested reset behavior and unchanged recording/replay. Run affected runtime tests, formatting and scoped Clippy; broaden only if a concrete integration failure demands it. Independently review the fix. Record signed/DCO commit, original failure and final receipts in midnight before accepting the child issue.

### Limits

This closes a concrete path-validation gap, not every untrusted-input assurance criterion. Raw serialized-state/verifier decoding resource policy remains separately under investigation. Caller-defined CellValue/traits and callbacks retain documented trusted-Rust assumptions. External audit is frozen at4c8aebc3; this subsequent fix needs targeted independent retest at its own exact source.

### Scope extension from independent review — 2026-10-08

The independent source review at4c8aebc3 confirms the same input class in collection/list and cell helpers: list_reset_program has an empty-path expect, collection builders narrow length with asu8/+1, and cell path duplication narrows depth. Expand this decision and#479 to shared **ledger VM path operand validation**, covering merkle/collections/cell native/context/recorded routes. Preserve public Result signatures and legitimate root-addressing semantics; internal fallible builders/shared checks are preferred over a breaking LedgerPath From removal. Exact operation-specific stack/index offsets define accepted bounds. New tests must cover the reproduced public list reset and first-invalid byte operands as well as Merkle. This is one discovered boundary defect class, not a broad runtime rewrite.

### Pinned VM representation correction — 2026-10-08

Exact registry midnight-onchain-vm3.0.0 source (ops.rs448–461) encodes Ins operands as0x90|n or0xa0|n, reserving4bits, and Idx uses opcode|(path.len−1), admitting1..16keys (emptyIdx omitted). The initial u8-storage-width assumption is insufficient for canonical proof representation. Validate **Ins<=15, Idx<=16**, with each builder's actual offset and empty-path semantics, plus checkedarithmetic. Native oversizedpaths that cannot produce an unambiguous canonical program are rejected; do not claim all formerly native-executable paths are legitimate proof-compatible inputs. Public Rust signatures and validgeneratedoutputs remain unchanged.

Do not cite newer ledger-checkout invariant guards as evidence for the pinned3.0.0 implementation: that version lacks those guards. Root verified the encoding directly. Existing qualifiedcoin-cell Dupdepth<=15 checks already establish its casts; no duplicate arbitrary cap is needed there. Counterbuilders share the rawpathoperandissue and are included. Ordinarycell/query paths require checkedIdx representability where assembled.


### Local implementation delivered — 2026-10-08

Signed conventional GPG/DCO commit `e6807884`; combined source at `e68078846260e4c6b2d18f40ec9ec1a2571cae33`. Root and independent sibling source review found no actionable issues in this change. Focused test and strict Clippy receipts are retained in [ADR0351 — Implementation receipt](references-0.3.0.md#note-129). Shared runtime batch:76 tests; backend diagnostic cohort:14 focused tests. Counts are shared/attributed in each report, not additive test coverage. No version, ledger, IR or ABI change.

Evidence: [ADR0351 — Ledger VM path validation.zip](references-0.3.0.md#note-130), SHA256 `f6d7e57939afb192d8d4797cab262717a6c4d2160818f0c479734071870a6af6`; 26 archive members verified byte-for-byte. Source at commit matches tested/reviewed files. Independent external retest and changed-line instrumentation are underway on the combined source. Parent coverage/assurance/audit acceptance and final-candidate qualification remain open; decoded heap/CPU scope requires the pending owner decision.


### Independent retest and instrumentation complete

External read-only coding-agent retest at e6807884 found no actionable defect in these three fixes. Fresh local instrumentation passed681 package tests,198 renders and21 existing generated behavior tests; changed-line backend95.11%, runtime97.81%, with no added exclusions. [Independent audit-fix retest at e6807884](references-0.3.0.md#note-152) and [Current coverage at e6807884](references-0.3.0.md#note-149) retain exact scope, commands, source identity and limits. The bounded implementation issue is closed; parent assurance/audit and final qualification remain open. No total decoded heap/CPU assurance is inferred, and path encoding boundaries do not imply semantic support for every manual path depth.
