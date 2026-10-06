---
id: RUST-ADR-0036
alias: ADR-0036
title: "Unify contract recording access across witness shapes"
date: 2026-10-03
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["architecture", "generated-api"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: e0a4ae852aa9c24b258770a55302e2a7a802438fb4653293af1d5a1f1e69b7ec
---
# RUST-ADR-0036 — Unify contract recording access across witness shapes

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept additive recording() access for witness-free facades while retaining the old public field, matching the consumer call shape of witness-bearing facades. The different typed borrowed/owned handles remain intentional. This is an API consistency change without new trace semantics or a claim that every circuit is recordable.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#135 closure](https://github.com/MediaNoxLabs/compact/issues/135#issuecomment-6017458188). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`20fdf358`](https://github.com/MediaNoxLabs/compact/commit/20fdf3586f6f57f29d297a9f617fa70a98981276). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 36
status: accepted-partial
date: 2026-10-03
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/135
```

## Historical decision and amendments

### Problem

The generated `Contract<W>` facade exposes the same recorded operation through two different syntaxes based on whether any recorded circuit needs witnesses. A no-witness contract such as `counter` uses the public field `contract.recording.increment(context)`. A witnessed contract such as `nested-witness-call-oracle` uses `contract.recording().outer(context)` so the returned handle borrows `W`. A consumer with several generated contracts must inspect each generated crate before choosing the access syntax, even though both are replayable circuit handles. On current ABI-15 fixtures, 57 generated crates have recorded calls; 53 lack the `recording()` method.

### Before

```rust
let counter = CounterContract::default();
let incremented = counter.recording.increment(counter_context)?;

let witnessed = WitnessedContract::from(MyWitnesses);
let called = witnessed.recording().outer(witnessed_context)?;
```

### Decision and after

**Proposal.** Give no-witness generated contracts an additive `recording()` method returning a reference to their existing typed `recorded::Contract` field. Keep the field for compatibility. Witnessed contracts keep their current `recording()` method returning `recorded::BorrowedContract<'_, W>`. Users can use one call shape in both cases; the receiver and recorded result remain statically typed.

```rust
impl<W> Contract<W> {
    pub fn recording(&self) -> &recorded::Contract {
        &self.recording
    }
}

let incremented = counter.recording().increment(counter_context)?;
let called = witnessed.recording().outer(witnessed_context)?;
```

The return types differ because only the witnessed handle needs `&W`; the consumer-facing method call is uniform. Rust permits a field and method with the same name, as confirmed by a compiled standalone shape probe. The existing `contract.recording.increment(...)` syntax remains valid.

### Alternatives and rationale

A generated contract-specific witness-context type alias was also probed. It replaces a verbose generic type but adds three generated lines per crate and leaves most formatter-wrapped signatures unchanged; it does not address the inconsistent contract call surface. Generating `BorrowedContract` for no-witness crates duplicates every recorded method and substantially grows output. Removing the public `.recording` field would break existing consumers. The additive method costs one short method per affected crate and preserves static types and frame ownership.

### Emitter and runtime ownership

`tools/compact-rust-backend/src/lib.rs` owns the `Contract<W>` facade. When `recorded_items` is nonempty and no recorded circuit uses witnesses, emit `recording(&self) -> &recorded::Contract`; when witnessed recording exists, preserve the current borrowed witness method. The typed IR (`schema 8`), `recorded.rs` completeness check, runtime `RecordingFrame`, ledger-8 VM builder, derives and procedural macros are unchanged. This is an additive public generated-crate convenience, with no runtime dependency or behavioral change; the runtime/generated ABI assertion may remain 15 because existing generated code and runtime types continue to interoperate. Review that decision if the final emitted type changes beyond this method. No migration is required.

### Verification and risks

The real checked-in `counter` generated file is 185 lines. A `/tmp` copy with the proposed method formats to 189 lines (+4); a minimal Rust field-plus-method shape compiles. The full generated-copy direct typecheck timed out under host load, so no full generated compile result is claimed yet. A successful implementation should regenerate fixtures and measure how many change, assert the method in a renderer test, execute `counter.recording().increment(...)` in the fixture consumer, and prove that both old field syntax and the new method syntax compile and return identical native context/gas/Verify behavior. Keep witnessed `recording()` calls working. Run fixture freshness, focused tests, external consumer and the existing packaged proof/application gate; measure same-head warm Cargo checks for counter and a witnessed crate, noting host noise. No speed or source-size improvement is expected; report the added lines and compile cost honestly. Negative type behavior should remain unchanged: only complete recorded circuits appear on the handle.

### Tracking and delivery

- Focused issue: pending MediaNoxLabs/compact issue, assigned to [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2) before implementation.
- Related: [ADR-0003 — Expose a contract facade and one-dependency consumer crate](0003-expose-a-contract-facade-and-one-dependency-consumer-crate.md), [ADR-0016 — Reuse recorded callee bodies within one frame](0016-reuse-recorded-callee-bodies-within-one-frame.md), [#103](https://github.com/MediaNoxLabs/compact/issues/103), [#117](https://github.com/MediaNoxLabs/compact/issues/117).
- Local commits: none. Branch local/unpushed.
- Delivery: proposed; no renderer, runtime or generated fixture edit yet.

### Amendments

Append the issue URL and actual generated code, tests, size and timing after implementation. Preserve this proposal.
### 2026-10-03 implementation evidence

Focused issue [#135](https://github.com/MediaNoxLabs/compact/issues/135) is assigned to rust-backend-v2. The additive no-witness accessor is implemented in the AST facade emitter. The generated `counter` example now has `pub fn recording(&self) -> &recorded::Contract { &self.recording }`; its old public field remains. The witnessed `recording() -> recorded::BorrowedContract<'_, W>` shape is unchanged. If an exported source circuit itself is named `recording`, the new accessor is omitted to avoid duplicate inherent methods, and callers use the existing field. The pre-existing witnessed variant of that name collision remains outside this slice.

Generated source: 53 of 132 fixture libraries changed, each by exactly four added lines: 212 lines total and no removals. Counter grew 185 → 189 lines (+2.2%). The goal is one ordinary access syntax across the 53 no-witness fixtures and witnessed fixtures, not source-size reduction. Schema 8, runtime, and public ABI 15 are unchanged.

Validation on the local uncommitted checkout:
- `cargo test -p compact-rust-backend --test render --locked`: 55/55 passed. After collision fallback, the focused renderer case passed 1/1.
- `cargo test -p compact-rust-counter-fixture --locked`: 4/4 passed. Its new consumer test calls both access forms and compares ordered Verify operations, state, effects, four-dimensional gas and private outputs; after the final assertions, the focused case passed 1/1.
- `cargo test -p compact-rust-nested-witness-call-oracle-fixture --locked`: 7/7 passed, retaining the borrowed witness handle.
- `COMPACTC=${HISTORICAL_NIX_STORE}/hyhgnx3c139h4z4fln10ggqqq1i8hhym-compactc/bin/compactc-scheme python3 -B tools/compact-rust-backend/check_fixture_outputs.py`: 132 checked, 0 stale, 0 failed after the collision fallback.
- `PATH=${HISTORICAL_NIX_STORE}/015aw3x0q4snnzah5zakm9ybfs8jmavn-zkir-2.2.0/bin:$PATH COMPACTC_SCHEME=${HISTORICAL_NIX_STORE}/25fizykx9qg9m2752j3lays42mj5wx57-compactc/bin/compactc-scheme COMPACTC=$PWD/target/debug/compactc python3 -B tools/compact-rust-backend/check_compactc_target.py --proof`: passed the existing 57-call offline replay/proof/validation/application gate. This ran before the collision fallback; fixture output remained byte-identical after it.
- `cargo fmt --all --check` passed. Scope-limited `git diff --check` passed. Whole-tree `git diff --check` reports an unrelated pre-existing trailing blank line in `doc/ledger-adt.mdx`.

Compile behavior: the first counter test build reported 55.20s, its later incremental build 6.17s, and the unchanged witnessed crate build 0.26s. These are host-load samples, not a same-head before/after benchmark. No compile-speed claim is made. The separate clean consumer target and remote CI remain unrun; the full proof gate used the existing target because free disk was about 22 GiB. No commit or push has been made for this slice.

### 2026-10-03 commit checkpoint

Local conventional commit `20fdf3586f6f57f29d297a9f617fa70a98981276` records the bounded implementation. `git log -1 --show-signature --format=fuller` verifies a good GPG signature and `Signed-off-by` DCO trailer. It changes 56 files: the AST facade emitter, renderer tests, counter consumer test, and 53 generated fixture libraries. No push or remote CI is claimed. The only remaining working-tree change is the pre-existing `doc/ledger-adt.mdx` edit, excluded from this commit.
