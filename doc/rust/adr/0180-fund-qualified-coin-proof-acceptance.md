---
id: RUST-ADR-0180
alias: ADR-0180
title: "Fund qualified coin proof acceptance"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["proof", "qualified-coin", "funding"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 3fb6d4c3e24d77b26e2b0c12d9cdf941b6364dbf7f74d8de318343a633ca6b57
---
# RUST-ADR-0180 — Fund qualified coin proof acceptance

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Real ledger/Zswap wallet inputs fund qualified Set and Cell calls so the retained strict proofs apply with default balancing and replay checks. This establishes those seeded calls, not every possible qualified-coin lifecycle or all unsigned cases.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#284 closure](https://github.com/MediaNoxLabs/compact/issues/284#issuecomment-6017711607). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`831276e3`](https://github.com/MediaNoxLabs/compact/commit/831276e3e6bef3c845cc688086929d1e98344032). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
title: ADR-0180 — Fund qualified coin proof acceptance
status: delivered-locally
date: 2026-10-05
milestone: rust-backend-v2
```

## Historical decision and amendments

### Problem

ADR-0170 and ADR-0176 prove offer-bound qualified coin Set insertion and Cell replacement with the real offer-derived commitment index. Their single output of value 42 has no funding input. Ledger-8 default strict `well_formed` correctly rejects `BalanceCheckOverspend(-42)`, and the proof smoke applies only with `enforce_balancing = false`. That validates VM/proof wiring but leaves the funded default-strict acceptance part of #105 open.

### Before and after

Before, both proof modules create an output-only offer and relax balancing:

```rust
let offer = Offer { inputs: vec![].into(), outputs: vec![output].into(),
                    transient: vec![].into(), deltas: vec![Delta { token_type, value: -42 }].into() };
let mut strictness = WellFormedStrictness::default();
strictness.enforce_balancing = false;
let verified = sealed.well_formed(&ledger, strictness, now)?;
```

After, a local funded fixture should seed an owned coin into the ledger Zswap tree and matching local wallet state, spend that actual coin, and create the call's output with the same token/value. The offer must be assembled by upstream `Offer::new` so it derives/normalizes its delta. The observed call remains bound to that exact offer:

```rust
let (ledger, wallet, owned_coin) = seed_local_owned_coin(ledger, &keys, seed_output)?;
let (_, input) = wallet.spend(&mut rng, &keys, &owned_coin, None)?;
let offer = Offer::new(vec![input], vec![output], vec![]).ok_or("empty offer")?;
let bound = OfferBackedObservedState::new(observed, &ledger, offer)?;
let prepared = bound.prepare(call, verifier, randomness)?;
let sealed = prove_and_seal(prepared.into_transaction(&mut rng, network, ttl))?;
let verified = sealed.well_formed(&ledger, WellFormedStrictness::default(), now)?;
let (updated, outcome) = ledger.apply(&verified, &context);
```

The seed is explicit privileged **local fixture setup**, not a claim of a valid token mint, wallet funding API, network finality, or node submission. The tested transaction itself must have a real input Merkle witness/nullifier, output proof, balanced upstream commitment/delta, and default strict proof/signature/limit checks.

### Domain and implementation decision

Use ledger-8.0.3/midnight-zswap-8.0.3 primitives: `SecretKeys`, `local::State::apply`, `local::State::spend`, global `ZswapState::try_apply`, `Input`, `Output`, and `Offer::new`. The seed output is encrypted to the same user keys so local state can recover its `QualifiedCoinInfo`; global and local trees must agree on root and index before spending. Preserve ADR-0170's opaque offer-bound observed API and generated Set/Cell crates. Keep an output-only negative smoke asserting exact `BalanceCheckOverspend(-42)`; never toggle balancing in the funded path. No emitter, runtime ABI, or IR schema change is expected unless upstream constraints expose a genuine missing API.

### Guards and acceptance

Prove with pinned ZKIR 2.1.0, call and Zswap output/spend key material. Assert default strict `well_formed` succeeds for both Set `insert_coin` and Cell `write_coin`, ledger-8 application succeeds, expected contract state matches the generated native/recorded state, spent input nullifier is claimed, output index/commitment are from the same offer, and a second spend/replay is rejected by ledger state. Retain the existing unfunded strict negative tests and verify wrong offer/observation or missing index still fail. Run only focused local proof and relevant tests/Clippy; conventional GPG+DCO local commit, no push or remote CI.

Ledger-8 source reference: [Zswap offer and balance specification](https://github.com/midnightntwrk/midnight-ledger/blob/ledger-8/spec/zswap.md). Delivery is part of [#105](https://github.com/MediaNoxLabs/compact/issues/105); a dedicated issue tracks this bounded local acceptance proof.


### Local acceptance evidence (2026-10-05)

Delivered in GPG+DCO signed local commit `4b3bec1ee62e0db11c261b7456b347507abd91d9` on `codex/adr180-funded-qualified-coin`, based on schema20/runtime ABI45 fixture checkpoint `54474fb4`. `git verify-commit` reports a good signature. No emitter/runtime ABI/IR schema changes were needed; the implementation adds a shared proof-smoke funding helper and extends both existing qualified-coin proof paths.

Both final Set `insert_coin` and Cell `write_coin` passed unchanged `WellFormedStrictness::default()` and returned successful ledger-8 application. The encrypted genesis coin is recovered by upstream local wallet state, with matching local/global Merkle root and index, and spent through its actual witness. `Offer::new` balances a real input against the same token/value 42 output. The exact offer remains owned by `OfferBackedObservedState`. Assertions compare funded native/recorded state, public effects and gas, compare applied state, and inspect the final Set member or Cell coin with the actual allocated output index/commitment. The spent input nullifier is present and replay through upstream `zswap.try_apply` rejects exactly `NullifierAlreadyPresent` for that nullifier. Calling `well_formed` on a later ledger state alone is not the stateful double-spend check.

The initial funded-only attempt exposed a Dust fee deficit, as reported during implementation. Final acceptance requires upstream `TestState::give_fee_token`: privileged Dust generation registration, a Night reward, and advancing fixture time to the Dust cap. `balance_tx` supplies and proves the Dust fee spend. The seeded Zswap root is sealed with `post_block_update` at that fixture time, and the call expiry is one hour later. These are explicit local fixture prerequisites. The transaction being accepted uses default strict balancing/proof/signature/limit checks; genesis/reward setup is not a token mint or wallet funding API claim.

Reproduction also requires `MIDNIGHT_LEDGER_TEST_STATIC_DIR`, consumed by the upstream reward resolver. Final runs used `${MIDNIGHT_LEDGER_SOURCE}/ledger/static`. The first takeover rerun omitted it and failed; its log remains `${LOCAL_EVIDENCE}/compact-adr180-set-proof-missing-static-dir.log`. The helper now reports this missing prerequisite as an error rather than entering the upstream panic. Earlier worker Dust/replay failures were reported in chat; no corresponding persisted logs were found during takeover, so they are not represented as independently reproduced receipts.

Validation used Rust 1.99, `CARGO_INCREMENTAL=0`, and the existing `target/adr157` cache. Final Set and Cell logs are `${LOCAL_EVIDENCE}/compact-adr180-final-set-proof.log` and `${LOCAL_EVIDENCE}/compact-adr180-final-cell-proof.log`; each retains exact unfunded `BalanceCheckOverspend(-42)` before the labeled relaxed smoke and funded strict acceptance. Targeted `cargo clippy -p compact-rust-proof-smoke --all-targets -- -D warnings`, package formatting and `git diff --check` pass. Focused fixture freshness: 2 checked, 0 stale, 0 failed. Pinned ZKIR 2.1.0 artifacts were reused at `${LOCAL_EVIDENCE}/compact-adr180-funded-set-proof` and `${LOCAL_EVIDENCE}/compact-adr180-funded-cell-proof`.

The exact command/source/artifact/log hash receipt is `${LOCAL_EVIDENCE}/compact-adr180-receipt.json` (SHA-256 `9355657c3e9aa935c71dbfea3e1e87edce2b1d9ff4c02190db2e236f740210cd`). It identifies all 8 focused verification commands and 74 proof artifact files. No broad suite, push or remote CI was run. Local proof acceptance does not establish network submission or finality. Tracking issue: [#284](https://github.com/MediaNoxLabs/compact/issues/284), under [#105](https://github.com/MediaNoxLabs/compact/issues/105).


### Main ABI46 integration — 831276e3

Integrated with conventional description, verified GPG+DCO. Both funded Set and Cell proofs were rerun successfully on combined runtimeABI46/schema20 with default strictness unchanged. Asserted native/recorded/applied state, exact qualified coin data, commitment/index and spent-nullifier replay rejection. Logs: ${LOCAL_EVIDENCE}/compact-integrated-adr180-set-proof.log and ${LOCAL_EVIDENCE}/compact-integrated-adr180-cell-proof.log. Strict proof-smoke all-target Clippy, formatting and diff checks pass. Explicit prerequisite MIDNIGHT_LEDGER_TEST_STATIC_DIR=${MIDNIGHT_LEDGER_SOURCE}/ledger/static. Funding is privileged local genesis plus upstream Night/Dust fixture/time; network wallet submission/finality remains open.

Receipt ${LOCAL_EVIDENCE}/compact-831276e3-integration-receipt.json records source-tree equality with77505018, whose six-fixture focused receipt and inventory remain valid; only the signed commit description was expanded. No push/remote CI.
