# Retained ledger-8 decoder input sizing

Measured 2026-10-08T06:09:27.491286+00:00; repository HEAD at report `231559f12aa67a37065b4663778d69ab278678d2`. Read-only artifact census; no builds, proof generation, deserialization or vault/source edits.

## Recommendation

Adopt **64 MiB (67,108,864 bytes)** as a documented, configurable **opt-in** `EncodedSizeLimit::default()` preset. This is roughly **3,846×** the largest observed ContractState and **31,670×** the largest observed verifier. No measured evidence calls for a larger preset. Preserve `EncodedSizeLimit::new(bytes)` for application-specific increases or decreases, expose the byte count through a public constant/getter, and leave legacy decoder behavior unchanged.

This is a practical generous starting policy, not a protocol maximum or guarantee for every growing application state. It bounds encoded input admission only, not upstream decoded heap, object count or CPU. A more restrictive verifier-only policy (e.g. 1 MiB, approximately 495× the observed key maximum) is possible through `new`; it need not become a second default API in this milestone.

## Measurements

| Input | Samples | Unique payload SHA-256 | Minimum bytes | Maximum bytes |
|---|---:|---:|---:|---:|
| Verifier key | 43 | 35 | 1,351 | 2,119 |
| Tagged ContractState | 1,262 | 535 | 70 | 17,450 |

338 selected source files read; 12,637,686 source bytes. Repeated embedded states count as samples but are deduplicated by payload SHA-256 in unique totals. JSON container size is not used as encoded payload size.

| Cohort | Verifier samples / maximum bytes | State samples / maximum bytes |
|---|---:|---:|
| did-original-eleven | 11 / 1,591 | 6 / 17,450 |
| did-original-joined | 8 / 1,591 | 8 / 6,352 |
| did-reducers-nine | 15 / 1,351 | 21 / 3,371 |
| did-digest-reducer | 1 / 1,351 | 1 / 1,853 |
| jubjub-scalar-cell | 1 / 1,591 | 7 / 1,862 |
| counter-historical-live | 7 / 2,119 | — |
| runtime-oracle-fixtures | — | 1218 / 5,973 |
| proof-smoke-retained-fixtures | — | 1 / 1,482 |

The `counter-historical-live` inventory label covers both Counter and shielded historical artifacts. Its verifier maximum is the shielded `release.verifier` (and equal-sized `accept.verifier`), not a claim that Counter itself needs that size.

### Largest exact artifacts

- Verifier: `/Users/ysh/.codex/worktrees/07ae/compact/target/rust-live-e671db3b-devnet/shielded/keys/release.verifier`; **2,119 bytes**; SHA-256 `d45475e1c0952fe4a0dd5c83366c72257ba65d949634f72ce6ae6420808f8734`.
- State: `/private/tmp/rust030-adr288/strict-original-gate-final/did/did-jwk-methods-final-state.bin`; **17,450 bytes**; SHA-256 `a4ac0f061a18e5d0d67674120680d5aa95d23242f32c72b17ce84d3b96e557b6`.

## Smallest integration improvement

The existing `ObservedContractState::decode_with_limit` and `decode_verifier_key_with_limit` already accept `EncodedSizeLimit`. The preset plus public `max_bytes()` avoids each consumer inventing a number and lets acquisition code reuse it. There is no need to change existing decoder signatures or silently fall back to unlimited decoding on refusal.

A maintained representative consumer, `tools/compact-rust-backend/consumers/observed_call.rs`, currently calls legacy decoding after `std::fs::read`. Switching its positive path to the existing limited APIs with the preset would demonstrate adoption. Any untrusted transport must bound acquisition before allocating its full input; passing a byte limit after `fs::read` only checks the already acquired slice. Trusted retained-fixture acquisition is not evidence of network acquisition limits.

## Scope and limitations

- Selected qualified retained resource roots came from `/tmp/rust030-resource-baseline/resources.json` (SHA-256 `9a803a9ab1d7c5a934198c20629a94dfe1fecb72e665d4656616f53859be1888`), plus maintained runtime oracle JSON and proof-smoke state fixtures. `sizes.json` records roots, source hashes, payload hashes and JSON pointers.
- Census includes DID original, joined and reducer artifacts, Jubjub scalar-cell evidence, historical Counter/shielded verifier files, and maintained runtime fixture state payloads. Selected digital-passport adoption is pure and has no applicable proof/verifier artifacts by design.
- No all-disk or entire-cache scan. Selected roots were traversed with cache/build directories pruned; each candidate source had a 4 MiB read cap. No selected source was skipped for exceeding that cap. Eight wallet-state binaries were excluded because their tag is Zswap local state rather than ContractState.
- StateValue-only bytes, out-of-scope P-256/secp256k1 resources, arbitrary generated crates outside selected roots, future larger states and deployment populations are not covered. Historical files are retained evidence, not fresh deployment validation.
- Tags/extensions identify the artifact categories for sizing; this task did not rerun upstream parsing or proof validation. Sizes alone cannot establish decoder heap/CPU safety or parent milestone acceptance.

## Receipts

- `summary.json`: small machine-readable recommendation and maxima.
- `sizes.json`: full measured sample/source manifest.
- `measure.py`: bounded census procedure.


Verified archive: [ADR0359 — Decoder artifact sizing.zip](decoder-artifact-sizing.zip), SHA256 `26759204a72aa54f0c39aa70eb3484b5fb1c85ea4a23ae7736c945a93f84eba4`, 4members.
