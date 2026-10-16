# Generated Rust readability sample (#491)

Baseline: merged AST `ffe3010ddd08e13b20b48116f3aabc0c2ae6865f`, Rust1.99,
locked syntax dependencies. [corpus.json](corpus.json) selects ten retained Rust
bodies across pure, stateful, witness, collection, Merkle, shielded, DID and Jubjub
cases. The shielded build wrapper is deliberately replaced by its retained
generated snapshot; the survey does not expand macros.

## Reproduce

```sh
cargo run --locked --offline -p compact-rust-backend --example readability -- \
  . tools/compact-rust-backend/tests/readability/corpus.json > /absolute/evidence/readability.json
cargo test --locked --offline -p compact-rust-backend --example readability
```

Run from the repository root. The tool only reads inputs and prints JSON. Missing
files, invalid Rust, duplicate corpus entries or invalid corpus metadata fail.
The example and corpus ship in the backend source package; no dependency or
runtime change is required.

## Before any emitter cleanup

[baseline.json](baseline.json) is an immutable measurement of the original
retained source bytes, with per-file hashes and the exact corpus hash. Later
fixture changes must not overwrite this baseline; record a separate after report.

| Case | Lines | Literal Unit bindings | Internal identifier occurrences |
| --- | ---: | ---: | ---: |
| field-add | 30 | 0 | 0 |
| bounded-uint-oracle | 208 | 2 | 11 |
| let-return-oracle | 185 | 1 | 17 |
| witness-cell-write | 628 | 12 | 99 |
| map-boolean-field | 722 | 11 | 84 |
| nested-map-oracle | 164 | 2 | 7 |
| merkle-path-verify | 519 | 4 | 58 |
| shielded-receive-oracle | 437 | 8 | 92 |
| did-point-map-reducer | 496 | 14 | 127 |
| jubjub-cell | 393 | 4 | 24 |
| Total | 3782 | 58 | 519 |

The first proposed slice removes synthetic literal Unit payload bindings only
after checked recording operations. Review before/after ASTs and preserve actual
effects, signatures, failure ordering, runtime/ABI and checked capability decisions.

## Interpretation and limits

- Line counts include headers/docs and use the retained formatting. Block depth
  counts syntax blocks, not semantic complexity or runtime stack depth.
- Internal identifiers start with `__compact_`; distinct counts are per file and
  include lexical reuse. Occurrences count parsed identifiers, not comments or
  strings. Identifiers hidden inside macro tokens are outside the survey.
- Clone, collect and single-expression-block counts are inventories, not proof
  that those constructs can safely be removed. No clone/ownership optimization
  is implied by this baseline.
- Function-signature hashes include private functions, methods and parameter
  names. They do not fingerprint every public type, field, trait expansion or
  associated item; targeted structural/consumer checks remain necessary.
- Ten cases are a finite readability sample, not language coverage, proof counts
  or an accepted whole-milestone scope. No source/build-time speedup is claimed.
