# ADR0324 external tutorial validation

Validated at source `5efa91c280c059e375dcde0998c64dd88f615fb6` with reviewed compiler0.31.133 / Scheme pair, Rust1.99.0 on aarch64-apple-darwin, default features. Runtime, macros, testkit and their required path fixtures were copied byte-identically into an isolated external directory. Both generated contract outputs remain unedited. The exact graphs contain one shared runtime path identity and no local package outside the external directory.

- W2 witnessed ContractLab: 1 test passed.
- D2 original DID: 2 lifecycle tests passed, including independent retained TS-capture assertions and late witness failure rollback boundaries.
- Both examples: rustfmt check and Clippy with `-D warnings` passed.
- Initial fresh offline lock selected newer cached versions; qualification rejected it before compilation. Its lock and mismatch report remain retained. Seeded source locks were pruned by Cargo, then every registry identity/checksum matched the source lock. No new dependency graph accepted accidentally.

[historical validation receipt](../evidence/0.3.0/tutorials-adr0324/validation-receipt.json) records commands, features, exact paths, source/compiler/lock/toolchain identities, preserved generated hashes, and staged SDK hashes. Selected command logs are retained in [the retained ADR0324 evidence](../evidence/0.3.0/index.md#tutorials-adr0324), under `tutorials-adr0324/logs/`; the relocation index lists the retained subset. The original full archive remains historical evidence. In the retained subset, `logs/witnessed-cell-test-final.log` and `logs/did-lifecycle-test-final.log` hold the final focused test runs, and [historical validation receipt](../evidence/0.3.0/tutorials-adr0324/validation-receipt.json) binds the source/compiler/lock/toolchain and command identities. The command recipes in the two tutorials now seed the source lock explicitly. The original unexecuted draft archive remains unchanged; the current guide prose has since been reconciled with these results.

W2 does not itself compare TS. These are native tests plus VM replay, not proof or network acceptance; no cold-cache, MSRV or WASM claim. Separate rustdoc and compile-fail evidence is described in [the snippet matrix](snippet-matrix.md); [candidate qualification](candidate-qualification.md) records the overall status. At this historical validation checkpoint ADR0285 remained stopped and untouched. It was later resumed and delivered at4c8aebc3; that separate receipt does not extend this checkpoint’s test scope.


## Reusing this historical receipt

The results above belong to source `5efa91c2`, Rust1.99.0 on aarch64-apple-darwin and the recorded default-feature graphs. Later prose corrections and API-navigation additions are not new executions of those examples. The separately retained ADR0326 WitnessScript rustdoc run has its own identity and result. Final candidate qualification must compare the selected compiler, source, lock and generated outputs before promoting any current-release claim; it must not relabel these frozen logs as a later run.
