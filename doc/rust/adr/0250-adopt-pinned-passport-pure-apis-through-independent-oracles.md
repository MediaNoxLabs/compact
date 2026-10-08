---
id: RUST-ADR-0250
alias: ADR-0250
source_sha256: 8f6752b30d3dcd19cb0668ccee3d8cdc8e124d59df8cf085d497051a2d87b910
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0250 — Adopt pinned passport pure APIs through independent oracles

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted-implementation-in-progress. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

```yaml
status: accepted-implementation-in-progress
date: 2026-10-07
milestone: "0.3.0"
issue: https://github.com/MediaNoxLabs/compact/issues/375
```

## ADR-0250 — Adopt pinned passport pure APIs through independent oracles

### Problem and baseline
The pinned passport family has 75 exported pure circuits and no stateful ledger API. Compilation alone does not establish adoption. Exact 17-file closure is pinned in targets-data. Published credential-compact0.2.0 tarball integrity and all ten core source modules now independently verified. Two TS compiler/runtime profiles agree on 17 selected cases across seven exports; direct Rust matches them. Sixty-eight other exports still require behavior coverage.

### Before / after
```rust
// Before: compile the generated crate; behavior remains untested.
// After: call the untouched generated function and compare independent capture.
let actual = pure_circuits::compute_commitment(input, opening)?;
assert_eq!(actual, expected_from_typescript);
```
Illustrative API; concrete signatures and capture programs belong to the fixture evidence.

### Decision and ownership
Preserve the unchanged seven passport and ten core Compact sources in an adoption fixture with exact origin/hash manifest, compiler-generated Rust, deterministic TS capture programs/results and direct Rust consumer tests. Runtime primitives remain ledger-owned; emitter remains unchanged unless a minimized failed source primitive proves a defect. Expand coverage by exported API and semantic family, including exact assertion errors, boundary inputs, changed-field sensitivity and actual upstream scenarios. Do not add a fake ledger simulator or proof requirement to pure-only functions.

### Compatibility and limitations
Upstream compactc0.31.1/runtime0.16.0 reports ledger8.0.2; this Rust line pins ledger8.0.3. Agreeing selected pure hashes/outputs does not prove general ledger version equivalence. Keep compiler/runtime/source identities with every oracle. The initial seven-export tranche is useful evidence, not closure of parent adoption. No generated source hand edits or weaker assertions are allowed.

### Verification and alternatives
Require raw upstream and branch TS captures, matching direct Rust cases, deterministic repeatability, fixture freshness and explicit per-export coverage gaps. Reject handwritten expected cryptographic constants without independent origin. If a primitive fails, isolate the smallest source example under R030-11, fix it with tests and rerun affected adoption cases. Broaden to complete API/scenario evidence before parent acceptance. Planning/reporting remains vault-first; source/test/provenance artifacts are implementation inputs.
