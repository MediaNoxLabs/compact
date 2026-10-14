# Announcement drafts

## Repository README

### The Compact Rust journey

We have published an evidence-backed story of the Compact Rust backend—from
the original Scheme text emitter, through the typed Rust-AST target, to the
current checked recording-effect prototype.

The story explains the compiler/runtime boundary, reuse of official Midnight
ledger and ZK primitives, DID and passport qualification, the limits of our
parity claims, the bugs independent implementations exposed, and why recording
moved from whole-contract shape recognition to composable typed effects.

Start with [`doc/rust/journey`](./journey/README.md), or open the interactive
storybook under `site/rust-ast-storybook`.

## Release-note style

### Documentation: Compact Rust engineering storybook

This change adds a publishable engineering narrative and interactive static
storybook for the Rust backend. It covers the ledger-8 baseline, original Rust
oracle, Rust-AST milestones, 0.3 qualification and active effect-algebra
prototype. Architecture diagrams, decision cards and evidence matrices make
native execution, recording, replay, proof and application claims explicit.

The documentation is frozen at local prototype commit `f3cef799`. It does not
claim the active prototype is merged, released or EA13-complete.

## Engineering-blog opening

### When a code generator becomes a compiler backend

Our first Compact-to-Rust path proved the destination was possible. It also
gave us a priceless oracle: real programs, real state transitions and real
places to disagree. The next path changed the engineering question. Instead of
asking how to print Rust for a compiler node, we asked what typed contract the
frontend should hand to a Rust backend—and which layer owns ledger execution,
recording and proof preparation.

That separation took us through a closed IR, syntax-tree emission, a native
runtime over official Midnight primitives, source-bound proof gates and two
application cohorts. Then it exposed the next ceiling: our recording support
understood more and more contract shapes, but did not yet understand why their
effects composed. This is the story of replacing that catalogue with checked
effect composition, including the wrong turns and the evidence boundaries we
learned to keep.

## Community post

We turned the Compact Rust backend history into an interactive engineering
storybook. It is approachable if you are new to Compact, but every milestone,
diagram and claim links back to code, ADRs and evidence. You can follow one
contract from Chez Scheme analysis to typed Rust, ledger replay and proof—or
jump straight to the candid chapter on bugs, removed scope and remaining work.
