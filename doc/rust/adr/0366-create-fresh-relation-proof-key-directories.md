---
id: RUST-ADR-0366
alias: ADR-0366
source_sha256: b0e9bdaed1609fc3f7824f09108a35fd8326a60f744f13883278ee1de0fd0f59
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0366 — Create fresh relation proof key directories

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md)

## Publication disposition

Delivered at signed/DCO `f9a496660a4a69f4dd7c7927111767ad5c0c12b2`, issue #503. Fresh relation proof qualification passed; the prior failed full run is retained separately. Publication preserves the original chronology and does not extend the proof scope.

## Original decision and amendments

## ADR0366 — Create fresh relation proof key directories

Date: 2026-10-08
Status: delivered; fresh proof qualification passed
Issue: [#503](https://github.com/MediaNoxLabs/compact/issues/503)
Parent: #364

### Problem

The full 551f065c qualification reached the DID relation gate after the consumer/proof suite, DID lifecycle, digest reducer and primitive reducer gates passed. Fresh key generation failed with `No such file or directory`. The ZKIR input exists; the key-output parent does not. The `--skip-zk` compiler output deliberately leaves key generation to the caller. Retained-material copying creates parents, so that path masked the fresh-path omission. The original-DID path and all three relation reducer paths have the same ownership mistake.

### Decision and before/after

The relation qualification harness owns its output directories. Create `keys/` before each fresh key-generation invocation. Retain exact inventories, material validation and proof assertions.

```python
# Before: fresh compiler output has no keys directory.
execute([zkir, "compile", input_zkir, output / "keys/op.prover", output / "keys/op.verifier"], label)
```

```python
# After: prepare the destination owned by the harness before key generation.
(output / "keys").mkdir(exist_ok=True)
execute([zkir, "compile", input_zkir, output / "keys/op.prover", output / "keys/op.verifier"], label)
```

### Ownership and backend impact

Only `did_relation_gate.py` changes behavior. The compiler, Rust emitter, generated crate, runtime ABI, official ledger/zk types and TS backend remain unchanged. This finding is in the Rust qualification harness and does not demonstrate a defect in mainstream ledger8 or the Compact specification. No directory side effect is added to the upstream key generator or `--skip-zk` compiler contract.

### Verification

Add a `run_gate` fresh-path regression with real temporary files and a command double that requires key-output parents to exist. It must cover five original operation keys and all three reducer keys, with exact existing scenario validation. This is orchestration evidence only. Run the actual fresh relation gate to establish key generation and 19 original plus six reducer proof calls. Then run the retained Jubjub gate, which the stopped full run had not reached.

Preserve the original failed full receipt. Completed stages may join a final qualification record only through explicit unchanged source/tool/lock evidence; do not rewrite the failed run as successful or call retained measurements fresh executions. Repeat unrelated expensive proofs only if the repair affects their inputs.

### Alternatives

Creating directories manually in the failed run would hide the fresh-path regression. Reusing old keys would bypass the failing path. Repeating the entire multi-hour gate without a fix would reproduce the same deterministic error. None is the chosen repair.

### Implementation checkpoint

Signed/DCO commit `f9a496660a4a69f4dd7c7927111767ad5c0c12b2` is pushed and has a verified Good GPG signature. The initial local signature failed verification; the unpushed commit was re-signed with identical source/message before pushing. Only two harness/test files differ from 551f065c; production code and dependency locks are unchanged.

The fresh-path regression reproduced the missing key parent before the fix. All nine relation-harness tests and all 151 Python harness tests pass after the fix. Recovery session 1996 runs genuine fresh key generation and proof/apply checks, followed by the unexecuted retained Jubjub gate. Its receipt will preserve the original failed full run and the successful unchanged prefix. Remote run 37775401108 reports success; artifact verification remains separate.

### Final repair acceptance

All 19 original relation and six reducer proof calls passed with freshly generated material. The retained generic Jubjub gate passed seven cases. The root technical record authenticates the423 successful unchanged prefix commands plus the three recovery commands, including151 Python tests. Final source/tool/lock checks pass. Remote37775401108 source tree, locks and all advertised artifacts are verified. #503 is closed; parent #364 awaits the documentation deliverable.

Evidence: [Final technical qualification](../guides/candidate-qualification.md). Archive SHA256 `15c8b8500a69336e1e0255b714bc2aa4dcb6fc0c3db1e2bb57b40a393143fdc4`. The failed551 full receipt remains failed and retained; composite qualification is explicitly labeled.
