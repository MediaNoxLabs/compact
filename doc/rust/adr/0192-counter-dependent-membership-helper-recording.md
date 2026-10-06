---
id: RUST-ADR-0192
alias: ADR-0192
title: "Counter-dependent membership helper recording"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "Counter", "helper"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: cd7cd4bd0df71127cfdad0e13491d25270ba8a37b84fabbb3ddd1155f5e14c8c
---
# RUST-ADR-0192 — Counter-dependent membership helper recording

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Declaration-directed helper resolution records Counter-dependent reveal/nullifier/hash helpers with isolated typed scopes and cycle guards. The original vote_reveal proof is bounded by its seeded ballot/path cases, not a full DAO lifecycle.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#295 closure](https://github.com/MediaNoxLabs/compact/issues/295#issuecomment-6017729325). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`40d3b707`](https://github.com/MediaNoxLabs/compact/commit/40d3b707fcde6aaf9072e25cee3876c0170a7eb9) · [`89bd7764`](https://github.com/MediaNoxLabs/compact/commit/89bd7764d2ff0fd71eccd8b634c866005f671b2f) · [`ee912835`](https://github.com/MediaNoxLabs/compact/commit/ee912835169d314851b99fc92c3e543ffaa2fd2e). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
status: accepted
date: 2026-10-05
```

## Historical decision and amendments

### Problem and prior decisions
Original microDAO vote_reveal remains unrecorded after ADR0190 token queries. It calls two stateful scalar helpers: reveal_nullifier(sk) and commit_with_sk(ballot,sk), each hashing three Bytes32 members with round.read() converted through Field to Bytes32. The root also calls the existing pure Merkle helper graph. Current call dispatch selects a declaration map by profile (pure versus stateful composite), ordinary If permits only Boolean values, and membership admission rejects every Counter read. Existing runtime primitives already implement all required ledger operations.

### Before / after
```compact
const rev_nul = reveal_nullifier(sk);
const cm = commit_with_sk(vote.value ? pad(32,"yes") : pad(32,"no"), sk);
```
Before: native helpers work; recording reports an enclosing Assert capability gap. After, declaration-directed call resolution applies explicit per-profile audit and evaluates a supported helper into the same typed frame:
```rust
let sk = evaluated_caller_argument;
let (frame, round) = round_slot.record_read(frame)?;
let digest = runtime::persistent_hash((prefix, field_to_bytes32(round), sk));
```
Each helper query remains observable in source order, including two distinct round reads. Callee locals cannot capture caller locals; arguments are evaluated once in caller order before fresh callee scope. The ballot branch joins matching Bytes32 values with its selected frame.

### Decision: shared emitter ownership
Extend shared typed_plan, not a separate renderer. Resolve calls by actual pure/stateful declaration, reject ambiguous or missing names, then apply existing context-query/composite policies or a new bounded scalar-counter-hash policy. Share typed argument/scope/result handling while preserving existing profile boundaries and cycle audits. Pure Merkle helpers continue through the existing audited pure closure path; stateful scalar helpers inline into the caller frame.

The scalar policy admits action-free Bytes32 helpers with Bytes32 arguments and a three-Bytes32 PersistentHash tuple containing one declared CounterRead via checked Uint64→Field→Bytes32. Track reads originating in these audited helper bodies. Membership admission may accept those reads only; do not loosen global Counter-read or witness guards. Existing enum phase reads, Maybe/witness codecs, Merkle root verification, Set membership/insert, selected Counter increment and final Unit witness are reused. No Kernel/coin/intent effects, Cell writes, Counter writes inside helpers, arbitrary helper bodies, or general action-helper composition. ABI48/schema20 unchanged if runtime reuse holds.

### Scope and source denominator
Unchanged complete test-center/test-contracts/micro-dao.compact; existing full-source fixture reused. vote_reveal becomes the second recorded proof-required export (with dao_voting_token), leaving five of seven recording gaps. Coracle remains unchanged. No funded buy-in/commit lifecycle claim.

### Evidence plan
Independent TS capture and Rust native/recorded parity for both ballots at zero and nonzero rounds, with a high Uint64 round boundary. Preserve exact seven public queries: phase Cell, round, revealed Set membership, round, committed-tree root, selected tally increment, revealed Set insert. Preserve five private callbacks: state, secret, vote, path, advance. The path witness uses local tree inspection on the actual readonly ledger view, matching TS snapshot inspection without extra VM charges.

Reject wrong public/private phase, duplicate/repeated reveal, absent vote/path, wrong root/leaf, cross-round commitment mismatch, malformed path and gas failures with exact witness prefixes. Successful calls compare result/alignment, full VM, gas, private outputs/state, ledger state/effects and replay. Rejections do not invent post-error Rust context or gas. Typed IR negatives cover wrong helper signatures/scopes, recursion/ambiguity, counter-slot/declaration mismatch, wrong tuple/cast/branch types, hidden effects and unrelated retained witnesses.

Proofs use an explicitly seeded prior-state fixture built with canonical ledger operations and matched to independent TS; they validate reveal execution, not historical funded setup. Generate only original vote_reveal keys with pinned ZKIR, prove both ballots, verify and ledger-apply under the shared unbalanced smoke policy. Run targeted backend/consumer/Clippy, source cohort/freshness and signed exact-head receipt. GPG+DCO conventional local commit; no push/remote CI.

### Coordination
This delivery owns typed_plan through signing; ADR0191 planned composite+Zswap integration waits for ownership handoff. It does not modify zswap_plan or native circuit-intent runtime.


### Delivered evidence and gas accounting decision

Implementation retains ABI48/schema20 and reuses the runtime unchanged. Call ownership is now explicit: `stateful_circuits` is a declaration index, `composite_values` is a separate admission flag, and `Plan::call` resolves pure/stateful declarations before applying a profile audit. `inline_call` materializes caller arguments before entering the callee's cycle guard and isolated scope. Scalar body depth and read counts prevent unrelated root Counter reads from acquiring scalar-helper provenance. Existing composite pure-call eligibility remains unchanged for ADR0191 to extend explicitly.

Seventeen fresh original-TypeScript scenarios pass: yes/no at rounds 0 and 7, yes at Uint64 maximum, wrong public/private phases, absent vote/path, wrong root/leaf, cross-round commitment, malformed path, repeated private state, duplicate nullifier, and zero/prefix gas budgets. Full successful transcripts preserve seven queries and five private outputs. Failure cases compare exact witness/path-argument prefixes and native/recorded errors; malformed path and out-of-gas errors retain their respective typed/runtime categories. Neither Rust post-error context nor gas is fabricated. TS path lookup uses a captured current-query snapshot; Rust uses readonly local LedgerView tree inspection. Neither adds a VM query.

**Gas policy evidence:** original `runtime/src/circuit-context.ts:206` assigns `circuitContext['gasCost'] = res.gasCost`; it reports the last query, not a sum. The fixture preserves three independent fields, with the round-0 yes case:

| Measurement | readTime | computeTime | bytesWritten | bytesDeleted |
| --- | ---: | ---: | ---: | ---: |
| TS wrapper `reportedGas` (last query) | 170000000 | 1335120837 | 1816 | 1644 |
| TS `queryCostSum` = native = recorded | 1360000000 | 9086161452 | 3460 | 3288 |
| Whole-program `replayGas` | 1360000000 | 2573548005 | 1816 | 1644 |

Per-query setup/cache behavior explains why a concatenated replay has a distinct cost. The tests assert each measurement separately, including reportedGas exactly matching the last accepted query; this delivery neither alters TS/runtime gas behavior nor claims aggregate wrapper equality. This follows the ADR0184/0188 distinction.

Both original `vote_reveal` outcomes (yes round0, no round7) now prove, verify and ledger-apply from explicitly seeded prior states. The applied-state duplicate-nullifier guard rejects a second reveal. Pinned ZKIR: k14, 11528 rows. Persistent keys/source/output: `${LOCAL_EVIDENCE}/compact-adr192-proof`; evidence `${LOCAL_EVIDENCE}/compact-adr192-proof.log` and `${LOCAL_EVIDENCE}/compact-adr192-keygen.log`. Shared unbalanced smoke policy remains explicit; this does not establish funded buy-in/commit history or close the five remaining microDAO recording gaps.

Focused validation before signing: 13 backend unit tests, 13 CLI tests, 151 renderer tests; original microDAO token and reveal consumer tests (17 reveal scenarios); 34 Python tests; targeted backend/consumer/proof-runner Clippy with warnings denied; format/diff checks; three generated fixture freshness checks (original microDAO, stateful_struct, election). Original-source cohort joins all eleven exports to authoritative contract-info (seven proof-required, two recorded). The new reveal source guard passes current compiler and fails the predecessor compiler. Signed exact-head receipt and commit follow below.

### Signed delivery

- Commit: `6364a444210be97c3e7626e09595af24a57f28ed` (`feat(rust-backend): record round-dependent microDAO reveals`), GPG verified, DCO included.
- Exact-head receipt: `${LOCAL_EVIDENCE}/compact-focused-adr192-micro-dao-reveal/receipt.json`; passed, one original-source fixture, **2/7 proof-required exports recorded**. Worktree clean.
- Local proof command: `CARGO_TARGET_DIR=${COMPACT_SOURCE}/target/compactc-consumer CARGO_INCREMENTAL=0 cargo +1.99.0 run -p compact-rust-proof-smoke -- --micro-dao-reveal ${LOCAL_EVIDENCE}/compact-adr192-proof`.
- Frozen compiler: `${LOCAL_EVIDENCE}/compact-adr192-compactc`; combined Scheme `${HISTORICAL_NIX_STORE}/1d5x83y7dgg6abzv0fzvna893na3nfjj-compactc-scheme-local-slice/bin/compactc-scheme`.
- Issue: https://github.com/MediaNoxLabs/compact/issues/295, milestone rust-backend-v2. No push or remote CI.
- Shared planner ownership handed to ADR0191 after signing; declaration index and profile admission remain separate.
### Integrated delivery — 89bd7764

ADR192/#295 is integrated as GPG/DCO-verified `89bd7764`, with the helper preflight follow-up at `40d3b707`. Private schema 20 / runtime ABI 48 remain unchanged. All 167 generated fixtures are fresh. Backend 13 library, 13 CLI and 151 renderer tests passed, as did targeted strict backend/generated-microDAO/proof-smoke Clippy. Four-fixture focused gate passes **9/15 proof-required APIs recorded**, with 3 separate native-only nonproof exports: `${LOCAL_EVIDENCE}/compact-focused-89bd7764/receipt.json`. This covers original microDAO, composite returns, native Zswap intents and funded transfer.

Both original `vote_reveal` ballot proofs were rerun from the combined main branch and verified/ledger-applied; applied-state duplicate attempts reject. `${LOCAL_EVIDENCE}/compact-integrated-adr192-proof.log`. Explicit seeded prior state and the shared unbalanced smoke policy remain the scope; this is not a funded buy-in/commit/reveal lifecycle claim.

Exact source inventory `${LOCAL_EVIDENCE}/compact-89bd7764-inventory.json`: **352/362 proof-required APIs available, 10 explicit gaps, zero unassessed exports**; 209 sources, 731 exports, 188 compiled roots and 369 nonproof exports. No missing/unmatched compiler rows or declaration baseline drift. Remaining gaps: five microDAO, four Coracle, one planned composite output. Availability is not full semantic parity.

The preceding full gate and clean macOS ARM portable package belong to ee912835. New192 has the focused integrated evidence above. ADR191 composite intents and ADR193 original Coracle guess continue in isolation. User-owned documentation edit preserved; no push, publication or remote CI.
