---
id: RUST-ADR-0179
alias: ADR-0179
title: "Ordered typed stateful struct construction"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["compiler", "struct", "expression"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 2a6d19a3dc8ba894c8669ed0a1a3fdf59af01a13801568a8072d38d188ad7b5e
---
# RUST-ADR-0179 — Ordered typed stateful struct construction

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Stateful struct construction checks declared shape/member count and evaluates members once in source order. The original-source inventory improvement establishes compiler assessment, not full behavioral or funded-contract completion.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#283 closure](https://github.com/MediaNoxLabs/compact/issues/283#issuecomment-6017710045). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`ff25c003`](https://github.com/MediaNoxLabs/compact/commit/ff25c0030711004cac68ca75a1339c3e7044af4f). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
status: accepted
date: 2026-10-05
```

## Historical decision and amendments

### Problem
Original micro-dao now lowers into IR but its standard-library mintShieldedToken fails Rust rendering. The actual ShieldedCoinInfo StructLiteral has nonce parameter, color=tokenType(domain, Kernel.self), and a Uint64→Uint128 cast. Current struct handling goes through the pure expression renderer. Scheme stateful lowering also falls back to ordinary struct lowering, which cannot preserve witness classification inside members.

### Before / after
```compact
return Snapshot { first: disclose(next_value()) as Uint<128>, address: kernel.self(), second: disclose(next_value()) as Uint<128> };
```
Before: effectful members reject or lose stateful witness classification. After: recurse using each typechecked declared member type and preserve explicit source coercions. Generate one typed temporary per member in Compact AST evaluation order before constructing the Rust struct.
```rust
let first: runtime::BoundedUint<MAX128> = first_value;
let address: types::ContractAddress = queried_address;
let second: runtime::BoundedUint<MAX128> = second_value;
let value = types::Snapshot { first, address, second };
```

### Emitter/runtime decision
Add a stateful new-struct arm in Scheme using stateful-typed-expression-ir for each member. Add StructLiteral handling to render_state_expression: require Struct shape and exact field count, evaluate each member once with the shared stateful renderer, validate its actual resulting type against the declaration, bind it before processing the next member, and retain nested witness/query/native-intent effects. Existing Coerce/UnsignedCast nodes handle actual types; no fabricated type annotation substitutes for coercion. Ordinary pure structs retain their existing path. No new IR schema, runtime ABI, or runtime semantic API.

### Dependencies and boundaries
Base main4ef0fa57 plus signed ADR0177 cherry-pick dcf0bc60 preserves metered Kernel.self from ADR0170 and the authoritative observed allocation lock from ADR0175. RuntimeABI45/schema20 unchanged. Broad production/recording claims remain separate from native admission. Struct field labels/types come from the typed domain model, never contract names.

### Evidence plan
Independent TS source tests mixed witnesses, Kernel.self, nested structs, selected branches and local helpers that emit native coin intents. Distinguish field values and private-state order to catch reordering/double evaluation; verify explicit Uint widening, actual query programs/gas, state/effects, private outputs and intent order. Reject malformed struct kind/member count/member type, omitted coercions and pure-use effects. Recompile original micro-dao unchanged and report each remaining boundary honestly; use targeted freshness/Clippy and local signed receipt, no old fixture sweep/new target/push/remote CI.


### Evaluation-order clarification
Independent TypeScript shows that reversed named-field spelling is normalized into declared member order before execution: both snapshot and reverse invoke witness tags [1, 2]. This implementation preserves that typed AST order, including the interleaved Kernel.self query. It does not promise textual named-field order. The direct stateful-return path also delegates new-struct nodes into the same typed expression lowering.


### Delivery
Signed GPG+DCO commit c9993227b1fb94b06c4ae11843030053750529c8; issue https://github.com/MediaNoxLabs/compact/issues/283 in rust-backend-v2. Six independent TS/native cases pass, including normalized reverse member spelling, selected false, nested helper, interleaved native output and pure Uint64-to-Uint128 widening. Exact public state/effects, values, private state/aligned outputs, actual query gas, provisional index7 and cursor8 match. Backend8 library +144 renderer tests, targeted Clippy, source guard and one-fixture freshness pass. Frozen focused receipt ${LOCAL_EVIDENCE}/compact-focused-c9993227-stateful-struct/receipt.json passed1fixture,0/4recorded; all4stateful exports proof-required/native-only. No proof/funding claim. Compiler ${LOCAL_EVIDENCE}/compact-adr179-compactc; Scheme ${HISTORICAL_NIX_STORE}/c6cnhvmbjy14wwamkp7jbk4s4v75ybfg-compactc-binary-nixos/bin/compactc-scheme. Original unchanged micro-dao diagnostic saved ${LOCAL_EVIDENCE}/compact-adr179-original-dao.stderr: standard-library.compact line207 char1, unsupported Compact Uint maximum 680564733841876926926749214863536422911; expected canonical u128. That 2^129−1 intermediate remains for ADR181. No push or remoteCI.


### Main integration evidence

Integrated signed90424741; formatted fixture refreshff25c003. All163 fixtures fresh (four changed); backend8+145 renderer tests and strict targeted Clippy pass. Frozen six-source receipt ${LOCAL_EVIDENCE}/compact-focused-ff25c003/receipt.json passes (30/35 selected proof APIs available). Exact inventory ${LOCAL_EVIDENCE}/compact-ff25c003-inventory.json:206sources/722exports/183compiled roots;333/347 assessed proof APIs,14known gaps,20unassessed, zero missing/unmatched rows and empty baseline drift. No new ABI. Native member evaluation follows independently observed Compact-normalized declaration order. Recording gaps remain explicit.
