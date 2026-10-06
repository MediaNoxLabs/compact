---
id: RUST-ADR-0162
alias: ADR-0162
title: "Typed Merkle membership recording plans and election commit"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "typed-plan", "Merkle"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 65ab7d9c759240c385ae620d875b6a996041727da427988679adfdffdb04ef7d
---
# RUST-ADR-0162 — Typed Merkle membership recording plans and election commit

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. A typed Merkle recording plan owns lexical values, coercions and ordered state effects for the election commit slice. Counter-dependent reveal and local tree lookup boundaries remained separate decisions.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#266 closure](https://github.com/MediaNoxLabs/compact/issues/266#issuecomment-6017679938). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

### Original source metadata

```yaml
status: accepted
date: 2026-10-05
```

## Historical decision and amendments

### Problem
Original election vote$commit has native Rust only. It combines phase/private-state short-circuit guards, five ordered witness effects, typed optional path projection, pure Merkle folding, membership/root/leaf assertions, commitment-tree insertion and nullifier insertion. Extending monolithic syntax matchers would duplicate type/scope handling and risk skipped coercions or eager witnesses.

### Before / after
```rust
// Native-only before
ledger_contract::vote_commit(context, &witnesses, ballot)?;
// Proposed developer-facing proof API
let contract = ledger_contract::Contract::from(witnesses);
contract.recording().vote_commit(context, ballot)?;
contract.recording().vote_commit_call(&observed, private, ballot)?;
```

### Decision
Introduce a bounded module recorded/typed_merkle_plan.rs. TypedValue stores actual IR type and Rust expression; lexical binding environments keep declared types and provenance. Every coercion uses actual/target types. Expressions emit ordered statements; effectful conditionals move RecordingFrame through only the selected branch. Pure helper calls require a transitive effect-free AST audit, including fixed vector folding for Merkle paths. Reuse generated pure functions and ledger8 Cell/Set/Merkle/witness recording primitives. No source rewrite, new IR schema, runtime ABI or new cryptography.

### Initial admission boundary
An enum-parameter Unit circuit containing Merkle root membership observation, with supported assertions, lets, witnesses and Bytes32 Set/Merkle writes. No Counter or arbitrary ledger mutations, stateful helper calls, opaque values or unsupported expressions. This is a reusable typed planner with a bounded entry domain, not a contract-name matcher. Existing recorder remains fallback. Later integration should reuse typed values instead of broadening old untyped-local helpers.

### Required evidence
Original TS independent yes/no execution plus readonly path snapshot adapter, native/recorded state/program/four-dimensional gas/private outputs/replay. Phase and private state short-circuit failures, duplicate nullifier, absent path, wrong root, wrong leaf, malformed path rejection with witness order and selected queries. Pinned proofs verify and ledger apply for both ballots. Renderer guards, capability boundaries, fixture freshness, Clippy and signed commit.

### Limits
Both API visibility and local tree lookup gas boundaries from ADR0160 remain explicit. vote$reveal has Counter branches and remains a separate slice. No production-ready claim before the remaining backlog is complete.



### Delivery — 2026-10-05

Issue: https://github.com/MediaNoxLabs/compact/issues/266 (rust-backend-v2).
Local signed conventional GPG+DCO commit: `3e5da728dc3481a22ff05d0ade3c11bab37ca767`, parent `d3a3a12541f4aab70f828a0a21c75bfe1e094889`, branch `codex/adr162-election-commit`. No push or remote CI.

#### Emitter and runtime changes
The new typed planner stores actual `Type` beside each `syn::Expr`, evaluates lexical declarations once, and validates formal arguments, struct projections and ledger slot provenance. Coercions call the existing native coercion implementation with both actual and target types. Boolean conditionals return the selected branch frame, preserving witness/read order. Closed pure-call validation is transitive, rejects recursive helper cycles and effectful helpers, and permits the fixed Merkle path fold. Existing ledger8 recording operations and native pure helpers are reused. No Scheme/schema/runtime ABI change.

Before, only native `vote_commit` was available. After, the generated crate additionally exposes recorded and observed-call APIs with typed ballot parameters and five ordered private outputs. The unchanged original election and election oracle each now admit four of five recorded circuits. `vote$reveal` remains excluded.

#### Execution and rejection evidence
Fresh independent TypeScript capture: `runtime-rs/tests/fixtures/capture-election-commit-oracle.mjs` and `election-commit-oracle.json`. Both yes and no ballots match native/recorded final state and effects, private state, private output atoms/alignment, query program, aggregate gas and replay. Each successful call has five queries, thirty VM operations and witness order `state → record → secret → path → advance`. Gas: readTime 2380000000; computeTime 8584605137; bytesWritten 1448; bytesDeleted 2252.

The TypeScript adapter inspects the actual pre-call eligible-voter tree snapshot because the original source does not export these ledger fields. Rust witnesses use the read-only LedgerView. Both local path-inspection APIs are unmetered; this is deliberately distinct from metered VM reads. Source visibility is unchanged.

Wrong phase, private phase, duplicate nullifier, absent path, wrong root, wrong leaf and malformed path reject in native and recorded Rust with the expected ordered witness prefix. TypeScript captures the selected query prefixes independently. A malformed Rust witness path is rejected during typed path conversion; TypeScript rejects its wrong-length witness result. These errors have different wording. **Limitation:** Rust failure APIs return an error without the partial trace or gas. Rejection tests establish error and witness order, not exact cross-language failure-gas parity. Successful calls compare every gas dimension and replay gas exactly.

#### Local gates
- 7 backend library tests; 130 renderer tests; 5 election integration tests passed.
- Renderer negatives cover effectful/recursive pure helpers and wrong ledger indexes; name independence is checked. A typed-local coercion regression verifies widening uses the actual source bound and narrowing is rejected.
- Targeted all-target/all-feature Clippy passed.
- 152 fixtures checked: zero stale and zero failures.
- Original `examples/election.compact` compiled with four recorded circuits and reveal excluded.
- Pinned ZKIR 2.1.0 artifacts `${LOCAL_EVIDENCE}/compact-adr162-rust`: vote$commit k=15, 16593 rows. Both yes/no proofs verified and their transactions ledger-applied; final state matches native execution.
- Clean signed-head focused gate: `${LOCAL_EVIDENCE}/compact-focused-3e5da728-election/receipt.json` (4/5 recorded).

#### Integration and remaining scope
The parent delivery lane owns the original source cohort manifest change: remove vote$commit's old expected gap and add it to required recorded circuits. This branch does not modify that shared manifest. The typed planner's deliberately bounded entry admission must be revisited through a separate ADR before broadening to generic recording. Reveal, failure-trace observability and remaining source parity remain outside this slice.
