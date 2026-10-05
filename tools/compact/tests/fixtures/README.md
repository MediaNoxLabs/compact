# Genuine compiler fixtures (ADR228)

These files pin official historical compiler archives for **test-only** installer acceptance. Production sources and existing scenario assertions are unchanged. The writable `ArchiveFixture` is distinct from the no-install/read-only fixture.

## Mandatory setup before installer integration tests

Run from the repository root, before the existing `cargo nextest run -p compact ...` job. Python 3.11+ and ordinary HTTPS access are required for acquisition. Execution uses only verified local ZIPs; missing assets fail, with no ignored tests or implicit downloads.

```sh
python3 -m unittest discover -s tools/compact/tests/fixtures -p 'test_*.py'
python3 tools/compact/tests/fixtures/acquire_compilers.py \
  --platform auto \
  --cache "$RUNNER_TEMP/compact-compiler-fixtures" \
  --receipt "$RUNNER_TEMP/compact-compiler-fixture-provenance.json"
export COMPACT_TEST_ARCHIVE_CACHE="$RUNNER_TEMP/compact-compiler-fixtures"
cargo +1.99.0 test -p compact --test test_archive_fixture --locked -- --test-threads=1
```

The helper selects `--platform auto` from actual OS/architecture, so runner label changes cannot select the wrong asset: macOS ARM `aarch64-darwin`, macOS Intel `x86_64-apple-darwin`, Linux x86 `x86_64-unknown-linux-musl`; published Linux ARM rows use `aarch64-unknown-linux-musl`. Both configured macOS runner labels are covered regardless of their current architecture. Unsupported historical platforms stay absent in the catalogue. No test skip is introduced. Set `PYTHONDONTWRITEBYTECODE=1` for Python checks.

`--seed-dir` optionally reads already acquired files at `compactc-vVERSION/ASSET_NAME`; each is verified before reuse. It never reads or changes the user's installed version tree automatically. `--verify-only` refuses missing/corrupt assets without network. Cache files are hash-named and published by atomic rename only after size/digest validation. CI may cache this directory keyed by the manifest hash; verification still runs on every fixture construction.

## Evidence and boundaries

The manifest carries official release/asset IDs, canonical URLs, sizes and hashes. Most hashes are GitHub-reported SHA256 values. Historical assets whose `reported_digest` is null instead use explicitly labeled hashes of exact official downloads. These hashes ensure repeatable bytes, not a publisher signature. Asset availability still matters on a cold acquisition runner.

The nine-version metadata catalogue preserves the existing list expectations; the five versions executed by existing scenarios have archive pins. Selecting an unacquired metadata-only version fails locally. The standard-library HTTP fixture allows only its own exact origin and configured asset paths; unexpected external/unknown requests are refused and counted. It supplies private home/config/cache/default installation directories and refreshes catalogue timestamps for each child command. Its own command wrapper clears inherited environment and invokes Cargo's exact built binary.

This first harness tests genuine install/default/repeat/clean and real counter compilation with `--skip-zk`. Existing compiler scenarios retain real key-generation and formatter assertions when root wires their adapters; those are not claimed by this initial smoke. Historical installer/self-update is separate. Local execution evidence is ARM macOS only; other platform metadata is pinned without an execution claim.

## Existing scenario adapters (ADR229)

`test_update` and the four `test_scenarios*` binaries retain their exact arguments,
expected output, platform exclusions, real compiler/key-generation and formatter
assertions. One `ArchiveFixture` lives for the complete scenario; the common
`run_archive_command` wrappers refresh its private catalogue for each call and
merge explicit caller variables afterward, preserving custom `COMPACT_DIRECTORY`.
They use the existing command helpers, retaining an explicitly supplied
`MIDNIGHT_PP`. Pure update help/version/argument errors retain their original
read-only presentation. `test_self_scenarios` is a separate acceptance boundary.

Before running these scenarios, prepare the small, version-specific counter
parameter set **outside** the proxy-isolated tests:

```sh
python3 tools/compact/tests/fixtures/acquire_parameters.py \
  --platform auto --archive-cache "$COMPACT_TEST_ARCHIVE_CACHE" \
  --cache "$RUNNER_TEMP/compact-counter-parameters" \
  --receipt "$RUNNER_TEMP/compact-counter-parameter-provenance.json"
export MIDNIGHT_PP="$RUNNER_TEMP/compact-counter-parameters"
```

The helper verifies archive bytes before inspecting their `zkir` binary and
checks that each managed parameter digest is embedded in that exact tool.
It acquires only historical Filecoin k=10 (0.24) and Midnight k=5 (0.30/0.31).
Compiler 0.22 predates this provider: its unchanged wrapper exports archive-local
`ZKIR_PP`, selecting bundled `kzg`/`kzg.vp`. Setup verifies that wrapper binding
and records those bundled file hashes from the pinned archive; no Filecoin k=9
file is acquired or substituted.
Hashes come from pinned upstream `base-crypto/src/data_provider.rs` source;
receipts identify source commits, archive/tool digests and parameter bytes.
ARM probes measured k=10/29 rows for 0.24 and k=5/24 rows for 0.30/0.31.
The existing 0.22 output assertion specifies k=9/49 rows; this remains a
non-ARM execution requirement, not a local ARM success claim.

`--seed-dir` can reuse an explicitly selected existing public-parameter cache
without changing it. `--verify-only` refuses missing/corrupt files without
network. Files are atomically published read-only after size/SHA256 checks.
No whole parameter set is fetched. The two families are never substituted.
Unexpected test-time HTTP requests remain refused and counted; do not use
`--skip-zk` to work around missing setup. Proxy assertions cover the cooperating
child HTTP stack, not an operating-system-wide network sandbox.

The workflow runs Python guards and acquisition even on a cache hit, exports
absolute archive/parameter paths, and retains receipts. Cold network failures
are explicit setup failures. Metadata/caching supports the configured Linux and
macOS matrix; local ARM execution does not attest to every platform. The cohort
contains 53 declared tests (49 ARM macOS, 44 Intel macOS, 51 Linux under existing
cfgs), plus the shared archive transport unit test in each binary.
