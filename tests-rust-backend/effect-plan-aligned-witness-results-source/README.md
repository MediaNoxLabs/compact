# Aligned witness result composition

This genuine default-ZKIR-v2 source qualifies four result domains: owned strings,
Entry (string + Jubjub point), Nested (two string/point tuples), and a standalone
11-member Bytes32 tuple. The frozen full IR retains the tuple as Tuple11.

The five generated APIs are `text`, `entry`, `nested`, `tuple11` and `unit_text`.
The first four take `(fallback, selected)` and return fixed named products.
Every call increments Counter before selected witnesses and then invokes permit.
`unit_text` exercises a local Unit helper; it was already recorded through the
legacy Unit selector at signed #583. The four value roots previously failed
recording at the witness-result policy. The build asserts all five APIs become
recorded and observed with the qualified predicate.

Three runtime tests bind 15 unique same-source TS scenarios: 12 execution successes
and 3 provider/guard refusals. They compare native, recorded, observed and manual
state/witness execution; exact private FAB values and alignments; callback ledger
reads/private-state order; execution gas and independent manual replay gas. Expected
hash answers and point coordinates come from the frozen official TS capture.

Nine successful calls prepare. Three trailing-NUL witness outputs execute exactly
but preparation returns `NonCanonicalPrivateOutput`: text/Entry at output0 atom0,
nested at output0 atom3. Official TS serialization independently refuses these
three cases. No public opaque-input widening is involved: all fallbacks are
canonical. Fixed Bytes32 values ending in zero remain valid and prepare.

The TS oracle also retains empty/ASCII/Unicode/interior-NUL strings, point identity
and nonidentity, fixed tuple boundary bytes, lazy suppression of a failing provider,
early provider error, later provider error and later false assertion. Each callback
is evaluated once; the original observed Counter stays zero.

The source counterpart rejects PersistentHash on opaque String/Bytes/Entry/Nested;
those four frontend-negative receipts are research evidence for the containing
slice. This package tests persistent hashing only for the fixed tuple. Keccak,
selector retirement and persistent-opaque support are outside this fixture.

Run locally with:

```sh
cargo test --offline --locked -p compact-rust-effect-plan-aligned-witness-results-source-fixture --all-features
```

To refresh the independent TS capture, regenerate both targets from the unchanged
source with the pinned frontend, then pass its contract output and prepared official
TS runtime to `node oracle/capture.mjs GENERATED_CONTRACT_DIR OFFICIAL_TS_RUNTIME_DIR`.
The capture hashes source/full IR/generated JS, script, runtime lock/dist and WASM.
The captured 15 rows are execution/preimage evidence, not proof-generation claims.
The selected strict proof has its own source-bound adapter and receipts.
