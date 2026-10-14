# Rust recording capability and retirement book

**Publication successor draft at signed `34ed8528`; final CLI/history acceptance remains pending.** The earlier `9faee816` and `532927b3` checkpoints changed current test-oracle expectations; the current checkpoint also changes compiler resource bookkeeping through private `TypeBoundsSession`. This supersedes the planning prose of the retained `66da6c77` draft, not its original 84-row retirement data or immutable family authorities. Signed source freeze 266 and independently joined artifact receipts are inputs to the final closure record; the accepted 72-map/16-owner join below retains its explicitly attributed freeze-262 epoch. No fresh test execution was performed for this documentation draft.

## What the backend now does

The default recorder admits supported typed operations and their composition. Ordinary roots use the current `select_checked_observed` → `effect_plan::lower_with_calls_observed` → `check_with_calls_observed` → `validate::check` → `render` route; Effectful roots use the same algebra at their earlier preserved boundary. The older `lower_with_calls_diagnostic` wrapper is test-only. Do not label a retained wrapper anchor as current production authority. Invalid plans are fatal. Unsupported operations retain checked refusals. A refused plan cannot enter the old generic recorder.

The pure frontend has one exhaustive shared semantic fold. `NativeSink` generates syntax; `BodyCheckSink` checks reachable pure bodies with `()` output. Neither the old native recursion dispatcher nor the retired pure-body Syntax bridge is a production fallback. Formal signature, carrier and representation guards remain real obligations.

Recording support is bounded. Type/layout/codec validity, resources, namespace safety, declaration authority and transaction readiness still matter. The old named profiles are test references/composition examples, not production admission mechanisms or formal proofs.

## Capability dimensions

| Dimension | Meaning | Evidence required |
| --- | --- | --- |
| Native compilation | The IR produces Rust and that public surface compiles | Source/IR/generated/package/toolchain bindings |
| Native execution | The generated native API actually executes selected cases | Executable case receipts; result/error/side effects/order |
| Recording | A validated typed plan emits a recorded API | Independent plan validation, capability outcome and source binding |
| Public replay | Recorded public operations reproduce the declared public behavior | Independent initial-state replay and metering/failure comparisons |
| Preparation | Runtime reconciles trace/intents/context and official transcript partition | Selected successful/refused preparation receipts |
| Proof | An applicable genuine source/circuit/key path proves and verifies | ZKIR/key/runtime/binary binding and changed-binding rejection |
| Application | The balanced transaction applies under declared ledger policy | Default-strict funded application, replay refusal and state receipts |

An emitted API is not an execution or proof result. A native-valid program may receive a typed recording refusal. A recorded API may produce an empty trace or an input-dependent preparation failure. Pure nonproof APIs have no standalone proof/application obligation; a proof-visible caller has its own row.

## Supported domains and explicit limits

| Domain | Current owners | Qualification boundary |
| --- | --- | --- |
| Scalar/aggregate values, arithmetic/coercion, bytes, enum, hash/commit/Jubjub | `pure_semantics/*`, `pure_native/*`; typed recording model/lower/validate/render | Neutral facts own rules; native formatters do not recheck source. Signature/carrier/layout/bounds and hash encodings remain explicit. |
| Control and lexical names | Shared expression/control/visitor/scope; `pure_native/local.rs` | Both branches statically checked; one branch executed; source occurrences and first-error order preserved. |
| Map/Fold/native aggregates | Shared aggregate visitor +native formatters | Native Fold qualification is distinct from direct recording. Direct recording VectorMap has its own checked operation; VectorFold used inside a certified pure helper is not a new direct effect-plan Fold operation. |
| Ledger operations | Typed Cell/Counter/Map/Set/List/Merkle slot domains and official runtime | Full slot path/key/value codec/signature authority; native support does not bypass recording policy. |
| Witness and calls | Exact declarations, bounded call graph and reachable pure certification | Arguments/order/signature/frame boundaries; recursion and expanded-work refusals. |
| Context/kernel/intents | Typed ledger8 observations, kernel operations and ordered Zswap values | Official domains/reconciliation; proof and application require separate source-bound receipts. |
| Formal body checking | Same fold with `BodyCheckSink`; relocated Default/named-Struct guards | `()` output does not mean zero allocation or universal body complexity. |

Excluded later-ledger primitives and production network SDK work remain outside this goal. Explicit ZKIR-v3 Keccak qualification cannot be described as universal ledger8 proof availability.

## Current retirement accounting

The original ledger has **84 boundaries; 69 are explicitly retired**. The other 15 are seven production semantic/integration obligations and eight test-only artifacts. None of those15 authorizes a whole-contract shape in production. See the included [retirement ledger](retirement-ledger.json) for all 84 original rows and accepted retirement issue identities. The exact 15 source dispositions and the other named research datasets below are retained external evidence, not additional published files.

Keep this historical accounting. It is not a language-coverage fraction and does not need a fictional84/84 claim. Nine Effectful and 478 ordinary owner observations in the retained 487-export corpus are finite emission observations, not complete old-domain proofs or runtime effect summaries.

The source and original-domain argument is the immutable per-family manifest/inverse/law history. The original 49 oracle authorities remain unchanged. The retained D5 checkpoint binds 669 protected paths and 79 historical manifests with zero new retirements. Current qualification uses the explicit final source transition and retains the original source authorities; these counts are epoch-specific.

## Changes that deliberately corrected prior behavior

These corrections must accompany final compatibility claims:

- **#618 checked compiler refusals:** the approved policy reports the actual failed node/resource condition; exact historical shape diagnostics are retained only as test history. Supported interfaces/execution remain separately qualified.
- **#621/#624 tuple trait obligations:** native/stateful operation-specific tuple rules diagnose unavailable derives/default/equality where used; this is not a new recording profile.
- **#627 coercion work:** the explicit per-conversion work envelope prevents materialization blowups. It is not total compiler-memory or complete body-work certification.
- **#626/#629 lexical names:** fixed Rust 2024 `gen` escaping and checked Let live-symbol allocation correct panic/collision cases. Raw semantic key shadowing and ordinary names preserve their reviewed semantics.
- **D1c nested Eq/Ne:** boxed Binary children preserve association where the prior Rust parser panicked. Genuine TS cases remain accepted; prior nested Rust output is not falsely called byte-identical.
- **#630 Item/File/Map assembly:** direct AST assembly corrects deep-token reparse stack failures. Native Map evaluation/association, unchanged Fold behavior and actual 2 MiB near/over gates are separately retained.
- **Field commitment fixtures:** fixed-width commitments became supported; former unsupported sentinels were migrated to genuine retained refusal controls with explicit frontend-excluded IR labels. Transient-origin diagnostics (#612) remain a separate proposed concern, not an enforced restriction here.

- **#633 source diagnostics and metadata:** retain exact original borrowed source occurrences at actual failure producers; first-write unavailable facts and bounded related callers remain explicit. Private transport schema 3 preserves physical and compiler-context extents; live configured invocation authority does not authenticate frontend source consumption. Metadata absence/budget unavailability does not reject otherwise supported native programs.
- **#633 recursive expression stack repair:** the measured debug-arm64 recording expression frame decreased from 39,536 to 2,912 bytes. The unchanged selected full-root 2 MiB near/over/malformed law passes; this is a selected frame/worker qualification, not a universal stack bound.
- **#631/#632 validation repairs:** the registry distinguishes actual generated outputs, flat goldens and export-only probes. Original historical tests execute in explicit authenticated epochs. Requested lexical paths remain visible outside normalizing readers; worker-owned physical D5 fixtures are disjoint from current/protected source roots. Failed pilots and corrected authority closures remain retained.
- **ADR0499 nonproof recording:** exactly six current native Zswap recording/observed expectations changed from false to true. All three proof-required flags remain false. No compiler/runtime behavior changed, and a focused proof pass is not the complete CLI pass.

A later signed current-oracle checkpoint `532927b3` corrects only the wide-add and native own-public-key qualification expectations. The two direct helpers passed their four original compiler-generation commands; all proof-false / not-applicable rules, IR, signature and value checks remain unchanged. This is a test-only continuation of ADR0499, not a compiler/runtime behavior change or a complete CLI pass. The earlier failed CLI receipts remain preserved.

## Evidence book status

`export-applicability-matrix-draft.json` contains all 487 retained exported rows with exact source/IR and owner identities. It truthfully records compilation/emission and leaves execution/replay/preparation/proof/application unjoined until a case receipt applies. The raw schema2 census has no frontend `proof_required`; all 487 require a genuine frontend applicability join before final proof summaries.

That 487-row corpus is historical continuity evidence. The current 257-source gate supplies a genuine frontend applicability join for 605 current exported APIs; its 545 proof-required flags are not 545 completed proofs. Keep the two populations and evidence stages separate when final case receipts are joined. `old-admission-rule-map.md` records remaining concrete joins: selected proof premises, isolated surface execution, required #633 native/recording source-range attachment and final resource/review qualification.

`family-evidence-matrix-draft.md` provides the concrete selected runtime/proof/consumer receipts and their reuse boundaries. `registry-reconciliation.json` compares the current full-gate source map with the retained202. These datasets share source identities rather than defining another hand-maintained capability authority.

D5’s 20 measured processes, existing recursive workers and call/type/layout/resource controls are representative budget evidence. Debug near-case warnings remain visible; the optimized same-fixture result supports keeping current limits at this checkpoint, not a universal compiler-performance guarantee.

Signed `34ed8528` adopts private `TypeBoundsSession` resource bookkeeping. Each fold starts a fresh session over immutable borrowed bindings; lexical children preserve the existing map clone and may inherit successful bounds facts for unchanged annotations. Only successful resource checks are retained. Inserting a raw key invalidates that key’s fact before shadowing; raw/private binding lookups still perform the original defensive bounds check. These facts certify neither a carrier nor a semantic type operation. Original source-budget, equality and carrier checks remain in place.

Selected default debug-test measurements on macOS ARM64 support this scoped adoption: medium whole-process time fell from 11.31 s to 4.95 s with the untimed probe fields unchanged; near took 23.60 s under the unchanged 60 s deadline, retaining its 10 s soft warning. Near Certifier took 11.60 s and the separately repeated checker 11.89 s. The one-over case retained the exact nodes 32769/32768 public refusal and ran neither later checker phase. These three samples are separate from the historical D5 measurements. The original 60 s timeout and rejected originless bookkeeping experiment remain failed evidence. This is no universal performance bound, cap increase or final CLI/history pass.

Final acceptance requires the terminal local CLI/history gates, case-specific proof/applicability disposition and independent EA13 review. This reconciled draft and its pinned publication inputs satisfy preparation for publication; they do not require closing the entire 0.4.0 milestone now. No new Compact/Agda conformance expansion is required for this packet. No push, merge or remote CI is authorized.

## Current source and consumer join

| Selected current population | Actual result | Scope |
|---|---|---|
| Genuine source registry | 257 emitted sources; 169 true flat goldens, 86 generated consumers, two accepted export-only probes | Source/IR/Rust/capability/origin joins; probes are not application execution |
| Export applicability | 605 recorded/observed rows; 545 proof-required, 60 nonproof | Flags describe applicability and generated APIs, not completed proofs |
| Narrow consumer continuation | 85 packages, 87 output bindings, 206 executed harnesses, 300 passed cases | Selected consumers; not the entire dependency graph |
| Maintained full workspace | 156 all-target plus 110 integration-only packages, strict workspace Clippy | 581 executed harnesses; 3,222 passed, seven existing ignored; 87 output/include/depfile joins |
| Focused native Zswap correction | Four unchanged commands passed; 4,480-byte transfer proof, changed-binding rejection, strict funded application and replay rejection | Genuine selected transfer; no proof claim for the three nonproof exports, no complete CLI pass |
| Final CLI and historical shards | Pending final terminal receipts | Preparation, bounded controls and pilot success are not full pass |

The narrow and full consumer populations overlap; never sum them as distinct tests. Separate per-family proof/preparation/replay/application rows must retain their actual receipt and policy. The current source freeze identifies 5,597 tracked paths; a source count is not language coverage.

## Historical map and current source anchors

The original admission-rule map retains its exact 84 rows, 72 source premises and historical entry names. Diagnostic-era changes affect 16 of those source owners. The separate independently reviewed current-source continuity join authenticates all 72 premises: 56 unchanged inputs, 14 exact final-owner preimages and two additional held original source copies. It does not repin the old map or prove formal semantic equivalence. Current observation callbacks, tests and source guards have their own reviews and receipts. The current route above is pinned independently to the signed current source; the old diagnostic wrappers remain test-only.
