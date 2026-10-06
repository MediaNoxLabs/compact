---
id: RUST-ADR-0165
alias: ADR-0165
title: "Typed conditional counter recording and election reveal"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "Merkle", "Counter"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 68f2adc23060d14ead67e8c8aac592a8940ab85dd77ce3ae8bd780ed2e8c4aa2
---
# RUST-ADR-0165 — Typed conditional counter recording and election reveal

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. The election reveal slice extends typed Merkle membership with exact Uint16 Counter and branch-local Set effects. Admission is confined to the audited zero-argument Unit structure.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#269 closure](https://github.com/MediaNoxLabs/compact/issues/269#issuecomment-6017685194). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

### Original source metadata

```yaml
status: accepted
date: 2026-10-05
```

## Historical decision and amendments

### Problem
Election reveal remains native-only after ADR0162. It requires private phase/ballot witnesses, optional committed-vote membership, a one-time reveal nullifier and exactly one selected tally increment. Eager evaluation of branches would overcount votes or witnesses.

### Before / after
```rust
// Before: native only
ledger_contract::vote_reveal(context, &witnesses)?;
// After: replayable typed API and observed proof call
contract.recording().vote_reveal(context)?;
contract.recording().vote_reveal_call(&observed, private)?;
```

### Decision
Extend the typed Merkle membership plan from ADR0162 with unsigned literals, typed u16 Counter amounts, and branch-local StateAction::If frames. Admit zero-parameter Unit circuits containing root membership, Set insertion and a Counter increment, retaining the existing enum-parameter commit domain. Validate actual amount type and ledger slot provenance. Reuse ledger8 CounterSlot.record_increment; no runtime ABI, source or schema change. Future generic recorder reuse remains a separate design step.

### Evidence required
Independent unchanged-source TypeScript oracle, native/recorded/query/gas/private/replay comparisons for both ballots. Wrong public/private phases, missing or malformed committed paths, wrong roots/leaves, double reveal with honest private state and reset private state. Both ballot proofs verify and ledger apply; selected tally changes exactly once. Renderer negatives for unsupported mutations, wrong counters, amount type and unsupported domain. Fixture freshness, focused gate, Clippy and signed delivery.

### Boundaries
The unexported source ledger requires TS pre-call snapshot inspection while Rust uses read-only LedgerView; local tree lookups remain unmetered. Failed Rust calls do not expose partial trace/gas, so rejection evidence compares error and ordered witness effects; exact gas/program parity applies to successful calls. No remote CI or push.



### Delivery — 2026-10-05

Issue https://github.com/MediaNoxLabs/compact/issues/269; milestone rust-backend-v2. Signed GPG+DCO local commit `b81305ff29a3af752434d84a0e2a91e3a650163f`, parent `3e5da728dc3481a22ff05d0ade3c11bab37ca767`, branch `codex/adr165-election-reveal`. No push or remote CI.

#### Implementation
The existing typed plan now supports unsigned literals and action-level conditionals with branch-local frames. Counter increments validate the declared Counter slot/index and an actual Uint<16> bound before conversion to the runtime u16 amount. The zero-parameter membership/Counter/Set domain admits reveal; the existing enum-parameter membership/Merkle/Set domain retains commit. Native ledger8 CounterSlot recording is reused; no runtime ABI, source or schema change. Both branches are statically checked and only the selected branch executes. Uniform frame returns use the same narrowly scoped Clippy allowance as the existing recorder.

The shared live-ledger witness adapter now supports ballot reads and committed-vote paths. Execution and proof tests share it through one module declaration. Original election and its oracle fixture now have five of five recorded and observed-call circuits.

#### Independent oracle and proof evidence
`capture-election-reveal-oracle.mjs` freshly captures the unchanged source TypeScript. Both ballots match native/recorded final state/effects, query program, private state, all private output atoms/alignment and replay state/effects/gas. Each successful reveal has five queries and twenty-three VM operations. Ordered witnesses: state, secret, vote, path, advance. Private phase becomes revealed with five callbacks. Exactly one selected tally increments; the other remains zero and the commitment tree is preserved.

Both gas vectors: readTime 170000000; computeTime 1400815470; bytesWritten 724; bytesDeleted 724. All four dimensions agree with summed independent TS queries and replay.

Eight rejection scenarios distinguish wrong public phase, wrong private phase, honest repeated reveal, reset-private repeated nullifier, absent committed path, wrong root, wrong leaf and malformed path. Native/recorded errors and witness prefixes match; malformed path error wording differs because Rust rejects typed path conversion while TS rejects witness return shape. TS selected-query prefixes are captured. Failed Rust calls still expose no partial trace/gas, so no exact failure-gas claim is made. The source keeps ledger fields unexported: TS uses the actual pre-call tree snapshot and Rust uses read-only LedgerView; both local path lookups remain unmetered.

Pinned ZKIR 2.1.0 artifacts: `${LOCAL_EVIDENCE}/compact-adr165-rust`, vote$reveal k=14 and 10737 rows. Both yes/no proofs verified and transactions ledger-applied, matching native final state and selected tally. Observed-call preparation matches the checked recorded trace.

#### Gates and remaining scope
- Eight backend library tests, 131 renderer tests, six election tests passed.
- Renderer guards cover decrement exclusion, ledger provenance, helper closure and source-name independence. Typed counter amount regression rejects narrower/wider actual bounds.
- Targeted all-target/all-feature Clippy passed.
- All 152 fixtures fresh; election regenerated after the final lint-only emission adjustment.
- Exact-head focused receipt: `${LOCAL_EVIDENCE}/compact-focused-b81305ff-election/receipt.json` (five of five recorded, subject to receipt completion).
- Parent lane owns original-source cohort vote$reveal gap removal and positive requirement at integration.
- Broader production backlog and failure-trace observability remain separate. Next planned source slice is original bboard; typed planner reuse will be evaluated before extending admission.


Final exact-head focused gate completed successfully: 5/5 recorded at b81305ff. Worktree clean; no running build/proof process.

Artifact retention correction: final lint-only regeneration into ${LOCAL_EVIDENCE}/compact-adr165-rust replaced its prover/verifier files after the successful proof run. The parent integration lane is regenerating them for its independent rerun; the successful proof evidence remains valid but that original directory is not a complete reusable key artifact. Subsequent slices keep proof and native/focused directories separate.
