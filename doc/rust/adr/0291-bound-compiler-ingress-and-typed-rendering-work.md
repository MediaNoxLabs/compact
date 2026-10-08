---
id: RUST-ADR-0291
alias: ADR-0291
source_sha256: d75703d3b84648545b58a6af8808ecf113c775dd5e926ad6e30f20b32f209d07
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0291 — Bound compiler ingress and typed rendering work

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted staged prototype, 2026-10-07. Parent R030-17/#361 and R030-18/#362; milestone3. This decision precedes implementation. Numeric ceilings remain pending the complete183-source census and ordinary-worker calibration. Production port requires explicit root release after ADR0288 source freeze. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0291 — Bound compiler ingress and typed rendering work

Status: accepted staged prototype, 2026-10-07. Parent R030-17/#361 and R030-18/#362; milestone3. This decision precedes implementation. Numeric ceilings remain pending the complete183-source census and ordinary-worker calibration. Production port requires explicit root release after ADR0288 source freeze.

### Problem

The public renderer accepts an already constructed recursive Contract without explicit structural or call-work limits. Stdin in src/main.rs uses read_to_string without a byte ceiling; compactc and compact-rustc read complete frontend IR files. serde_json's existing depth guard does not cover input bytes, typed callers or interprocedural work. Cycle guards terminate cycles but repeated acyclic helper calls can amplify analysis work. ADR0282 fixed a supported-source ordinary-worker abort; it did not qualify arbitrary depth.

Read-only baseline1705fc1a evidence is recorded in [Compiler resource boundary proposal — 2026-10-07](references-0.3.0.md#note-147).19 finite subprocess probes had no abort or timeout.48 available schema20 files are only a preliminary subset, not the required full183 census. No unsafe fallback, hosted exploit or universal denial-of-service guarantee is claimed.

### Decision and staged ownership

Build an isolated prototype from exact frozen1705fc1a source. One shared bounded-read owner handles stdin and IR/contract-info file bytes; no stat-only trust, silent truncation or disabling serde's recursion guard. Count actual bytes using a bounded reader before deserialization.

Use an iterative borrowed typed visitor, exhaustive over relevant IR families, to count nodes, strings, local syntax depth, declarations and call references. Budget worklist capacity before adding child collections, not after allocating an unbounded child batch. Do not serialize or clone the whole Contract. Error summaries must not recursively format rejected input. Saturating counters and graph arithmetic prevent overflow.

Account for call-chain depth across callable boundaries, separate from isolated syntax depth. Count repeated call occurrences in a saturating graph-work estimate; unique edges alone cannot bound repeated expansion. Preserve native schema precedence and existing unknown/cyclic semantic diagnostics. Structural budget refusal is a resource-envelope decision, not recording-profile rejection or a new evaluator. Existing semantic cycle/arity/type/scope guards remain authoritative.

The production API shape, any resource error variant and exact numeric defaults require review after measurement. No unlimited public switch, catch_unwind, stacksize override, runtime/VM rewrite, or new global codegen setting. Caller construction/drop of arbitrary owned recursive data and frontend subprocess resources are separate boundaries; document limits honestly.

### Before and intended after

Before: complete unbounded ingress read, recursive analysis/lowering without explicit resource envelope; supported historical default-worker regression remains tested.

After candidate acceptance: all183 admitted outputs remain exact under measured defaults; oversized bytes/work refuse deterministically before expensive recursive lowering. Valid bounded inputs preserve old semantic diagnostics, witness/effect ordering and emitted bytes. A large finite input is refused rather than partially rendered or silently accepted. This is not OS sandboxing or whole-pipeline time/memory enforcement.

### Numeric policy pending evidence

No proposed number is adopted by this ADR. Collect exact typed visitor measurements for every183 supported source closure plus original DID/passport. Report maxima and responsible fixtures. Calibrate ordinary default-worker syntax/call-chain shapes, including mixed local nesting across helper boundaries. Candidate budgets from the research note are test points only. If safe ceilings conflict with existing supported sources, diagnose the specific frame/work owner; do not enlarge worker stacks or weaken corpus acceptance.

### Required validation

- Shared reader exact-limit/one-over/short-read/UTF8/file-growth cases, schema/parser precedence and existing output preservation.
- Iterative visitor exact/over limits, all recursive IR families, unknown/unselected/unused nodes, pending-worklist bounds and saturating arithmetic using tiny limits.
- Local graph chains, diamonds, repeated edges, disconnected declarations, cycle and missing callee; occurrence counts and cross-call depth; deterministic declaration-order-independent metrics.
- All183 valid source closures measured by the exact visitor and retain Rust/capability bytes. Existing ADR0282 ordinary-worker control stays unchanged.
- Finite generated property cases with fixed seeds/budgets, minimized failures, bounded subprocesses and no core dumps. No intentionally huge inputs.
- Scratch-only bypass/counting mutants must fail precise guard tests.
- Paired preflight/render cost measurements on small controls/DID/passport with exact compiler/lock/toolchain/platform, spread, and explicit startup exclusion. No speed claim or timing assertions from single samples.
- Focused tests/Clippy and retained failures/receipt. No livecompiler port until root approves constants and releases ADR0288 ownership. No CI/push/commit by this lane.

### Planned files

Isolated new compiler resource module (shared reader + borrowed visitor/graph metrics), surgical lib.rs entry point and three ingress callers src/main.rs, src/bin/compactc.rs and src/bin/compact-rustc.rs, focused tests and scratch measurement driver. Production stays unchanged during prototype. Any later port is coordinated separately.


Tracking: https://github.com/MediaNoxLabs/compact/issues/415 (milestone 3). Created and verified before prototype implementation.

### Prototype checkpoint

Exact183-source census and10 focused guards passed in isolated source. Ordinary-worker sweep exposed five existing baseline stack aborts; pure expression frame135,008 bytes. Numeric defaults remain unaccepted pending ADR0292 repair. See [ADR0291 resource preflight checkpoint — 2026-10-07](references-0.3.0.md#note-072). No production port.

### Prototype review and census correction — 2026-10-07

The root's cross-kind raw-name concern was reviewed against frozen 1705. Identical raw pure/stateful names already fail the existing global DuplicateCircuit check; the existing ADR0245 positive control is distinct raw names `a$b` and `a_b` in separate generated namespaces. No valid-program bypass from that specific raw-name collision was reproduced, and no new source refusal is authorized.

Concrete prototype gaps were found: StateAction::PureCall edges are omitted; VectorFoldCall references also need classification. The early return for a duplicate, unknown callee or cyclic component skips work/depth analysis of unrelated resolvable components. These findings concern the isolated metrics prototype, not a delivered resource guard.

Before numerical acceptance, graph edges must retain their semantic call kind: pure-only actions/folds, stateful CircuitCall, and Expr::Call resolved using the actual pure-versus-stateful expression owner. Preserve the valid cross-namespace normalized-name control. Recompute all 183 graph metrics after edge correction. Unknown/cyclic/duplicate findings must retain existing semantic diagnostics; do not silently treat incomplete graph totals as full validation, and do not add a blanket cycle/name refusal. The exact orchestration of bounded structural metrics, graph findings, and existing semantic precedence remains a reviewed implementation condition before production adoption.


### Correction to the prototype edge claim — 2026-10-07

Focused exact-edge tests showed that the original visitor already records both StateAction::PureCall and VectorFoldCall. The preceding amendment's claim that these edges were omitted was an inspection error; it is superseded by this correction. The initial attempted repair double-counted them, and the new tests correctly failed before that candidate could be accepted. The original census is not known to undercount these edges.

Remaining work: explicit call-kind resolution following existing semantic owners, and continued accounting of unrelated resolved components when another component has a duplicate, unknown target, or cycle. Recompute the census to verify whether valid corpus metrics change, without presuming an increase. Preserve the original global raw-name duplicate refusal and the distinct raw names/same normalized spelling positive control. No production guard or numerical policy is adopted.


### Graph review disposition — 2026-10-07

The original visitor already records PureCall and VectorFoldCall edges. Fifteen focused prototype tests now retain exact edge counts and expose the attempted double-counting error; no missing-edge defect remains asserted. Raw duplicate declaration names already receive the existing global semantic error; the valid normalized-name control remains admitted. The experimental call-kind refinement itself falsely classified five valid census fixtures, including constructor stateful calls, and is not adopted. Its failed census and corrected test logs are retained under `/tmp/compact-adr291/graph-v2/`. Original corpus maxima remain the working baseline.

Focused frozen ADR0292 controls place an unknown or cyclic unrelated circuit after the otherwise failing valid chain12. Both refuse during native rendering before recorded helper expansion: exactly UnknownCircuit("missing") and UnsupportedStatefulCall("bad"), without abort. Evidence `/tmp/compact-adr291/semantic-precedence/receipt.json`. Therefore this review does not establish an early-return recorded-expansion bypass. Stop speculative graph redesign; preserve semantic precedence. Numeric ceilings remain pending the separate measured ADR0296 legacy-frame repair.



#### 2026-10-07 — Isolated evidence gate correction

Root reproduced stale-output reuse in the evidence renderer helper, not production rendering: refusal exited zero and old output survived. The helper now exits nonzero, each fixture uses fresh output directories, and the gate requires both fresh source/capability files and no error. Corrected paired refusal controls and all 193 comparisons pass with zero changed outputs. Exact-limit file ingress now pads both IR and contract-info to 4 MiB; all four ingress tests pass again. Earlier 402 backend tests, strict Clippy and 18 Rust 1.88 worker tests cover unchanged production source.

Final isolated receipt SHA256: `8e4198cbb3f855668455bef7d28397e9535acab1a842ad1e071c8124ec158e1d`. Patch SHA256: `cb3e6fa00548ca2309dbd3cf7471722965e2254b5208f3efd2a84008241a11b9`. Corrected evidence archive SHA256: `94965ae7df3b2eb6cd2033afd51f8c1ecf05aef59185540bb20edffa988a7959`. Original root reproduction and superseded evidence are preserved. Numeric policy/API/live port remain awaiting root acceptance; this entry does not adopt them.

### Root decision: adopt bounded compiler policy — 2026-10-07

Root accepts the exact 18-file candidate and numerical limits in receipt 8e4198cbb3f855668455bef7d28397e9535acab1a842ad1e071c8124ec158e1d for live integration. Independent review verified all 18 source/base identities and 17 artifact hashes. The corrected 193-input comparison uses fresh per-fixture directories and nonzero render refusal; both stale-output negative controls and four exact-input ingress tests pass. Earlier 402 backend tests, strict Clippy and 18 actual Rust1.88 resource tests remain source-bound evidence; the test-harness correction does not imply a new full-suite run.

Policy:4MiB per typed-IR/contract-info ingress,32768 nodes,4096 pending nodes,262144 string bytes,65536 literal bytes,256 callable declarations,syntax depth48,call chain16,expanded depth64,expanded work131072 and1664 calibrated path units separately per rendering owner. Limits are conjunctive. Input reading, borrowed structural traversal and saturating graph analysis have separate owners. There is no unlimited flag, stack override, caller-name exemption or catch-unwind fallback. Public ResourceLimit reports resource/limit/observed; ADR0293 owns the package/error migration mapping.

These are practical supported-input ceilings calibrated on macOS arm64 with Rust1.99 and checked on1.88. They do not measure actual stack bytes or universally bound Scheme, rustc, prover, packaging, caller-owned construction/drop or all-host resource behavior. Final joined candidate and supported-host qualification remain required.

The live port follows dependency remediation e79c639f and includes the independent c8450ee3 unit slice. Run a fresh joined backend cohort and strict Clippy under the actual live lock. Later ADR0295 relation admission must pass the same resource guard before acceptance; no proof repetition is needed merely for unchanged emitted outputs.

### Signed local delivery

ADR0291/#415 delivered at `03445d6bdb680c78ee617da4939fbde02b7546bf` with conventional GPG+DCO verified. All 18 owned paths exactly match the reviewed isolated candidate; the live port joins ADR0301 dependency updates and ADR0302 unit tests.

Live validation: **414 backend tests**, strict all-target/all-feature Clippy and owned-path Rust 2024 formatting pass. The isolated actual Rust 1.88 resource cohort has 18 passing tests. A freshness-checked comparison shows all 193 existing generated Rust/capability pairs unchanged. The earlier stale-output comparison flaw and its rejected evidence remain recorded; only the corrected fresh-output gate is accepted. No proof rerun was needed for byte-identical output.

Compiler input/metadata is bounded to 4 MiB, AST nodes 32,768, pending nodes 4,096, strings 262,144 bytes, literals 65,536 bytes, callable declarations 256, syntax depth 48, call depth 16, expanded depth 64, expanded work 131,072 and each renderer owner 1,664 calibrated path units. Public ResourceLimit errors identify resource, limit and observed value. Existing semantic diagnostics retain ownership. These are finite compiler policies, not universal stack/DoS guarantees; final host qualification remains open. ADR0293 owns the public error/package migration.

[ADR0291 — Signed resource policy delivery.zip](references-0.3.0.md#note-073) SHA256 `eafcdb1cfef8f75d7bc5a76e7f61b8c9321c8aff8b910730eb9f498196ee2f40` retains exact source, live receipt and logs. Whole-workspace cargo fmt additionally found preexisting formatting in map_nested_product_composition.rs; this is tracked for a separate formatting-only correction. An initial diff-check included the user's ledger document trailing blank line; the scoped check then passed and that document was preserved byte-for-byte. No broad formatting success is claimed.

Parents R030-17/#361 and release qualification remain open; 6/20 parents accepted. No push or CI. User ledger documentation SHA256 remains `6e126928f8fbd8b48bf16984b419e421b63a84b3db3013cc2b6c598a9ee3a8ce`.
