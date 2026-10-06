---
id: RUST-ADR-0181
alias: ADR-0181
title: "Checked native wide unsigned addition"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-native-only"
topics: ["compiler", "unsigned", "arithmetic"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: e353a9409dfa7c3cb8209cce1581064396a1ac1c833819ee88a38a4301659b8c
---
# RUST-ADR-0181 — Checked native wide unsigned addition

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-native-only. Wide unsigned addition reuses checked bounded and limb-backed values without an extra bigint dependency. The witnessed export is proof-not-applicable and no recorded proof API is asserted for this slice.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#285 closure](https://github.com/MediaNoxLabs/compact/issues/285#issuecomment-6017713263). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`7133d39f`](https://github.com/MediaNoxLabs/compact/commit/7133d39fca95d5d0f454c80892dafc2cda49dd00). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
status: accepted
date: 2026-10-05
```

## Historical decision and amendments

### Problem
Original micro-dao/Coracle mergeCoin evaluates (a.value + b.value) as Uint128. Actual typed IR first casts both inputs to maximum680564733841876926926749214863536422910, adds with maximum680564733841876926926749214863536422911, then checked-narrows. Arithmetic emitters currently parse result maxima asu128, although runtime WideUint already supports checked248bit values and upstream FAB/FieldRepr/hash codecs.

### Before / after
```compact
return (a.value + b.value) as Uint<128>;
```
Before: compile-time rejection of the129bit intermediate. After: checked native addition preserves carry and returns the declared WideUint, then existing narrowing rejects a value aboveUint128.
```rust
let sum = runtime::add_wide_unsigned::<1, MAX128, _, _>(left, right)?;
let value = runtime::narrow_wide_uint::<MAX128, 1, MAX128>(sum)?;
```

### Emitter/runtime changes
Centralize arithmetic syntax for pure/stateful expressions; validate every maximum with the existing canonical248bit parser and every actual operand type. Reuse existing WideUint and a sealed unsigned operand view implemented by BoundedUint/WideUint. Add checked low-limb carry plus checked high-limb addition, then validate the declared output bound. No modular Field arithmetic or bigint public carrier. Keep existing small arithmetic behavior. Wide subtraction/multiplication and unsupported output/type combinations fail closed. ABI46/schema20.

### Tests and boundaries
Independent TS/native cases cover limbcarry, mixed widths, maximum129 and out-of-range max129+1, exact2*u128MAX, checked narrowing rejection, stateful witness ordering and selected branches/private outputs/gas/state. Unit tests exercise impossible-public-domain highlimb overflow in the internal checked helper. Malformed maxima/types and wide subtraction/multiplication receive renderer guards. Record unchanged original-source progress and remaining diagnostic. Native-only claim; recording/proof/funding separate. Reuse existing warmtarget; no broad oldfixture refresh, remoteCI or push.


### Implemented scope and source evidence
Issue https://github.com/MediaNoxLabs/compact/issues/285 in rust-backend-v2. Reused existing two-limb WideUint and upstream codecs without another bigint dependency. Sealed UnsignedOperand permits only checked BoundedUint/WideUint operands; arithmetic helper checks low carry and high addition before validating the exact result bound. Ordered comparisons with wide operands now reject explicitly instead of emitting an unavailable .value() call; wide subtract/multiply also fail closed. Fourteen independent TS cases pass (including three checked-narrowing rejections); runtime8library+3wide-domain tests and backend8library+145renderer tests pass. Unchanged micro-dao advances to line187 stateful expression requires stateful evaluation, saved ${LOCAL_EVIDENCE}/compact-adr181-original-dao.stderr. This isolated branch lacks ADR174 Effectful return IR, so combined Coracle evidence is deferred to parent integration; do not claim its source compiles.


### Signed delivery receipt
GPG+DCO commit eb0a640300a787fe00bb1460437d29ac11740ca5. Frozen receipt ${LOCAL_EVIDENCE}/compact-focused-eb0a6403-wide-add/receipt.json passed1fixture0/1recorded; witnessed is proof-not-applicable, four other exports pure. All14independent TS cases, runtime8library+3wide-domain tests, backend8library+145renderer tests, targetedClippy with warningsdenied, sourceguards179/181 and onefixturefreshness pass. ABI46 requires parent-owned combined regeneration of oldfixturelibs; no oldlibs manuallyedited. FrozenCLI ${LOCAL_EVIDENCE}/compact-adr181-compactc paired with unchanged Scheme ${HISTORICAL_NIX_STORE}/c6cnhvmbjy14wwamkp7jbk4s4v75ybfg-compactc-binary-nixos/bin/compactc-scheme. No push/remoteCI/proofclaim. Next microDAO line187 is nested Expr::Assert falling into pure rendering despite short-circuit Cell/Counter reads; separate follow-up.


### Main integration evidence

Integrated signed3c22f479; ABI46/schema20 fixture refresh7133d39f. All164fixtures fresh (163updated); backend8+146 renderer and10runtime units pass. Frozen six-source receipt ${LOCAL_EVIDENCE}/compact-focused-7133d39f/receipt.json passes. Exact inventory ${LOCAL_EVIDENCE}/compact-7133d39f-inventory.json:207sources/727exports/184compiled roots;333/347 assessed proof APIs,14known gaps,20unassessed;360nonproof exports; zero missing/unmatched rows and empty baseline drift. Wide addition uses checked two-u128-limb arithmetic and existing WideUint validation, with no Field modular shortcut. Wider subtraction/multiplication/ordered comparison remain separate unsupported cases.
