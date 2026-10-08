---
id: RUST-ADR-0324
alias: ADR-0324
source_sha256: a809e1047fdea86e9d647ab624ef8cd057f0acb07c42c936b70cde1de83b58a7
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0324 — Consolidate current developer guides without rewriting history

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** drafts saved; validation pending · 2026-10-07 · R030-08/#352 · milestone 0.3.0. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0324 — Consolidate current developer guides without rewriting history

Status: drafts saved; validation pending · 2026-10-07 · R030-08/#352 · milestone 0.3.0

### Problem and decision
Current developer notes coexist with historical ABI49/package0.1/ACC and pending-adoption text. Engineers need a current entry point, complete witnessed ContractLab and DID examples, current version/target and troubleshooting tables. Keep historical ADRs/logs immutable; explicitly supersede obsolete instructions in current guides. Use normal Rust and existing generated APIs, not fictional sugar.

### Before / after
```text
Before: useful but fragmented probes and historical snippets
After: current guide index -> compile/run instructions -> API/domain explanation
       -> complete witnessed ContractLab and DID scenarios -> troubleshooting
```
Planning and guides remain in midnight until approved milestone closeout, then publish doc/rust. No new runtime/emitter/API changes are expected. Copy examples from actual existing independently tested sources; label adaptations as uncompiled until verified. Bind examples to receipts, exact required features, versions and source paths. Preserve standalone-vs-workspace limitations. Do not convert illustrative snippets into runnable claims.

### Delivery
Delegate drafts in scratch, root reviews and saves through Obsidian CLI. With low local disk, avoid new build-heavy consumers until space is available. Existing receipt reuse requires source identity; any adapted runnable example needs a bounded compile before acceptance. Parent #352 retains repository publication and example verification gates; no early closure. Stop boundaries for ADR0285 remain unchanged.

Delivery issue: https://github.com/MediaNoxLabs/compact/issues/449


### 2026-10-07 — Current-edition developer drafts preserved (ADR0324/#449)

Seven current-edition guides now live at [Current Rust developer guide draft](references-0.3.0.md#note-156). They cover versions/targets, witnessed ContractLab, original DID, proof stages, troubleshooting and an explicit snippet matrix. Complete witnessed/DID external adaptations and SDK preparation script are **uncompiled/unexecuted**; exact source/reference hashes and latest accepted compatibility policy are included. Small285KB payload; no dependency/build trees copied. Historical guides/ADRs are preserved.

Root inspected APIs, claim boundaries, source/reference manifest and pending-validation labels. New examples still require qualified locks, unique runtime/path isolation, generation, focused external tests, actual rustdoc/negative-example mapping and independent final guide review. #449 and parent#352 remain open; closeout repository publication is still required. No passing/runnable or production-ready claim follows from these drafts.

Archive [ADR0324 — Current developer guide drafts.zip](references-0.3.0.md#note-106), SHA256 `e5715a5111f0632e41d557b7d89911a6ab2fa50a4a34ebbe522716cc1ed94072`. No build/test/proof run, runtime/emitter/API changes, push or remote dispatch in this documentation slice.


### Current developer-guide navigation corrected — 2026-10-07

Under existing ADR0324/#449 and ADR0331, the current-edition troubleshooting/index now explain the conditional borrowed recording handle conversion when a source circuit owns `recording`. VALIDATION/index link to the actual durable archive; log paths are explicitly archive members. Refreshed the mutable guide manifest, including the earlier versions-page prose correction. All guide-manifest entries verify. Historical receipts and executable examples remain unchanged; no unnecessary tutorial rerun. [R03008 — Developer documentation closeout readiness](references-0.3.0.md#note-161) records current findings and publication-only work. R03008/#352 and #449 stay open until publication/final qualification.

### Current README compatibility labels corrected — 2026-10-08

Signed GPG/DCO [74d04c1a](https://github.com/MediaNoxLabs/compact/commit/74d04c1a8ed761263baac1679b05010f64b2fc73) corrects exactly two current ABI labels from49 to50 in tools/compact-rust-backend/README.md. Verified against runtime-rs/src/lib.rs and compatibility.json; historical ABI descriptions remain unchanged. Owned diff check passes; no tests warranted for two prose substitutions. Existing ADR0324/#449/#352 owns this correction; guide publication and final candidate qualification remain open.

The read-only current documentation review confirms25 current-edition manifest entries and477 SDK identities, with only the already qualified WitnessScript rustdoc delta. No new tutorial execution is claimed. Remaining work: durable publication links/example packaging, testkit README and final candidate source/evidence qualification.
