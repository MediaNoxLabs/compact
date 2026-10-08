---
id: RUST-ADR-0251
alias: ADR-0251
source_sha256: 2d5c419f0d5d2c7f84f5fbb0adf9c198aa45afd8ee7ebcedc0954762b1ebf628
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0251 — Compose audited Unit helpers over complete ledger paths

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted-implementation-in-progress. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

```yaml
status: accepted-implementation-in-progress
date: 2026-10-07
milestone: "0.3.0"
issue: https://github.com/MediaNoxLabs/compact/issues/376
```

## ADR-0251 — Compose audited Unit helpers over complete ledger paths

### Problem and decision
Original DID deactivation needs recording composition of existing native capabilities. Use a bounded typed Unit composition domain classifying declaration-resolved PureValue, LocalUnit and RecordedUnit calls. Whole reachable body audit must precede lowering, including unused bindings and unselected branches. Reuse existing typed Plan and RecordingFrame::call_local; preserve cryptographic semantics in upstream-backed generated native helpers. Preserve full validated chunked slot paths and lexical/call order. No name whitelist or implicit audit bypass.

### Before / after
```rust
// Before: root native call exists; recorded API refuses a nested helper.
ledger_contract::deactivate(context, witnesses, signature, version)?;
// After: same public source call is recordable through an audited composition.
ledger_contract::recorded::deactivate(context, witnesses, signature, version)?;
```
Illustrative signatures; actual source identities and APIs remain authoritative.

### Ownership and compatibility
Emitter composition audit owns call classification, declaration type/scope checks and public-effect ordering. Runtime owns metered execution, witness output ordering and replay. Existing native EC/hash/wide-integer helpers and upstream ledger/zk retain their semantics. No ABI beyond current50 or IR beyond20 is anticipated. Extract helper discovery/planning as one cohesive component, leaving legacy profile restrictions intact. Static source audit plus trusted generated helper execution is the intended boundary; arbitrary Rust callbacks are not sandboxed.

### Scope, alternatives and acceptance
Start with flat/chunked scalar Cell/Counter helpers, then imported Schnorr, then unchanged original deactivate. Do not assume original support from the reduction alone. Require TS/native/recorded parity, hidden-effect/scope/cycle/type negatives and an actual original ZKIR proof/verify/application with separate Dust. Preserve all original assertions and source bytes. Other eleven exports remain explicitly open until adopted. Defer collections and wallet effects. Reject a second cryptographic evaluator and contract-specific pattern checks. Detailed grammar, ordered failure matrix, full slot paths and proof gate are captured in the research below.

### Accepted detailed implementation proposal

## First DID recording composition vertical

Read-only proposal, 2026-10-07. Repository inspected at `77c9ee144f3c326f29cb44a69b7174c5d81b8945` plus current shared work. No compiler changes, new probes, key generation or builds performed for this research.

### Recommendation

Implement a **typed Unit helper composition domain over scalar Cells and Counters**, with three declaration-derived call kinds: pure values, audited local Unit helpers, and recorded Unit helpers. The first real acceptance target is the unchanged original DID `deactivate` export. Use the existing typed Plan and the runtime's existing `RecordingFrame::call_local`; do not create a second expression evaluator or a DID-name gate.

This is a composition gap, not a request to implement Schnorr cryptography again. `recordUpdate` already records when synthetically exported. The Schnorr verifier uses the expression/action kinds already enumerated in `audited_local` (including wide unsigned casts, EC operations and its typed reduction witness). Its verification logic can remain in the existing generated native helper, invoked only after the complete helper graph is proved free of public effects.

### Evidence and original execution order

Inputs examined:

- `/tmp/rust030-adoption-baseline/did/first-gap-families.json`: explicitly a first diagnostic/syntactic inventory, not a full transitive capability proof.
- `helper-probes/results.json`: `recordUpdate` becomes recorded when only its export keyword changes. `assertController` fails at its first Assert; `assertControllerCanUpdate` traces that same dependency. Several other synthetic exports fail the frontend's legitimate disclosure checks, so those results do not prove Rust gaps.
- `output/contract/compact-rust-ir.json` and `output/contract/lib.rs`: complete original helper IR and native output.
- Original source hashes: DID `632f34af543924edb185fe9c8eda54aa4ca0503b0b9e8ee3ff06ec470f6d7456`; Schnorr `f072731d730d72f8b76b183df2c5187b233a0838d9462894cfb0a6d2f0f66bff`.

Original `deactivate` is at copied `did.compact:833`; it has no root Let:

1. Materialize signature and expected version arguments; read `id`; compute the nested pure authorization digest **before entering** `assertController`.
2. `assertController` (`did.compact:396`) reads version and asserts equality. Only then read controller public key and call `schnorrVerifyDigest` → `schnorrVerify`.
3. Local Schnorr runs its reduction witness and asserts quotient bound, exact reduction equation, then EC signature equality. It must not contribute public VM queries itself. A witness callback's permitted metered ledger reads remain private witness activity under the existing runtime model.
4. Read/assert `active`; write active=false, deactivated=true.
5. `recordUpdate` (`did.compact:440`) increments operationCount, increments version, invokes currentTimestamp, then writes updated.

Do not move the active guard before signature verification or version checking. Do not hoist timestamp before the writes/counters. Timestamp is a witness in this source, not an assertion against block time; preserve that distinction.

#### Hidden additional boundary: original chunked ledger

The original has >15 fields. Its slots are `controllerPublicKey=[0,1]`, `id=[0,3]`, `version=[1,1]`, `updated=[1,3]`, `deactivated=[1,4]`, `active=[1,5]`, `operationCount=[1,6]`. Many share the IR's first index. Current `Plan::field` rejects `physical_path().len()!=1` (`typed_plan.rs:323`). A flat reduced fixture alone would conceal this second gap.

Use the existing globally validated LedgerField declaration and emitted slot constant, preserving its complete physical path. Domain-scoped allowance for supported validated nested paths is sufficient; do not weaken every existing profile's field gate. Never identify a slot solely by `index`, and do not invent flat aliases or hand-code VM indexes. Existing CellSlot/CounterSlot methods already accept the full path (`runtime-rs/src/slots.rs:503,590,597`).

### Current boundaries to reuse

- `recorded/typed_plan.rs:1081`: resolves pure vs stateful declarations before profile admission; ambiguity/missing declarations reject.
- `typed_plan.rs:1296`: shared inline_call evaluates caller arguments once, then establishes a fresh typed callee scope and cycle guard.
- `typed_plan.rs:1365,1407,1481`: typed arguments, lexical bindings, ordered actions and branch-local frames.
- `typed_plan.rs:670,873,1636`: metered witness encoding, Counter reads, typed Counter increments. Counter operands are literal/u16 parameters; source-generated Let aliases must retain type checking.
- `recorded/audited_local.rs:22–176`: exhaustive effect-free native helper audit. Its current renderer (`:185`) is a narrow guard → helper → optional increment shape, which does not match original deactivate. Expose/factor the audit without broadening that old renderer.
- `runtime-rs/src/recording.rs:207`: call_local checks unchanged public query/state/effects, whole Zswap state/plan, identity and execution policy; then adopts private outputs in order and measured witness gas. This checks returned state, not arbitrary transient mutations of malicious Rust callbacks. Generated helpers plus complete static audit are the intended trust boundary.
- `recorded.rs:293`: existing recursively recordable Cell types; native codecs and emitted declared Rust types remain authoritative.

### Bounded component design

Use a small internal audited-call representation, not more loosely coupled booleans:

```rust
// Private constructors: produced only by the closed audit.
enum AuditedCall<'a> {
    PureValue(&'a PureCircuit),
    LocalUnit(&'a StatefulCircuit),
    RecordedUnit(&'a StatefulCircuit),
}
struct UnitComposition<'a> { /* typed declaration maps + audited call classification */ }
```

This need not become a whole-program effect framework. Classify the reachable graph once per candidate; retain an active-call stack and memoized completed results. Complete all bodies, unused Let initializers and both branches before emitting. A call with public reads but no writes is still RecordedUnit, never LocalUnit.

**First closed public-effect grammar:** Sequence; lexical Let; Assert; Unit helper calls; typed CellRead/CellWrite; CounterRead/CounterIncrement; and conditional branches if the same audit validates both arms. Require Unit return and at least one structural public query in the exported entry. This prevents advertising an intent-free/local-only export as proof-required recording merely because a local helper runs.

**First typed value grammar:** parameters/literals/coercions, typed structural projection, equality/Boolean branch expressions, named pure calls and witnesses. Public Cell reads needed here are Boolean, Uint64, JubjubPoint and finite struct/tuple/vector values over Field/Boolean/fixed bytes/unsigned<=64/JubjubPoint. Public writes can initially remain Boolean/Uint64 (covers deactivate and recordUpdate). The UInt16 Counter operand is a separate checked type. No opaque containers, collection mutations, Kernel/Zswap effects, NativeWitnessCall, unknown expressions or effectful return plans in this profile. Do not admit an otherwise unsupported node because its branch is currently unselected.

**PureValue:** evaluate arguments with the shared Plan, materialize them once in caller order, check exact declared types/arity, then invoke the generated pure function. Audit every reachable pure body for forbidden effects and validate its result through the existing typed pure renderer. Add needed Vector and typed struct/transient-hash closure support to this *separate* audit; current Plan::pure_value does not cover the vector authorization digest. Do not toggle a shielded profile to gain its unrelated expression permissions.

**LocalUnit:** resolve a stateful Unit declaration; recursively audit all actions and local helper callees using the existing audited_local effect grammar and type validation. Evaluate arguments via Plan first, so `controllerPublicKey` is recorded outside the local call. Emit `frame.call_local(|context| super::helper(context, witnesses, args...))?`. Keep Uint248, reduction arithmetic and EC operations inside the already-supported native implementation; no wide-arithmetic extension to Plan is needed for this vertical. Keep pure calls inside LocalUnit refused unless a separate complete pure audit is explicitly reused; the actual Schnorr helper graph does not require them.

**RecordedUnit:** inline its ordered actions into the same RecordingFrame through shared Plan::inline_call with Unit tail. Enable generic typed Unit signatures for this domain, including zero-argument recordUpdate and composite arguments for assertController. No exact parameter-count, operation-count, source-name, circuit-name or witness-name whitelist. Retain old profile rules unchanged.

The new domain should consume this audited classification when dispatching calls; do not repeat fresh syntax pattern matching during lowering and accidentally route a failed audit to native execution.

### Minimal source reductions

These are proposed test source sketches, not newly compiled evidence. Keep the original source as the final gate.

#### A. Scoped Unit helpers with public effects

```compact
import CompactStandardLibrary;
ledger active: Boolean;
ledger version: Counter;
ledger updated: Uint<64>;
witness now(): Uint<64>;
circuit guard(expected: Uint<64>): [] {
  assert(disclose(expected) == version, "stale");
}
circuit note(): [] {
  version.increment(1);
  updated = disclose(now());
}
export circuit close(expected: Uint<64>): [] {
  guard(expected);
  assert(active, "inactive");
  active = false;
  note();
}
```

Initialize active=true in a normal constructor. Add a variant with enough unrelated fields to induce chunking and a renamed/reordered declaration variant. Same semantic permissions, no special source identity.

#### B. Public argument evaluation before audited local verification

Use the actual imported Schnorr module (no edited substitute verifier), one public key Cell, one Counter, Boolean active and timestamp Cell. The root calls a recorded authorization helper with a typed signature/Uint64/pure Vector<4,Field> digest; that helper reads version, then passes a recorded public-key read into local Schnorr. Minimal digest helper composes a struct transientHash and vector constructor from arguments. Finally close/update as in A. This isolates all three call kinds without the unrelated DID maps/sets.

#### C. Unchanged original

Compile exact original DID and imported Schnorr hashes with recorded deactivate required. Preserve the other eleven missing exports as independently reported gaps unless they actually become admitted by the new structural domain and receive appropriate evidence. Do not claim all DID recording support from this one export.

### Acceptance and negative matrix

1. **Frontend/native baseline:** compile the sketches and unchanged original; independent corrected/pinned TS captures. Preserve constructor state and original ledger chunking. Native exported deactivate is already present; compare its unchanged behavior with newly recorded output.
2. **Success parity:** deterministic valid controller signature for correct contract id/version, real reduction witness and timestamp. Compare final ledger, private state, exact ordered public VM program, complete private FAB outputs, per-query/execution gas and independent replay gas. Unrelated 12+ ledger fields remain unchanged. Use ContractLab for local sequence/fork/failed-call rollback; no transaction claims from that report alone.
3. **Failure ordering:** stale version consumes neither reduction nor timestamp; wrong signature/reduction fails before active query; already inactive with otherwise valid current authorization consumes reduction but not timestamp; injected timestamp error happens after attempted flags/counters, yet owned scenario rollback retains the prestate. Observe Counter max behavior from actual TS/ledger rather than assuming checked overflow or wrap. Test max operationCount and max version independently, retain actual prefix/order.
4. **Signature negatives:** wrong controller, wrong contract-id or version-bound digest, quotient 116 vs allowed 115 boundary, incorrect reduction equation, malformed/canonical point/scalar input boundaries through actual existing encoders. Do not redefine canonical EC input semantics or synthesize valid signatures by bypassing original verifier.
5. **Typed/audit mutants:** pure helper containing hidden Cell/Counter read; local verifier containing public read/write inside unused Let or unselected branch; pure/stateful declaration collision; missing helper, wrong arity/argument/result types; cycles; caller parameter/name shadowing, helper-local/branch/earlier-sibling scope escape; field name/index/path mismatch; branch that adds Kernel or Zswap intent; duplicate Rust identifier normalization. Each refuses at a concrete source/IR path; existing supported domains remain unchanged.
6. **Actual proof gate:** one original valid deactivate invocation from a clearly identified constructor/seeded DID state, prove against the original ZKIR, verify and apply under unchanged default strictness with separate Dust fees. Capture actual transcript guaranteed/fallible partition first; do not assume it. Assert flags, both counters, timestamp, unchanged unrelated fields and exact subsequent failure on stale/inactive replay. No shielded offer is needed by this source. A changed-prestate negative must assert the actual upstream outcome, not broad `.is_err()`.
7. **Capability gate:** new reduced and original exported rows cross-tab against compiler proof applicability. Query-free local helpers stay local; empty recording is not made proof-ready. No relaxed strictness, fabricated queries, or removed original assertions.

### Cohesive extraction while doing this work

Move helper planning as a component from `recorded.rs` to `recorded/helpers.rs`:

- `helper_ident` (currently ~1983): stable generated helper naming/collision logic;
- `collect_field_callees` (~2028), `collect_shared_callees` (~2044);
- `plan_recorded_helpers` (~2075–2229): candidate discovery, dependency closure, render viability and item emission.

Keep `render_recorded_circuit`/`render_recorded_helper` thin public-internal adapters and `render_recorded_item` orchestration in recorded.rs initially. Existing helper planning behavior should move verbatim first, with old tests retained. The new composition audit can live alongside it (`recorded/composition.rs` or a typed_plan child), while `audited_local` becomes an explicitly reusable private effect audit plus its old adapter renderer. Do not move unrelated asset/opaque-map/curve special-case functions merely to reduce line count.

If implementing same-frame inlining makes old shared-helper emission unnecessary for this domain, do not force new helpers through the old collector's Field-only traversal. Its existing domain remains valid; new typed composition performs its own complete declaration audit. Preserve stable helper names/public paths for legacy output.

### Scope and expected compatibility

No new VM opcodes, no runtime leaf, no generated dependency API requirement: IR20/runtime ABI50 should remain unchanged unless implementation exposes an actual new requirement. Existing RecordingFrame, slot path, typed witness, EC native and pure generated APIs suffice. Proof support must be demonstrated, not inferred from successful Rust compilation or call_local guards.

The initial implementation should not absorb DID Map/Set mutations, Schnorr verification through MapLookup, key rotation, wallet policy, durable ContractLab persistence or a broad effect-system redesign. Those can reuse the composition boundary later with their own audited effect extensions.

### Local delivery —2026-10-07

`675547556ed5d841d07d38ad3ee980f8e0efaa53` delivers this bounded slice.230backend+30fixturetests,28independentTS cases,182freshfixtures,strictClippy and original3296-byte deactivate proof/strictDustledgerapply pass. Other11statefulexports remain gaps; constructor-derived seeded prior state is not full lifecycle proof. False empty branches replay but refuse proof preparation with EmptyTranscript. Missing source active guard accepted only by the new generic domain with no invented guard; legacy profile retains its refusal. Three legacy helper renderings inline with equivalent independent tested behavior. [ADR0251 — Unit composition and DID proof receipt](references-0.3.0.md#note-018).
