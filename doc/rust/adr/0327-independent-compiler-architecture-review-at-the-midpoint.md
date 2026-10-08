---
id: RUST-ADR-0327
alias: ADR-0327
source_sha256: fb90992dcd167893edd8e0d337483a92732617f6efe0fc154253753517eaddfc
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0327 — Independent compiler architecture review at the midpoint

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** planned first pass; 2026-10-07; R03013/#357, milestone0.3.0. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0327 — Independent compiler architecture review at the midpoint

Status: planned first pass; 2026-10-07; R03013/#357, milestone0.3.0.

### Problem
The milestone requires independent external coding-agent review in addition to implementation-agent tests. Current compiler/domain/API changes are stable, local679-owner tests and197fixture renders pass, and bounded Linux CI is green. Starting an architecture/correctness review now permits remediation before final candidate qualification.

### Decision and scope
Ask the separately configured local Claude Code CLI to review exact committed bf7c719beea14f7532feaf0d57577245f42c98ca, using read-only file tools. Review compiler ownership, structured AST generation, lexical/effect handling, API friendliness, local resource/error handling and unit-test sufficiency. Review actual source; distinguish actionable bugs from preferences and uncertainty. No source edits, command execution, network operations or test creation by the reviewer. Preserve prompt, CLI version, source manifest, output and tool/model identity where returned.

This first pass excludes runtime offer/observation trust, cross-instance consumer safety and ADR0285, which remain stopped and unaccepted. It is not a workaround for that stop or a full security audit. Final milestone audit still requires its separate approved scopes and dispositions. This is an independent coding-agent review, not human certification.

### Before / after
```text
Before: implementation review + scoped test/render receipts
After:  same implementation, independent source findings with severity and evidence
        -> root verification -> issue/ADR remediation -> independent retest
```
No emitter/runtime code or ABI change is planned by this review. Before/after code belongs in any resulting remediation ADR. Prefer existing ownership boundaries and ordinary Rust to speculative architecture rewrites.

### Acceptance
Bind source before/after, record read-only tool restrictions and reviewer identity. Preserve successful, failed or timed-out invocation accurately (up to15minutes). Verify every concrete finding at exact source lines. Track severity, owner and disposition. Do not close parent#357 or claim a complete audit from this partial review.


### 2026-10-07 — Independent external compiler review returned findings

ADR0327/#452 ran read-only ClaudeCode2.1.273 against bf7c719b (641.4s;797 compiler/runtime/testkit sourcefiles unchanged). Returned model identities include claude-opus-5[1m]/claude-haiku-4-5-20251001. Five potential correctness findings now undergoing root/subagent verification: fallible vector conversion; normalized observed-call names; witnessed recording accessor; typealias namespace; purecallcycles. No findings are called fixed or independently reproduced yet. Architectural preferences remain advisory.

Preserved fullprompt/toolscope/result/receipt in [ADR0327 — Independent Claude compiler review bf7c719b.zip](references-0.3.0.md#note-111), SHA256 `94199f29fb32b139d61316e15df6cfe1cd9de27ef4ecdc652cb5aeaca1cfcc90`. This partial external coding-agent review excludes runtime trust/crossinstance and stoppedADR0285, and does not close parentaudit#357.
