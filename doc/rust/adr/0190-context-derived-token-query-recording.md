---
id: RUST-ADR-0190
alias: ADR-0190
title: "Context-derived token query recording"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "Kernel", "token"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 5bf4ec5e0098c3723e47377656275830c279e1daa4558e44670696aedae1ff04
---
# RUST-ADR-0190 — Context-derived token query recording

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. A bounded action-free context query records the original DAO voting token derivation through audited pure helpers and Kernel.self. It closes that one microDAO export, not stateful reveal or shielded flows.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#294 closure](https://github.com/MediaNoxLabs/compact/issues/294#issuecomment-6017727839). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

### Original source metadata

```yaml
status: accepted
date: 2026-10-05
```

## Historical decision and amendments

### Problem
Original test-center/test-contracts/micro-dao.compact compiles natively with eleven exports, but seven proof-required exports remain unrecorded. The smallest is dao_voting_token (line 252): a pure tokenType call whose address argument is the metered Kernel.self query. Existing typed composite recording admits struct-returning expression helpers; ordinary pure-call recording does not audit PersistentCommit and scalar Bytes32 context-query results have no admission profile.

### Before / after
```compact
export circuit dao_voting_token(): Bytes<32> {
  return tokenType(dao_token_domain_separator(), kernel.self());
}
```
Before: native Rust succeeds, recording/observed-call report an explicit return gap. After: generated typed recording evaluates the domain argument, records the canonical Kernel.self VM query exactly once in caller order, and derives the token using the existing upstream persistent commitment.
```rust
let domain = domain_bytes;
let (frame, address) = frame.kernel_self()?;
let result = runtime::persistent_commit((domain, address_bytes), opening);
```
The actual generated code retains declared types and intermediate bindings; the example summarizes evaluation order.

### Decision: emitter and runtime
Add a separate bounded context-query admission profile to typed_plan. It accepts action-free, parameter-free Bytes32 results assembled from canonical Kernel.self, closed Bytes literals, exact typed struct projections/coercions/tuples/lexical bindings, and acyclic audited pure helper calls with Bytes32/ContractAddress arguments and Bytes32 results. Evaluate arguments exactly once before entering isolated callee parameter scope. Inline supported helper expressions into the same typed plan so argument/member/opening/result type checks are explicit. Require at least one Kernel.self observation and zero witness/public-slot/native-intent effects. Do not use unrelated global witness declarations as an admission veto.

PersistentCommit reuses runtime::persistent_commit and ledger8 crypto. No runtime change, ABI47/schema20 retained. No hardcoded source/helper names and no broad arbitrary Bytes-return acceptance. Unsupported pure helper forms, malformed signatures/coercions, recursion, leaked bindings, wrong opening length, extra actions and hidden witness/query/intent effects remain refusals. Existing composite/helper profiles remain unchanged.

### Scope and dependencies
Closes only dao_voting_token: microDAO seven recording gaps become six; all four Coracle gaps remain. No dependency on ADR0188 Zswap intents or ADR0189 packaging. Stateful scalar/action helper composition for vote_reveal/Coracle guess remains later work. Original source stays unchanged.

### Validation plan
Compile/check the complete original microDAO; join capabilities with authoritative contract-info for eleven exports. Independently capture original TS at several distinct deployed contract addresses and compare native/recorded Bytes32, full VM transcript, query gas, private outputs, ledger state/effects and replay. Selectively generate pinned ZKIR artifacts for dao_voting_token from the unchanged full contract, prove/verify and ledger-apply the nonempty query; state whether shared smoke relaxation is used. Negative renderer tests cover typed signatures, scope, recursion, opening width, argument order/evaluate-once, hidden effects, unrelated retained witness declarations, and rejection of parameterized/general Bytes-return shapes. Run focused backend/consumer/Clippy/freshness/source gates, then conventional GPG+DCO commit. No push/remote CI.


### Native source lint finding and precise cleanup
Adding the complete original microDAO fixture to strict Clippy exposed four pre-existing explicit Unit tails in native conditional Kernel claim helpers. Before: `if condition { evaluated_steps; () } else { () }`. After: `if condition { evaluated_steps; }`. The native syn emitter omits only syntactically empty tuple tails; an else branch is omitted only when its evaluated statement list and remaining value are both empty. Conditions, effectful Unit expressions, and all evaluated branch statements remain. No lint suppression. Searching all existing generated fixtures found literal Unit tails only in the new microDAO fixture, so no old fixture regeneration is needed.

The context-query commitment shape is explicitly a pair of Bytes32 values and a Bytes32 opening. Pure helper signatures use unique parameter names and exact Bytes32/ContractAddress argument types with Bytes32 result. Issue: https://github.com/MediaNoxLabs/compact/issues/294.


### Validation evidence
Fresh independent original-source TypeScript capture passes against native and recorded Rust at three distinct addresses, including upstream ContractAddress::custom_shielded_token_type agreement, exact transcript/query gas, private state/outputs, unchanged ledger state/effects and replay. Original eleven exports remain: four proof-free and seven proof-required; exactly dao_voting_token records and six gaps remain. The pre190 compiler fails the new positive guard.

Backend tests pass 12 library +12 CLI +150 renderer cases; typed-plan negatives include opening/pair shape, duplicate/wrong formal signatures, wrong Kernel.self carrier, bad projection, recursion, lexical leak, hidden witness/Cell/Kernel/Zswap effects, no-query constant, extra actions and arbitrary parameters. Two argument-side Kernel.self observations preserve caller order/evaluate-once in the emitted typed steps. Strict targeted Clippy/format and one-fixture freshness/source-cohort checks pass. The only native formatting change is the new full microDAO fixture.

Pinned key generation: zkir 2.1.0 compile ${LOCAL_EVIDENCE}/compact-adr190-proof/zkir/dao_voting_token.zkir ${LOCAL_EVIDENCE}/compact-adr190-proof/keys/dao_voting_token.prover ${LOCAL_EVIDENCE}/compact-adr190-proof/keys/dao_voting_token.verifier (k13, 3967rows), using the unchanged complete source compiled with --skip-zk. Proof harness --micro-dao-token ${LOCAL_EVIDENCE}/compact-adr190-proof verifies and ledger-applies the nonempty query at the actual deployed contract address under the shared unbalanced smoke policy. No funding claim. Artifacts and ${LOCAL_EVIDENCE}/compact-adr190-proof.log are preserved separately from normal compile outputs.


### Signed delivery receipt
Commit eb8437eb89c5022f6227a779d3d30ad31e7c2180, GPG verified and DCO signed. Exact-head focused receipt ${LOCAL_EVIDENCE}/compact-focused-adr190-micro-dao-token/receipt.json passed: one fixture, 1/7 recorded. Python29 tests passed. Frozen compiler ${LOCAL_EVIDENCE}/compact-adr190-compactc; source unchanged; no push/remote CI.
