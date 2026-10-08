---
id: RUST-ADR-0345
alias: ADR-0345
source_sha256: ae67e7ca956d36fe86bb4fba445e66f6e7d52a4f7127da2095358a0ad21e17cd
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0345 — Review remaining dependency notices against shipped feature graphs

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted for current audit, dependency-graph review and isolated compatible-update proposal. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0345 — Review remaining dependency notices against shipped feature graphs

Date: 2026-10-07
Status: accepted for current audit, dependency-graph review and isolated compatible-update proposal
Parents: R03013/#357 and final qualification#364
Baseline: signed5a9cd356 on codex/rust-backend-ast

### Problem

ADR0301 delivered compatible dependency remediation but retained lru unsoundness and bincode/number_prefix/paste maintenance notices. Earlier broad metadata describes potential paths, not selected target/feature exposure. The user approved refreshing these notices, verifying actual ownership/use, testing compatible changes where justified, and independently reviewing residual dispositions.

### Decision and before/after

Before: four historical notices with inherited constraints and deferred final-audit decisions.
After: current raw strict scans with pinned tool/database/lock identity; per-root normal/build/dev and selected feature/target paths; source-grounded ownership/precondition analysis; tested compatible fix or explicit unresolved disposition.

No new domain/API/emitter/runtime behavior is chosen by this ADR. Illustrative control flow remains `audit match -> selected dependency path -> inspected usage -> compatible proposal -> focused gates -> root disposition`. A clean scanner result is not the acceptance criterion by itself.

### Scope and gates

Refresh root and standalone locks using the already qualified scanner. Reuse existing Cargo metadata and upstream source caches when their identities match. Delegate read-only package/feature inventories. Inspect documented lru usage/preconditions without constructing an exploit. Keep native ledger8.0.3 and existing zk identities, ABI50/private IR20 and actual Rust1.88 compatibility. Treat maintenance as a distinct category from demonstrated vulnerability.

Any live dependency change follows a concrete isolated lock/manifest delta and measured focused validation; add a more specific ADR/issue if substitution changes APIs, serialization or upstream ownership. No blanket ignores or advisory-DB edits. No whole proof/build suite for documentation-only dispositions. Preserve the user-owned ledger document.

The user will arrange a separate Claude review using [Claude prompt — Remaining dependency advisories](references-0.3.0.md#note-141). Record independent results without treating the implementing agent as its own reviewer. Do not send the prompt externally on the user's behalf.

ADR0285/#409 remains stopped and outside this slice. This dependency review cannot accept that separate requirement. Planning/evidence stay in midnight until closeout. Parent count remains12/19.


Issue: https://github.com/MediaNoxLabs/compact/issues/469
Fresh audit: [ADR0345 — Current dependency audit checkpoint](references-0.3.0.md#note-117). Independent user-arranged Claude review pending.


### ADR0345/#469 local review assembled — 2026-10-08

[ADR0345 — Remaining dependency dispositions](references-0.3.0.md#note-120) records current strict scans (0 vulnerability matches, 4 workspace / 3 standalone warnings; both exit 1), 66 successful offline/locked graph commands, exact caller/serialization/macro/CLI ownership, and proposed residual actions. No supported owned feature-only or lock-only fix was established while preserving the native ledger8 pins. No source/dependency changes or new build campaign.

[Claude prompt — Remaining dependency advisories](references-0.3.0.md#note-141) is ready for the user's independent review. #469 remains open pending that review and disposition; #357/#364 remain open. No risk acceptance or safety certification inferred. Parent progress remains 12/19; the stopped ADR0285/#409 lane remains excluded.

Archive [ADR0345 — Dependency disposition evidence.zip](references-0.3.0.md#note-118), SHA256 `7e93a851072708f45401df48ea36f6b617a02dcade0aab71cf087c8c8578e2b4`, 416 byte-verified entries. Exact locks, upstream advisory texts, source snapshots/hashes, commands, graph outputs and review reports retained.


### Independent Fable review reconciled — 2026-10-08

[ADR0345 — Independent dependency review disposition](references-0.3.0.md#note-119) records the supplied Fable 5 High assessment and root decision: retain four documented dependency residuals, no immediate pin-preserving update established. Review applies to5a9cd356; current f3526275 differs only by a test and both locks match. Root qualifies mutex poisoning as advisory (not a soundness guarantee) and distinguishes paste's host executable from emitted code.

Dependency review #469 is complete; broader #357 audit and #364 final release acceptance remain open. Deferred upstream items #471–#474 cover lru, optional CLI/indicatif, curve macros and serializer ownership separately. Strict audit still exits1 on warnings; no clean-security claim or new tests/builds. Parent count12/19 unchanged; stopped #409 unaffected.

Original external report is preserved. Reconciliation archive SHA256 `519681ec509763c3e2336394602c3e3bf0c26fb8e5aa547bf72693f271b3d66d`. Reviewer model identity is user-attributed; raw independent command logs were not supplied with the report and are not replaced by our logs.
