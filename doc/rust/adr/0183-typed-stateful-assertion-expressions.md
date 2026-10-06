---
id: RUST-ADR-0183
alias: ADR-0183
title: "Typed stateful assertion expressions"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-native-only"
topics: ["compiler", "assertion", "source-admission"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 401ead6b6fafa76636e188efcdce70fd5fe0d562d050cb19d563dc722ea315f7
---
# RUST-ADR-0183 — Typed stateful assertion expressions

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-native-only. Stateful assertion expression support allows complete unchanged Coracle and microDAO sources to compile natively. The resulting zero-unassessed inventory is source admission only; their recording, proofs and funded flows remained later work.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#286 closure](https://github.com/MediaNoxLabs/compact/issues/286#issuecomment-6017714895). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`17c627ab`](https://github.com/MediaNoxLabs/compact/commit/17c627ab738b87a03dfb01ed0b05755123caa664) · [`b16428c4`](https://github.com/MediaNoxLabs/compact/commit/b16428c44f5b35e1d7d65b7c1875f412fa20f02e) · [`b433acec`](https://github.com/MediaNoxLabs/compact/commit/b433acec08f29c0d3e4571d6c25c8d642334396d). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
status: accepted
date: 2026-10-05
```

## Historical decision and amendments

### Problem
Original micro-dao cash_out atline187 contains an Expr::Assert within a rootLet/Sequence. Its Boolean condition uses short-circuit Cell reads and CounterRead→CounterLessThan. Current stateful rendering delegates Assert to the pure renderer, which rejects those effects.

### Before / after
```compact
const addr = ownPublicKey();
assert(state == LedgerState.final && beneficiary.is_some && beneficiary.value == addr && no.lessThan(yes), "Attempted illegal cash-out.");
```
Before: stateful expression requires stateful evaluation. After: recursively evaluate the assertion condition using stateful lowering, preserve short-circuit branch order, require Boolean, reject false with the same assertion error, and return typed Unit.
```rust
let condition = /* ordered, branch-local stateful condition */;
if !condition { return Err(runtime::CompactError::AssertionFailed(message.into())); }
```

### Emitter/runtime
One native stateful Assert arm reuses shared typed expression lowering and existing assertion errors. Preserve witness/query flags and materialization order. No runtime API/ABI/schema change (ABI46/schema20); recorded assertion admission remains unchanged.

### Evidence plan and boundaries
Independent TS cases distinguish short-circuit query/witness branches, exact programs/gas/state/effects/private outputs and typed return. False assertions compare rejection and performed-query/witness order without claiming a recoverable Rust failed-call context. Reject nonBoolean condition and Unit/result mismatches. Recompile unchanged original micro-dao and retain next diagnostic. Local focused gates and signedDCO delivery; no remoteCI/push or new recording/proof claim.


### Research adjustment and measured behavior
Issue https://github.com/MediaNoxLabs/compact/issues/286. Witness-in-condition probe also required a Scheme stateful-expression-ir Assert arm; its old fallback misclassified the witness as a pure Call. The new arm retains the same IR shape/schema. New Scheme ${HISTORICAL_NIX_STORE}/zr15mfdx4akl4lzkq6kxw4kv1qc1v5hd-compactc-binary-nixos/bin/compactc-scheme. Twelve TS cases capture full query/witness event order and successful outputs; Rust matches witness/private prefixes and successful actual querysum. TS reported aggregate contains only the last query; this difference is retained, not normalized away. Failed Result cannot expose Rust partial gas/context, so those are not claimed. ZeroGas selectedfalse gives assertion before query; selectedtrue rejects the first read before witness2.

### Original source progress
Complete unchanged original micro-dao Rust Cargo check passes. Authoritative contract-info has11exports:4pure and7proofrequired; Rust capability report has7unavailable rows. Added exact positive sourcecohort with strict recordinggaps and full-local-gatehook. Native source admission does not imply full behavioral, funded or proof closure.


### Signed delivery
GPG+DCO commit f8ad873d4c5f4b3680ff055b9fd56ae4b3dc6e6f. Frozen receipt ${LOCAL_EVIDENCE}/compact-focused-f8ad873d-stateful-assert/receipt.json passed1fixture0/2recorded; both probe exports proofrequired/native-only. Twelve independent TS cases, backend8library+146renderer tests (existing recorded gates unchanged), targetedClippy, freshness, sourceguards179/181/183, and3gateharness tests pass. Original native crate Cargo check Rust1.99 passes; exact cohort receipt ${LOCAL_EVIDENCE}/compact-adr183-micro-dao-source-scope.json joins11exports=7proofrequired+4pure and all7recordinggaps. FrozenCLI ${LOCAL_EVIDENCE}/compact-adr183-compactc; Schemezr15mfdx4akl4lzkq6kxw4kv1qc1v5hd. ABI46/schema20 unchanged. Parent owns combined fixture refresh and Coracle integration with174. No push/remoteCI/fullbehavior/proof/fundingclaim.


### Combined integration — 17c627ab / b16428c4

Original microDAO and original Coracle now both emit Rust crates and pass Rust1.99 Cargo checking with combined174+178+183. Coracle9exports=5pure+4proof, all4recording gaps. microDAO11exports=4pure+7proof, all7recording gaps. Permanent source-cohort registration is in progress; do not count these as assessed in the older inventory until its registry update lands.

All165 fixtures fresh, one updated. Backend9units+12CLI+147renderer tests,27Python harness tests, strict backend Clippy and formatting pass. Six-source focused gate ${LOCAL_EVIDENCE}/compact-focused-77505018/receipt.json passes3/11recorded APIs in its selected cohort. New assertions are native-only. Logs ${LOCAL_EVIDENCE}/compact-integrated-adr183-source.log and ${LOCAL_EVIDENCE}/compact-integrated-adr183-coracle-cargo.log verify complete generated crates. ABI46/schema20 unchanged.


### Combined Coracle integration addendum (2026-10-05)

Combined ADR0174/0178/0183 admits the complete unchanged original Coracle source and its generated native crate. Validation commit `69aef3e39763e6541123a7fc51da2800cd40a7f2` is GPG+DCO signed; parent integrated it as `b433acec` with an explanatory commit body. No emitter/runtime/API/schema edits in this validation slice.

The exact positive cohort checks nine authoritative exports: five pure (`red_pk`, `is_red`, `blue_pk`, `is_blue`, `is_player_alive`) and four proof-required (`start`, `guess`, `concede`, `withdraw`). All four recording gaps remain exact: `start` has `unsupported_return / StateReturn::Effectful / return_value`; the other three have `unsupported_return / StateReturn::Expression / return_value`. Both `--effectful-return` and `--coracle-root-let` now require original-source native success, exact metadata/capabilities and strict recording refusal. The full local gate runs the Coracle cohort with exactly one offline Rust 1.99 Cargo check. Micro-dao and Coracle manifests are both registered in inventory. The full gate also checks `MIDNIGHT_LEDGER_TEST_STATIC_DIR` before expensive work; its absence produces the funded-proof fixture setup instruction.

Focused evidence passes: Coracle TS/Rust source acceptance and native Cargo check (`${LOCAL_EVIDENCE}/compact-adr183-coracle-source-scope.json`); registered micro-dao source cohort (`${LOCAL_EVIDENCE}/compact-adr183-micro-dao-registered-scope.json`); both source guards; 25 inventory and 4 gate tests; whitespace and GPG verification. No broad suite ran in this child worktree. Frozen compiler `${LOCAL_EVIDENCE}/compact-integrated-adr183-compactc`, Scheme `${HISTORICAL_NIX_STORE}/1d5x83y7dgg6abzv0fzvna893na3nfjj-compactc-scheme-local-slice/bin/compactc-scheme`; native Cargo uses existing `target/adr157` and `CARGO_INCREMENTAL=0`.

Exact postcommit inventory `${LOCAL_EVIDENCE}/compact-adr183-coracle-integrated-inventory.json`: 208 sources, 729 exports, 187 compiled sources, 334/360 proof-required APIs available, 26 explicit gaps, 369 nonproof exports, zero unassessed exports, zero missing/unmatched compiler proof rows. Baseline added/removed arrays are empty. This accounts for original-source assessment without claiming full original-contract behavioral, recorded proof or funded transaction closure.

Command/source/receipt hashes: `${LOCAL_EVIDENCE}/compact-adr183-coracle-delivery-receipt.json` (SHA-256 `4d463fff3d2b698bc657a1094b90d6cc83a551682f879d696141a8d44c7a10e8`). Existing issue [#286](https://github.com/MediaNoxLabs/compact/issues/286). No push or remote CI.
