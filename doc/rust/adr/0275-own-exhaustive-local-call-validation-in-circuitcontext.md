---
id: RUST-ADR-0275
alias: ADR-0275
source_sha256: ce6de82a0bc150064901bf19d974bdb81d9031d23a6c88ee39dd183c24ff6361
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0275 — Own exhaustive local-call validation in CircuitContext

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted for implementation. Parents R030-05/#349, R030-03/#347 and R030-13/#357. Independent re-review N1 is a LOW future-maintenance observation; all current fields are covered, no current bypass is claimed. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0275 — Own exhaustive local-call validation in CircuitContext

Status: accepted for implementation. Parents R030-05/#349, R030-03/#347 and R030-13/#357. Independent re-review N1 is a LOW future-maintenance observation; all current fields are covered, no current bypass is claimed.

### Decision

Adopt the narrow ownership change below: move the existing local callback context check into a private context child with one consuming pub(crate) method. The context owner may exhaustively destructure its own private state without exposing fields. RecordingFrame retains its existing result/gas/private-output adoption. Move upstream helper comparisons intact, preserve snapshot count and callback/error order, and add no public trait, snapshot transport, runtime API, ABI, schema or generated change.

Environment::apply classifies every owned/upstream block field explicitly, preserving address construction elsewhere, seed as provenance and None coin identity as a no-op. A focused private test must pin that distinction if not already covered. No reset behavior is introduced.

Before: RecordingFrame takes selected field snapshots individually; a new locally owned context field can compile without a policy choice. After: CircuitContext's private call_local_checked borrow-destructures all fields without '..', takes the same snapshots, executes the original owned callback once and applies the same check. Ignoring private_state is explicit and justified by the local-helper contract. New fields must be classified. This forces a visible choice; it does not prove that future choices are correct.

### Accepted design and exact verification

#### Design proposal

Design basis: independent re-review at91bf9a15. N1 is a LOW maintenance gap, not a current bypass. The review correctly finds all seven current CircuitContext fields classified; upstream QueryContext/CallContext/wallet records already use exhaustive patterns. No code changes or new tests were executed for this proposal.

### Smallest ownership change

Move the existing local-call validation, including its upstream comparison helpers, into a private child module of `context` (for example `runtime-rs/src/context/local_boundary.rs`). Implement one `pub(crate)` consuming method on CircuitContext:

```rust
pub(crate) fn call_local_checked<Output, F>(
    self, call: F,
) -> Result<CircuitResult<Private, Output, D>, CompactError>
where F: FnOnce(Self) -> Result<CircuitResult<Private, Output, D>, CompactError>;
```

This is an internal ordinary function, not a new public trait, public field, consumer API or ABI requirement. The context child can read the private `coin_public_key`; `recording` cannot, and should not gain that access merely for this audit.

At entry, destructure **`&self`**, not owned self, without `..`:

```rust
let Self {
    private_state: _, // local helpers may replace private state
    query,
    zswap_state,
    circuit_zswap,
    coin_public_key,
    cost_model,
    gas_limit,
} = &self;
```

Clone the same four owned snapshots already taken today: query, wallet, intent plan and cost model. Copy coin key (the existing getter already copies this Copy carrier) and gas limit. Call the closure with the original owned self once. Compare the returned context to these snapshots using the existing exhaustive query/wallet helpers and same error text/ordering. Return the unchanged CircuitResult on success. No second context clone, private-state clone, placeholder context, borrow-through-callback, public snapshot type or tuple transport is needed.

`RecordingFrame::call_local` then calls `self.state.context.call_local_checked(call)?` and retains exactly its present result adoption: context, accumulated gas, ordered private outputs, result. It still appends no verify operations. This leaves context-boundary semantics in one owner; the frame only owns recording accumulation.

Move `recording/local_boundary.rs` into the context child rather than copying it or making its helper functions public. The new child can keep all comparison functions private. Add `mod local_boundary` in context.rs; remove the former recorded child declaration. Do not move witness execution or other recording methods.

#### Why this option

- A `same_context` free function under recording cannot exhaustively destructure CircuitContext because its coin key is private. Making that field `pub(crate)` only to silence this finding broadens ownership needlessly.
- A new owned snapshot plus borrowed projection requires extra types and field duplication for a single use. A tuple-returning splitter makes call sites less readable.
- A shared generic comparison trait would expose policy beyond its one internal boundary and risk conflating local helpers with testkit's different acceptance policy.
- Merely listing field names in a comment or comparing accessors does not make adding a field fail compilation.
- Borrow-destructuring in the context owner gives the desired compiler tripwire without reconstructing the context or changing clone counts.

### Environment.apply: classify, do not reset

Add an exhaustive `let Self { ... } = self;` pattern inside apply. Bind policy fields actually applied. Explicitly ignore `address` and `fixture_seed` with concise reasons:

- **address:** already consumed by `ContractLab::context` when `ConstructorResult::into_circuit_context(environment.address)` creates the query and call context (`testkit-rs/src/lab.rs:109–115`). `apply` must not overwrite query address or own_address on arbitrary incoming contexts. This is not an environment reset function.
- **fixture_seed:** deterministic-fixture provenance, stored/compared in snapshots; it never instantiates or resets RNG. Current docs explicitly say so (`environment.rs:33–35`).
- **coin_public_key:** retain current `Some` setter and `None` no-op. The actual lab constructs a fresh context with no identity before apply. Do not reinterpret None as an identity-clear operation.
- **block:** current four block metadata fields overwrite the corresponding call fields. An exhaustive borrowed BlockContext pattern here is appropriate too; it gives apply its own compile-time decision point for any new upstream block member.
- **cost_model/query_gas_limit:** unchanged clone/copy to context. Per-query semantics remain unchanged.

`matches` remains a separate exhaustive snapshot-compatibility comparison; it compares address and seed even though apply intentionally does not write them. Do not share it with local-helper equality or remove these fields because they are not applied here.

### Verification scope after acceptance

1. Existing runtime local-boundary integration tests: every call-context field, real effects-only mutation, each wallet collection, intent plan, coin key, gas/cost model; valid private and metered-witness controls; helper error propagation and private output order. Preserve exact error precedence and frame accumulation.
2. Existing testkit boundary suite, especially the configured environment control around boundaries.rs:379–397 and snapshot seed/address mismatch checks. Reuse these instead of duplicating all policies.
3. One narrow private Environment.apply test, if current coverage lacks it: nondefault incoming address/key + Environment address different and key None; verify apply leaves incoming address/key unchanged while applying block/policy. This documents internal no-reset semantics, not a new supported public entry point. Provenance seed remains unchanged on Environment.
4. One scratch compile-mutation demonstration can add a dummy field to CircuitContext/Environment and repair initializers only: the new exhaustive pattern must fail to compile until that field is classified. No need to check a fake field into product source or add token-string mirror tests.
5. Focused runtime/testkit tests and strict Clippy. No compiler regeneration, proof rerun, ABI or schema change: generated call APIs stay identical.

### Limits

The existing trusted generated-local-helper adapter boundary remains. Final comparison does not attest transient arbitrary callback clear/run/restore changes, deep external alias mutations, or hostile Rust code. This proposal addresses explicit ownership on future field additions, not those separately documented non-goals. Static classification cannot prove a future developer chooses the correct policy; it forces that choice to be visible.

### Exact source anchors

- `runtime-rs/src/context.rs:95–105`: seven fields and private coin key.
- `runtime-rs/src/context.rs:578–582`: own_coin_public_key copies the typed key to bytes.
- `runtime-rs/src/recording.rs:208–239`: current one-snapshot local check and recording adoption.
- `runtime-rs/src/recording/local_boundary.rs:22–69`: exhaustive upstream query/call/wallet comparison to relocate intact.
- `testkit-rs/src/environment.rs:48–59`: current apply behavior; :62–76 exhaustive matches.
- `testkit-rs/src/lab.rs:109–115`: fresh context construction applies address before environment policy.
- `/tmp/rust030-external-review/retest-91bf9a15/review.json`: independent review N1, explicitly complete today and no new bypass.

Issue: https://github.com/MediaNoxLabs/compact/issues/399

### 2026-10-07 — Delivered locally

`f965a646af4175b2fa84cc401d236f10b82882ba`, good GPG signature, conventional and DCO. CircuitContext now owns the private local-call boundary check. Its consuming internal method exhaustively classifies all seven fields, clones the same four snapshots once, preserves optional byte-key identity and invokes the original callback exactly once. RecordingFrame retains only its unchanged context/gas/private-output adoption. Existing upstream comparison bodies moved intact; no field visibility broadened.

Environment.apply exhaustively classifies its own fields and BlockContext while preserving intentional no-reset behavior: address is applied at lab construction, seed is provenance, None key does not erase an incoming key. The new private control pins those distinctions.11 focused runtime and21 testkit tests pass with strict Clippy. Scratch added-field probes produce E0027 at both intended patterns; restored source compiles.

This closes a low future-maintenance gap, not a current bypass. No public API/ABI/schema/manifest/generated/proof semantics change. A focused independent external follow-up at the committed revision is running; the whole-milestone audit remains open.

[ADR0275 — delivery-receipt.json](references-0.3.0.md#note-046).
