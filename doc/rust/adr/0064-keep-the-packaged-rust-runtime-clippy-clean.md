---
id: RUST-ADR-0064
alias: ADR-0064
title: "Keep the packaged Rust runtime Clippy clean"
date: 2026-10-04
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["distribution", "provenance", "validation"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 6961cca639bb13f429af2b84e2299857e203fc87a969de55395e0b3b4f829bd6
---
# RUST-ADR-0064 — Keep the packaged Rust runtime Clippy clean

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept targeted runtime and test cleanup for the validated Rust 1.99 Clippy gate without changing canonical VM semantics. The amendments distinguish a passing runtime target from initially failing generated-fixture/workspace gates. Later milestone success does not erase the precise scope of those earlier runs.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#163 closure](https://github.com/MediaNoxLabs/compact/issues/163#issuecomment-6017506997). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`1495bf64`](https://github.com/MediaNoxLabs/compact/commit/1495bf64f9b45e26ff543a50e1b29241cc2019ca) · [`2ba454cc`](https://github.com/MediaNoxLabs/compact/commit/2ba454cc010a1eb766e31a8d08ad924185174087) · [`79ba4c7a`](https://github.com/MediaNoxLabs/compact/commit/79ba4c7a29279fe091fbd7f2db6c982967fdbfad) · [`7ea026b4`](https://github.com/MediaNoxLabs/compact/commit/7ea026b42a1b68561d99c121e19c0008a0c7394b). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 64
status: accepted-partial
date: 2026-10-04
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/163
```

## Historical decision and amendments

### Problem and evidence

The first same-commit remote `Compact Tool Test` dispatch on `codex/rust-backend-ast` at `1495bf64` failed its `Pre-check` before running the test matrix. Its Rust 1.99.0 `cargo clippy --all-targets --all-features -- -D warnings` reports 53 runtime errors: 48 `Option<RunningCost>::clone()` calls, one `RunningCost::clone()` call, and four immediately dereferenced borrows in collection/Merkle structural readers. `RunningCost` is `Copy` on this pinned dependency graph. The complete failure log is `${LOCAL_EVIDENCE}/compact-ci-clippy-1495bf64.log`; [remote run 37193679605](https://github.com/MediaNoxLabs/compact/actions/runs/37193679605) is the reproducible branch result. This is a CI gate failure, not evidence of changed VM semantics.

### Before and proposed after

```rust
// Before, within the runtime context and structural readers:
let limit = self.gas_limit.clone();
let observed = self.observed_gas.borrow().clone();
let next = read_cell::<u64, _>(&self.fields.get(1).expect("tree shape checked"))?;

// After, preserving the same Copy values and reference:
let limit = self.gas_limit;
let observed = *self.observed_gas.borrow();
let next = read_cell::<u64, _>(self.fields.get(1).expect("tree shape checked"))?;
```

Actual edits should follow each diagnostic location and preserve ownership and return types. Avoid a blanket Clippy allow or suppressing `-D warnings` in CI.

### Ownership and compatibility

The runtime owns gas state and structural collection/Merkle readers; the compiler `syn` emitter and generated crate need no change. No new primitive, VM program, proof adapter, macro, IR variant, generated API, ABI increment, or crate version is proposed. The goal is source hygiene on the repository's existing full-workspace CI command, while preserving exact gas, state, transcript and proof behavior.

### Acceptance and limits

Create and assign a focused MediaNoxLabs issue to `rust-backend-v2` before code edits. Reproduce and eliminate the 53 reported warnings without suppressions; rerun `cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt --all -- --check`, focused runtime tests, and the exact-head packaged proof/ledger gate if the runtime source changes. Audit the final commit for conventional subject, DCO and GPG. Push the new head and rerun same-commit `Compact Tool Test`, `Compiler Build`, and other required jobs; only remote green closes the issue. A future Rust/Clippy release may add warnings, so this decision claims the current pinned CI toolchain only.

### Tracking

- Parent release/CI issue: [#106](https://github.com/MediaNoxLabs/compact/issues/106).
- Focused issue: pending creation before implementation.
- Delivery: proposed; no code or green CI claim.

### Delivery-order amendment — 2026-10-04

The user subsequently deferred remote CI until the local backlog is complete. Local Clippy and behavior verification remain appropriate for this scoped cleanup. Keep the focused issue open and its remote acceptance box unchecked; do not dispatch another remote run during local delivery.

### Tracking amendment — 2026-10-04

Focused [#163](https://github.com/MediaNoxLabs/compact/issues/163) was created and assigned to `rust-backend-v2` before code edits. The pending sentence above preserves the proposal sequence.

### Local implementation checkpoint — 2026-10-04

Conventional GPG-signed/DCO commit `2ba454cc` (`fix(rust-runtime): use Copy gas values in ledger reads`, `Refs: #163`) replaces the 48 `Option<RunningCost>` clones, one `RunningCost` clone and four redundant borrows at the exact Rust 1.99 diagnostic sites. `git verify-commit` reports a good signature. The large `context.rs` diff is principally rustfmt compaction after removing `.clone()`: 83 added/202 removed lines there, and two-line substitutions in each structural reader. No emitter, generated crate, macro, VM program, API, ABI 33 or private schema 8 change was made.

In the warm rehearsal checkout, `cargo check -p midnight-compact-runtime --lib --offline` passed in about 10 seconds. Focused `context`, `collection_initial_parity` and `typed_slot_witness` integration suites passed 1+3+2 tests in about 17 seconds. `cargo fmt --all -- --check`, Python gate syntax and scoped diff checks passed at final local HEAD `79ba4c7a`; the exact-head Nix compiler consumer gate also passed with the changed runtime source. A Rust 1.99 full-workspace Clippy attempt spent over 20 minutes compiling dependencies and was stopped before reaching the runtime; a focused 1.99 runtime attempt was also stopped during dependency builds. Therefore no post-fix Clippy pass is claimed. Per the user’s local-first direction, the full Clippy and remote runner gates remain open for final stabilization; [#163](https://github.com/MediaNoxLabs/compact/issues/163) stays open.


### Rust 1.99 runtime-test extension — 2026-10-04

The exact full workspace Clippy attempt after ADR-0068 exposed two additional `useless_conversion` diagnostics in the existing Merkle VM program test, after the 53 original runtime findings were removed. Conventional GPG-signed/DCO `7ea026b4` (`fix(rust-runtime): remove identity conversions in Merkle tests`, `Refs: #163`) passes captured root fields directly and removes the newly unused test import. The focused `native_merkle_programs_match_captured_ledger8_operation_order` test passes. `cargo fmt --all -- --check`, staged diff and `git verify-commit` pass. A repeat full Rust 1.99 command advances past the runtime test target and fails only in generated fixture libraries, which need separate emitter decisions; therefore this is runtime-target evidence, not a full-workspace Clippy pass. No VM, runtime library, emitter, generated ABI 34 or schema 8 behavior changes. Same-commit remote CI remains deferred and #163 stays open.
