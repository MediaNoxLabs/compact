---
id: RUST-ADR-0259
alias: ADR-0259
source_sha256: 6bba5aae024a87ae55008be48765cc8fd34e32fdcf59bb82885f1b118da158cb
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0259 — Record Point authorization transitions through shared composition

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted implementation plan. Implementation and acceptance evidence pending. Parent reserved ADR0259. Source at root commit `012fbb44`; frozen compiler `/tmp/compact-adr257/bin/compactc`, same existing Scheme, runtime ABI 50 / IR 20. `baseline.json` contains actual source compilation and capability evidence; `compiled/*/contract/compact-rust-ir.json` contains complete actual IR. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0259 proposal — record DID Point authorization transitions through shared composition

Status: accepted implementation plan. Implementation and acceptance evidence pending. Parent reserved ADR0259. Source at root commit `012fbb44`; frozen compiler `/tmp/compact-adr257/bin/compactc`, same existing Scheme, runtime ABI 50 / IR 20. `baseline.json` contains actual source compilation and capability evidence; `compiled/*/contract/compact-rust-ir.json` contains complete actual IR.

### Decision and precise scope

Extend the existing `UnitComposition` declaration audit and shared typed `Plan` for: (1) Jubjub Point coordinate projection to Field; (2) inequality of two Fields yielding Boolean; (3) Point Cell writes in that composition policy. Reuse the upstream-backed runtime operations and typed slot recording. No new evaluator, DID-name gate, profile flags, crypto implementation, runtime API, schema or ABI change is expected.

Target two additional unchanged original DID v0.7.0 exports: `rotateControllerKey` (line 577) and `recoverControllerKey` (line 595). Keep all other recording domains intact. Original DID currently has 1/12 recorded stateful exports (`deactivate`); proposed acceptance is 3/12, subject to actual fresh capability results. Remaining collections/verification-method exports remain separate work.

#### Important baseline correction

Point writing is **already supported** by older recorded paths. The actual reducers confirm standalone, helper-forwarded and guarded Point writes record today; a witnessed Boolean helper followed by a Point write also records. This is not a missing runtime primitive or blanket Point support gap. `composition::write_type` excludes Point, so the **new complete composition path** must permit that already-supported typed operation when it also includes coordinate/digest/authorization helpers. Preserve the current successful lowerings unless the newly admitted graph deliberately changes selection; inspect any resulting output diff.

### Actual minimal Compact probes

All files below compiled successfully with the current compiler. The first attempt at the lazy guard omitted source `disclose` and was correctly rejected by the frontend; the retained final reducer declares disclosure explicitly, matching original DID.

| Actual source | Export | Current recorded | What it isolates |
|---|---|---|---|
| `reducers/point_store.compact` | `direct_control` | yes | Existing direct Point Cell write |
| same | `through_helper` | yes | Existing forwarded write |
| same | `guarded` | yes | Boolean read/assert helper then Point write |
| `reducers/point_digest.compact` | `scalar_control` | yes | Existing Boolean witness helper plus Boolean write |
| same | `point_write_only` | yes | Witness helper plus Point write through existing paths |
| same | `pure_projection_only` | no | Point coordinates in typed pure transient-hash helper, Boolean write |
| same | `update` | no | Same pure helper plus Point write |
| `reducers/point_guard.compact` | `check` | no | Two source-ordered lazy Point inequality guards |

Representative source, already compiled:

```compact
pure circuit digest(value: JubjubPoint): Field {
  return transientHash<Coordinates>(Coordinates {
    x: jubjubPointX(value), y: jubjubPointY(value)
  });
}
circuit checked(message: Field): [] {
  assert(active, "inactive");
  assert(disclose(authorize(message)), "unauthorized");
}
export circuit update(value: JubjubPoint): [] {
  checked(digest(disclose(value)));
  key = disclose(value);
}
```

The lazy reducer uses `const visible = disclose(value); changes(visible); distinct(visible);`, and each helper uses the same two coordinate comparisons as the original source. The full source files, not hand-built substitute IR, are retained beside this note. For production coverage, keep these compact reducers or consolidate their independent controls into one focused Point-composition fixture without importing DID-specific policy.

### Exact IR and ordering

Original `assertControllerPublicKeyChanges` at source line 424 is:

```text
Assert(
  If(
    NotEqual(JubjubPointX(Parameter(new)), JubjubPointX(CellRead(controller))),
    Boolean(true),
    NotEqual(JubjubPointY(Parameter(new)), JubjubPointY(CellRead(controller)))
  )
)
```

**Both `NotEqual` operands are Field; its result is Boolean.** The source `||` has become `Expr::If`; there is no new Boolean inequality or OR opcode to invent. The second branch includes a second Point Cell query, not a reused read. The recovery-distinct helper has the same shape against the recovery Cell. All original DID fields here share index 0 with their actual chunked physical paths; keep the declared slot descriptors, never reconstruct a flat index.

Original rotate/recover graph order:

1. Materialize disclosed new Point local.
2. Evaluate authorization arguments, including id Cell read and pure digest of contract id, expected version, operation domain, and the new Point coordinates.
3. Authorize: current version query/assert, controller or recovery Point query, audited local Schnorr reduction witness/verification, active Cell query/assert.
4. Current-controller change guard, then recovery-authority distinctness guard. Preserve lazy coordinate queries and assertion precedence.
5. Write new controller Point through its declared slot.
6. `recordUpdate`: operationCount increment, version increment, timestamp witness, updated Cell write.

The original current native oracle (`tests-rust-backend/did-adoption/oracle/lifecycle.json`, `authorization-lifecycle`) already records: successful rotate/recover each 10 queries and reduction→timestamp witnesses; wrong recovery signature 3 queries/reduction only; same-current rejection 6 queries/reduction; recovery-equal rejection 7 queries/reduction. These are existing independent TS observations, not new execution claims from this research. Match them before adding proofs. Do not claim aggregate TS wrapper gas when only its last-query field is exposed; sum individual captured query costs and retain replay cost separately as in ADR0251.

### Minimal production changes and ownership

1. `tools/compact-rust-backend/src/recorded/typed_plan/composition.rs:38`: include exactly `Type::JubjubPoint` in allowed Cell writes. Keep Boolean/Uint64 behavior, declared Cell/index/physical-path validation, and no arbitrary struct writes.
2. Same file `Audit::value` at line 67: recursively audit `JubjubPointX/Y` in both stateful and pure contexts. The pure context still prohibits nested ledger reads, witnesses and effectful calls. Its existing `expression_with_calls` check owns coordinate input typing and declared pure result correctness.
3. Same audit: allow `NotEqual` only in non-pure composition expressions for this slice; recursively audit both operands. Shared typed Plan must require both actual operand types to be Field. Do not broadly admit generic inequality over Boolean/struct/Uint or add an unnecessary pure inequality domain. Actual pure authorization helpers only need projections/hashes.
4. `typed_plan.rs` expression dispatch (current funded-mint-only `NotEqual` at line 531): add a composition-specific typed Field inequality leaf alongside, preserving the existing funded-mint branch. Evaluate left then right once and bind a Boolean. Reuse current ordinary Rust comparison over runtime Field.
5. Same dispatch: add composition-specific `JubjubPointX/Y` leaf. Evaluate operand once through existing shared Plan (thereby recording any real Cell query in order); require exact Point type; invoke existing `runtime::jubjub_point_x/y` and bind Field. Existing lazy If at line 637 owns branch-local frame joins unchanged. Small private leaf methods are preferable if adding arms inflates the large expression method's debug frame; no new planner or broad abstraction module.
6. Existing action path at `typed_plan.rs:1570–1627` already delegates typed writes through `slot.record_write` after exact value/slot type equality and `composition::write_type`; no new runtime operation. Existing generic slot runtime `slots.rs:510`, Point coordinate adapters `natives.rs:93/98`, pure coordinate renderer `lib.rs:1984`, and local Schnorr audit are reused unchanged.

Keep `Audit::call` declaration-directed routing, whole-graph/unused-binding/unselected-branch checks, cycle checks, `Plan` argument materialization, fresh callee scope, and actual Unit helper requirement. Do not move source checks across authorization. No native emitter or constructor change is part of this decision.

Before generated API: native `Contract::rotateControllerKey`/`recoverControllerKey` exist; recorded/observed methods do not. After: ordinary existing recording facade gains `recording().rotateControllerKey(...)` and `_call(&observed, ...)`, likewise recover, with the same typed Point/Signature/Uint64 parameter order and FAB layout. This is illustrative expected output, not a compiled implementation claim.

### Focused acceptance matrix

#### Reducer and structural tests

- Keep all five current successful controls above successful. Both missing pure-projection variants and lazy guard should gain recording through the shared planner. Confirm declaration renaming and inline/helper forms retain source behavior, rather than relying on DID names.
- Valid Point read/projection → Field inequality → Boolean lazy condition; valid Point Cell write; identity coordinates use existing runtime identity (0,1) behavior. Use valid canonical subgroup points. Equality forces the Y branch; distinct-X success omits it. Do not invent non-subgroup points solely to force an impossible branch.
- Malformed IR: project Boolean/Field/struct as Point; inequality mixed Field/Boolean or Field/Point; same-typed non-Field inequality still refused by this narrow profile; Point write to Boolean/struct slot; wrong slot index/path; misdeclared pure helper result; wrong call arity/type; unbound/escaped local; recursive pure/stateful calls.
- Hidden public read/witness in pure projection operand, including unused binding and unselected branch, must refuse. Hidden unsupported action in a stateful unselected branch must refuse; supported but unselected branch must not execute reads/witnesses.
- Ensure funded-mint inequality tests and earlier unit-composition Schnorr safety tests remain unchanged.

#### Original source TS/native/recorded/replay

Reuse the reviewed ADR0255 native oracle and add recorded comparison, preserving source/import bytes and distinguishing current branch runtime profile from the original package's historical 8.1 dependency graph. Compare typed result, full public state/effects, private state, actual ordered private outputs/witness arguments, every query/program and per-query/summed gas. Exact errors and successful prefix observations for equal/current/recovery keys, wrong authority, stale version, inactive state, timestamp failure, malformed state, and Counter overflow stress controls. ContractLab rollback means no committed lab snapshot after failure; it does not undo external callback effects or invent inaccessible post-error gas/context.

Run backend tests, focused original/reducer fixture suites and strict Clippy; current corpus freshness/capability diff must identify all newly recorded APIs and leave native function bodies unchanged. No unrelated profile enablement or proof-applicability change. ABI 50 / schema 20 remain unless actual new emitted runtime requirements demonstrate otherwise.

### Minimum meaningful strict sequential ledger gate

The old `did_deactivate.rs:44–49` seeds an already-created ContractState directly into the ledger. That existing evidence remains valid but does **not** satisfy this gate. Also, shared `main.rs` deploy/call helper currently sets `enforce_balancing=false`; do not reuse that weaker policy for this acceptance.

1. Generate/retain keys for unchanged original `rotateControllerKey`, `recoverControllerKey`, `deactivate`. Prepare built-in proof material/SRS through existing authenticated preparation, and use the established separate Dust funding fixture (fee funding only).
2. Invoke the **original generated constructor** with deterministic synthetic test witnesses. Build one actual `ContractDeploy` whose operation map contains all three verifier keys. Preserve complete constructor result bytes; never seed/patch the contract state or its id afterward.
3. Build/prove/seal/Dust-balance the deployment transaction, validate default `WellFormedStrictness`, and actually apply it to the funded ledger. Obtain the deployed contract only from the resulting ledger. Deployment validates supplied initial data; it does not prove constructor execution.
4. Record/sign/prove/default-strict/apply rotate using controller key 1→5. Observe the actual applied state; record/sign/prove/apply recover with recovery key 3 to controller 7. Observe again; record/sign/prove/apply deactivate with key 7. Reuse retained ledger state and proof provider, no contract-state insertion/reset between calls. Compare every resulting state to the native/independent TS sequence; operationCount/version advance exactly once, updated timestamp exact.
5. Negative controls use clones/forks of real applied states: old controller signature after rotate, wrong recovery authority, stale version before verification, same-current/recovery-equal key; retain exact source error/prefix. Prepare a valid call then change its input/state binding and assert a specific proof or ledger refusal. Replay an accepted transaction and retain the actual precise rejection, not generic `Err`. Failed attempts must leave the accepted ledger/public/private snapshot unchanged.

The strict sequence gives three original call proofs plus actual default-strict deployment, without a full DID collection lifecycle. It proves execution of these source transitions under the selected runtime profile, not DID security or cross-network interoperability.

### Constructor-id and version boundaries (mandatory labels)

The original constructor stores `kernel.self()` under dummy-constructor semantics. Do not rewrite that id to the eventual random deploy address, and do not sign a different digest to make the test appear safer. Record both stored id and actual deploy address; signatures use the exact stored id/source digest, while observed call and proof bind the real deployment address. A separate authorization investigation was stopped by an automated tool review; its preliminary observations are unverified and are not resumed by this slice. This recording slice neither fixes nor endorses the source authorization design.

Current profile remains pinned Rust ledger 8.0.3 and branch TS/runtime acceptance. The coherent 8.1 scratch compile/goldens are separate ADR0252 evidence; this slice does not promote those dependencies or claim universal raw-Field EC provider parity.

### Implementation sequence

After parent records ADR/issue: (a) add checked-in small source/IR negatives and independent capture; (b) extend only shared audit/typed leaves; (c) original recorded parity and exact capability/freshness; (d) real deploy→rotate→recover→deactivate strict gate; (e) independent review, receipt and parent integration. Retain the documented constructor identity limitation; do not resume the blocked authorization investigation or patch the original contract. Stop and report any new operation outside the inspected closure rather than broadening the profile speculatively.

### Local delivery

`51b9ea435f32d7ce7fef767e33d4259f84ae568b`.298backend/32DIDtests,strictClippy,182freshfixtures.21independentTS/native/recordedcases; actualdefault-strictdeploy→rotate→recover→deactivate with3verified3296byteproofs and exact same-time IntentAlreadyExists replay refusal. Source/constructor preserved;3/12statefulexports recorded/proven, nine remain. Currentzkir2.1.0rows1930/1930/1804; prior1780deactivateprofile unresolved despite identicalZKIR, no source-growth claim. [ADR0259 — Original DID Point proof and recording receipt](references-0.3.0.md#note-026).
