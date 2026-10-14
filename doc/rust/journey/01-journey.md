# The journey: four eras of one Rust target

## Era 0 — ledger-8 as the fixed ground

The work began from the plain `ledger-8` branch at
[`eb72a5ab`](https://github.com/LFDT-Minokawa/compact/commit/eb72a5ab6085bf9b8564cc80c98c973e04825c2d). That choice was
less glamorous than jumping to newer protocol lines, but technically useful:
the Compact compiler, ledger VM, ZK libraries and TypeScript runtime could be
pinned as one coherent family. It gave comparisons a stable meaning.

The target was never “replace Midnight semantics with Rust semantics.” The
runtime reused official ledger-8 field, FAB, serialization, state, VM, curve
and proof primitives wherever they supplied the authority. Rust code organized
their use and made ownership visible; it did not become a second ledger spec.

## Era 1 — the original Rust backend as an oracle

The original `codegen-rust` branch proved that Compact could generate useful
Rust. Its Scheme passes read the TypeScript-oriented compiler representation
and emitted Rust text. At the first architecture comparison
([`589c92ef`](https://github.com/MediaNoxLabs/compact/commit/589c92ef961ce89ac730140e81b76d4c08222a70)), it
offered something the replacement did not yet have: a mature generated-crate
experience and a large corpus of examples.

That made it invaluable as an oracle. It also revealed the maintenance cost of
text-oriented generation:

- target semantics, Rust formatting and string assembly met in the same pass;
- a new source construct could touch many walkers and output fragments;
- correctness checks often inspected generated text after the fact;
- runtime ownership and generated-code ownership were harder to distinguish.

The lesson was not that Scheme was the problem. Scheme remains the Compact
frontend in the selected architecture. The problem was letting source analysis,
target admission and Rust surface syntax collapse into one textual operation.

## Era 2 — Rust-AST and a closed compiler boundary

Milestone 1 separated those concerns. After shared Compact analysis, Chez
Scheme lowered supported nodes into a closed, versioned JSON model. A Rust
backend deserialized and validated that model, built syntax with `syn`,
`quote`, `proc-macro2` and `prettyplease`, and delegated execution primitives
to a native runtime.

The first broad local baseline closed at
[`ea4cc006`](https://github.com/MediaNoxLabs/compact/commit/ea4cc006f973632c4c7b1fb4569af2b4c93ff38a);
the branch was published at
[`a5ee30f1`](https://github.com/MediaNoxLabs/compact/commit/a5ee30f171322f07c06e3846117f20c9e075ef95)
on 2026-10-02. Its evidence included 128 generated sources and
410 workspace test/doc-test groups. Those counts demonstrated the named gate,
not whole-language parity.

Milestone 2 turned the experiment into a usable compiler target. By the
2026-10-06 close at
[`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c),
`compactc --target rust` emitted a Cargo crate,
matching runtime sources, ZKIR/proving artifacts and a hashed manifest. The
accepted scope separated:

- 386/386 source availability;
- sampled behavior across 37 sources and 203 rows;
- selected original proof/application cases;
- historical local wallet observations;
- platform and installer validation.

It also preserved the limits: no new release tag, no crates.io publication,
no Windows certification and no claim that every generated API was proved.

## Era 3 — 0.3: qualification, consumers and real applications

Milestone 0.3.0 was less about inventing a new compiler shape and more about
qualifying the system people would actually touch. It closed on 2026-10-09
with qualified implementation
[`f9a49666`](https://github.com/MediaNoxLabs/compact/commit/f9a496660a4a69f4dd7c7927111767ad5c0c12b2)
and closure
[`0d7610b2`](https://github.com/MediaNoxLabs/compact/commit/0d7610b2792d60fee5d3713d4eb0ede3c98cc82a).

The work added or consolidated:

- task-oriented developer guides and external consumer checks;
- runtime, backend, macros and testkit coverage with separate denominators;
- DID v0.7.0 adoption: 12 stateful exports and 42 finite relation calls;
- a bounded digital-passport cohort: 17 files, 75 pure exports, 202 behavior
  cases and four codec vectors;
- proof/application receipts for applicable DID, reducer and retained Jubjub
  cases;
- compatibility, MSRV, installer, packaging and source-provenance controls;
- coding-agent architecture/security review and explicit residual issues.

The full Passport ACC experiment was removed from milestone scope after a
construct inventory found no unique language mechanism worth adopting as one
large application. The useful generic Jubjub work stayed. Foreign-curve and
ledger9/10 requirements did not become ledger-8 success stories by omission.

The 0.3 close is substantial, but bounded: 198 source roots, 232 inputs and
1,055 functions are an inventory; they are not 1,055 formal proofs. Line
coverage is not branch coverage. Coding-agent review is not human security
certification. Experimental WASM and manually assembled portable packages are
not production browser or release qualification.

## Era 4 — the recording-profile ceiling

The AST backend had a strong typed boundary, yet one subsystem kept growing by
accretion. Recording answers a stricter question than native execution:

> Can this circuit be turned into an ordered, replayable, proof-bindable ledger
> program with the exact private/public values required by the runtime?

Early support sensibly recognized known-good shapes. As coverage expanded,
specialized profiles accumulated: guarded writes, voting, shielded receive,
funded mint, phase reset, payouts and application-specific combinations. Each
was individually useful. Together they created a ceiling: already supported
operations did not automatically compose in a new arrangement.

The 0.4 work therefore changed the unit of reasoning from “which contract
shape is this?” to “which checked effects occur, in what order, under which
types, scopes, resource limits and proof policies?”

## Era 5 — checked effect composition

The current prototype introduces a `RecordedPlan`: a typed intermediate form
for values, reads, writes, assertions, witness calls, helper calls,
conditionals, lexical bindings, returns and Zswap-related operations. Admission
and validation happen before Rust syntax emission. The renderer consumes a
checked plan instead of inferring support while printing code.

Migration was deliberately staged. Existing recognized families remained an
oracle while one family at a time moved to composition. The history records
real corrections along the way: source binder identity, inactive curve
operands, opaque trailing zeros, Map value policy, resource refusal causes,
result roles, syntax reparsing and recursive stack use.

By the `f3cef799` cutoff:

- old production shape admission had been retired from the active path and
  retained as test/reference material;
- the census still reported 69 of 84 source boundaries unchanged, avoiding a
  false “everything changed” narrative;
- generated-source and capability inventories remained explicit at 202 whole
  outputs and 487 raw ownership/capability observations in final planning;
- a shared syntax-free semantic fold served certification and native consumers;
- the 974-case historical validation retained its assertions and live mutation
  controls while process sharding reduced measured wall time from about
  171 minutes to 55 minutes on the frozen machine/source;
- EA13 final frozen qualification, evidence-reuse reconciliation and independent
  closure review remained open.

## Where the story stops

This document stops at saved commit `f3cef799`. It does not call the prototype
complete, published or merged. Later moving-worktree changes may improve source
diagnostics and final qualification, but they need their own signed checkpoint
and evidence before this story can adopt them.

The next chapter is therefore a gate, not a victory lap: freeze the candidate,
run the maintained full qualification, rebind every reused proof to its exact
premises, publish the capability/retirement book, obtain independent closure
review, and state remaining diagnostic/source-provenance limits plainly.
