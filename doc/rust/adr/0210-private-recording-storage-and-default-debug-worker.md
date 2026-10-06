---
id: RUST-ADR-0210
alias: ADR-0210
title: "Private recording storage and default debug worker"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-runtime-fix"
topics: ["recording", "storage", "stack"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 9e28dd74269cf4208371369893ff9cfbf012fc35dd360c9729c8aea504a81a5b
---
# RUST-ADR-0210 — Private recording storage and default debug worker

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-runtime-fix. RecordingFrame moves its private state into one owned allocation to make original withdraw fit the default debug worker stack. Public frame semantics, results and ABI stay unchanged; measured frame size and passing default-worker tests support this specific stack fix.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#314 closure](https://github.com/MediaNoxLabs/compact/issues/314#issuecomment-6017762342). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`3404c30c`](https://github.com/MediaNoxLabs/compact/commit/3404c30c965248d69f07e759e22902fc4faf5de6) · [`a7e14034`](https://github.com/MediaNoxLabs/compact/commit/a7e14034f6153212a356b47a1be73c72182e4fa5) · [`e9ef0535`](https://github.com/MediaNoxLabs/compact/commit/e9ef05356d43af401bfc721513a619720a3109bb) · [`fbe42d3d`](https://github.com/MediaNoxLabs/compact/commit/fbe42d3db3dbc6d142ce3b46c65868cfbc7f792c). These include historical prerequisites and probes, not only final delivery commits.

## Historical decision and amendments

Status: accepted for bounded probe, 2026-10-06. Runtime ABI49/schema20 unchanged; safe private storage only. ADR207 owns emitter.

Root approved the following proposal before implementation. Actual allocation, stack and behavior measurements will determine acceptance; no fix is claimed yet. Preserve ADR206 historical8MiB evidence and the separate compiler structural-test stack boundary.

## ADR210 proposal — reduce private recording-frame move size

Research only, 2026-10-06. ADR206 is signed as `556ef73e9f3ca7943f12a8e81a3a9a4685018c86` and integrated as `e9ef0535`; its final receipt is complete. No ADR210, issue, runtime edit, new build, or implementation probe has been made. ADR207 owns the shared emitter. This proposed slice changes runtime-private storage and focused test/measurement support only.

### Problem and existing evidence

Use `${LOCAL_EVIDENCE}/compact-withdraw-debug-stack-review.md` and the retained ADR206 failure as the baseline, without turning inferred aggregate sizes into newly measured facts. Existing arm64 debug disassembly reserves 2,068,064 bytes in generated recorded withdraw, plus 58,064 bytes in its parity caller; those two frames alone exceed 2 MiB. Native withdraw reserves 106,400 bytes. The successful ADR206 behavior harness currently creates an explicit 8 MiB worker.

Copy sites imply RecordingFrame<Private> = 5,880 bytes, CircuitContext<Private> = 3,736, initial QueryContext = 1,944. The last two constitute 96.6% of the moved frame. The generated function has 76 frame bindings and 150 immediate 5,880-byte copy widths; its 587 memcpy call sites are static sites, not execution counts. Runtime owns the large aggregates privately; generated code only consumes and returns RecordingFrame through methods.

The structural planner test's separate stack overflow is a compiler/IR recursion path. Runtime storage work does not solve that problem. Preserve that test's honest 8 MiB limitation and do not touch typed_plan, source admission or generated code in this slice.

### Recommendation

First measure a **single boxed private core**:

```rust
// Before: each consuming move carries all inline state.
pub struct RecordingFrame<Private, D: DB = DefaultDB> {
    context: CircuitContext<Private, D>,
    initial: QueryContext<D>,
    // Existing identity/intent snapshots, transcript vectors and gas.
}

// Proposed private representation; generated/consumer APIs unchanged.
pub struct RecordingFrame<Private, D: DB = DefaultDB> {
    state: Box<RecordingState<Private, D>>,
}
struct RecordingState<Private, D: DB> {
    context: CircuitContext<Private, D>,
    initial: QueryContext<D>,
    // The exact existing snapshots, vectors and gas, in the same ownership model.
}
```

There is one box allocation at frame construction, retained by consuming frame methods and released at finish/error. Keep all existing method signatures, `context() -> &CircuitContext`, public result fields and `PublicTrace::initial/into_parts` unchanged. No public Deref implementation, shared Rc/Arc core, interior mutability, context cloning, unsafe runtime storage, leaked state or new VM operations. `new` still snapshots the same initial values at the same point. `finish` moves the same owned fields into existing output structures once.

Normal query/witness/intent methods mutate fields through the retained core without allocating another box. `call_local` remains a checked move of the owned context through the existing closure, with identical before/after comparisons and error behavior; do not add a placeholder default context or weaken its audit. If Rust's ownership/borrowing rules require more than localized safe field splitting, report that concrete boundary rather than changing the API or adding unsafe code.

Private layout changes need no runtime ABI/schema bump: generated methods and required semantics are identical. Keep ABI49/schema20 and all generated fixture bytes unchanged. There is no supported repr(C) or public field layout on RecordingFrame.

### Alternatives and tradeoffs

| Storage | Expected moved frame | Allocation impact | Decision |
| --- | --- | --- | --- |
| One private boxed core | One pointer, to be measured | One persistent allocation per recording | Preferred probe: largest move reduction with one allocation, mechanical private access changes |
| Box context and initial query only | Approximately 200 bytes, to be measured | Two allocations; call_local can require care to reuse owned storage | Fallback if whole-core ownership ergonomics create excessive changes; preserves most field accesses |
| Box only context | Still approximately 2 KiB | One allocation | Likely insufficient across many debug temporaries; measure only if evidence warrants |
| Box public CircuitContext/QueryContext fields or cost model | Broader public API/layout effects | Variable | Outside this slice |
| Compiler scope shortening/helper extraction | Can reduce root temporaries | May avoid allocations | Separate emitter work with semantic/callee-scope risks; ADR207 owns emitter |
| Larger default worker stack | Hides the measured size | Per-thread memory | Not the solution or acceptance condition |

Box::new can itself construct a large stack temporary in debug. Measure constructor, leaf, finish and caller frames; shrinking RecordingFrame's size alone is not proof that the complete call fits. The final CircuitResult/PublicTrace still carry inline context/query values once, and must be included in the measurement.

### Measurement plan after approval and ADR/issue creation

1. Freeze before/after source hashes, rustc version/profile/flags, executable SHA and architecture. Reuse `target/adr157`, `CARGO_INCREMENTAL=0`; no new cold target or broad suite.
2. Run actual size_of probes for RecordingFrame, CircuitContext, QueryContext, CircuitZswapPlan, CostModel, PublicTrace, result and original coin/composite types. Use a small isolated measurement executable or test support; do not infer runtime values from disassembly alone.
3. Run unchanged generated original red and blue calls in isolated child processes using an unconfigured default Rust test/thread worker, with RUST_MIN_STACK unset. Stack overflow aborts a process, so the controlling process must retain the outcome. Also run a controlled explicit 2 MiB worker for an unambiguous byte limit, plus the existing 8 MiB baseline to establish semantic equivalence before the change.
4. Capture equivalent arm64 recorded/native/caller/new/leaf/finish prologues and memcpy width/site evidence using the pinned local llvm-objdump. Report static reservations/copy sites separately from measured behavior; no claims about executed memcpy count.
5. Count allocations in a standalone measurement harness around preconstructed-context frame new/finish and complete successful red/blue execution, before and after. A test-only counted System allocator may use the standard unsafe allocator wrapper; runtime implementation remains safe. Keep setup, JSON parsing and fixture loading outside the counted interval. Report allocations, deallocations and bytes as deltas, and verify no per-frame-leaf box churn. PublicTrace/result allocations must be reported, not hidden.
6. Do not claim release performance or universal stack bounds. Release behavior remains unmeasured unless an already available equivalent artifact permits a bounded comparison.

### Correctness and acceptance

- Mandatory: both red and blue original calls plus all 27 existing TypeScript/native/recorded parity/error cases pass on the normal test worker without the runtime behavior harness's 8 MiB wrapper. Add a process-isolated default-worker regression so an overflow is reported as a failed child rather than aborting an entire unrelated test suite.
- Preserve results, all state/effects, gas, VM replay, witness/native output order, intent/cursor/index order, initial/final identity and intent snapshots, and selected-branch error prefixes. No source/emitter/ABI/schema/generated fixture changes.
- Run affected runtime recording/call_local tests, identity tests, feature-enabled transaction controls (guaranteed, canonical, funding, transient and fallible), original withdraw parity and strict affected-package Clippy. Use existing test coverage instead of adding mirrored storage tests.
- Confirm actual single-box allocation behavior, constructor/finish stack usage and safe cleanup on both success and error. A small synthetic large Private payload can measure whether caller-owned input size remains a distinct boundary; do not promise an arbitrary payload can originate on a 2 MiB stack.
- Since storage affects every frame method, run the existing local runtime test set and selected generated query/witness/call_local controls. Reuse the original ADR206 proof keys; a selected strict original withdraw call can confirm preparation/proof compatibility if parent wants that gate, without rerunning the entire legacy proof suite or regenerating keys.
- If default-worker acceptance is not achieved, preserve the failed measurement and propose the next isolated change. Do not silently broaden into compiler helper extraction or leave a larger-stack default as the product fix.

### Ownership and delivery

Start an isolated runtime-only branch from signed556ef73e. ADR207 may independently change emitter files and fixtures; coordinate any common proof-main or test integration before touching it. Expected implementation files are `runtime-rs/src/recording.rs`, its `recording/zswap.rs` private accesses, focused runtime/Coracle test support and documentation. Transaction policy, observed identity semantics and compiler admission remain unchanged. Create ADR210 and a MediaNoxLabs issue under rust-backend-v2 only after root approves this proposal; commit conventional explanatory GPG+DCO delivery with before/after receipts and the exact stack/allocation tradeoff.


### ADR210 signed runtime delivery

`111fe5f34e2d5deebfb347825dd3fb3bb1aac923` is GPG+DCO signed, based on ADR206 `556ef73e`. Five files only; no emitter or generated fixture changes, no push/remote CI.

RecordingFrame now owns one private boxed RecordingState. Public consuming methods, context access, result/trace types, identity/intent snapshots and call_local checks are unchanged. The allocation is retained across frame leaves; no shared/interior mutable state, unsafe runtime code, placeholder context or context cloning was introduced. ABI49/schema20 remain.

#### Measured before/after

Arm64, Rust 1.99 debug, same original source and measurement harness:

| Measurement | Inline before | Boxed after |
| --- | ---: | ---: |
| RecordingFrame bytes | 5,880 | 8 |
| Generated recorded withdraw static frame | 2,068,064 | 76,352 |
| Measurement caller frame | 11,584 | 11,584 |
| Frame constructor | 8,016 | 13,952 |
| Frame finish | 12,656 | 12,672 |
| call_local example | 42,576 | 30,864 |
| own-key leaf | 12,224 | 480 |
| Static memcpy call sites in recorded withdraw | 587 | 364 |
| `mov w8, #5880` sites | 150 | 0 |

The red/blue calls aborted (signal 6) on both unconfigured default and explicit 2 MiB workers before the change; all four child cases now pass. All 27 original TypeScript/native/recorded scenarios pass on the normal test worker after removing the prior 8 MiB wrapper. A process-isolated regression unsets RUST_MIN_STACK and preserves both default/explicit checks.

The cost is exactly **one additional allocation/deallocation and 5,880 allocated/freed bytes** for the measured Private type. This same delta appears in new/finish, local-helper success, local-helper error, witness error, and full red/blue execution; no per-leaf box churn. Setup is outside allocation counting, so absolute totals are not net-leak measurements. Instrumentation exists only in the test executable. Static memcpy metrics are not executed counts. Release performance, arbitrary payloads and universal stack limits are not claimed.

#### Validation and limits

- All **96** runtime tests pass with ledger-transaction enabled, including identity, call_local and existing transaction policy controls.
- Original **27** parity cases and the process-isolated stack regression pass on normal workers.
- Strict runtime/Coracle Clippy and formatting pass.
- Both original red/blue default-strict proof and ledger application confirmations pass with exact rollback, relocation and replay negatives. The focused confirmation uses the unchanged ADR206 keys in a separate output directory. It retains original offline seeded-state scope, default-strict application, exact rollback and relocation/replay negatives.
- The separate compiler structural-test 8 MiB limit is unchanged. ADR206 historical failure and successful enlarged-worker evidence are preserved.

Receipt `${LOCAL_EVIDENCE}/compact-adr210-delivery-receipt.json` includes exact executable/source/log hashes, allocation counts, prologue measurements and process exit codes. Baseline/after executables: `${LOCAL_EVIDENCE}/compact-adr210-{baseline,boxed}-stack`; raw disassembly and repeatable extraction script `${LOCAL_EVIDENCE}/compact-adr210-disassemble.py`; strict proof log `${LOCAL_EVIDENCE}/compact-adr210-strict-final.log`.

Final status: accepted implementation, all focused checks and both strict original proof paths passed (exit0). Receipt SHA-256: `6b3ad08bed8199bb0943e34b11de9691f96f402109915c04d44f2a34cbbc8ef9`. Runtime ownership released; no further source edits.


### Runtime storage fix integrated — 3404c30c (2026-10-06)

ADR210 / #314 is integrated as signed/DCO `3404c30c` from `111fe5f3`. One uniquely owned private allocation retains RecordingState across consuming steps. The public API, recorded outputs, generated source, schema 20 and ABI 49 are unchanged. No shared state, per-leaf allocation, context clone or unsafe product code was introduced.

Measured aarch64 Rust 1.99 debug evidence: frame size 5,880 → 8 bytes; original recorded withdrawal stack reservation 2,068,064 → 76,352 bytes; both red/blue paths on default and explicit 2 MiB workers change from abort to pass. Instrumented construction, finish, witness errors, call_local success/error and full calls each add exactly one allocation/free of 5,880 bytes. Constructor stack increases by 5,936 bytes. This does not establish a release performance bound or support arbitrary private payloads/recursion. The independent compiler structural-test 8 MiB limit remains separate.

Root focused receipt `${LOCAL_EVIDENCE}/compact-3404c30c-integration-receipt.json` passes 27 feature-enabled runtime library tests, the complete Coracle all-target test package including process-isolated stack regressions and 27 original withdrawal cases, and strict affected-package Clippy. Source receipt `${LOCAL_EVIDENCE}/compact-adr210-delivery-receipt.json` additionally carries 96 runtime tests and both unchanged strict original withdrawal proofs using retained ADR206 keys and separate artifacts. Receipt SHA-256: `6b3ad08bed8199bb0943e34b11de9691f96f402109915c04d44f2a34cbbc8ef9`.

A new full gate is now running at frozen root `3404c30c`: `${LOCAL_EVIDENCE}/compact-full-3404c30c`. A clean same-head portable Nix build runs in the reused merkle-root-recording checkout. Neither is a completed acceptance claim yet; last full/portable pass remains `a7e14034`. Last compiler inventory at `fbe42d3d` has 380/386 available APIs and six original-source gaps; this runtime-only change adds no API. No remote CI or push. User documentation remains untouched.



### Combined full and portable acceptance passed — 3404c30c (2026-10-06)

The frozen signed/DCO root `3404c30c965248d69f07e759e22902fc4faf5de6` passed the complete local gate: **374 commands, 176 fresh fixtures**, source/format/refusal checks, workspace/generated tests, strict Clippy, external consumers and proof/ledger validation. Final consumer/proof/ledger stage:1,314.527 seconds. This combines ADR206 original withdrawal and sealed execution identity/ABI49, ADR207 qualified/immediate merge, ADR208 direct pure boundaries, and ADR210 private recording storage. The generated default-worker regression passes without the former runtime stack wrapper.

The fixture subset has365 of371 proof-required APIs recorded; the wider whole-source inventory has **380 of386 available**, with six explicit original-source gaps. Different source sets explain the denominators; neither count means complete behavioral coverage. Inventory217sources/755exports/196compiledroots/369nonproof/1,024declarations has zero unassessed, missing metadata, unmatched exports or baseline drift.

Both original withdrawal paths and both merge paths now pass the full main-checkout proof gate with actual upstream components, separate Dust, default-strict validation/application and replay refusal. Withdrawal also checks exact wrong-placement proof rejection and fallible ReadMismatch rollback. Prior game/coin state is explicitly seeded; older unbalanced smoke cases retain their documented limits. No funded full application lifecycle or current-head live-wallet claim.

The clean same-head Nix package passed **18 portable verification commands on aarch64-darwin**: relocated paths with spaces, relative installer symlink, six binaries without Nix linkage, unset runtime/Scheme overrides, default TypeScript, strict Rust, bundled ZKIR keys, offline Cargo consumers, exact copied runtime including private recording storage and placement, merge/send/canonical/transient capability checks and original Coracle guess/withdraw capability subset. Archive SHA256 **c25044e821ab7fd8c87596ec083d005311fa4465375a5c5a138d1f886b4940e1**.

- Combined receipt: `${LOCAL_EVIDENCE}/compact-3404c30c-full-integration-receipt.json`
- Full receipt: `${LOCAL_EVIDENCE}/compact-full-3404c30c/receipt.json`
- Wider inventory: `${LOCAL_EVIDENCE}/compact-3404c30c-inventory.json`
- Portable receipt: `${LOCAL_EVIDENCE}/compact-3404c30c-portable-final/receipt.json`
- Archive: `${LOCAL_EVIDENCE}/compact-3404c30c-portable-final/compactc.zip`

All six commits sincea7e14034 have valid GPG signatures and DCO trailers. Schema20/ABI49. Main tree retains only the user-owned documentation edit; no push, remote CI or registry publication. Issues remain open for final acceptance. Root now proceeds to signed ADR209 integration; its later focused evidence must remain separate from this same-head full checkpoint. Remaining source cohort will use focused gates, followed by combined full/portable acceptance at its completion.
