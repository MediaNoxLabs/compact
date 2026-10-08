# ADR0363 / #500 — stale DID qualification expectations

## Conclusion

The two failures at candidate `679be285d91baf87ae55ae3161140e371d259b62` are stale harness expectations. No production/backend support was changed. All twelve maintained source-scope manifests now pass (18 positive source entries, both TS and Rust compilation); 21 mocked DID proof-gate unit tests pass. Cargo lease released to root. Final successor full/proof qualification remains root-owned.

## Evidence and continuity

- Original stopped receipt: `full-gate/unit-composition-source-scope.json`; raw failure: `full-gate/logs/400-unit-composition-source-scope.log`. Only failures were `verifySchnorrJubjubDigestSignature` and `setVerificationMethodRelation` recording-gap changes. All five sources compiled on both targets, generated Cargo checks passed, maintained fixture comparisons matched.
- Manifest last changed at `770f8dcb` before digest recording commit `8a52a01001894ecb5d0e773463436b78993a119b` (ADR0288/#413) and relation recording commit `329bf1bc80441125d2800fc9f1dae8b570da5dac` (ADR0295/0303/#427). Those implementation commits intentionally added the exact exported recorded facades and maintained finite tests/reducer proof gates.
- Old expected digest diagnostic `unsupported_action / StateAction::Let / actions[0].actions[1].action.actions[1]` and relation diagnostic `unsupported_action / StateAction::CircuitCall / actions[0].action.action.action.actions[0]` described former unsupported paths. Current capability rows are available with recorded and observed-call flags true; replacing diagnostics with positive assertions retains refusal-on-regression behavior.
- Historical accepted `/tmp/rust030-joined-relation-gate/receipt.json`, head `4d2730fd418d4a48143aac88c5c182d14f9769da`, passed 19 original and six reducer proof/apply/replay calls. Its relation-schnorr sequence includes both exact operations, with nonempty 3296-byte proofs, applied true, changed binding rejection and IntentAlreadyExists replay refusal. Relation sequence covers insert/remove for five families. Constructor execution remains unproved; these historical receipts are not claimed as fresh candidate proof execution.
- Original DID source SHA in that accepted receipt and current scoped compile is identical: `632f34af543924edb185fe9c8eda54aa4ca0503b0b9e8ee3ff06ec470f6d7456`. Current normalized Rust output matches the maintained DID fixture. No product source modification or unsupported gap outside these delivered entries was made.

## Patch scope

1. `parity_positive_unit_composition_sources.json`: remove only these two obsolete gaps, require both exports recorded, update stale purpose to twelve recorded exports. Exact twelve-name capability inventory, TS/Rust success, fixture check, proof metadata and Cargo-check requirements remain.
2. `did_proof_gate.py`: compare recorded capability set to existing twelve-name `EXPORTS`, not eleven-name `KEYS`. Keep KEYS and six proof scenarios unchanged. Relation proof qualification remains separate in `did_relation_gate.py`.
3. `test_did_proof_gate.py`: mocks reflect current twelve available exports; regression rejects lost relation recording before runner build; successful orchestration asserts twelve exported capabilities versus eleven keys and no relation keygen/proof operation in this gate. Mocked orchestration is not actual proof execution.

Read-only scan of other `*sources.json` found no other original DID entry. Parent authorized one bounded preflight of all remaining full-gate source manifests: all eleven passed without further edits. No broad conformance campaign or proof rerun was performed.

## Verification

- Initial corrected unit-composition checker: exit0; same candidate compiler/Scheme snapshots and same maintained checker arguments as full gate except output receipt path.
- Remaining eleven checkers: all exit0; exact argv/elapsed/log paths in `source-scope-preflight/remaining-commands.json`, individual JSON receipts and logs alongside.
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools/compact-rust-backend -p test_did_proof_gate.py`: exit0, 21 tests, raw `source-scope-preflight/did-proof-gate-unit.log`.
- Scoped `git diff --check` on these three files: exit0. Unrelated dirty documents preserved.
- Generated checker Cargo commands are `cargo +1.99.0 check --offline --manifest-path ...`; they are not `--locked`. Compiler acceptance, fixture equality and proof-applicability metadata do not establish fresh runtime/cryptographic parity.

Environment: COMPACTC_SCHEME candidate full-gate/bin/compactc-scheme; COMPACT_RUST_RUNTIME_DIR repository root; CARGO_TARGET_DIR target/compact-rust-parity-gate; CARGO_BUILD_JOBS=4; CARGO_INCREMENTAL=0; CARGO_TERM_COLOR=never; RUSTUP_TOOLCHAIN=1.99.0; PYTHONDONTWRITEBYTECODE=1. Raw receipts retain individual source, compiler and generated lock identities. Patch/evidence hashes in `did-source-scope-fix.json`.
