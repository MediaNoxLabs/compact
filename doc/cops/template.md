# CoPS-NNN — Problem title

Status: investigating / mitigated / resolved / deferred (state the exact scope)
Date:
Owner:
Issues:
Related ADRs / CoIPs:

## Problem and affected use case

State the concrete failure or missing guarantee and why an engineer should care. Identify the boundary and who supplies the input.

## Evidence classification

Observed behavior / source-established limitation / hypothesis / not tested. Include minimal inputs or commands, expected versus actual behavior, and source/log identities. Avoid unsupported exploit or security claims.

## Backend and upstream comparison

| Target | Exact repository commit / package | Relevant entrypoint and implementation | Finding present? | Evidence limits |
|---|---|---|---|---|
| Rust-AST | | | | |
| TypeScript | | | | |
| Mainstream ledger8 | | | | |
| Other relevant implementation | | | | |

## Ownership and impact

Distinguish compiler, generated API, owned runtime, upstream primitives, application transport and deployment responsibility. Describe compatibility and affected supported workflows.

## Mitigation / decision

Before and after examples; alternatives; what is implemented and what remains unresolved. If a limit is policy rather than protocol, say so and explain configuration and sizing.

## Verification and acceptance

Named positive/negative tests and exact result identities. Describe untested paths and source continuity. Do not substitute a test count for the required guarantee.

## Follow-up and history

Track explicit owner decisions, issue/ADR delivery, unresolved work and superseded premises. Record an actual resolution criterion.
