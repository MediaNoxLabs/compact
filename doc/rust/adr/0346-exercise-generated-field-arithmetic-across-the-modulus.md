---
id: RUST-ADR-0346
alias: ADR-0346
source_sha256: 9ce09fbc5b19dcba28ce0c5b3a343785f522940e7da87b4cec1288755b14d7be
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0346 — Exercise generated Field arithmetic across the modulus

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted for focused implementation. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0346 — Exercise generated Field arithmetic across the modulus

Date: 2026-10-08
Status: accepted for focused implementation
Parent: R03007 / #351; branch codex/rust-backend-ast at 5a9cd356

### Problem

The maintained generated field-arithmetic fixture checks small multiplication (7×6=42), small subtraction and 0−1 against runtime negation. It does not independently assert full-width modular multiplication through the generated API. ADR0344 covers compiler literal admission at the modulus; it does not execute generated arithmetic.

### Decision and before/after

Before: `multiply(Field::from(7), Field::from(6)) == Field::from(42)` and `subtract(0,1) == -Field::from(1)`.
After: retain those cases and add generated multiplication/subtraction assertions against explicit little-endian byte values derived from the pinned upstream modulus: `(p−1)*(p−1)=1`, `(p−1)*2=p−2`, and `0−1=p−1`. Additional zero/identity and subtraction order checks may share those independently pinned values where useful.

Input and expected byte constants come from midnight-curves 0.2.0 Fq, selected by midnight-transient-crypto 2.0.1 outer::Scalar. Do not compute expected values with runtime multiplication/subtraction/negation. Preserve explicit arithmetic meaning and readable case names. No new dependency is required.

### Ownership and alternatives

Only the hand-maintained generated consumer integration test changes. Compact source, generated library, emitter, runtime, API/ABI, dependency graph and upstream pins remain unchanged. This is a generated arithmetic regression check, not a cryptographic implementation change, fuzz campaign, new TS oracle, proof or general field audit. Reusing the compiler literal test would not exercise this boundary; copying runtime arithmetic into the expected value would weaken independence.

### Verification

Confirm upstream constants and exact lock identity; run the one integration target under the existing warm local Rust 1.99 cache, focused strict Clippy and rustfmt. Record source/lock/tool/command hashes. Independent subagent review checks constants, assertions and claimed scope. Broaden only if the focused result reveals a concrete problem. No new line-coverage percentage is claimed. Preserve user-owned doc/ledger-adt.mdx. Planning/history stay in midnight until milestone closeout.

ADR0285/#409 remains outside this work. Dependency review #469 can proceed independently. This child does not close #351 or increase the 12/19 parent count.


### ADR0346/#470 generated Field arithmetic — 2026-10-08

Signed GPG/DCO `f352627515e39ce100fdb2d66bbdae910b4deb0c` adds one integration method with three independent canonical-byte assertions: `(p−1)²=1`, `2(p−1)=p−2`, `0−1=p−1`. Two tests pass; scoped strict Clippy, formatting and owned diff check pass. Independent subagent review reports no findings. Upstream modulus provenance and 458 stable gate input hashes retained. No product/generated/dependency change or new TS/proof/coverage claim.

[ADR0346 — Generated Field modular arithmetic](references-0.3.0.md#note-121) and [ADR0346 — Generated Field modular arithmetic.zip](references-0.3.0.md#note-122) retain commands, logs, exact test, patch, source/commit bindings and review. Archive SHA256 `96c4576aa5d81f8d50a5fb38f1a6e04132e575f2c21bc774ebfa897015b261e3`. Parent #351 remains open; milestone count stays12/19. Dependency review#469 remains independent and pending Claude.

Issue: https://github.com/MediaNoxLabs/compact/issues/470
Status: delivered; parent acceptance unchanged.
