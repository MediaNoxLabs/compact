# Local circuit artifact caching with Midnight ZKIR Forge

This is an **opt-in two-circuit experiment**, not a dependency upgrade or a
replacement for the maintained uncached proof gates. It adds no runtime or
compiler dependency on Forge. Run one sequential Python controller; the Rust
proof consumer retains its own existing stack/thread policy.

## Contract and compatibility

The consumer remains pinned to ledger8 **8.0.3**, ZKIR 2.1.0, transient-crypto 2.0.1,
serialize 1.0.0, proofs 0.7.0, circuits 6.0.0 and stdlib 1.0.0. The installed Forge
profile is ledger8 **8.1.3**, ZKIR 2.1.1 and its newer pinned dependencies. Those
graphs are different; parsing old ZKIR is insufficient evidence of compatibility.

For **only** `let_return_oracle.compact::replace` and
`effect_plan_counter_only.compact::adjust`, local qualification demonstrated:

1. Fresh official ZKIR 2.1.0 generation and installed Forge generate byte-identical
   binary IR, prover keys and verifier keys.
2. Both use the same actual public-parameter bytes (k=6 and k=8 respectively).
3. The existing 8.0.3 proof consumer deserializes these artifacts and proves,
   verifies and ledger-applies the original scenarios.

`forge_circuit_qualification.json` pins the exact current inputs, parameters and
outputs. The adapter also checks the installed Forge profile/build fingerprint
and consumer Cargo.lock versions. Unknown inputs or version changes fail closed;
do not refresh these pins automatically. Qualify a changed circuit with uncached
generation and real consumer execution before reviewing new pins. No general
cross-version ABI guarantee or complete ledger8.1.3 adoption is claimed. This
qualification pins the installed ARM macOS build; another Forge build must be
explicitly qualified rather than bypassing the fingerprint check.

## Run the two real tests

Build the current consumer with the repository's existing pinned toolchain:

```sh
cargo +1.99.0 build --locked --offline -p compact-rust-proof-smoke
```

Set `COMPACTC` to a current Rust-target compiler and `COMPACTC_SCHEME` as required
by that compiler. Set `MIDNIGHT_LEDGER_TEST_STATIC_DIR` to the authenticated
ledger8.0.3 static directory, following the ordinary local proof setup.
The compiler still needs its standard library/tool environment. Use the same
`MIDNIGHT_PP` directory as the ordinary proof runner, containing verified public
parameters. All paths below are explicitly configured, not repository-private
runtime dependencies.

```sh
python3 -B tools/compact-rust-backend/check_forge_circuit_cache.py \
  --compiler "$COMPACTC" \
  --proof-runner "$CARGO_TARGET_DIR/debug/compact-rust-proof-smoke" \
  --params-dir "$MIDNIGHT_PP" \
  --report /absolute/evidence/forge-tests.json
```

Default executable: `~/.local/bin/midnight-zkir-forge` (override `--forge`).
Default store: Forge's normal `~/.midnight-zkir-forge`, respecting
`MIDNIGHT_ZKIR_FORGE_DIR`. Use `--cache-dir /absolute/private/cache` for controlled
cold/warm measurements or lifecycle experiments. The test runner never evicts or
cleans the shared store. Private test fixture directories are automatically removed.

Repeat the identical command with a different report path and the same private
cache directory to measure warm behavior. The report captures before/after Forge
statistics, full in-process wall time, per-case phase wall time, prepare process
wall time, Forge's separate ensure/generation timing and child process memory
highwater. Time the complete Python command externally for startup-inclusive
wall time. Memory is the cumulative largest-child-process highwater, **not**
aggregate concurrent memory. Avoid interpreting warm/cold host noise as a general
speedup guarantee.

## What runs on every hit

- Compile the current Compact source with `--skip-zk`: regenerate/read current
  ZKIR; check the original compiler manifest and the selected generated fixture.
- Invoke `prepare` with an absolute input path using a subprocess argument array;
  parse JSON and verify the persistent exported files and parameter identities.
- Give the original consumer the returned persistent artifact paths through
  temporary links used only for reads, with post-use artifact hash checks.
  These links do not enforce filesystem write protection. Retain the consumer's
  Counter IR-pairing check.
- Execute the original proof generation, verification, replay/parity checks,
  ledger application and state assertions. Counter exercises two branches;
  `replace` exercises one call. These are existing unbalanced ledger smoke
  scenarios, not a claim of funded production transaction policy.
- Copy the consumer fixture into private files, corrupt its verifier and require
  rejection. Never write through a link into Forge's persistent export.
- Check the original artifacts remain unchanged, then remove temporary fixtures.

Nonzero Forge exits, invalid JSON, profile/input/parameter drift, missing or
corrupt exports propagate. There is no stale-file or uncached fallback. No proof,
test result, assertion or pass/fail decision is cached.

## Keep uncached key-generation qualification

The same two scenarios can bypass Forge and freshly regenerate their keys with
the pinned legacy ZKIR executable:

```sh
python3 -B tools/compact-rust-backend/check_forge_circuit_cache.py \
  --compiler "$COMPACTC" \
  --proof-runner "$CARGO_TARGET_DIR/debug/compact-rust-proof-smoke" \
  --params-dir "$MIDNIGHT_PP" \
  --uncached-zkir /absolute/legacy-2.1.0/bin/zkir \
  --report /absolute/evidence/uncached-tests.json
```

This path does not require Forge to be installed. It compares regenerated output
digests and still runs proofs and controls. The existing `check_compactc_target.py
--consumer --proof` recipe also remains unchanged and uncached. Forge's own
`qualify` is useful additional evidence, not a replacement for this consumer's
qualification.

Fast adapter checks:

```sh
cd tools/compact-rust-backend
python3 -B -m unittest -v test_forge_cache
```

## Where this helps

The cache removes repeated circuit artifact generation when input and recipe
match. Circuit compilation and actual proofs still cost time. For these small
fixtures key generation is only tens of milliseconds, so complete-loop savings
are modest. Larger key-generation workloads need their own measurements.

The multi-hour historical admission suite instead reconstructs/authenticates
source snapshots; these circuit files are not its dominant cost. This integration
does not change its assertions, guards, worker assignment or execution machinery.
A ledger8.1.3 upgrade remains a separate compiler/runtime/application parity slice.

### Measured local pilot

Three repetitions of each mode on ARM macOS, two sequential tests per invocation:

| Mode | Median complete process wall | Median combined Forge ensure time |
| --- | ---: | ---: |
| Cold Forge, private empty store | 2.800s | 220.46ms |
| Warm Forge, same store | 2.508s | 12.46ms |
| Uncached legacy ZKIR | 2.567s | not applicable |

Observed warm median: 10.4% below cold Forge and 2.3% below the uncached legacy
test. These are three local samples of tiny circuits, not a broad performance
guarantee. Warm requests report zero generation time. Each invocation still
executes three proof/ledger applications and two private corrupt-verifier
refusals. Both modes' largest-child-process memory highwater is approximately
233MiB; no meaningful memory saving is demonstrated.
