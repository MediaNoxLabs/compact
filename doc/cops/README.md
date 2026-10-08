# Compact Problem Statements (CoPS)

CoPS records a concrete problem and its evidence across Compact backends and upstream branches. It is separate from CoIPs (proposals) and Rust ADRs (implementation decisions). Each record links its supporting evidence, decisions and tracking issues, with explicit source versions and acceptance limits.

## Register

| ID | Problem | Status |
|---|---|---|
| [CoPS-001 — Ledger decoding resource limits](0001-ledger-decoding-resource-limits.md) |Encoded input size does not establish total decoded memory/CPU bounds |0.3.0 mitigation delivered in9ffd7880; pinned TS/ledger8 comparison complete; aggregate containment tracked in#495 |
| [CoPS-002 — Rust contract client and provider boundaries](0002-rust-contract-client-and-provider-boundaries.md) | Application-facing Rust provider/client layer comparable to MidnightJS | Local ProofLab slice delivered in #497; broader #496 remains proposed, not a 0.3.0 gate |

## Rules

- Name records `CoPS-NNN — Short problem title.md`; IDs are stable and never reused.
- Pin each compared repository/branch to a commit and each distributed dependency to a version/source identity. Moving branch names alone are insufficient.
- Separate observed failure, source-demonstrated missing guarantee, hypothesis and untested exposure. A missing budget is not itself a reproduced denial-of-service exploit.
- Always include Rust-AST, TS and the relevant mainstream baseline (ledger8 for0.3.0). Mark not-applicable or unverified explicitly; do not infer parity from shared dependency names alone.
- Describe user impact, minimal evidence/reproducer, mitigation, owner, issue/ADR links, acceptance boundaries and follow-up. Preserve corrections and historical failures.
- Keep generated API differences separate from transport/runtime/deployment ownership. A problem statement may remain open after a scoped mitigation ships.

[CoPS-template](template.md)
