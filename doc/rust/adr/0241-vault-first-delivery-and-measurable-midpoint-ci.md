---
id: RUST-ADR-0241
alias: ADR-0241
source_sha256: d854c590cd6bd884bbef8118fd03d0f38388a5bb46bcfee54578e4e74c87b1a2
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0241 — Vault-first delivery and measurable midpoint CI

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

```yaml
status: accepted
date: 2026-10-07
milestone: "0.3.0"
issue: https://github.com/MediaNoxLabs/compact/issues/363
```

## ADR-0241 — Vault-first delivery and measurable midpoint CI

### Problem statement

The milestone has broad engineering and adoption work. Duplicated plans drift, slow remote checks delay small local iterations, and counting child issues can disguise incomplete parent outcomes. The user requires a durable engineering history and a midpoint for CI stabilization.

### Decision

Obsidian `midnight` owns the 0.3.0 plan, new ADRs, findings, research and step-by-step receipts until publication to the repository at closeout. Existing published ADRs remain historical records. GitHub issues mirror actionable scope and contain problem statements; they link back to vault ADR identifiers. Code and tests remain in Git.

Use the 20 approved parent outcomes as a frozen denominator. Begin CI stabilization after 10 satisfy full local acceptance. Partial slices, extra child issues and the deferred ACC placeholder do not count. Explicitly record any scope amendment before changing the denominator. Final candidate qualification retains full applicable local and required remote evidence.

### Before / after workflow

Before: local planning documents were also committed under `doc/rust/milestones/0.3.0`; remote validation was concentrated at final qualification.

After: approved planning lives in this vault; the current repository planning additions are reversed in `4c5eb5de`. Local evidence drives small iterations, CI stabilization begins at 10/20, and reviewed milestone documentation publishes at closeout. The original planning commit remains in history.

### Emitter/runtime implications

No emitted code, ABI or runtime behavior changes. Focused local checks are selected by the actual change risk; broaden checks for affected compiler IR, ABI, effects, consumers or security obligations.

### Alternatives and limitations

A Git-only planning workflow conflicts with the user's chosen knowledge base. Counting issue closures permits progress inflation. Deferring all CI work to the final candidate postpones platform problems. Local success alone does not establish remote qualification.

### Verification and progress

The approved plan, backlog and dashboard have been saved through the Obsidian CLI and byte-verified. GitHub parent #363 tracks this operating policy. Local acceptance starts at 0/20. Each meaningful journal receipt records revisions, changes, commands/results, limits and next action.
