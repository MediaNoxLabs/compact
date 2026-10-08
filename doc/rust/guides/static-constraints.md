# Static constraints and migration evidence

Current guide reconciliation, 2026-10-07. This page maps a specific developer-facing claim to its controls. Reading or hashing a retained result is not a new compiler/test execution. Full evidence paths and hashes are in [evidence relocation index](../evidence/0.3.0/relocation-index.json); original vault archives remain unchanged.

## What Rust checks, and what it does not

| Claim | Control and retained evidence | Scope |
|---|---|---|
| Old generated ABI49 source refuses an ABI50 runtime | ADR0293 `consumers/historical-abi49/lib.rs`, `historical-abi49.log`: E0080 at both unchanged ABI assertions | Actual historical compiler refusal; do not edit assertions to force compatibility |
| Existing ABI50 consumers can use the new matching package pair | ADR0293 `generated-consumer-results.json`: old and bundled1.99 plus shared1.88 consumers pass | Those exact generated sources, locks, features and hosts; not arbitrary same-version sources |
| Backend `RenderError` requires a fallback in external matches | ADR0293 `new-exhaustive.log` and `new-complete-no-fallback.log`: E0004; `migrated-primary-final.log` and `migrated-msrv-final.log`: migrated programs pass | The fallback remains required even when all currently known variants are named |
| Exact runtime metadata/source pairing is checked before output publication | `tools/compact-rust-backend/tests/compatibility.rs`: version, manifest/source, dependency-source and output-preservation controls; ADR0293 `compatibility.log` | CLI preflight and its tests, not source authentication or a cryptographic signature |
| Typed collection views reject ordinary element/key mistakes | `check_archive_consumer.py`: `wrong_set_key`, `wrong_map_key`, `wrong_list_element`, `wrong_merkle_leaf`; mismatched-types diagnostics required | Existing maintained negative consumers. Milestone2 `03c03a39` archive-consumer success is historical; no new0.3 run claimed here |
| Typed slots reject ordinary writes/arguments of the wrong type | `check_compactc_target.py`: wrong Cell/Set/List/Map/Merkle values, aggregate keys and generated arguments, with diagnostic checks | Existing maintained controls; this reconciliation did not locate and join a fresh0.3 per-case run receipt |
| Byte lengths and numeric bounds are explicit in carrier types | `runtime-rs/src/primitives.rs`: `FixedBytes<N>::new([u8; N])` and private-field `BoundedUint<MAX>::new` | Array length is part of the Rust type. Integer magnitude is checked at construction/decoding/arithmetic; it is not universally proved at compile time |
| Well-typed application data may still fail source rules | Passport two-test historical external consumer: zero schema major version fits the carrier but returns `AssertionFailed` | Source-dependent validation remains dynamic; typed construction is not credential or signature validity |

The preserved source compatibility and migration receipts are enough to explain these existing distinctions. They do not establish that every public misuse has a negative consumer test. In particular, this mapping found no focused retained external compile-fail receipt specifically for wrong `FixedBytes<N>` length or a wrong generated witness signature. Those dedicated external cases are outside this evidence set; no claim here depends on them.

## Read the historical migration evidence

The [retained migration evidence](../evidence/0.3.0/index.md#migration-cf1) preserves the selected migration logs and receipt; its original directory was `/tmp/compact-adr293/` (historical provenance only). The receipt was built against an intermediate source candidate; the signed joined delivery then integrated the changes. Use both records:

- `receipt.json`: frozen source/package mapping and artifact hashes.
- `migration-results.json`: old exhaustive positive control, new exhaustive refusals and migrated positive control.
- `generated-consumer-results.json`: old ABI50, bundled and shared generated consumers.
- `historical-abi49.log`: actual E0080 refusal against the0.2.0 pair.
- `migrated-primary-final.log` / `migrated-msrv-final.log`: final migrated execution with Rust1.99/1.88.
- Vault evidence archives `ADR0293 and ADR0304 pre-join evidence.zip` and `ADR0293-0295-0303-0304 — Signed joined delivery.zip`: original measurements and integration provenance.

ADR0304's standalone lock correction preserves qualified package identities and dependency edges; its actual Rust1.88 result is a separate dependency-graph gate. It is not a compile-fail test for generated application types.

## Keep the ownership boundary explicit

A type-safe function signature prevents some accidental combinations. It does not establish authorization, a trusted ledger origin, absence of shared interior mutation, proof acceptance or network finality. ContractLab adapters are trusted Rust closures, and rollback covers the lab's owned checkpoint. Read the testkit guide for that boundary.

ADR0285/#409 was resumed with owner authorization and delivered at4c8aebc3. Its external generated-consumer controls passed two runtime positives and intended E0382 moved-context / E0423 private-construction refusals. The earlier stopped partial work remains unaccepted history; it is not substituted for this later receipt. See “ADR0285 resumed consumer qualification — 2026-10-08” (historical vault reference; not bundled here) and “Typed consumer safety local acceptance — 2026-10-08” (historical vault reference; not bundled here). Broader assurance and candidate qualification have separate dispositions; see [candidate qualification](candidate-qualification.md).
