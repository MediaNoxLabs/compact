---
id: RUST-ADR-0119
alias: ADR-0119
title: "Attribute original Election and Zerocash source proof gaps"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["inventory", "original-contracts", "source-provenance"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 75cbfdb3590e40aa5090908a261f7ab4ac26838126ea3f6cb631c5421b786c6c
---
# RUST-ADR-0119 — Attribute original Election and Zerocash source proof gaps

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted exact original Election and Zerocash source attribution, making seven then-native-only proof gaps explicit without changing generated code. The Election oracle constructor differs from the original and supplies representative evidence only; this inventory decision itself provides no original-source proof gain.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#222 closure](https://github.com/MediaNoxLabs/compact/issues/222#issuecomment-6017607154). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`a6bfc04a`](https://github.com/MediaNoxLabs/compact/commit/a6bfc04af726fbf820958a27f731b6b984e7fd9d). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
initiative: compact-rust-backend
milestone: rust-backend-v2
adr: 0119
status: accepted
base: a6bfc04af726fbf820958a27f731b6b984e7fd9d
```

## Historical decision and amendments

### Problem

The exact original `examples/election.compact` and `examples/zerocash.compact` already compile to Rust but were not in the compiler-backed positive source inventory. Their seven exported circuits appeared as unassessed, hiding the actual distinction: all seven are proof-required and have native Rust entry points, while recorded/observed-call lowering remains unavailable. The existing `examples/rust_backend/*_oracle.compact` sources cannot substitute for original source identity. In particular, the Election oracle adds an authority constructor parameter absent from the original source.

### Decision

Add one explicit two-source positive cohort, using the original files and authoritative TypeScript `contract-info.json` proof flags. Pin all seven current recording gaps to typed `StateAction::Assert` or `StateAction::Let` paths. Register the cohort in the parity inventory and full local compiler gate. Compare the original Zerocash generated Rust byte-for-byte with its checked oracle fixture after formatting. Do not equate native entry points with recorded proof APIs.

### Before and after

Before, the inventory row for `examples/election.compact::vote$commit` was effectively:

```json
{"proof_required": null, "rust_recording_status": null}
```

After compiler-backed attribution it is:

```json
{"proof_required": true, "rust_recording_status": "unavailable",
 "rust_recorded": false, "rust_observed_call": false}
```

The generated Rust did **not** change. Both before and after this source-scope change, original Election emits a native method equivalent to:

```rust
pub fn vote_commit<Private, W: TryWitnesses<Private>>(
    context: CircuitContext<Private>, witnesses: &W, ballot: PermissibleVotes,
) -> Result<CircuitResult<Private, ()>, CompactError> { /* native evaluation */ }
```

There is no generated recorded `vote_commit` method and no observed-call proof API. The original Zerocash `spend` and `zerocash_mint` have the same native-only status. This ADR changes no typed IR schema, renderer, generated runtime ABI, or ledger primitive mapping. It changes the inventory's source-to-compiler provenance and a compiler acceptance gate.

### Evidence and boundary

At exact clean schema-12 base `a6bfc04a`, `${HISTORICAL_NIX_STORE}/yvvwqzj9z5a8cp0lclhni9vxirnabzk4-compactc/bin/compactc` compiles both original sources for TS and Rust. Both fresh standalone generated Cargo crates pass `cargo check --offline`. The source-scope gate checks seven compiler-proof-true flags, all seven expected unavailable recording reasons, and byte equality for the original Zerocash generated library. The Election generated library differs from the existing oracle only in its constructor authority parameter/default; the existing oracle execution test is representative, not original-constructor parity. Existing fixture tests compare selected native Election/Zerocash state with checked TypeScript captures but do not prove these exact original sources or create ledger-8 proof calls. The full inventory must show exactly seven unknown-to-known transitions, with zero proof-available gain and seven additional known proof gaps. New recorded/proof work needs separate ADRs and evidence.

### Consequences

Future recording slices will intentionally update the expected typed gap path and then prove availability with recorded/replay/ledger evidence. Compiler source acceptance remains distinct from native behavioral parity and deployment readiness. This decision belongs to milestone `rust-backend-v2`; local delivery is signed and DCO committed, with no push or remote CI.


### Delivery — 2026-10-05

Focused milestone issue: [#222](https://github.com/MediaNoxLabs/compact/issues/222) in `rust-backend-v2`. Signed GPG/DCO local commit `4578074b0a6e18684b8e493a02927b2a3c8aacfe` on `codex/original-election-zerocash-source` from `a6bfc04a`; no push or remote CI. Exact-head source receipt `${LOCAL_EVIDENCE}/original-election-zerocash-4578074b-source-scope.json` passes two TS and two Rust compilations, all seven proof flags, seven unavailable recording reasons, and the original Zerocash fixture byte comparison. Exact-head inventory receipt `${LOCAL_EVIDENCE}/original-election-zerocash-4578074b-inventory.json` has 316 proof-required, 249 available, 67 known gaps and 25 unassessed; baseline added/removed identities are empty. An isolated before/after inventory run over the two sources shows +7 assessed proof-required, +7 known gaps, −7 unassessed and zero available gain. Both fresh original generated Cargo crates pass `cargo check --offline`. Existing Election oracle one-test and Zerocash oracle two-test native TypeScript state suites pass, with the Election constructor difference retained as a limit. All 23 inventory unit tests pass.
