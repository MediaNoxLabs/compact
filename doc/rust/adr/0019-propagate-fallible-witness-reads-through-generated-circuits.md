---
id: RUST-ADR-0019
alias: ADR-0019
title: "Propagate fallible witness reads through generated circuits"
date: 2026-10-02
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["runtime", "witnesses", "gas"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 879fe078ce80b771c7e1f17ecc91a054bf8742c347047c913855fe06661dcca7
---
# RUST-ADR-0019 — Propagate fallible witness reads through generated circuits

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept generated TryWitnesses plus the infallible Witnesses adapter and propagation of returned CompactError before adopting failed private transitions. This preserves existing infallible implementations but does not catch user panics, and a type implements one trait route rather than both. Later metering coverage is separate from this error contract.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#120 closure](https://github.com/MediaNoxLabs/compact/issues/120#issuecomment-6017431927). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`f1cd704a`](https://github.com/MediaNoxLabs/compact/commit/f1cd704a9e0bc3dae79ead935d56898846d27cc7). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 19
status: accepted-partial
date: 2026-10-02
milestone: rust-backend-v2
issue: 120
```

## Historical decision and amendments

### Problem and evidence

Generated Cell, Counter, Set, Map, and List witness projections now return `Result` from canonical ledger-8 VM queries (ADRs 0015, 0017, 0018). Yet the generated `Witnesses<Private>` trait requires each method to return `(Private, T)`. A user implementing a witness cannot use `?` on a view read; current fixtures call `.unwrap()`, turning a rejected ledger query into a panic. Native frame, recorded frame, and general circuit emitters assume the witness call cannot fail. A production-facing crate needs typed error propagation without losing existing infallible user implementations.

### Before and after Rust

```rust
// Before: the generated trait forces a value pair.
impl Witnesses<u64> for ReadFlag {
    fn read_flag(&self, context: WitnessContext<'_, u64, LedgerView<'_>>) -> (u64, bool) {
        (*context.private_state + 1, context.ledger.flag().unwrap())
    }
}
let (frame, value) = frame.witness_metered(|context, meter| witnesses.read_flag(
    context.witness_context_with(LedgerView { state: context.query.state.get_ref(), meter })
));
```

```rust
// After: a new fallible implementation can propagate a failed VM read.
impl TryWitnesses<u64> for ReadFlag {
    fn read_flag(&self, context: WitnessContext<'_, u64, LedgerView<'_>>)
        -> Result<(u64, bool), CompactError> {
        let flag = context.ledger.flag()?;
        Ok((*context.private_state + 1, flag))
    }
}
let (frame, value) = frame.try_witness_metered(|context, meter| witnesses.read_flag(
    context.witness_context_with(LedgerView { state: context.query.state.get_ref(), meter })
))?;
```

Existing `impl Witnesses<Private>` methods returning `(Private, T)` continue to work through an emitter-generated blanket `impl<W: Witnesses<Private>> TryWitnesses<Private> for W` that wraps each result in `Ok`. A new implementation chooses `TryWitnesses` instead of `Witnesses`; implementing both for the same type conflicts with the blanket adapter and is not supported.

### Decision and ownership

The emitter generates both contract-specific traits from the typed witness declarations. All generated circuit and facade bounds use `TryWitnesses`; the adapter preserves existing `Witnesses` implementations. Native and recorded emitters call new fallible frame methods and use `?`; the general emitter invokes the fallible trait and uses `?` before changing private state or appending a FAB output. Runtime `CircuitFrame::try_witness_metered` and `RecordingFrame::try_witness_metered` borrow the current context/meter, accept a `Result<(Private, T), CompactError>`, and only adopt the private transition, cost, and FAB on success. The existing direct `witness_metered` methods remain available for explicit infallible frame use. No new Compact IR node, opcode, or duplicated cost constant is needed.

Generated/runtime ABI advances 9→10 because the generated trait and frame contract changes; private IR schema remains 8. A returned error aborts the generated circuit with `CompactError`, without a partial `CircuitResult` or public Verify program. A user witness can still panic by choice; this decision makes the fallible route possible and testable.

### Alternatives and risks

Changing `Witnesses` itself to return `Result` forces all existing implementations and consumers to migrate at once. Using `impl Trait` in generated trait returns would accept both pairs and Results, but a minimal Rust 2024 probe emits `refining_impl_trait` warnings for the common concrete implementation signatures in downstream crates. Catching panics would erase typed query errors and is not a sound API. A separate `TryWitnesses` trait plus blanket adapter is explicit, stable and keeps old implementations source-compatible.

Potential risks are trait-method name collisions, blanket-impl coherence for types trying to implement both traits, and missed emitter routes. The generated `TryWitnesses` adapter must disambiguate calls through `<W as Witnesses<Private>>::method`; tests must compile existing fixtures and an independent fallible implementation. Error paths must not publish a FAB, advance private state, or return a partially recorded call. The runtime's noncumulative witness gas-limit semantics remain a separate issue.

### Verification and delivery gate

- Test successful fallible witness implementations through general, native, recorded, and borrowed `Contract<W>` entry points, comparing existing TypeScript state/FAB/gas where applicable.
- Force a ledger read rejection (for example an insufficient VM gas limit) and assert `CompactError` returns without panic, private transition, FAB or public Verify result; verify native and recorded paths.
- Confirm all existing pair-returning `Witnesses` implementations compile and execute through the blanket adapter; add a compile-facing example of `?` in a generated crate.
- Regenerate all fixture libraries and pass renderer/runtime, all-target workspace compile, 131 fixture freshness, oracle/rejection, packaged external consumer, offline ledger-8 proof/application, and clean-source package compatibility gates.
- Append a dated local signed/DCO commit and evidence to this ADR, the issue, and [Milestone 2 — ADR delivery map](references.md#private-note-08). Remote CI and publication remain separate gates.

### Decision history

- 2026-10-02: Proposed after ABI-9 List parity. A standalone Rust 2024 probe verified the dual-trait blanket adapter; implementation and full gates pending. Tracking: [#120](https://github.com/MediaNoxLabs/compact/issues/120), parent [#116](https://github.com/MediaNoxLabs/compact/issues/116) / [#104](https://github.com/MediaNoxLabs/compact/issues/104), related [#110](https://github.com/MediaNoxLabs/compact/issues/110), milestone [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).


### Delivery amendment — 2026-10-02: ABI-10 fallible witness API

Local conventional GPG-signed/DCO commit `f1cd704a9e0bc3dae79ead935d56898846d27cc7` implements the dual-trait decision. Generated `Witnesses<Private>` retains pair-returning methods. Generated `TryWitnesses<Private>` returns `Result<(Private, T), CompactError>`, and its blanket adapter wraps existing `Witnesses` implementations in `Ok` with explicitly qualified calls. New user implementations choose `TryWitnesses` and can write `let value = context.ledger.flag()?;`. Native, recorded, general and borrowed contract emitters bind to `TryWitnesses`; all 131 generated fixture libraries were regenerated. Runtime `CircuitFrame` and `RecordingFrame` add `try_witness_metered` so a failure returns `CompactError` before adopting private state, metered cost or FAB output. Direct infallible frame methods remain. Generated/runtime ABI advances 9→10; private IR schema remains 8.

**Before/after behavior.** The old witness implementation used `.unwrap()` because the trait returned `(Private, T)`; a rejected ledger read panicked. A new `TryWitnesses` implementation uses `?` and the generated circuit returns `Err(CompactError::LedgerQueryRejected(_))`. Focused tests set the VM gas limit to zero to force rejection in general, native and recorded routes. Successful fallible calls match existing pair-returning calls on result, private state, serialized ledger state, gas and private FAB; the borrowed `Contract<W>` facade and its recording handle also work. The external one-dependency consumer gate now runs two tests, including a fallible `?` implementation and rejected-read path. A failed call yields no `CircuitResult` or recorded public Verify program; the user witness can still choose to panic explicitly.

**Costs and verification.** The dual trait/adapter adds 15 lines to the generated `witness-ledger-cell` and `tiny-oracle` libraries and 21 lines to `witness-cell-write` relative to ABI 9. This is a measured readability/compatibility tradeoff, not a code-size optimization. Nine focused witnessed Cell write tests and three witnessed Cell view tests pass, including legacy adapter, fallible success/error, native, recorded, general and facade paths. The 53 renderer/four CLI tests, runtime suite, all-target workspace compile, 131/131 fixture freshness, 37 pinned oracle sources, two rejection probes, formatting and scoped diff checks pass. The packaged external consumer and all 45 offline ledger-8 replay/proof/verification/validation/application cases pass. Clean-source manifest `target/rust-runtime-abi10-clean.json` was written and verified from `f1cd704a` with `dirty: false`; macro/runtime archives have 8/227 entries and compile after unpacking. The unrelated `doc/ledger-adt.mdx` edit was preserved byte for byte and excluded from the commit.

**Limits.** The blanket adapter means a type implements either `Witnesses` or `TryWitnesses`, not both. The fallible path propagates returned `CompactError`; it does not catch user panics. Cumulative witness gas-limit semantics, Merkle witness view metering, wider List element coverage, remote CI, registry publication and wallet/node submission remain open. The branch is local and unpushed. Track this delivery under [#120](https://github.com/MediaNoxLabs/compact/issues/120), parent [#116](https://github.com/MediaNoxLabs/compact/issues/116) / [#104](https://github.com/MediaNoxLabs/compact/issues/104), related frame [#110](https://github.com/MediaNoxLabs/compact/issues/110), in [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).
