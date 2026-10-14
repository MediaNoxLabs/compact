# Contributor reading guide

## If you have 15 minutes

1. Read [the journey](01-journey.md).
2. Study the pipeline in [architecture](02-architecture.md).
3. Read the “current limitations” in
   [comparison and evidence](03-comparison-and-evidence.md).
4. Open the current backend guide at
   [`tools/compact-rust-backend/README.md`](../../../tools/compact-rust-backend/README.md).

## If you are changing the frontend or private IR

- Start at `compiler/rust-ir-passes.ss` and the Rust IR model.
- Treat the schema number as an internal compatibility boundary.
- Add real frontend source evidence, not only hand-authored JSON.
- Update resource traversal and every exhaustive semantic consumer.
- Preserve error precedence and source-owner diagnostics.
- Read [ADR-0008](../adr/0008-preserve-compact-source-locations-through-the-rust-ir.md),
  [ADR-0332](../adr/0332-reject-recursive-pure-calls-at-the-private-ir-admission-boundary.md)
  and [ADR-0347](../adr/0347-track-versioned-compact-specification-conformance-separately-from-fixtures.md).

## If you are adding a native Compact operation

- Put language/type facts in the shared semantic layer where possible.
- Keep Rust syntax in an output consumer, not the semantic rule.
- Reuse official ledger/crypto primitives instead of recreating them.
- Test the valid case, malformed types, boundaries and error order.
- Compare with TypeScript where it is an independent semantic reference.
- Do not infer recording support from native support.

## If you are adding recording support

- Start from a checked operation and its declared slots/types.
- Preserve source order, lexical identity, short-circuiting and witness order.
- Return a structured gap when the full replayable plan is unavailable.
- Separate execution result values from FAB/proof inputs.
- Exercise replay and preparation independently.
- Add proof/application only when the exact source and artifacts make it
  applicable; record balancing and funding policy.

## If you are changing the runtime

- Identify whether the authority belongs to Compact representation, the
  generated API, ledger-8 or proof integration.
- Keep runtime ABI and dependency identities explicit.
- Test decoding and resource admission before allocating/decoding untrusted
  input.
- Do not convert additive convenience into an accidental public ABI break.
- Read the [runtime guide](../../../runtime-rs/README.md),
  [ownership guide](../guides/ownership.md) and
  [version selection](../guides/version-selection.md).

## Evidence checklist

- [ ] Exact source, branch and commit are named.
- [ ] Planned, implemented, validated and closed are not conflated.
- [ ] Native, recorded, replay, preparation, proof and application are separate.
- [ ] Fresh runs and reused receipts are labeled with their premises.
- [ ] Fixture counts are not called specification coverage.
- [ ] Private IR does not masquerade as a public Compact specification.
- [ ] Failed attempts and changed policies remain in the record.
- [ ] No absolute local paths, secrets, keys or proof material enter publication.

## Where to go next

- [Rust documentation index](../README.md)
- [Developer guides](../guides/index.md)
- [ADR register](../adr/README.md)
- [CoPS problem statements](../../cops/README.md)
- [0.3 closure report](../0.3.0-closure.md)
- [0.3 final metrics](../0.3.0-final-metrics.md)
