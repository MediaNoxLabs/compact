---
id: RUST-ADR-0082
alias: ADR-0082
title: "Gate Rust recording by compiler proof applicability"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "capabilities", "proof-applicability", "validation"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 93c43072cc36878071c3643386b61d516946215fcd0d81d1ad502457544d6a82
---
# RUST-ADR-0082 — Gate Rust recording by compiler proof applicability

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted schema-3 capability finalization using authoritative compiler proof flags, with exact exported-name joins and no implicit proof-false default. The implementation retains a schema-2 unclassified renderer draft for diagnostics and a proof-aware publication wrapper. Strict recording applies to proof-required exports; historical raw versus proof-required denominators remain distinct.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#182 closure](https://github.com/MediaNoxLabs/compact/issues/182#issuecomment-6017539007). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`4656cd31`](https://github.com/MediaNoxLabs/compact/commit/4656cd31cb411bc1ada7f808bdb996df476a7b9d) · [`706d251e`](https://github.com/MediaNoxLabs/compact/commit/706d251e7281ffc78a598be32b3287002ca1e753) · [`970cf105`](https://github.com/MediaNoxLabs/compact/commit/970cf10540ab28142f7b176236bafd048c0aa036) · [`e5df4782`](https://github.com/MediaNoxLabs/compact/commit/e5df478287dad318e010852f57fd80ca0a57b046). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 82
status: accepted-partial
date: 2026-10-05
milestone: rust-backend-v2
```

## Historical decision and amendments

### Problem and evidence

`rust-capabilities.json` schema 2 reports `recorded` and `observed_call` for every exported stateful circuit, and `--rust-require-recording` rejects every false value. That treats native-only private computations as missing proof APIs. The compiler already declares proof applicability in `compiler/contract-info.json`: its `circuits[].proof` comes from the frontend's `proof-circuit-name*` derivation in `compiler/passes.ss`, the same set consumed by TypeScript `ProvableCircuits` and ZKIR generation. Derive applicability from this compiler product, not from the Rust lowering reason, absence of ledger actions, or generated TypeScript text.

Exact local cross-tab at integrated HEAD `4656cd31cb411bc1ada7f808bdb996df476a7b9d`, immutable Rust compiler `${LOCAL_EVIDENCE}/adr79-adr80-compactc` SHA-256 `e25c3992b900d0a2ce5a2b7840be7b206814f1d9f3618d25e532095b510d3ca6`, and packaged TypeScript compiler `${HISTORICAL_NIX_STORE}/pvwmk79f78h54kjafymkhsix3a3g8pca-compactc/bin/compactc`: all **333** exported rows from **137** fixture entrypoints joined uniquely by circuit name to their own `contract-info.json`, with zero unmatched or duplicate rows.

| Compiler proof flag | Rust recorded and observed | Rust unavailable | Total |
|---|---:|---:|---:|
| `true` | 170 | **116** | **286** |
| `false` | 0 | **47** | **47** |
| Total | 170 | **163** | **333** |

Thus **163/333** remains the historical *raw unavailable* count, while **116/286** is the actionable gap for proof-capable exported circuits. It is incorrect to call all 163 missing proof APIs. No recorded-true/proof-false anomaly exists in this snapshot. Of 47 `StateReturn::Expression` first-blocked circuits, 46 are proof false and one is proof true (`merkle_path_verify.verify`). Another proof-false circuit, `assert_witness.checked_value`, first blocks at `StateAction::Assert`; no heuristic over action count or return node can replace the compiler flag. The fixture corpus is curated and does not cover all 677 repository Compact files or establish behavioral parity.

### Developer-facing behavior

`boolean_logic.compact::witnessed_both` calls a witness and discloses a Boolean. TypeScript exposes `contract.impureCircuits.witnessed_both(...)` but excludes it from `ProvableCircuits`; its `contract-info.json` says `proof:false` and it has no ZKIR. Rust already supports native `contract.witnessed_both(context, left, right)?`. The desired change makes `compactc --target rust --rust-require-recording` accept this contract without inventing a `contract.recording.witnessed_both_call(...)` proof call. `witness_ledger_cell.compact::private_check` similarly calls a witness that reads a ledger view, yet its compiler proof flag is false; a witness read alone does not require a public proof. `merkle_path_verify.compact::verify` is the converse: TypeScript includes it in `ProvableCircuits`, `contract-info.json` says `proof:true`, and `verify.zkir` exists. It must still fail the strict Rust gate until a recorded method and observed call exist (ADR-0083).

Before, a representative unavailable row is:

```json
{"schema_version":2,"name":"witnessed_both","recorded":false,"observed_call":false,"recording_unavailable":{"code":"unsupported_return","ir_node":"StateReturn::Expression"}}
```

After, preserve the booleans as API availability, add compiler proof applicability and a derived status:

```json
{"schema_version":3,"name":"witnessed_both","proof_required":false,"recorded":false,"observed_call":false,"recording_status":"not_applicable"}
{"schema_version":3,"name":"verify","proof_required":true,"recorded":false,"observed_call":false,"recording_status":"unavailable","recording_unavailable":{"code":"unsupported_return","ir_node":"StateReturn::Expression"}}
```

The exact nested reason fields should retain schema-2 diagnostic detail. If useful, retain a separate optional raw lowerer diagnostic for proof-false rows; it must not be presented as an actionable proof gap. Existing consumers of `recorded`/`observed_call` still read their original meaning. Schema 2 readers must reject schema 3 until updated; no silent reinterpretation. A status of `available` requires both APIs on a proof-required row. The status is `not_applicable` when `proof_required=false`, regardless of whether a future native recorder happens to exist.

### Decision and ownership

Version the capability report **2 → 3**. Feed compiler-neutral proof applicability from the frontend-generated `compiler/contract-info.json` into the Rust capability finalization path. Join on the exact exported external circuit name within each compiled contract; reject missing, duplicate, ambiguous or inconsistent rows rather than assuming a default. Keep the renderer's typed lowering and `recording_unavailable` reasons as independent API evidence. Prefer one finalization function used by CLI and library callers so schema 3 cannot diverge between paths; the integration point may receive a validated proof-name set or contract-info-derived map. Do not parse generated TS or infer proof from Rust IR shape. Private Rust IR schema 8 and runtime ABI 35 need no change for this report/gate-only decision.

`--rust-require-recording` fails only when `proof_required=true` and either recorded or observed-call API is absent. It still fails on invalid/missing proof metadata. The local inventory and receipt must ingest `proof_required`, publish both raw availability and actionable proof-capable counts, and use proof-required rows for `--require-full`. Keep source membership, compiler coverage, behavior, ZK proof and ledger-application gates separately explicit. No generated contract body or runtime method is changed by this ADR.

### Acceptance and unresolved details

1. Report one-to-one proof joins and schema-3 compatibility tests, including renamed/export-aliased circuits, missing and duplicate contract-info names, and invalid proof values.
2. Prove on the measured corpus the 170/116/47 matrix and zero recorded-true/proof-false rows; persist exact compiler hashes and manifest scope in the receipt. Counts may change as code is delivered, but invariants and denominator semantics remain.
3. Strict-gate `boolean_logic.witnessed_both` and `witness_ledger_cell.private_check` as proof-false positives; reject proof-true `merkle_path_verify.verify`. A `proof:false` `assert_witness.checked_value` tests that the gate is metadata-driven, not ReturnExpression-driven.
4. Inventory test distinguishes historical raw 163/333 from actionable 116/286, rejects unknown proof status, and does not claim TS/Rust behavior parity from API presence.
5. Run targeted renderer/CLI/inventory tests and all 137 fixture/report consistency checks. No VM, transaction or proof behavior changes are expected; broader integration remains a milestone gate.

Open question: direct Rust renderer callers currently may not have `contract-info.json` at their entry point. The implementation must define a required validated proof map or explicit report-finalization phase; it must never silently tag unknown rows proof false. The exact schema-3 field naming may be refined before code, while retaining the semantic distinctions and version bump above.

### Follow-up boundary

`merkle_path_verify.verify` is a genuine proof-required gap. Its typed Merkle `check_root` recording and proof/application semantics are proposed separately in [ADR-0083 — Record Merkle root verification through typed slots](0083-record-merkle-root-verification-through-typed-slots.md). Neither this ADR nor its counts claim that implementation is complete.


### Tracking

Focused rust-backend-v2 issue: https://github.com/MediaNoxLabs/compact/issues/182. This ADR is proposed and has no code delivery yet.

### Exported-row join clarification — 2026-10-05

Require **each exported Rust capability name** to match exactly one `contract-info.json` circuit entry with a Boolean `proof` flag. Additional `contract-info.json` entries for nonexported pure/helper circuits are valid and must be ignored for the Rust exported-capability join. For example, `boolean_logic` includes helper metadata `invert`, `both`, and `either` in addition to exported stateful rows. Duplicate metadata names remain errors when they collide with an exported capability. This clarifies one-to-one exported-row coverage; it does not require the two files to have equal name sets.



### Local delivery — 2026-10-05

Conventional GPG-verified/DCO commit `9746f6f1b44b4aa05eaafc4f951c1498930d9855` delivers this decision in an isolated local worktree. Published `rust-capabilities.json` is schema 3 with `proof_required` and derived `recording_status`; existing `recorded`/`observed_call` and typed reasons retain their meanings. The CLI uses `render_with_proof_capabilities`, which requires validated frontend `contract-info.json`. The library's one-argument `render_with_capabilities` remains an explicitly unclassified schema-2 lowering draft for diagnostics; callers needing a publishable report use the proof-aware wrapper. Exported names must each join exactly one Boolean proof flag; helper-only metadata rows are allowed. No implicit proof-false default exists. The strict CLI gate ignores proof-false missing APIs and still rejects proof-true gaps. Private Rust IR schema 8 and runtime ABI 35 are unchanged; generated Rust source is byte-identical.

Exact post-commit receipt `${LOCAL_EVIDENCE}/adr82-9746f6f1-receipt.json` binds the immutable compiler SHA-256 `628c96cce5dc1bc77671cbdcf6d529134fea1b4434192bef011a72db255884ac` to that commit: 167 checked sources, 137 compiled fixture entrypoints, 648 declarations, 333 joined capability rows, **170 proof-required available / 116 proof-required unavailable / 47 proof-false unavailable**, 163 raw unavailable, zero unmatched, zero baseline drift, and 114 exported source declarations unassessed outside compiled entrypoints. The receipt is local API evidence, not semantic TypeScript parity.

Validation: 5 Rust unit tests, 68 renderer tests, 6 Python inventory tests, 137 fresh fixtures with zero stale output, source rejection/output publication/strict proof-capability gate, compactc target and manifest gate, package all-target Clippy with `-D warnings`, fmt and diff checks. Strict recording accepts full `boolean_logic` and `witness_ledger_cell`, plus a standalone `checked_value` source preserving the original assertion body; it rejects `merkle_path_verify.verify`. The full `assert_witness` contract still fails because its *other* circuit `checked_write` is proof-required and unavailable. The commit is local and unpushed; broader same-revision CI, semantic parity and final milestone gates remain open. Root's newer `local_parity_gate.py` on the main branch hardcodes schema 2 and needs schema-3 adaptation during integration; it was intentionally not edited in this isolated branch.


### Integrated gate checkpoint — 2026-10-05

Cherry-picked as signed/DCO `970cf105`; the local receipt harness was adapted in `e5df4782`. At exact integrated HEAD `706d251e7281ffc78a598be32b3287002ca1e753`, the full local gate passed schema-3 report validation and all 137 compiled fixtures. The matrix is 186/286 proof-required available, 100/286 proof-required unavailable and 47 proof-false native-only; raw availability is 186/333 and 147/333 unavailable. Receipt `${LOCAL_EVIDENCE}/compact-local-full-abi36-706d251e/receipt.json` binds frozen compiler SHA-256 `f5c8e80e3b50a9b6aee8425b928e55fb98bdc55dba6479e5f53e7270da5b1db8`. The full gate scopes Cargo integration to backend packages plus Compact CLI unit tests because legacy compactup integration cases depend on installed local toolchains and mutable GitHub releases. This gate does not establish source or TypeScript behavioral exhaustiveness.
