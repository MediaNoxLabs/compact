---
id: RUST-ADR-0038
alias: ADR-0038
title: "Declare generated witness signatures once"
date: 2026-10-03
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["architecture", "generated-api", "macros"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 47fc33e46134c4dcec395f5b5f4ff5d9f3ecfaa4569a86bfbc504c01af6fb58c
---
# RUST-ADR-0038 — Declare generated witness signatures once

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept the narrow witness-signature attribute macro that derives fallible/infallible trait plumbing from one ordinary trait declaration. It does not own circuit semantics or replace typed body lowering. Keep grammar diagnostics, inspectable expansion, ABI 16 compatibility and the bounded size/timing evidence separate.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#137 closure](https://github.com/MediaNoxLabs/compact/issues/137#issuecomment-6017461620). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`34ff328e`](https://github.com/MediaNoxLabs/compact/commit/34ff328e839581de6f5ab4910bb0e06439f829f2). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 38
status: accepted
date: 2026-10-03
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/137
```

## Historical decision and amendments

### Problem

Each generated `ledger_contract` repeats every witness signature in `Witnesses<Private>`, `TryWitnesses<Private>`, and a blanket infallible-to-fallible adapter. This obscures the public witness API and grows large contracts. The two-trait model itself is useful: consumers can implement an infallible witness or implement `TryWitnesses` directly when ledger projections can fail. The AST emitter at `tools/compact-rust-backend/src/witness.rs` builds all three from each declaration; `tools/compact-rust-backend/src/lib.rs` inserts them into the generated module.

### Before and after

Current ABI-15 generated shape from `witness-cell-write`, abbreviated only in path spelling:

```rust
pub trait Witnesses<Private> {
    fn secret(&self, context: WitnessContext<'_, Private, LedgerView<'_>>, seed: Field)
        -> (Private, Field);
}
pub trait TryWitnesses<Private> {
    fn secret(&self, context: WitnessContext<'_, Private, LedgerView<'_>>, seed: Field)
        -> Result<(Private, Field), CompactError>;
}
impl<Private, W: Witnesses<Private>> TryWitnesses<Private> for W {
    fn secret(&self, context: WitnessContext<'_, Private, LedgerView<'_>>, seed: Field)
        -> Result<(Private, Field), CompactError> {
        Ok(<W as Witnesses<Private>>::secret(self, context, seed))
    }
}
```

Proposed generated source keeps the infallible trait as ordinary visible Rust and annotates it once:

```rust
#[runtime::compact_witness_bridge]
pub trait Witnesses<Private> {
    fn secret(
        &self,
        context: runtime::context::WitnessContext<'_, Private, LedgerView<'_>>,
        seed: runtime::Field,
    ) -> (Private, runtime::Field);
}
```

The attribute proc macro emits the original trait plus the same public `TryWitnesses<Private>` trait and blanket adapter. User implementations continue to target either trait. The generated crate does not expose VM instructions or change witnessed execution.

### Why an attribute macro

The existing `midnight_compact_runtime_macros` crate already uses `syn`, `quote`, and `proc-macro2` for ledger derives. A new attribute can parse the generated `syn::ItemTrait`, retain its source-shaped signature, clone each method signature with `Result<Output, runtime::CompactError>` for `TryWitnesses`, and quote the forwarding blanket impl. It should validate the narrow generated grammar: one `<Private>` parameter, `&self`, identifier parameters, no default bodies, and zero or more methods. Unsupported input should produce a spanned compile error rather than panic. The attribute is reexported by the runtime crate as `runtime::compact_witness_bridge`. The generated module already imports that crate as `runtime`.

A declarative `runtime::compact_witnesses! { pub trait Witnesses<Private> { ... } }` alternative was prototyped at `${LOCAL_EVIDENCE}/compact-witness-bridge-probe/bridge.rs`: a standalone `rustc` run compiled and executed infallible and direct fallible implementations, including an error return. It has the same Rustfmt-only line reduction, but places the only human-readable trait inside an opaque macro body. Prettyplease's generic `ItemMacro` printer handles that body as tokens; an attribute leaves the trait on the normal `syn::ItemTrait`/prettyplease path and is easier for rustdoc and source inspection. A later standalone attribute-macro prototype also compiled and ran; real generated-crate compilation remains an implementation gate.

### Oracle comparison

The `codegen-rust` oracle has a user-impl `#[witnesses(Name, PS = Type)]` proc macro that forwards inherent methods into a generated `Witnesses<PS>` trait ([oracle macro](https://github.com/MediaNoxLabs/compact/blob/codegen-rust/runtime-rs-macros/src/lib.rs#L53-L110)). Its tiny snapshot has a single trait and direct calls ([oracle snapshot](https://github.com/MediaNoxLabs/compact/blob/codegen-rust/compiler/snapshots/tiny-rust-expected.rs.txt#L66-L99)). That macro does not model the current AST backend's fallible `TryWitnesses` bridge or metered ledger projection. Copying it would change user implementation syntax without removing generated duplicate trait signatures. Deleting either present trait would break existing infallible or fallible consumers.

### Emitter, runtime and ABI ownership

The attribute macro belongs in `runtime-rs-macros`, reexported through `runtime-rs`. It owns only mechanical trait and adapter definitions; the runtime retains `WitnessContext`, `CompactError`, metering, state and transcript semantics. The AST emitter emits the ordinary `Witnesses` trait with this attribute and stops emitting the duplicate `TryWitnesses`/adapter blocks. Compact declaration type checking remains in `witness.rs`; no Rust body text is appended. Since newly generated code depends on a runtime macro absent from ABI 15, bump the generated/runtime assertion to ABI 16; private IR schema remains 8. Existing generated ABI-15 crates can continue to compile against the new runtime if its macro addition is additive, but the final compatibility gate must verify this.

### Measured source-shape probe

A read-only script removed the duplicate blocks in actual checked-in ABI-15 generated libraries, added the proposed attribute, and ran `rustfmt` on temporary copies. Across 108 of 132 fixture libraries, 58 witness methods in 33 nonempty fixtures and 75 empty fixtures, formatted source shrank 29,001→28,036 lines (−965, −3.3%). `witness-cell-write` shrank 413→394 (−19), `election-oracle` 1128→1035 (−93), `zerocash-oracle` 774→668 (−106), and `counter` 189→188 (−1). These are source-shape measurements, not generated-crate compilation or runtime improvements. The standalone declarative alternative compiled in 0.11s on one warm `rustc` run, while a separate attribute-macro consumer compiled in 0.54s on one run. These tiny samples are neither same-head generated-crate comparisons nor evidence of a speed improvement. No repository compiler or runtime files were edited for this proposal.

### Acceptance and risks

- Compile zero-, one-, and multi-method generated crates using the actual attribute macro. Confirm expanded `Witnesses`, `TryWitnesses`, and blanket impl signatures and coherence are equivalent, including typed parameters and structured returns.
- Preserve direct infallible `Witnesses<Private>` and direct fallible `TryWitnesses<Private>` consumer implementations. Exercise ledger projection failure, witness rejection, native/recorded calls, FAB/private outputs, gas, state and ordered public Verify transcript. Keep wrong-signature diagnostics at the user impl site and inspect rustdoc visibility.
- Regenerate fixtures, run renderer/runtime/consumer tests, independent TypeScript parity captures and packaged proof/application gate. Measure clean and warm generated-crate compile times before/after on the same host and target; do not claim speed from source reduction alone.
- The expanded `TryWitnesses` trait becomes invisible in raw generated source. Add a concise generated comment or rustdoc link beside the attribute and verify IDE navigation and docs before adoption. Proc macro span handling and generic validation are the main implementation risks.

### Tracking

- Focused issue: [#137](https://github.com/MediaNoxLabs/compact/issues/137) in [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).
- Related: [ADR-0036 — Unify contract recording access across witness shapes](0036-unify-contract-recording-access-across-witness-shapes.md), [#135](https://github.com/MediaNoxLabs/compact/issues/135).
- Delivery: proposal and temporary source/macro probes only; no repo edit, commit or push.

### Standalone attribute-macro check (2026-10-03)

A `/tmp` attribute macro built with the same `syn`/`quote`/`proc-macro2` dependency shape as `runtime-rs-macros` (`bridge_attr.rs`, compiled directly against cached workspace rlibs) parsed `syn::ItemTrait`, cloned each method signature into `TryWitnesses`, and generated the blanket adapter. Its consumer (`bridge_attr_consumer.rs`) compiled and ran with an empty trait, a two-method trait, an infallible implementation adapted to `TryWitnesses<u64>`, and a direct fallible implementation returning `Err` for one seed. A deliberately wrong user return type failed with E0053 at the user impl line and pointed to the original trait signature. `rustdoc` emitted both `trait.Witnesses.html` and `trait.TryWitnesses.html`.

This establishes proc-macro syntax, trait coherence, basic spans and rustdoc visibility in isolation. It does not prove compilation of the 58 actual ABI-15 witness signatures, ledger projection semantics, packaged proof parity, or clean/warm generated-crate timing. The prototype is outside the repository and has no commit.


### Local implementation and verification — 2026-10-03

Local conventional GPG-signed/DCO commit `34ff328e839581de6f5ab4910bb0e06439f829f2` implements the attribute bridge in the AST backend. The generated source now declares one ordinary, documented `Witnesses<Private>` trait with `#[runtime::compact_witness_bridge]`; the `runtime-rs-macros` expansion produces the same public `TryWitnesses<Private>` signature list and blanket infallible adapter. It validates the narrow generated grammar and returns spanned errors for unsupported bounds, defaults, method bodies, receivers, return shapes and signature qualifiers. The runtime crate reexports the macro. `witness.rs` owns Compact declaration types and emits each signature once; the macro owns only Rust mechanical duplication. No VM, gas, FAB, transcript, state or private IR schema logic changed. Private IR stays schema 8; generated/runtime ABI is now 16.

Actual generated `witness-cell-write` source changed from:

```rust
pub trait Witnesses<Private> { fn secret(/* typed arguments */) -> (Private, Field); }
pub trait TryWitnesses<Private> { fn secret(/* typed arguments */) -> Result<(Private, Field), CompactError>; }
impl<Private, W: Witnesses<Private>> TryWitnesses<Private> for W { /* forward */ }
```

to:

```rust
/// Implement for infallible callbacks; use TryWitnesses for fallible ledger reads.
#[runtime::compact_witness_bridge]
pub trait Witnesses<Private> {
    fn secret(&self, context: WitnessContext<'_, Private, LedgerView<'_>>, seed: Field)
        -> (Private, Field);
}
```

The explicit `TryWitnesses` source is now proc-macro output; consumer implementations and generated circuit bounds stay source compatible at the trait API level. The compile-time ABI assertion intentionally rejects an ABI-15 generated crate paired with ABI-16 runtime, so the earlier conditional compatibility sentence in this ADR does not apply to this delivery. Regenerate the crate or use its matching ABI-15 runtime.

All 132 checked-in generated fixture libraries were regenerated with the pinned Scheme compiler and subsequently passed the no-update freshness check (132 current, 0 stale, 0 failed). Across those libraries, total lines fell by 965. Representative actual source counts: `witness-cell-write` 413→394, `election-oracle` 1128→1035, `zerocash-oracle` 774→668, `counter` 189→188. The repository commit changes 141 files, with 764 additions and 1,466 deletions including macro/tests/docs. This is a generated source reduction and reviewability result, not a compile-time speed result.

Focused validation passed: macro unit tests 5/5, runtime witness bridge integration tests 2/2, AST renderer tests 55/55, `witness-cell-write` 10/10, election 1/1, zerocash 2/2, nested witness 7/7, counter 4/4, `cargo fmt --all --check`, fixture freshness, and staged diff checks. A separate Cargo project depending only on the generated `witness-cell-write` crate passed `cargo check --offline`, typechecking both infallible and direct fallible implementations and the borrowed `Contract::recording()` facade. Its 8m22s wall time included ledger dependency compilation and has no same-head ABI-15 comparison; no compile-time improvement is claimed. The user-owned `doc/ledger-adt.mdx` change was untouched.

The parent task owns the final rebuilt Nix compiler `--consumer --proof` gate against this commit. Independent TypeScript parity and 57-call proof/application evidence exists for the unchanged execution paths on the preceding ledger-8.0.3 baseline, but the ABI-16 commit does not claim a fresh packaged proof result until that gate completes. Publication, remote CI, package release, IDE expansion review and clean/warm same-host compile-time comparison remain open. The branch has not been pushed.


### Exact-head packaged gate amendment — 2026-10-03

At signed/DCO commit `34ff328e839581de6f5ab4910bb0e06439f829f2`, the Nix compiler shell rebuilt the Rust backend CLI and `compactc` wrapper from the ABI-16 source. In that shell, `COMPACTC=compactc python3 tools/compact-rust-backend/check_compactc_target.py --consumer --proof` exited 0. Separate generated-crate consumers and compile-fail checks passed; proving artifacts compiled; all 57 offline ledger-8.0.3 generated calls proved, verified, validated and applied. This establishes the macro expansion preserves the tested recorded, native, witnessed, state and public/private execution paths. The source worktree retained the unrelated user-owned `doc/ledger-adt.mdx` edit. Remote CI, registry publication, unpatched released consumer, funded current-TTL wallet/node submission and release candidate provenance remain open. No branch push occurred.

### ABI-16 release archive rehearsal — 2026-10-03

`check_release_packages.py --manifest target/rust-runtime-release-abi16.json` packaged and compiled the unpacked `midnight-compact-runtime-macros` and `midnight-compact-runtime` 0.1.0 archives at commit `34ff328e`; `--verify-manifest` repackaged both and matched all recorded hashes. Macro archive SHA-256 `e4b5acd287cd4950f4d39681e8aaaf57d0da7df3343ae90069bc8586921ea521` (10,907 bytes, 9 entries); runtime archive SHA-256 `127d9160f2313ad3e84eddd7dd1519c7b90559a49a23b7253c6f6e7ee7673cbd` (155,642 bytes, 230 entries). The manifest reports `dirty: true` because of the unrelated pre-existing `doc/ledger-adt.mdx` edit and uses a local macro patch while the exact macro version remains unpublished. There is no signed clean release tag, registry publication, unpatched registry consumer or remote CI result.
