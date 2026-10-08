---
id: RUST-ADR-0322
alias: ADR-0322
source_sha256: a533b3a00bcd6df358ed9030f280afce670a25043b532f49ca90bc2c9f5efa89
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0322 — Preserve pure-helper admission through conversion wrappers

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** delivered bounded test slice · 2026-10-07 · R030-07/#351 · milestone 0.3.0. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0322 — Preserve pure-helper admission through conversion wrappers

Status: delivered bounded test slice · 2026-10-07 · R030-07/#351 · milestone 0.3.0

### Problem
Recorded helper admission already rejects ledger reads, witness calls and unresolved parameters in scalar bodies. Existing focused tests do not sufficiently establish that coercion wrappers preserve these checks for both Field and Unsigned subtraction. A wrapper must not hide effects or bypass lexical admission.

### Before / after
```text
Before: direct arithmetic-body admission tests
After: Field Coerce(Subtract(left,right)), Unsigned Coerce(Subtract(...))
       and UnsignedCast controls admit valid scalar bodies
       but reject hidden effects/unknown operands in either position
```
Also reject wrong target domains and verify recursive visiting state is restored after refusal. These are real admission-owner tests, not synthetic coverage-only calls.

### Ownership and validation
Only recorded/pure_calls/tests.rs changes; no runtime, production emission, generated API, IR or ABI change. Delegate implements tests, root reviews; local focused tests and strict Clippy, then fresh instrumented library coverage with source-identical Boolean hit union and frozen denominator. A numerical floor does not replace semantic/case/constructor evidence. Keep stopped ADR0285 lane untouched. ADR->issue->milestone->signedDCO delivery; no push/CI before actual tenth-parent acceptance.

Delivery issue: https://github.com/MediaNoxLabs/compact/issues/447


### 2026-10-07 — Pure helper conversion admission (ADR0322/#447)

Signed GPG/DCO `c924a4e115cdd56e276e35405d80f233860dfbf6` adds two real admission-owner methods. Field/Unsigned coercion and cast wrappers around transitive subtraction accept valid scalar bodies and reject hidden effects, caller-only parameters, missing helpers, recursion and wrong coercion domains in either operand. Visiting state is clear after refusal and a valid retry succeeds. These are policy assertions, not execution of numeric conversions. Delegate implemented; root reviewed.

203 fresh instrumented backend library tests, strict library/test/all-feature Clippy and rustfmt pass. Source-identical Boolean union now **8611/9060 changed backend lines=95.04%**, total mapped22165/24020=92.28%. Nine additional changed lines, no new production mapping or denominator change; frozen test exclusions preserved. Numeric95%floor passes. Parent #351 remains open for the finite semantic/effect and required source/export evidence; no whole-current-tree coverage claim. Other owner percentages keep prior cohort scope.

Evidence [ADR0322 — Pure helper conversion tests and coverage.zip](references-0.3.0.md#note-104), SHA256 `8e077a64927e6b36529acb4b71a79fb50cfc4706fdf97e4a622f18e849968831`. #447 closes bounded test slice. No production source/API/ABI/output changes, push or CI. User doc preserved.
