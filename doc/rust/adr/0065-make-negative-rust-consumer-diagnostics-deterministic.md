---
id: RUST-ADR-0065
alias: ADR-0065
title: "Make negative Rust consumer diagnostics deterministic"
date: 2026-10-04
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["distribution", "provenance", "validation"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: bf2009680aaee04615fee82904092cc6fb49b3fb37f723b71e593622498ee0c2
---
# RUST-ADR-0065 — Make negative Rust consumer diagnostics deterministic

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept disabling ANSI color in the controlled negative-consumer subprocess environment so intended diagnostics remain deterministic. Production compiler output and global user configuration stay unchanged. Preserve the historical compiler-source equivalence and the distinction between consumer-only and proof/remote validation.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#164 closure](https://github.com/MediaNoxLabs/compact/issues/164#issuecomment-6017508709). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`1495bf64`](https://github.com/MediaNoxLabs/compact/commit/1495bf64f9b45e26ff543a50e1b29241cc2019ca) · [`79ba4c7a`](https://github.com/MediaNoxLabs/compact/commit/79ba4c7a29279fe091fbd7f2db6c982967fdbfad). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 65
status: accepted-partial
date: 2026-10-04
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/164
```

## Historical decision and amendments

### Problem and evidence

The first published-head `Compiler Build` run at `1495bf64` reached the Apple Silicon generated-consumer gate and compiled a deliberately wrong Set element. Rust reported the intended `E0308` mismatch, but `check_compactc_target.py` asserted that raw stderr contained `error[E0308]: mismatched types`. On this runner the string is split by ANSI color codes, so the assertion failed despite the correct type rejection. The failed [run 37193647381](https://github.com/MediaNoxLabs/compact/actions/runs/37193647381), Apple job 111410879996, and local log `${LOCAL_EVIDENCE}/compact-ci-apple-1495bf64.log` record the exact output. Similar literal stderr assertions exist for other wrong-type and unavailable-method probes.

### Before and proposed after

```python
# Before: captured cargo diagnostics inherit runner color settings.
environment = os.environ.copy()
rejected = subprocess.run(["cargo", "check", "--example", "wrong_set_element"],
                          env=environment, capture_output=True, text=True)
assert "error[E0308]: mismatched types" in rejected.stderr

# After: normalize Cargo output for this test process before subprocesses.
os.environ["CARGO_TERM_COLOR"] = "never"
environment = os.environ.copy()
# The same negative probes can assert stable text.
```

Set this once at the `check_compactc_target.py` entry point, before any Cargo invocation, so every external consumer probe inherits it. Keep the required nonzero exit-code check; the diagnostic text alone must never count as a rejection. This is a test-harness output policy, not a compiler diagnostic change.

### Ownership and compatibility

The Python consumer/proof gate owns this setting. The `syn` emitter, generated crates, runtime, macros, Compact compiler, ledger/zk primitives, VM, proof bytes, generated ABI and private IR schema do not change. Disabling ANSI in this gate also keeps failed logs readable. Cargo color remains unchanged outside this process.

### Acceptance and limits

Create a focused MediaNoxLabs issue in `rust-backend-v2` before code edits. Verify that `CARGO_TERM_COLOR=always` in the parent environment no longer breaks wrong-type probes; the negative compiler error and exact expected type must still be asserted. Run the exact-head Nix-built consumer/proof gate, fixture freshness and `cargo fmt --all -- --check`; commit with conventional subject, DCO and GPG. Push and rerun Apple Silicon `Compiler Build` on the same commit, along with required Linux jobs. This does not claim diagnostics are stable across arbitrary future rustc wording changes; it addresses ANSI encoding of the current asserted messages.

### Tracking

- Parent production CI issue: [#106](https://github.com/MediaNoxLabs/compact/issues/106).
- Focused issue: pending creation before implementation.
- Delivery: proposed; no code or remote-green claim.

### Delivery-order amendment — 2026-10-04

The user subsequently deferred remote CI until the local backlog is complete. Verify the harness locally under a parent `CARGO_TERM_COLOR=always` and preserve the eventual Apple runner check as an open release gate. Do not dispatch another remote run during local delivery.

### Tracking amendment — 2026-10-04

Focused [#164](https://github.com/MediaNoxLabs/compact/issues/164) was created and assigned to `rust-backend-v2` before the harness edit. The pending sentence above preserves the proposal sequence.

### Local implementation checkpoint — 2026-10-04

Conventional GPG-signed/DCO commit `79ba4c7a29279fe091fbd7f2db6c982967fdbfad` (`fix(rust-backend): capture uncolored consumer diagnostics`, `Refs: #164`) sets `CARGO_TERM_COLOR=never` in the Python gate before it copies the environment and starts any Cargo subprocess. `git verify-commit` reports a good signature. This changes two harness lines only; the emitter, generated source, runtime, macro, compiler package, proof adapter, ABI 33 and private IR schema 8 are unchanged.

From a clean exact-head `79ba4c7a` rehearsal checkout, the Nix compiler `${HISTORICAL_NIX_STORE}/c09kjdk3rapk46pms7yva6blh0hqhncq-compactc/bin/compactc` passed `check_compactc_target.py --consumer` with the parent environment explicitly set to `CARGO_TERM_COLOR=always`. All existing negative consumer probes retained their nonzero-exit and expected diagnostic assertions, including the wrong Set element that failed the Apple job. The gate ended `compactc target boundary and manifest: passed`; formatting, Python syntax and clean checkout status also passed. The Nix compiler binary was built at the prior `1495bf64` source, whose compiler implementation is unchanged by these two commits. The proof option and remote Apple rerun are deferred to final stabilization under the user's local-first direction; [#164](https://github.com/MediaNoxLabs/compact/issues/164) stays open.
