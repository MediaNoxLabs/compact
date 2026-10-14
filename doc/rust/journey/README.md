# The Compact Rust journey

> From a ledger-8 experiment to a typed Rust target—and from a catalogue of
> recording shapes to checked effect composition.

This is the long-form engineering story behind the Compact Rust work in
`MediaNoxLabs/compact`. It is written for Minokawa maintainers, Shielded and
Midnight engineers, and community contributors who want to understand not
only what exists, but why the architecture changed.

The shortest honest version is this:

1. We retained the original Scheme-to-Rust backend as a behavior oracle.
2. We rebuilt Rust emission around a closed internal IR and Rust syntax trees.
3. We made native execution, recording, replay, proof preparation, proving and
   ledger application explicit, separately evidenced stages.
4. Broad recording support exposed an architectural ceiling: whole-program
   shape recognizers accumulated faster than they composed.
5. The current prototype replaces that admission model with checked effects
   and a shared semantic fold. It is locally developed and still awaiting its
   final frozen qualification at this documentation cutoff.

## Choose a reading path

### Read the engineering story

- [Ledger streams](00-ledger-streams.md) — why this is a ledger-8 story and
  why ledger-9 remains a future comparison line.
- [The journey](01-journey.md) — the chronology, mistakes and turning points.
- [Decisions and corrections](04-decisions-and-corrections.md) — what broke,
  what changed, and what we deliberately did not claim.
- [Announcement drafts](announcements.md) — concise versions for a README,
  release note and engineering blog.

### Explore the architecture

- [System architecture](02-architecture.md) — frontend, internal IR, renderer,
  generated crate, runtime, ledger and proof boundaries.
- [Comparison and evidence](03-comparison-and-evidence.md) — TypeScript,
  original Rust, Rust-AST and the effect prototype on consistent dimensions.
- [Generated Rust examples](06-generated-rust-examples.md) — one Compact
  contract followed through types, slots, native execution and recording.
- [Runtime and macro domain model](07-runtime-and-macros-domain-model.md) —
  ownership, core types and the intentionally narrow macro boundary.
- [Gas, errors and corrected boundaries](08-gas-errors-and-fixes.md) — what is
  rejected by the compiler, what remains a runtime result, and what was fixed.
- [Evidence index](evidence-index.md) — commits, ADRs, closure reports and
  limitations behind the narrative.
- [Contributor guide](05-contributor-guide.md) and [glossary](glossary.md).

An interactive version lives in
[`site/rust-ast-storybook`](../../../site/rust-ast-storybook/README.md).

The review branch is `rust-ast-docs`, based directly on Rust-AST documentation
head `46fdf089`. It intentionally remains separate from
`codex/rust-backend-ast` until maintainers choose to merge it.

## Status and cutoff

| Line | Evidence cutoff used here | Status represented |
| --- | --- | --- |
| Upstream base | [`LFDT-Minokawa/compact` `ledger-8` at `eb72a5ab`](https://github.com/LFDT-Minokawa/compact/commit/eb72a5ab6085bf9b8564cc80c98c973e04825c2d) | Historical base |
| Original Rust oracle | [Initial comparison at `589c92ef`](https://github.com/MediaNoxLabs/compact/commit/589c92ef961ce89ac730140e81b76d4c08222a70); [current retained branch at `b9fdb80b`](https://github.com/MediaNoxLabs/compact/commit/b9fdb80bfd474c24ce0553e66b7219e32d0add90) | Oracle, not the selected architecture |
| Rust-AST 0.3 | [Qualified implementation `f9a49666`](https://github.com/MediaNoxLabs/compact/commit/f9a496660a4a69f4dd7c7927111767ad5c0c12b2); [closure `0d7610b2`](https://github.com/MediaNoxLabs/compact/commit/0d7610b2792d60fee5d3713d4eb0ede3c98cc82a); [published branch documentation through `46fdf089`](https://github.com/MediaNoxLabs/compact/commit/46fdf0895a012a50250d9b8f3c0b49e49de48b00) | Closed scoped milestone |
| Effect-algebra prototype | Local signed checkpoint `f3cef799` on 2026-10-14 | Active; EA13 final qualification not complete |

The main implementation checkout contained later uncommitted work while this
documentation was authored. It was treated as read-only. This story does not
promote that moving state to delivered evidence.

## How to read claims

Every important statement uses one of these meanings:

- **planned** — agreed work, no implementation claim;
- **prototyped** — a bounded implementation exists;
- **implemented** — code exists at the named revision;
- **locally validated** — named local checks passed;
- **independently reviewed** — a separately configured coding agent reviewed
  the bounded change; this is not a human security audit;
- **closed** — the milestone's agreed evidence and administration completed.

Likewise, native execution, recording, replay, preparation, proof generation,
verification and ledger application are distinct columns. Availability in one
column never implies the next.

## The core lesson

The Rust-AST architecture was not valuable because Rust syntax trees are
fashionable, or because Rust is automatically safer or faster. It was valuable
because the compiler boundary became inspectable: unsupported nodes fail
closed, Compact types survive into checked Rust models, generated syntax is
constructed rather than spliced, and ledger/proof ownership can be tested at
named boundaries.

The effect-algebra prototype applies the same lesson one level deeper. A
recorder should understand composable operations and their order—not memorize
an ever-growing set of contracts that happen to contain them.
