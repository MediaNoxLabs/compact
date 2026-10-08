---
id: RUST-ADR-0292
alias: ADR-0292
source_sha256: 69beb29125885aaf563059fb407172bd78f31dc01b4d537e9ee9d15371ad5067
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0292 — Reduce recursive pure renderer frames

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted bounded isolated repair, 2026-10-07. Parent R030-17/#361 and R030-18/#362. Production port waits ADR0288 compiler freeze and root release. This record precedes candidate code changes. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0292 — Reduce recursive pure renderer frames

Status: accepted bounded isolated repair, 2026-10-07. Parent R030-17/#361 and R030-18/#362. Production port waits ADR0288 compiler freeze and root release. This record precedes candidate code changes.

### Problem and measured cause

ADR0291's full 183-source ordinary-worker calibration exposed five stack aborts in the unchanged frozen 1705fc1a renderer: both original passport source closures, transient_receive_send_oracle, original Coracle and original microDAO. Baseline rendering executes before the resource preflight in the instrumented subprocess, so these are not a resource visitor failure. A finite dynamic If fixture passes at depth 8 and aborts at depth 16; the same depth 24 passed on the main thread.

LLDB localizes dynamic If16 to expression_with_calls's stack probe after 15 recursive pure-expression activations. Passport reaches syn::GenericArgument::parse underneath 16 pure-expression frames. The aarch64 debug prologue reserves 135,008 bytes (32 + 0x20000 + 0xf40). This is the remaining pure frame already identified by ADR0282, not evidence of a syn semantic defect.

Raw stage logs, exact input/binary/source hashes, full census and traces are preserved in [ADR0291 resource preflight checkpoint — 2026-10-07](references-0.3.0.md#note-072). Cores disabled; finite subprocesses; no stack override or caught panic. No arbitrary-depth or release-stack failure claim.

### Decision

Prototype a thin pure-expression dispatcher and cohesive control/aggregate/operation helpers in frozen isolated source. Preserve every existing arm body, internal recursion through the dispatcher, match selection and early diagnostic order. No evaluator rewrite, admission widening, arithmetic/VM change or new macro. Start with one responsibility: prevent every recursive structural pure node from reserving all operation temporaries.

Measure each resulting frame. If supported failures remain, localize before enlarging the scope. Do not reject previously supported inputs through ADR0291 limits merely to hide this failure. Do not raise thread stacks, catch_unwind or add a public abstraction switch.

### Before and intended after

Before: supported original IR renders on main thread but five normal debug-worker subprocesses abort; dynamic If16 also aborts.

After required candidate acceptance: all 183 existing sources and finite selected dynamic controls complete on ordinary default workers; generated Rust and capabilities are exact against the original successful main-thread baseline. The existing shielded-send ADR0282 regression remains passing. Thin helper frames replace the repeated kitchen-sink frame; no generalized memory or time guarantee is claimed.

### Validation

- Preserve original failed baseline logs and root-lock package identity.
- Exact arm-body/order comparison before/after; rustfmt may change whitespace only inside moved arms.
- Independent ordinary-worker subprocesses, RUST_MIN_STACK unset, no stack override, core dumps disabled and bounded timeouts. Exact 183 Rust/capability identity and dynamic If controls.
- Focused existing pure/native/recorded diagnostic tests and strict Clippy; source hashes and frame prologues retained.
- No cryptographic proof rerun when admitted bytes are identical. Any emitted-body change is investigated rather than silently refreshed.
- Only after root review and compiler handoff may the precise patch be ported; resource defaults remain pending ADR0291 calibration.

### Ownership

Isolated lib.rs expression_with_calls and possible private pure expression helper owner, focused tests/measurement wrapper only. Runtime/IR/ABI/generated API and contract source are unchanged. Coordinate future backend SemVer/non_exhaustive error work with ADR0293; it is not part of this mechanical frame repair. No production commit, CI or push from this lane.


Tracking: https://github.com/MediaNoxLabs/compact/issues/417 (milestone 3). Created before isolated candidate edits.

### Approved scope amendment — 2026-10-07

The isolated pure-only repair passes the twelve mixed operation/control probes and four of the five previously failing corpus sources. Original Coracle still fails on an ordinary default worker. The retained debugger backtrace and prologue locate a second recursive owner: recorded `Plan::expression`, whose measured aarch64 debug frame is 79,056 bytes. This is a baseline-supported source failure, not a preflight rejection or a reason to exclude the source.

The approved repair now also splits this method into a thin dispatcher and cohesive methods on the same `Plan`. Original arm bodies, guard order, scope/frame borrowing, emitted ordering, and refusal behavior must remain unchanged. This does not widen admission or add an evaluator/runtime API. Preserve the failed pure-only candidates and measurements. Acceptance remains all 183 sources on ordinary workers, identical generated Rust/capabilities, twelve finite mixed controls, focused tests and strict Clippy. Live compiler port remains blocked until ADR0288 ownership is released; all current edits are isolated.


### Isolated candidate validated — 2026-10-07

183/183 ordinary-worker sources and exact Rust/capability equality; 12 mixed controls; 338 unique focused tests and strict Clippy, including the maintained recorded If20 regression. All 87 moved arms preserve normalized tokens. Root still owns production port authorization after ADR0288. Numeric resource ceilings remain pending ADR0291. See [ADR0292 isolated renderer frame repair — 2026-10-07](references-0.3.0.md#note-074); evidence archive SHA256 `f7f92969b76731d9e3594eb1dfd34d9f15752a69b914da5aaf7894afffeb9b00`.


### Live port validated — 2026-10-07

After ADR0288 release, three owned paths ported onto 8a52a010 without changing its additions. Full backend 366 tests, strict Clippy and 184/184 ordinary-worker Rust/capability equality pass. Five maintained worker tests include the witnessed Boolean Cell recorded If20 regression. No commit by this agent; root owns signing. See [ADR0292 live renderer repair delivery — 2026-10-07](references-0.3.0.md#note-075). Receipt SHA256 `ade455dad4d474446b5b27f59cde68e44876f802ebc11cc86a2f8737659447f1`.


### Signed integration — 2026-10-07

Root integrated this repair as `926d2bf56b7e7d8532e4d8e03f1806d106aefb1b` with verified GPG and DCO. Live receipt remains bound to pre-commit base 8a52a010 plus its three exact owned source hashes; root verified those hashes before signing. IR20/ABI50 unchanged.
