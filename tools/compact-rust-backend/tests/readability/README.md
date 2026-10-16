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


## First local improvement: checked Unit payloads

The first slice renders a checked recording-plan Unit reference as `()` and omits
a leaf's synthetic binding only when its expression is exactly that literal.
Producer operations, checked-plan validation, failure paths and nonliteral Unit
expressions remain. This is a rendering rule, not a general dead-code optimizer.

```rust
// Before: the write is followed by a synthetic payload.
let frame = ledger_slots::stored.record_write(frame, next)?;
let __compact_effect_2: () = ();
Ok(frame.finish(__compact_effect_2))

// After: the write and its failure point are identical.
let frame = ledger_slots::stored.record_write(frame, next)?;
Ok(frame.finish(()))
```

[after.json](after.json) measures the same corpus after fresh current-emitter
regeneration, without overwriting the original baseline.

| Metric, ten files | Before | After |
| --- | ---: | ---: |
| Source lines, including headers/docs | 3,782 | 3,735 |
| Literal Unit bindings | 58 | 11 |
| Internal identifier occurrences | 519 | 445 |
| Clone calls | 78 | 78 |
| Collect calls | 0 | 0 |

All ten declared-function-signature fingerprints match. No measured per-file
metric increases. Eleven literal Unit bindings remain in delegated map/tree
renderers; parameter/call/branch aliases are also outside this first slice.
Source lines fell 1.2% and internal-name occurrences
14.3% in this finite sample; no build-time or runtime speedup is established.
`baseline_revision` identifies the corpus's original anchor, not an assertion
that after-source bytes belong to that revision; each row hashes the measured bytes.

The common rule intentionally changes 186 of 255 regenerated golden outputs.
They must be reviewed with the emitter change, not treated as new behavioral
oracles merely because they were regenerated. A separate lexical-scope-aware
AST audit matched all 186 outputs after the intended Unit substitution/omission,
including unchanged declarations, fields, signatures and remaining operation
syntax. This finite structural check does not prove formal equivalence.

### Reviewed Rust successors versus original TS evidence

Eleven existing `reviewed-rust-binding.json` manifests now identify the newly
generated Rust `lib.rs` bytes, explicitly labeled a #491 current-emission
successor. The original TS captures, semantic rows, captured hashes, provenance
tests, support verifier and original provenance-transition manifest are unchanged.
The existing nine Python manifest pins and one DID matrix artifact pointer follow
those reviewed current manifests; no verification rule or registry scope expands.
Old identities remain in the frozen baseline commit. This does not relabel prior
TS/native/proof acceptance as a fresh capture at the new emission epoch.

ADR0529 and step-by-step qualification are maintained in the midnight Obsidian
vault under the Rust initiative. Their repository publication remains part of
milestone closeout. #491 remains open for further measured readability slices.

## Local qualification of the first slice

- Five new structural renderer tests preserve nonliteral Unit initialization,
  assertions, fallible witness execution and non-Unit payload references.
- Backend library: 1,700 passed in the broad run; its sole failure was the old
  expectation that an empty Unit plan emits a temporary. The corrected test
  requires an empty successful plan and a literal Unit result and passes on its
  targeted rerun. Seven existing tests remain ignored. These are composed
  results, not a fresh all-green monolithic run.
- Render integration: 154 passed. Fourteen selected runtime fixture packages:
  44 passed, including lazy branches, failure ordering, witnesses and replay/gas.
- All eleven current-binding provenance fixtures: 13 passed. The existing
  Python capture verifier's twelve positive/mutation/refusal controls pass.
  Populations overlap and must not be added as unique coverage counts.
- Fresh uncached keys, actual proof verification and ledger application pass for
  Counter's two branches, Let-return and three Jubjub cases. Private corrupt
  verifier copies are rejected; original generated artifacts remain unchanged.
  These use the existing smoke policy, not a funded/live-network acceptance claim.
- The three measurement-tool tests, all-target/all-feature backend Clippy with
  warnings denied, formatting and source-package inventory checks pass.

Qualification used the locked ledger 8.1.3/ZKIR 2.1.1 graph and the unchanged
Scheme frontend. It does not reuse cached proofs or pass decisions, claim full
historical qualification, or establish formal equivalence.
