---
id: RUST-ADR-0085
alias: ADR-0085
title: "Preserve native ownPublicKey witness effects"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "runtime", "witnesses", "identity", "private-transcript"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 95aadd6aab24e1a2a4d5be15ad2eb7866d708e749af85b7af799b23e68e27132
---
# RUST-ADR-0085 — Preserve native ownPublicKey witness effects

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted explicit discarded OwnPublicKey native-witness actions, caller-supplied typed execution key, missing-key refusal and exactly one private FAB output without a user witness trait requirement. Implementation uses schema9/ABI37. Its motivating export is proof-false and has no ledger VM query or proof/application claim; expression-valued handling is a separate extension.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#186 closure](https://github.com/MediaNoxLabs/compact/issues/186#issuecomment-6017545731). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`2895c986`](https://github.com/MediaNoxLabs/compact/commit/2895c98603b04d3a9fcb9a2a94ad0fba23a30ece). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 85
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
```

## Historical decision and amendments

### Problem

The positive TypeScript source `examples/bugs/pm-19252/example_ten.compact` imports `CompactStandardLibrary` and exports `test1(): [] { ownPublicKey(); }`. Pinned Scheme/TypeScript compilation succeeds. The generated TypeScript executes a private native witness even though its result is discarded:

```ts
_ownPublicKey_0(context, partialProofData) {
  const key = __compactRuntime.ownPublicKey(context);
  partialProofData.privateTranscriptOutputs.push(_descriptor_1.toValue(key));
  return key;
}
_test1_0(context, partialProofData) {
  this._ownPublicKey_0(context, partialProofData);
  return [];
}
```

A known 32-byte key `[1, ..., 32]` gives `result=[]`, zero read/compute/write/delete gas, empty public transcript and one private transcript output with one `Bytes<32>` atom. `contract-info.json` marks exported `test1` as `pure:false, proof:false`; the unexported `test` is not an exported capability. The Rust target at integrated HEAD `2895c986` fails at the export source location with `unknown circuit "ownPublicKey"` before generating a crate.

### Root cause in the typed pipeline

The schema-8 Scheme Rust IR for this exact source contains `witnesses:[]`, `circuits:[]`, and exported `test1.actions = [{ kind: "circuit_call", name: "ownPublicKey", arguments: [] }]`. In `compiler/midnight-natives.ss`, `ownPublicKey` is a native witness, not a user circuit. `compiler/rust-ir-passes.ss` builds `witness-ids` only from explicit `witness` program elements. Its `state-action-ir` therefore misclassifies the discarded native witness call as `circuit_call`; `tools/compact-rust-backend/src/stateful.rs` resolves such an action only against stateful user circuits and correctly reports unknown circuit. The TypeScript pass independently recognizes the native witness, so TS success does not validate this Rust IR classification.

A parser name special case in the Rust renderer would make this one error disappear but would leave the native result and private FAB effect ambiguous. Treating `ownPublicKey` as a generated user `Witnesses` method would also burden developers with an implementation that TypeScript takes from the execution context. The result must not be optimized away merely because the Compact expression discards it.

### Proposed first slice

Represent the native call explicitly in the compiler-owned typed IR, for example `StateAction::NativeWitnessCall { builtin: OwnPublicKey }`, rather than as a user `CircuitCall`. The Scheme pass should classify the native entry by identity/class, validate zero arguments, and emit a source location. A new tagged IR variant requires a schema increment from 8 at implementation; reject unknown natives rather than silently routing them through a user circuit name. Later expression-valued native witness calls can use a matching typed `Expr` variant and a separate ADR/slice.

The Rust execution context needs an explicit caller-provided coin public key, typed using the pinned ledger-8 coin key primitive where its exact byte mapping permits. Upstream `midnight_zswap::local::State` stores coins and pending operations but no execution key; deriving `ownPublicKey` from that state is not possible. Provide a clear constructor/setter such as `context.with_coin_public_key(key)` and reject an absent key at execution. This changes the runtime surface, so bump the compiler/runtime ABI from the current 36 when implemented. Use upstream FAB `AlignedValue`/`DynAligned` for the one `Bytes<32>` private output; no ledger VM operation or gas charge occurs.

The emitter should separately track whether a circuit needs the user `Witnesses` trait and whether it writes private transcript outputs. Native `ownPublicKey` requires the latter but not the former. Keep the generated method source-typed and free of an artificial `Witnesses::ownPublicKey` requirement.

Before Rust:

```rust
// compactc --target rust fails: unknown circuit "ownPublicKey".
```

After, illustrative generated API and body (names to be confirmed by the implementation):

```rust
let context = context.with_coin_public_key(coin_public_key);
let result = contract.test1(context)?; // no user witness implementation
assert_eq!(result.private_transcript_outputs.len(), 1);

// generated body: lookup is a context effect even when its value is unused
let key = context.own_coin_public_key()?;
private_transcript_outputs.push(AlignedValue::from(key));
```

### Alternatives and scope boundary

Hardcoding `ownPublicKey` as a user witness would change ownership and the developer API; deleting the discarded call would lose the TypeScript private transcript. A broad native-witness framework is attractive but should follow an inventory of `createZswapInput`, `createZswapOutput` and expression-valued cases. Related positive examples include `examples/external/examples.compact`, `examples/types/examples.compact`, camelCase standard/coracle sources, and `examples/bugs/pm-20295/example_five.compact`; they have additional expression, constructor or Zswap operations and are not acceptance claims for this first slice.

The PM-19252 source is `proof:false`. Passing its Rust target should count as positive TS source acceptance and native behavior parity, not as a gain in proof-required recorded coverage or strict ZK acceptance. Current measured 137-fixture capability percentages do not cover every repository source.

### Acceptance

1. Frozen compiler reproduces current TS success and Rust failure, then after the fix produces a crate whose exported `test1` has no user witness argument and one private `Bytes<32>` output for a known nonzero key.
2. Rust/TypeScript compare unit result, zero four-dimensional gas, empty ordered public transcript, one private FAB value/alignment and unchanged state. Missing execution key fails explicitly.
3. Add a focused IR classification test distinguishing native witness, user witness and stateful user circuit, plus generated crate consumer compilation. Verify expression-valued `ownPublicKey` remains an explicit unsupported shape with a useful source diagnostic until supported.
4. Confirm versioned IR and runtime ABI boundaries, fixture freshness, local source acceptance and focused Clippy. This source is proof-false; no source-to-proof or ledger application claim is needed for the first slice.

### Evidence and tracking

Read-only probe at integrated HEAD `2895c98603b04d3a9fcb9a2a94ad0fba23a30ece` used immutable compactc SHA-256 `f5c8e80e3b50a9b6aee8425b928e55fb98bdc55dba6479e5f53e7270da5b1db8` and pinned Scheme `${HISTORICAL_NIX_STORE}/sd3wkamq8xk51h1fb5z48pm9cp8lgl0n-compactc/bin/compactc-scheme`. No repo code, generated fixture, ABI/schema or coverage count changed in this investigation. GitHub issue link follows.


Tracking issue: https://github.com/MediaNoxLabs/compact/issues/186 (rust-backend-v2).

### Decision and first implementation, 2026-10-05

The source classifier now indexes native witnesses from the compiler program and emits schema-9 `NativeWitnessCall { builtin: OwnPublicKey }` for a discarded zero-argument call. Other native witness actions and value-bearing native witness calls fail at their Compact source location. A separate transitive private-output check forwards native effects through stateful callees without adding a user `Witnesses` requirement. The runtime and generated crate use ABI 37; callers supply the pinned ledger `CoinPublicKey` in `CircuitContext`, or the call returns `MissingCoinPublicKey`.

Actual generated Rust for the PM-19252 export:

```rust
pub fn test1<Private>(
    context: runtime::context::CircuitContext<Private>,
) -> Result<runtime::context::CircuitResult<Private, ()>, runtime::CompactError> {
    let mut private_transcript_outputs = Vec::new();
    private_transcript_outputs.extend([runtime::fab::AlignedValue::from(
        context.own_coin_public_key()?,
    )]);
    // Unit result, zero ledger gas, no public operation.
    // The returned CircuitResult retains this private output.
}
```

The generated package exposes `Contract::test1(context)` without a user witness argument. The coin key comes from `midnight-coin-structure` and is encoded as the same upstream FAB `Bytes<32>` atom/alignment as the pinned TypeScript runtime. This path does not call the ledger VM. The initial `with_coin_public_key_bytes([u8; 32])` helper permits known-key parity tests; production consumers should use the typed `with_coin_public_key(CoinPublicKey)` input.

Focused evidence on the isolated branch before integration: original `example_ten` compiles through frozen Scheme plus frozen local Rust renderer. Its exported report remains `proof_required:false`, `recorded:false`, `observed_call:false`, `recording_status:not_applicable`. A generated-package Rust test matches the pinned TypeScript oracle for unit result, zero read/compute/write/delete gas, unchanged serialized ledger state, empty ordered public transcript, and one private 32-byte FAB output with equal alignment. A missing key is an explicit error. The oracle capture script reproduces the checked JSON byte for byte. Renderer tests pass 70/70; PM generated consumer tests pass 2/2; focused backend/runtime/consumer Clippy with warnings denied passes; Python parity inventory tests pass 6/6; compactc target boundary passes using an immutable package snapshot. A separate source probe confirms a value-bearing call fails at line 3 char 15 with a clear unsupported diagnostic.

The exact source fixture is fresh after rustfmt against frozen Scheme SHA-256 `b88a4fe84ca151cb78e57ab990f77414089825c072698539f989fb6a7b0bcf56` and frozen local Rust renderer SHA-256 `b56eae87f011a1e50e78839f820185d3eb8ba87d39cc7b4a8870b9f5ba52df7e`. Broad generated ABI-line refresh and the positive-source manifest transition are integration tasks on main after cherry-pick; this isolated branch does not claim those full gates. The source remains nonprovable, so no proof or ledger application is claimed.

Isolated delivery commit: `c1979c0c53a8acc7b941a7d38030fad691bf5f74` (conventional, GPG verified, DCO signed).
