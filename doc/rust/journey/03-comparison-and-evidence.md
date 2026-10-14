# Comparison and evidence report

## Four generations on consistent dimensions

| Dimension | TypeScript / JS path | Original Rust backend | Rust-AST 0.3 | Effect prototype at cutoff |
| --- | --- | --- | --- | --- |
| Frontend boundary | Established TS preparation/runtime path | Reuses TypeScript-oriented IR | Starts before TS-specific lowering and emits private typed IR | Same private IR; adds shared checked semantic folds |
| Rust construction | Not applicable | Scheme emits Rust text | `syn` + `quote` + `proc-macro2`, formatted by `prettyplease` | Same renderer, fed by checked effects/facts |
| Runtime | JS/WASM providers and ledger APIs | Native runtime with generated VM-oriented code | Typed runtime over official ledger-8 primitives | Same authority; clearer execution/result roles |
| Recording admission | Established TS semantics, different surface | Shape/walker-driven Rust path | Typed gaps plus many specialized recording profiles | Compositional effect plan; old selectors retained as test oracles |
| Source diagnostics | Native frontend/source-map path | Coupled to compiler emission | Declaration locations + structural paths | Authenticated origins improved; nested ranges still a stated limit at cutoff |
| Proof evidence | Existing reference ecosystem | Bounded historical cases | Selected exact-source prove/verify/application receipts | Per-family receipts; final frozen EA13 matrix pending |
| Product maturity | Established integration | Useful oracle/prototype branch | Closed scoped milestones and generated consumer story | Local prototype, not pushed/merged/released |

## Evidence is a ladder, not a number

| Evidence level | What it establishes | Typical artifact | Does not establish |
| --- | --- | --- | --- |
| Source inventory | Known inputs, exports and feature classification | Source/capability census | Behavior or proof |
| Compiler acceptance | Source reaches a checked target or is deliberately refused | Frontend + IR fixtures | Correct runtime result |
| Generated compilation | Public Rust surface type-checks externally | Isolated Cargo consumer | Semantic parity |
| Native behavior | Results, errors, state and witness order for named cases | Rust/TS captured scenarios | Recording/proof availability |
| Recording/replay | Ordered effects and replay invariants | Recorded traces and replay checks | Proof applicability |
| Prove/verify | Exact artifacts produce and verify a proof | Source/key-bound receipt | Ledger application or finality |
| Ledger application | Verified transaction applies under the pinned policy | Offline or local ledger receipt | Production network adoption |
| Live observation | A named node/indexer/wallet observed a transaction | Historical network receipt | Universal finality or current release status |

## Milestone evidence snapshots

### Milestone 1 — local native baseline

- Implementation cutoff: `ea4cc006`.
- 128 generated sources current; 410 workspace test/doc-test groups passed.
- All 37 top-level oracle sources ran through the new path, plus selected
  `tiny`, election, Zerocash and passport flows.
- No first-class proof/deployment pipeline; the wrapper forced `--skip-zk`.

### Milestone 2 — usable target and remote closure

- Closed at `03c03a39` on 2026-10-06.
- 386/386 source availability.
- Sampled 37-source / 203-row behavior.
- Selected original four Coracle plus seven microDAO proof selectors, with
  three explicitly balancing-disabled smoke selectors.
- Ten required remote workflows passed at the final revision.
- No crates.io release, Windows certification or universal parity claim.

### Milestone 0.3 — qualification and adoption

- Qualified implementation `f9a49666`; closure `0d7610b2`.
- 19/19 required outcomes; 93 criteria; 121 milestone ADRs.
- 198 roots, 232 inputs and 1,055 functions classified.
- Runtime changed-line coverage 97.87%; mapped total 88.03%; testkit 97.62%;
  backend and macro cohorts retained separately at 95.11% and 100%.
- DID v0.7.0: 12 stateful exports; 42 finite relation calls.
- Passport cohort: 17 files, 75 pure exports, 202 behavior cases and four
  codec vectors. Full Passport ACC adoption was removed.
- Remote core: 742 cases in 63 result blocks at the named revision.
- RustSec: zero vulnerability matches, with four workspace / three standalone
  notices and strict nonzero exit retained.

Counts overlap and must not be summed into one “tests passed” figure.

### Effect prototype — active local program

At `f3cef799`, the saved story cutoff includes the historical-test process
sharding change. The full installed validation kept 974 unique planned and
executed IDs with no failures, errors, skips or unrun cases. Four independent
process shards reduced the measured wall time from 10,270 seconds (about
171 minutes) to 3,308 seconds (about 55 minutes), a 3.105× result on this
frozen machine and source.

What remained unchanged matters more than the headline:

- the original 940 cases and their assertions remained;
- workers reconstructed independent temporary state;
- live source/manifest mutation checks still ran;
- original case IDs and accounting remained exact;
- eight new corruption/source controls lived outside frozen discovery;
- no validation result cache or live path cache was introduced.

It is a measured local optimization, not a universal CI speed claim.

## Parity findings that changed the design

The TypeScript comparison was not a ceremonial check. It found genuine Rust
divergences, including incorrect hash-to-curve representation, the wrong
Jubjub Schnorr predicate, permissive Boolean/bounded-unsigned decoding, and a
validated-point alias that could panic for attacker-controlled coordinates.

Rust also found genuine source/runtime defects: malformed native widths that
JS silently sliced, and Merkle update underflow before a capacity check. The
right conclusion is not “Rust won.” Independent implementations made hidden
assumptions observable in both directions.

## Current limitations

- No formal proof of complete Compact semantics or universal TS parity.
- Proof support is per applicable export and exact artifact set.
- The effect prototype is local and not yet EA13-closed.
- Source locations at the cutoff are owner locations plus structural paths,
  not complete nested ranges.
- Aggregate decoded heap/object/CPU containment remains separate work.
- Experimental WASM does not imply a production browser prover.
- No Windows certification, package registry release or upstream adoption is
  implied by the closed local/fork milestones.
