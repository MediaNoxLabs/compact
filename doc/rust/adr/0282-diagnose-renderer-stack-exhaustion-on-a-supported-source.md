---
id: RUST-ADR-0282
alias: ADR-0282
source_sha256: 59c75d166c426f6887c6d93880e6443224b81d0f3bc0c4a7e88747c4c110ba4f
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0282 — Diagnose renderer stack exhaustion on a supported source

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted for bounded root-cause diagnosis and isolated candidate only; production fix requires root review. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0282 — Diagnose renderer stack exhaustion on a supported source

- Status: accepted for bounded root-cause diagnosis and isolated candidate only; production fix requires root review.
- Date: 2026-10-07
- Parent: R030-17 / #361; separate from locally reviewed R030-05 ownership acceptance.
- Milestone: rust-backend-v0.3.0

### Problem and actual evidence

At exact committed backend `d5d4a6c935f5ada7d20d43adf06126d601db19d4`, the unchanged original `shielded-send-schema20-ir.json` parses on the main thread, enters the real public `render_with_capabilities` API on a default std::thread worker, then aborts with stack overflow. The same wrapper binary and fixture on an explicit16MiB worker succeed with four capability rows. Default Rust worker configuration normally uses2MiB; actual OS allocation was not independently measured in this probe. `RUST_MIN_STACK` is unset.

The host is aarch64 macOS26.2, Rust1.99.0, dev/unoptimized+debuginfo, incremental off. Separate subprocesses have cores disabled and60-second timeouts, no catch_unwind. Neither timed out. All302 registry package version/source/checksum identities match committed root Cargo.lock. A preliminary historical differential-renderer lock run is retained separately and superseded, not counted as exact-root acceptance.

Exact receipt SHA256 `fae3cc6a1e48715fb9208255d9da4524d3284cbf0f8f5e2ffad84a014bbdf3f4`; binary `e3407574d1b87bd0c69b241939e750f33b1baee3c66d1aa58203e5ba447aab69`. This is a measured local renderer resource limitation, not a demonstrated exploit, network/CLI security claim, or reachable final syn-parser assertion.

### Decision and bounds

Use the frozen exact-source snapshot and pinned dependency closure for bounded local stack/frame/backtrace inspection. Initial diagnosis may reuse the existing binary without compiling. Request the shared warm target lease before any isolated candidate compile; no fresh dependency target given current disk limits.

Identify the responsible large frame or recursive evaluation path before proposing a minimum helper/ownership extraction or iterative lowering change. Preserve source semantics, Rust output and capability output exactly. Do not increase default stack as acceptance, catch and mask the abort, rewrite the VM, weaken admission, or redesign resource budgets in this ADR. No production source changes until root reviews the concrete cause and proposed candidate.

### Before / intended after

Before: supported original typed IR → default worker render → process stack overflow; explicit16MiB diagnostic control succeeds.

Candidate acceptance, if approved: the same public API and fixture succeed on an ordinary default worker, Rust/capability bytes equal the frozen successful control, and the responsible reduced path has meaningful regression coverage. Source diagnostics, scopes, witness/effect ordering and all existing supported shapes remain unchanged. No generated API, runtime, ABI50, IR20 or package version change is planned.

### Alternatives

- Raising stack only: useful control, not the accepted fix.
- catch_unwind: stack abort is not a recoverable ordinary panic and masking failures would not establish valid output.
- Broad iterative compiler/resource-limit redesign: deferred until specific evidence warrants it.
- Another cosmetic file move: insufficient unless it changes the measured frame/lifetime behavior while preserving owner boundaries.

### Validation and reporting

Retain source/lock/binary hashes, toolchain/profile, stack configuration, subprocess exit/status and stderr markers. Record failed candidate attempts honestly. After diagnosis root decides the smallest production slice and its required focused/default-worker/output-equality checks. Current acceptance authorizes diagnosis, not issue closure or universal resource-safety claims.

### Durable evidence

[ADR0282 — Exact-lock renderer stack — REPORT.md](references-0.3.0.md#note-055) and [ADR0282 — Exact-lock renderer stack — receipt.json](references-0.3.0.md#note-056) link the final exact-lock finding. Adjacent evidence notes preserve source inventory, wrapper source/lock, build log, both run logs and registry comparison. Raw scratch remains `/tmp/rust030-render-stack-d5`.

### Tracking

Milestone3 issue: [MediaNoxLabs/compact#406](https://github.com/MediaNoxLabs/compact/issues/406). Created after this ADR and exact evidence were physically verified, before stack diagnostics.

### Existing-binary diagnosis and proposed isolated candidate

LLDB places the guard-page access in pure expression lowering after eleven native expression activations. Measured dev aarch64 frames172,640bytes and135,008bytes sum2,034,048bytes before higher callers. See [ADR0282 — DIAGNOSIS-PROPOSAL](references-0.3.0.md#note-054) and preserved prologues. A thin structural dispatch/control-aggregate helper candidate is proposed for root review; no candidate or production change is yet approved or compiled. No infinite recursion or final syn assertion finding is claimed.

### Isolated candidate result — root review pending

Default worker and16MiB control now both succeed for the original source, with exact Rust/capability equality. Thin dispatcher192bytes, control16,560bytes and aggregate15,424bytes; remaining operation141,040 and pure135,008byte frames retain an explicit limit. All38 original arm bodies are verbatim, dependency lock unchanged, no root source edits. See [ADR0282 — Isolated candidate — CANDIDATE-REPORT](references-0.3.0.md#note-058) and [ADR0282 — Isolated candidate — candidate-receipt.json](references-0.3.0.md#note-059). Production port and full regression/corpus acceptance remain pending root review.

### Root promotion decision and permanent regression plan

Root approved the thin-dispatch candidate for production promotion **after ADR0278 releases the compiler source freeze**. Ownership is limited to stateful/expression.rs and tests/recording_profile_attempt.rs. Private control/aggregate/operation helper documentation now makes boundaries explicit. Rustfmt preserves all38 original arm bodies under whitespace-only comparison. The existing profile test wrapper will perform ordinary parse/render on the default test worker instead of creating a16MiB nested worker; no RUST_MIN_STACK override.

The formatted isolated patch is prepared. Its first focused test build stopped before compilation with ENOSPC in the shared bench target; this is an environment/setup failure, not test acceptance. The earlier unformatted isolated candidate default/16MiB output-equality evidence remains retained. Await available disk/target and source-freeze release. Final production validation will include the coordinated backend/Clippy/183-output gate and separate default-worker receipt.

### Formatted scratch regression validation

After root freed confirmed-idle build products, all5 existing recording_profile_attempt tests pass on ordinary default test workers with RUST_MIN_STACK unset; selected integration-target strict Clippy also passes. Native error/action precedence and hidden unused/unselected branch refusals remain intact. The earlier ENOSPC setup attempt is retained. All38 original arm bodies remain whitespace-equivalent after rustfmt. See [ADR0282 — Formatted scratch validation.json](references-0.3.0.md#note-057). No production port yet; await ADR0278 source release and root joint gate.

### Delivered locally — 2026-10-07

ADR0281 fixes a reproduced Rust parameter namespace collision in both pure and stateful emission. One shared allocator reserves semantic identifiers; original source binding keys, declaration order and diagnostic precedence remain intact. Before-fix consumers fail E0415. After-fix generated consumers compile, and pure consumers execute both selected arguments correctly. Raw Rust identifiers in these cases enter through typed IR; no claim that the Compact text frontend admits them.

ADR0282 fixes the reproduced original shielded-send renderer abort on an ordinary debug worker. Thin native expression dispatch reduces repeated control/aggregate frames; all 38 original arm bodies retain their behavior. The permanent profile tests now render on ordinary workers with no enlarged nested stack. Exact-lock original default-worker failure, 16 MiB diagnostic control, frame diagnosis and successful candidate evidence remain preserved separately. Large remaining operation/pure frames and arbitrary-depth resource qualification remain open.

Combined final production sources passed 336 backend tests with RUST_MIN_STACK unset, strict all-target/all-feature Clippy, and identical complete Rust/capability output across 183 source closures. Joint evidence covers both fixes on top of ADR0278. No runtime, ABI, schema or dependency changes; existing generated consumers and proof receipts remain applicable without another identical proof run.

Signed conventional/DCO commit `1f9b13944c1ac06d983e952cb1a130750e2a9ee1`. [ADR0281-0282 — Signed integration receipt](references-0.3.0.md#note-053) preserves exact combined validation scope. Parent acceptance remains 5/20.
