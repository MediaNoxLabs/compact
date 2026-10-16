# Recording effect algebra: capability and retirement

**Maintainer-reported local closure: compiler 0.31.135 / ledger8.1.3, with historical checkpoints retained separately.** The [closure report](closure-report.md) and [evidence index](closure-evidence.json) record the original eleven DoD, EA00–EA13 and contribution history. [Evidence availability](evidence-availability.md) separates published source/reviews from maintainer-only historical acceptance. This publication preserves all original 84 retirement rows and creates no new execution or coverage result.

## Architecture

Production recording follows typed lowering, composition checking, independent structural/type validation and rendering. Supported operations compose into a checked plan; unsupported operations produce checked compiler refusals. An invalid plan is fatal. Historical whole-contract recognizers are test references, with no production admission fallback.

Ordinary entry points use `select_checked_observed` → `lower_with_calls_observed` → `check_with_calls_observed` → `validate::check` → rendering. Effectful roots share the algebra at their preserved boundary. The common pure semantic fold has a syntax-producing `NativeSink` and a `BodyCheckSink` with unit output. Signature, carrier, layout, encoding and resource rules remain explicit obligations. Structural validation is not a formal semantic equivalence proof.

Domains include scalar and aggregate values; bounded arithmetic/coercions; bytes/enums/hashes/commitments/Jubjub; control and lexical scopes; typed ledger slots; witnesses/calls; ledger8 context, kernel and ordered Zswap intents. Native Map/Fold support and direct recording support are distinct: native Fold or a certified pure helper does not imply a new direct effect-plan Fold operation. Recursive calls, expanded work, layout and representation violations receive bounded refusals. Later-ledger primitives and a production network SDK remain outside the goal.

## Stages must stay separate

| Stage | Required evidence |
|---|---|
| Native emission/compilation | Source, IR, generated files, package inputs and actual toolchain |
| Native execution | Selected executable outcomes, ordering and errors |
| Recording | Checked plan, capability outcome and source identity |
| Public replay | Initial-state replay, effects, failures and metering |
| Preparation | Trace/intents/context reconciliation and official transcript partition |
| Proof | Applicable genuine circuit/key/runtime binding, proof and verification |
| Application | Balanced transaction under declared ledger policy and replay refusal |

An emitted API is not an executed proof. Applicability flags are not completed proofs. Native-valid code can receive a typed recording refusal. Selected successful and negative outcomes establish only their declared cases and premises.

## Original boundary accounting

The [retirement ledger](retirement-ledger.json) retains all **84 original boundaries: 69 retired, seven production semantic/integration obligations and eight historical test artifacts**. The remaining 15 do not authorize whole-contract shape admission. These counts are historical boundary accounting, not language coverage. All original row fields and issue identities are unchanged.

Original family source maps, manifests, inverse payloads and laws retain their own authority. Machine-specific receipt locators in the old document have become explicit historical labels with availability statements. This is a metadata successor; it neither repins original authorities nor makes missing external receipts available. The original document is preserved at signed staging commit `05a7885f6518e1ca358896aec03cc028a66f8c13`.

## Reviewed corrections and limits

- #618 uses checked compiler refusals at the actual failed node, including resource limits. Exact old shape diagnostics remain historical.
- #621/#624 enforce tuple trait obligations where operations require them; #627 bounds conversion work.
- #626/#629 correct Rust keyword escaping and lexical allocation/collision handling.
- #630 uses direct Rust AST assembly to avoid deep token reparsing; selected Map/Fold and stack controls have separate scope.
- Fixed-width Field commitments are supported. Tests that formerly used them as unsupported sentinels moved to genuine refusal controls. Transient-origin diagnostics (#612) remain proposed, not enforced here.
- #633 preserves source occurrences and bounded related callers where available. Missing source metadata does not reject an otherwise supported native program. Selected frame reductions are not a universal stack bound.
- Successful-only `TypeBoundsSession` facts are scoped to immutable bindings and invalidated on raw-key shadowing. They do not certify semantic type/carrier rules.
- ADR0499 and successors correct nonproof native Zswap recording expectations. Recording/observed availability never turns a nonproof export into a proof-required one.

## Evidence epochs

| Population / checkpoint | Recorded outcome and scope |
|---|---|
| Extracted core `d18cddae` | Fresh 1,673 backend unit and 154 rendering tests passed; seven existing ignored unit tests; 162 default runtime tests; 121 focused Python checks. Six source-qualified frontend emissions, three selected generated snapshot comparisons. |
| Retained ledger8.1.3 `904964b6` / `7dece65f` | Accepted affected proof, consumer and archive evidence retains its original commands, keys, compiler label and policy. Current production sources/dependency inputs were separately matched. This is authenticated reuse, not newly executed current-core proof totals. |
| Retained 257-source qualification | 169 flat goldens, 86 generated consumers and two export-only probes; 605 recorded/observed API rows, 545 proof-applicable. All 257 isolated crates compiled in their original recorded epoch. Flags are not 545 proofs. |
| Deferred Forge `82654354` | Separate opt-in local tooling, excluded by the owner from this contribution stack. Historical adapter controls passed; a newer installed Forge build is not automatically qualified. Forge qualification is not a #508 closure gate. |
| Frozen historical source `ab132650` | On 2026-10-16, independent review accepted all 974 exact cases and original outer source/authority/final Git checks. Complete wall time was 27,005.032 seconds. This is historical-law qualification, not current proof execution. The disclosed configuration incident remains part of acceptance. |
| Retained fixture/discovery qualification | Explicit authenticated conformance and graph inputs repair ordinary discovery. All 281 current cases passed in 18.27 seconds complete process after independent review; direct graph CLI compatibility passed its ten overlapping cases separately. Original 30 conformance and ten graph test methods remain unchanged. Earlier 90 affected passes are included in this population, not additive coverage. |

Earlier source continuity checked 814 backend, 47 runtime, nine macro, nine testkit and 169 proof-smoke source-tree files against accepted `7dece65f`; these are file counts, not test totals. All 59 prior runtime pins and 606 source/IR/consumer pins matched. Runtime8.0.3 evidence cannot be relabeled8.1.3. Compiler .133 artifacts cannot be relabeled .134. Original 487-export and later 605-row populations must not be added together.

## Current contribution qualification — 2026-10-16

The original `d18cddae` / `b2d1122e` / `a1d6dc33` rows above describe their own earlier source epochs. The reconstructed contribution stack has separate accepted current-profile evidence:

| Current checkpoint | Accepted finite scope |
|---|---|
| Ledger8.1.3 npm-scope cohort | 31 packaging controls and both real Nix adapter outputs; current compiler 0.31.134 Scheme suite: 46 reported groups and 3,881 passed occurrences. Current normal CLIs and three source-consumer roles were checked, including two Rust behavior methods. Tuple12 was compilation only. |
| Unchanged affected source/input premises | Earlier 1,673 backend passes / seven ignored cases, 21 compatibility passes and 89 Python passes retain their original execution epoch. Independent review accepted relevant-premise reuse after the Nix-only change. These are not new executions or additive coverage. |
| Runtime refresh | 18 fresh current controls: nine helper-boundary, four VM and five testkit methods. Helper and VM controls overlap retained runtime populations. Testkit resolver/IR/assertion controls generate no proofs, parameters or keys. |
| Historical machinery / retained fixtures | 28 and 83 methods passed in their respective earlier source epochs. Independent review accepted unchanged relevant test/source/input premises after the seven-path Nix cohort inheritance: 71 machinery inputs, and the fixture modules plus 150 archive and six graph inputs. This does not relabel the whole-source execution epoch or rerun the historical 974 cases. |

These accepted results have recorded source, tool, input, artifact and feature bindings. The present documentation overlay executes nothing. The ledger preserves both the original source checkpoint and the subsequent publication checkpoint; the original local implementation is identified in the [maintainer-only evidence record](evidence-availability.md), separately from the published source checkpoint. Later closure corrections change documentation only. Original 70 proof calls remain ledger8.0.3 historical evidence. Current 8.1.3 affected proof receipts retain their own narrower circuit/key/runtime scope. The 605 applicability rows are not proofs.

## Reported local composed closure

The [closure report](closure-report.md) separates fresh affected checks, authenticated retained proof/consumer/historical evidence, source identity and all original completion conditions. The original local epoch reported 1,829 Rust passes and seven existing ignored tests. Authenticated Python discovery at that epoch reported 289 passes and zero skips. Default discovery reports manual skips honestly; invalid explicit inputs fail.

The original local contribution train was checked against its recorded qualified epoch. That historical source-identity and acceptance claim follows the [availability policy](evidence-availability.md); it is not a claim that the public whole tree is identical to the private checkpoint. Public source and reviewer-reported local checks have their own linked anchors. Original refs, failed attempts and raw signatures remain retained.

The wide-add producer hold is resolved: the portable current 3.1.2 capture has its own epoch and leaves the old 3.0.0 oracle unchanged. Restored independent output snapshots and checker controls remain present. The private replay-input archive retains original path-bearing provenance and is not an approved public upload or an outer-qualification substitute.

No universal TS/spec parity, Agda semantic proof, production network readiness or newly executed monolithic release gate is claimed. Compiler-level Forge integration remains deferred; the separate opt-in local adapter is already published. Coordinated Rust crate/version/lock migration is required before registry publication and is outside this local epic.

## Candidate-runner incident

An external candidate invocation inherited repository-selection Git environment variables. Temporary Git fixtures then added an accidental common `core.worktree` setting; that 281-case attempt with nine errors remains failed evidence. The setting was removed with every other configuration byte preserved relative to the corrupted pre-repair backup, not a pristine pre-incident snapshot. The accepted run used a real worktree without Git overrides. All 5,602 frozen source hashes/sizes still matched. Independent review found no source-view authentication dependency on the transient setting. The original terminal source/authority/cleanup and post-run Git checks subsequently passed and were independently accepted. Continuous metadata invariance is not claimed.

## Historical terminal acceptance — 2026-10-16

The unchanged four-worker run accounted for **974 unique cases**: 292 ordinary,
548 earlier historical, 128 latest historical and six retained measurement laws.
There were no failures, errors, skips or unrun cases. The measurement laws retain
their D5 inputs; they are not new current performance measurements. Worker
fixtures and original authentication/cleanup contracts passed independent review.
The complete outer execution receipt is bound to report SHA-256
`7cae10e33764e51486e9300d9c937a567ebb85ca8ec0cb92c3053ebc0ae0907b`.

The postterminal audit rechecked the accepted source and evidence premises and
found no additional finite compiler implementation gap. The final composed contribution/history and publication record is now
identified in the closure report. This successor neither reruns proofs nor
changes the original 84 ledger rows or their authorities.
