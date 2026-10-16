# Local circuit artifacts with Midnight ZKIR Forge

Local circuit tests can optionally reuse binary ZKIR, prover keys and verifier keys through
[Midnight ZKIR Forge](https://github.com/betsudo/forge).
Proof generation, verification, assertions, negative controls and fixture checks
still execute on every run. Test outcomes and proofs are never cached.

The optional [Forge proof workflow](../../../.github/workflows/forge-local-ci.yml)
prepares the same integration for CI. The required Rust-AST checks run without
Forge; the proof workflow is a separate lane.

## Matching tools

The active consumer uses the official ledger **8.1.3** revision
`b5d3e965a18e4cac17fac4f38446fc55c524439a`: ZKIR/transient-crypto 2.1.1,
base-crypto 1.0.2 and serialization 1.1.1. The matching registry ZK graph is
proofs 0.7.1, circuits 6.1.0, stdlib 1.1.0 and curves 0.2.0.
Generated consumers and the paired runtime require Rust 1.89 or newer.

The adapter checks the exact installed Forge profile/build, consumer lock,
actual public parameters and current source bytes. It also runs the actual
ZKIR 2.1.1 CLI's `mock-compile` on every preparation to check binary serialization
and obtain fresh k/rows measurements. This performs no key generation. Receipts
identify Forge preparation separately from these actual CLI measurements.
Another Forge build needs explicit qualification; a matching version string
alone is insufficient. Failures propagate without stale-artifact fallback.

## Local commands

Install Forge following its README. Default CLI: `~/.local/bin/midnight-zkir-forge`.
Default store: `~/.midnight-zkir-forge`; override with
`MIDNIGHT_ZKIR_FORGE_DIR` or the launcher's `--cache-dir`.
Set `MIDNIGHT_PP` to the same verified public parameters used by the proof runner.
The current actual `zkir` executable must be on PATH.

Ordinary compiler and test entry points generate artifacts with the existing tools.
They do not require Forge. This includes `compiler/go`, `run-e2e-tests.sh`,
`run-debug-test.sh`, and maintained Python gates with no cache mode set.

For an explicitly cached local run, use `./run-e2e-tests-cached.sh` or
`./run-debug-test-cached.sh`. An inherited `COMPACT_ZKIR_MODE=uncached` also
disables caching in these explicit wrappers. For a Python gate, set
`COMPACT_ZKIR_MODE=forge`.
To wrap another local command, enter the matching development environment and run:

```sh
python3 -B tools/compact-rust-backend/forge_circuits.py \
  --receipt-dir /absolute/evidence/forge-events -- YOUR_TEST_COMMAND ARGUMENTS
```

The wrapper supplies a temporary `zkir` launcher for `compile` and `compile-many`.
Each request reads the newly generated input, invokes `prepare` with its absolute
path through an argument array, parses JSON and checks every returned artifact.
Forge returns persistent export paths. Private copies populate the normal compiler
output layout before manifests are generated. Tests can safely mutate their own
copies without changing exports or pinned verifier identities.

For ordinary Python delivery, use affected tests first. Run the relevant real proof
gate for changes to code generation or runtime behavior. Complete historical
qualification is required when historical machinery changes and at final candidate
freeze; a Forge hit never replaces that qualification.

## Fresh key-generation qualification

Keep an explicit uncached lap:

```sh
COMPACT_ZKIR_MODE=uncached python3 -B tools/compact-rust-backend/check_compactc_target.py --counter-less-than --proof
./compiler/go
python3 -B tools/compact-rust-backend/forge_circuits.py --uncached -- YOUR_TEST_COMMAND
```

These paths call the actual tool and generate keys afresh. Hermetic Nix builders
also retain uncached generation and do not depend on a user's Forge installation.
Production compiler use outside the local test wrapper retains its original tool
behavior. The optional bundle launcher hook preserves a configured test shim;
ordinary installations have no Forge dependency.

ZKIR v3 uses its separate actual CLI and is **uncached** by this v2 integration.
TS WASM checks with intentionally empty keys keep executing as checks; they are
not key generation. Gates explicitly given retained material continue checking
and consuming that material, preserving corrupt-key controls rather than replacing
it with a cache copy.

## Cold/warm real proof sample

Build the current proof runner:

```sh
cargo build --locked -p compact-rust-proof-smoke
```

Configure `COMPACTC`, `COMPACTC_SCHEME`, `COMPACT_PATH` and the authenticated
`MIDNIGHT_LEDGER_TEST_STATIC_DIR` as for the ordinary proof gates. Resolve the
latter with `resolve_ledger_test_static.py` against current Cargo metadata/lock.

```sh
python3 -B tools/compact-rust-backend/check_forge_circuit_cache.py \
  --compiler "$COMPACTC" \
  --proof-runner "$CARGO_TARGET_DIR/debug/compact-rust-proof-smoke" \
  --params-dir "$MIDNIGHT_PP" \
  --cache-dir /absolute/private/forge-store \
  --report /absolute/evidence/cold.json
```

Repeat with the same private store and a different report path for the warm lap.
Use `--uncached-zkir /absolute/path/to/zkir` for the fresh qualification lap.
The two circuits in `forge_circuit_qualification.json` are smoke examples, not a
whitelist limiting other matching circuits. Their original proof/ledger assertions
and private corrupt-verifier controls execute in all modes.

Reports separate complete test wall time, Forge process/ensure timing, key
generation timing, actual proof/assertion phases and before/after cache statistics.
Measure Python startup-inclusive wall time externally when benchmarking. Child
memory highwater describes the largest child process, not simultaneous aggregate
memory. Report only measured savings for the tested cases.

## Cache integrity and scope

Use private stores for deletion, eviction and corruption experiments. Never delete
the shared default cache to manufacture a cold run. Returned persistent exports
must match their manifest, current source, profile and actual parameter digest;
the adapter verifies all bytes before use and checks source stability during
preparation. Non-ASCII or long compiler filenames retain their output names; only
Forge's cosmetic bundle name is replaced with a deterministic content-derived name.

The earlier 8.0.3 two-circuit experiment is retained in its historical evidence.
It does not qualify new circuits or the upgraded runtime. Artifact caching also
does not address the historical Python suite's reconstruction/authentication cost.

## Observed local result

A sequential controller tested the current 8.1.3 consumer with a private store:

| Complete case | Cold Forge | Warm Forge | Fresh uncached |
|---|---:|---:|---:|
| Counter + Let | 1.955s | 1.725s | 1.736s |
| Jubjub | 5.567s | 3.743s | 4.939s |

The tiny cases showed no meaningful saving. Jubjub was 24% shorter warm in this
single comparison; its compiler/artifact phase was 1.263s warm versus 2.298s
uncached. These are local observations, not a universal speedup or an uncontended
host benchmark. Largest-process RSS was approximately 84–86 MiB.

Those six laps executed 18 proof/verification/ledger-application scenarios and
nine private corrupt-verifier refusals. All source, binary-IR and key hashes
matched uncached generation; internal compiler manifests were also checked.
Jubjub's third circuit, `raw_after_write`, was generated and compared but was not
proof-executed by that selector. A later compiler rebuild was checked separately:
its complete generated tree matched, then three fresh Jubjub scenarios and one
corrupt-verifier control passed. The historical Python qualification was not rerun
or measured as part of this sample.

## Public Forge qualification — 2026-10-17

The public source home is [betsudo/forge](https://github.com/betsudo/forge).
The additional qualified source is commit
[`4ad82c8e1e687b64b7a440679fb8f648358106f0`](https://github.com/betsudo/forge/commit/4ad82c8e1e687b64b7a440679fb8f648358106f0),
with ledger8 build fingerprint
`bd1a54e776b8af58fb6ed682489b82d265aca4175040bd90c4acafa6db843e9f`.
Qualification was performed locally on **aarch64-darwin**, Compact 0.31.135,
using the current locked ledger graph. This does not qualify Linux or hosted CI.
The older accepted build remains supported by its original adapter contract.

For the new build, the adapter also requires the materialized descriptor and its
live sidecar to match the manifest, recipe and exact binary serialization:
`ir-source[v2]`, `prover-key[v7](ir-source[v2])`, and `verifier-key[v6]`.
A missing descriptor, incompatible build or corrupt artifact is an error.

| Complete test process | Cold Forge | Warm Forge | Fresh uncached |
|---|---:|---:|---:|
| Counter + Let | 3.787s | 1.324s | 1.285s |
| Jubjub | 4.409s | 2.642s | 3.031s |

These are single sequential comparisons, including actual proof/verification,
ledger assertions and private corrupt-verifier refusals in every mode. All source,
binary IR and key bytes matched fresh generation. The small sample was slightly
slower warm; Jubjub was 12.8% shorter warm. This does not establish a general or CI
speedup. `raw_after_write` was generated and compared but not proof-executed.
The earlier table is historical evidence for the earlier build, not a new result.

## Isolated local snapshot lane

Build the pinned public Forge package, current compiler and proof runner. Keep the
matching actual `zkir` on PATH and configure the authenticated ledger static fixture
as described above. Use fresh output paths for every invocation:

```sh
python3 -B tools/compact-rust-backend/forge_ci.py bootstrap \
  --compiler "$COMPACTC" --forge "$FORGE" --params-dir "$MIDNIGHT_PP" \
  --output "$EVIDENCE/bootstrap"
python3 -B tools/compact-rust-backend/forge_ci.py plan \
  --compiler "$COMPACTC" --forge "$FORGE" --helper "$FORGE_CI" \
  --proof-runner "$PROOF_RUNNER" --params-dir "$MIDNIGHT_PP" \
  --generation local-1 --output "$EVIDENCE/plan-1"
python3 -B tools/compact-rust-backend/forge_ci.py run \
  --plan "$EVIDENCE/plan-1" --snapshot "$EVIDENCE/absent-snapshot" \
  --snapshot-out "$EVIDENCE/snapshot-1" --uncached-zkir "$ZKIR"
```

Bootstrap only verifies existing parameters unless `--fetch-params` is explicitly
provided; acquisition uses Forge's official verifying provider. A wrong existing
parameter is refused. Planning binds current source, compiler, proof consumer,
helper, Forge, parameter bytes and platform. A fresh second plan can restore
`snapshot-1`; proofs and controls still run. Run markers prevent reusing a test
verdict. The official helper owns partitioning, snapshot validation and post-restore
statistics; malformed present snapshots fail instead of becoming cold misses.

The prepared workflow uses an isolated store for each job, separate build and
parameter caches, and a 256 MiB snapshot budget. Pull requests restore only.
Only successful pushes to `codex/rust-backend-ast` may publish circuit snapshots;
GitHub event/ref conditions and cache scopes establish that trust. The helper's
`--tests-passed` argument alone grants no authority. It never stores proofs or test
reports in the snapshot. An all-hit run does not save another snapshot.

The Apple Silicon workflow is prepared and locally linted. Its first hosted run,
hosted timing and any branch-protection configuration remain operational steps
after publication. A different build/profile fails closed pending qualification.

### Snapshot runner acceptance

The same runner was exercised locally with a fresh store and a new isolated restore.
The cold run recorded three misses; the restored run recorded three hits, zero
misses/errors and zero key-generation time. Both executed the three Jubjub proof
scenarios and verifier-corruption refusal. Artifact hashes were identical; private
fixtures were removed. A reused plan and a present corrupt snapshot were refused.
The snapshot contained 3,527,075 payload bytes, below the workflow budget.

Complete final runner processes took 9.570s cold and 5.438s restored. These numbers are
**not a speed comparison**: the cold invocation additionally ran the two-case
uncached qualification and created the snapshot. Helper job elapsed time also
includes its source/tool/parameter checks, not just Forge preparation. No hosted
cache transfer or hosted timing was measured.

### CI cost controls

The optional proof workflow filters pushes and pull requests to compiler/runtime,
backend and proof inputs, fixtures, locked dependencies, Nix definitions and its
own workflow. Compiler source inputs also include `third_party/compiler` and
`srcMaps`. It does not encode temporary stack branch names. Documentation-only
changes outside those inputs do not start the macOS lane; explicit dispatch remains
available after GitHub registers the workflow. Required Rust-AST checks have no
path filters.

The Forge lane caches only Cargo registry/git downloads, public parameters and
bounded stopped Forge snapshots. It no longer caches the entire macOS target
directory or keys a build cache on every workspace Rust file. This reduces cache
churn and storage pressure; the actual hosted time trade-off must be measured.
The default Darwin Chez fix is already present in the AST line (`159d4bc51a`);
the pinned compiler uses `chez = pkgs.chez` rather than the obsolete LLVM18 override.
