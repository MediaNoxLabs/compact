---
id: RUST-ADR-0089
alias: ADR-0089
title: "Preserve expression-valued ownPublicKey effects"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["compiler", "ir", "backend", "identity", "private-transcript"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 1431043705efe2a796cf96fde1f2ba0c78b4e8507022304112dd8630b90e05f8
---
# RUST-ADR-0089 — Preserve expression-valued ownPublicKey effects

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted typed expression-valued OwnPublicKey and projected bytes, retaining one private output and no artificial user witness. Isolated schema10 delivery required integration as schema11; ABI37 remained. Both exported probe calls are proof-false and the decision does not implement Zswap input/output or confer spend authorization.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#192 closure](https://github.com/MediaNoxLabs/compact/issues/192#issuecomment-6017555547). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

### Original source metadata

```yaml
adr: 89
status: proposed
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/192
```

## Historical decision and amendments

### Problem and measured source inventory

ADR-0085 handles discarded `ownPublicKey()` as a native witness action. It deliberately rejects a value-bearing call. Pinned Scheme/TypeScript accepts this smallest source, marks its export `proof:false`, and Rust IR currently fails at the return expression:

```compact
import CompactStandardLibrary;
export circuit key(): ZswapCoinPublicKey {
  return ownPublicKey();
}
```

```text
adr89-key-return.compact line 3 char 10:
  Rust backend does not yet support value-bearing native witness calls
```

`compiler/midnight-natives.ss` defines exactly three native witnesses: `ownPublicKey(): ZswapCoinPublicKey`, `createZswapInput(QualifiedShieldedCoinInfo): []`, and `createZswapOutput(ShieldedCoinInfo, Either<ZswapCoinPublicKey, ContractAddress>): []`. Seven checked-in `.compact` source files call at least one. Six compile under the pinned TypeScript compiler; `examples/camelCase/new/coracle.compact` currently fails TypeScript before it can be a parity candidate. The current positive-source manifest has 18 entries and includes PM-19252 `example_ten` as Rust success, but does not claim full coverage of these other examples. `examples/external/examples.compact` exports nonprovable `nativeMethods` with all three native witnesses; `examples/types/examples.compact` has a value-bearing `ownPublicKey` among many independent unsupported constructs. The two standard examples fail Rust earlier in constructor lowering, and PM-20295 is constructor-heavy. A dedicated source is therefore needed for one complete, attributable acceptance result.

Pinned TypeScript execution of the minimal `key()` source with key bytes `[1, ..., 32]` returns `{bytes: key}`, charges zero in all four gas dimensions, emits no public transcript, and emits exactly one private FAB atom aligned as `Bytes<32>`. The circuit is `proof:false`. This is a native execution-context read, not a user witness or ledger VM query.

### Before and proposed after

Before, the generated TypeScript performs the native read and retains its private output, while the Rust IR rejects it:

```ts
const result = ownPublicKey(context);
partialProofData.privateTranscriptOutputs.push(ZswapCoinPublicKeyDescriptor.toValue(result));
return result;
```

Proposed Rust consumer and generated body (exact syntax to be confirmed by implementation):

```rust
let context = initial.into_circuit_context(address)
    .with_coin_public_key(coin_public_key);
let step = Contract::default().key(context)?;
assert_eq!(step.result.bytes, coin_public_key.0.0);
assert_eq!(step.private_transcript_outputs.len(), 1);

// Generated inside ledger_contract::key; no W: TryWitnesses bound.
let key_bytes = context.own_coin_public_key()?;
private_transcript_outputs.extend([runtime::fab::AlignedValue::from(key_bytes)]);
let result = crate::types::ZswapCoinPublicKey { bytes: key_bytes };
```

The generated return type should remain the source-named struct with a typed `[u8; 32]` field. Calling without a key must produce `MissingCoinPublicKey`. A value-bearing call used as a struct field, binding, argument, or nested expression must retain the same private FAB output exactly once and in source order; the first test source is the direct return, with at least one nested expression check to guard the general lowering.

### Decision and ownership

Introduce a distinct tagged `Expr::NativeWitnessCall { builtin: OwnPublicKey }` in the private typed IR. The Scheme pass classifies native identity/class before user circuit fallback and validates zero arguments. This advances the private IR schema from 9 to 10 on this branch; final integration may assign a later schema if ADR-0088 also changes it. The renderer validates the specific result shape, reads the ABI-37 `CircuitContext::own_coin_public_key()`, appends upstream FAB `AlignedValue`, and constructs the source-named result. Native private effects must be tracked separately from user `Witnesses` requirements, including through nested stateful calls. No runtime method or public ABI change is proposed: ADR-0085 already versioned the typed caller coin key and missing-key error. Do not fabricate a user witness trait method, a ledger VM operation, or a recorded/provable capability.

The Rust value/fab mapping reuses the pinned ledger-8 `midnight_coin_structure::coin::PublicKey` and `midnight_base_crypto::fab::AlignedValue`. Midnight-zk contributes no proving primitive to this nonprovable native read. Strict IR decoding rejects unknown native builtins and prior schema versions.

### Why Zswap input and output are subsequent slices

Pinned TypeScript `createZswapInput` appends the encoded qualified coin to execution-local `inputs` and emits one **empty** private FAB output. `createZswapOutput` inserts a computed coin commitment into the query context, advances `currentIndex`, appends a local output and emits one empty private FAB output. The pinned `midnight-zswap::local::State` is wallet/offer state with coins, pending spends/outputs and a Merkle tree; it is not the TypeScript execution-local list/index model. Mapping these operations directly to that Rust type would silently change semantics. The upstream `coin::QualifiedInfo`/`Info` and `transfer::Recipient` are useful primitive candidates, and `Info::commitment` is relevant to output, but a separate typed execution-effect state, value validation, ordered ledger transcript and contract application tests are required before supporting either call. The next production slice here is expression-valued `ownPublicKey`, because it reuses the already proven execution key and private FAB path and unlocks typed recipient construction without inventing Zswap state. No Zswap input/output implementation is claimed.

### Verification and delivery boundary

Before code: reproduce TypeScript oracle from pinned compiler/runtime, source-location Rust rejection, and `proof:false` status. After code: a checked-in minimal Compact source, frozen compiler IR and capability receipt, exact generated Rust fixture, standalone consumer test comparing result, ledger state, private state, four-dimensional gas, ordered public transcript and private FAB value/alignment with TypeScript; missing-key test; render/nested expression tests; local fmt and focused Clippy. Verify the generated package has no artificial user witness bound. Broader source inventory, global ABI fixture freshness and combined gates are integration work. Since the source is `proof:false`, no ZKIR or ledger proof application is applicable, and no recorded-capability gain should be counted. Generated source size and compile cost can be noted but are not success criteria for this small slice.

### Tracking and amendments

- Issue: pending MediaNoxLabs/compact `rust-backend-v2` issue.
- Local commit: pending, no push.
- Delivery state: proposed; implementation and exact evidence will be appended.

Tracking issue: https://github.com/MediaNoxLabs/compact/issues/192 (rust-backend-v2).

### Implementation evidence — 2026-10-05

The bounded source now exports `key(): ZswapCoinPublicKey` and `key_bytes(): Bytes<32>` using direct `ownPublicKey()` and `ownPublicKey().bytes`. The Scheme pass lowers both to typed IR schema 10; the projected return contains `StructField(NativeWitnessCall(OwnPublicKey))`. The renderer consumes the typed native expression, reads `context.own_coin_public_key()?`, appends exactly one FAB private output, and constructs `ZswapCoinPublicKey { bytes: runtime::FixedBytes::new(key_bytes) }`. The named struct and no-user-witness `Contract::default()` API compile under runtime ABI 37. Missing key fails as `MissingCoinPublicKey`. No runtime API or ABI changed.

The pinned TypeScript compiler/runtime oracle returns key bytes 1–32 for both exports, proof:false, zero read/compute/write/delete gas, empty public transcript, unchanged contract state, and one `Bytes<32>`-aligned private FAB atom. The checked Rust consumer compares all of those values for both calls. `cargo test --locked -p compact-rust-native-own-public-key-value-fixture --test native_own_public_key_value` passed 2/2; backend render tests passed 71/71; focused backend/consumer Clippy with `-D warnings`, full fmt check, and parity inventory baseline test passed. The compiler snapshot at `${LOCAL_EVIDENCE}/adr89-scheme-root` was made read-only after emission; source Scheme SHA-256 `bb05e5d445d7e976541a8831cb34f6b98da8592b508a4f6cc9d6d8cda8dd0e4c`; emitted IR SHA-256 `dcf71e7cddd69b4d13181b1529e756c472a92c8f5931b16109934e83f22d213a`; Rust emitter executable SHA-256 `f22414ea9e756054030e6ee3b7266cfef73322ff5d96f8c3f35fbabcd2b480e8`. The generated library from this snapshot exactly matches the checked fixture after rustfmt.

The checked corpus baseline gains exactly two circuit declarations. The local branch is schema 10 because it predates ADR-0088; integration must assign schema 11 and regenerate global fixtures. This source is proof:false; no ZK proof or recorded-capability claim applies. The original proposed code example above uses a raw byte field for exposition; the actual generated Rust wraps it in `runtime::FixedBytes::new(...)` to satisfy the named struct type. Local signed commit and combined gates remain pending.

### Isolated delivery

Signed, DCO conventional commit: `96236debe4027b67f3bfe3bd5a859262e43087ac` (`feat(rust-backend): preserve expression-valued native public keys`). Worktree clean; no push. This completes the focused local implementation and tests. Parent integration will assign private IR schema 11 after ADR-0088, refresh all global fixtures, and run the combined packaged-compiler gates before the ADR is marked accepted. Issue #192 remains the delivery tracker.
