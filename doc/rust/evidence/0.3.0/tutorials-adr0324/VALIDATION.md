# ADR0324 external tutorial validation

Validated at source `5efa91c280c059e375dcde0998c64dd88f615fb6` with reviewed compiler0.31.133 / Scheme pair, Rust1.99.0 on aarch64-apple-darwin, default features. Runtime, macros, testkit and their required path fixtures were copied byte-identically into an isolated external directory. Both generated contract outputs remain unedited. The exact graphs contain one shared runtime path identity and no local package outside the external directory.

- W2 witnessed ContractLab: 1 test passed.
- D2 original DID: 2 lifecycle tests passed, including independent retained TS-capture assertions and late witness failure rollback boundaries.
- Both examples: rustfmt check and Clippy with `-D warnings` passed.
- Initial fresh offline lock selected newer cached versions; qualification rejected it before compilation. Its lock and mismatch report remain retained. Seeded source locks were pruned by Cargo, then every registry identity/checksum matched the source lock. No new dependency graph accepted accidentally.

`validation-receipt.json` records commands, features, exact paths, source/compiler/lock/toolchain identities, preserved generated hashes, and staged SDK hashes. Full command logs are in `logs/`. The command recipes in the two tutorials now seed the source lock explicitly. Original draft/vault files remain unchanged.

W2 does not itself compare TS. These are native tests plus VM replay, not proof or network acceptance; no cold-cache, MSRV or WASM claim. Separate rustdoc/compile-fail mapping and eventual repository publication remain parent requirements. ADR0285 remains stopped and untouched.
