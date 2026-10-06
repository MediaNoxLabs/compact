---
id: RUST-ADR-0054
alias: ADR-0054
title: "Record Field subtraction and multiplication in ordered frames"
date: 2026-10-04
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "recording", "ledger-adts"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: e76f6ed658ae2935024050aed1fe532ba4ed3d0b0293811d235b152f408f8d3c
---
# RUST-ADR-0054 — Record Field subtraction and multiplication in ordered frames

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept ordered recorded Field subtraction/multiplication using local Field arithmetic and actual frame reads/writes, without fabricating VM arithmetic. Preserve exact proof case-count corrections and the explicitly interrupted older workspace run. Later passing gates are separate receipts, not retrospective completion of that interrupted run.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#153 closure](https://github.com/MediaNoxLabs/compact/issues/153#issuecomment-6017489661). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`1b4f9128`](https://github.com/MediaNoxLabs/compact/commit/1b4f9128ba40f86e372833355ec347a0dcc9803c) · [`785db0a8`](https://github.com/MediaNoxLabs/compact/commit/785db0a889b90413fd9a7c5b09709e6978f04e91) · [`f155ae78`](https://github.com/MediaNoxLabs/compact/commit/f155ae7834bfcc9acec31648019f8e4eef45f7e2). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 54
status: accepted-partial
date: 2026-10-04
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/153
```

## Historical decision and amendments

### Problem and real compiler probe

ABI-28 `compactc --target ts --target rust --skip-zk` accepts a contract with a Field Cell at physical path `[1,13]` and the following exported circuits:

```compact
export circuit subtract_amount(delta: Field): [] {
  amount = amount.read() - disclose(delta);
}
export circuit multiply_amount(factor: Field): [] {
  amount = amount.read() * disclose(factor);
}
```

The private typed IR carries a `StateAction::Let` Field binding whose value is `Expr::Subtract` or `Expr::Multiply`, with a left `Expr::CellRead` and right `Expr::Parameter`, then a `StateAction::CellWrite`. Native Rust methods are generated, but `rust-capabilities.json` marks both recorded and observed-call availability false. The exact recorded emitter handles the Let/Cell write, and its `field_expression` supports ordered `Expr::Add` but lacks the two other Field arithmetic variants. This is a proving coverage gap, not a missing runtime Field type or VM write primitive.

### Before and proposed after

Before, the consumer can execute the circuit locally but has no generated proof path:

```rust
let native = Contract::default().subtract_amount(context, Field::from(2))?;
// recording.subtract_amount(...) and subtract_amount_call(...) are absent.
```

After, the same generated facade should expose typed replayable and observed-state methods for both circuits:

```rust
let recorded = contract.recording.subtract_amount(context, Field::from(2))?;
let observed = ObservedContractState::decode(address, indexed_bytes, observation)?;
let prepared = contract.recording.multiply_amount_call(&observed, private, Field::from(7))?
    .prepare(verifier, randomness)?;
```

Within the emitter, evaluate the left operand and all its ledger/witness effects first, then the right operand, bind the arithmetic result once as `runtime::Field`, and pass that value to the existing typed Cell slot's `record_write`. Use the upstream ledger-8 field type's `-` and `*` operators, as the native emitter already does. The recording frame preserves ordered VM reads/writes and four-dimensional per-query gas; the proof adapter remains unchanged.

### Alternatives and ownership

Generating raw VM arithmetic instructions would duplicate upstream ledger-8 semantics: arithmetic over a disclosed Field is local Field computation, while the ordered Cell read and write are the public VM effects. Text substitution in emitted Rust would violate the AST boundary. Computing the product outside the recording frame would lose ordered operand evaluation if a later operand reads ledger state or witnesses. A typed extension of `recorded::field_expression` reuses its existing result binding, source-order evaluation, frame and slot abstraction.

The AST emitter owns `Expr::Subtract`/`Expr::Multiply` lowering and generated method eligibility from typed IR schema 8. The runtime owns `Field`, typed Cell slots, recorded frame and existing ledger-8 VM adapter; no runtime, derive or proc-macro code change is expected. No new midnight-zk/ledger primitive is introduced. The generated Rust API gains methods for these supported circuits, but the runtime contract is unchanged and ABI 28 is expected to remain valid; record any evidence that forces an ABI bump. The capability report must change from false/false to true/true for these exports. Keep a separate real native-only contract in the strict rejection gate so ADR-0053's capability boundary stays tested.

### Verification and risks

1. Capture independent TypeScript results, serialized state, public FAB input/alignment, full ordered public transcript shape and all four per-query gas dimensions for subtract/multiply at `[1,13]`. Compare native and recorded Rust outputs, state, gas and replay. Include noncommutative subtraction with distinct values and multiplication with a nontrivial factor.
2. Check the exact generated capability report and `--rust-require-recording`; a supported chunked arithmetic contract must strictly compile. Keep source-located strict failure and output preservation for an unsupported pure-call/write contract. Include a wrong-type generated-only consumer compile rejection.
3. Generate complete ZKIR/keys and compare manual versus generated observed-call prototype and tagged pre-proof bytes, then prove, independently verify, ledger-validate and apply both new circuits. Confirm TypeScript and Rust public transcript order and final state. Do not require independently generated PLONK proof bytes to match because upstream proof blinding uses OS randomness.
4. Run renderer, fixture, workspace, package/archive and clean exact-head packaged compiler gates. Record exact signed/DCO commit, before/after, ABI/schema, source/build impact, limits and release status in this ADR, its focused issue and [Milestone 2 — ADR delivery map](references.md#private-note-08). Branch remains local unless separately authorized.

### History

- 2026-10-04: proposed after a real ABI-28 source probe at signed/DCO `1b4f9128`. The two IR expressions and false/false capability report were inspected before implementation. Focused issue pending in `rust-backend-v2`.


- 2026-10-04: focused [MediaNoxLabs/compact#153](https://github.com/MediaNoxLabs/compact/issues/153) created in `rust-backend-v2` before implementation.


### Local delivery amendment — 2026-10-04

Conventional GPG-signed/DCO commit `f155ae7834bfcc9acec31648019f8e4eef45f7e2` implements the typed Field arithmetic extension. The recorded emitter now lowers `Expr::Add`, `Expr::Subtract`, and `Expr::Multiply` through one ordered operand path, binds a typed `runtime::Field`, then reuses the existing Cell `record_write`. No runtime or macro source changed; the upstream ledger-8 Field operators and Cell VM adapter remain authoritative. ABI 28, private IR schema 8 and capability-report schema 1 remain unchanged. The generated chunked Cell crate gains typed native, recorded and observed-call methods for both new circuits; capability changes from false/false to true/true. Its generated `lib.rs` grows 146 lines; the handwritten emitter change is 18 changed lines and reuses the existing frame/slot APIs.

Before: `Contract::default().subtract_amount(context, Field::from(2))` could run natively but no `recording.subtract_amount` or `subtract_amount_call` existed. After: `contract.recording.subtract_amount(context, Field::from(2))` and `contract.recording.subtract_amount_call(&observed, (), Field::from(2))?.prepare(verifier, randomness)?` are generated; multiplication has the analogous Field-typed methods. At physical Cell path `[1,13]`, independent TypeScript capture and Rust native/recorded execution agree for 3 - 2 = 1 and 3 * 7 = 21. The comparison includes serialized state (with the newly declared operation set), public FAB input/alignment, complete ordered public transcript shape, replayed state, and each query's readTime/computeTime/bytesWritten/bytesDeleted. Both circuits' manual and typed observed-call prototypes and 646 tagged pre-proof bytes agree; both generated calls proved, independently verified, ledger-validated and applied in the full local `--consumer --proof` gate. Proof bytes themselves are randomized and are not compared.

Verification at the local source head: 57 renderer tests, 137 fresh fixtures, the chunked Cell Rust/TypeScript parity test, two source rejection/output-preservation cases, wrong-typed generated-only consumer rejections, formatting, Python/JavaScript syntax, scoped diff, bounded all-features check, and full pinned Nix compiler/consumer/proof/ledger gate passed. A real pure-call/write contract remains native-only and fails strict mode at its source with prior output preserved; the old Field-product negative fixture is intentionally retired because this ADR makes it recordable. Clean GPG-signed `rust-backend-v2-abi28-rc7` at this commit has a verified clean manifest `target/rust-runtime-release-abi28-clean-rc7.json`: macro SHA-256 `6ad381315a85e786b17dba38467f36456235f9dcb689198fef314be6a8f66f17` (9 entries) and runtime SHA-256 `b8dcaa7f41b54b3b60ab6b17e5eb9c95021999cee1aa7aedc3a9f64f85d0b681` (247 entries). Exact-head packaged compiler and archive consumer gates are still running, so this is not yet a packaged-release acceptance claim.

Limit: this proves these Field expressions in the tested chunked Cell shape, not all arithmetic or arbitrary nested pure calls. The branch/tag remain local. Remote CI, published registry crates, and production wallet policy remain open.


### Clean exact-head verification — 2026-10-04, rc7

The previously pending packaged gates passed at the same signed/DCO `f155ae7834bfcc9acec31648019f8e4eef45f7e2` commit and GPG-signed annotated `rust-backend-v2-abi28-rc7` tag. The detached clean worktree has no tracked modifications. The clean release manifest `target/rust-runtime-release-abi28-clean-rc7.json` was written and independently reverified (`dirty: false`); macro/runtime SHA-256 are `6ad381315a85e786b17dba38467f36456235f9dcb689198fef314be6a8f66f17` / `b8dcaa7f41b54b3b60ab6b17e5eb9c95021999cee1aa7aedc3a9f64f85d0b681`. Nix built exact-head `compactc` at `${HISTORICAL_NIX_STORE}/jdma2ya2p4yxb7p30fpzngqqwk5vpkxa-compactc`. Its packaged rejection/output-publication/capability gate passed (zero failures), as did the untouched two-contract Counter+Cell archive-only consumer with one shared runtime. The packaged `check_compactc_target.py --consumer --proof` gate passed: complete ZKIR/keys, generated external consumers, recorded replay, manual-versus-typed observed-call parity, independent proof verification and ledger validation/application, including both new Field circuits. Counter sealed deploy/call handoffs were written locally. No runtime API or ABI change was required.

This is local release rehearsal, not branch/tag publication or remote CI. Public registry packages, external registry consumer, broader arithmetic/nested pure-call coverage, and production wallet policy remain open. Issue #153 stays open for same-head remote and release gates.


### Milestone exit clarification — 2026-10-04

The milestone's later scope amendment makes the final gate a complete same-revision `Compiler Build` CI run on the published dedicated branch, including Nix-built `compactc`, fresh generated consumer, fixture/parity/rejection, proof/ledger and wallet-handoff checks. A new tag, registry publication, public-registry consumer or independent release-candidate gate is not required to close rust-backend-v2. The signed rc7 and archive rehearsals above remain useful local provenance evidence; they are not additional milestone exit conditions. Focused issue #153's final unchecked item has been corrected to this CI criterion. The original request to keep M2 work local still governs branch publication until the user authorizes that step.



### Exact-head packaged proof and ledger gate — 2026-10-04

A fresh pristine `git archive` of signed/DCO `785db0a889b90413fd9a7c5b09709e6978f04e91` passed the complete local `nix develop .#compiler --command env COMPACTC=${HISTORICAL_NIX_STORE}/nsaiqjddnzwldv8q02nbdnvkwfn53z51-compactc/bin/compactc ... python3 tools/compact-rust-backend/check_compactc_target.py --consumer --proof` gate with exit 0 and `compactc target boundary and manifest: passed`. The command used the same Nix-built compiler from the exact commit; its separate Cargo consumers, ZKIR/prover/verifier artifacts, recorded replay, proof verification, ledger validation and application all completed. The smoke script and proof program are byte-unchanged since `f155ae78`, where the gate's **93 call cases** were enumerated; this is exact-head revalidation of that 93-case program, including observed Counter and Field subtraction/multiplication calls. A reused Cargo target cache reduced build work but did not alter source or emitted output. This invocation did not set `COMPACT_RUST_WALLET_HANDOFF` or `COMPACT_RUST_DEPLOY_HANDOFF`; the CI JavaScript wallet-handoff check is a separate pending gate. Same-commit remote `Compiler Build` CI remains open.


### Proof case-count correction — 2026-10-04

The preceding exact-head note's **93** count was the pre-arithmetic ABI-28 baseline. Signed/DCO `f155ae78` added `subtract_amount` and `multiply_amount`, so its program and committed `785db0a8` run contain **95** recorded/proven/validated/applied call cases. A second direct run of the exact-head compiled proof smoke binary against a preserved copy of its generated artifacts exited 0; `${LOCAL_EVIDENCE}/compact-proof-wallet-785db0a8.log` contains 95 `generated ... trace replayed and partitioned` lines and 95 `deployment and proven call validated and applied` lines. This correction supersedes only the case count in the preceding paragraph; its gate outcome remains valid. The replay emitted a 3.3 KiB call handoff and 1.7 KiB deploy handoff, SHA-256 `1aefb8bf0c0d96d1a0c2d9f36727b43fbdc400d76d0895c396ec7464787c0bac` and `30e0dd810e2363fd6ca37265f47f44c67d99d574bd02e051f9e0f4b77e731924`. JavaScript ledger-v8 handoff validation is pending.


### Exact-head ledger-v8 wallet handoff — 2026-10-04

The second exact-head `785db0a8` proof-smoke replay with `COMPACT_RUST_DEPLOY_HANDOFF` and `COMPACT_RUST_WALLET_HANDOFF` exited 0 and produced 1,765 deploy bytes and 3,372 call bytes. The committed-source `check_wallet_handoff.mjs` decoded both with pinned `@midnight-ntwrk/ledger-v8@8.0.3`, checked one deploy and one proven call in segment 1, matching address `85e623ca9ada2b6379b5ce1c929467474d7690c9b38e1bec78177bf92b589a2b`, and exact deserialize→serialize byte equality. The installed package's lockfile matches the exact archive's `wallet-handoff/package-lock.json`; the checker also validates the package name/version. The command exited 0: `ledger-v8 8.0.3 decoded 1765 deployment and 3372 call bytes ...`. The preserved log independently counts 95 replayed and ledger-applied call cases. This is a local binary handoff check, not a live wallet/node submission or same-commit remote CI result.


### Older-head macOS workspace-run limit — 2026-10-04

The clean `CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2 cargo test --workspace --exclude compact --offline --quiet` run in the separate `f155ae78` checkout advanced through executable fixture/integration test binaries without a reported failure, then entered doctests. The first fixture rustdoc process stayed asleep for over four minutes after the entire command had run nearly five hours on this macOS host; a diagnostic sample also stalled. I intentionally sent SIGINT to this **older-head optional local run**; its terminal exit is **130**, not a passing full workspace result. The prior notes saying it was still running are superseded. The local exact-head `785db0a8` Nix compiler, 491-case E2E, 137 fixture, 37-oracle, two rejection, 95 proof/ledger, pinned ledger-v8 handoff, and clean package/archive-consumer gates remain separate passing evidence. Full workspace completion at the **published exact head** remains a required remote Compiler Build CI check; do not infer it from this interrupted macOS attempt.
