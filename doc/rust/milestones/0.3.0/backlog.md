# 0.3.0 draft delivery backlog

These are **20 proposed work packages**, preserving the user's numbers 1–15 and adding 16–20. They are issue-ready specifications, not GitHub issues or accepted ADRs. Split implementation into small slices after scope review; use the next available ADR identifier (currently after 0240), with problem/before-after/ownership/verification before implementation. Confirmed choices: DID v0.7.0, midnight-vc-passport as the digital-passport integration root, ACC deferred until their adoption, and independent external coding-agent review (not human certification).

[Milestone plan](README.md) · [Source targets](targets.md) · [Machine-readable dependency graph](backlog.json)

Phases: **A** source/compatibility/threat baselines and feasibility; **B** testkit and typed architecture verticals; **C** contract adoption; **D** hardening/docs/audits/WASM; **E** final qualification and closure. DID and digital-passport source baselines are phase A; ACC remains a deferred placeholder until their adoption passes. Dependencies mean the named readiness checkpoint, not completion of the whole package. Testkit demonstrations, generated-code follow-up probes and full coverage mature during adoption; R030-20 checks final completion independently.

## R030-01 — Crystallize runtime and compiler domain models

**Phase:** B · **Outcome:** required · **ADR for design changes:** yes

**Start after these readiness checkpoints:** R030-16: version-and-source-baseline, R030-18: measurement-baseline

Write the invariant/ownership map, then consolidate one audited domain at a time: resolved lexical identities → structured admission → checked effect plan → Rust AST. Keep upstream ledger/zk as semantic owners; avoid a runtime rewrite.

### Acceptance

- [ ] Document ownership of source typing, declared slots, witness work, VM execution, recording, offer binding, proving and observation trust. Identify public vs internal extension points.
- [ ] Distinguish not-applicable, malformed/rejected and admitted lowering; diagnostics retain source/callee context and never silently accept after a rejected guard.
- [ ] Migrate representative pure, read-only, witnessed branch and funded composition slices with exact old accept/refuse characterization; preserve ordering, short circuit, alignment and policy boundaries.
- [ ] Use explicit binding/circuit IDs and typed operands where the probe proves their value; retain source provenance and reject shadow/scope leakage. Keep domain changes in small ADR-led slices.

## R030-02 — Improve generated Rust through repeated design probes

**Phase:** B/C · **Outcome:** required · **ADR for design changes:** yes

**Start after these readiness checkpoints:** R030-01: first-checked-plan-vertical, R030-06: native-replay-core-ready, R030-18: measurement-baseline

Run before/after probes at baseline, after the first API/testkit slice, after DID/digital-passport integration and before RC. Research reusable helpers, source-linked names, modules, argument records, derives and allocation/clone behavior.

### Acceptance

- [ ] Each candidate records the problem, real emitted before/after code, ordinary Rust alternative, affected emitter/runtime ownership and ABI implications in an ADR.
- [ ] Use unedited generated crates in external consumers; measure expanded and displayed source, build time, stack/memory and diagnostics.
- [ ] Preserve effect/witness order, variable hygiene, early errors and default debug-worker execution. Keep rejected experiments and their measurements.
- [ ] Adopt improvements only with measured usability or maintenance benefit; fewer lines or more macros alone does not pass.

## R030-03 — Strengthen typed primitives and consumer safety

**Phase:** B · **Outcome:** required · **ADR for design changes:** yes

**Start after these readiness checkpoints:** R030-01: first-checked-plan-vertical, R030-16: version-and-source-baseline, R030-17: threat-model-and-test-seeds

Prioritize checked scalar/domain conversion, generated circuit argument descriptors, checked offer-policy constructors and narrower helper capabilities; keep runtime checks at untrusted boundaries.

### Acceptance

- [ ] List concrete prevented misuses and compare them with the equivalent TS boundary. Use compile-fail tests for static guarantees and malformed-input tests for dynamic guarantees.
- [ ] Preserve canonical vs reduced scalar semantics, declared integer bounds, domain separation, verifier/address/input binding and exact offer ownership/allocation.
- [ ] Distinguish caller-provided identity, observed provenance and authenticated ownership; avoid claiming types establish finality or wallet authority.
- [ ] No new unsafe code in owned runtime; audit dependency unsafe boundaries and FFI separately. No implicit fallback from strict preparation to a weaker policy.

## R030-04 — Consolidate mechanical repetition with selective macros and derives

**Phase:** B/C · **Outcome:** required · **ADR for design changes:** yes

**Start after these readiness checkpoints:** R030-01: first-checked-plan-vertical, R030-02: baseline-codegen-probes, R030-18: measurement-baseline

Inventory duplicate representation/witness/argument glue and compare ordinary functions, generic helpers, derives and small attributes. Keep semantic analysis and full circuit bodies outside consumer macro magic.

### Acceptance

- [ ] Publish a duplication inventory with ownership and selected/declined abstraction decisions.
- [ ] Chosen derives/macros preserve FAB/hash/ordinal/layout behavior, generics where supported, hygiene and source-span diagnostics; positive and compile-fail expansion tests exist.
- [ ] Prefer normal functions for shared VM/gas/context mechanics; retain actual observed aligned atoms and canonical upstream programs.
- [ ] Report macro expansion/build effects and examples. An evidence-backed decision to retain explicit Rust is a valid outcome for a candidate.

## R030-05 — Resolve concrete SOLID, KISS and DRY design gaps

**Phase:** B/C · **Outcome:** required · **ADR for design changes:** yes

**Start after these readiness checkpoints:** R030-01: first-checked-plan-vertical, R030-16: version-and-source-baseline, R030-17: threat-model-and-test-seeds

Decompose mixed responsibilities and overlapping semantic evaluators based on measured change risk. Keep interfaces narrow and explicit rather than adding generic trait hierarchies to satisfy slogans.

### Acceptance

- [ ] Inventory high-risk coupling and duplicated invariants with file references and tests that characterize them.
- [ ] Preserve native/recorded obligations while sharing semantic leaves and checked plans where valid; remove a duplicate only after differential proof.
- [ ] New context fields require explicit classification in helper/recording safety boundaries; error handling does not rely on string sniffing.
- [ ] Each refactor has a bounded ownership change, no accidental public API break and documented before/after maintenance benefit.

## R030-06 — Deliver ContractLab: the generated-contract testkit

**Phase:** A/B · **Outcome:** required · **ADR for design changes:** yes

**Start after these readiness checkpoints:** R030-16: version-and-source-baseline, R030-17: threat-model-and-test-seeds

Propose crate midnight-compact-testkit and API ContractLab. Use generated typed methods/witness traits over real ledger-8 execution, without a second VM or a general contract DSL.

### Acceptance

- [ ] Native and recorded/replay scenarios support explicit clock/address/gas/identity, deterministic fixture seeds, typed witness answers/errors/journals and owned snapshots/forks.
- [ ] Snapshot restore checks version, source/runtime/ledger identities and state mode; parallel scenarios have no global mutable witness registry.
- [ ] Call reports preserve typed output, state/effects, ordered operations/private alignment and separately named query/replay gas; private output is redacted by default.
- [ ] Failure keeps committed lab state at its checkpoint; do not invent partial VM traces or promise rollback of external witness side effects.
- [ ] Optional strict proof/ledger adapter and explicit network tier have separate requirements and receipts. Demonstrate Counter, witnessed Cell, collections, Merkle and the adopted real contracts.

## R030-07 — Establish meaningful generated-code coverage

**Phase:** B/C · **Outcome:** required · **ADR for design changes:** yes

**Start after these readiness checkpoints:** R030-06: native-replay-core-ready, R030-16: version-and-source-baseline, R030-17: threat-model-and-test-seeds, R030-19: focused-pipeline-ready

Publish a source/export/case/dimension matrix plus instrumented coverage. Preserve independent TS observations; never infer behavior or proofs from emitted API availability.

### Acceptance

- [ ] Classify 100% of pinned required sources/imports/exports and regenerate the release fixture cohort; compilation, execution, replay, proof and network evidence are separate.
- [ ] Every required contract export (and constructor, where present) has a direct typed success invocation or intentional error where success is impossible by design; security-sensitive exports also have negative and state-transition cases.
- [ ] Cover every enumerated changed semantic/security obligation and effectful branch; no exclusion may conceal a mandatory contract primitive.
- [ ] Proposed instrumentation floor: 90% lines in new testkit-owned code and 95% changed lines in owned runtime/backend/testkit code, measured per component after baseline; exclusions require rationale. Generated line/branch coverage is diagnostic alongside export coverage.
- [ ] Keep stable line/region reports; branch coverage uses a separately pinned compatible nightly if adopted. Preserve exact toolchain, features, source hashes and denominators.

## R030-08 — Create developer-friendly Rust backend documentation

**Phase:** D · **Outcome:** required · **ADR for design changes:** yes

**Start after these readiness checkpoints:** R030-02: baseline-codegen-probes, R030-03: first-typed-safety-vertical, R030-06: native-replay-core-ready, R030-09: adoption-complete, R030-10: adoption-complete, R030-12: activation-or-deferral-recorded, R030-16: version-and-source-baseline

Publish a task-oriented guide under doc/rust, rustdoc examples and runnable standalone consumers; separate tutorials from architecture/history.

### Acceptance

- [ ] Cover install/version selection, compile, generated types/witnesses, ContractLab, errors, recording, proof/ledger integration and trust boundaries.
- [ ] Provide DID and digital-passport walkthroughs, ACC scope/status notes (and ledger-8 migration notes only if activated), TS-to-Rust examples, troubleshooting and supported-target/version matrices.
- [ ] All claimed runnable snippets build/test from a clean external directory without checkout-relative runtime paths; compile-fail examples test documented safety constraints.
- [ ] Keep ADRs as decision history and link current guides to the decisions. Document generated API migration and limitations precisely.

## R030-09 — Adopt Midnight DID v0.7.0

**Phase:** C · **Outcome:** required · **ADR for design changes:** yes

**Start after these readiness checkpoints:** R030-01: first-checked-plan-vertical, R030-02: baseline-codegen-probes, R030-03: first-typed-safety-vertical, R030-06: native-replay-core-ready, R030-07: coverage-matrix-ready, R030-11: reducer-workflow-ready, R030-16: version-and-source-baseline

Use the user-selected midnightntwrk/midnight-did v0.7.0 release and its full import closure, compile unchanged Compact to Rust where compatible and port its maintained TS scenario semantics into direct typed Rust tests.

### Acceptance

- [ ] Resolve the v0.7.0 tag to an immutable commit and freeze source/import hashes, lockfile, compiler/language/runtime/ledger versions and exported signatures. Later DID releases do not silently replace the user-selected baseline.
- [ ] All required constructor/exports compile and execute through unedited generated Rust; no handwritten generated-source patches or disabled authentication.
- [ ] Port positive, wrong-authority, duplicate/missing-key, lifecycle and cryptographic-boundary scenarios that exist in the pinned contract; assert typed output, state, failures and witness effects.
- [ ] For every proof-applicable export run a valid default-strict proof/verify/ledger-apply case; cover each distinct authorization/effect path and critical lifecycle, with network acceptance separately recorded.
- [ ] Every compatibility failure follows R030-11. No unresolved required primitive or export remains at closure.

## R030-10 — Adopt the latest maintained digital-passport contract

**Phase:** C · **Outcome:** required · **ADR for design changes:** yes

**Start after these readiness checkpoints:** R030-01: first-checked-plan-vertical, R030-02: baseline-codegen-probes, R030-03: first-typed-safety-vertical, R030-06: native-replay-core-ready, R030-07: coverage-matrix-ready, R030-11: reducer-workflow-ready, R030-16: version-and-source-baseline

Use user-selected midnightntwrk/midnight-vc-passport as the integration root. Inspect its latest contributions and follow its actual declared contract/core dependency, preserving the source/import closure and maintained TS tests and consumer scenarios.

### Acceptance

- [ ] Pin midnight-vc-passport consumer commit, latest relevant contributions and exact resolved contract/core package and source hashes. Distinguish merged work from open PRs; do not replace the declared dependency with a similarly named repository by assumption.
- [ ] All compiled required pure exports compile and execute through unedited generated Rust, including composed imported functions.
- [ ] Port claim-root, age/date-boundary, selective-disclosure/request, private-part opening, holder-binding and issuance/verification envelope cases from the pinned pure family. Assert invalid claims/authorization and context matching; do not invent ledger state transitions absent from the source.
- [ ] Classify the compiled pure family honestly: direct typed value/error and cross-language codec evidence is mandatory; proof/verify/ledger-apply is required only for actual proof-applicable exports. If none exist, record zero with authoritative compiler metadata; do not fabricate recorded calls or ledger application.
- [ ] R030-11 resolves every failed required primitive; no silently narrowed feature set qualifies as adoption.

## R030-11 — Turn contract compatibility failures into minimal regression slices

**Phase:** A/C · **Outcome:** required · **ADR for design changes:** yes

**Start after these readiness checkpoints:** R030-06: native-replay-core-ready, R030-16: version-and-source-baseline, R030-19: focused-pipeline-ready

Build a reducer/triage workflow joining real source exports to the failing compiler IR, primitive, runtime operation, upstream version and independent oracle evidence.

### Acceptance

- [ ] Each failure has original source hash/export/input, expected vs actual behavior, minimized .compact reproducer and first failing layer.
- [ ] Create a focused issue and ADR when design changes; add renderer/admission negative tests, a generated-crate behavior test and TS/upstream comparison as appropriate.
- [ ] Fix the smallest semantic cause using upstream primitives; for proof-applicable reproducers, prove/verify/default-strict apply the snippet, and always re-run the original contract scenario. Pure/diagnostic/non-proof reproducers retain their legitimate proof refusal or not-applicable classification.
- [ ] Unsupported source diagnostics remain typed and actionable; no blanket denylist or fixture-specific name match substitutes for the fix.
- [ ] Preserve the full contract regression after minimization, including any failure that exposes a TS/upstream defect rather than a Rust defect.

## R030-12 — Placeholder: Passport ACC adoption after DID and digital passport

**Phase:** after C · **Outcome:** deferred-placeholder · **ADR for design changes:** yes

**Start after these readiness checkpoints:** R030-09: adoption-complete, R030-10: adoption-complete

User-directed deferred placeholder. After R030-09 and R030-10 pass, select the ACC source, inspect its ledger9/Compact requirements and propose a ledger8 downgrade/adoption scope before implementation.

### Acceptance

- [ ] Keep this work explicitly deferred while DID and digital passport are being adopted; no ACC port, dependency downgrade or expensive proof work starts in the first waves.
- [ ] Activation review pins the chosen ACC source and inventories account/device/recovery/authentication exports, P-256/WebAuthn, caller identity, cross-contract interfaces and ZKIR/resource differences.
- [ ] Propose a reviewed compatibility patch and security-equivalence matrix. Unsupported primitives cannot be dropped or replaced by trusted witnesses while calling the result full adoption.
- [ ] Before final milestone closure, record whether ACC was activated with its own complete acceptance gates or remains explicitly deferred by owner decision. A placeholder is not evidence that ACC works on ledger8.

## R030-13 — Complete internal and independent external audits

**Phase:** D · **Outcome:** required · **ADR for design changes:** yes

**Start after these readiness checkpoints:** R030-01: first-checked-plan-vertical, R030-02: baseline-codegen-probes, R030-03: first-typed-safety-vertical, R030-04: selected-abstractions-reviewed, R030-05: initial-ownership-refactor, R030-06: native-replay-core-ready, R030-07: coverage-matrix-ready, R030-08: consumer-guides-ready, R030-09: adoption-complete, R030-10: adoption-complete, R030-11: reducer-workflow-ready, R030-12: activation-or-deferral-recorded, R030-15: prototype-disposition-recorded, R030-16: version-and-source-baseline, R030-17: threat-model-and-test-seeds, R030-18: measurement-baseline, R030-19: focused-pipeline-ready

Use one requirements/threat-model checklist for architecture, security, correctness, performance and operational behavior, with independent evidence and tracked remediation.

### Acceptance

- [ ] Internal review covers semantic ownership, IR/admission, generated output, codecs, witness/privacy, offer/proof/observation trust and failure behavior.
- [ ] Independent external coding-agent review uses a separately configured tool/model and exact frozen source; implementing agents do not grade their own changes. The user selected independent external coding-agent review; this is not a human security audit.
- [ ] Each finding has severity, reproducer/source evidence, owner, remediation and independent retest; no open critical/high security or correctness finding at closure.
- [ ] Medium/low findings have explicit disposition and residual-risk documentation; neither tests nor agent review are called a human security certification.
- [ ] Repeat relevant review after fixes and preserve architecture/functional/nonfunctional checklists in the closure bundle.

## R030-14 — Compose the 0.3.0 closure report

**Phase:** E · **Outcome:** required · **ADR for design changes:** as needed

**Start after these readiness checkpoints:** R030-20: candidate-qualified

Write a publishable engineering report with the final domain model, generated code examples, compatibility matrix, audits, performance and precise coverage/parity evidence.

### Acceptance

- [ ] Map every deliverable, issue, ADR, commit and acceptance criterion to an immutable result or explicit experimental outcome.
- [ ] Include ACC deferred/activated scope and provenance (if activated), direct DID/passport results, code/ABI migration notes and supported target/feature matrices.
- [ ] Separate inventory, sampled behavior, source branch obligations, replay, strict proofs, live runs and WASM feasibility; record failed/rejected approaches.
- [ ] Provide final known limits, audit residuals, local/remote exact-commit results and developer entry points; reconcile Git and Obsidian.
- [ ] Close only after R030-20 passes; documentation does not convert blocked required contract work into completion.

## R030-15 — Prototype generated Rust in Node.js and browsers through WASM

**Phase:** B/D · **Outcome:** bounded-prototype · **ADR for design changes:** yes

**Start after these readiness checkpoints:** R030-01: first-checked-plan-vertical, R030-06: native-replay-core-ready, R030-16: version-and-source-baseline, R030-18: measurement-baseline

Treat “wast” as WebAssembly wasm, pending correction. Test wasm32-unknown-unknown with thin wasm-bindgen adapters; WASI is a separate comparison, not browser proof.

### Acceptance

- [ ] Resolve the exact target-specific dependency/feature graph; dependency presence in Cargo.lock alone is not proof of a target blocker.
- [ ] Probe untouched pure arithmetic/hash and Counter/witnessed-Cell generated crates in pinned Node and headless browser engines, using actual VM semantics.
- [ ] Test large-integer/bytes marshalling, results/state/replay, witness order and error mapping; no lossy JS Number conversion for Field/u128.
- [ ] Record module/import graph, compressed size, startup, peak linear memory and per-call measurements; fixture entropy and production entropy remain separate.
- [ ] Exit with a validated supported subset or an exact blocker/minimal reproducer plus proposed dependency boundary. No browser-prover or production WASM support promise follows from feasibility alone.

## R030-16 — Define version, compatibility and migration contracts

**Phase:** A · **Outcome:** required · **ADR for design changes:** yes

**Start after these readiness checkpoints:** none

Keep milestone 0.3.0 separate from compiler 0.31.133, Rust package 0.1.0, runtime ABI 49, private IR 20, capability schema 3 and ledger8 pins until a versioning ADR defines the release mapping.

### Acceptance

- [ ] Publish a machine-readable compatibility/source matrix and feature policy for Rust-only, ledger transaction, testkit and experimental WASM consumers.
- [ ] Validate the declared MSRV independently from the tested Rust 1.99 toolchain; update the claim or toolchain only with an explicit decision.
- [ ] Decide SemVer changes, generated/runtime pairing, ABI/IR migration and deprecated low-level APIs before breaking public consumers.
- [ ] Compile old/new compatible consumer fixtures; mismatches fail clearly. A ledger9 package must not enter the ledger8 graph unnoticed. DID declares ledger-v8 8.1.0 while the Rust baseline uses ledger 8.0.3: prove the required compatibility or adopt an explicitly reviewed coherent ledger/zk pin update before DID acceptance.
- [ ] Resolve latest-contract policy at implementation start and recheck drift before RC; no moving branch name is the sole provenance.

## R030-17 — Build adversarial, property and differential assurance

**Phase:** A/D · **Outcome:** required · **ADR for design changes:** yes

**Start after these readiness checkpoints:** R030-16: version-and-source-baseline

Create a threat model and finite semantic obligation registry, then property/fuzz/mutation tests around typed admission, codecs, witness boundaries, offer binding and snapshots.

### Acceptance

- [ ] Enumerate malformed IR, scope/slot/type/call mismatches, overflow/canonical scalars, aliasing, invalid FAB, duplicate/foreign offers and observation mismatch cases before changing behavior.
- [ ] Use upstream ledger/zk and independent TS captures as applicable; label known oracle/provider disagreements instead of auto-blessing one backend.
- [ ] Each owned trust boundary has deterministic seeds and minimized regression corpus; no panic or unbounded resource use for supported untrusted-input APIs.
- [ ] Fuzzing runs have reproducible seeds/budgets/results; targeted mutants prove important guards are exercised. Do not disable cryptographic validation in acceptance paths.
- [ ] Security-critical state transitions and privacy output policies retain explicit positive/negative tests and no open critical/high findings.

## R030-18 — Set and measure performance and resource budgets

**Phase:** A/D · **Outcome:** required · **ADR for design changes:** as needed

**Start after these readiness checkpoints:** R030-16: version-and-source-baseline

Baseline compile time, generated size, runtime latency, allocations, memory, default-worker stack and proof resources before changing architecture. Treat design wins as measurements.

### Acceptance

- [ ] Pin compiler/source/lock/toolchain/target/profile/hardware and separate cold dependency builds, warm generated builds, runtime, proof and startup costs.
- [ ] Use small controls plus DID and digital-passport cohorts (ACC only after activation), paired samples and spread; retain the default debug-worker stack regression.
- [ ] Proposed regression trigger: >10% median runtime/warm-build/peak-memory increase on controlled matching cases requires investigation and reviewed acceptance, never a single noisy sample verdict.
- [ ] Measure circuit/prover rows/k/key size when source changes; For a later activated ACC scope, assess resource feasibility before full key generation.
- [ ] Freeze practical ceilings after baseline; publish tradeoffs and rejected macro/helper optimizations rather than asserting Rust always wins.

## R030-19 — Make the local delivery pipeline reproducible and efficient

**Phase:** A/E · **Outcome:** required · **ADR for design changes:** as needed

**Start after these readiness checkpoints:** R030-16: version-and-source-baseline

Reuse the existing focused gate and immutable compiler handoffs. Add only missing scheduling/provenance/selection facilities, with bounded parallel worktree ownership.

### Acceptance

- [ ] Seal compiler, Scheme runtime, lockfile, generated source and fixture identities in receipts; concurrent slices do not share mutable compiler artifacts.
- [ ] Use affected renderer/generated/TS tests per slice; expand for ABI/schema/effects and full integration checkpoints. Measure checks rather than skipping equivalent assertions by assumption.
- [ ] Keep caches scoped to source/toolchain/features/profile; record command selection and prove generated behavior targets really ran.
- [ ] Capture dependency/license/advisory review and artifact checksums; use locked, reproducible external consumers and explicit provider/material prerequisites.
- [ ] Defer broad remote CI until required local implementation and acceptance prerequisites pass (excluding remote qualification and the final report), then qualify the final revision; keep conventional GPG/DCO commits, ADR→issue→milestone links and vault history.

## R030-20 — Qualify the final release candidate and downstream migration

**Phase:** E · **Outcome:** required · **ADR for design changes:** as needed

**Start after these readiness checkpoints:** R030-01: first-checked-plan-vertical, R030-02: baseline-codegen-probes, R030-03: first-typed-safety-vertical, R030-04: selected-abstractions-reviewed, R030-05: initial-ownership-refactor, R030-06: native-replay-core-ready, R030-07: coverage-matrix-ready, R030-08: consumer-guides-ready, R030-09: adoption-complete, R030-10: adoption-complete, R030-11: reducer-workflow-ready, R030-12: activation-or-deferral-recorded, R030-13: audit-remediation-complete, R030-15: prototype-disposition-recorded, R030-16: version-and-source-baseline, R030-17: threat-model-and-test-seeds, R030-18: measurement-baseline, R030-19: focused-pipeline-ready

Freeze one final source/toolchain/dependency/contract cohort and verify independent consumers, supported host platforms, packaged distribution and upgrade paths before calling 0.3.0 complete.

### Acceptance

- [ ] All required deliverables 1–11,13 and 16–19 pass; WASM15 has its documented experimental outcome. ACC12 has an explicit activation/defer disposition; if activated, its approved acceptance gates also pass. No required DID/digital-passport feature remains unsupported.
- [ ] Run the full applicable local suite and required remote checks at the frozen revision, with artifact provenance and no unrelated dirty source in release inputs.
- [ ] Exercise clean generated-crate consumers on declared supported hosts and the asserted MSRV; qualify default and transaction/testkit feature graphs.
- [ ] Validate runtime/API/ABI migration examples and final audit retests; authenticate package inputs and account for all dependency/advisory dispositions.
- [ ] Produce signed/DCO commit and candidate manifest. Tags, registries, upstream PRs and production deployment require their own explicit publication scope; milestone naming alone is not a compiler or package-version bump.
