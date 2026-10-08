# Final technical qualification — f9a49666

Status: technical execution accepted; documentation publication and milestone closure remain pending.

Candidate: `f9a496660a4a69f4dd7c7927111767ad5c0c12b2`, signed conventional commit with DCO and verified Good GPG. ADR0366/#503 fixes two key-directory omissions in the relation harness and adds one fresh-path orchestration regression. Production compiler/runtime/generated fixtures and locks are unchanged from551f065c.

## Complete execution evidence

The qualification combines 423 successful commands from the551 full-run prefix with three fresh recovery commands: 151 Python harness tests, the repaired fresh DID relation gate, and the remaining Jubjub gate. The original551 full run remains failed at its missing key-output directory; it is not relabeled or represented as a single fresh full run at the successor.

All198 fixture source/generated hashes were checked. Every completed command log, all five typed proof-gate receipts and their command logs were authenticated. Current tracked non-document source matches the signed candidate; the only successor changes are the relation harness and its test. Compiler/Scheme snapshots, selected tools, locks and Cargo configuration are unchanged. Final build-input drift is empty.

Fresh relation proofs include19 original calls and6 reducer calls with default-strict application and replay controls. The retained generic Jubjub gate passes7 cases; this is not Passport ACC adoption. The earlier consumer/proof/ledger suite passed in2,399seconds. DID lifecycle, digest and primitive reducers retain their successful551 receipts.

[Remote run37775401108](https://github.com/MediaNoxLabs/compact/actions/runs/37775401108) passes. Source tree, commit, both local lock hashes and every advertised artifact hash were verified. Core has742 passing cases in63 result blocks plus formatting/Clippy; MSRV remains selected Linux Rust1.88 backend/Jubjub/isolated standalone compilation. No whole-workspace MSRV, Linux external ContractLab, network or human security certification is implied.

## Artifact and remaining work

[[Final technical qualification — f9a49666.zip]] retains 602 original files plus a byte-hash manifest, including the failed prefix receipt, all required successful gate receipts/logs and current remote artifacts. Archive SHA256: `15c8b8500a69336e1e0255b714bc2aa4dcb6fc0c3db1e2bb57b40a393143fdc4`. Executable binaries, proving keys, large cached inputs and generated SDK trees are not embedded; their identities remain in the source-bound receipts.

`/tmp/rust030-candidate-f9a49666/technical-qualification.json` is the root-accepted technical record. Earlier2c798d2d external consumer, migration, coverage, package and audit evidence keeps its measured identity and joins through source equality. Its scope limits remain unchanged.

#503's bounded repair is complete. #352/#449 still require actual documentation publication, #364 closes after that required deliverable, and #358 requires the final93-criterion crosswalk and closure report. Parent acceptance remains16/19 until those actions are evidenced. The corpus now has121 milestone ADRs; the prior120-record import preparation must be extended before use.
