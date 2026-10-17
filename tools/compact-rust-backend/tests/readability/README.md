# Generated Rust readability sample (#491)

Current local result: see [cumulative policy and qualification](#cumulative-local-result-and-policy-491).
Earlier sections retain each delivered stage as history; they do not describe
the current remaining backlog. Remote publication and issue closure are separate.

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

## Follow-up: delegated collection and tree mutations (ADR0531)

The same checked-Unit rule now applies to Map, List and Merkle action renderers.
Their mutations keep the original fallible slot call and frame binding, without
appending a literal Unit temporary. Query payloads and all validation rules stay
unchanged. This removes four redundant emitter statements, without adding an
optimizer, helper abstraction or runtime API.

[after-collections.json](after-collections.json) records the new measurement;
both earlier reports remain frozen.

| Metric, same ten files | Original | First slice | Collection follow-up |
| --- | ---: | ---: | ---: |
| Source lines | 3,782 | 3,735 | 3,724 |
| Literal Unit bindings | 58 | 11 | 0 |
| Internal identifier occurrences | 519 | 445 | 434 |
| Clone calls | 78 | 78 | 78 |

All ten function-signature fingerprints still match. Parameter, call and branch
aliases remain outside this rule. List mutations are tested outside the finite
ten-file survey; the sample was not changed to improve its measurements.

Fresh regeneration checks all 255 outputs; 37 change solely by removing 123
literal Unit declarations. Independent comparison confirms unchanged remaining
AST tokens. Five reviewed current-Rust binding successors, four Python manifest
pins and one dependent DID matrix pointer follow those generated libraries.
Original TS captures, case rows, captured identities and verifier logic remain
unchanged; this does not expand any recorded coverage claim.

Follow-up local checks: 708 effect-plan tests pass (three existing ignores),
169 tests across 30 affected/runtime fixture packages pass, and the twelve Python
capture mutation/refusal controls pass. Fresh uncached Map (four scenarios),
Field List and Merkle reset proof/verification/application checks pass, including
private corrupt-verifier refusal and unchanged original artifact hashes. Map and
List use existing strict funded application checks; Merkle reset uses the shared
unbalanced smoke policy. These checks do not certify every mutation variant or
constructor deployment proof. Backend Clippy with warnings denied, formatting
and source-package inventory checks pass. The broader #491 work remains open.


## Cumulative local result and policy (#491)

[after-final.json](after-final.json) measures the **same original ten-file corpus**
at emitter revision `a2d56251ca30341049e3ca2fa6ec505e19b7874f`. The report's
`baseline_revision` identifies the original comparison baseline, not the current
emitter. Earlier reports and corpus identities remain immutable.

| Metric, same ten files | Original | Current |
| --- | ---: | ---: |
| Source lines | 3,782 | 3,721 |
| Internal identifier occurrences | 519 | 423 |
| Literal Unit bindings | 58 | 0 |
| Clone calls | 78 | 78 |
| Collect calls | 0 | 0 |

No per-file measured count increases; all ten declared-function-signature hashes
match. This is 61 fewer lines (1.6%) and 96 fewer internal-name occurrences
(18.5%) in the fixed sample. It is not a claim about all generated code.

Later focused passes carry only the recording frame through checked Unit Call/If
control flow (ADR0534), use runtime `FixedVector::try_map` for pure Map (ADR0535),
use standard iterator `try_fold` for pure Fold (ADR0536), and omit checked Unit
root/helper aliases (ADR0537). The pure Map addition intentionally advances the
runtime ABI to 53 with an additive runtime API; it does not change existing
Compact/public generated signatures. Many of those fixtures are outside this
sample. Their individual raw output audits, affected consumer executions and
fresh proof/refusal results are retained separately in the midnight vault.

### Repeatable comparison

The existing survey accepts an optional same-corpus report:

```sh
cargo run --locked --offline -p compact-rust-backend --example readability -- \
  . tools/compact-rust-backend/tests/readability/corpus.json \
  tools/compact-rust-backend/tests/readability/baseline.json > /absolute/evidence/current.json
```

It prints the current measurements and a comparison, returning nonzero for any
increased per-file count or changed declared-function signature. Changed corpus
identities, missing/duplicate/replaced cases, malformed reports and missing typed
metrics refuse comparison. Counts describe syntax, not redundancy or semantics.
An intentional regression requires a documented tradeoff and separate review;
there is no silent allowlist or automatic baseline rewrite. The survey does not
replace behavior, fixture, public API, proof or ledger checks.

### Naming contract

Public Compact names use the existing checked Rust identifier mapping. Parameters
prefer normalized source names, with deterministic indexed fallbacks for reserved
names and collisions. Recorded helper names use the source circuit identifier;
colliding helper names use a sorted declaration index and raw identifier bytes.
Checked-plan temporaries use deterministic SSA ordinals in their isolated emitted
namespace, rather than inventing source names that the checked model does not
retain. Adding declarations or changing IR structure may change these ordinals.

`src/naming/tests.rs` checks reserved/raw/`$` collisions, helper allocation under
map insertion-order changes, repeated actual rendering and source-location
changes. Apart from the declared identifiers described above, source paths, arbitrary
source text and witness values are not naming inputs.
This is a naming guarantee, **not** a general promise that intentionally emitted
literals or public source identifiers cannot contain sensitive content. Public
identifier normalization collisions continue to refuse instead of aliasing.

### Change and qualification policy

Use syn construction and prettyplease formatting. Select a measured opportunity,
state its checked type/ownership boundary in an ADR, preserve every effect and
failure point, and test syntax structure plus actual execution. Regenerate
fixtures and independently audit the changed declarations and remaining AST;
function-signature hashes alone are not a complete public API check. Run affected
native/replay/gas and proof/application/refusal paths as appropriate. Keep the
original authorities and assertions, and label finite coverage precisely.

Current scoped checks include backend library 1,718 passes (seven existing
ignores), render integration 154 passes and all 255 generated outputs checked
(one updated for ADR0537). The affected public-products consumer's four tests
include native/recorded/prepared `empty`; the two fresh original proof selectors
exercise same-crate public-products and Unit-composition, **not** a proof of that
`empty` export. Both private corrupt-verifier controls refuse. Later naming and
survey checks do not change production emission; they reuse this exact-source
behavior evidence. Formatting, scoped Clippy and header regression checks pass.
These populations overlap and are not language-coverage totals.

## Scoped generated-consumer build measurement

A private Map<Boolean,Field> consumer compares original `ffe3010d` emission with
current `a2d56251c`, against the same current runtime/toolchain/profile. The old
copy adapts **only two ABI assertions from 52 to 53**, retaining the original file
and exact patch separately. This is a compatible-emission benchmark, not a claim
that an old ABI consumer builds unmodified or that different runtimes are equal.

Source bytes fall 30,923 to 30,348; lines fall 722 to 711. Each trial cleans only
the private consumer, then runs `cargo check --locked --offline -vv` under
`/usr/bin/time -l`, with Rust 1.99.0, incremental compilation disabled, six jobs,
dev debug symbols disabled and unchanged warm dependencies. All 233 dependencies
report Fresh. Setup is outside trial timing; proofs/tests are not cached.

| Order | Emission | Complete check wall seconds | Command peak RSS bytes |
| --- | --- | ---: | ---: |
| 1 | Original, ABI53 assertion | 0.415664 | 122,159,104 |
| 2 | Current | 0.288461 | 122,175,488 |
| 3 | Current | 0.482927 | 122,191,872 |
| 4 | Original, ABI53 assertion | 0.321325 | 122,257,408 |

The direction reverses between pairs: **no repeatable compilation improvement or
regression is established**. RSS is the measured command's process maximum, not
whole-machine/aggregate memory. This single small consumer, warm-dependency
`cargo check` experiment says nothing about linking, clean dependencies, proofs,
hosted CI or overall build speed. Exact source hashes, adapted bytes, commands,
lockfile and raw logs are retained in the midnight vault's #491 closure evidence.

The finite issue scope is locally complete after independent review and the
consolidated ADR0538; remote publication/issue closure is separate. Future
ownership/liveness optimization, universal source naming and module-layout
redesign require their own measured proposals and are not implied by this result.
