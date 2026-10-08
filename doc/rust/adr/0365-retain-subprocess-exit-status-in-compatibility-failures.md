---
id: RUST-ADR-0365
alias: ADR-0365
source_sha256: d6ca456e761ead01d89f798abd86379a3057409ec8b1aa2a524fe90b15e6da60
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0365 — Retain subprocess exit status in compatibility failures

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

**Bounded diagnostic delivery:** signed commit `551f065cd6267e07020a755db5469df03e41b775` adds child exit status to failing test diagnostics. It does not establish the cause or repair of the original intermittent launch failure. Parent #364 remains pending. Original status, code examples and dated evidence below remain historical; no fresh full-gate or milestone acceptance is implied.

## Original decision and amendments

## ADR0365 — Retain subprocess exit status in compatibility failures

Date: 2026-10-08
Status: accepted diagnostic improvement; original intermittent launch cause unresolved
Parent: #364

### Problem

The full57fab775gate fails a CLI compatibility preflight assertion with empty child stderr. Replaying the exact selected-package stage fails a different diagnostic assertion. Neither existing assertion prints the child exit status, preventing distinction between compiler refusal and process-level termination. The local executable verifies valid-on-disk ad-hoc signing. Host logs contain an ASP denial for an installed hardlink and AMFI notices also seen for successful binaries, so they do not establish the cause of these assertion failures.

### Decision / before and after

Keep assertions and invocation semantics unchanged. Add child ExitStatus to the failure messages at the public preflight assertion, installed-root assertion and shared public-refusal helper. Do not add retries, skip tests, weaken diagnostic matching, alter runtime semantics or change host security policy. Do not introduce synchronization without evidence that hardlink concurrency caused the failure.

```rust
assert!(error.contains(expected), "expected {expected}: {error}");
```

```rust
assert!(error.contains(expected),
        "expected {expected}; child status {:?}: {error}", result.status);
```

### Domain and component impact

Test diagnostic visibility only. The emitted crate, compiler compatibility selection, runtime ABI and official ledger/zk ownership are unchanged. This does not assert that the original intermittent failure has been fixed.

### Evidence and acceptance boundary

Focused default-feature root rerun passed. Agent diagnostic-only all-feature backend and exact selected-package compatibility runs passed21cases each; three direct repeats also passed21each. No failing child status was reproduced. Original full/stage failures remain retained. Formatting and scoped diff checks pass. Full successor qualification is still required; any recurrence must be investigated using the improved status rather than ignored. No broader reliability, host-policy or security conclusion follows from the bounded successful reruns.

### Delivery

[#502](https://github.com/MediaNoxLabs/compact/issues/502) delivered at signed/DCO551f065cd6267e07020a755db5469df03e41b775, GoodGPGverified and pushed. [ADR0365 — CLI launch diagnostic receipt](references-0.3.0.md#note-169) retains failed and bounded successful runs. Full successor32750running; intermittent original cause remains unconfirmed.
