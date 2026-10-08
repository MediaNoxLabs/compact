---
id: RUST-ADR-0321
alias: ADR-0321
source_sha256: 4fe8bd4be668812460d07a0bec1cb6a08a2d938c71b06825ed15c7c7d9ef9e90
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0321 — Reconcile compatibility policy and qualify the MSRV delta

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** locally accepted · 2026-10-07 · R030-16/#360 · milestone 0.3.0. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0321 — Reconcile compatibility policy and qualify the MSRV delta

Status: locally accepted · 2026-10-07 · R030-16/#360 · milestone 0.3.0

### Problem
Compatibility and migration evidence is distributed across ADR0293, ADR0304 and the independent ledger8.1 bridge. Runtime/macros/testkit sources have not changed since the signed accepted qualification, but backend typed Jubjub implementation has. The existing1.88 receipt cannot be called current-source qualification. A consolidated machine-readable source/feature policy and current upstream-ref check are also needed.

### Before / after
```text
Before: correct individual compatibility records and historical receipts
After:  one vault source/feature policy referencing immutable receipts
        + actual Rust1.88 backend/Jubjub and standalone checks
```
No API or version change: backend/runtime/macros0.2.0, testkit0.1.0, ABI50, IR20, capability3. Native ledger8.0.3 and isolated application/receiver8.1 remain distinct. Generated library package defaults are not milestone versions.

### Work and ownership
Reuse prior migrated/ABI49 refusal/ABI50 compatibility receipts after byte-identity review. Run actual1.88 locked/offline backend all-feature and representative Jubjub consumer checks, then isolated standalone backend with tracked lock, on the warmed MSRV target. Root freezes source and changes; delegate owns the MSRV target sequentially. Verify immutable DIDv0.7.0 and passport source baselines against remote refs read-only; keep changes on moving upstream heads as drift observations, not implicit scope expansion. Save approved feature/source policy to Obsidian; repository publication at milestone closeout.

### Acceptance boundaries
These checks do not qualify Linux, WASM1.88, fullNix, remote registry availability, network acceptance or a finalRC. Final frozen-candidate and upstream-drift recheck remain mandatory in R03020; current parent acceptance can only follow all present checks. The stopped ADR0285/#409 investigation remains untouched and unaccepted. No silent weakening or artificial tenth-parent completion. No source changes expected; ADR/issue/evidence delivery can be documentation-only in the vault.

Delivery issue: https://github.com/MediaNoxLabs/compact/issues/446


### 2026-10-07 — R03016 local compatibility acceptance (ADR0321/#446)

Current checkpoint `5efa91c280c059e375dcde0998c64dd88f615fb6` retains backend/runtime/macros0.2.0, testkit0.1.0, ABI50, IR20, capability3 and native ledger8.0.3. The machine-readable [Compatibility feature-source policy.json](references-0.3.0.md#note-144) joins source/feature/migration/support limits; milestone0.3.0 is not a package/ABI version.

Independent review verified78migration artifacts,35selected bridge artifacts and2vault archives. Runtime/macros/testkit/public compatibility source and registry package records remain unchanged; existing old/new ABI and migration receipts are reused within their original scope. Actual Rust1.88 on macOS ARM64 passes current backend plus new Jubjub fixture all-feature compilation and isolated standalone backend compilation at signed49898aeda (4.74s/3.77s, warm cached checks);805workspace and75standalone files bound, locks unchanged. Subsequent c924/5efa changes are two test files only. This is not Linux/WASM1.88/fullNix or whole-corpus qualification.

Read-only GitHub drift check `2026-10-07T08:20:54.167066+00:00` confirms DIDv0.7.0 and passport develop still match selected commits, all3pinned commits exist and19adopted source hashes match remote bytes. Passport still pins credential-compact0.2.0. Core main advanced39commits to0.3.0, an unselected newer dependency; retain the owner-approved passport closure without promotion. Eight initial path404 lookups were corrected and retained as lookup attempts. FinalRC must repeat source/ref drift and frozen-candidate checks.

R03016/#360 is locally accepted for present compatibility/migration/MSRV/source-policy criteria. Its future final-candidate obligations remain explicitly owned by R03020; none are called executed. ADR0285/#409 consumer safety remains stopped/unaccepted, and independent security audit remains open. No remote registry or network acceptance claim.

Archive [ADR0321 — Compatibility policy and current MSRV.zip](references-0.3.0.md#note-103), SHA256 `e87b8acab734cf37a11059a71cf74f33261b8cf04332d950534b9cf6d7c8284a`; policy SHA256 `c9491c30fab726cd03fe22389e6a2c12ea678230e079123481392f861d2646f9`. #446 bounded reconciliation closes. This is the **tenth accepted original parent**: original20=10accepted,1removedACC,9open; required19=10accepted/9open. Begin CI stabilization inventory under R03019; full frozen-revision remote qualification is later. Build-heavy local follow-ups wait for disk capacity (below2GiB); no caches/evidence removed.
