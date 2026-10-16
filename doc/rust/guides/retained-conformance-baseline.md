# Retained conformance baseline inputs

The original repository-baseline law checks the source-bound baseline from
`6e37443b4fc2f5363954e0aef88373a48f4d3ae8`. Its source hashes deliberately refuse
the current recording-effect implementation. That refusal must not become a
current conformance success by updating the historical baseline.

The test caller selects the checked-in immutable 150-file archive by default for
this **one historical case**; an explicit environment override selects another archive. The existing
checker authenticates its source hashes, mappings, reference anchors and formal
artifact inventory. The baseline JSON and original test method/assertions remain
unchanged. The fixture selects that archive only for the original case, then
restores the current source root and checks the archive again. The other 29
conformance tests still execute their current code and live synthetic controls.

For example, when the original Git commit is available locally:

```sh
BASELINE_ARCHIVE_ROOT=$(mktemp -d)
git archive 6e37443b4fc2f5363954e0aef88373a48f4d3ae8 |
  tar -x -C "$BASELINE_ARCHIVE_ROOT"
COMPACT_CONFORMANCE_BASELINE_ROOT="$BASELINE_ARCHIVE_ROOT" \
  python3 -B -m unittest discover -s tools/compact-rust-backend \
  -p test_specification_conformance.py
```

The existing authenticated replay kit's 150-file `archive/` is also sufficient.
It retains exact required bytes from that public provider commit; it does not
need the larger historical runner or its other source projections. An explicitly selected original commit or kit must actually be supplied: no
download or current-root fallback is implicit. When no override is set, the
checked-in bundle supplies the same authenticated public bytes. Do not edit an archive while a test is using it.

The ordinary full gate also selects the bundle by default; an explicit archive
override remains available:

```sh
python3 -B tools/compact-rust-backend/local_parity_gate.py --full \
  --conformance-baseline-root "$BASELINE_ARCHIVE_ROOT" \
  --guarded-point-artifacts /absolute/path/to/retained-graph \
  --compiler /absolute/path/to/compactc --scheme /absolute/path/to/compactc-scheme
```

Existing tool, fee-fixture and proof prerequisites still apply. Archive preflight
happens before tool lookup or compiler/build execution. Missing, changed or wrong
inputs fail; they are never replaced with current files or a cached pass. The
receipt records the selected historical root and immutable baseline identity.

This keeps the old source-bound law executable. It establishes neither current
language coverage nor semantic equivalence, and it does not run or replace the
separate complete 974-case historical qualification. Standalone execution of the original baseline case uses the bundled archive when
`COMPACT_CONFORMANCE_BASELINE_ROOT` is unset. An explicit invalid override still
fails without fallback. The authenticator itself retains its strict explicit-root
contract.
