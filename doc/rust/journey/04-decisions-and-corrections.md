# Decisions, mistakes and corrections

The useful history is not a straight line. These are representative moments
where the evidence changed the architecture or narrowed a claim.

## 1. Preserve the old backend instead of deleting it

**Problem.** A clean rewrite could regress behavior while making comparison
impossible.

**Decision.** Keep `codegen-rust` as an oracle and compare exact sources,
results, state, transcripts and refusals.

**Evidence.** The first Rust-AST milestone ran all 37 exact top-level oracle
sources through the new path.

**Benefit.** The project could change the architecture without rewriting its
own history.

**Limit.** An oracle can contain defects; agreement is evidence, not truth.

## 2. Use a closed IR, not a Rust-source escape hatch

**Problem.** Text fragments let unsupported source shapes leak into output and
made exhaustive review difficult.

**Decision.** Model Compact types, expressions, actions and returns as strict
serde variants and reject unknown input.

**Evidence.** Compiler-backed refusal probes, fixture regeneration and external
Cargo consumers accompanied schema changes.

**Benefit.** New constructs require an explicit compiler/backend decision.

**Limit.** The private schema still couples the Chez producer and Rust consumer;
its version is not a public language specification.

## 3. Generated syntax is necessary, not sufficient

**Problem.** `syn` guarantees parseable Rust construction, but semantic bugs
can still live inside valid syntax.

**Decision.** Keep AST construction and add execution, independent expected
values, TypeScript observations, negative controls, replay and proof gates.

**Evidence.** Field modular arithmetic, Boolean witness order, source-defined
returns and output-recovery tests caught stale or overly textual assumptions.

**Benefit.** Generated code is checked as a program, not a string.

**Limit.** Hundreds of substring assertions remain useful smoke tests but are
not semantic proof.

## 4. The point type was too “safe”

**Problem.** Rust represented Compact `JubjubPoint` as a validated subgroup
point. Compact's source value is a coordinate pair, and the upstream constructor
could panic on some invalid/subgroup-external coordinates.

**Decision.** Separate source representation from checked operation admission;
validate only where an operation requires a real point.

**Evidence.** Cross-language parity tests reproduced the divergent values and
panic boundary.

**Benefit.** Rust no longer changes source representation under the banner of
type safety.

**Limit.** A checked backend may still deliberately refuse behavior that the
reference path accepts; that difference must be documented, not hidden.

## 5. Shape recognizers reached their ceiling

**Problem.** Every useful application family added another whole-program
admission profile. Supported operations did not compose automatically.

**Decision.** Introduce a typed `RecordedPlan`, migrate one family at a time,
and retain the old paths as differential oracles until retirement evidence
exists.

**Evidence.** Per-family default/feature tests, generated-source inventories,
source-bound proof/application cases and an explicit retirement ledger.

**Benefit.** The unit of reuse becomes an operation plus its invariants.

**Limit.** A compositional IR can centralize bugs too; exhaustive variants,
resource limits and old-first laws remain necessary.

## 6. Full Passport ACC adoption was the wrong milestone goal

**Problem.** A large ledger9-era application risked turning unrelated foreign
curve, WebAuthn and newer-ledger requirements into alleged ledger-8 support.

**Decision.** Inventory constructs, retain generic Jubjub work, remove full ACC
adoption and keep bounded passport/DID cohorts separately evidenced.

**Evidence.** The 0.3 closure records the removed outcome and the exact retained
application cases.

**Benefit.** Useful compiler work survived without a misleading product claim.

**Limit.** The bounded passport cohort is not the full ACC application.

## 7. A faster test gate may not skip authentication

**Problem.** Historical validation took about 171 minutes, more than 99% in
historical checks. Naive memoization would have hidden live mutation.

**Decision.** Reject path/result caches and shard whole modules into isolated
processes, preserving fixture boundaries and independent temporary state.

**Evidence.** All 974 IDs and assertions remained; corruption and source
mutation controls passed; measured wall time fell to about 55 minutes.

**Benefit.** Faster feedback without turning a historical assurance suite into
a snapshot replay.

**Limit.** The result is machine/source specific and still needs the full gate
at final freeze.

## 8. Source provenance must be honest

**Problem.** Hand-built IR can make a backend look more source-compatible than
the frontend actually is, while absolute paths and private artifacts are unsafe
to publish.

**Decision.** Authenticate source, IR and artifact sidecars; distinguish
original occurrences, owned snapshots and missing origins; carry safe logical
source identity and structural paths.

**Evidence.** Authenticated input views, original-source guards and publication
manifests preserve exact hashes and failure cases.

**Benefit.** A passing fixture can be tied back to a real Compact source.

**Limit.** Nested expression ranges and transient-value provenance are not
claimed implemented at this cutoff.

## Independent review boundary

The project used independent coding-agent reviews for architecture, dependency,
correctness and final-slice checks. They found actionable issues and provided a
useful second model. They are not a human security audit, formal verification
or production certification. The story uses the exact phrase “independent
coding-agent review” for that reason.
