---
id: RUST-ADR-0270
alias: ADR-0270
source_sha256: f17aad91ea04fde35336c04051ba0abc8059c99bb2e090a81ebb8aaf2184f6f1
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0270 — Validate every declared Midnight dependency context

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted for implementation. Parents R030-16/#360 and R030-13/#357. External review F1/F4, independently checked by the root agent. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0270 — Validate every declared Midnight dependency context

Status: accepted for implementation. Parents R030-16/#360 and R030-13/#357. External review F1/F4, independently checked by the root agent.

### Problem and reproduction

The compatibility preflight validates only the runtime manifest's ordinary dependency table. Conditional dependencies and aliases can introduce Midnight package declarations outside that scan. With the frozen ADR0265 compiler, a selected runtime root containing a Unix-only same-version VM path override successfully generates a crate and compatibility record. An aliased Unix-only ledger9 declaration also passes generation. No dependency was resolved or executed in those probes; the external review's stronger claim that a different VM necessarily links is not established. Cargo may reject some inconsistent source combinations later. The confirmed problem is incomplete early mismatch detection.

Two further public CLI probes confirm that a missing runtime root and `--rust-runtime-root ""` print only `No such file or directory (os error 2)`. The equals spelling already rejects an empty value explicitly. This obscures the user-controlled path that needs correction.

### Before and after

Before (unexpectedly accepted by preflight):

```toml
[target.'cfg(unix)'.dependencies]
extra-ledger = { package = "midnight-ledger", version = "=9.0.0" }
```

After: a contextual typed validation failure names the runtime package, conditional dependency table, alias/resolved package and mismatched version or unsupported source override, before frontend invocation or output replacement.

Before: `compactc: No such file or directory (os error 2)`. After: the error identifies the selected runtime root or manifest/source file. Both empty-value flag spellings say that a directory is required.

### Decision and ownership

Extend compatibility.rs to inspect ordinary, dev and build dependency tables and all target-specific variants for both runtime and macro packages. Resolve the dependency's `package` field before classifying Midnight packages; a harmless-looking alias must not skip validation. Preserve required ordinary dependencies and exact reviewed versions/source rules. Known matching dev declarations already exist and must remain valid; do not follow the reviewer's blanket suggestion to ban every dev Midnight dependency. The macros package currently has no Midnight dependency and may reject unreviewed additions.

Use one small dependency-entry checker with an explicit manifest/table path in diagnostics. Keep ordinary valid TOML strings, inline tables and regular tables supported. Reject unreviewed Midnight source substitutions, aliases or contexts according to an explicit tested policy; do not execute Cargo, build scripts or dependencies to inspect configuration. Keep macro sibling-path handling limited to its intended normal dependency.

Add contextual read/parse/canonicalization errors in the preflight and selected runtime source copy path where the failing filename would otherwise disappear. Align both runtime-root flag spellings. No cryptographic authentication claim: this remains a developer mismatch check on selected source, not trust in arbitrary edited Rust or a full transitive dependency audit. No compatibility-record schema, ABI, ledger pin or package version change is planned.

### Acceptance

Regression tests cover the two observed public preflight omissions, harmless aliases that resolve to Midnight packages, normal/dev/build/target tables, macro-manifest additions, known matching dev controls, TOML forms and empty/missing roots. Each refusal identifies its manifest/table/entry, does not execute a frontend/build script, and leaves prior output intact. Existing consumer and compatibility tests remain passing. A frozen compiler reproduces the original probes as early failures after the fix. Strict focused Clippy and generated-source equality pass. Preserve the external report, before/after receipts and an independent external retest.

Issue: https://github.com/MediaNoxLabs/compact/issues/394

### 2026-10-07 — Delivered locally

`91bf9a1521321214bf5bdc1bf751dcecb5369a18`, conventional, good GPG signature and DCO. All ordinary/dev/build and target-specific dependency contexts in both runtime packages are inspected. Package aliases are resolved before Midnight classification, then unreviewed aliases/source overrides are refused. Matching reviewed ledger pins remain accepted in runtime dev/build/target contexts, including the actual matching dev-dependency. Required ordinary dependencies remain exact; the macro package cannot introduce a new Midnight graph. Root/source/parse/copy failures identify their selected path; both empty runtime-root flag spellings agree.

34 focused compiler tests and strict all-target/all-feature Clippy pass. Five public early-refusal reproductions leave frontend marker absent and old output unchanged. Six paired outputs (three contracts, bundled/shared runtime) preserve generated lib/Cargo/capability/compatibility bytes. Both bundled and shared external consumers compile offline with all features. This closes external F1/F4 at the local component level; independent re-review is running against the committed source.

This preflight detects developer misconfiguration; it does not authenticate arbitrary source or resolve every transitive Cargo dependency. The original public reproduction showed generation was accepted, not that a mismatched dependency graph linked or executed. No package/ABI/schema/pin change and no full corpus rerun.

[ADR0270 — delivery-receipt.json](references-0.3.0.md#note-041).
