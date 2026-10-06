---
id: RUST-ADR-0032
alias: ADR-0032
title: "Keep generated crate guides aligned with ABI 15"
date: 2026-10-03
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["distribution", "provenance", "validation"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 8901ec6e96800fe0ad17f1d07e20db2dda631ca78d711bae45ba7f887d527b89
---
# RUST-ADR-0032 — Keep generated crate guides aligned with ABI 15

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept documentation alignment with the implementation checkpoints recorded in the amendments. The ABI 15 title and later ABI 28 guide/live-evidence updates are historical snapshots, not current ABI 49 reference documentation. Preserve earlier measured provenance while directing current users to the current guides and final acceptance report.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#131 closure](https://github.com/MediaNoxLabs/compact/issues/131#issuecomment-6017451296). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`0eec27ef`](https://github.com/MediaNoxLabs/compact/commit/0eec27ef86d2da9c98c933e513ebcc8f2db224ff) · [`18ea4f38`](https://github.com/MediaNoxLabs/compact/commit/18ea4f3881ab72fc249684cfe281176089c1b646) · [`2f19f055`](https://github.com/MediaNoxLabs/compact/commit/2f19f0559ff13d167e32a218c006bc9dbc8e1b12) · [`cbc22fb9`](https://github.com/MediaNoxLabs/compact/commit/cbc22fb9970aca7901c4d76166fd473c1763d779) · [`e75f13ba`](https://github.com/MediaNoxLabs/compact/commit/e75f13ba2fdc954a29114177c31994a63e3cbec2) · [`eefde98d`](https://github.com/MediaNoxLabs/compact/commit/eefde98dffd8e4d6281f5c4e055ae8904af79a7f). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 32
status: accepted-local
date: 2026-10-03
milestone: rust-backend-v2
issue: 131
```

## Historical decision and amendments

### Problem

The developer guide says “Merkle calls remain native” and claims generated/runtime ABI 14. The runtime guide also starts at ABI 14. The actual renderer and runtime both assert ABI 15. Generated plain and historic Merkle append circuits already have recorded facade methods (ADR-0027/0028), and vector-key Set/Map calls have recorded methods (ADR-0030). A new consumer following these guides could avoid supported proof paths or misdiagnose an ABI mismatch. The proof section also omits the delivered Merkle and vector-key call cases.

### Before and after code examples

The old guide leads an engineer to assume this is unavailable:

```rust
let call = Contract::default().recording.append(context, leaf)?;
```

The generated `merkle-tree-oracle` and `hmt-insert-oracle` crates expose that method for their supported append circuits; the vector-key fixture similarly exposes:

```rust
let call = Contract::default().recording.setInsert(context)?;
let replay = call.public.initial().query(
    call.public.verify_ops(), None, &call.execution.context.cost_model,
)?;
```

The docs should distinguish these exact recorded slices from other Merkle operations that remain native. The compatibility table and runtime guide should identify ABI 15 as current, explaining that it adds vector-key Set/Map recorded methods while reusing ledger-8 `FixedVector`/slot primitives.

### Decision and ownership

Correct `tools/compact-rust-backend/README.md` and `runtime-rs/README.md` against the checked-in generated fixtures and `RUST_RUNTIME_ABI`. Keep the language scoped: plain and historic append have recorded paths, while other Merkle operations are native until complete trace support; vector-key Set/Map recorded methods are available only for supported circuit shapes. The existing 55 packaged offline calls include one vector-key Set insert and the two Merkle append slices. No emitter, runtime source, VM builder, IR schema, macro, generated API, package version or ABI changes. This is a guide correction for ABI 15, not a claim of broader language support.

### Alternatives and verification

Leaving the guide stale creates a misleading public compatibility contract. Bumping the ABI or changing code solely to match a stale guide would be wrong. Verify the renderer/runtime ABI constants and generated method presence directly, run a scoped documentation consistency check and `git diff --check` on staged files. Do not rerun proof gates for a text-only edit when their successful ABI-15 result is already documented in ADR-0030; record this limitation. Keep publication, remote CI, registry release and wallet/node submission open.

### Tracking

- Focused issue: pending MediaNoxLabs issue, assigned to rust-backend-v2 before editing.
- Sources: ADR-0027/#127, ADR-0028/#128, ADR-0030/#129, parent #106.
- Commit and validation: pending; branch `codex/rust-backend-ast`, unpushed.


### Issue assignment — 2026-10-03

Focused [#131](https://github.com/MediaNoxLabs/compact/issues/131) is assigned to rust-backend-v2 before editing. It carries the documentation acceptance gates above.


### Local delivery — 2026-10-03

Conventional GPG-signed/DCO commit `2f19f0559ff13d167e32a218c006bc9dbc8e1b12` updates only `tools/compact-rust-backend/README.md` and `runtime-rs/README.md`; `git log -1 --format=%G?` returns `G` and the DCO trailer is present. The backend guide now distinguishes recorded plain/historic Merkle append from other native-only Merkle operations, identifies declaration-typed vector-key Set/Map recording, sets the compatibility table to ABI 15, and names the 55 offline call gate including Merkle and vector-key Set insert. The runtime guide explains ABI 15 without inventing a new VM builder or macro.

A scoped consistency check asserted both ABI constants equal 15 and that checked-in plain/historic Merkle and vector-key fixtures contain the claimed recorded methods. `git diff --cached --check` passed. This text-only correction did not rerun the 55-call gate; ADR-0030 records its successful ABI-15 run. No code, schema, runtime, emitter, macro, dependency or ABI mutation occurred. The user-owned `doc/ledger-adt.mdx` edit was not staged.

Focused [#131](https://github.com/MediaNoxLabs/compact/issues/131) stays open for branch publication and clean remote CI. Registry release and wallet/node submission remain in the M2 exit gate. Branch local and unpushed.


### Proposed extension: branch architecture overview — 2026-10-03

The top-level doc/rust-backend-ast.md is a third guide under this decision and still claims JSON schema 6, generated/runtime ABI 3, only Counter/Boolean Cell recorded functions, and a small Counter/Cell proof slice. The current private IR is schema 8, runtime ABI 15, and the supported proof gate reaches 57 offline calls. A reader of the top-level architecture page receives a different contract from the backend/runtime guides.

Before: “The version 6 JSON schema…” and “The generated runtime ABI is 3”; recorded methods are described as Counter or Boolean Cell only.

After: identify schema 8/ABI 15, the first-class compactc --target rust packaging, and bounded recorded Counter/Cell/Set/Map/List/plain-and-historic Merkle/vector-key/nested-witness slices. Explicitly keep unsupported operations, wallet/node submission, remote CI and release open. No emitter/runtime/schema/ABI change. The owner is documentation; verification should check source constants and representative generated facade methods, with a staged diff check. Focused issue #131 is already in rust-backend-v2. This is an amendment to the accepted documentation decision, not a replacement of its original rationale.


### Architecture guide delivery — 2026-10-03

Local conventional GPG-verified/DCO commit 18ea4f3881ab72fc249684cfe281176089c1b646 corrects doc/rust-backend-ast.md under focused #131. Before, this third guide said “version 6 JSON schema”, “runtime ABI is 3”, described compact-rustc as the principal path, and confined recording/proof to Counter and Boolean Cell. After, it states private schema 8, generated/runtime ABI 15, first-class compactc --target rust packaging, bounded recorded Cell/Counter/Set/Map/List/plain-and-historic Merkle/vector-key/nested-witness slices, and the 57-call offline gate. The document still distinguishes shape eligibility, wallet/node submission, remote CI and runtime publication.

Ownership is documentation only; no emitter, runtime, macro, IR, ABI or generated crate mutation. Verification checked compiler/rust-ir-passes.ss schema_version=8, runtime-rs/src/lib.rs ABI=15, representative generated recorded facade methods, the 57-call packaged gate already passed at eefde98d, and staged diff formatting. No proof rerun for this text-only commit. The user-owned doc/ledger-adt.mdx edit was not staged. Branch remains local/unpushed; clean remote CI is open.


### Proposed ABI-28 guide refresh — 2026-10-04

The backend guide's compatibility table still calls ABI 24 current while `runtime-rs/src/lib.rs` and the AST renderer assert ABI 28. The top-level architecture page describes ABI-20 observed calls but does not summarize ABI 21–28. A developer following the guides may incorrectly reject a generated crate or miss typed multi-argument and chunked collection/Cell calls. This is the same documentation consistency problem as the original ADR, not a new runtime decision.

Before:

```text
Generated code and Rust runtime | ABI 24
```

After:

```text
Generated code and Rust runtime | ABI 28
```

Refresh the bounded ABI 25–28 descriptions and reference the exact 93-call local proof gate, while keeping remote CI, registry release and wallet admission limits explicit. Documentation owns this change. Typed IR schema 8, AST emitter, runtime, macro, ledger/zk dependencies, generated code and ABI constants remain unchanged. Verify source constants, generated fixture assertion, markdown links and diff formatting. #131 remains focused and assigned to rust-backend-v2; append exact signed/DCO delivery after verification.


### ABI-28 guide delivery — 2026-10-04

The same local conventional GPG-verified/DCO commit `e75f13ba2fdc954a29114177c31994a63e3cbec2` corrects the backend guide's stale `ABI 24` compatibility row to `ABI 28` and updates the architecture page's obsolete “multi-parameter calls pending” and 58-call language. The guides now identify ABI 21–28's bounded typed observed calls and chunked collection/Cell paths, private IR schema 8, and the **93-call** offline proof/verify/validate/apply gate. They distinguish earlier ABI-20 funded local wallet evidence from the still-open same-head ABI-28 wallet run. The backend guide documents the new archive-only generated consumer rehearsal and its public-registry limit.

Documentation and release tooling own this commit; no typed IR, emitter, runtime, macro, ledger/zk dependency, generated fixture or ABI constant changed. Python syntax/ABI/schema/fixture consistency, Ruby workflow YAML parse, scoped staged diff check, direct and Nix archive-consumer gates, and the clean rc2 release checks passed. The full 93-call proof gate was not rerun for this documentation/release-tool commit; it passed at the preceding ABI-28 code commit `0eec27ef`. Branch/tag remain local; remote CI and crate publication remain open. Focused #131 stays open.


### Proposed ABI-28 live-evidence guide correction — 2026-10-04

The top-level architecture guide was accurate when `e75f13ba` was committed: it said the current ABI-28 head still needed a same-head wallet/node run. That run has now passed without a code change. Leaving the sentence in place would make the published guide contradict ADR-0040–0043, #105 and the measured local result.

Before:

```text
the current ABI-28 head still needs a same-head wallet/node run
```

After, document the exact local pinned stack and Counter `round = 1 → 2` admission at tested signed/DCO `e75f13ba`, while keeping authenticated finality, remote CI, registry distribution and broader production wallet policy open. Add a concise link from the backend guide to the public #105 delivery comment so external reviewers can inspect evidence without vault access. This is documentation ownership only: no typed IR, emitter, runtime, macro, ABI 28/schema 8, generated source, ledger/zk primitive or wallet driver edit. Verify the source guides and scoped diff; no proof rerun is needed for a text-only correction. Focused #131 is already in rust-backend-v2; append the delivery commit after editing.


### ABI-28 live-evidence guide delivery — 2026-10-04

Conventional GPG-verified/DCO documentation-only commit `cbc22fb9970aca7901c4d76166fd473c1763d779` corrects the architecture guide's now-stale “same-head wallet/node run pending” sentence. It records the measured local ABI-28 deploy/first call/generated observed-state second call at tested signed/DCO code commit `e75f13ba`, with Counter `round = 1 → 2`, exact transaction-to-indexer and canonical/finalized node checks, and the local trust boundary. The backend guide clarifies that the confirmed-state builder was introduced at ABI 20 but was retested at ABI 28, and links the [public #105 delivery](https://github.com/MediaNoxLabs/compact/issues/105#issuecomment-5972867741) for engineers without vault access.

A scoped Python check matched both guides to runtime/emitter ABI 28 constants and the public issue link; scoped `git diff --check` passed. No compiler, typed IR, runtime, macro, generated fixture, dependency, wallet driver, ABI or schema changed, so the live proof was not rerun for this prose-only commit. The branch HEAD is now `cbc22fb9`, while local signed rc2 remains at its tested code commit `e75f13ba`. The pre-existing user-owned `doc/ledger-adt.mdx` edit was excluded. Branch publication, same-head remote CI, registry packages and production wallet trust stay open; #131 remains open.
