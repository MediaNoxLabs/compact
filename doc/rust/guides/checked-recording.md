# Checked recording and source diagnostics

**Publication draft at signed `34ed8528`; final CLI/history closure remains pending.** The earlier `9faee816` checkpoint changed only six native Zswap test expectations; `532927b3` continued the current test-oracle corrections. The current checkpoint also changes compiler resource bookkeeping through private `TypeBoundsSession`. Earlier failed gates remain failed receipts.

A later signed current-oracle checkpoint `532927b3` corrects only the wide-add and native own-public-key qualification expectations. The two direct helpers passed their four original compiler-generation commands; all proof-false / not-applicable rules, IR, signature and value checks remain unchanged. This is a test-only continuation of ADR0499, not a compiler/runtime behavior change or a complete CLI pass. The earlier failed CLI receipts remain preserved.

## What owns support

Pure expressions use one exhaustive shared semantic fold. NativeSink turns checked facts into NativeFragment syntax; BodyCheckSink has unit Value/Binding and CheckSink carriers. Formal signature/body checking uses the same rules and separate representation obligations, without a production native-syntax bridge. Stateful native interpretation and typed recording lowering remain distinct consumers; syntax-free checking does not make every native-valid program recordable.

Recording lowers source operations into typed values, blocks and effects, independently validates the plan, then renders a privately constructed CheckedPlan. Ordinary admission uses `profile_selection::select_checked_observed`, then `effect_plan::lower_with_calls_observed` → `check_with_calls_observed` → `validate::check` → `render`. These current entries live in `recorded/profile_selection.rs` and `recorded/effect_plan.rs`. The old `lower_with_calls_diagnostic` and `check_with_calls_diagnostic` wrappers are `#[cfg(test)]` historical adapters, not the production call path. A previously checked early Effectful plan can be admitted at its preserved boundary. Unsupported produces a typed recording refusal; Invalid becomes RecordingPlanInvariant. Historical profile selectors, expression allowlists and complete-body recognizers are test oracles rather than production acceptance fallback in this route. Optional checked-plan sharing cannot change acceptance.

Sequence/calls keep source order and first failure. Let initializers use the previous scope. Branches check both static arms but execute only the selected arm; equal arms, unused bindings and zero-length Map bodies retain static obligations. Frame, witness, transcript, intent and preparation rules remain real obligations. A checked plan does not guarantee every input can be prepared, proved or applied.

## Worked extension: existing FieldToBytes32

This is an existing private IR primitive, not a new Compact spelling or new public algebra API. Its conversion owner visits the child once and decides the type before native formatting:

```rust
            let child = children.visit(value, sink)?;
            if child.ty != Type::Field {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Field,
                    actual: child.ty,
                });
            }
            (
                ConversionFacts::FieldToBytes32,
                child.output,
                Type::Bytes { length: 32 },
            )
```

The shared owner passes ConversionFacts::FieldToBytes32 and the prepared child to its sink. Native formatting uses the existing field_to_bytes_32_syntax helper; the unit consumer discards output. Recording uses Operation::FieldToBytes32(Value), checks the operand/result signature independently in the validator, and emits only from CheckedPlan. Add primitives at these rule/sink/validator boundaries, with exhaustive child/type/resource accounting and explicit refusal, rather than a recognizer for an example contract. This excerpt is exact source; it requires its surrounding match/function to compile.

## Where to extend a primitive

Paths below are relative to `tools/compact-rust-backend/src/`.

| Responsibility | Existing source owner | Required extension |
|---|---|---|
| Source variant and exhaustive classification | `ir.rs`, `pure_semantics/expression/source.rs` | Add an explicit supported/refused class; do not hide new variants behind a wildcard |
| Original occurrence/resource accounting | `resource_limits/traversal.rs`, `resource_limits/pure.rs`, `resource_limits/borrowed_types.rs` | Visit each original occurrence, both static arms and typed children under unchanged declared budgets |
| Neutral operation semantics | `pure_semantics/conversion.rs` (for the worked example), `sink.rs`, `children.rs` | Visit children once in source order, decide types/facts, preserve first refusal |
| Native formatting | `pure_native/conversion.rs`, `pure_native/sink.rs` | Format already checked facts; do not repeat semantic checks |
| Formal body checking | `pure_semantics/body_check.rs`, `recorded/effect_plan/pure_certification/checker.rs` | Reuse the same semantic fold; add only separately owned representation/signature obligations |
| Typed recording | `recorded/effect_plan/model.rs`, `lower/expressions/conversions.rs`, `validate/`, `render/` | Lower into a closed typed operation, independently validate operand/result rules, render only checked plans |
| Source refusal capture | `source_diagnostics.rs`, `origins/recording.rs`, `origins/diagnostic.rs` | Observe the actual original borrowed owner/slot at the existing producer; use explicit unavailable for owned/global facts |

Use the existing family tests as the starting corpus, then add focused controls for child order, invalid types, exact refusal/code/resource, equal-but-distinct original nodes, successful output and None/observed parity. Exercise the actual full caller path on the ordinary 2 MiB worker near the relevant existing cap and one over it; a smaller leaf-only worker does not establish that full route. Freeze source/tool/dependency/artifact inputs before qualifying actual generated consumers. Add genuine source and independent TS/runtime cases where applicable; keep compilation, replay, preparation, proof and application receipts separate.

A new recording operation need not provide proof support automatically. If only native execution is supported, report the typed recording refusal honestly. If the frontend excludes a private IR fixture, label it private instead of presenting it as a genuine accepted Compact source.

## Live configured frontend API

The public library exports ConfiguredFrontend, FrontendRequest, FrontendArtifacts, RenderOptions, FrontendRender, NativeCompileFailure, MetadataStatus and SourceDiagnostic. The source CLIs provide the complete publication transaction. The existing executed F1–F3 probe invokes the configured process through:

```rust
    let artifact = ConfiguredFrontend::new(frontend).invoke(FrontendRequest {
        source, arguments:args.collect(), stage_parent:stage,
    })?;
```

The surrounding probe creates the existing staging parent, supplies the source operand and --skip-zk/--skip-ts arguments, and retains fingerprint/IR/raw artifacts. The configured executable receives --emit-rust-ir and an exclusive fresh output child. Only a live invocation constructs this private receipt; importing an arbitrary sidecar directory cannot mint it. Integrity hashes identify artifacts, not a signature or trusted-compiler attestation. A bare PATH executable may have no filesystem fingerprint; an explicit readable executable hash does not eliminate replacement races.

The same probe renders through this exact match header:

```rust
    let result = match artifact.render(RenderOptions {
        apply_frontend_proof_metadata:true, require_recording:false,
    }) {
```

The surrounding match checks originless cause/output parity and records diagnostics. Native rendering runs once inside the facade. Proof applicability can then be joined from retained contract-info bytes; require_recording needs that join and gates proof-required exports missing recorded/observed APIs. A finalized capability report has schema 3; render_with_capabilities alone returns an unclassified schema 2 draft, which apply_contract_info advances atomically. A proof-required flag is applicability metadata, not an executed proof.

FrontendArtifacts::render preserves a boxed NativeCompileFailure with its actual cause, independent metadata status, optional primary diagnostic and recording diagnostics. Native/IR/proof/recording-policy errors retain their priority over metadata presentation. On otherwise successful output, finish_metadata rejects Invalid; Missing/budget metadata remains Unavailable and does not add a native language refusal. Raw IR callers of originless render APIs do not gain authenticated source ranges by providing similarly named bytes.

## Source diagnostics and trust

Callbacks retain actual borrowed original Owner/Slot identities at existing failure producers, synchronously selecting a small structural address. They do not infer provenance from error strings, equal IR values, names or copied type pointers. First-write capture includes an unavailable sentinel, so an enclosing expression cannot replace an earlier unavailable failure. Recording roots reset their own session and retain actual bounded related direct callers. Owned snapshots/summary/global-budget facts without a unique original witness remain unavailable.

Only physical source extents attach a diagnostic; compiler-context virtual EOF ranges are excluded. Positions are end-exclusive **decoded-source character coordinates**, not UTF-8 byte offsets. Provided, inherited and derived provenance labels remain explicit. An inherited declaration range can cover the enclosing declaration rather than the exact identifier token. SourceSnapshotFingerprint/range correspondence describe post-invocation snapshots; they do not attest the bytes consumed by the frontend or the entire imported source closure. Invalid UTF-8 snapshots are unavailable rather than silently replacement-decoded. Metadata Available describes transport/typed binding and a bounded sampled range check; it does not certify every physical range.

Fresh genuine H231 controls preserve the same IR/raw and old native causes for depth and invalid Struct-name refusals, with physical addresses 247 / 13. `Vector<32769, Field>` remains native-successful but records the canonical fixed-layout refusal at Type 23, provided line 4, characters 29–49. Both source CLIs keep the primary error before range notes and preserve previous output on fatal failure. A valid combined target keeps Rust/IR/raw/capabilities/JS/d.ts bytes and map non-sourceRoot fields, resolving the actual source. These six controls establish those selected behaviours only; they do not execute generated applications or proofs.

## Publication duties

FrontendArtifacts::promote_into checks the same invocation fingerprint, pending metadata, and retained IR/raw/contract-info bytes, then moves entries into that invocation's original empty staging parent. It is not final publication and does not authenticate every generated asset. The caller owns lock/rollback/atomic final replacement, rebasing relative source-map paths from the original map directory to the final requested map directory, packaging and affected manifest refresh. Both source CLIs perform relocation explicitly. Do not copy a facade promotion example as a complete transactional publisher.

## Compatibility and evidence columns

Current compiler/runtime ABI is 52, private IR schema 20, completed capability schema 3 and private raw-origin transport schema 3. The optional origin reader is bounded at 4 MiB; missing/budget/numeric-width status is separate from source semantic admission. The current source uses official ledger 8.1.3 primitives and codec/transcript/transaction semantics at revision `b5d3e965a18e4cac17fac4f38446fc55c524439a`. Historical source-law and proof receipts retain their original ledger cohort; the [ledger qualification guide](ledger-8.1.3-local-qualification.md) records the upgraded local proof scope. Explicit ZKIR-v3 Keccak native qualification does not establish ledger 8 proof or later-ledger/network support.

| Column | Required case evidence | Do not infer |
|---|---|---|
| Native | generated code actually compiled/executed, or explicitly emission-only | recording/replay/proof from emission |
| Recorded | emitted API plus actual ordered frame/trace execution | every input succeeds or emits a nonempty public trace |
| Public replay | replay from independent initial state with bound transcript/inputs | witness rollback or preparation |
| Preparation | actual checked intent/identity/partition/empty-trace policies | prove/verify or funded application |
| Proof | actual prove/verify with genuine source/full IR/ZKIR/keys/runtime/prover/profile | proof-required flag, key generation or constructor-data execution |
| Ledger application | actual selected transaction policy, funding/balancing and apply result | deployment/network SDK/security audit |

Label each cell fresh, authenticated reuse, N/A, refused or unproved. Preserve historical 202 outputs / 487 observations separately from the current 257-source registry and 605 applicability exports (545 proof-required / 60 nonproof); these are independent evidence dimensions and not language coverage percentages. Generated fixture and genuine frontend claims must remain distinct.

## Retirement and resource record

Original 84 grouped boundaries have 69 retired production routes, seven retained production semantic/integration obligations and eight historical artifacts. These unequal boundaries are not 84 / 84 removed rules or 69 / 84 language coverage. Complete old owner/inverse/oracle/law authorities remain the domain argument; a small passing fixture set is insufficient. The seven retained IDs and eight historical IDs belong in the companion manifest and book, unchanged by this draft.

D5 retained 20 measurements contain 16 accepted / 4 public-refused processes, 15 debug / 5 dist runs and one distinct probe method. Production Certifier and the separately labeled repeated checker are distinct timed phases. Near-case dist Certifier measured 1.8077–1.9547 s; whole artificial double-check process about 4.33 s. Three debug whole-process soft warnings remain disclosed. These measurements supported retaining existing policy at that checkpoint; current affected 2 MiB gates add representative stack evidence. They do not prove universal CPU/memory/stack bounds, cumulative per-contract coercion work or unchanged H231 performance.

Signed `34ed8528` adopts private `TypeBoundsSession` resource bookkeeping. Each fold starts a fresh session over immutable borrowed bindings; lexical children preserve the existing map clone and may inherit successful bounds facts for unchanged annotations. Only successful resource checks are retained. Inserting a raw key invalidates that key’s fact before shadowing; raw/private binding lookups still perform the original defensive bounds check. These facts certify neither a carrier nor a semantic type operation. Original source-budget, equality and carrier checks remain in place.

Selected default debug-test measurements on macOS ARM64 support this scoped adoption: medium whole-process time fell from 11.31 s to 4.95 s with the untimed probe fields unchanged; near took 23.60 s under the unchanged 60 s deadline, retaining its 10 s soft warning. Near Certifier took 11.60 s and the separately repeated checker 11.89 s. The one-over case retained the exact nodes 32769/32768 public refusal and ran neither later checker phase. These three samples are separate from the historical D5 measurements. The original 60 s timeout and rejected originless bookkeeping experiment remain failed evidence. This is no universal performance bound, cap increase or final CLI/history pass.

## Current qualification and closure prerequisites

The independently joined current source registry has 257 actual sources: 169 flat golden outputs, 86 build-generated consumers and two export-only probes. It has 87 real build bindings. Its 605 stateful capability rows include 545 proof-required rows and 60 nonproof rows; all advertise recorded and observed-call APIs. These are applicability and emitted-surface results, not 605 executed proofs.

The actual maintained whole-workspace run completed all 156 all-target packages, 110 integration-only packages and strict workspace Clippy. Its two Cargo streams contain 581 executed harnesses, 3,222 passed cases and seven existing ignored cases. The maintained artifact join binds all 87 generated outputs to real includes/depfiles and reauthenticates the signed source closure. Do not add those counts to the narrower 85-consumer execution: the populations overlap. Runtime, macros, testkit, doctests, CLI, release/archive, proof and historical-law receipts retain their own scopes.

The focused native Zswap correction completed its four unchanged commands, including a genuine transfer proof and default-strict funded application with replay rejection. It does not replace the complete CLI consumer/proof gate. The exact original 974 historical case identities are prepared under the reviewed current-to-D5 adapter; preparation and bounded pilots are not a complete historical test pass. The six original D5 inventory laws remain retained D5 inventory evidence, not current #633 timing evidence. The 20 measured D5 processes are a separate performance population.

Before final #508 acceptance, the root must attach terminal complete CLI and historical receipts, any remaining selected proof-premise joins, and the independent final acceptance. Reconcile all 11 approved definition-of-done rows, the complete family/correction map, 15 dispositions, source diagnostics trust/availability, resource warnings, ADR/CoPS records and source/evidence fingerprints. Root review and Obsidian reconciliation make the packet ready for end-of-milestone publication. DoD 11 does not require publishing every 0.4.0 ADR or closing the entire milestone now. No human security audit, universal Compact/TS conformance, network attestation, push, merge or remote CI is implied.

## Validation and semantic preservation

The independent validator checks the plan's scopes, types, signatures and budgets. It does not prove that lowering preserves the Compact source program. A same-type operand swap, a write to another Cell of the same type, or a dropped effect can still be well typed. Semantic preservation therefore also depends on the lowering rules, independent expected observations, native/recorded/TypeScript comparisons and the proof/application cases that actually run. A successful type check is not a semantic-equivalence certificate.

Retired whole-body recognizers remain test references. The [retired recording-oracle milestone](../plans/retired-recording-oracles.md) defines family-by-family replacement, preservation of negative controls and historical authorities, and explicit removal criteria.
