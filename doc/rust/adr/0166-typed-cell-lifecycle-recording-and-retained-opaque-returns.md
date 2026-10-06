---
id: RUST-ADR-0166
alias: ADR-0166
title: "Typed Cell lifecycle recording and retained opaque returns"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "Cell", "Counter"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 3f101849d0eb77270003ce20814c9b74bd6b2ab1fd0713a7f95d08996b846701
---
# RUST-ADR-0166 — Typed Cell lifecycle recording and retained opaque returns

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. The shared typed plan records bounded bboard optional-Cell and Counter lifecycle paths, including retained prewrite values. Failed calls expose no partial trace or gas, so parity assertions on rejected calls cover error and witness order only.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#270 closure](https://github.com/MediaNoxLabs/compact/issues/270#issuecomment-6017687046). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`8f2e134f`](https://github.com/MediaNoxLabs/compact/commit/8f2e134f685117d6b6a8825e64d2db9f447afdb5). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
status: accepted
date: 2026-10-05
```

## Historical decision and amendments

### Problem
The original test-center bboard source has complete native support after ADR0156 but lacks recorded post/take_down. It combines witness-owned poster identity, instance-counter hashing, optional opaque messages and a final top-level lexical binding returned after its Cell is cleared. A separate whole-contract matcher would duplicate ADR0162/0165 type and scope logic.

### Before / after
```rust
// Before: native execution only
let taken = ledger_contract::take_down(context, &witnesses)?;
// After: retained typed output and a replayable proof call
let taken = contract.recording().take_down(context)?;
assert_eq!(taken.execution.result, former_message);
let call = contract.recording().take_down_call(&observed, private)?;
```

### Decision
Reuse and rename recorded/typed_merkle_plan.rs to typed_plan.rs. A typed plan owns ordered syntax statements and its typed result. Keep bounded admission profiles for membership writes and optional-Cell/Counter lifecycles. Add typed Counter reads, Uint64-to-Field-to-Bytes32 conversion, enum/Bytes32/Maybe<OpaqueString> Cell read/write, and pure opaque default for none(). A final top-level Let contributes its own lexical bindings to the returned expression; earlier siblings and nested Lets do not escape, matching native ADR0156. Return the saved value after ordered mutation. Reuse ledger8 slots and existing conversion/hash helpers; no ABI/schema/source change.

### Validation
Fresh independent original TypeScript capture with empty and Unicode messages, two consecutive post/take-down cycles and instance-derived poster identity. Native/recorded result/state/effects/query program/four-dimensional gas/private outputs/replay. Reject occupied post, empty take-down, wrong owner and old-instance identity; witness prefix ordering. Pinned post and take-down proofs verify and ledger apply. Guards cover return scope, wrong types/slots, effectful helpers and unsupported operations; freshness, targeted Clippy and signed delivery.

### Limits
Only the declared bounded Cell/type/Counter profile is newly admitted. The generalized recorder remains gradual work. Failed Rust calls return no partial trace/gas; rejected-call evidence checks error and witness order, while successful calls compare gas/program exactly. All validation is local; no push or remote CI.



### Delivery — 2026-10-05

Issue https://github.com/MediaNoxLabs/compact/issues/270 in rust-backend-v2. Signed conventional GPG+DCO local commit `6182e54576c95835141dfb79a9f4649bf007a7a4`, parent `8f2e134f`, branch `codex/adr166-bboard-recording`. No push or remote CI.

#### Changes and engineering review
`recorded/typed_merkle_plan.rs` is now `recorded/typed_plan.rs`. The same actual-type lexical environment produces ordered statements plus its result. Membership admission remains bounded; a separate Cell lifecycle profile requires a Counter read, optional opaque Cell access and typed Cell writes. Post has an opaque argument and no counter mutation; take-down has an opaque result and a Counter increment. Pure helper calls remain transitively audited; the added pure default is restricted to OpaqueString.

Counter reads use the real recorded CounterSlot VM operation, then the existing Uint64→Field→Bytes32 conversion. Typed enum, Bytes32 and optional opaque Cell operations reuse runtime slots. The final top-level Let captures its own bindings once before mutation and supplies the returned expression. Earlier sibling and nested Lets cannot leak into the result. The original source, runtime ABI and schema are unchanged. A scoped renderer integration supplies the typed result without adding another whole-contract matcher.

#### Evidence
New independent capture `capture-bboard-recording-oracle.mjs` / `bboard-recording-oracle.json` exercises an empty message followed by `🌙 Midnight — 你好, Привіт` on the same logical board. Every call rehydrates public state while preserving private state. Native and recorded Rust match TS serialized state, actual pre-clear result, private callback inputs/output atoms/alignment, query VM program, all four gas dimensions and replay state/effects/gas. One witness per successful call advances private state from 5 to 9 across the two cycles. Poster keys differ with the instance Counter, which advances to 2 then 3. Each post has five queries/fifteen operations; each take-down has seven queries/twenty-one operations.

TS reported aggregate gas differs from summed query gas for some calls; tests compare Rust with independently captured query sums and replay, preserving actual VM accounting rather than copying reported aggregates. All dimensions are checked. The original native fixture remains separate; its former native-only comment now points to recording coverage.

Occupied post and empty take-down reject before a witness. Wrong secret and an adversarial state carrying the previous instance's poster key reject after the expected witness. The old-instance case intentionally constructs invalid owner metadata to prove the key binds the Counter; it is not a normal source transition. Failure APIs still omit partial Rust trace/gas, so rejected-call claims are limited to errors and ordered witness effects.

Four pinned ZKIR 2.1.0 proof transactions verified and ledger-applied: post and take-down for both empty and Unicode cycles. Post k=13/4569 rows; take-down k=13/4580 rows. Observed-call preparation agrees with checked recorded traces. Final applied state matches native, including cleared optional message and incremented instance.

#### Local gates
- Eight backend library tests and 136 renderer tests passed.
- Both bboard integration tests passed. Renderer tests cover renamed domains, effectful helper rejection, wrong Counter slot and final-root scope boundaries.
- Targeted all-target/all-feature Clippy passed.
- All 153 generated fixtures fresh.
- `check_compactc_target.py --test-center-bboard` passed with required recorded/observed capabilities.
- Positive original-source cohort passed: one TS/Rust source, two proof-required calls and one non-proof pure export. The manifest requires both post and take_down; stale negative gaps removed.
- Exact signed-head focused receipt `${LOCAL_EVIDENCE}/compact-focused-6182e545-bboard/receipt.json` passed 2/2 recorded.
- Dedicated and broad proof gates now invoke `--bboard` smoke.

#### Preserved rerun artifacts
Proof output is separate from native/focused output: `${LOCAL_EVIDENCE}/compact-adr166-proof`. Its four key files are retained (post prover 2826482 bytes / verifier 2119; take_down prover 2827107 / verifier 2119). Run `cargo +1.99.0 run --quiet -p compact-rust-proof-smoke -- --bboard ${LOCAL_EVIDENCE}/compact-adr166-proof` with an isolated Cargo target. Native output `${LOCAL_EVIDENCE}/compact-adr166-native` and frozen compiler `${LOCAL_EVIDENCE}/compact-adr166-local/compactc` may be regenerated without replacing proof keys.

#### Remaining scope
Broader source parity and general recording admission remain separate backlog. This slice establishes complete original bboard recorded/observed support without claiming the whole initiative production-ready.
