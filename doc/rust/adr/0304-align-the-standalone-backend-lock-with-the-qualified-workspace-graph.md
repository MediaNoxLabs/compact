---
id: RUST-ADR-0304
alias: ADR-0304
source_sha256: 06c4b79d2751ae0185fd3939d62b4efb47813b288c19b01a39f5b459d9d151df
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0304 — Align the standalone backend lock with the qualified workspace graph

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted corrective release slice. Parents R030-16/#360 and R030-19/#363. Discovered during ADR0293 package/MSRV migration, before dependency changes. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0304 — Align the standalone backend lock with the qualified workspace graph

Status: accepted corrective release slice. Parents R030-16/#360 and R030-19/#363. Discovered during ADR0293 package/MSRV migration, before dependency changes.

### Problem

The tracked `tools/compact-rust-backend/Cargo.lock` is consumed by the isolated Nix backend build (`flake.nix` copies it). It has drifted from the qualified workspace graph. A standalone backend copy with that lock fails actual `cargo +1.88.0 check --offline --locked`: konst 0.4.3 and konst_proc_macros 0.4.1 require Rust 1.89. It also resolves midnight-circuits 6.3.0 and rand 0.8.8, versus workspace 6.0.0 and 0.8.6. Passing workspace-only MSRV checks cannot establish that this other distributed build path works. Preserve the failing command, log and lock as evidence; do not replace only a temporary consumer lock and call the tracked path fixed.

### Before / after

```text
Before:
workspace Cargo.lock -> approved graph -> Rust 1.88 passes
standalone/Nix Cargo.lock -> independent drift -> Rust 1.88 refuses

After:
workspace qualified lock -> exact standalone dependency closure
standalone/Nix --locked -> same retained package versions/sources/edges
                         -> actual Rust 1.88 check and focused backend tests
```

Synchronize the standalone backend dependency closure to the approved workspace graph, accounting explicitly for package-version changes in ADR0293. Permit pruning unreachable packages and local workspace identity differences only; retain exact registry/git package identities, checksums and dependency edges for the selected closure. Do not run an unconstrained update, independently advance Midnight primitives or hide the failure with a new MSRV. The package remains Rust 1.88.

### Ownership and validation

The tracked backend Cargo.lock is the production owner. ADR0293 owns coordinated backend/runtime/package versions; the workspace lock remains its authoritative dependency baseline. Do not alter emitter/runtime semantics. Capture both locks and verify the selected standalone graph against the workspace, including the previously divergent dependencies. Use actual Rust 1.88 locked/offline standalone compilation and focused backend tests to establish the repaired path. Keep external consumer and Nix claims separate: copying Nix's manifest/lock inputs into an isolated build validates those inputs, not an unexecuted full Nix derivation.

Add a bounded regression/check owner if existing release tooling can compare the two graphs without network or broad rebuilds. Document package-resolution fallback and consumer lock provenance precisely. The ordinary workspace dependency audit does not automatically audit a divergent standalone lock; the repaired selected closure needs matching advisory identity or a separate scan.

### Alternatives and consequences

Raising the advertised MSRV would unnecessarily break consumers while the approved graph already builds on 1.88. Ignoring the standalone lock would leave an actual packaging path broken. A fresh unconstrained resolution could introduce different unreviewed Midnight and registry packages. Synchronizing to the existing graph minimizes the remediation and makes future drift detectable.

Keep planning/history in midnight; publish ADRs at closeout. Local gates only, no registry publication, tags, push or CI. Parent completion still requires final-source compatibility and release qualification.

Issue: https://github.com/MediaNoxLabs/compact/issues/428


### Signed joined delivery — 2026-10-07

Release migration and standalone lock: `b6fcb06cec7e8913e91faae167926805c4872ffb`. Relation compiler, maintained reducers and public exporter: `329bf1bc80441125d2800fc9f1dae8b570da5dac`. Both commits are conventional, GPG verified and DCO signed.

The joined source passes 427 backend tests, 43 original DID test methods (including the 42-row relation table), three reducer methods, nine exporter tests, 99 Python checks, strict Clippy, whole-workspace formatting and all 196 generated-fixture freshness checks. Actual Rust 1.88 passes both workspace and freshly isolated standalone backend checks. The unchanged original DID source/import hashes remain pinned to v0.7.0.

The maintained relation gate passes 19 original calls and six reducer calls under default ledger strictness, with changed-binding rejection and replay refusal. All 512 recorded source hashes still match the signed delivery; the receipt records the pre-commit HEAD and explicitly binds unchanged source bytes to the final commits. Existing keys were verified and reused. Original relation TS outcomes match 23 successes and 19 refusals, including ordered programs, per-query/total gas, witnesses, state and ContractLab rollback.

The separate ledger 8.1 receiver passes eleven independent relation snapshots with full-ledger byte equality and unchanged replay refusal. JavaScript passes 66 carrier roundtrips and 132 malformed-input refusals. Eight original setup calls are retained separately. Earlier 14+2 public snapshots are historical evidence, not a fresh 27-row run. Native pins remain ledger 8.0.3; constructor data is deployed but constructor execution is not proved. No live network acceptance is claimed.

[ADR0293-0295-0303-0304 — Signed joined delivery.zip](references-0.3.0.md#note-076) contains 3081 verified entries; SHA256 `9062cfe7afcf8c15f77cb8674ea75cfcb0865a37898cdc31f45e0fb8c6b7c748`. Tool executables, parameter files and reused keys retain original paths/hashes and are not duplicated into this archive. It includes joined source, proof receipts/logs, public carriers/receiver results, release migration evidence and standalone-lock validation. Failed intermediate attempts remain alongside corrected passing runs.

These four child deliveries are complete. Parent evidence reconciliation, final coverage/performance, audit and release qualification remain separate obligations. Accepted parents remain 6/20 pending that reconciliation. No push, registry publication or remote CI. The user-owned ledger document remains byte-identical.
