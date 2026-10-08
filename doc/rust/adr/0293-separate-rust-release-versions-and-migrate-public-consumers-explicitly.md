---
id: RUST-ADR-0293
alias: ADR-0293
source_sha256: e40e2c9bec4f50f3b3ffe866c2c2a47fbb474e69a57cbca0468ad42ba37c9d8f
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0293 — Separate Rust release versions and migrate public consumers explicitly

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** delivered-locally. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

```yaml
status: delivered-locally
date: 2026-10-07
parent: R030-16
milestone: "0.3.0"
issue: https://github.com/MediaNoxLabs/compact/issues/416
```

## ADR0293 — Separate Rust release versions and migrate public consumers explicitly

### Problem

The milestone0.3.0 label, compiler/language/TS runtime labels, Cargo packages, runtime ABI and private/public schemas identify different contracts. Current unpublished Rust0.1.0 snapshots already span incompatible generated/runtime pairings. Runtime ABI50 does not describe backend library source compatibility: ADR0291's planned new public RenderError variant breaks exhaustive downstream matches even when generated/runtime ABI remains unchanged. A release mapping and ordinary consumer migration evidence must precede publishing a new compatibility promise.

[R03016 — Current compatibility evidence and release mapping — 2026-10-07](references-0.3.0.md#note-163) consolidates existing metadata/preflight, actual Rust1.88, old/new ABI50 consumer, original DID release and8.1 public interchange evidence. This ADR accepts its mapping; it does not rerun or broaden those historical receipts.

### Decision

Root accepted this mapping on2026-10-07. Implement only after ADR0291 resource API and ADR0292 frame fix are finalized and root releases their source ownership.

| Boundary | Before | Accepted release value |
|---|---|---|
| Fork artifact / initiative | milestone0.3.0, no candidate tag | `rust-backend-v0.3.0` artifact label; no tag/upload now |
| Backend Cargo library and CLI package |0.1.0|**0.2.0** |
| Runtime / macros exact source pair |0.1.0 /0.1.0|**0.2.0 /0.2.0**, runtime pins macros exactly |
| ContractLab testkit |0.1.0, publish=false|Keep0.1.0, publish=false; exact accepted runtime dependency |
| Generated application crate default |0.1.0, publish=false|Keep; application owner controls its own release version |
| Installer workspace |0.5.1|Keep |
| Compiler / language / TS runtime |0.31.133 /0.23.105 /0.16.101|Keep |
| Generated/runtime ABI |50|Keep50 |
| Private IR / public capability report / compatibility record |20 /3 /1|Keep20 /3 /1 |
| Native MSRV / primary toolchain |1.88 /1.99|Keep, within recorded host/lock/feature scope |

The runtime/macros change is an explicit fresh compatibility line for the ABI50 source-distributed pair; it does not assert that every added runtime API is a Rust source break. Historic0.1.0 archives retain their exact hashes and provenance. The backend's new line independently acknowledges the concrete exhaustive-enum break. No package availability or registry publication follows from these labels.

Mark public **RenderError `#[non_exhaustive]` in the same0.2.0 breaking line** as ADR0291's accepted resource error. Both the new variant and adding non_exhaustive affect old exhaustive callers; acknowledge them together. Keep the precise resource error chosen by ADR0291. Do not disguise resource refusal as an unrelated old semantic error to preserve a version label. Future diagnostic additions can then coexist with a downstream fallback arm.

### Before/after consumer examples

Before package pairing:

```toml
[dependencies]
compact-rust-backend = "=0.1.0"
midnight-compact-runtime = { version = "=0.1.0", path = "../runtime-rs" }
```

After accepted source distribution (paths are examples; registry availability is unverified):

```toml
[dependencies]
compact-rust-backend = { version = "=0.2.0", path = "../tools/compact-rust-backend" }
midnight-compact-runtime = { version = "=0.2.0", path = "../runtime-rs" }
```

A previous backend consumer could enumerate every RenderError variant without a fallback. Such a complete old match must be preserved as an actual migration fixture. The next consumer retains explicit error handling and includes a fallback:

```rust
use compact_rust_backend::RenderError;
fn explain(error: &RenderError) -> String {
    match error {
        RenderError::SchemaVersion(found) => format!("Unsupported IR schema: {found}"),
        // Add explicit handling for ADR0291's finalized resource variant here
        // when structured reporting is needed; no variant spelling is invented yet.
        _ => format!("{error}"),
    }
}
```

This small example is illustrative, not the exhaustive old fixture or test evidence. Final implementation must use the actual finalized API. Display text is a user diagnostic, not a stable machine-parse protocol.

ABI49 generated consumers either retain their matching historical bundle or regenerate with the accepted ABI50 compiler/runtime/macro pair. Editing the generated assertion to50 is not migration. Old ABI50 generated code remains the ordinary compatible consumer control. Package labels alone do not authenticate source; selected runtime metadata/source checks and artifact hashes remain required.

### Emitter/runtime/packaging impact

Coordinate backend/runtime/macros Cargo manifests, exact local version requirements (including testkit and maintained fixture manifests), affected lock package identities, Nix/backend packaging version references and the two authoritative compatibility records. Inventory hardcoded0.1.0 assertions/fixtures before changing them: generated application defaults and historical evidence must stay unchanged. Keep generated ABI50 assertions, IR20/capability3/compatibility1 semantics, selected-source preflight and source fingerprinting.

Runtime VM/crypto/state semantics and ledger/zk graph remain unchanged. No new runtime operation is required by this version decision. Produce a current feature/source policy and migration guide in the vault for final repository publication at milestone closure. Artifact metadata must show the fork label, backend package and source identities alongside unchanged compiler component version so users do not confuse two distributions with the same compiler query result.

No low-level API deprecation/removal without an identified replacement and ordinary consumer evidence. Retain positional generated APIs; ADR0276 named Args remain research with promotion deferred. Private module refactoring does not authorize public path removal.

### Focused migration acceptance

1. Preserve exact old library/source consumer fixture before0291/0293. Its complete exhaustive RenderError match compiles against old source; against the new public enum it fails for the expected exhaustiveness cause. A migrated consumer with fallback compiles and exercises a finite accepted resource refusal. No giant or resource-exhausting input.
2. Compatible old/new ABI50 generated consumer fixtures compile with the new validated runtime pair, across bundled and explicit shared source roots. Keep an incompatible ABI49/macro/version root refusal before frontend/output replacement. Generated source semantic bytes must not change solely due package labels.
3. Generated application default stays0.1.0; testkit stays0.1.0/publish=false; expected runtime/macros/backend versions and exact dependency pairing are0.2.0. Generated compatibility metadata and package rehearsal reflect actual approved manifests without stale literals.
4. Rehearse runtime/macro archives locally with explicit local dependency patch evidence. Registry emission can be checked as a manifest option; do not claim a remote registry consumer or broaden package publication scope. No tag/upload/push under this ADR.
5. Requalify the bounded final-source delta on actual native Rust1.88 and primary1.99 with representative external generated/testkit consumer. Reuse the existing measured306-test checkpoint as historical evidence; do not claim it ran against changed source. Delegate integrated broader milestone gates to root instead of duplicating them here.
6. Retain unchanged compiler/language/TS labels, ABI50/IR20/caps3/compat1 and exact ledger8.0.3 native graph. Final source policy separately labels original compiler ledger8.0.2, original DID application wire8.1 and isolated8.1 receiver. `midnight-ledger-static`9.0.0 is a valid support dependency, not ledger9 consensus promotion.

### Rationale / alternatives

Cargo treats adding a variant to an exhaustive public enum and adding non_exhaustive itself as breaking; pre1.0 compatibility lines follow the first nonzero component. [Cargo SemVer reference](https://doc.rust-lang.org/cargo/reference/semver.html). Keeping the backend0.1 line while claiming that unchanged runtime ABI prevents downstream breakage would conflate two interfaces. Synchronizing every component to0.3.0 would similarly conceal distinct language and package histories.

Registry publication, new native8.1 profile, CI stabilization and broad architecture changes are separate decisions. This slice is accepted policy with focused migration implementation held behind0291/0292. Root owns integration, signed commits and closure.


### Signed joined delivery — 2026-10-07

Release migration and standalone lock: `b6fcb06cec7e8913e91faae167926805c4872ffb`. Relation compiler, maintained reducers and public exporter: `329bf1bc80441125d2800fc9f1dae8b570da5dac`. Both commits are conventional, GPG verified and DCO signed.

The joined source passes 427 backend tests, 43 original DID test methods (including the 42-row relation table), three reducer methods, nine exporter tests, 99 Python checks, strict Clippy, whole-workspace formatting and all 196 generated-fixture freshness checks. Actual Rust 1.88 passes both workspace and freshly isolated standalone backend checks. The unchanged original DID source/import hashes remain pinned to v0.7.0.

The maintained relation gate passes 19 original calls and six reducer calls under default ledger strictness, with changed-binding rejection and replay refusal. All 512 recorded source hashes still match the signed delivery; the receipt records the pre-commit HEAD and explicitly binds unchanged source bytes to the final commits. Existing keys were verified and reused. Original relation TS outcomes match 23 successes and 19 refusals, including ordered programs, per-query/total gas, witnesses, state and ContractLab rollback.

The separate ledger 8.1 receiver passes eleven independent relation snapshots with full-ledger byte equality and unchanged replay refusal. JavaScript passes 66 carrier roundtrips and 132 malformed-input refusals. Eight original setup calls are retained separately. Earlier 14+2 public snapshots are historical evidence, not a fresh 27-row run. Native pins remain ledger 8.0.3; constructor data is deployed but constructor execution is not proved. No live network acceptance is claimed.

[ADR0293-0295-0303-0304 — Signed joined delivery.zip](references-0.3.0.md#note-076) contains 3081 verified entries; SHA256 `9062cfe7afcf8c15f77cb8674ea75cfcb0865a37898cdc31f45e0fb8c6b7c748`. Tool executables, parameter files and reused keys retain original paths/hashes and are not duplicated into this archive. It includes joined source, proof receipts/logs, public carriers/receiver results, release migration evidence and standalone-lock validation. Failed intermediate attempts remain alongside corrected passing runs.

These four child deliveries are complete. Parent evidence reconciliation, final coverage/performance, audit and release qualification remain separate obligations. Accepted parents remain 6/20 pending that reconciliation. No push, registry publication or remote CI. The user-owned ledger document remains byte-identical.
