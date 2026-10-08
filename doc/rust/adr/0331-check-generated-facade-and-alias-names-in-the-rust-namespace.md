---
id: RUST-ADR-0331
alias: ADR-0331
source_sha256: d968eb3af282a8f18838bd6fa87eaca015ad6423e619035150eb59512f1ce2a8
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0331 — Check generated facade and alias names in the Rust namespace

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** planned remediation; 2026-10-07; independent audit ADR0327/#452 and R03013/#357; milestone rust-backend-v0.3.0. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0331 — Check generated facade and alias names in the Rust namespace

Status: planned remediation; 2026-10-07; independent audit ADR0327/#452 and R03013/#357; milestone rust-backend-v0.3.0.

### Problem
ADR0327 findings2/3/4 reproduced with valid Compact source: foo+foo$call creates duplicate observed helper under ledger-transaction; witnessed export recording collides with the convenience accessor under default features; pure$circuits alias and a$b/a_b aliases collide with emitted module/type names. Generated consumers fail E0592/E0255/E0428. Raw Compact-name comparisons disagree with the emitter normalization.

### Before / after
```text
Before: "foo$call" != "foo_call"; guard allows two Rust foo_call methods.
After:  compare normalized semantic Rust identifiers before advertising helpers.
Before: witnessed recording accessor ignores exported recording method.
After:  preserve source export; emit convenience accessor only when name is free.
Before: raw aliases checked against raw types/modules.
After:  one namespace normalization checks aliases/types/reserved modules.
```
Centralize semantic identifier normalization in existing naming owner. Use it for observed helper collision, both witnessed/unwitnessed accessor branches and typealias namespace comparisons (including normalized struct/enum names). Preserve existing source exports and free recorded APIs; suppress only colliding optional convenience methods with existing capability reasons. No blanket renaming, runtime/IR/ABI change or public switch.

### Verification
Retain four failing valid-source compiler/consumer reproductions and qualified locks. Add focused normalized dollar/raw spelling regressions and positive controls; actual generated default and ledger-transaction consumers must compile (except deliberate invalid alias namespaces, which should fail clearly during rendering without partial publication). Keep existing exact-name behavior and diagnostic ordering. Run focused owner/alias/render tests, strict lint/format; fullfixture comparison once integrated. Signed GPG/DCO and independent reviewer retest. Scope is compiler namespace correctness, not stopped runtime trust work.


### Root review scope extension — normalized struct/enum declarations
Confirmed source probes additionally demonstrate struct/struct, enum/enum and struct/enum `a$b` versus `a_b` collisions inside the generated `types` module. Extend this same identifier-namespace decision to declaration collection: preserve identical repeated raw definitions, reject distinct raw names that normalize to one Rust type, and retain the owning source location through existing `ConflictingStruct`/`ConflictingEnum` diagnostics. Cover both kind/order combinations, valid `$` spellings, separate raw-IR cases and positive repeated/distinct types. No field or enum-variant namespace expansion. External scratch probe additionally verifies both nonwitness and witnessed recorded free functions remain publicly callable after convenience-accessor suppression.


### Independent retest: alias-only declarations and borrowed facade access

Valid source reproduces omitted struct/enum declarations when only an alias references them. An alias with the normalized type name can even make the second alias silently resolve to Field. Collect alias target types under their source location before declaration rendering, and cover alias-only, nested and collision cases with actual generated consumers.

The source recording export must continue to suppress the colliding recording accessor, but a witnessed BorrowedContract then lacks an external construction path. Preserve its methods through a conditional `From<&ledger_contract::Contract<W>>` implementation when that accessor is suppressed. It borrows the same private witness field through the existing generated parent/child module relationship; it adds no runtime operation. A standard trait conversion avoids reserving another circuit method name. Existing noncolliding output stays unchanged. Verify construction and method accessibility from an external crate, including explicit trait syntax to avoid inherent method-name shadowing. Capability claims must remain accurate.

The external reviewer missed the maintained check_fixture_outputs.py inventory: vector_widen participates in the completed 197-fixture generation comparison. This is recorded as a reviewer-evidence correction. New cycle tests number nine; source inspection is not test execution.


### Final delivery

Locally delivered in signed GPG/DCO `300a5cd0`, pushed as part of dbfd7dc2. [Joined compiler audit remediation — 2026-10-07](references-0.3.0.md#note-155) records the 701-test final core gate, 197 fresh fixtures, 95.11% changed-backend coverage, actual consumer/MSRV checks, independent retest and exact source-to-commit binding. Earlier attempt receipts retain their original source scope.
