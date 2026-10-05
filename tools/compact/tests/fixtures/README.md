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
