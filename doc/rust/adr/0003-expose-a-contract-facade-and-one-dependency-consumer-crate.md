---
id: RUST-ADR-0003
alias: ADR-0003
title: "Expose a contract facade and one-dependency consumer crate"
date: 2026-10-02
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["architecture", "generated-api"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: abba5f863fb30f658dbe6ee1a670a0a32f4a167b4c775072f4f5af6704c7f6cd
---
# RUST-ADR-0003 — Expose a contract facade and one-dependency consumer crate

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept the thin generated Contract facade, explicit context ownership and exact-runtime re-export for one-dependency consumers. Later witnessed and archive-only consumer amendments supply bounded evidence. Default bundled runtime copies do not automatically provide portable multi-contract type identity; archive rehearsal is not registry publication.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#103 closure](https://github.com/MediaNoxLabs/compact/issues/103#issuecomment-6017401895). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`02bd0be3`](https://github.com/MediaNoxLabs/compact/commit/02bd0be3bd117daed751a235a77d372cd327002a) · [`054986ee`](https://github.com/MediaNoxLabs/compact/commit/054986ee1db8090990b46a4fe896b67975809173) · [`159d4bc5`](https://github.com/MediaNoxLabs/compact/commit/159d4bc51a30390c7cece1abe5a6ac35a0f06014) · [`1ed45da6`](https://github.com/MediaNoxLabs/compact/commit/1ed45da67b0530c0d32e6eef2e736d2855dbfdd2) · [`785db0a8`](https://github.com/MediaNoxLabs/compact/commit/785db0a889b90413fd9a7c5b09709e6978f04e91) · [`99aaeb1f`](https://github.com/MediaNoxLabs/compact/commit/99aaeb1fd32f583f0943dbad71c8515aac5a0a90) · [`cbbd92a4`](https://github.com/MediaNoxLabs/compact/commit/cbbd92a468ea058379abeee4b662d54e62a4b7e7). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 0003
status: accepted-partial
date: 2026-10-02
milestone: rust-backend-v2
issue: 103
```

## Historical decision and amendments

### Problem

Module-level circuit functions required consumers to discover `ledger_contract` internals and pass witnesses separately. The oracle branch's `Contract<PS, W>` methods were easier to find, but bundling mutable private/ledger state in the generated object would obscure the explicit ledger context ownership in this AST backend. A separate consumer also needed a consistent runtime type identity.

### Before and after

```rust
// Before: valid, but the contract entry points were scattered module functions.
let result = generated::ledger_contract::increment(context)?;

// After: the facade groups circuits and keeps context explicit.
let contract = generated::ledger_contract::Contract::default();
let result = contract.increment(context)?;
let recorded = contract.recording.increment(next_context)?;
```

For a witness call, `Contract::from(MyWitnesses)` stores the witness implementation, while only methods that call witnesses require `W: Witnesses<Private>`.

### Decision and ownership

Keep `pure_circuits`, `types`, and `ledger_contract` modules and their free functions for compatibility. Generate a thin `Contract<W>` method facade; expose `.recording` only for circuits with complete traces. Keep `CircuitContext<Private>` explicit at the call boundary and return the successor context in the result. Re-export the exact runtime as `generated::runtime`, and forward the optional `ledger-transaction` Cargo feature, so an external app can depend on one generated crate for its public types.

Emitter work is in `tools/compact-rust-backend/src/lib.rs` and `stateful.rs`; runtime context ownership remains in `runtime-rs/src/context.rs`. The generated `Cargo.toml` remains contract-specific and unpublishable by default.

### Evidence, consequences, and remaining work

Local commits `cbbd92a4`, `1ed45da6`, `99aaeb1f`; first-class target `159d4bc5`. The packaged external consumer builds the generated Counter, Cell, and pure crates; a combined Cargo graph uses a shared runtime root. The `--consumer --proof` gate passes. The facade is an API discovery improvement, not proof that every method is transaction-ready.

Add a documented generated prelude only after name-collision and ergonomics review. Dogfood tiny, election, zerocash, and passport from separate consumer crates. Preserve ABI 3 checks, schema rejection, and exact compiler/runtime compatibility in release tests.

### Tracking

- Issue: [M2 first-class compiler target and consumer crate #103](https://github.com/MediaNoxLabs/compact/issues/103).
- Milestone: [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).
- Delivery: facade and one-dependency feature are local; release/remote acceptance remains open.


### Decision history

- 2026-10-02: Created ADR-0003 from the generated-crate research probes with status `accepted-partial`; linked MediaNoxLabs/compact#103 in `rust-backend-v2`. Local delivery evidence is recorded above. Future changes should append a dated amendment or link a superseding ADR.


### Amendment — 2026-10-02: witnessed standalone consumer gate

**Problem.** The previous packaged `--consumer` check built Counter, Boolean Cell, and pure generated crates from a fresh Cargo consumer, but exercised witnessed recording only inside this workspace. That left the borrowed `Contract<W>::recording()` API and generated runtime type identity untested at the package boundary.

**Before:** the external consumer gate called only `Contract::default().recording.increment(context)` and `CellContract::default().recording.set_flag(context)`. The witnessed fixture test imported both its generated crate and the workspace runtime directly.

**After (separate Cargo project):**

```toml
[dependencies]
compact-contract-witness-cell-write = { path = "../witness-contract/contract", features = ["ledger-transaction"] }
```

```rust
use compact_contract_witness_cell_write::{ledger_contract::Contract, runtime::Field};
let contract = Contract::from(Secret);
let call = contract.recording().write_twice(context, Field::from(2_u64))?;
assert_eq!(call.execution.private_transcript_outputs.len(), 2);
let read = contract.recording().read_cell(call.execution.context)?;
assert_eq!(read.execution.result, Field::from(10_u64));
```

**Emitter/runtime impact.** No emitter or runtime logic changed in this amendment. `check_compactc_target.py` now emits the witnessed crate to a temporary directory and compiles a consumer with only that generated crate dependency. The test imports `Field`, context types, and optional `CallSpec` through the generated runtime re-export; implements the generated `Witnesses<u64>` trait; checks private state/output count, trace replay effects, and Field readback. This is a package-boundary regression gate for the borrowed facade added by ADR-0001/commit `02bd0be3`.

**Delivery.** Local signed/DCO commit `054986ee`. The packaged `--consumer` gate passed with the standalone witnessed consumer and the prior Counter/pure/shared-runtime consumers. Issue [#103](https://github.com/MediaNoxLabs/compact/issues/103) remains open in [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2) for broader API dogfood, release/publishing, and remote CI. The branch has not been pushed.



### Exact-head archive-only generated consumer — 2026-10-04

In the same clean `785db0a889b90413fd9a7c5b09709e6978f04e91` checkout, `python3 tools/compact-rust-backend/check_archive_consumer.py --manifest target/rust-runtime-release-785db0a8.json --compiler ${HISTORICAL_NIX_STORE}/nsaiqjddnzwldv8q02nbdnvkwfn53z51-compactc/bin/compactc` exited 0: `archive-only Counter + Cell consumer passed with one shared runtime`. The gate verified the manifest's exact commit/tree and both archive hashes, installed the unpacked macro/runtime archives into an offline vendor directory, generated independent Counter and Boolean Cell crates with exact `=0.1.0` runtime dependencies and no copied runtime sources, checked their compiler manifest entries, ran both contract tests in a separate Cargo consumer, and inspected the dependency graph for one shared runtime. This is a local archive-only simulation, not public registry publication. The full 95-call proof/ledger and ledger-v8 handoff gates had already passed with the same Nix-built compiler; same-commit remote CI is still pending.
