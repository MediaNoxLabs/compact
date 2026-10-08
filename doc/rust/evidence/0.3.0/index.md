# Documentation evidence — 0.3.0

These historical receipts support the developer guides. Their original bytes, commands and measured source identities are preserved; collecting them here does not rerun the checks or qualify a later candidate.

[Machine-readable relocation index](relocation-index.json) maps original paths and archive members to portable destinations and SHA-256 identities. Paths inside original receipts remain provenance strings. SDK trees, binaries, proof artifacts and large metadata dumps are deliberately excluded.

<a id="tutorials-adr0324"></a>
## W2 / D2 external tutorials

At source 5efa91c2: one witnessed Cell test and two DID lifecycle tests, default native features, Rust 1.99/macOS ARM64. The DID scenarios use retained independent TS capture. No proof or network claim.

[Primary receipt](tutorials-adr0324/validation-receipt.json)

| Artifact | SHA-256 |
|---|---|
| [VALIDATION.md](tutorials-adr0324/VALIDATION.md) | `4734917463a0318b46c89d911f19798c5bcafe3950b3779b639938900a9cc179` |
| [validation-receipt.json](tutorials-adr0324/validation-receipt.json) | `e0f960c5d9fdc7ba74f0195a482ad294632214a0cec4e5c5cb1a62e007a5ff7e` |
| [artifact-manifest.json](tutorials-adr0324/artifact-manifest.json) | `7ec6633342c116019196e852380bd9a6bf83b65061cf0afb4cbccd44e43c2451` |
| [commands.json](tutorials-adr0324/commands.json) | `ce114ec90a0a1b750958aebed7575a4694f7731045f2860127d6f2d94a4ed21e` |
| [initial-commands.json](tutorials-adr0324/initial-commands.json) | `afb5601e9db0627f7e918f892121df677384dfec3a675d5bb67fbfa02385046c` |
| [feature-source-policy.json](tutorials-adr0324/feature-source-policy.json) | `c9491c30fab726cd03fe22389e6a2c12ea678230e079123481392f861d2646f9` |
| [generated-before-format.json](tutorials-adr0324/generated-before-format.json) | `865527f60784091b7972f8330dcfbefd7cd5daed0651f30149f9d8444f5b08f9` |
| [reference-manifest.json](tutorials-adr0324/reference-manifest.json) | `6ff4e7f076fca7166be2548e1a2fb6cd4fe5cc370e828b4f5290c015995b05e2` |
| [initial-lock-comparison.json](tutorials-adr0324/initial-lock-comparison.json) | `f1c3cd7a7b5f2c19639068c620fd11d3514e18695b7775b36744e2c06013eb23` |
| [did-lifecycle-lock-comparison.json](tutorials-adr0324/did-lifecycle-lock-comparison.json) | `ef9e446b7ce7dc92515af737f505d213bd6d6fefcf35fd11c7c5383c19acde71` |
| [witnessed-cell-lock-comparison.json](tutorials-adr0324/witnessed-cell-lock-comparison.json) | `ef9e446b7ce7dc92515af737f505d213bd6d6fefcf35fd11c7c5383c19acde71` |
| [did-lifecycle-clippy.log](tutorials-adr0324/logs/did-lifecycle-clippy.log) | `dfcfb66860b54554811484029045c30f64b06f3146ffb690c94f4f5bd2fe6b1d` |
| [did-lifecycle-format.log](tutorials-adr0324/logs/did-lifecycle-format.log) | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| [did-lifecycle-generate.log](tutorials-adr0324/logs/did-lifecycle-generate.log) | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| [did-lifecycle-test-final.log](tutorials-adr0324/logs/did-lifecycle-test-final.log) | `a9ff2980ec61952c322950fccd44adc96bc379466efbea5efff6143f449b980e` |
| [did-lifecycle-test.log](tutorials-adr0324/logs/did-lifecycle-test.log) | `31545ace79ac234cab2ed561a8ff0e72051942d5cdc3c83c9836bb57339a00af` |
| [witnessed-cell-clippy.log](tutorials-adr0324/logs/witnessed-cell-clippy.log) | `1292ddaad188ab7245192b8ed4251b5d5ea91c8a98c99ad01f7f589f66198c59` |
| [witnessed-cell-format.log](tutorials-adr0324/logs/witnessed-cell-format.log) | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| [witnessed-cell-generate.log](tutorials-adr0324/logs/witnessed-cell-generate.log) | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| [witnessed-cell-test-final.log](tutorials-adr0324/logs/witnessed-cell-test-final.log) | `bc2982a5ddd5709dd39e7a58f648017b7f1dfa300a74eed50a8d640b4087fd1f` |
| [witnessed-cell-test.log](tutorials-adr0324/logs/witnessed-cell-test.log) | `bb263e17d904c7474b1bb121058e9f463cec69e0dda8c8d53c15ad8ff67c6c0f` |

<a id="rustdoc-adr0326"></a>
## RD1 WitnessScript rustdoc

Historical rustdoc execution and scoped compiler input tests. The receipt retains its candidate identity; signed integration was 9012a86a.

[Primary receipt](rustdoc-adr0326/receipt.json)

| Artifact | SHA-256 |
|---|---|
| [receipt.json](rustdoc-adr0326/receipt.json) | `e7f0ae341b8dcaa4dafa3508120cd480d6884d0c01cfe69b3b9d3344f621913c` |
| [input-tests-final.log](rustdoc-adr0326/input-tests-final.log) | `9e1c75b58e35d7de19637be2b33a69f532385506445ef63f1a440dafc7bbeed4` |
| [testkit-doc-tests.log](rustdoc-adr0326/testkit-doc-tests.log) | `06fde780b0b682d87bbbef1993ace0775b9d1beac5b396e6be5464b2845b82c7` |
| [backend-test-clippy.log](rustdoc-adr0326/backend-test-clippy.log) | `826fc39b7aab0b92c0bbb872b7e6e9645b4c0f0cd968380597db92e70e6d452e` |
| [testkit-clippy.log](rustdoc-adr0326/testkit-clippy.log) | `1cf5d8dd08178181580cfc9694b328ad83cac4d9b811939da8956640b2812f02` |
| [fmt-final.log](rustdoc-adr0326/fmt-final.log) | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

<a id="passport-p1"></a>
## P1 external passport consumer

Two external pure API tests with unedited generation and qualified dependency identities. This is not the complete 202-case adoption suite.

[Primary receipt](passport-p1/receipt.json)

| Artifact | SHA-256 |
|---|---|
| [receipt.json](passport-p1/receipt.json) | `f6148d0a62019db3a5fe663c9968d8eeebf579e4c0bbeb25d00c11a234df5dda` |
| [cargo-local-packages.json](passport-p1/cargo-local-packages.json) | `7a8a64f4a90226c11e16a8a7911cfb83dd8bc47fc75ab3eec8756497a31d8523` |
| [compile.log](passport-p1/compile.log) | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| [external-generation.log](passport-p1/external-generation.log) | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| [consumer-final.log](passport-p1/consumer-final.log) | `03e1ea661532214a330d5546d040ea711c4704195d92138bb7f6fbbb0de21884` |
| [consumer-first.log](passport-p1/consumer-first.log) | `1f64285682eb1acb04c41deeebe79072be9ef96923ea4802ab999c3c524b47d5` |
| [consumer-locked.log](passport-p1/consumer-locked.log) | `30975e8f78b66360e968ac9dd90603de9df56437fc47828f01d7da13f6e0b186` |

<a id="named-record"></a>
## Named inputs: 25 age-predicate cases

Application-owned wrapper against 25 retained independent age-predicate capture cases at fba6845f. Only generated manifest runtime paths were relocated; generated Rust stayed unchanged.

[Primary receipt](named-record/evidence/receipt.json)

| Artifact | SHA-256 |
|---|---|
| [README.md](named-record/README.md) | `87ab0b6311d05e2d554aeab42c7155b98a076698e3a5ce354e9045613260eee0` |
| [artifact-manifest.json](named-record/artifact-manifest.json) | `d763f79b4f4dea0b165ef2dabff2c21462a6d60dfd57c61ac81c807eb4b5abed` |
| [receipt.json](named-record/evidence/receipt.json) | `fd50f65079ec8238401e456beedcfbfe10929d17f591649a707b11d1796bc04f` |
| [source.json](named-record/evidence/source.json) | `eb111c65534b03b893dc4ec4044e0702623c86e73ff7e42ec3870c1c802f37b8` |
| [lock-qualification.json](named-record/evidence/lock-qualification.json) | `37e60b3767099c32eeffb501d8d4c25f23fdc3d28e4f6aba052daf4d02f8a6db` |
| [test.log](named-record/evidence/test.log) | `55580e38fc910090980d334d5749947e2cef5d5f1186b2eb12636deb1c629feb` |
| [clippy.log](named-record/evidence/clippy.log) | `a320ca03c820127fb7b33c2b9469be714971fe2ed4870e03d6ea7c8d7bae9577` |
| [format-check.log](named-record/evidence/format-check.log) | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

<a id="counter-c1"></a>
## C1 Counter ContractLab consumer

Historical standalone application with shared absolute checkout dependencies. Native/recorded state, effects, replay, underflow and restore assertions. This receipt does not establish an isolated SDK packaging run.

[Primary receipt](counter-c1/retained-receipt.md)

| Artifact | SHA-256 |
|---|---|
| [retained-receipt.md](counter-c1/retained-receipt.md) | `d9dc376acedd7166ad57fba33c590ebfbf3d36041c1965db3134d5c1aa29e237` |
| [run.log](counter-c1/run.log) | `963b25c8d3d18a1110f3f0c20fe4fe0381c27f827e5db23d9e848b55b07beb84` |

<a id="capability-k1"></a>
## K1 capability report checker

Historical report checker only. Its unavailable relation result belongs to that older report. The separate ADR0324 report advertises all 12 DID APIs; availability is not execution or proof evidence.

[Primary receipt](capability-k1/receipt.json)

| Artifact | SHA-256 |
|---|---|
| [receipt.json](capability-k1/receipt.json) | `db30251990c540b471bd27ddb0333488d5096ea70589a3583c27085b07ee93e6` |
| [check_observed_api.py](capability-k1/check_observed_api.py) | `d74106475ee986f3773768c04b15476f0143d2a25150e4c0d405c7e132857179` |
| [historical-rust-capabilities.json](capability-k1/historical-rust-capabilities.json) | `8d3dc57a405875ecce5974e945719b6ae7bbb00b3543de1a40ded0756c15d5d2` |
| [adr0324-rust-capabilities.json](capability-k1/adr0324-rust-capabilities.json) | `5892af4a3ddd03f41cb7728d26fef8f17971cbd2d57fa0afea907da104d04228` |
| [retained-receipt.md](capability-k1/retained-receipt.md) | `d51c8badaf7345dedbcdded1af4eb9a3dfeb59c7be6c5dd23e8a050ace94e3e0` |

<a id="migration-cf1"></a>
## M1 / CF1 migration and static constraints

Historical ABI49 refusal, ABI50 controls and exhaustive-match migration checks. Maintained controls without a joined execution receipt remain explicitly limited in guide-evidence-map.json.

[Primary receipt](migration-cf1/receipt.json)

| Artifact | SHA-256 |
|---|---|
| [receipt.json](migration-cf1/receipt.json) | `b96d9a0368311fe14f2331ad866c89be850a2eb785ab0ba418007cc991104c40` |
| [migration-results.json](migration-cf1/migration-results.json) | `fd31990ee0c9d152d082cf0dde0f1abe6928f03284965a18890424b4e0b48614` |
| [generated-consumer-results.json](migration-cf1/generated-consumer-results.json) | `0411f3f3e94c1d7970911739f890a5add14b01a59c9ff2db89e32ea166317634` |
| [historical-abi49.log](migration-cf1/historical-abi49.log) | `295b9f0c8e61eaa5dd15460503297cde682febe38ce2dc18bf009ac80186556f` |
| [new-exhaustive.log](migration-cf1/new-exhaustive.log) | `bbec98e14f8d5546ba2ae8015f7c7513f06601fd30e25c0ffc6fd43b857f8779` |
| [new-complete-no-fallback.log](migration-cf1/new-complete-no-fallback.log) | `fbb9fbbff3dcc2e8cab4d425dab38d782fa7b042cf47d21baea0f674e6799f8f` |
| [migrated-primary-final.log](migration-cf1/migrated-primary-final.log) | `94414be571255aed1839e2b1e4155393b64113a9a85865b3e8d69db3d9de1dbb` |
| [migrated-msrv-final.log](migration-cf1/migrated-msrv-final.log) | `bc66c291b5e4f1d010f1c0f4411943e0582e666395682bf4a1209692ce6cdc77` |
| [compatibility.log](migration-cf1/compatibility.log) | `2895607f0325127dd57d6da5c6da0c6597525b1882a58e27ed2372d9bb288380` |
| [MIGRATION.md](migration-cf1/MIGRATION.md) | `b658a175e3b97a1e570895f6a36dcdacc15f22be53cc1e0f1ac7ff958b37b988` |
| [guide-evidence-map.json](migration-cf1/guide-evidence-map.json) | `620e6096cdd58b6128f371f0f75574ba9ebc4f2bd196162860531dcace6fd045` |
| [v2-archive-consumer-closeout.md](migration-cf1/v2-archive-consumer-closeout.md) | `b91ee198921d73053860ca9ea26454cd4f39c34acafa0d6dd6431e962bac3e62` |

<a id="recording-handle-adr0331"></a>
## Conditional recording handle conversion

Focused default and ledger-transaction external typechecking of conditional From<&Contract<W>> fallback after a recording accessor collision. Not a claim that every contract emits that fallback.

[Primary receipt](recording-handle-adr0331/receipt.json)

| Artifact | SHA-256 |
|---|---|
| [receipt.json](recording-handle-adr0331/receipt.json) | `305566c7986a8f9093b821ef75b7970fa5ede3a88fcc58206f55392afa525e13` |
| [DELIVERY.md](recording-handle-adr0331/DELIVERY.md) | `1cd338a4ecede0ef0aba2cd05945ac3cc211c870d1a8061dd0b534f54038715b` |
| [manifest.json](recording-handle-adr0331/manifest.json) | `ed756cd5a274ce2aea49ecf1ae2487327ef5f10eb75bae4574cd5e631ac53ba6` |
| [default.log](recording-handle-adr0331/default.log) | `c8e9b10672c9ba50896591cc545e0812eebf624eb0c06376c11e574962fd66a0` |
| [default.stderr.log](recording-handle-adr0331/default.stderr.log) | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| [transaction.log](recording-handle-adr0331/transaction.log) | `082c00e554b8cbfb0b789953228f9a14336624871249912c4633323f6d410c7c` |
| [transaction.stderr.log](recording-handle-adr0331/transaction.stderr.log) | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| [default-clippy.log](recording-handle-adr0331/default-clippy.log) | `8760f61dfb052bdc5f6393b9f9e35e8b23ae0ccef58a12e19111e3e0a8bfc8ba` |
| [transaction-clippy.log](recording-handle-adr0331/transaction-clippy.log) | `d6dea380c82a92fc64b3164a52105940eb705903ecde8f860f91449793264efa` |
| [generated-runtime-source-comparison.json](recording-handle-adr0331/generated-runtime-source-comparison.json) | `0e3679a4c6027e9d59c5defa7999c4f779374dbf2ae1457741d03e3ac8c2eda0` |
| [sdk-source-comparison.json](recording-handle-adr0331/sdk-source-comparison.json) | `f55b1778c189b2cdbe406c20c7732dcd5db115691a1e9b59d5943aa08dd03da9` |

## Limits and missing references

No requested selected artifact was missing.
Historical receipts can refer to files intentionally omitted from this bounded package. The original full vault archives retain those files. Dedicated external byte-length/witness-signature negative receipts remain unestablished in the companion mapping. Final source, feature, host and release qualification remain separate.

## Subsequent status and qualification

ADR0285/#409 scoped controls were resumed and accepted at `4c8aebc3`; older STOP text inside the preserved receipts or policy JSON remains historical provenance. Later audit fixes and the [encoded-input admission APIs](../../guides/proof-integration.md#admit-encoded-state-and-verifier-inputs) have their own source and test evidence. No historical tutorial receipt is relabeled as execution of those fixes or the final release candidate. The owner approved encoded-byte admission for0.3.0 with a configurable opt-in64MiB preset. Aggregate decoded heap/object/CPU containment is deferred under CoPS-001; no total-containment claim is made.

## Local proof fixture ADR0360

Source checkpoint `17433b40a14da83c5c500d92420149c28f755b88`. The proof-enabled package run passed 25 unit/integration tests and its then-existing doc test. After adding the usage example, a separate doc run passed two tests. Default-feature control passed 21 unit/integration tests and one doc test. Four new tests cover actual official-provider checks/refusals and verifier lookup/decoding; no successful proof, verification, ledger application or network receipt. The usage fence in [the guide](../../guides/local-proof-tests.md) is not a standalone runnable consumer.

- [REPORT.md](local-proof-fixture-adr0360/REPORT.md)
- [tests.log](local-proof-fixture-adr0360/tests.log)
- [default-tests.log](local-proof-fixture-adr0360/default-tests.log)
- [doctests.log](local-proof-fixture-adr0360/doctests.log)
- [clippy.log](local-proof-fixture-adr0360/clippy.log)
- [format.log](local-proof-fixture-adr0360/format.log)
- [review.md](local-proof-fixture-adr0360/review.md)
- [source-hashes.json](local-proof-fixture-adr0360/source-hashes.json)

## Candidate qualification follow-ups

These dated receipts retain their original source identities and bounded scopes. See [candidate qualification](../../guides/candidate-qualification.md) for the combined validation and documentation status.

### Candidate CI ADR0361

- [remote-receipts/rust-bounded-core-2c798d2dc5f94ad13e89610068035ed70d94f469/clippy.log](candidate-ci-adr0361/remote-receipts/rust-bounded-core-2c798d2dc5f94ad13e89610068035ed70d94f469/clippy.log)
- [remote-receipts/rust-bounded-core-2c798d2dc5f94ad13e89610068035ed70d94f469/format.log](candidate-ci-adr0361/remote-receipts/rust-bounded-core-2c798d2dc5f94ad13e89610068035ed70d94f469/format.log)
- [remote-receipts/rust-bounded-core-2c798d2dc5f94ad13e89610068035ed70d94f469/result.json](candidate-ci-adr0361/remote-receipts/rust-bounded-core-2c798d2dc5f94ad13e89610068035ed70d94f469/result.json)
- [remote-receipts/rust-bounded-core-2c798d2dc5f94ad13e89610068035ed70d94f469/source.json](candidate-ci-adr0361/remote-receipts/rust-bounded-core-2c798d2dc5f94ad13e89610068035ed70d94f469/source.json)
- [remote-receipts/rust-bounded-core-2c798d2dc5f94ad13e89610068035ed70d94f469/tests.log](candidate-ci-adr0361/remote-receipts/rust-bounded-core-2c798d2dc5f94ad13e89610068035ed70d94f469/tests.log)
- [remote-receipts/rust-bounded-core-2c798d2dc5f94ad13e89610068035ed70d94f469/workflow.yml](candidate-ci-adr0361/remote-receipts/rust-bounded-core-2c798d2dc5f94ad13e89610068035ed70d94f469/workflow.yml)
- [remote-receipts/rust-bounded-msrv-2c798d2dc5f94ad13e89610068035ed70d94f469/msrv-standalone.log](candidate-ci-adr0361/remote-receipts/rust-bounded-msrv-2c798d2dc5f94ad13e89610068035ed70d94f469/msrv-standalone.log)
- [remote-receipts/rust-bounded-msrv-2c798d2dc5f94ad13e89610068035ed70d94f469/msrv-workspace.log](candidate-ci-adr0361/remote-receipts/rust-bounded-msrv-2c798d2dc5f94ad13e89610068035ed70d94f469/msrv-workspace.log)
- [remote-receipts/rust-bounded-msrv-2c798d2dc5f94ad13e89610068035ed70d94f469/result.json](candidate-ci-adr0361/remote-receipts/rust-bounded-msrv-2c798d2dc5f94ad13e89610068035ed70d94f469/result.json)
- [remote-receipts/rust-bounded-msrv-2c798d2dc5f94ad13e89610068035ed70d94f469/source.json](candidate-ci-adr0361/remote-receipts/rust-bounded-msrv-2c798d2dc5f94ad13e89610068035ed70d94f469/source.json)
- [remote-receipts/rust-bounded-msrv-2c798d2dc5f94ad13e89610068035ed70d94f469/workflow.yml](candidate-ci-adr0361/remote-receipts/rust-bounded-msrv-2c798d2dc5f94ad13e89610068035ed70d94f469/workflow.yml)
- [review-note.md](candidate-ci-adr0361/review-note.md)

### Candidate consumers ADR0362

- [audit-receipt.json](candidate-consumers-adr0362/audit-receipt.json)
- [audit-standalone.json](candidate-consumers-adr0362/audit-standalone.json)
- [audit-workspace.json](candidate-consumers-adr0362/audit-workspace.json)
- [dependency-source-continuity.json](candidate-consumers-adr0362/dependency-source-continuity.json)
- [current-w2/commands.json](candidate-consumers-adr0362/current-w2/commands.json)
- [current-w2/lock-comparison.json](candidate-consumers-adr0362/current-w2/lock-comparison.json)
- [current-w2/receipt.json](candidate-consumers-adr0362/current-w2/receipt.json)
- [current-w2/logs/witnessed-1.88.0.log](candidate-consumers-adr0362/current-w2/logs/witnessed-1.88.0.log)
- [current-w2/logs/witnessed-1.99.0.log](candidate-consumers-adr0362/current-w2/logs/witnessed-1.99.0.log)
- [current-migrations/commands.json](candidate-consumers-adr0362/current-migrations/commands.json)
- [current-migrations/receipt.json](candidate-consumers-adr0362/current-migrations/receipt.json)
- [current-migrations/migrated.log](candidate-consumers-adr0362/current-migrations/migrated.log)
- [current-migrations/new-complete-no-fallback.log](candidate-consumers-adr0362/current-migrations/new-complete-no-fallback.log)
- [current-migrations/migrated-lock-qualification.json](candidate-consumers-adr0362/current-migrations/migrated-lock-qualification.json)
- [current-migrations/new-complete-no-fallback-lock-qualification.json](candidate-consumers-adr0362/current-migrations/new-complete-no-fallback-lock-qualification.json)
- [coverage-delta/coverage-reconciliation.json](candidate-consumers-adr0362/coverage-delta/coverage-reconciliation.json)
- [coverage-delta/run-receipt.json](candidate-consumers-adr0362/coverage-delta/run-receipt.json)
- [coverage-delta/runtime.log](candidate-consumers-adr0362/coverage-delta/runtime.log)
- [coverage-delta/testkit.log](candidate-consumers-adr0362/coverage-delta/testkit.log)
- [coverage-delta/exclusions.json](candidate-consumers-adr0362/coverage-delta/exclusions.json)
- [current-portable-fixed/commands.json](candidate-consumers-adr0362/current-portable-fixed/commands.json)
- [current-portable-fixed/receipt.json](candidate-consumers-adr0362/current-portable-fixed/receipt.json)
- [current-portable-fixed/10-cargo.log](candidate-consumers-adr0362/current-portable-fixed/10-cargo.log)
- [current-portable-fixed/11-cargo.log](candidate-consumers-adr0362/current-portable-fixed/11-cargo.log)
- [publication-rejection-before.log](candidate-consumers-adr0362/publication-rejection-before.log)
- [publication-rejection-fixed.log](candidate-consumers-adr0362/publication-rejection-fixed.log)
- [publication-rejection-fix.json](candidate-consumers-adr0362/publication-rejection-fix.json)
- [publication-rejection-fix.md](candidate-consumers-adr0362/publication-rejection-fix.md)
- [scheme-source-binding.json](candidate-consumers-adr0362/scheme-source-binding.json)
- [scheme-derivation.json](candidate-consumers-adr0362/scheme-derivation.json)
- [scheme-output-closure.json](candidate-consumers-adr0362/scheme-output-closure.json)
- [consumer-runner-provenance.json](candidate-consumers-adr0362/consumer-runner-provenance.json)
- [review-note.md](candidate-consumers-adr0362/review-note.md)

### Candidate baselines ADR0363–0364

- [ADR0364/REPORT.md](candidate-baselines-adr0363-0364/ADR0364/REPORT.md)
- [ADR0364/all-harness-tests.log](candidate-baselines-adr0363-0364/ADR0364/all-harness-tests.log)
- [ADR0364/boolean-native-receipt.json](candidate-baselines-adr0363-0364/ADR0364/boolean-native-receipt.json)
- [ADR0364/boolean-native.log](candidate-baselines-adr0363-0364/ADR0364/boolean-native.log)
- [ADR0364/final-receipt.json](candidate-baselines-adr0363-0364/ADR0364/final-receipt.json)
- [ADR0364/frozen-baseline-check.json](candidate-baselines-adr0363-0364/ADR0364/frozen-baseline-check.json)
- [ADR0364/projection-check.json](candidate-baselines-adr0363-0364/ADR0364/projection-check.json)
- [ADR0364/specification-tests.log](candidate-baselines-adr0363-0364/ADR0364/specification-tests.log)
- [candidate57fab/candidate-continuity.json](candidate-baselines-adr0363-0364/candidate57fab/candidate-continuity.json)
- [candidate679/did-source-scope-fix.json](candidate-baselines-adr0363-0364/candidate679/did-source-scope-fix.json)
- [candidate679/did-source-scope-fix.md](candidate-baselines-adr0363-0364/candidate679/did-source-scope-fix.md)
- [candidate679/source-scope-preflight/unit-composition-source-scope.json](candidate-baselines-adr0363-0364/candidate679/source-scope-preflight/unit-composition-source-scope.json)
- [candidate679/source-scope-preflight/unit-composition-source-scope.log](candidate-baselines-adr0363-0364/candidate679/source-scope-preflight/unit-composition-source-scope.log)
- [candidate679/source-scope-preflight/did-proof-gate-unit.log](candidate-baselines-adr0363-0364/candidate679/source-scope-preflight/did-proof-gate-unit.log)
- [review-note.md](candidate-baselines-adr0363-0364/review-note.md)

### Candidate CI 57fab775

- [remote-receipts/rust-bounded-msrv-57fab7755097380b90619672be5150746f9d4a49/workflow.yml](candidate-ci-57fab775/remote-receipts/rust-bounded-msrv-57fab7755097380b90619672be5150746f9d4a49/workflow.yml)
- [remote-receipts/rust-bounded-msrv-57fab7755097380b90619672be5150746f9d4a49/result.json](candidate-ci-57fab775/remote-receipts/rust-bounded-msrv-57fab7755097380b90619672be5150746f9d4a49/result.json)
- [remote-receipts/rust-bounded-msrv-57fab7755097380b90619672be5150746f9d4a49/msrv-standalone.log](candidate-ci-57fab775/remote-receipts/rust-bounded-msrv-57fab7755097380b90619672be5150746f9d4a49/msrv-standalone.log)
- [remote-receipts/rust-bounded-msrv-57fab7755097380b90619672be5150746f9d4a49/source.json](candidate-ci-57fab775/remote-receipts/rust-bounded-msrv-57fab7755097380b90619672be5150746f9d4a49/source.json)
- [remote-receipts/rust-bounded-msrv-57fab7755097380b90619672be5150746f9d4a49/msrv-workspace.log](candidate-ci-57fab775/remote-receipts/rust-bounded-msrv-57fab7755097380b90619672be5150746f9d4a49/msrv-workspace.log)
- [remote-receipts/rust-bounded-core-57fab7755097380b90619672be5150746f9d4a49/workflow.yml](candidate-ci-57fab775/remote-receipts/rust-bounded-core-57fab7755097380b90619672be5150746f9d4a49/workflow.yml)
- [remote-receipts/rust-bounded-core-57fab7755097380b90619672be5150746f9d4a49/result.json](candidate-ci-57fab775/remote-receipts/rust-bounded-core-57fab7755097380b90619672be5150746f9d4a49/result.json)
- [remote-receipts/rust-bounded-core-57fab7755097380b90619672be5150746f9d4a49/format.log](candidate-ci-57fab775/remote-receipts/rust-bounded-core-57fab7755097380b90619672be5150746f9d4a49/format.log)
- [remote-receipts/rust-bounded-core-57fab7755097380b90619672be5150746f9d4a49/source.json](candidate-ci-57fab775/remote-receipts/rust-bounded-core-57fab7755097380b90619672be5150746f9d4a49/source.json)
- [remote-receipts/rust-bounded-core-57fab7755097380b90619672be5150746f9d4a49/clippy.log](candidate-ci-57fab775/remote-receipts/rust-bounded-core-57fab7755097380b90619672be5150746f9d4a49/clippy.log)
- [remote-receipts/rust-bounded-core-57fab7755097380b90619672be5150746f9d4a49/tests.log](candidate-ci-57fab775/remote-receipts/rust-bounded-core-57fab7755097380b90619672be5150746f9d4a49/tests.log)
- [remote-verification.json](candidate-ci-57fab775/remote-verification.json)
- [review-note.md](candidate-ci-57fab775/review-note.md)

### Candidate CI 551f065c

Exact551f065c bounded CI37750105111 passed both lanes. Core has742 passing cases across63 result blocks; MSRV qualifies only selected backend/Jubjub/isolated-backend checks. This is not full local/proof/network qualification. See [current candidate qualification](../../guides/candidate-qualification.md).

- [remote-receipts/rust-bounded-core-551f065cd6267e07020a755db5469df03e41b775/clippy.log](candidate-ci-551f065c/remote-receipts/rust-bounded-core-551f065cd6267e07020a755db5469df03e41b775/clippy.log)
- [remote-receipts/rust-bounded-core-551f065cd6267e07020a755db5469df03e41b775/fetch.log](candidate-ci-551f065c/remote-receipts/rust-bounded-core-551f065cd6267e07020a755db5469df03e41b775/fetch.log)
- [remote-receipts/rust-bounded-core-551f065cd6267e07020a755db5469df03e41b775/format.log](candidate-ci-551f065c/remote-receipts/rust-bounded-core-551f065cd6267e07020a755db5469df03e41b775/format.log)
- [remote-receipts/rust-bounded-core-551f065cd6267e07020a755db5469df03e41b775/result.json](candidate-ci-551f065c/remote-receipts/rust-bounded-core-551f065cd6267e07020a755db5469df03e41b775/result.json)
- [remote-receipts/rust-bounded-core-551f065cd6267e07020a755db5469df03e41b775/source.json](candidate-ci-551f065c/remote-receipts/rust-bounded-core-551f065cd6267e07020a755db5469df03e41b775/source.json)
- [remote-receipts/rust-bounded-core-551f065cd6267e07020a755db5469df03e41b775/tests.log](candidate-ci-551f065c/remote-receipts/rust-bounded-core-551f065cd6267e07020a755db5469df03e41b775/tests.log)
- [remote-receipts/rust-bounded-core-551f065cd6267e07020a755db5469df03e41b775/workflow.yml](candidate-ci-551f065c/remote-receipts/rust-bounded-core-551f065cd6267e07020a755db5469df03e41b775/workflow.yml)
- [remote-receipts/rust-bounded-msrv-551f065cd6267e07020a755db5469df03e41b775/fetch.log](candidate-ci-551f065c/remote-receipts/rust-bounded-msrv-551f065cd6267e07020a755db5469df03e41b775/fetch.log)
- [remote-receipts/rust-bounded-msrv-551f065cd6267e07020a755db5469df03e41b775/msrv-standalone-fetch.log](candidate-ci-551f065c/remote-receipts/rust-bounded-msrv-551f065cd6267e07020a755db5469df03e41b775/msrv-standalone-fetch.log)
- [remote-receipts/rust-bounded-msrv-551f065cd6267e07020a755db5469df03e41b775/msrv-standalone.log](candidate-ci-551f065c/remote-receipts/rust-bounded-msrv-551f065cd6267e07020a755db5469df03e41b775/msrv-standalone.log)
- [remote-receipts/rust-bounded-msrv-551f065cd6267e07020a755db5469df03e41b775/msrv-workspace.log](candidate-ci-551f065c/remote-receipts/rust-bounded-msrv-551f065cd6267e07020a755db5469df03e41b775/msrv-workspace.log)
- [remote-receipts/rust-bounded-msrv-551f065cd6267e07020a755db5469df03e41b775/result.json](candidate-ci-551f065c/remote-receipts/rust-bounded-msrv-551f065cd6267e07020a755db5469df03e41b775/result.json)
- [remote-receipts/rust-bounded-msrv-551f065cd6267e07020a755db5469df03e41b775/source.json](candidate-ci-551f065c/remote-receipts/rust-bounded-msrv-551f065cd6267e07020a755db5469df03e41b775/source.json)
- [remote-receipts/rust-bounded-msrv-551f065cd6267e07020a755db5469df03e41b775/workflow.yml](candidate-ci-551f065c/remote-receipts/rust-bounded-msrv-551f065cd6267e07020a755db5469df03e41b775/workflow.yml)
- [remote-verification.json](candidate-ci-551f065c/remote-verification.json)
- [review-note.md](candidate-ci-551f065c/review-note.md)
- [candidate-continuity.json](candidate-ci-551f065c/candidate-continuity.json)

## Final technical qualification

The [candidate guide](../../guides/candidate-qualification.md) explains the qualified f9a49666 cohort and scope. [Technical record](final-candidate-f9a49666/technical-qualification.json), [recovery receipt](final-candidate-f9a49666/remaining-receipt.json), [remote verification](final-candidate-f9a49666/remote-verification.json) and [602-file evidence archive](final-candidate-f9a49666/qualification-evidence.zip) preserve source identities and the original failed run.
