---
id: RUST-ADR-0050
alias: ADR-0050
title: "Record read-only scalar return expressions"
date: 2026-10-04
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "recording", "ledger-adts"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: f74134dcf29d6b36fe0f7f8c7a14d8ae737a1ac7b6fa50cad47cb53ae50b8383
---
# RUST-ADR-0050 — Record read-only scalar return expressions

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept supported action-free Boolean/Field return expressions that perform actual recorded observations, preserving ordered reads and typed results. The clean historical ABI 28 candidate evidence is separately dated. Capability of this bounded expression lowerer does not imply arbitrary pure or effectful expression recording.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#149 closure](https://github.com/MediaNoxLabs/compact/issues/149#issuecomment-6017482664). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`0eec27ef`](https://github.com/MediaNoxLabs/compact/commit/0eec27ef86d2da9c98c933e513ebcc8f2db224ff). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 50
status: accepted-partial
date: 2026-10-04
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/149
```

## Historical decision and amendments

### Problem and evidence

A source contract with chunked Boolean Cell `active` at `[1,14]` and Field Cell `amount` at `[1,13]` compiles and runs native `active_equals(expected): Boolean { return active.read() == expected; }` and `plus_amount(delta): Field { return amount.read() + delta; }`. Fresh packaged ABI-27 `compactc --target rust --skip-zk` emits no recorded methods or observed call builders for either. The existing `boolean_expression` and `field_expression` lowerers can already record Cell reads inside assertions and writes; the exported `StateReturn::Expression` match accepts only a Field helper body or a few special shapes. This is an emitter API gap, not a missing ledger primitive.

### Before and proposed after

Before:

```rust
let native = contract.active_equals(context, true)?;
let sum = contract.plus_amount(context, Field::from(7_u64))?;
// Neither contract.recording.active_equals_call nor plus_amount_call exists.
```

Proposed after:

```rust
let recorded = contract.recording.active_equals(context, true)?;
let prepared = contract.recording
    .active_equals_call(&observed, (), true)?
    .prepare(verifier, randomness)?;
let sum = contract.recording.plus_amount(context, Field::from(7_u64))?;
```

### Decision proposal and alternatives

Allow action-free exported Boolean and Field `StateReturn::Expression` circuits through the existing typed scalar expression lowerers when lowering produces a real recording step. Preserve their source-order frame reads, scalar result type and failure behavior. Continue to reject unsupported expression shapes with no generated recorded method. Do not build a second VM sequence or generated raw-op DSL. Keeping native-only output would force the user back to `RecordingFrame` for ordinary read-and-return calls. A broad untyped expression escape hatch would weaken the AST acceptance boundary. Bump generated/runtime ABI 27→28 for new public methods; keep private IR schema 8.

### Emitter and runtime ownership

`tools/compact-rust-backend/src/recorded.rs` owns exported return-shape eligibility and reuses `boolean_expression`/`field_expression`; `src/lib.rs` owns ABI and generated observed methods. `CellSlot<T>`, `RecordingFrame`, `ledger/cell.rs` and the upstream ledger-8 VM/FAB/gas primitives own runtime semantics. No new runtime VM, derive or macro is proposed. Keep the internal Field helper branch and shared-callee behavior consistent; do not accidentally generate public APIs for unsupported pure-only expressions.

### Verification and risks

Check in Boolean equality and Field arithmetic return circuits over chunked Cells, capture TypeScript result/state, ordered VM transcript, input FAB atoms/alignment and four-dimensional per-query gas, then compare Rust native/recorded/replayed execution. Test generated-only consumer compilation and wrong-type rejection. Compare manual/generated pre-proof bytes and independently prove, verify, validate and apply both calls from a fresh packaged compiler. Check root and chunked Cell regression fixtures, all fresh outputs, source size, build impact and release archives. Risk: lifting a return-shape guard may expose other supported expression combinations; inspect every newly emitted method and keep the acceptance test focused on actual complete traces. Remote CI, clean release, registry and wallet/node remain separate gates.

### History

- Baseline probe: ignored `target/direct-return-cell-probe.compact`; packaged ABI-27 compiler emits native methods but no `recording.*_call` for `active_equals` and `plus_amount`.
- Related: ADR-0001/0006 for complete replayable calls, ADR-0034 for Field helper sharing, ADR-0049 for chunked Cell reads.
- Proposal only. Append dated implementation/verification evidence with signed/DCO commit rather than rewriting this rationale.

### Design refinement before delivery — 2026-10-04

An initial local emitter probe that removed the exported Field helper guard and added a Boolean branch generated the two intended Cell-return APIs **and six additional witness-return observed builders** in unrelated fixtures. Those witness-only calls have different private transcript and proof acceptance needs. The delivery scope is therefore explicit Cell-read return expressions: a typed IR visitor detects a Cell read through supported scalar composition, while the existing private shared Field helpers remain eligible without that public condition. This refines the proposal's “real recording step” gate to a concrete source-level Cell read. The six witness-return candidates are deferred for separate review rather than silently exposed. No earlier rationale is deleted.

### Implementation and verification amendment — 2026-10-04

Decision accepted for exported action-free Boolean and Field return expressions with an **explicit Cell read**. Signed/DCO local commit `0eec27ef86d2da9c98c933e513ebcc8f2db224ff` adds a typed IR `contains_cell_read` eligibility visitor and reuses the existing `boolean_expression`/`field_expression` lowerers. The prior private Field helper and special `if` return paths retain their earlier eligibility. Exactly two new public observed builders appear in the 137-fixture diff; the six witness-only candidates from the first probe remain unexposed. Generated/runtime ABI is 28; private IR schema stays 8. No runtime VM, ledger primitive, FAB encoder, derive or macro code changed.

The delivered developer-facing API is:

```rust
let equal = contract.recording.active_equals(context, true)?;
let call = contract.recording
    .active_equals_call(&observed, (), true)?
    .prepare(verifier, randomness)?;
let sum = contract.recording.plus_amount(context, Field::from(7_u64))?;
```

The checked-in chunked Cell source now has eight circuits. Boolean `active_equals` reads the `[1,14]` Cell and returns equality with its declared Boolean argument. Field `plus_amount` reads the `[1,13]` Cell and returns its sum with a declared Field argument. TypeScript ledger-8 and Rust fixture results, serialized state, ordered public VM transcript, input FAB atoms/alignment and all four per-query gas dimensions agree. Native, recorded, replayed and metered witness paths remain consistent; generated-only consumer builds both calls, and wrong Boolean/Field arguments are rejected.

A fresh packaged `compactc` at `${HISTORICAL_NIX_STORE}/65mg76yq8qdlaw1ql883lavr6galazjs-compactc` passed the complete generated consumer/manifest and **93-call independent proof, verification, validation and application gate** after the final private-helper preservation fix. Manual and generated calls match byte-for-byte before proof: `active_equals` 491 bytes and `plus_amount` 519 bytes. The focused fixture, 57 renderer tests, bounded all-features proof-smoke check, fresh 137-fixture comparison and formatting pass. A full all-features workspace check was not rerun after ABI 28 because broad parallel compilation had caused disk pressure in the previous slice; no full-workspace result is claimed. The generated chunked Cell source is 733 lines / 33,768 bytes versus ABI-27's 593 lines / 27,304 bytes, but two circuits and their public methods were added, so this is not a like-for-like optimization measurement. Compile-time impact was not measured.

The runtime macro and runtime crate archives package and verify with 9 and 247 entries, and a second run reproduced `target/rust-runtime-release-abi28.json` at `0eec27ef`. Its dirty flag reflects the pre-existing user-owned unstaged `doc/ledger-adt.mdx` change. The branch remains local/unpushed. Witness-only scalar returns, other expression shapes, remote CI, a clean signed release, registry publication and wallet/node submission remain open; issue #149 stays open for those external gates.


#### Full workspace gate amendment — 2026-10-04

After the signed commit and archive rehearsal, `CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2 cargo check --workspace --all-features --offline --quiet` completed with exit 0 at `0eec27ef`. It compiled the full current workspace, including all checked-in Rust fixture crates, while free disk space stayed near 12 GiB. This supersedes the earlier “full workspace unverified” caveat for the local ABI-28 head only. Remote CI and clean release gates remain open.


#### Clean candidate provenance amendment — 2026-10-04

The same signed/DCO ABI-28 commit now has local GPG-signed annotated candidate tag `rust-backend-v2-abi28-rc1`. A clean managed checkout passed release archive write/re-verification with `dirty: false`, and clean `nix build .#compactc` resolved to the exact package path used by the 93-call proof gate. Full hashes and remaining publication limits are in [ADR-0007 — Bundle the matching runtime until a versioned release exists](0007-bundle-the-matching-runtime-until-a-versioned-release-exists.md) and [Milestone 2 — ADR delivery map](references.md#private-note-08). The primary checkout's user-owned documentation edit remains untouched.
