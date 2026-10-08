# Candidate qualification — f9a49666

**Technical qualification passed.** The qualified implementation is `f9a496660a4a69f4dd7c7927111767ad5c0c12b2`. Documentation publication and milestone closure have separate provenance; no release tag, registry publication or deployment is implied.

## Local execution

Qualification combines **423 successful commands** from the unchanged `551f065c` run with three recovery commands at `f9a49666`: 151 Python harness tests, fresh DID relation proofs, and the remaining generic Jubjub gate. The earlier full run remains failed because its relation harness omitted the key-output directory. ADR0366/#503 repairs that directory setup and tests the fresh path. This is an explicit combination of source-identical evidence and repaired checks, not a claim that one complete run passed at the successor.

The [technical qualification record](../evidence/0.3.0/final-candidate-f9a49666/technical-qualification.json) binds the command logs, 198 regenerated fixtures, all five required proof-gate receipts, tool snapshots, source revisions and build inputs. Production compiler, runtime, generated fixtures and locks are unchanged between these two commits. All tracked non-document sources were checked against the signed candidate; final selected build-input drift is empty.

| Proof gate | Accepted execution |
|---|---|
| Consumer/proof/ledger suite | Completed at `551f065c`; each scenario retains its own strictness, funding and prior-state limits |
| DID lifecycle | Completed at `551f065c` |
| DID digest and primitive reducers | Completed at `551f065c` |
| DID relations | Fresh keys, 19 original calls and six reducer calls at `f9a49666` |
| Generic Jubjub | Seven proof/application cases at `f9a49666`; this is not Passport ACC adoption |

The [evidence archive](../evidence/0.3.0/final-candidate-f9a49666/qualification-evidence.zip) retains 602 original files plus their hash manifest, including the failed receipt. Its SHA256 is `15c8b8500a69336e1e0255b714bc2aa4dcb6fc0c3db1e2bb57b40a393143fdc4`. It excludes large binaries, proving keys and generated SDK trees; their identities remain in the receipts.

## Verified remote checks

[Run 37775401108](https://github.com/MediaNoxLabs/compact/actions/runs/37775401108) passed for `f9a49666`. The source tree, commit, both local lock hashes and every advertised artifact hash were verified. The [verification record](../evidence/0.3.0/final-candidate-f9a49666/remote-verification.json) preserves those bindings.

Linux Rust 1.99 core has **742 passing cases in 63 result blocks**, plus formatting and strict Clippy. Linux Rust 1.88 checks selected backend/Jubjub features and isolated standalone compilation. This does not establish whole-workspace MSRV, external ContractLab execution on Linux, proof execution in CI or network acceptance.

## Earlier measured cohorts

The `2c798d2d` external consumer, API migration, coverage, manually assembled portable package and dependency audit receipts retain their original identities. [Continuity through 551f065c](../evidence/0.3.0/candidate-ci-551f065c/candidate-continuity.json), followed by the two harness/test changes listed in the technical record, joins these measurements to the qualified implementation. See the [candidate consumer receipts](../evidence/0.3.0/index.md#candidate-consumers-adr0362) and [version guide](versions-and-targets.md) for exact host, feature and distribution limits.

The decoder policy remains opt-in encoded-byte admission, with a configurable 64 MiB preset; aggregate decoded heap/CPU containment is deferred in CoPS-001/#495. Strict dependency scans retain their four/three notices and recorded dispositions. Independent coding-agent review is not human security certification. No universal language parity or formal Rust correctness claim follows from this qualification.
