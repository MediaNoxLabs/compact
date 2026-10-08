# 0.3.0 acceptance crosswalk

19 required outcomes accepted; one removed ACC outcome. All93 original criteria are retained below. Historical checkbox values, source hashes,121 decision-to-issue/commit mappings and the full public milestone issue catalogue are in [crosswalk.json](crosswalk.json). [Immutable source archive](acceptance-sources.zip) retains original acceptance notes and pre-report GitHub snapshot. #358 closes after this artifact is published.

## R030-01 — Crystallize runtime and compiler domain models

[345](https://github.com/MediaNoxLabs/compact/issues/345) — **accepted-source-bound**

Ownership map; three-way checked admission; representative pure/read/witness/funded composition; measured lexical-model disposition.

- **R030-01.C1** Document ownership of source typing, declared slots, witness work, VM execution, recording, offer binding, proving and observation trust. Identify public vs internal extension points. Evidence: P01, QFINAL.
- **R030-01.C2** Distinguish not-applicable, malformed/rejected and admitted lowering; diagnostics retain source/callee context and never silently accept after a rejected guard. Evidence: P01, QFINAL.
- **R030-01.C3** Migrate representative pure, read-only, witnessed branch and funded composition slices with exact old accept/refuse characterization; preserve ordering, short circuit, alignment and policy boundaries. Evidence: P01, QFINAL.
- **R030-01.C4** Use explicit binding/circuit IDs and typed operands where the probe proves their value; retain source provenance and reject shadow/scope leakage. Keep domain changes in small ADR-led slices. Evidence: P01, QFINAL.

## R030-02 — Improve generated Rust through repeated design probes

[346](https://github.com/MediaNoxLabs/compact/issues/346) — **accepted-source-bound**

ADR-led explicit/helper/derive/Args/Rc/BindingId probes, unchanged generated external consumers, semantic/order controls and measured accepted/rejected choices; public abstraction-level flag not promised.

- **R030-02.C1** Each candidate records the problem, real emitted before/after code, ordinary Rust alternative, affected emitter/runtime ownership and ABI implications in an ADR. Evidence: P02, QFINAL.
- **R030-02.C2** Use unedited generated crates in external consumers; measure expanded and displayed source, build time, stack/memory and diagnostics. Evidence: P02, QFINAL.
- **R030-02.C3** Preserve effect/witness order, variable hygiene, early errors and default debug-worker execution. Keep rejected experiments and their measurements. Evidence: P02, QFINAL.
- **R030-02.C4** Adopt improvements only with measured usability or maintenance benefit; fewer lines or more macros alone does not pass. Evidence: P02, QFINAL.
- **R030-02.C5** Prototype selectable generated-code abstraction levels against ordinary Rust, derives and small runtime helpers; record semantic equivalence, diagnostics and build cost. This research is nonblocking and does not promise a new public switch. Evidence: P02, QFINAL.

## R030-03 — Strengthen typed primitives and consumer safety

[347](https://github.com/MediaNoxLabs/compact/issues/347) — **accepted-source-bound**

Resumed ADR0285 controls at4c8aebc3 (E0382/E0423 plus positive behavior), retained scalar/FAB/offer/observation negatives, scoped unsafe/FFI inventory and strict-policy distinction.

- **R030-03.C1** List concrete prevented misuses and compare them with the equivalent TS boundary. Use compile-fail tests for static guarantees and malformed-input tests for dynamic guarantees. Evidence: P03, QFINAL.
- **R030-03.C2** Preserve canonical vs reduced scalar semantics, declared integer bounds, domain separation, verifier/address/input binding and exact offer ownership/allocation. Evidence: P03, QFINAL.
- **R030-03.C3** Distinguish caller-provided identity, observed provenance and authenticated ownership; avoid claiming types establish finality or wallet authority. Evidence: P03, QFINAL.
- **R030-03.C4** No new unsafe code in owned runtime; audit dependency unsafe boundaries and FFI separately. No implicit fallback from strict preparation to a weaker policy. Evidence: P03, QFINAL.

## R030-04 — Consolidate mechanical repetition with selective macros and derives

[348](https://github.com/MediaNoxLabs/compact/issues/348) — **accepted-source-bound**

Duplication inventory, CompactStructRepr/name hygiene/diagnostic controls and bounded expansion/build measurements; keep ordinary functions for semantic work.

- **R030-04.C1** Publish a duplication inventory with ownership and selected/declined abstraction decisions. Evidence: P04, QFINAL.
- **R030-04.C2** Chosen derives/macros preserve FAB/hash/ordinal/layout behavior, generics where supported, hygiene and source-span diagnostics; positive and compile-fail expansion tests exist. Evidence: P04, QFINAL.
- **R030-04.C3** Prefer normal functions for shared VM/gas/context mechanics; retain actual observed aligned atoms and canonical upstream programs. Evidence: P04, QFINAL.
- **R030-04.C4** Report macro expansion/build effects and examples. An evidence-backed decision to retain explicit Rust is a valid outcome for a candidate. Evidence: P04, QFINAL.

## R030-05 — Resolve concrete SOLID, KISS and DRY design gaps

[349](https://github.com/MediaNoxLabs/compact/issues/349) — **accepted-source-bound**

Cohesive capability/coin/naming/declaration/value/native/recorded ownership, characterized admission and byte-identical generated/capability cohorts; no generic module-count claim.

- **R030-05.C1** Inventory high-risk coupling and duplicated invariants with file references and tests that characterize them. Evidence: P05, QFINAL.
- **R030-05.C2** Preserve native/recorded obligations while sharing semantic leaves and checked plans where valid; remove a duplicate only after differential proof. Evidence: P05, QFINAL.
- **R030-05.C3** New context fields require explicit classification in helper/recording safety boundaries; error handling does not rely on string sniffing. Evidence: P05, QFINAL.
- **R030-05.C4** Each refactor has a bounded ownership change, no accidental public API break and documented before/after maintenance benefit. Evidence: P05, QFINAL.
- **R030-05.C5** Decompose recorded.rs, stateful.rs and lib.rs into cohesive domain/component modules with explicit narrow interfaces and nearby relevant tests. Document how engineers navigate ownership; no generic dumping grounds or module-count target. Evidence: P05, QFINAL.

## R030-06 — Deliver ContractLab: the generated-contract testkit

[350](https://github.com/MediaNoxLabs/compact/issues/350) — **accepted-source-bound**

ContractLab explicit environment, typed reports/witnesses, snapshots/forks and checkpoint failure semantics with Counter/Cell/collection/Merkle/DID demonstrations. ADR0360 adds optional official-provider ProofLab, with no successful-proof claim.

- **R030-06.C1** Native and recorded/replay scenarios support explicit clock/address/gas/identity, deterministic fixture seeds, typed witness answers/errors/journals and owned snapshots/forks. Evidence: P06, QFINAL.
- **R030-06.C2** Snapshot restore checks version, source/runtime/ledger identities and state mode; parallel scenarios have no global mutable witness registry. Evidence: P06, QFINAL.
- **R030-06.C3** Call reports preserve typed output, state/effects, ordered operations/private alignment and separately named query/replay gas; private output is redacted by default. Evidence: P06, QFINAL.
- **R030-06.C4** Failure keeps committed lab state at its checkpoint; do not invent partial VM traces or promise rollback of external witness side effects. Evidence: P06, QFINAL.
- **R030-06.C5** Optional strict proof/ledger adapter and explicit network tier have separate requirements and receipts. Demonstrate Counter, witnessed Cell, collections, Merkle and the adopted real contracts. Evidence: P06, QFINAL.

## R030-07 — Establish meaningful generated-code coverage

[351](https://github.com/MediaNoxLabs/compact/issues/351) — **accepted-source-bound**

198-root/232-input census, current/historical1055-function classification, typed export/constructor assertions, finite compiler/runtime obligations and per-component instrumentation at stated source cohorts.

- **R030-07.C1** Classify 100% of pinned required sources/imports/exports and regenerate the release fixture cohort; compilation, execution, replay, proof and network evidence are separate. Evidence: P07, QFINAL.
- **R030-07.C2** Every required contract export (and constructor, where present) has a direct typed success invocation or intentional error where success is impossible by design; security-sensitive exports also have negative and state-transition cases. Evidence: P07, QFINAL.
- **R030-07.C3** Cover every enumerated changed semantic/security obligation and effectful branch; no exclusion may conceal a mandatory contract primitive. Evidence: P07, QFINAL.
- **R030-07.C4** Proposed instrumentation floor: 90% lines in new testkit-owned code and 95% changed lines in owned runtime/backend/testkit code, measured per component after baseline; exclusions require rationale. Generated line/branch coverage is diagnostic alongside export coverage. Evidence: P07, QFINAL.
- **R030-07.C5** Keep stable line/region reports; branch coverage uses a separately pinned compatible nightly if adopted. Preserve exact toolchain, features, source hashes and denominators. Evidence: P07, QFINAL.

## R030-08 — Create developer-friendly Rust backend documentation

[352](https://github.com/MediaNoxLabs/compact/issues/352) — **accepted-source-bound**

Published developer guides, executable-example receipts, CoPS and121 milestone ADRs at20cd967e; source hashes and1455 links validated; #352/#449 closed.

- **R030-08.C1** Cover install/version selection, compile, generated types/witnesses, ContractLab, errors, recording, proof/ledger integration and trust boundaries. Evidence: P08, PUBFINAL, QFINAL.
- **R030-08.C2** Provide DID and digital-passport walkthroughs, ACC scope/status notes (and ledger-8 migration notes only if activated), TS-to-Rust examples, troubleshooting and supported-target/version matrices. Evidence: P08, PUBFINAL, QFINAL.
- **R030-08.C3** All claimed runnable snippets build/test from a clean external directory without checkout-relative runtime paths; compile-fail examples test documented safety constraints. Evidence: P08, PUBFINAL, QFINAL.
- **R030-08.C4** Keep ADRs as decision history and link current guides to the decisions. Document generated API migration and limitations precisely. Evidence: P08, PUBFINAL, QFINAL.
- **R030-08.C5** Maintain draft developer guides and examples in Obsidian during delivery; publish reviewed documentation and the milestone decision/history corpus to the repository at closeout. Evidence: P08, PUBFINAL, QFINAL.

## R030-09 — Adopt Midnight DID v0.7.0

[353](https://github.com/MediaNoxLabs/compact/issues/353) — **accepted-source-bound**

Pinned DIDv0.7.0 source/imports; constructor and12exports native/recorded scenarios; union of strict proof receipts for all12exports; separate8.1 snapshot transport and maintained reducers.

- **R030-09.C1** Resolve the v0.7.0 tag to an immutable commit and freeze source/import hashes, lockfile, compiler/language/runtime/ledger versions and exported signatures. Later DID releases do not silently replace the user-selected baseline. Evidence: P09, QFINAL.
- **R030-09.C2** All required constructor/exports compile and execute through unedited generated Rust; no handwritten generated-source patches or disabled authentication. Evidence: P09, QFINAL.
- **R030-09.C3** Port positive, wrong-authority, duplicate/missing-key, lifecycle and cryptographic-boundary scenarios that exist in the pinned contract; assert typed output, state, failures and witness effects. Evidence: P09, QFINAL.
- **R030-09.C4** For every proof-applicable export run a valid default-strict proof/verify/ledger-apply case; cover each distinct authorization/effect path and critical lifecycle, with network acceptance separately recorded. Evidence: P09, QFINAL.
- **R030-09.C5** Every compatibility failure follows R030-11. No unresolved required primitive or export remains at closure. Evidence: P09, QFINAL.

## R030-10 — Adopt the latest maintained digital-passport contract

[354](https://github.com/MediaNoxLabs/compact/issues/354) — **accepted-source-bound**

Pinned midnight-vc-passport consumer and its declared credential-compact0.2.0 dependency;75public pure exports/202sampled cases and codec vectors. Zero proof-applicable exports established by compiler metadata.

- **R030-10.C1** Pin midnight-vc-passport consumer commit, latest relevant contributions and exact resolved contract/core package and source hashes. Distinguish merged work from open PRs; do not replace the declared dependency with a similarly named repository by assumption. Evidence: P10, QFINAL.
- **R030-10.C2** All compiled required pure exports compile and execute through unedited generated Rust, including composed imported functions. Evidence: P10, QFINAL.
- **R030-10.C3** Port claim-root, age/date-boundary, selective-disclosure/request, private-part opening, holder-binding and issuance/verification envelope cases from the pinned pure family. Assert invalid claims/authorization and context matching; do not invent ledger state transitions absent from the source. Evidence: P10, QFINAL.
- **R030-10.C4** Classify the compiled pure family honestly: direct typed value/error and cross-language codec evidence is mandatory; proof/verify/ledger-apply is required only for actual proof-applicable exports. If none exist, record zero with authoritative compiler metadata; do not fabricate recorded calls or ledger application. Evidence: P10, QFINAL.
- **R030-10.C5** R030-11 resolves every failed required primitive; no silently narrowed feature set qualifies as adoption. Evidence: P10, QFINAL.

## R030-11 — Turn contract compatibility failures into minimal regression slices

[355](https://github.com/MediaNoxLabs/compact/issues/355) — **accepted-source-bound**

13maintained minimal source reducers,81behavior cases and28strict snippet calls across historical receipts, joined to original DID export/source/root-cause and full scenario retests.

- **R030-11.C1** Each failure has original source hash/export/input, expected vs actual behavior, minimized .compact reproducer and first failing layer. Evidence: P11, QFINAL.
- **R030-11.C2** Create a focused issue and ADR when design changes; add renderer/admission negative tests, a generated-crate behavior test and TS/upstream comparison as appropriate. Evidence: P11, QFINAL.
- **R030-11.C3** Fix the smallest semantic cause using upstream primitives; for proof-applicable reproducers, prove/verify/default-strict apply the snippet, and always re-run the original contract scenario. Pure/diagnostic/non-proof reproducers retain their legitimate proof refusal or not-applicable classification. Evidence: P11, QFINAL.
- **R030-11.C4** Unsupported source diagnostics remain typed and actionable; no blanket denylist or fixture-specific name match substitutes for the fix. Evidence: P11, QFINAL.
- **R030-11.C5** Preserve the full contract regression after minimization, including any failure that exposes a TS/upstream defect rather than a Rust defect. Evidence: P11, QFINAL.

## R030-12 — Removed: Passport ACC adoption

[356](https://github.com/MediaNoxLabs/compact/issues/356) — **removed-not-delivered**

Owner-approved removal underADR0316 after construct inventory; removed is not delivered. Generic Jubjub tests remain. Standalone time support issue442 is outside milestone.


## R030-13 — Complete internal and independent external audits

[357](https://github.com/MediaNoxLabs/compact/issues/357) — **accepted-source-bound**

Internal subject crosswalk plus separately configured external reviews/retests at4c8aebc3/e6807884/ef352bcd/231559f1; explicit findings, withdrawals and dependency residuals. Later owner-approved decoder exception belongs toADR0359.

- **R030-13.C1** Internal review covers semantic ownership, IR/admission, generated output, codecs, witness/privacy, offer/proof/observation trust and failure behavior. Evidence: P13, QFINAL.
- **R030-13.C2** Independent external coding-agent review uses a separately configured tool/model and exact frozen source; implementing agents do not grade their own changes. The user selected independent external coding-agent review; this is not a human security audit. Evidence: P13, QFINAL.
- **R030-13.C3** Each finding has severity, reproducer/source evidence, owner, remediation and independent retest; no open critical/high security or correctness finding at closure. Evidence: P13, QFINAL.
- **R030-13.C4** Medium/low findings have explicit disposition and residual-risk documentation; neither tests nor agent review are called a human security certification. Evidence: P13, QFINAL.
- **R030-13.C5** Repeat relevant review after fixes and preserve architecture/functional/nonfunctional checklists in the closure bundle. Evidence: P13, QFINAL.

## R030-14 — Compose the 0.3.0 closure report

[358](https://github.com/MediaNoxLabs/compact/issues/358) — **accepted-source-bound**

This published closure artifact maps93criteria/19required outcomes/121ADRs; qualification and documentation parents closed first. Remote #358 closure follows this commit.

- **R030-14.C1** Map every deliverable, issue, ADR, commit and acceptance criterion to an immutable result or explicit experimental outcome. Evidence: CLOSURE, QFINAL.
- **R030-14.C2** Include ACC deferred/activated scope and provenance (if activated), direct DID/passport results, code/ABI migration notes and supported target/feature matrices. Evidence: CLOSURE, QFINAL.
- **R030-14.C3** Separate inventory, sampled behavior, source branch obligations, replay, strict proofs, live runs and WASM feasibility; record failed/rejected approaches. Evidence: CLOSURE, QFINAL.
- **R030-14.C4** Provide final known limits, audit residuals, local/remote exact-commit results and developer entry points; reconcile Git and Obsidian. Evidence: CLOSURE, QFINAL.
- **R030-14.C5** Close only after R030-20 passes; documentation does not convert blocked required contract work into completion. Evidence: CLOSURE, QFINAL.

## R030-15 — Prototype generated Rust in Node.js and browsers through WASM

[359](https://github.com/MediaNoxLabs/compact/issues/359) — **accepted-source-bound**

ADR0256 bounded native/Node24.14.0/Chromium151 probe, actual VM subset, byte/integer transport, errors and resource measurements; not a browser prover or production target guarantee.

- **R030-15.C1** Resolve the exact target-specific dependency/feature graph; dependency presence in Cargo.lock alone is not proof of a target blocker. Evidence: P15, QFINAL.
- **R030-15.C2** Probe untouched pure arithmetic/hash and Counter/witnessed-Cell generated crates in pinned Node and headless browser engines, using actual VM semantics. Evidence: P15, QFINAL.
- **R030-15.C3** Test large-integer/bytes marshalling, results/state/replay, witness order and error mapping; no lossy JS Number conversion for Field/u128. Evidence: P15, QFINAL.
- **R030-15.C4** Record module/import graph, compressed size, startup, peak linear memory and per-call measurements; fixture entropy and production entropy remain separate. Evidence: P15, QFINAL.
- **R030-15.C5** Exit with a validated supported subset or an exact blocker/minimal reproducer plus proposed dependency boundary. No browser-prover or production WASM support promise follows from feasibility alone. Evidence: P15, QFINAL.

## R030-16 — Define version, compatibility and migration contracts

[360](https://github.com/MediaNoxLabs/compact/issues/360) — **accepted-source-bound**

Machine feature/source policy; actual1.88 checks; ABI49-to50 migration and isolated ledger8.1 wire bridge; immutable application baselines. Final source/ref/platform/feature qualification remains.

- **R030-16.C1** Publish a machine-readable compatibility/source matrix and feature policy for Rust-only, ledger transaction, testkit and experimental WASM consumers. Evidence: P16, QFINAL.
- **R030-16.C2** Validate the declared MSRV independently from the tested Rust 1.99 toolchain; update the claim or toolchain only with an explicit decision. Evidence: P16, QFINAL.
- **R030-16.C3** Decide SemVer changes, generated/runtime pairing, ABI/IR migration and deprecated low-level APIs before breaking public consumers. Evidence: P16, QFINAL.
- **R030-16.C4** Compile old/new compatible consumer fixtures; mismatches fail clearly. A ledger9 package must not enter the ledger8 graph unnoticed. DID declares ledger-v8 8.1.0 while the Rust baseline uses ledger 8.0.3: prove the required compatibility or adopt an explicitly reviewed coherent ledger/zk pin update before DID acceptance. Evidence: P16, QFINAL.
- **R030-16.C5** Resolve latest-contract policy at implementation start and recheck drift before RC; no moving branch name is the sole provenance. Evidence: P16, QFINAL.

## R030-17 — Build adversarial, property and differential assurance

[361](https://github.com/MediaNoxLabs/compact/issues/361) — **accepted-source-bound**

Finite compiler/runtime obligation registries, deterministic negative/property controls and actual mutants; owner-approved explicit byte admission exception defers aggregate decoded heap/object/CPU containment underCoPS001/#495.

- **R030-17.C1** Enumerate malformed IR, scope/slot/type/call mismatches, overflow/canonical scalars, aliasing, invalid FAB, duplicate/foreign offers and observation mismatch cases before changing behavior. Evidence: P17, QFINAL.
- **R030-17.C2** Use upstream ledger/zk and independent TS captures as applicable; label known oracle/provider disagreements instead of auto-blessing one backend. Evidence: P17, QFINAL.
- **R030-17.C3** Each owned trust boundary has deterministic seeds and minimized regression corpus; no panic or unbounded resource use for supported untrusted-input APIs.
  - Owner-approved exception (2026-10-08, ADR0359/CoPS-001): raw tagged contract-state/verifier decoding is qualified for explicit encoded-byte admission and its deterministic codec corpus; aggregate decoded heap/object/CPU containment is deferred beyond0.3.0. Transport acquisition and deployment budgets remain caller responsibilities; no total containment guarantee is claimed. Evidence: P17, QFINAL.
- **R030-17.C4** Fuzzing runs have reproducible seeds/budgets/results; targeted mutants prove important guards are exercised. Do not disable cryptographic validation in acceptance paths. Evidence: P17, QFINAL.
- **R030-17.C5** Security-critical state transitions and privacy output policies retain explicit positive/negative tests and no open critical/high findings. Evidence: P17, QFINAL.

## R030-18 — Set and measure performance and resource budgets

[362](https://github.com/MediaNoxLabs/compact/issues/362) — **accepted-source-bound**

Scoped dce3c9e9 paired baseline,136phase-separated observations, default-worker regression and compiler/proof budgets; >10%matching-candidate median trigger requires reviewed investigation.

- **R030-18.C1** Pin compiler/source/lock/toolchain/target/profile/hardware and separate cold dependency builds, warm generated builds, runtime, proof and startup costs. Evidence: P18, QFINAL.
- **R030-18.C2** Use small controls plus DID and digital-passport cohorts (ACC only after activation), paired samples and spread; retain the default debug-worker stack regression. Evidence: P18, QFINAL.
- **R030-18.C3** Proposed regression trigger: >10% median runtime/warm-build/peak-memory increase on controlled matching cases requires investigation and reviewed acceptance, never a single noisy sample verdict. Evidence: P18, QFINAL.
- **R030-18.C4** Measure circuit/prover rows/k/key size when source changes; For a later activated ACC scope, assess resource feasibility before full key generation. Evidence: P18, QFINAL.
- **R030-18.C5** Freeze practical ceilings after baseline; publish tradeoffs and rejected macro/helper optimizations rather than asserting Rust always wins. Evidence: P18, QFINAL.

## R030-19 — Make the local delivery pipeline reproducible and efficient

[363](https://github.com/MediaNoxLabs/compact/issues/363) — **accepted-source-bound**

Reproducible gate mechanism and midpoint stabilization accepted at22ef5f23; frozen-source/tool/lock/selection provenance. Original final-revision checkboxes were pending under R030-20 at the mechanism checkpoint. Final candidate execution now accepted through QFINAL; original unchecked states remain historical.

- **R030-19.C1** Seal compiler, Scheme runtime, lockfile, generated source and fixture identities in receipts; concurrent slices do not share mutable compiler artifacts. Evidence: P19, QFINAL.
- **R030-19.C2** Use affected renderer/generated/TS tests per slice; expand for ABI/schema/effects and full integration checkpoints. Measure checks rather than skipping equivalent assertions by assumption. Evidence: P19, QFINAL.
- **R030-19.C3** Keep caches scoped to source/toolchain/features/profile; record command selection and prove generated behavior targets really ran. Evidence: P19, QFINAL.
- **R030-19.C4** Capture dependency/license/advisory review and artifact checksums; use locked, reproducible external consumers and explicit provider/material prerequisites. Evidence: P19, QFINAL.
- **R030-19.C5** Defer broad remote CI until required local implementation and acceptance prerequisites pass (excluding remote qualification and the final report), then qualify the final revision; keep conventional GPG/DCO commits, ADR→issue→milestone links and vault history. Evidence: P19, QFINAL.
- **R030-19.C6** Prefer focused local gates. Begin remote CI stabilization when 10 of the frozen 20 parent outcomes satisfy local acceptance. Child issue closures and deferred ACC do not increase progress; keep final remote qualification at the release candidate. Evidence: P19, QFINAL.

## R030-20 — Qualify the final release candidate and downstream migration

[364](https://github.com/MediaNoxLabs/compact/issues/364) — **accepted-source-bound**

Explicit composite qualification:423 successful unchanged-prefix commands plus3 recovery commands, source/tool/lock joins, all5 proof gates and exact-candidate bounded remote37775401108; #364 closed after publication.

- **R030-20.C1** All required deliverables 1–11,13 and 16–19 pass; WASM15 has its documented experimental outcome. ACC12 is removed under ADR0316; publish its removal disposition without an adoption claim. No required DID/digital-passport feature remains unsupported. Evidence: QFINAL.
- **R030-20.C2** Run the full applicable local suite and required remote checks at the frozen revision, with artifact provenance and no unrelated dirty source in release inputs. Evidence: QFINAL.
- **R030-20.C3** Exercise clean generated-crate consumers on declared supported hosts and the asserted MSRV; qualify default and transaction/testkit feature graphs. Evidence: QFINAL.
- **R030-20.C4** Validate runtime/API/ABI migration examples and final audit retests; authenticate package inputs and account for all dependency/advisory dispositions. Evidence: QFINAL.
- **R030-20.C5** Produce signed/DCO commit and candidate manifest. Tags, registries, upstream PRs and production deployment require their own explicit publication scope; milestone naming alone is not a compiler or package-version bump. Evidence: QFINAL.

## Decision and commit index

| ADR | Issue references | Recorded commit references |
|---|---|---|
| [ADR0241](../../../adr/0241-vault-first-delivery-and-measurable-midpoint-ci.md) | #363 | `4c5eb5de` |
| [ADR0242](../../../adr/0242-separate-capability-coin-shape-and-recording-facade-ownership.md) | #346, #349, #365 | `537fd641`, `939b7aaa` |
| [ADR0243](../../../adr/0243-evaluate-generated-abstractions-with-bounded-probes.md) | #346, #366 | `1f35ac47`, `24fa6639`, `4c5eb5de` |
| [ADR0244](../../../adr/0244-test-owned-semantic-boundaries-before-claiming-code-quality.md) | #368 | `c70dd63c` |
| [ADR0245](../../../adr/0245-validate-circuit-names-before-constructing-call-maps.md) | #369 | `c70dd63c` |
| [ADR0246](../../../adr/0246-reject-circuit-collisions-in-emitted-rust-namespaces.md) | #370 | `c70dd63c` |
| [ADR0247](../../../adr/0247-make-macro-name-resolution-contracts-explicit.md) | #371, #374 | `77c9ee14` |
| [ADR0248](../../../adr/0248-own-plain-ledger-scenarios-in-contractlab.md) | #373 | `ceaac44c` |
| [ADR0249](../../../adr/0249-isolate-generated-struct-representation-from-source-names.md) | #374 | `ee768a60` |
| [ADR0250](../../../adr/0250-adopt-pinned-passport-pure-apis-through-independent-oracles.md) | #375 |  |
| [ADR0251](../../../adr/0251-compose-audited-unit-helpers-over-complete-ledger-paths.md) | #376 | `67554755`, `77c9ee14` |
| [ADR0252](../../../adr/0252-characterize-a-coherent-ledger8-1-compatibility-profile.md) | #360, #377 |  |
| [ADR0253](../../../adr/0253-validate-exported-aliases-against-emitted-module-names.md) | #378 | `2c34c78a` |
| [ADR0254](../../../adr/0254-give-circuit-dependency-analysis-an-explicit-owner.md) | #345, #349, #379 | `24fa6639` |
| [ADR0255](../../../adr/0255-establish-original-did-lifecycle-and-direct-api-oracles.md) | #351, #353, #355 | `775907ea`, `9ec520a3` |
| [ADR0256](../../../adr/0256-measure-a-bounded-generated-rust-webassembly-consumer.md) | #359 |  |
| [ADR0257](../../../adr/0257-reuse-runtime-frame-ownership-for-simple-native-leaves.md) | #346, #348, #349, #366 | `012fbb44` |
| [ADR0258](../../../adr/0258-exercise-merkle-scenarios-through-contractlab.md) | #350 | `8490847f`, `c43e8fd3` |
| [ADR0259](../../../adr/0259-record-point-authorization-transitions-through-shared-composition.md) |  | `012fbb44`, `51b9ea43` |
| [ADR0261](../../../adr/0261-make-rust-compatibility-and-runtime-selection-explicit.md) |  | `012fbb44`, `987b56b1` |
| [ADR0262](../../../adr/0262-exercise-adopted-did-recording-through-contractlab.md) | #350, #352 | `0b297501`, `c43e8fd3` |
| [ADR0263](../../../adr/0263-give-closed-pure-call-recording-policies-an-explicit-owner.md) | #349, #351 | `6e78bd2b`, `c43e8fd3` |
| [ADR0264](../../../adr/0264-share-semantic-rust-function-names-across-declarations-and-helpers.md) |  | `c43e8fd3`, `f103fb34` |
| [ADR0265](../../../adr/0265-compose-opaque-string-set-mutations-through-typed-recording.md) | #353 | `23d5e8ca`, `51b9ea43` |
| [ADR0266](../../../adr/0266-preserve-checked-recording-profile-rejection-reasons.md) |  | `9949b013` |
| [ADR0267](../../../adr/0267-test-runtime-source-selection-and-repeated-helper-execution-boundaries.md) | #351, #361, #391 | `0b297501`, `82e1336d` |
| [ADR0268](../../../adr/0268-register-did-proof-scenarios-in-the-local-full-gate.md) | #353, #363, #392 | `6b5e98e8` |
| [ADR0269](../../../adr/0269-record-opaque-string-product-maps-through-shared-typed-composition.md) | #353, #355, #393 | `23d5e8ca`, `9949b013`, `a536360a` |
| [ADR0270](../../../adr/0270-validate-every-declared-midnight-dependency-context.md) | #357, #360, #394 | `91bf9a15` |
| [ADR0271](../../../adr/0271-classify-and-test-every-local-helper-context-boundary.md) | #347, #349, #351, #357, #395 | `095b67c5` |
| [ADR0272](../../../adr/0272-reject-explicit-discriminants-in-compact-cell-enums.md) | #347, #348, #351, #357, #396 | `6250d454` |
| [ADR0273](../../../adr/0273-explain-script-failure-privacy-and-distinct-replay-costs.md) | #351, #352, #357, #397 | `dc745e70` |
| [ADR0274](../../../adr/0274-preserve-default-worker-stack-in-typed-map-lowering.md) | #349, #362, #393, #398 | `a536360a` |
| [ADR0275](../../../adr/0275-own-exhaustive-local-call-validation-in-circuitcontext.md) | #347, #349, #357, #399 | `91bf9a15`, `f965a646` |
| [ADR0276](../../../adr/0276-prototype-generated-named-argument-facades.md) | #346, #347, #352, #400 | `f0775cc6`, `fca7577b` |
| [ADR0277](../../../adr/0277-separate-closed-guarded-mutation-profiles-from-recording-assembly.md) | #349, #401 | `fca7577b` |
| [ADR0278](../../../adr/0278-record-flat-point-product-maps-and-boolean-membership-helpers.md) | #353, #355, #393, #398, #404 | `7d65bd8e`, `a536360a`, `d5d4a6c9`, `fca7577b` |
| [ADR0279](../../../adr/0279-give-typed-value-lowering-a-cohesive-compiler-owner.md) | #349, #402 | `d5d4a6c9`, `fca7577b` |
| [ADR0280](../../../adr/0280-separate-native-state-expressions-and-public-facade-from-circuit-assembly.md) | #349, #403 | `d5d4a6c9`, `fca7577b` |
| [ADR0281](../../../adr/0281-track-semantic-rust-names-in-public-parameter-allocation.md) | #346, #347, #351, #405 | `f0775cc6` |
| [ADR0282](../../../adr/0282-diagnose-renderer-stack-exhaustion-on-a-supported-source.md) | #361, #406 | `1f9b1394`, `d5d4a6c9` |
| [ADR0283](../../../adr/0283-record-nested-jwk-verificationmethod-map-crud.md) | #407 | `1f9b1394`, `770f8dcb`, `7d65bd8e` |
| [ADR0284](../../../adr/0284-measure-lexical-binding-identity-against-simpler-shared-scopes.md) | #345, #346, #408 | `1f9b1394` |
| [ADR0285](../../../adr/0285-qualify-typed-consumer-safety-and-pinned-upstream-trust-boundaries.md) | #347, #409 | `4c8aebc3` |
| [ADR0286](../../../adr/0286-classify-checked-unit-composition-attempts.md) | #345, #410 | `1f9b1394` |
| [ADR0287](../../../adr/0287-compare-unchanged-did-against-its-original-release-compiler-and-runtime.md) | #411 |  |
| [ADR0288](../../../adr/0288-record-original-did-digest-verification-through-checked-read-and-local-unit-helper.md) | #413 | `770f8dcb`, `8a52a010` |
| [ADR0289](../../../adr/0289-verify-a-public-did-transaction-across-ledger8-0-3-and-ledger8-1.md) | #353, #360, #412 |  |
| [ADR0290](../../../adr/0290-qualify-the-existing-did-effect-families-across-ledger8-profiles.md) | #353, #360, #414 |  |
| [ADR0291](../../../adr/0291-bound-compiler-ingress-and-typed-rendering-work.md) | #361, #362, #415 | `03445d6b`, `c8450ee3`, `e79c639f` |
| [ADR0292](../../../adr/0292-reduce-recursive-pure-renderer-frames.md) | #361, #362, #417 | `1705fc1a`, `8a52a010`, `926d2bf5` |
| [ADR0293](../../../adr/0293-separate-rust-release-versions-and-migrate-public-consumers-explicitly.md) | #416 | `329bf1bc`, `b6fcb06c` |
| [ADR0294](../../../adr/0294-maintain-executable-and-strictly-proved-did-primitive-reducers.md) | #418 | `40047fb8` |
| [ADR0295](../../../adr/0295-record-original-did-relation-mutation-through-checked-selected-sets-and-nested-jwk-reads.md) | #419 | `329bf1bc`, `8a52a010`, `b6fcb06c` |
| [ADR0296](../../../adr/0296-reduce-legacy-recorded-action-recursion-frames.md) | #361, #362, #420 | `40047fb8`, `f6a87df1` |
| [ADR0297](../../../adr/0297-measure-generated-consumers-with-isolated-phase-boundaries.md) | #362, #421 | `dce3c9e9`, `ec10f324` |
| [ADR0298](../../../adr/0298-share-fixture-selection-and-require-maintained-reducer-proof-gates.md) | #182, #193, #363, #422 | `1b6c5d02` |
| [ADR0299](../../../adr/0299-reduce-native-recursive-operation-frames.md) | #361, #362, #423 | `ca97a69f`, `f6a87df1` |
| [ADR0300](../../../adr/0300-qualify-original-did-digest-public-transactions-on-ledger8-1.md) | #424 |  |
| [ADR0301](../../../adr/0301-triage-dependency-advisories-without-ledger-pin-drift.md) | #357, #363, #364, #425 | `c8450ee3`, `ca97a69f`, `e79c639f` |
| [ADR0302](../../../adr/0302-exercise-value-lowering-boundaries-through-focused-unit-tests.md) | #351, #426 | `c8450ee3`, `d5d4a6c9`, `fa7600bc` |
| [ADR0303](../../../adr/0303-qualify-original-did-relation-transactions-through-the-ledger-8-1-public-bridge.md) | #353, #360, #427 | `329bf1bc`, `b6fcb06c` |
| [ADR0304](../../../adr/0304-align-the-standalone-backend-lock-with-the-qualified-workspace-graph.md) | #360, #363, #428 | `329bf1bc`, `b6fcb06c` |
| [ADR0305](../../../adr/0305-pin-passport-acc-pr177-and-preserve-authorization-across-the-ledger8-port.md) | #175, #356, #429, #436, #438, #439 |  |
| [ADR0306](../../../adr/0306-preserve-acc-jubjub-scalar-cast-reduction-on-ledger8.md) | #356, #429, #430, #432, #434 | `dce3c9e9`, `debb05f9`, `ec10f324` |
| [ADR0307](../../../adr/0307-test-resource-accounting-across-typed-ir-carriers.md) | #351, #415, #431 | `ec10f324` |
| [ADR0308](../../../adr/0308-record-typed-jubjub-value-operations-and-a-point-cell-write.md) | #356, #429, #430, #432, #434 | `dce3c9e9`, `ec10f324` |
| [ADR0309](../../../adr/0309-qualify-pinned-secp256k1-chip-proof-transport-through-the-ledger8-verifier.md) | #356, #429, #433 |  |
| [ADR0310](../../../adr/0310-probe-p256-prime-modular-multiplication-using-existing-ledger8-integer-gadgets.md) | #356, #429, #434 |  |
| [ADR0311](../../../adr/0311-exercise-stateful-expression-diagnostics-through-the-real-dispatcher.md) | #351, #435 | `3361d696`, `dce3c9e9`, `ec10f324`, `f1140869` |
| [ADR0312](../../../adr/0312-measure-a-complete-p256-variable-base-scalar-component-on-ledger8.md) | #356, #429, #436, #438 |  |
| [ADR0313](../../../adr/0313-exclude-p256-and-webauthn-from-the-ledger8-acc-variant.md) | #356, #429, #436, #438, #439 |  |
| [ADR0314](../../../adr/0314-restrict-acc-authorization-to-jubjub-and-bound-adoption-work.md) | #356, #429, #438, #439 |  |
| [ADR0315](../../../adr/0315-preserve-ledger8-time-checks-in-the-reduced-passport-profile.md) | #356, #429, #439, #440 | `eb72a5ab` |
| [ADR0316](../../../adr/0316-remove-acc-adoption-while-retaining-generic-backend-evidence.md) | #356, #429, #441, #442 |  |
| [ADR0317](../../../adr/0317-accept-the-scoped-performance-and-resource-baseline.md) | #362 | `dce3c9e9` |
| [ADR0318](../../../adr/0318-test-collection-query-admission-and-effect-ordering.md) | #351, #443 | `d7c6b1ac` |
| [ADR0319](../../../adr/0319-exercise-generated-reducer-control-exports-directly.md) | #351, #444 | `d404d8c1` |
| [ADR0320](../../../adr/0320-test-effect-operation-domain-admission.md) | #351, #445 | `49898aed` |
| [ADR0321](../../../adr/0321-reconcile-compatibility-policy-and-qualify-the-msrv-delta.md) | #360, #409, #446 | `5efa91c2` |
| [ADR0322](../../../adr/0322-preserve-pure-helper-admission-through-conversion-wrappers.md) | #351, #447 | `c924a4e1` |
| [ADR0323](../../../adr/0323-bind-legacy-oracle-and-constructor-assertions-to-current-execution.md) | #351, #448 | `5efa91c2` |
| [ADR0324](../../../adr/0324-consolidate-current-developer-guides-without-rewriting-history.md) | #352, #449 | `74d04c1a` |
| [ADR0325](../../../adr/0325-stabilize-a-bounded-rust-ci-lane-at-the-midpoint.md) | #352, #363, #449, #450, #451 | `bf7c719b`, `fba6845f` |
| [ADR0326](../../../adr/0326-preserve-input-read-failures-and-document-owned-witness-scripts.md) | #351, #352, #409, #449, #450, #451 |  |
| [ADR0327](../../../adr/0327-independent-compiler-architecture-review-at-the-midpoint.md) | #357, #452 | `bf7c719b` |
| [ADR0328](../../../adr/0328-exercise-regression-fixture-helpers-through-public-rust-apis.md) | #351, #453 | `a162144f` |
| [ADR0329](../../../adr/0329-accept-the-bounded-generated-api-research-portfolio.md) | #346, #364 | `bf7c719b` |
| [ADR0330](../../../adr/0330-preserve-fallible-vector-coercions-without-expanding-generated-arrays.md) | #357, #452 | `a136f58f`, `dbfd7dc2` |
| [ADR0331](../../../adr/0331-check-generated-facade-and-alias-names-in-the-rust-namespace.md) | #357, #452 | `300a5cd0`, `dbfd7dc2` |
| [ADR0332](../../../adr/0332-reject-recursive-pure-calls-at-the-private-ir-admission-boundary.md) | #357, #452 | `dbfd7dc2` |
| [ADR0333](../../../adr/0333-preserve-historical-passport-separately-from-current-adoption.md) | #351, #457 |  |
| [ADR0334](../../../adr/0334-execute-witness-conditions-and-witness-produced-collection-operands.md) | #351, #458, #459 | `6dd4e965`, `827ac965` |
| [ADR0335](../../../adr/0335-require-expected-errors-in-negative-generated-code-controls.md) | #351, #458, #459 | `6dd4e965`, `827ac965` |
| [ADR0336](../../../adr/0336-preserve-empty-vector-element-types-during-rust-emission.md) | #452, #460 | `dbfd7dc2`, `e35688eb` |
| [ADR0337](../../../adr/0337-avoid-writable-executable-copies-in-installation-layout-tests.md) | #363, #452, #458, #459, #460, #461 | `31a02ebe`, `dbfd7dc2` |
| [ADR0338](../../../adr/0338-anchor-generated-value-tests-to-independent-expected-values.md) | #351, #462, #463, #464 | `22ef5f23`, `c8406a4d`, `c921622d` |
| [ADR0339](../../../adr/0339-record-reusable-local-gate-tool-and-log-provenance.md) | #363, #462, #463, #464 | `22ef5f23`, `c8406a4d`, `c921622d` |
| [ADR0340](../../../adr/0340-assert-source-defined-returns-in-chunked-ledger-tests.md) | #351, #462, #463, #464 | `22ef5f23`, `c8406a4d`, `c921622d` |
| [ADR0341](../../../adr/0341-verify-bounded-call-graph-resource-accounting-with-seeded-properties.md) | #361 | `22e2689c` |
| [ADR0342](../../../adr/0342-check-post-removal-state-and-pure-helper-defaults-independently.md) | #351 | `66dc6fd5` |
| [ADR0343](../../../adr/0343-preserve-previous-compiler-output-when-publication-fails.md) | #351, #361 |  |
| [ADR0344](../../../adr/0344-pin-compiler-field-admission-at-the-ledger8-modulus.md) | #351, #361 |  |
| [ADR0345](../../../adr/0345-review-remaining-dependency-notices-against-shipped-feature-graphs.md) | #357, #364, #409, #469, #471, #474 | `f3526275` |
| [ADR0346](../../../adr/0346-exercise-generated-field-arithmetic-across-the-modulus.md) | #351, #409, #469, #470 | `5a9cd356`, `f3526275` |
| [ADR0347](../../../adr/0347-track-versioned-compact-specification-conformance-separately-from-fixtures.md) | #351, #361, #409, #475 | `1aa60622`, `f3526275` |
| [ADR0348](../../../adr/0348-qualify-generated-boolean-values-and-witness-failure-order.md) | #351, #361, #409, #476 | `cb687ddc` |
| [ADR0349](../../../adr/0349-verify-shared-boolean-typing-through-both-public-compiler-targets.md) | #351, #361, #409, #477 | `3455f2e9` |
| [ADR0350](../../../adr/0350-stage-a-portable-developer-documentation-package-for-closeout.md) | #352, #409, #449, #478 |  |
| [ADR0351](../../../adr/0351-check-merkle-vm-path-operands-before-program-construction.md) | #351, #357, #361, #479 | `e6807884` |
| [ADR0352](../../../adr/0352-distinguish-unsupported-wide-operations-from-malformed-uint-bounds.md) | #346, #357 | `6aa345fd`, `e6807884` |
| [ADR0353](../../../adr/0353-admit-serialized-ledger-inputs-with-explicit-byte-limits.md) | #351, #357, #361 | `16f5b55d`, `e6807884` |
| [ADR0358](../../../adr/0358-address-the-vm-context-relative-to-qualified-set-path-depth.md) | #351, #357, #361, #493 | `e6807884`, `ef352bcd` |
| [ADR0359](../../../adr/0359-select-a-practical-ledger-decoding-byte-policy.md) | #361, #364, #494, #495 |  |
| [ADR0360](../../../adr/0360-compose-official-ledger8-providers-in-an-experimental-client-model.md) | #352, #358, #364, #496, #497 | `17433b40` |
| [ADR0361](../../../adr/0361-preserve-upstream-error-assertions-in-strict-path-tests.md) | #364, #498 | `2c798d2d` |
| [ADR0362](../../../adr/0362-test-output-recovery-after-runtime-compatibility-admission.md) | #364, #499 | `679be285` |
| [ADR0363](../../../adr/0363-reconcile-did-gate-inventories-with-delivered-recording-support.md) | #364, #413, #427, #500 | `57fab775` |
| [ADR0364](../../../adr/0364-refresh-current-conformance-anchors-without-rewriting-historical-evidence.md) | #364, #501 | `57fab775` |
| [ADR0365](../../../adr/0365-retain-subprocess-exit-status-in-compatibility-failures.md) | #364, #502 | `551f065c` |
| [ADR0366](../../../adr/0366-create-fresh-relation-proof-key-directories.md) | #364, #503 | `f9a49666` |
