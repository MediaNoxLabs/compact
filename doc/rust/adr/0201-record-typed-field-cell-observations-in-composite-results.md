---
id: RUST-ADR-0201
alias: ADR-0201
title: "Record typed Field Cell observations in composite results"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "Field", "composite"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 3b2faa482acc8aca19041ea32272c3afa8e61ae4e3d59c9406c8dbcf22e6e3f9
---
# RUST-ADR-0201 — Record typed Field Cell observations in composite results

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. The shared typed evaluator records Field Cell observations inside bounded composite results without a duplicate planner. Per-query gas, replay and seeded state are checked; historical smoke cases retain their documented balancing limits.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#305 closure](https://github.com/MediaNoxLabs/compact/issues/305#issuecomment-6017746576). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`305df36a`](https://github.com/MediaNoxLabs/compact/commit/305df36abb6730b8bb240cfaa46abf38af437a94) · [`7a311c9f`](https://github.com/MediaNoxLabs/compact/commit/7a311c9fe80ef588b744407fb39f0130ba6d352f) · [`ee912835`](https://github.com/MediaNoxLabs/compact/commit/ee912835169d314851b99fc92c3e543ffaa2fd2e) · [`fb4c2e24`](https://github.com/MediaNoxLabs/compact/commit/fb4c2e2456c89055062baf2d1740e4533071e564). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
status: accepted
date: 2026-10-06
```

## Historical decision and amendments

### Problem and evidence
The direct Field Cell read records, but wrapping the same observation in a typed Snapshot does not. Immutable integratedADR0195 compiler e743834caadda4808488be45195ea4ace5ceeb67c6934e0061a1a79ad74eefdc compiles all seven TS/native probes at ${LOCAL_EVIDENCE}/compact-adr201-research; only direct records. Six proof-required gaps cover Snapshot, scalar Field helper→Snapshot, Snapshot helper, two ordered reads, selected Cell branch and optional read. The helper returning Field uses StateReturn::CellRead rather than Expression.

### Before / after
```compact
export circuit direct(): Field { return first_cell.read(); }
export circuit snapshot(): Snapshot { return Snapshot { first_cell.read() }; }
```
Before: direct has typed recorded/observed-call APIs; Snapshot only has native execution. After: both use the same canonical CellSlot.record_read frame operation; Snapshot wraps the typed observed value with its generated struct codec. No consumer VM instructions, duplicate carrier or new runtime DSL.

```rust
let (frame, value) = ledger_slots::first_cell.record_read(frame)?;
Ok(frame.finish(types::Snapshot { value }))
```
The actual emitted AST retains hygienic typed bindings and precise source evaluation order.

### Decision: explicit policy and one evaluator
Use the existing composite audit walker and shared Plan value/scope/call evaluator. Add an explicit read-only Field-observation domain admitting Field/Boolean/recursive nonempty structs, literal/parameter/Let/typed If/Struct forms and correctly declared root Field Cell observations. Root result must be struct and actions empty. Helpers may return Field or bounded struct, with empty actions and acyclic declaration-directed resolution. Normalize only a permitted Field helper StateReturn::CellRead into the existing CellRead expression leaf for the shared audit/lowering. No new evaluator, fake field-update profile, runtime primitive, ABI or schema.

Structurally inspect every binding, operand, branch and reachable helper, including unselected branches. Reject witnesses, Kernel queries/effects, Zswap intents, Counter/collection operations, writes and unsupported pure helpers in this new domain. Preserve exact declared slot/type/member/argument/result checks, caller argument evaluation once, fresh callee scope and branch-local frames. Require a statically present read without asserting every selected path has a nonempty transcript; an empty optional path must retain upstream EmptyTranscript preparation refusal.

### Policy migration compatibility
Do not replace the current composite flags with an incomplete three-variant enum. ADR0195 shielded receive uses composite_values=false, composite_intents=true, unit_actions=true, which differs from the existing composite-intent domain with both composite flags true. Preserve it explicitly as a separate policy variant or equivalently clear capability methods, including pure-call/helper routing. Inspect ADR0194's final new flags/routing before migration and preserve all prior domains exactly. Shared policy migration is approved only after the signed194 handoff; independent source/oracle/tests/keys can proceed now.

### Validation
Use a complete unchanged fixture with distinct initial Field values. Compare direct scalar vs Snapshot, inline vs scalar/Snapshot helpers, two distinct reads with exact order, both selected branches, and optional empty branch. Fresh independent TS/native/recorded result, state/effects, ordered VM, private outputs, summed query gas and replay must agree. Retain wrapper last-query gas separately. Negative IR guards cover missing/wrong/nonroot slots, non-Field Cell type, member/result mismatch, argument arity/scope/cycles and hidden unsupported effects in either branch or helper.

Prove, verify and ledger-apply each nonempty API, including both selected outcomes. Explicitly reject empty optional-path preparation without fabricating public operations. This read-only fixture needs no shielded funding; use the established proof policy with its limits stated. Enforce fresh source/capability applicability, targeted test/Clippy/freshness and exact signed receipt.

### Coordination
ADR0194 owns typed_plan.rs/recorded.rs until signed handoff. Prepare new files only first. Existing runtimeABI48/schema20 stay unchanged. No push or remoteCI; reuse assigned warmtarget and isolated proof artifacts.


### Proof policy amendment — 2026-10-06
The initial signed implementation dfd0a05d and its eight passing proof/application checks used the explicitly documented shared unbalanced smoke policy. Before integration, review required the stronger default-strict gate. Preserve that initial evidence as limited smoke evidence; the delivery gate now uses ADR0194 fee_funded_state/fee_resolver, pinned Night-backed Dust fees, separate cryptographic verification and changed-binding rejection, transaction proving/sealing/balancing, WellFormedStrictness::default() and ledger application. No ledger strictness flag is disabled. Empty optional(false) remains EmptyTranscript and receives no synthetic read. Explicit seeded prior contract state does not prove its deployment lifecycle. This amendment changes the proof harness and README only; shared compiler ownership remains released to ADR0202.


### Delivered — 2026-10-06
Signed implementation **dfd0a05d24813ff6b1109b62cc836bab7b1d9347** plus proof-policy correction **35aeb6b415cebcbf91bcb7e7e5158f50ce726542**, both conventional/GPG/DCO verified. Integrated main as fb4c2e24 + 7a311c9f. Issue [#305](https://github.com/MediaNoxLabs/compact/issues/305) remains open for the initiative's eventual CI gate; no push/remote CI claimed.

- `CompositeDomain` replaces the composite Boolean pair with None/Values/Intents/ShieldedReceive/FieldObservations; ADR0195 receive retains its intent-only helper routing and ADR0194 phase_reset remains independent.
- The new small structural audit inspects all values/branches/bindings/calls; shared Plan remains the only evaluator. Field helper StateReturn::CellRead is normalized only in this audited domain. Runtime ABI48/schema20 unchanged.
- Seven exports now expose typed recorded/observed APIs. All23 independent TS/native/recorded/replay cases pass: direct/scalar-helper/struct-helper equivalence, ordered distinct reads, selected branches, empty optional branch, exact gas rejection and preserved empty private transcripts.
-19 unit +13 CLI +153 renderer tests,34 Python tests, seven-package strict Clippy and all172 fixture freshness pass. No older generated fixture changed. The old immutable compiler fails the positive source cohort on the six previously missing recorded APIs; the new source/proof metadata join is7/7.
- Eight2912-byte proofs verify independently, reject changed bindings and apply under **default ledger strictness with separate pinned Night-backed Dust fees**. Optional(false) refuses preparation with exact EmptyTranscript. Explicit prior contract states do not establish a deployment lifecycle.

#### Gas evidence
One read: readTime170000000, computeTime1249914853. Pair: native/recorded sum readTime340000000, computeTime2499829706; TS wrapper last-query cost170000000/1249914853; concatenated replay340000000/1425343237. Empty optional execution costs0, while querying an empty replay program itself has base compute1074486469; transaction preparation still refuses the empty transcript. No normalization hides these differences.

#### Exact receipts and reproduction
Final head35aeb6b4 is clean. Five-source receipt (new observations plus existing composite values/intents, receive and original microDAO): `${LOCAL_EVIDENCE}/compact-adr201-strict-delivery/receipt.json`, SHA256 `3fb6a9721fcc0b723cbb9f5357bfc784646dc1f96787a2f0f4679ec42c38e227`;17/21 proof-required recorded, four existing microDAO gaps retained.
Strict proof receipt: `${LOCAL_EVIDENCE}/compact-adr201-strict-proof-receipt.json`, SHA256 `9c43e9b65349debd0d5d9cc65acc46ebf8fae8e1e24d7c99e9c7e030f81ba76e`; includes exact signed head, source/generated crate/proof executable/key hashes, command and default-strict policy. Retained proof artifacts `${LOCAL_EVIDENCE}/compact-adr201-proof`, log `${LOCAL_EVIDENCE}/compact-adr201-strict-exact-proof.log`.

```sh
MIDNIGHT_LEDGER_TEST_STATIC_DIR=${MIDNIGHT_LEDGER_SOURCE}/ledger/static \
CARGO_TARGET_DIR=${COMPACT_SOURCE}/target/compactc-consumer \
CARGO_INCREMENTAL=0 cargo +1.99.0 run -p compact-rust-proof-smoke -- \
  --field-observation ${LOCAL_EVIDENCE}/compact-adr201-proof
```
Shared emitter lease released to ADR0202.


### Completed full composition checkpoint — 7a311c9f (2026-10-06)

**The full local gate passed in one attempt:365commands,172fresh fixtures**, format/rejection/source gates, workspace and generated integration tests, strict workspace Clippy, consumer/proof/ledger acceptance. `${LOCAL_EVIDENCE}/compact-full-7a311c9f/receipt.json`. The source was frozen at `7a311c9f`; only the same pre-existing user documentation edit was present before/after. The final proof stage took853.578seconds. No prior failed-attempt splice is needed for this checkpoint.

Full fixture subset:351/362proof-required recording APIs available and54nonproof exports,11known gaps. Separate whole-source inventory `${LOCAL_EVIDENCE}/compact-7a311c9f-inventory.json`: **366/377available**,213sources/746exports/192compiledroots/369nonproof, same11gaps and zero unassessed/missing/unmatched/baseline drift. These differently scoped counts are not universal language or production parity.

ADR201 is integrated (`fb4c2e24`,strict proof correction`7a311c9f`):7readonlyFieldcomposite APIs,23TS/native/recorded/replay scenarios,8nonempty default-strict separate-Dust proof/application cases, optional-empty exact refusal. ADR199 strictwallet42→contract42 acceptance is retained in the full gate via`305df36a`. Historical smoke cases still use their documented policies; this is not a claim that every old proof fixture is fully funded.

**Clean same-head portable build and archive verification pass**, aarch64-darwin only. `${LOCAL_EVIDENCE}/compact-7a311c9f-portable-final/receipt.json`; archive`${LOCAL_EVIDENCE}/compact-7a311c9f-portable-final/compactc.zip`; SHA256`018caa7e52355acf526e7f808d774463613453a6738859320430a5dfe0c19dca`. All6binary linkage checks, relocated path with spaces, relative installer symlink, defaultTS, strictRust, exact bundled runtime sources including199, offlineconsumer, bundledzkirkeys and7newFieldobservations pass. No compiler/runtime environment override was used. Package `${HISTORICAL_NIX_STORE}/q18vwvgmyfiy8dzviypikyw7ck7aqcr4-compactc-binary-dist`.

Combined evidence `${LOCAL_EVIDENCE}/compact-7a311c9f-integration-receipt.json`. This supersedes ee912835 as the latest full/package checkpoint. ADR202 is signed/reviewed and queued for focused integration;203 implements qualifiedsend,205 researches canonicalallocation,204 prepares dependenttransient vertical. No push, publication, remote CI, other-platform or production-completion claim.
