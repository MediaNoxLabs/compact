# ADR0364 / #501 — conformance anchor maintenance delivered locally

Source files are frozen. Root owns review/commit; no commit/push/vault operation by this agent. Shared Cargo/resource lease released.

## Changes

- Updated five reviewed current-source hashes and corresponding44 equal-block anchors. Original `baseline.source_commit` and grammar inventory remain unchanged; separate `current_anchor_commit` identifies679be285.
- Preserved ADR0348's original four closure hashes under `recorded_source_hashes`, keyed by filenames, with originalcb687ddc checkpoint. Preserved ADR0349's original harness hash and ownercb687ddc/root3455f2e9 checkpoints; its current test anchor is explicitly distinguished.
- Added separate current-native evidence `E-boolean-native-candidate-679be285`, backed by one actual five-test run, current source/lock/toolchain inventory and raw log. It does not relabel old logs, execute TS, regenerate code, establish recording/proof/formal correspondence or promote any existing support status.
- Added one regression proving historical recorded hashes do not replace current top-level or current-anchor drift guards. Checker code unchanged.

`projection-check.json` verifies original grammar/classifications/support/requirements/formal projections and old evidence IDs, strength, logs, results and qualification unchanged (only reviewed live source anchor coordinates/hashes may differ). The native source inventory matches the frozen candidate source bytes, independently of unrelated working changes.

## Verification

| Gate | Result |
|---|---|
| Existing Boolean conformance integration |5 passed;4.91s compile,0.02s tests |
| Specification Python tests, including negative drift controls |30 passed |
| Entire maintained Python harness discovery |150 passed in6.609s |
| Cached baseline check against frozen679be285 source |268 structural requirements,30 families, zero errors |
| Owned-file diff whitespace check |passed |

The prior49 failures were five source hashes repeated across44 anchors, not49 semantic gaps. This is baseline maintenance, not a new conformance campaign. The generic full-tree diff check still reports the user-owned trailing blank in `doc/ledger-adt.mdx`; it was preserved and is outside this task/input set.

## Final source identities

- `tools/compact-rust-backend/specification_conformance.json`: `59f85282379664c5b93fa37846c97b315699cbe41897d0180a6e80ccd6ef1bc9`
- `tools/compact-rust-backend/test_specification_conformance.py`: `25d02a9538030c4d0d772dc367b65ecb7e47c85fb0fb17fb6694279b3f266e96`

Raw logs, original baseline copy, reviewed refresh script, projection check, current native receipt and complete artifact hashes are under `/tmp/rust030-adr0364/`. `final-receipt.json` records commands and outcomes. No live process remains.
