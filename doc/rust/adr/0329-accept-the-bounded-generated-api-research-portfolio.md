---
id: RUST-ADR-0329
alias: ADR-0329
source_sha256: 45c4657d54dda63394f5cb791962a0b5a1362324bf9935fa2ab9f2e70b86cbc4
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0329 — Accept the bounded generated API research portfolio

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** locally accepted; 2026-10-07; R03002/#346. Source checkpoint bf7c719beea14f7532feaf0d57577245f42c98ca. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0329 — Accept the bounded generated API research portfolio

Status: locally accepted; 2026-10-07; R03002/#346. Source checkpoint bf7c719beea14f7532feaf0d57577245f42c98ca.

### Problem and decision
The approved research program asks for measured choices, repeated review after adoption, and human-friendly generated APIs. It does not require promoting a macro/DSL merely because it was prototyped. Existing probes now have a post-DID public API review, a compiled ordinary named-input recipe, current default-worker/effect tests and a qualified cost baseline.

Accept the current local research/design outcome. Retain positional generated APIs, bounded existing CircuitFrame helpers and mechanical derives. Keep generated Args promotion deferred and BindingId/Rc alternatives rejected under their existing ADRs. No new public abstraction-level flag or role inference is introduced. Final frozen-candidate public API/consumer/cost-drift review remains required under R03020/#364.

### Before / after developer choice
```rust
// Generated API remains source-shaped:
pure_circuits::assertValidDigitalPassportAgePredicate(
    credential, presentation, current_day, birth_day, opening, current_date, birth_date,
)?;
// Optional application-owned record makes a repeated call easier to read:
AgePredicateInputs {
    credential, presentation, current_day,
    date_of_birth_days: birth_day, date_of_birth_opening: opening,
    current_date, date_of_birth_date: birth_date,
}.evaluate()?;
```
The full typed wrapper is in the current passport developer guide and exact external recipe archive. It consumes its fields, forwards source order and preserves contract errors. Same-typed role swaps remain possible. This application pattern changes neither emitter/runtime implementation nor ABI.

### Evidence and measured limits
Independent agent acceptance review reverified1799 artifact/log hashes with zero mismatches and five public/emitter surfaces byte-identical to accepted dce3 baseline. Historical frame/Args/derive/lexical probes retain before/after output, ordinary alternatives, diagnostics and costs. Current679-owner local and Linux test cohorts preserve worker/effect controls;197fixture renders are unchanged. Witnessed/DID external tutorials pass3tests with unedited output; passport named-input recipe executes25storedTS cases,6successes/19exact refusals. Accepted ADR0317 portfolio baseline remains source-scoped.

No numerical Args-specific stack peak or runtime latency was measured. The rejected/unpromoted prototype has displayed/expanded/build/diagnostic costs; ordinary-worker passes and portfolio RSS/heap data are separate dimensions. No Args speedup, universal stack bound, cold-dependency timing or TS performance advantage is claimed. Its extra surface and roughly20% displayed-source growth already support deferral; another experiment without a concrete usability hypothesis is unwarranted. This scoped research acceptance does not turn unmeasured dimensions into measurements.

### Final candidate ownership
R03020 must recheck exact final API/source/lock/toolchain/platform identities, clean external consumers, affected semantic/diagnostic gates, full applicable local/remote suites, migration/package/audit results and any matching-candidate cost drift. Publish ADR/probe history and documentation at closeout. Do not refresh rejected historical prototypes only to rewrite their provenance. Parent audit/coverage/security gates remain open, including stoppedADR0285.
