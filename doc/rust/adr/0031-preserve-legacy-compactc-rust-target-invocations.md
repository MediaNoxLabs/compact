---
id: RUST-ADR-0031
alias: ADR-0031
title: "Preserve legacy compactc Rust target invocations"
date: 2026-10-03
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["compiler-cli", "diagnostics"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 4f004afd71bdb252f5cbf4e9bf449dc23d9ad7c3b5598865add4264d97a17efe
---
# RUST-ADR-0031 — Preserve legacy compactc Rust target invocations

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept the documented target selector plus the bounded legacy --rust/--skip-ts aliases and their conflict/error behavior. The launcher owns selection and frontend forwarding. This compatibility decision changes invocation support, not generated runtime semantics, wallet admission or distribution guarantees.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#130 closure](https://github.com/MediaNoxLabs/compact/issues/130#issuecomment-6017449524). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`e85c53c0`](https://github.com/MediaNoxLabs/compact/commit/e85c53c053d06ff2819c1e3a60bf9d332e95084b). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 31
status: accepted-local
date: 2026-10-03
milestone: rust-backend-v2
issue: 130
```

## Historical decision and amendments

### Problem

The oracle `codegen-rust` compiler accepts transitional `compactc --rust --skip-ts` invocations, including the shape documented for a Midnight identity consumer. The AST branch makes `--target rust` the public spelling but currently forwards `--rust` to the ledger-8 Chez frontend. On the current local `compactc`, `--rust --skip-ts --skip-zk field_add.compact out` exits 1 with only `Usage: compactc-scheme ...`; it does not produce a Rust crate. This is a concrete migration break for callers that already use the oracle branch's command line.

### Before

```sh
compactc --rust --skip-ts --skip-zk contract.compact out
# Exit 1: Usage: compactc-scheme ...
```

`--target rust --skip-zk` already generates a buildable crate, so the failure is target selection in the Rust launcher rather than Rust syntax or ledger execution.

### Decision and after

Retain `--target ts|rust` as the documented repeatable interface and accept `--rust` / `--skip-ts` as undocumented, fork-transitional aliases with the oracle's behavior:

```sh
compactc --rust --skip-ts --skip-zk contract.compact out   # Rust only
compactc --rust --skip-zk contract.compact out             # Rust and TypeScript
compactc --target rust --skip-zk contract.compact out      # documented Rust only
```

`--skip-ts` without `--rust` is an actionable error, as is mixing either alias with any `--target`. Resolve the selection in the wrapper and forward only ledger-8 frontend flags `--emit-rust-ir` and, for Rust-only, `--skip-ts`. The resulting generated Rust public API remains the existing `Contract::default()` / `recording` facade; the aliases do not select a different emitter.

### Alternatives and rationale

Rejecting aliases would make the new branch incompatible with an oracle consumer despite otherwise equivalent output. Passing `--rust` through to Chez fails with generic usage text; adding the oracle's text-appending Rust emitter back to Chez would undermine the independent AST backend. A silent precedence rule between aliases and `--target` would make the invocation ambiguous. Explicit selection and conflict checks preserve deterministic output.

### Emitter and runtime ownership

Only `tools/compact-rust-backend/src/bin/compactc.rs` target parsing and its CLI integration gate change. The Scheme frontend still owns typed schema-8 IR, TypeScript and ZKIR; the `syn` renderer still owns Rust syntax. No generated Rust body, runtime primitive, derive, macro, VM program, IR schema or runtime ABI change is proposed. The shared-runtime option remains valid whenever Rust is selected. `--target` remains the recommended spelling; aliases are local compatibility surfaces and should be reviewed before upstreaming.

### Verification and risks

Compiler-backed tests should compare Rust-only and combined alias output to the corresponding `--target` invocations, including `lib.rs`, `Cargo.toml`, expected presence/absence of `index.js`, and manifest validity. The Rust-only alias output must build in a separate Cargo consumer. Negative invocations must exit before Chez and name the conflicting flags or missing `--rust`; no partial output should be created. Unit tests should cover option permutations and repeated flags. Existing 132-fixture freshness, standalone consumer, proof gate and all-target workspace should remain green where the touched surface applies. This change does not claim wallet/node integration or release readiness.

### Tracking and delivery

- Issue: pending focused MediaNoxLabs/compact issue, assign to [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2) before coding.
- Parent: [#103](https://github.com/MediaNoxLabs/compact/issues/103), [#106](https://github.com/MediaNoxLabs/compact/issues/106).
- Local commit: pending; branch `codex/rust-backend-ast`, unpushed.
- Delivery state: proposed.

### Amendments

Append exact signed/DCO commit, test commands/outcomes, migration limits and release status after implementation.


### Issue assignment — 2026-10-03

Focused [#130](https://github.com/MediaNoxLabs/compact/issues/130) is assigned to rust-backend-v2 before implementation. The issue carries the acceptance gates above.


### Local delivery — 2026-10-03

Conventional signed/DCO commit `e85c53c053d06ff2819c1e3a60bf9d332e95084b` (`fix(compactc): accept legacy Rust target aliases`) implements this decision on `codex/rust-backend-ast`. `git log -1 --format=%G?` returns `G`, and the commit has a `Signed-off-by` trailer. The branch remains unpushed.

The wrapper now consumes `--rust` and `--skip-ts` before invoking Chez. `--rust` emits Rust and TypeScript; `--rust --skip-ts` emits Rust alone. `--target` remains the primary spelling. Mixing the interfaces and lone `--skip-ts` fail before output creation with actionable diagnostics. The generated crate from the alias is byte-for-byte identical to the equivalent explicit-target crate (`lib.rs`, `Cargo.toml`; combined `index.js` also matches). A separate Cargo consumer built and exercised the alias-generated Rust crate.

Ownership stayed in `src/bin/compactc.rs` target selection and `check_compactc_target.py` integration assertions. There is no IR, emitter, generated API, runtime, macro, or ABI change (ABI remains 15). Unit tests cover alias order, repeats and invalid mixing. `cargo test -p compact-rust-backend --bin compactc` passed 5 tests; `cargo fmt --all --check`, `cargo check --workspace --all-targets`, and `check_fixture_outputs.py` passed (132 fixtures, zero stale/failed). The compiler-backed target/consumer/proof gate passed, including all 55 packaged offline ledger proof/validation/application calls.

Focused [#130](https://github.com/MediaNoxLabs/compact/issues/130) remains open for branch publication and clean remote CI. This compatibility slice does not establish wallet/node submission, published crate consumption or the wider production exit gate in [#106](https://github.com/MediaNoxLabs/compact/issues/106).
