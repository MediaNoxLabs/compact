# Rust backend and runtime architecture decisions

This is the canonical published **Rust engineering decision collection** for `MediaNoxLabs/compact:codex/rust-backend-ast`. It covers the compiler backend, generated Rust APIs, runtime, macros, ledger/proof integration and delivery boundaries.

**360 published records:** 239 historical records through `RUST-ADR-0240` and 121 milestone 0.3.0 records: 0241–0353 except 0260, plus 0358–0366. Original `ADR-NNNN` aliases are retained. Preserve absent historical IDs **0116 and 0260**; do not renumber records or reuse assigned identifiers. [0022](0022-meter-merkle-witness-vm-reads-while-keeping-local-projections.md) explicitly supersedes [0021](0021-keep-merkle-witness-projections-local.md); other extensions do not automatically supersede earlier decisions.

These are implementation decisions on this branch. Their status does **not** imply upstream LFDT/TSC acceptance, a CoIP number or a released language feature. The separate [CoIP process](../../../coips/coip-0001.md) and [CoIP register](../../../coips/README.md) retain their own governance and numbering.

## Milestone 0.3.0 decision collection

The [0.3.0 register](index-0.3.0.md) lists 121 decisions: 0241–0353 except 0260, plus 0358–0366. [Reference conventions](references-0.3.0.md) distinguish retained historical evidence from current developer instructions. Per-record source hashes and dated amendments preserve the sequence of decisions, including bounded deliveries, scope removal and rejected experiments.

Documentation publication does not extend the acceptance recorded by implementation, audit, proof or candidate receipts. The earlier milestone baseline below retains its original revision and limits.

## Accepted milestone baseline

The [Milestone 2 closeout](https://github.com/MediaNoxLabs/compact/issues/103#issuecomment-6017383781) records all 10 required remote workflows passing at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c), 243 closed issues and 266 audited conventional, GPG-signed/DCO commits since Milestone 1. Compiler 0.31.133; **private IR schema 20**; runtime ABI 49. IR versions are internal backend contracts, not a public Compact language schema.

Acceptance covers the published branch and matching Nix distribution. It does not claim a new release tag, crates.io publication, Windows certification or universal program parity. Source inventory availability (386/386 proof-required exports), sampled TS behavior (37 sources / 203 rows), selected original proofs (4 Coracle + 7 microDAO selectors, including 3 balancing-disabled smoke selectors) and historical live-wallet observations are distinct scopes. Consult the closeout for precise limits.

## Reading a record

- **Reviewed disposition** states the bounded decision and links its issue closure evidence; delivery status and design status remain separate.
- **Original source metadata** and **historical decision/amendments** preserve the original chronology, failed experiments, proposed code and narrower earlier ABI/schema/evidence scopes.
- **Referenced repository commits** link resolvable commits mentioned in the source; prerequisites and probes are not relabeled as final delivery.
- [Reference conventions](references.md) explain symbolic local evidence roots and unpublished research notes. A public issue summary is not a substitute for missing raw evidence bytes.
- [Historical publication manifest](publication-manifest.json) retains the original 239 imported source/published hashes and editorial transformations. The 0.3.0 batch keeps its own source and publication provenance.

Publication preserves decisions and receipts at their actual source revisions. It neither creates a new milestone nor turns historical tests into current candidate qualification.

## Maintaining the collection

Use the [template](template.md). Record problem, before/after code, alternatives, emitter/runtime ownership, upstream primitive mapping, compatibility and verifiable acceptance. Link a focused issue and an agreed milestone before implementation. Preserve dated amendments and explicit supersession when a decision changes. Identifiers through 0366 are assigned; consult the current registers before allocating another identifier.

Git is the canonical published ADR text from this import onward. The Obsidian collection retains original source hashes and exploratory/historical evidence, and links to the published register. Edit published decisions in Git and reconcile any vault mirror explicitly; do not silently overwrite either history.

## Topic index

These are starting points by subject. The original register below and the separate 0.3.0 register together locate this collection; the historical manifest retains its original topic tags.

### Architecture and generated Rust APIs

[0001](0001-record-circuit-execution-as-a-replayable-ledger-trace.md) · [0002](0002-generate-typed-ledger-field-descriptors.md) · [0003](0003-expose-a-contract-facade-and-one-dependency-consumer-crate.md) · [0004](0004-derive-only-mechanical-rust-representation-rules.md) · [0005](0005-use-a-typed-runtime-dsl-instead-of-a-body-wide-macro.md) · [0012](0012-partition-the-ledger-adapter-by-value-domain.md) · [0016](0016-reuse-recorded-callee-bodies-within-one-frame.md) · [0036](0036-unify-contract-recording-access-across-witness-shapes.md) · [0038](0038-declare-generated-witness-signatures-once.md) · [0039](0039-read-witness-ledger-state-through-typed-slots.md) · [0043](0043-carry-typed-circuit-identity-into-observed-state-rust-calls.md) · [0044](0044-encode-multi-parameter-observed-rust-calls-from-typed-ir.md) · [0069](0069-retain-generated-values-according-to-compact-type-copyability.md) · [0070](0070-scope-generated-high-arity-lint-to-compact-signatures.md) · [0071](0071-lower-pure-unit-circuits-as-statements.md) · [0072](0072-return-fallible-pure-tail-expressions-directly.md) · [0073](0073-simplify-generated-boolean-literal-branches.md) · [0074](0074-discard-typed-stateful-unit-expressions-without-bindings.md) · [0075](0075-evaluate-identical-conditional-arms-once.md) · [0076](0076-lift-typed-block-conditions-before-generated-if-expressions.md)

### Compiler IR, diagnostics and source admission

[0008](0008-preserve-compact-source-locations-through-the-rust-ir.md) · [0031](0031-preserve-legacy-compactc-rust-target-invocations.md) · [0051](0051-select-the-matching-versioned-runtime-at-code-generation.md) · [0052](0052-publish-rust-compiler-output-only-after-complete-generation.md) · [0053](0053-report-and-require-generated-proving-capabilities.md) · [0057](0057-preserve-public-compactc-help-and-flag-order-through-the-rust-launcher.md) · [0058](0058-reject-unsupported-tuple-and-vector-spreads-at-their-source.md) · [0067](0067-preserve-compact-names-in-generated-public-rust-methods.md) · [0077](0077-serialize-rust-output-publication-and-recover-interrupted-replacement.md) · [0079](0079-explain-missing-rust-recording-capabilities-from-typed-lowering.md) · [0082](0082-gate-rust-recording-by-compiler-proof-applicability.md) · [0089](0089-preserve-expression-valued-ownpublickey-effects.md) · [0196](0196-preserve-native-terminal-lexical-return-scope.md) · [0199](0199-bind-explicit-wallet-funding-to-recorded-shielded-offers.md)

### Ledger collections, witness reads and recording

[0010](0010-report-total-rust-circuit-gas-across-ledger-queries.md) · [0014](0014-represent-nested-map-slots-as-typed-shapes.md) · [0015](0015-meter-witness-ledger-reads-through-typed-projections.md) · [0017](0017-meter-map-witness-queries-through-a-typed-projection.md) · [0018](0018-meter-list-witness-reads-and-map-vm-maybe-to-option.md) · [0019](0019-propagate-fallible-witness-reads-through-generated-circuits.md) · [0020](0020-preserve-ledger-8-per-query-gas-limits.md) · [0021](0021-keep-merkle-witness-projections-local.md) · [0022](0022-meter-merkle-witness-vm-reads-while-keeping-local-projections.md) · [0023](0023-prove-typed-list-witness-heads-across-compact-value-shapes.md) · [0026](0026-give-merkle-fields-typed-generated-slots.md) · [0027](0027-record-plain-merkle-append-through-the-typed-slot.md) · [0028](0028-record-historic-merkle-append-with-history-semantics.md) · [0029](0029-type-set-and-map-vector-arguments-from-their-declarations.md) · [0045](0045-record-composite-set-keys-as-typed-observed-calls.md) · [0059](0059-expose-typed-read-only-public-state-views.md) · [0060](0060-expose-typed-read-only-set-views.md) · [0061](0061-expose-typed-read-only-cell-valued-map-views.md) · [0062](0062-expose-typed-local-list-inspection.md) · [0063](0063-separate-typed-local-merkle-inspection-from-metered-checks.md) · [0081](0081-record-composite-cell-values-through-typed-slots.md) · [0083](0083-record-merkle-root-verification-through-typed-slots.md) · [0161](0161-record-typed-guarded-set-mutations-for-asset-watch.md) · [0162](0162-typed-merkle-membership-recording-plans-and-election-commit.md)

### Original contracts and typed composition

[0113](0113-record-closed-unsigned-ternary-comparisons.md) · [0115](0115-record-typed-closed-ternary-struct-members.md) · [0120](0120-record-original-welcome-check-in-with-opaque-set-keys.md) · [0138](0138-preserve-circuit-locals-across-ordered-actions.md) · [0139](0139-record-asset-removal-through-a-scoped-opaque-key.md) · [0146](0146-record-guarded-opaque-key-struct-map-reads.md) · [0163](0163-record-audited-local-schnorr-helper-calls.md) · [0165](0165-typed-conditional-counter-recording-and-election-reveal.md) · [0166](0166-typed-cell-lifecycle-recording-and-retained-opaque-returns.md) · [0167](0167-typed-historic-commitment-spend-recording.md) · [0182](0182-record-typed-effectful-return-plans.md) · [0183](0183-typed-stateful-assertion-expressions.md) · [0185](0185-record-typed-stateful-assertion-expressions.md) · [0186](0186-circuit-local-witness-eligibility.md) · [0187](0187-typed-composite-return-recording.md) · [0188](0188-record-native-zswap-intents-against-authoritative-offers.md) · [0189](0189-relocatable-rust-compiler-distribution.md) · [0190](0190-context-derived-token-query-recording.md) · [0191](0191-record-unit-valued-zswap-composite-results.md) · [0192](0192-counter-dependent-membership-helper-recording.md) · [0193](0193-same-frame-unit-action-helpers-for-coracle-guess.md) · [0194](0194-record-original-microdao-advancement-and-reset-helper.md)

### Coins, shielded offers, wallet and proof boundaries

[0006](0006-adapt-complete-recorded-results-to-ledger-calls.md) · [0033](0033-carry-proven-calls-across-the-wallet-serialization-boundary.md) · [0035](0035-preserve-deployment-and-call-identity-across-wallet-handoff.md) · [0037](0037-align-wallet-handoff-with-ledger-8-0-3-construction-fixes.md) · [0040](0040-export-network-ready-rust-transactions-for-wallet-admission.md) · [0041](0041-build-rust-calls-from-confirmed-contract-state.md) · [0042](0042-bind-wallet-observations-to-submitted-transaction-and-finalized-block.md) · [0178](0178-record-field-cell-root-let-returns.md) · [0180](0180-fund-qualified-coin-proof-acceptance.md) · [0195](0195-record-shielded-receive-with-canonical-coin-identity.md) · [0197](0197-match-typescript-shielded-coin-descriptor-to-u128-values.md) · [0198](0198-restore-pinned-typescript-jubjub-scalar-sampling.md) · [0200](0200-normalize-signed-jubjub-scalar-reduction.md) · [0201](0201-record-typed-field-cell-observations-in-composite-results.md) · [0202](0202-record-terminal-lexical-return-continuations.md) · [0203](0203-record-qualified-sendshielded-and-branching-outputs.md) · [0204](0204-bind-explicit-contract-transient-coins-to-recorded-offers.md) · [0205](0205-opt-in-canonical-persistent-output-allocation.md) · [0206](0206-original-coracle-withdraw-and-typed-execution-coin-identity.md) · [0207](0207-record-qualified-and-immediate-shielded-coin-merges.md) · [0208](0208-direct-pure-literal-boundary-and-cryptographic-coercion-evidence.md) · [0209](0209-record-original-coracle-concede-as-an-ordered-actionful-payout.md) · [0210](0210-private-recording-storage-and-default-debug-worker.md) · [0211](0211-original-set-topic-with-exact-whole-fallible-funding.md) · [0212](0212-original-microdao-cash-out-recording.md) · [0213](0213-record-original-coracle-start-with-two-player-funding.md) · [0214](0214-original-microdao-vote-commit-recording.md) · [0215](0215-record-original-microdao-buy-in-with-checked-price-and-mint-composition.md) · [0217](0217-current-revision-live-wallet-shielded-acceptance.md)

### Parity, measurements and validation

[0009](0009-pin-oracle-acceptance-provenance-separately-from-behavior.md) · [0055](0055-make-wallet-javascript-pass-the-repository-license-gate.md) · [0056](0056-align-the-compiler-changelog-with-toolchain-0-31-133.md) · [0064](0064-keep-the-packaged-rust-runtime-clippy-clean.md) · [0065](0065-make-negative-rust-consumer-diagnostics-deterministic.md) · [0068](0068-keep-ast-backend-clean-under-rust-1-99-clippy.md) · [0216](0216-direct-behavior-coverage-for-pinned-oracle-exports.md) · [0218](0218-direct-pure-ternary-oracle-behavior.md) · [0219](0219-pin-rust-refusal-locations-across-source-contexts.md) · [0220](0220-direct-pure-call-and-registry-boundaries.md) · [0221](0221-exercise-sampled-oracle-branches-and-nonempty-collections.md) · [0222](0222-direct-recorded-trace-parity-for-pinned-oracle-apis.md) · [0223](0223-complete-license-headers-and-read-only-checks.md) · [0224](0224-keep-packaged-compiler-e2e-help-and-missing-zkir-checks-deterministic.md) · [0227](0227-isolate-remaining-no-install-cli-baselines.md) · [0234](0234-enforce-canonical-schnorr-signing-keys-independently-of-curve-provider.md)

### Packaging, installer and CI

[0007](0007-bundle-the-matching-runtime-until-a-versioned-release-exists.md) · [0013](0013-bind-runtime-archives-to-a-release-manifest.md) · [0032](0032-keep-generated-crate-guides-aligned-with-abi-15.md) · [0225](0225-pin-ci-proof-fixtures-and-rust-consumer-wiring.md) · [0226](0226-bind-installer-tests-to-cargo-and-isolate-read-only-baselines.md) · [0228](0228-real-pinned-compiler-archives-for-installer-acceptance.md) · [0229](0229-bind-existing-installer-scenarios-to-genuine-archives.md) · [0230](0230-prove-historical-self-update-with-pinned-local-releases.md) · [0231](0231-dispatch-existing-extracted-compiler-and-codeql-checks-on-candidate-branches.md) · [0232](0232-prove-both-ledger-boolean-branches-of-conditional-counter-circuits.md) · [0233](0233-coalesce-overlapping-formatter-inputs-before-concurrent-writes.md) · [0235](0235-prepare-locked-dependencies-for-clean-macos-rust-consumers.md) · [0236](0236-select-an-actual-intel-macos-runner-in-installer-ci.md) · [0237](0237-allow-the-extracted-compiler-cold-build-budget.md) · [0238](0238-native-build-tools-for-the-static-linux-cli.md) · [0239](0239-explicit-preparation-of-built-in-proof-material.md) · [0240](0240-bound-rust-validation-storage-on-linux-ci.md)

## Original register

| Number | Decision | Reviewed decision status |
|---|---|---|
| 0001 | [Record circuit execution as a replayable ledger trace](0001-record-circuit-execution-as-a-replayable-ledger-trace.md) | accepted-bounded |
| 0002 | [Generate typed ledger field descriptors](0002-generate-typed-ledger-field-descriptors.md) | accepted-bounded |
| 0003 | [Expose a contract facade and one-dependency consumer crate](0003-expose-a-contract-facade-and-one-dependency-consumer-crate.md) | accepted-bounded |
| 0004 | [Derive only mechanical Rust representation rules](0004-derive-only-mechanical-rust-representation-rules.md) | accepted-bounded |
| 0005 | [Use a typed runtime DSL instead of a body-wide macro](0005-use-a-typed-runtime-dsl-instead-of-a-body-wide-macro.md) | accepted-bounded |
| 0006 | [Adapt complete recorded results to ledger calls](0006-adapt-complete-recorded-results-to-ledger-calls.md) | accepted-bounded |
| 0007 | [Bundle the matching runtime until a versioned release exists](0007-bundle-the-matching-runtime-until-a-versioned-release-exists.md) | accepted-interim |
| 0008 | [Preserve Compact source locations through the Rust IR](0008-preserve-compact-source-locations-through-the-rust-ir.md) | accepted-bounded |
| 0009 | [Pin oracle acceptance provenance separately from behavior](0009-pin-oracle-acceptance-provenance-separately-from-behavior.md) | accepted-bounded |
| 0010 | [Report total Rust circuit gas across ledger queries](0010-report-total-rust-circuit-gas-across-ledger-queries.md) | accepted-bounded |
| 0011 | [Replay Compact assertions and conditional returns through typed frames](0011-replay-compact-assertions-and-conditional-returns-through-typed-frames.md) | accepted-bounded |
| 0012 | [Partition the ledger adapter by value domain](0012-partition-the-ledger-adapter-by-value-domain.md) | accepted-bounded |
| 0013 | [Bind runtime archives to a release manifest](0013-bind-runtime-archives-to-a-release-manifest.md) | accepted-bounded |
| 0014 | [Represent nested Map slots as typed shapes](0014-represent-nested-map-slots-as-typed-shapes.md) | accepted-bounded |
| 0015 | [Meter witness ledger reads through typed projections](0015-meter-witness-ledger-reads-through-typed-projections.md) | accepted-bounded |
| 0016 | [Reuse recorded callee bodies within one frame](0016-reuse-recorded-callee-bodies-within-one-frame.md) | accepted-bounded |
| 0017 | [Meter Map witness queries through a typed projection](0017-meter-map-witness-queries-through-a-typed-projection.md) | accepted-bounded |
| 0018 | [Meter List witness reads and map VM Maybe to Option](0018-meter-list-witness-reads-and-map-vm-maybe-to-option.md) | accepted-bounded |
| 0019 | [Propagate fallible witness reads through generated circuits](0019-propagate-fallible-witness-reads-through-generated-circuits.md) | accepted-bounded |
| 0020 | [Preserve ledger-8 per-query gas limits](0020-preserve-ledger-8-per-query-gas-limits.md) | accepted-bounded |
| 0021 | [Keep Merkle witness projections local](0021-keep-merkle-witness-projections-local.md) | superseded |
| 0022 | [Meter Merkle witness VM reads while keeping local projections](0022-meter-merkle-witness-vm-reads-while-keeping-local-projections.md) | accepted-bounded |
| 0023 | [Prove typed List witness heads across Compact value shapes](0023-prove-typed-list-witness-heads-across-compact-value-shapes.md) | accepted-bounded |
| 0024 | [Record typed List head results through canonical VM reads](0024-record-typed-list-head-results-through-canonical-vm-reads.md) | accepted-bounded |
| 0025 | [Accept distinct Compact Maybe instantiations for List heads](0025-accept-distinct-compact-maybe-instantiations-for-list-heads.md) | accepted-bounded |
| 0026 | [Give Merkle fields typed generated slots](0026-give-merkle-fields-typed-generated-slots.md) | accepted-bounded |
| 0027 | [Record plain Merkle append through the typed slot](0027-record-plain-merkle-append-through-the-typed-slot.md) | accepted-bounded |
| 0028 | [Record historic Merkle append with history semantics](0028-record-historic-merkle-append-with-history-semantics.md) | accepted-bounded |
| 0029 | [Type Set and Map vector arguments from their declarations](0029-type-set-and-map-vector-arguments-from-their-declarations.md) | accepted-bounded |
| 0030 | [Record Vector-key Set and Map circuits through typed slots](0030-record-vector-key-set-and-map-circuits-through-typed-slots.md) | accepted-bounded |
| 0031 | [Preserve legacy compactc Rust target invocations](0031-preserve-legacy-compactc-rust-target-invocations.md) | accepted-bounded |
| 0032 | [Keep generated crate guides aligned with ABI 15](0032-keep-generated-crate-guides-aligned-with-abi-15.md) | accepted-bounded |
| 0033 | [Carry proven calls across the wallet serialization boundary](0033-carry-proven-calls-across-the-wallet-serialization-boundary.md) | accepted-bounded |
| 0034 | [Share expression-nested recorded Field calls](0034-share-expression-nested-recorded-field-calls.md) | accepted-bounded |
| 0035 | [Preserve deployment and call identity across wallet handoff](0035-preserve-deployment-and-call-identity-across-wallet-handoff.md) | accepted-bounded |
| 0036 | [Unify contract recording access across witness shapes](0036-unify-contract-recording-access-across-witness-shapes.md) | accepted-bounded |
| 0037 | [Align wallet handoff with ledger 8.0.3 construction fixes](0037-align-wallet-handoff-with-ledger-8-0-3-construction-fixes.md) | accepted-bounded |
| 0038 | [Declare generated witness signatures once](0038-declare-generated-witness-signatures-once.md) | accepted-bounded |
| 0039 | [Read witness ledger state through typed slots](0039-read-witness-ledger-state-through-typed-slots.md) | accepted-bounded |
| 0040 | [Export network-ready Rust transactions for wallet admission](0040-export-network-ready-rust-transactions-for-wallet-admission.md) | accepted-bounded |
| 0041 | [Build Rust calls from confirmed contract state](0041-build-rust-calls-from-confirmed-contract-state.md) | accepted-bounded |
| 0042 | [Bind wallet observations to submitted transaction and finalized block](0042-bind-wallet-observations-to-submitted-transaction-and-finalized-block.md) | accepted-bounded |
| 0043 | [Carry typed circuit identity into observed-state Rust calls](0043-carry-typed-circuit-identity-into-observed-state-rust-calls.md) | accepted-bounded |
| 0044 | [Encode multi-parameter observed Rust calls from typed IR](0044-encode-multi-parameter-observed-rust-calls-from-typed-ir.md) | accepted-bounded |
| 0045 | [Record composite Set keys as typed observed calls](0045-record-composite-set-keys-as-typed-observed-calls.md) | accepted-bounded |
| 0046 | [Record Set operations at chunked ledger paths](0046-record-set-operations-at-chunked-ledger-paths.md) | accepted-bounded |
| 0047 | [Carry physical List paths through typed slots and VM programs](0047-carry-physical-list-paths-through-typed-slots-and-vm-programs.md) | accepted-bounded |
| 0048 | [Record scalar Map calls at chunked ledger paths](0048-record-scalar-map-calls-at-chunked-ledger-paths.md) | accepted-bounded |
| 0049 | [Record Cell calls at chunked ledger paths](0049-record-cell-calls-at-chunked-ledger-paths.md) | accepted-bounded |
| 0050 | [Record read-only scalar return expressions](0050-record-read-only-scalar-return-expressions.md) | accepted-bounded |
| 0051 | [Select the matching versioned runtime at code generation](0051-select-the-matching-versioned-runtime-at-code-generation.md) | accepted-bounded |
| 0052 | [Publish Rust compiler output only after complete generation](0052-publish-rust-compiler-output-only-after-complete-generation.md) | accepted-bounded |
| 0053 | [Report and require generated proving capabilities](0053-report-and-require-generated-proving-capabilities.md) | accepted-bounded |
| 0054 | [Record Field subtraction and multiplication in ordered frames](0054-record-field-subtraction-and-multiplication-in-ordered-frames.md) | accepted-bounded |
| 0055 | [Make wallet JavaScript pass the repository license gate](0055-make-wallet-javascript-pass-the-repository-license-gate.md) | accepted-bounded |
| 0056 | [Align the compiler changelog with toolchain 0.31.133](0056-align-the-compiler-changelog-with-toolchain-0-31-133.md) | accepted-bounded |
| 0057 | [Preserve public compactc help and flag order through the Rust launcher](0057-preserve-public-compactc-help-and-flag-order-through-the-rust-launcher.md) | accepted-bounded |
| 0058 | [Reject unsupported tuple and Vector spreads at their source](0058-reject-unsupported-tuple-and-vector-spreads-at-their-source.md) | accepted-bounded |
| 0059 | [Expose typed read-only public state views](0059-expose-typed-read-only-public-state-views.md) | accepted-bounded |
| 0060 | [Expose typed read-only Set views](0060-expose-typed-read-only-set-views.md) | accepted-bounded |
| 0061 | [Expose typed read-only cell-valued Map views](0061-expose-typed-read-only-cell-valued-map-views.md) | accepted-bounded |
| 0062 | [Expose typed local List inspection](0062-expose-typed-local-list-inspection.md) | accepted-bounded |
| 0063 | [Separate typed local Merkle inspection from metered checks](0063-separate-typed-local-merkle-inspection-from-metered-checks.md) | accepted-bounded |
| 0064 | [Keep the packaged Rust runtime Clippy clean](0064-keep-the-packaged-rust-runtime-clippy-clean.md) | accepted-bounded |
| 0065 | [Make negative Rust consumer diagnostics deterministic](0065-make-negative-rust-consumer-diagnostics-deterministic.md) | accepted-bounded |
| 0066 | [Record metered Merkle fullness checks through typed slots](0066-record-metered-merkle-fullness-checks-through-typed-slots.md) | accepted-bounded |
| 0067 | [Preserve Compact names in generated public Rust methods](0067-preserve-compact-names-in-generated-public-rust-methods.md) | accepted-bounded |
| 0068 | [Keep AST backend clean under Rust 1.99 Clippy](0068-keep-ast-backend-clean-under-rust-1-99-clippy.md) | accepted-bounded |
| 0069 | [Retain generated values according to Compact type copyability](0069-retain-generated-values-according-to-compact-type-copyability.md) | accepted-bounded |
| 0070 | [Scope generated high-arity lint to Compact signatures](0070-scope-generated-high-arity-lint-to-compact-signatures.md) | accepted-bounded |
| 0071 | [Lower pure Unit circuits as statements](0071-lower-pure-unit-circuits-as-statements.md) | accepted-bounded |
| 0072 | [Return fallible pure tail expressions directly](0072-return-fallible-pure-tail-expressions-directly.md) | accepted-bounded |
| 0073 | [Simplify generated boolean literal branches](0073-simplify-generated-boolean-literal-branches.md) | accepted-bounded |
| 0074 | [Discard typed stateful Unit expressions without bindings](0074-discard-typed-stateful-unit-expressions-without-bindings.md) | accepted-bounded |
| 0075 | [Evaluate identical conditional arms once](0075-evaluate-identical-conditional-arms-once.md) | accepted-bounded |
| 0076 | [Lift typed block conditions before generated if expressions](0076-lift-typed-block-conditions-before-generated-if-expressions.md) | accepted-bounded |
| 0077 | [Serialize Rust output publication and recover interrupted replacement](0077-serialize-rust-output-publication-and-recover-interrupted-replacement.md) | accepted-bounded |
| 0078 | [Record Counter reset through the typed ledger slot](0078-record-counter-reset-through-the-typed-ledger-slot.md) | accepted-bounded |
| 0079 | [Explain missing Rust recording capabilities from typed lowering](0079-explain-missing-rust-recording-capabilities-from-typed-lowering.md) | accepted-partial |
| 0080 | [Record unsigned Cell reads and writes through typed slots](0080-record-unsigned-cell-reads-and-writes-through-typed-slots.md) | accepted-bounded |
| 0081 | [Record composite Cell values through typed slots](0081-record-composite-cell-values-through-typed-slots.md) | accepted-bounded |
| 0082 | [Gate Rust recording by compiler proof applicability](0082-gate-rust-recording-by-compiler-proof-applicability.md) | accepted-bounded |
| 0083 | [Record Merkle root verification through typed slots](0083-record-merkle-root-verification-through-typed-slots.md) | accepted-bounded |
| 0084 | [Record Boolean ledger observations inside stateful bindings](0084-record-boolean-ledger-observations-inside-stateful-bindings.md) | accepted-bounded |
| 0085 | [Preserve native ownPublicKey witness effects](0085-preserve-native-ownpublickey-witness-effects.md) | accepted-bounded |
| 0086 | [Record conditional Counter amounts after Boolean observations](0086-record-conditional-counter-amounts-after-boolean-observations.md) | accepted-bounded |
| 0087 | [Record conditional Uint64 Cell writes after Counter increments](0087-record-conditional-uint64-cell-writes-after-counter-increments.md) | accepted-bounded |
| 0088 | [Record nested Set size assertions through typed slots](0088-record-nested-set-size-assertions-through-typed-slots.md) | accepted-bounded |
| 0089 | [Preserve expression-valued ownPublicKey effects](0089-preserve-expression-valued-ownpublickey-effects.md) | accepted-bounded |
| 0090 | [Record persistent Field commitments through typed Cell slots](0090-record-persistent-field-commitments-through-typed-cell-slots.md) | accepted-bounded |
| 0091 | [Record closed pure Field calls before Cell writes](0091-record-closed-pure-field-calls-before-cell-writes.md) | accepted-bounded |
| 0092 | [Record direct Boolean witness assertions before ledger writes](0092-record-direct-boolean-witness-assertions-before-ledger-writes.md) | accepted-bounded |
| 0093 | [Record pure conditional Field Cell writes](0093-record-pure-conditional-field-cell-writes.md) | accepted-bounded |
| 0094 | [Record typed tuple-to-vector witness arguments in Let bindings](0094-record-typed-tuple-to-vector-witness-arguments-in-let-bindings.md) | accepted-bounded |
| 0095 | [Record local enum keys in Set assertions](0095-record-local-enum-keys-in-set-assertions.md) | accepted-bounded |
| 0096 | [Record scalar pure Field calls with typed arguments](0096-record-scalar-pure-field-calls-with-typed-arguments.md) | accepted-bounded |
| 0097 | [Record Boolean literal branches in assertions](0097-record-boolean-literal-branches-in-assertions.md) | accepted-bounded |
| 0098 | [Record closed pure Vector keys in ADT Set calls](0098-record-closed-pure-vector-keys-in-adt-set-calls.md) | accepted-bounded |
| 0099 | [Record closed unsigned equalities in streaming assertions](0099-record-closed-unsigned-equalities-in-streaming-assertions.md) | accepted-bounded |
| 0100 | [Record scalar pure Field calls returned after state actions](0100-record-scalar-pure-field-calls-returned-after-state-actions.md) | accepted-bounded |
| 0101 | [Record nested List queries in ADT assertions](0101-record-nested-list-queries-in-adt-assertions.md) | accepted-bounded |
| 0102 | [Attribute imported pure circuits through source provenance](0102-attribute-imported-pure-circuits-through-source-provenance.md) | accepted-bounded |
| 0103 | [Record nested conditional Set actions](0103-record-nested-conditional-set-actions.md) | accepted-bounded |
| 0104 | [Attribute top-level Counter and Tiny proof APIs](0104-attribute-top-level-counter-and-tiny-proof-apis.md) | accepted-bounded |
| 0105 | [Record typed vector and tuple pure hash calls](0105-record-typed-vector-and-tuple-pure-hash-calls.md) | accepted-bounded |
| 0106 | [Record List enum head comparisons](0106-record-list-enum-head-comparisons.md) | accepted-bounded |
| 0107 | [Record standalone Unit witness effects in test-center Counter](0107-record-standalone-unit-witness-effects-in-test-center-counter.md) | accepted-bounded |
| 0108 | [Record typed pair hashes in stateful calls](0108-record-typed-pair-hashes-in-stateful-calls.md) | accepted-bounded |
| 0109 | [Record List Field vector head comparisons](0109-record-list-field-vector-head-comparisons.md) | accepted-bounded |
| 0110 | [Record typed vector hash Boolean Cell assertions](0110-record-typed-vector-hash-boolean-cell-assertions.md) | accepted-bounded |
| 0111 | [Record literal Bytes List assertions](0111-record-literal-bytes-list-assertions.md) | accepted-bounded |
| 0112 | [Iterate typed constructor vectors in Welcome](0112-iterate-typed-constructor-vectors-in-welcome.md) | accepted-bounded |
| 0113 | [Record closed unsigned ternary comparisons](0113-record-closed-unsigned-ternary-comparisons.md) | accepted-bounded |
| 0114 | [Typed indexed Merkle recording](0114-typed-indexed-merkle-recording.md) | accepted-bounded |
| 0115 | [Record typed closed ternary struct members](0115-record-typed-closed-ternary-struct-members.md) | accepted-bounded |
| 0117 | [Record closed pure assertion calls](0117-record-closed-pure-assertion-calls.md) | accepted-bounded |
| 0118 | [Record wide casts and closed unsigned calls](0118-record-wide-casts-and-closed-unsigned-calls.md) | accepted-bounded |
| 0119 | [Attribute original Election and Zerocash source proof gaps](0119-attribute-original-election-and-zerocash-source-proof-gaps.md) | accepted-bounded |
| 0120 | [Record original Welcome check-in with opaque Set keys](0120-record-original-welcome-check-in-with-opaque-set-keys.md) | accepted-bounded |
| 0121 | [Record closed nested Uint4 conditional projections](0121-record-closed-nested-uint4-conditional-projections.md) | accepted-bounded |
| 0122 | [Record discarded Field witnesses with typed arguments](0122-record-discarded-field-witnesses-with-typed-arguments.md) | accepted-bounded |
| 0123 | [Record Welcome organizer-gated calls](0123-record-welcome-organizer-gated-calls.md) | accepted-bounded |
| 0124 | [Record a typed Field helper with an ordered Cell read](0124-record-a-typed-field-helper-with-an-ordered-cell-read.md) | accepted-bounded |
| 0125 | [Record closed OpaqueString Set operations](0125-record-closed-opaquestring-set-operations.md) | accepted-bounded |
| 0126 | [Record closed conditional curve arguments](0126-record-closed-conditional-curve-arguments.md) | accepted-bounded |
| 0127 | [Record closed OpaqueString Map Field calls](0127-record-closed-opaquestring-map-field-calls.md) | accepted-bounded |
| 0128 | [Record an annotated closed Uint8 conditional](0128-record-an-annotated-closed-uint8-conditional.md) | accepted-bounded |
| 0129 | [Record asset freshness pure guard before writes](0129-record-asset-freshness-pure-guard-before-writes.md) | accepted-bounded |
| 0130 | [Record typed pair hashes before Cell writes](0130-record-typed-pair-hashes-before-cell-writes.md) | accepted-bounded |
| 0131 | [Record a closed conditional Field vector write](0131-record-a-closed-conditional-field-vector-write.md) | accepted-bounded |
| 0132 | [Record nested Uint4 widening to Uint64 Cell](0132-record-nested-uint4-widening-to-uint64-cell.md) | accepted-bounded |
| 0133 | [Record guarded arithmetic before a Counter increment](0133-record-guarded-arithmetic-before-a-counter-increment.md) | accepted-bounded |
| 0134 | [Record mixed-width unsigned guards before Counter increments](0134-record-mixed-width-unsigned-guards-before-counter-increments.md) | accepted-bounded |
| 0135 | [Record distinct one-field struct constructor calls](0135-record-distinct-one-field-struct-constructor-calls.md) | accepted-bounded |
| 0136 | [Record typed literal Merkle index bindings](0136-record-typed-literal-merkle-index-bindings.md) | accepted-bounded |
| 0137 | [Record plain Merkle hash append from a typed Bytes32 argument](0137-record-plain-merkle-hash-append-from-a-typed-bytes32-argument.md) | accepted-bounded |
| 0138 | [Preserve circuit locals across ordered actions](0138-preserve-circuit-locals-across-ordered-actions.md) | accepted-bounded |
| 0139 | [Record asset removal through a scoped opaque key](0139-record-asset-removal-through-a-scoped-opaque-key.md) | accepted-bounded |
| 0140 | [Record plain Merkle indexed hash placement](0140-record-plain-merkle-indexed-hash-placement.md) | accepted-bounded |
| 0141 | [Record guarded custody grant lookup](0141-record-guarded-custody-grant-lookup.md) | accepted-bounded |
| 0142 | [Record historic Merkle hash append with root history](0142-record-historic-merkle-hash-append-with-root-history.md) | accepted-bounded |
| 0143 | [Record direct plain Merkle root checks](0143-record-direct-plain-merkle-root-checks.md) | accepted-bounded |
| 0144 | [Record historic Merkle indexed hash placement](0144-record-historic-merkle-indexed-hash-placement.md) | accepted-bounded |
| 0145 | [Lower explicit Field to Bytes32 casts](0145-lower-explicit-field-to-bytes32-casts.md) | accepted-bounded |
| 0146 | [Record guarded opaque-key struct Map reads](0146-record-guarded-opaque-key-struct-map-reads.md) | accepted-bounded |
| 0147 | [Record plain Merkle reset to default](0147-record-plain-merkle-reset-to-default.md) | accepted-bounded |
| 0148 | [Record historic Merkle resetHistory with current-root retention](0148-record-historic-merkle-resethistory-with-current-root-retention.md) | accepted-bounded |
| 0149 | [Record closed identity vector maps](0149-record-closed-identity-vector-maps.md) | accepted-bounded |
| 0150 | [Record historic Merkle resetToDefault with blank-root history](0150-record-historic-merkle-resettodefault-with-blank-root-history.md) | accepted-bounded |
| 0151 | [Record typed opaque-key struct Map writes](0151-record-typed-opaque-key-struct-map-writes.md) | accepted-bounded |
| 0152 | [Record historic Merkle root-history membership](0152-record-historic-merkle-root-history-membership.md) | accepted-bounded |
| 0153 | [Record typed hash assertions through helper formals](0153-record-typed-hash-assertions-through-helper-formals.md) | accepted-bounded |
| 0154 | [Preserve unused Field ledger reads in unit returns](0154-preserve-unused-field-ledger-reads-in-unit-returns.md) | accepted-bounded |
| 0155 | [Record typed authority and phase guarded optional Cell writes](0155-record-typed-authority-and-phase-guarded-optional-cell-writes.md) | accepted-bounded |
| 0156 | [Preserve typed root locals in bboard returns](0156-preserve-typed-root-locals-in-bboard-returns.md) | accepted-bounded |
| 0157 | [Record typed asset Map writes with class and count guards](0157-record-typed-asset-map-writes-with-class-and-count-guards.md) | accepted-bounded |
| 0158 | [Record authority guarded optional topic and enum phase advancement](0158-record-authority-guarded-optional-topic-and-enum-phase-advancement.md) | accepted-bounded |
| 0159 | [Record composite witnesses and typed commitment values](0159-record-composite-witnesses-and-typed-commitment-values.md) | accepted-bounded |
| 0160 | [Record witness admitted authorized Merkle insertion](0160-record-witness-admitted-authorized-merkle-insertion.md) | accepted-bounded |
| 0161 | [Record typed guarded Set mutations for asset watch](0161-record-typed-guarded-set-mutations-for-asset-watch.md) | accepted-bounded |
| 0162 | [Typed Merkle membership recording plans and election commit](0162-typed-merkle-membership-recording-plans-and-election-commit.md) | accepted-bounded |
| 0163 | [Record audited local Schnorr helper calls](0163-record-audited-local-schnorr-helper-calls.md) | accepted-bounded |
| 0164 | [Qualify Set coin insertion from ledger transaction context](0164-qualify-set-coin-insertion-from-ledger-transaction-context.md) | accepted-native-only |
| 0165 | [Typed conditional counter recording and election reveal](0165-typed-conditional-counter-recording-and-election-reveal.md) | accepted-bounded |
| 0166 | [Typed Cell lifecycle recording and retained opaque returns](0166-typed-cell-lifecycle-recording-and-retained-opaque-returns.md) | accepted-bounded |
| 0167 | [Typed historic commitment spend recording](0167-typed-historic-commitment-spend-recording.md) | accepted-bounded |
| 0168 | [Checked literal Bytes-to-Field normalization](0168-checked-literal-bytes-to-field-normalization.md) | accepted-bounded |
| 0169 | [Preserve effectful ledger writes in value-returning Coracle helpers](0169-preserve-effectful-ledger-writes-in-value-returning-coracle-helpers.md) | accepted-native-only |
| 0170 | [Record qualified coin Set insertion](0170-record-qualified-coin-set-insertion.md) | accepted-bounded |
| 0171 | [Typed Counter less-than queries and nested reads](0171-typed-counter-less-than-queries-and-nested-reads.md) | accepted-bounded |
| 0172 | [Select meaningful fixture test targets without losing coverage](0172-select-meaningful-fixture-test-targets-without-losing-coverage.md) | accepted-test-only |
| 0173 | [Native qualified-coin Cell writes](0173-native-qualified-coin-cell-writes.md) | accepted-native-only |
| 0174 | [Typed effectful return plans for Coracle conditionals](0174-typed-effectful-return-plans-for-coracle-conditionals.md) | accepted-native-only |
| 0175 | [Typed circuit Zswap intents and provisional allocation](0175-typed-circuit-zswap-intents-and-provisional-allocation.md) | accepted-native-only |
| 0176 | [Record qualified coin Cell writes](0176-record-qualified-coin-cell-writes.md) | accepted-bounded |
| 0177 | [Typed native Kernel shielded effects](0177-typed-native-kernel-shielded-effects.md) | accepted-native-only |
| 0178 | [Record Field Cell root Let returns](0178-record-field-cell-root-let-returns.md) | accepted-bounded |
| 0179 | [Ordered typed stateful struct construction](0179-ordered-typed-stateful-struct-construction.md) | accepted-bounded |
| 0180 | [Fund qualified coin proof acceptance](0180-fund-qualified-coin-proof-acceptance.md) | accepted-bounded |
| 0181 | [Checked native wide unsigned addition](0181-checked-native-wide-unsigned-addition.md) | accepted-native-only |
| 0182 | [Record typed effectful return plans](0182-record-typed-effectful-return-plans.md) | accepted-bounded |
| 0183 | [Typed stateful assertion expressions](0183-typed-stateful-assertion-expressions.md) | accepted-native-only |
| 0184 | [Record bounded Kernel shielded effects](0184-record-bounded-kernel-shielded-effects.md) | accepted-bounded |
| 0185 | [Record typed stateful assertion expressions](0185-record-typed-stateful-assertion-expressions.md) | accepted-bounded |
| 0186 | [Circuit-local witness eligibility](0186-circuit-local-witness-eligibility.md) | accepted-bounded |
| 0187 | [Typed composite return recording](0187-typed-composite-return-recording.md) | accepted-bounded |
| 0188 | [Record native Zswap intents against authoritative offers](0188-record-native-zswap-intents-against-authoritative-offers.md) | accepted-runtime-policy |
| 0189 | [Relocatable Rust compiler distribution](0189-relocatable-rust-compiler-distribution.md) | accepted-distribution |
| 0190 | [Context-derived token query recording](0190-context-derived-token-query-recording.md) | accepted-bounded |
| 0191 | [Record Unit-valued Zswap composite results](0191-record-unit-valued-zswap-composite-results.md) | accepted-bounded |
| 0192 | [Counter-dependent membership helper recording](0192-counter-dependent-membership-helper-recording.md) | accepted-bounded |
| 0193 | [Same-frame Unit action helpers for Coracle guess](0193-same-frame-unit-action-helpers-for-coracle-guess.md) | accepted-bounded |
| 0194 | [Record original microDAO advancement and reset helper](0194-record-original-microdao-advancement-and-reset-helper.md) | accepted-bounded |
| 0195 | [Record shielded receive with canonical coin identity](0195-record-shielded-receive-with-canonical-coin-identity.md) | accepted-bounded |
| 0196 | [Preserve native terminal lexical return scope](0196-preserve-native-terminal-lexical-return-scope.md) | accepted-bounded |
| 0197 | [Match TypeScript shielded coin descriptor to u128 values](0197-match-typescript-shielded-coin-descriptor-to-u128-values.md) | accepted-runtime-fix |
| 0198 | [Restore pinned TypeScript JubJub scalar sampling](0198-restore-pinned-typescript-jubjub-scalar-sampling.md) | accepted-runtime-fix |
| 0199 | [Bind explicit wallet funding to recorded shielded offers](0199-bind-explicit-wallet-funding-to-recorded-shielded-offers.md) | accepted-runtime-policy |
| 0200 | [Normalize signed JubJub scalar reduction](0200-normalize-signed-jubjub-scalar-reduction.md) | accepted-runtime-fix |
| 0201 | [Record typed Field Cell observations in composite results](0201-record-typed-field-cell-observations-in-composite-results.md) | accepted-bounded |
| 0202 | [Record terminal lexical return continuations](0202-record-terminal-lexical-return-continuations.md) | accepted-bounded |
| 0203 | [Record qualified sendShielded and branching outputs](0203-record-qualified-sendshielded-and-branching-outputs.md) | accepted-bounded |
| 0204 | [Bind explicit contract transient coins to recorded offers](0204-bind-explicit-contract-transient-coins-to-recorded-offers.md) | accepted-runtime-policy |
| 0205 | [Opt-in canonical persistent output allocation](0205-opt-in-canonical-persistent-output-allocation.md) | accepted-runtime-policy |
| 0206 | [Original Coracle withdraw and typed execution coin identity](0206-original-coracle-withdraw-and-typed-execution-coin-identity.md) | accepted-bounded |
| 0207 | [Record qualified and immediate shielded coin merges](0207-record-qualified-and-immediate-shielded-coin-merges.md) | accepted-bounded |
| 0208 | [Direct pure literal boundary and cryptographic coercion evidence](0208-direct-pure-literal-boundary-and-cryptographic-coercion-evidence.md) | accepted-test-only |
| 0209 | [Record original Coracle concede as an ordered actionful payout](0209-record-original-coracle-concede-as-an-ordered-actionful-payout.md) | accepted-bounded |
| 0210 | [Private recording storage and default debug worker](0210-private-recording-storage-and-default-debug-worker.md) | accepted-runtime-fix |
| 0211 | [Original set_topic with exact whole-fallible funding](0211-original-set-topic-with-exact-whole-fallible-funding.md) | accepted-bounded |
| 0212 | [Original microDAO cash-out recording](0212-original-microdao-cash-out-recording.md) | accepted-bounded |
| 0213 | [Record original Coracle start with two-player funding](0213-record-original-coracle-start-with-two-player-funding.md) | accepted-bounded |
| 0214 | [Original microDAO vote-commit recording](0214-original-microdao-vote-commit-recording.md) | accepted-bounded |
| 0215 | [Record original microDAO buy_in with checked price and mint composition](0215-record-original-microdao-buy-in-with-checked-price-and-mint-composition.md) | accepted-bounded |
| 0216 | [Direct behavior coverage for pinned oracle exports](0216-direct-behavior-coverage-for-pinned-oracle-exports.md) | accepted-test-only |
| 0217 | [Current-revision live wallet shielded acceptance](0217-current-revision-live-wallet-shielded-acceptance.md) | accepted-bounded |
| 0218 | [Direct pure ternary oracle behavior](0218-direct-pure-ternary-oracle-behavior.md) | accepted-test-only |
| 0219 | [Pin Rust refusal locations across source contexts](0219-pin-rust-refusal-locations-across-source-contexts.md) | accepted-test-only |
| 0220 | [Direct pure call and registry boundaries](0220-direct-pure-call-and-registry-boundaries.md) | accepted-test-only |
| 0221 | [Exercise sampled oracle branches and nonempty collections](0221-exercise-sampled-oracle-branches-and-nonempty-collections.md) | accepted-test-only |
| 0222 | [Direct recorded trace parity for pinned oracle APIs](0222-direct-recorded-trace-parity-for-pinned-oracle-apis.md) | accepted-test-only |
| 0223 | [Complete license headers and read-only checks](0223-complete-license-headers-and-read-only-checks.md) | accepted-test-only |
| 0224 | [Keep packaged compiler E2E help and missing-ZKIR checks deterministic](0224-keep-packaged-compiler-e2e-help-and-missing-zkir-checks-deterministic.md) | accepted-test-only |
| 0225 | [Pin CI proof fixtures and Rust consumer wiring](0225-pin-ci-proof-fixtures-and-rust-consumer-wiring.md) | accepted-ci |
| 0226 | [Bind installer tests to Cargo and isolate read-only baselines](0226-bind-installer-tests-to-cargo-and-isolate-read-only-baselines.md) | accepted-test-only |
| 0227 | [Isolate remaining no-install CLI baselines](0227-isolate-remaining-no-install-cli-baselines.md) | accepted-test-only |
| 0228 | [Real pinned compiler archives for installer acceptance](0228-real-pinned-compiler-archives-for-installer-acceptance.md) | accepted-test-only |
| 0229 | [Bind existing installer scenarios to genuine archives](0229-bind-existing-installer-scenarios-to-genuine-archives.md) | accepted-test-only |
| 0230 | [Prove historical self-update with pinned local releases](0230-prove-historical-self-update-with-pinned-local-releases.md) | accepted-test-only |
| 0231 | [Dispatch existing extracted compiler and CodeQL checks on candidate branches](0231-dispatch-existing-extracted-compiler-and-codeql-checks-on-candidate-branches.md) | accepted-ci |
| 0232 | [Prove both ledger Boolean branches of conditional Counter circuits](0232-prove-both-ledger-boolean-branches-of-conditional-counter-circuits.md) | accepted-test-only |
| 0233 | [Coalesce overlapping formatter inputs before concurrent writes](0233-coalesce-overlapping-formatter-inputs-before-concurrent-writes.md) | accepted-bounded |
| 0234 | [Enforce canonical Schnorr signing keys independently of curve provider](0234-enforce-canonical-schnorr-signing-keys-independently-of-curve-provider.md) | accepted-runtime-fix |
| 0235 | [Prepare locked dependencies for clean macOS Rust consumers](0235-prepare-locked-dependencies-for-clean-macos-rust-consumers.md) | accepted-ci |
| 0236 | [Select an actual Intel macOS runner in installer CI](0236-select-an-actual-intel-macos-runner-in-installer-ci.md) | accepted-ci |
| 0237 | [Allow the extracted compiler cold-build budget](0237-allow-the-extracted-compiler-cold-build-budget.md) | accepted-ci |
| 0238 | [Native build tools for the static Linux CLI](0238-native-build-tools-for-the-static-linux-cli.md) | accepted-ci |
| 0239 | [Explicit preparation of built-in proof material](0239-explicit-preparation-of-built-in-proof-material.md) | accepted-ci |
| 0240 | [Bound Rust validation storage on Linux CI](0240-bound-rust-validation-storage-on-linux-ci.md) | accepted-ci |

- [0366 — Create fresh relation proof key directories](0366-create-fresh-relation-proof-key-directories.md): fresh key ownership and retained-prefix qualification.
