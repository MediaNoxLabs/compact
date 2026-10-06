---
id: RUST-ADR-0037
alias: ADR-0037
title: "Align wallet handoff with ledger 8.0.3 construction fixes"
date: 2026-10-03
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["wallet", "transactions", "observations"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 70f851c312b8e75fbb79efba959aeaff192dd84922c79d7c230d39b5f29172a4
---
# RUST-ADR-0037 — Align wallet handoff with ledger 8.0.3 construction fixes

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept coordinated exact ledger/JavaScript 8.0.3 dependency alignment and revalidation of the existing handoff/proof paths. Release-note construction fixes are motivation, not proof of funded transfers or arbitrary node compatibility. Preserve the separate historical local-stack version observations and later compatibility decisions.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#136 closure](https://github.com/MediaNoxLabs/compact/issues/136#issuecomment-6017459885). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`83e1c715`](https://github.com/MediaNoxLabs/compact/commit/83e1c715e2a515958c393fe6c0cd960d7db9c730). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 37
status: accepted-partial
date: 2026-10-03
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/136
```

## Historical decision and amendments

- Status: Proposed (2026-10-03)
- Milestone: rust-backend-v2
- Parent: MediaNoxLabs/compact #105
- Related: #132 and #134
- Focused issue: pending

### Problem statement

The AST Rust runtime/proof smoke pin `midnight-ledger = 8.0.2`, and the cross-language handoff pins `@midnight-ntwrk/ledger-v8 = 8.0.2`. The official ledger 8.0.3 release identifies transaction-construction fixes, including transcript partitioning, fee accounting for unshielded/shielded transfers with contract calls, and Zswap proof construction. Our current counter fixtures have no fee-balancing transfers, so the 8.0.2 offline proof gate cannot exercise those repaired paths. A production wallet handoff should use the fixed matching Rust and JS version, after demonstrating no regression in generated traces, byte encoding, and package output.

### Before

```toml
midnight-ledger = "=8.0.2"
```

```json
"@midnight-ntwrk/ledger-v8": "8.0.2"
```

```rust
let call = prepare_call(recorded, spec)?;
let proven = tx_with(call).prove(provider, cost_model).await?;
```

The code path works for the bounded offline counter, but is pinned before the published construction fixes.

### Decision and after

Upgrade the ledger crate and matching JavaScript WASM package to exact `8.0.3` together. Keep the ledger primitives and transaction logic; do not implement local fee or transcript workarounds. Audit the patch release's transitive crate versions against existing exact pins and both Cargo lockfiles before updating. Revalidate canonical deployment+call bytes and the full packaged proof suite. Add a focused transfer-with-contract-call/fee-balancing test only when a matching wallet or ledger fixture can exercise the fixed path; do not claim this release upgrade alone proves wallet fee balancing.

```toml
midnight-ledger = "=8.0.3"
```

```json
"@midnight-ntwrk/ledger-v8": "8.0.3"
```

```rust
let call = prepare_call(recorded, spec)?;
let proven = tx_with(call).prove(provider, cost_model).await?;
```

The Rust adapter call site should stay unchanged. The acceptance evidence, not the version number, determines whether the upgrade is safe.

### Ownership and compatibility

- Emitter/typed IR: no behavior or schema change expected; schema 8 remains private.
- Runtime/proof integration: exact midnight-ledger pin and any required matching transitive Midnight crate pins; no custom ledger fork.
- Wallet handoff: exact npm ledger-v8 pin and integrity lock; cross-decode deploy and call at the same version.
- Generated ABI: expected to remain 15 if runtime public types and emitted code are unchanged; verify with external consumer and archive rehearsal.
- CI/release: preserve a clean source-to-proof and byte handoff gate, and document 8.0.3 in compatibility guidance.

### Alternatives and risks

Staying on 8.0.2 leaves published construction defects in the path to wallet balancing. Mixing 8.0.2 Rust bytes with 8.0.3 JS may conceal version-specific assumptions even if a simple counter decodes. Jumping to ledger 8.1+ is a separate migration and may change node/wallet support and the npm package scope. The exact 8.0.3 update may still require a transitive dependency or API adjustment; record each such change before accepting delivery.

### Acceptance

1. Exact Rust and JS 8.0.3 pins and lock integrity; `cargo check --workspace --all-targets --locked` and the isolated backend lock pass.
2. All checked-in generated fixtures are fresh; focused renderer/runtime/consumer tests pass without manual output editing.
3. Full packaged offline proof/validation/application gate passes; paired deploy/call Rust and JS round-trip, address match, and negative pair checks pass.
4. Clean release archive rehearsal is possible without a new patch dependency. A live balanced/current-TTL wallet/node submission remains separate under #105.

### Source and evidence limits

[Official ledger 8.0.3 release notes](https://github.com/midnightntwrk/midnight-ledger/releases/tag/ledger-8.0.3) say its fixes affect transaction construction and not node/indexer processing. Local `runtime-rs/Cargo.toml`, `tools/compact-rust-proof-smoke/Cargo.toml`, and wallet handoff `package.json` currently pin 8.0.2; `cargo info midnight-ledger@8.0.3` confirms Rust crate availability. No upgrade or compatibility gate is claimed in this proposal. The default current `midnight-local-dev` stack uses newer ledger/proof versions, so its docs are not evidence of an exact 8.0.3 node test.


### Issue assignment — 2026-10-03

Focused issue [#136](https://github.com/MediaNoxLabs/compact/issues/136) is assigned to [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2) before any dependency edit. The exact-version upgrade remains proposed, with no 8.0.3 test result claimed.


### Scope clarification — 2026-10-03

The source audit after opening [#136](https://github.com/MediaNoxLabs/compact/issues/136) found two `flake.nix` inputs (`zkir` and `zkir-wasm`) that still select `ledger-8.0.2`, plus the `runtime-rs::LEDGER_VERSION` constant and both Cargo lockfiles. The `onchain-runtime-v3` input follows the moving `ledger-8` branch at a pinned lock revision. A safe upgrade must either update the two tagged Nix inputs and lock in a controlled way or explicitly document why their ZKIR artifacts remain compatible; it must not relabel the runtime constant without checking the actual compiler graph. The 8.0.3 release is a transaction-construction patch, so the generated/proof artifact hashes may remain stable, but that is a test result to establish rather than assume. Keep the #136 acceptance open until this graph audit and the clean Nix compiler gate pass.


### Local dependency-alignment delivery — 2026-10-03

Local conventional GPG-verified/DCO commit `83e1c715e2a515958c393fe6c0cd960d7db9c730` advances [#136](https://github.com/MediaNoxLabs/compact/issues/136) without closing it. Rust `midnight-ledger` moves 8.0.2→8.0.3, matching the upstream 8.0.3 lock's `midnight-zswap` 8.0.3 and `midnight-storage-core` 1.1.0; the root Cargo lock resolves those exact packages. The isolated backend CLI lock remains its own graph with storage-core 1.2.1 and no ledger transaction dependency; it was inspected, not artificially forced to 1.1.0. JavaScript `@midnight-ntwrk/ledger-v8` moves to exact 8.0.3 with npm integrity lock. `flake.nix` `zkir` and `zkir-wasm` tags plus `flake.lock` now point to official ledger-8.0.3 commit `615be91b079ed8df4026c1fd75352ea6d49de1a4`. Runtime's `LEDGER_VERSION` and compatibility guides name 8.0.3. Schema 8, ABI 15, emitter and transaction adapter code are unchanged.

Validated locally: `cargo check -p midnight-compact-runtime -p compact-rust-proof-smoke --locked`; full `cargo check --workspace --all-targets --locked`; `cargo build -p compact-rust-proof-smoke --locked -j 4`; `cargo fmt --all -- --check`; 132 fresh fixtures; pinned `npm ci --ignore-scripts` and Node syntax; scoped Git diff check. Two packaged `check_compactc_target.py --proof` runs each passed all 57 offline replay/proof/validation/application calls against Rust ledger 8.0.3. Both new sealed deployments (1,765 bytes) and proven calls (3,372 bytes) decoded/re-serialized exactly in JS ledger-v8 8.0.3 with matching addresses; swapped and address-mutated pairs failed. The deployment bytes matched the earlier fixture, but call proof bytes differed both across 8.0.2/8.0.3 and between two 8.0.3 runs. This demonstrates proof-byte variability, so no version-specific byte-identity claim is made.

**Unpassed gates:** `nix develop .#compiler` evaluated the new flake and built the updated Compact runtime derivation, then began rebuilding the Rust CLI. It was interrupted before completion because local free disk fell below 10 GiB; there was no compiler error reported. The proof gate therefore used the cached Scheme compiler and ZKIR tools from the prior 8.0.2 Nix environment against the new 8.0.3 Rust graph. A clean Nix compiler build, fresh unpatched external consumer, release archive rehearsal, remote CI, transfer-with-call fee balancing, and funded/current-TTL wallet/node submission remain required under #136/#105. Branch stays local/unpushed.


### Follow-up: rebuilt compiler and external consumer gate (2026-10-03)

After removing only this worktree’s generated Cargo incremental cache, free space rose from 6.4 to 42 GiB. The updated Nix compiler shell rebuilt the `compact-rust-cli` and ledger-8.0.3 ZKIR derivations and reported `compactc 0.31.133`. From that shell, `check_compactc_target.py --consumer --proof` passed: separate generated Cargo consumers, positive and compile-fail API checks, proving artifact generation, and all 57 offline proof/verification/validation/application call shapes. Disk remained at about 38 GiB free. The source worktree included the pre-existing user-owned `doc/ledger-adt.mdx` edit; this was a local rebuild and local gate, not clean remote CI. It does not establish funded current-TTL submission, registry publication, or an unpatched external consumer.
