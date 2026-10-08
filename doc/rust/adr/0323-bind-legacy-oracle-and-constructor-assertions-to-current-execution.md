---
id: RUST-ADR-0323
alias: ADR-0323
source_sha256: 16948d94ca603938a91c98b21a7785484e26a901c86bfa4cf1932caddc795c47
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0323 — Bind legacy oracle and constructor assertions to current execution

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** delivered bounded evidence slice · 2026-10-07 · R030-07/#351 · milestone 0.3.0. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0323 — Bind legacy oracle and constructor assertions to current execution

Status: delivered bounded evidence slice · 2026-10-07 · R030-07/#351 · milestone 0.3.0

### Problem
122 inherited oracle rows lacked normalized case/assertion joins. Read-only review found120 exports and2constructor-only sources,125assertion joins,191existing JSON-pointer capture locators and53named test owners. These are locators, not invented producer case IDs. All36generated libraries changed since the retained execution receipt; historical test identity cannot establish current library behavior.13reducer constructor mappings also need consolidation; digest-read has native constructor execution but lacks exact TS-derived prestate comparison.

### Before / after
```text
Before: export -> historical reviewed assertion, current generated library unjoined
After:  source/export -> hash-bound assertion + capture locator -> named fresh test
```
Keep constructor state derivation distinct from proving constructor execution. Preserve setHash's composed TS scenario versus isolated native/replay case.

### Decision and ownership
Run only affected36packages/38integration targets (100declared test methods), explicit package/test selections and named-test checks, locked/offline Rust1.99 on warmed parity target. No proving, regeneration or whole-workspace rerun for this evidence refresh. Freeze sources/locks/generated/test/capture identities before and after. Root reviews emitted receipt and existing source-generation evidence separately. Add a small digest constructor/prestate test only if exact fixture constructor/capture inputs make the comparison valid; do not alter captures to fit Rust.

### Validation and limits
Machine joins, existing file hashes and local executions are separate records. Missing/filtered tests fail the runner. Source changes during run fail qualification. Archive prior reports and refreshed logs in vault; parent #351 still needs line floors and finite semantic obligation mapping. All new source edits use conventional signedDCO commits. Keep stopped ADR0285 lane untouched; no remoteCI/push from this slice.

Delivery issue: https://github.com/MediaNoxLabs/compact/issues/448


### 2026-10-07 — Oracle and constructor evidence joined (ADR0323/#448)

Normalized122inherited rows (120exports+2constructor-only):125assertion joins,191existing JSON-pointer locators and53named test owners. Locators are not new case IDs. Historical279hash checks preserved. Current execution at signed49898aeda passes **100tests /38integration targets /36packages**, all53required names exactly once;614relevant source/test/capture/runtime/lock identities unchanged. No source regeneration or proofs rerun. Preserve setHash's composed TS scenario versus isolated native/replay case.

All13reducer native constructors now have explicit mappings. Twelve suites supply77TS-prestate comparisons. Signed GPG/DCO `5efa91c280c059e375dcde0998c64dd88f615fb6` adds the remaining digest constructor equality test: **one constructor scenario checked against four captured prestates**, exact ledger data, active flag, full Method, size and private state. Three digest targets (constructor/behavior/provenance), strict new-target Clippy and formatting pass; independent read-only review found no material findings. Existing capture and reviewed behavior hashes unchanged. Constructor state comparison/deployment does not prove constructor execution.

Archive [ADR0323 — Oracle and constructor execution joins.zip](references-0.3.0.md#note-105), SHA256 `02e3fe3a1cc58f278a56a241e7c65ddb135e49a6031c3c4f4f25cd94d08bc280`. The four direct control gaps were separately resolved by ADR0319. Thus previously listed122oracle/13constructor mapping gaps are joined, with current selected execution refreshed. #448 closes this bounded work; #351 remains open for finite semantic/effect obligations and release-fixture source/regeneration reconciliation. Parent acceptance unchanged9/19. No remoteCI/push; user doc preserved.
