# Portable Python discovery and manual graph qualification

Ordinary discovery uses only public checked-in data:

```sh
python3 -B -m unittest discover -s tools/compact-rust-backend -p 'test_*.py'
```

## One immutable conformance baseline

The original repository-baseline case is a historical source/mapping law from
`6e37443b4fc2f5363954e0aef88373a48f4d3ae8`. Its unchanged baseline intentionally
refuses the current implementation. The case now explicitly selects the bundled
150-file source fixture in `tools/compact-rust-backend/tests/retained-conformance-6e37443b`.
The manifest records exact original Git paths, byte counts and SHA256 values;
original license notices remain intact. No network, compiler or private checkout
is required. The original checker authenticates all source, grammar, reference
and formal-artifact inputs before the case and on cleanup. The other 29 tests
keep their current roots. No current-language coverage or Agda proof is claimed.

`COMPACT_CONFORMANCE_BASELINE_ROOT` explicitly selects a different archive.
An invalid, missing or empty override fails; it never falls back to the bundle.
The authenticator itself still refuses a missing argument and current-root drift.
Do not regenerate the historical baseline or modify the bundle to hide drift.
The extra fixture files are inert historical data, never current compiler sources.

## Artifact-dependent graph controls

`test_guarded_point_zkir.py` is a sidecar for actual baseline/candidate graphs.
When ordinary discovery has neither an explicit `ARTIFACTS` selection nor
`COMPACT_GUARDED_POINT_ROOT`, its ten cases are reported as skipped with a manual
artifact reason. A skip does not qualify these laws. Their original assertions
remain unchanged, and runtime selection allows the existing CLI/sharder to set
`ARTIFACTS` after import. Invalid explicit selection is an error, not a skip.

With genuine fresh output from `guarded_point_zkir.py`, run the original CLI:

```sh
python3 -B tools/compact-rust-backend/test_guarded_point_zkir.py \
  --artifacts "$GRAPH_OUTPUT" -v
```

That command must execute ten cases with zero skips. Retained graph replay can
instead use `COMPACT_GUARDED_POINT_ROOT`, which authenticates the unchanged two
original graph hashes, four authority receipts and the nested source before
loading and on cleanup. Original producer identities remain historical. No raw
private graph receipt is distributed here, and replay is not fresh emission,
proof generation or full historical qualification.

The accepted contribution train also retains ten archive-fixture controls and ten
graph-input controls. Archive controls use the bundled default through scoped
environment selection and restore the prior environment after each case. Nine
graph-input controls require the private graph data and skip at fixture copying
when its environment variable is unset; their pure missing/relative/wrong-input
control always runs. Explicit bad selections still fail. Thus ordinary no-env
discovery reports nineteen manual-artifact skips; selecting authenticated graph
inputs executes all twenty graph controls without these skips.

## Full qualification remains explicit

The full local gate may select the bundled conformance root, but it still
requires actual graph inputs and the existing ledger/tool/proof prerequisites:

```sh
python3 -B tools/compact-rust-backend/local_parity_gate.py --full \
  --guarded-point-artifacts "$RETAINED_GRAPH" \
  --compiler "$COMPACTC" --scheme "$COMPACTC_SCHEME"
```

It authenticates historical data before tool lookup/builds, passes the canonical
roots into discovery and authenticates them again after qualification. Missing
graphs or changed archive/authority/source bytes fail closed. This full mode must
execute the original ten graph controls and ten retained graph-input controls
with no artifact-dependent skips. Default
unit discovery with skips is not a successful full gate, a rerun of the accepted
974 historical laws or a proof result. Test outcomes and proofs are never cached.
