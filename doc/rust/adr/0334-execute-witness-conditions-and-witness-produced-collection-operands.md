---
id: RUST-ADR-0334
alias: ADR-0334
source_sha256: 73f2043f9befd401e9eb0096df56760d18de5abd4cec9978ed45d4ca51bada7c
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0334 — Execute witness conditions and witness-produced collection operands

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** approved plan, awaiting current audit remediation delivery. 2026-10-07. R03007/#351 finite effect obligations; issue registration precedes implementation. No backend/runtime behavior change is proposed. Current audit batch must finish before repository edits. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0334 — Execute witness conditions and witness-produced collection operands

Status: approved plan, awaiting current audit remediation delivery. 2026-10-07. R03007/#351 finite effect obligations; issue registration precedes implementation. No backend/runtime behavior change is proposed. Current audit batch must finish before repository edits.

### Problem

Two reviewed effect obligations currently rely on emitter AST inspection:

1. `stateful/expression/tests.rs::equal_branch_condition_keeps_one_witness_call_and_its_type_error`: an effectful condition must still execute once when both branches return the same value.
2. `stateful/expression/tests/collection_queries.rs::witness_result_is_bound_once_before_the_matching_query`: SetMember, MapMember and MapLookup must evaluate the witness key once before issuing the query.

Current four-fixture execution joins demonstrate related witness/branch/order behavior but not these exact shapes. Passing syntax checks cannot establish observable witness effects. This decision adds the missing generated-consumer execution dimension, not a speculative production fix or a trust-boundary investigation.

### Decision and source probe

Add one cohesive maintained `witness_effect_order` source and generated consumer package. Reusing witness-conditional would change its witness trait and require unrelated Set/Map setup; other ledger-witness fixtures ask witnesses to read state rather than feed witness results into generated queries. A small separate fixture keeps these four operator shapes and expectations easy to inspect.

Already accepted by the frozen current Rust and TS compiler in scratch:

```compact
import CompactStandardLibrary;
export ledger seen: Set<Field>;
export ledger table: Map<Field, Field>;
witness gate(): Boolean;
witness key(): Field;
constructor() {
  seen.insert(7);
  table.insert(7, 42);
}
export circuit equal_result(value: Field): Field {
  return disclose(gate()) ? value : value;
}
export circuit set_member(): Boolean {
  return seen.member(disclose(key()));
}
export circuit map_member(): Boolean {
  return table.member(disclose(key()));
}
export circuit map_lookup(): Field {
  return table.lookup(disclose(key()));
}
```

### Before / after evidence

```rust
// Before: retained unit assertions inspect emitted witness/query syntax and bindings.
assert_eq!(calls(&mut lowered), ["witness", "query"]);
// After: also execute the unedited generated consumer using a counting witness.
let out = equal_result(context, &witness, Field::from(42_u64))?;
assert_eq!(out.result, Field::from(42_u64));
assert_eq!(witness.gate_calls.get(), 1);
assert_eq!(out.context.private_state, 8);
assert_eq!(out.private_transcript_outputs.len(), 1);
```

The real generated equal-arm code currently calls `gate`, updates private state, pushes its transcript, discards the Boolean value and returns the parameter. That is permitted simplification only if the observable call remains. No production emitter change is expected.

### Finite case matrix and assertions

Initialize private state7; witnesses increment it once on success and never read ledger state. Constructor sets membership key7 and map7→42. Each scenario starts from a fresh constructed context.

| Export | Cases | Expected execution |
| --- | --- | --- |
| equal_result | gate false /true, value42 | result42; one gate call, no key call; private8; exact one Boolean private output; public state/effects unchanged; zero native query cost. |
| equal_result | gate refusal | exact intentional witness error; one gate call, no key call; no success result. Do not claim consumed-context or external-effect rollback. |
| set_member | key7 /9 | true /false; one key call; private8 and exact disclosed-key output; state/effects unchanged; native total cost matches the single TS query cost. |
| map_member | key7 /9 | true /false with the same witness/unchanged-state/cost obligations. |
| map_lookup | key7 /9 | value42 for7; intentional missing-key ledger error for9; one key call in both. Private/output equality assertions apply to successful returned context only. |
| each query export | key witness refusal | exact intentional witness error, one callback. With zero gas budget also set, witness error must still win over ledger gas rejection; this distinguishes evaluation order. |
| each query export | successful key7 +zero gas | callback count1, then ledger gas rejection. Distinguish this from the witness-error case. |

Keep the existing AST binding assertions as separate syntax evidence. The Rust native API does not expose an exact query trace/program for these exports. Do **not** claim a directly counted Rust query or exact Rust program from TS instrumentation. Successful native total gas can be compared against the captured one-query gas; error precedence and callback counts provide runtime ordering evidence. Query operation sequences belong to the independently captured TS dimension unless a genuinely exposed Rust observation is available.

### Current capability limits (probed, not hypothetical)

All four Rust exports currently have `recorded=false`, `observed_call=false`, and `unsupported_return` at `StateReturn::Expression`.

- equal_result: `proof_required=false`, `recording_status=not_applicable`.
- set_member/map_member/map_lookup: `proof_required=true`, `recording_status=unavailable`.

Retain these classifications explicitly. This slice qualifies native generated execution only. Do not expand recording admission, synthesize Rust recorded/program evidence, run proof generation, or call these exports proof-ready. Constructor execution and prestate assertions are not constructor proof.

### TS oracle feasibility: completed scratch probe

`/tmp/rust030-adr334-probes/` contains source, generated Rust/TS, capabilities, capture script and12actual TS cases. Source compiled with reviewed compactc/Scheme, `--target rust --skip-zk` and `--target ts --skip-zk`; native Node24.14.0 used the existing branch runtime (generated contract checks runtime0.16.101).

Observed scratch cases:

- equal false/true →42, one witness, zero queries; refusal →same intentional witness error, zero queries.
- set/map member7/9 →true/false, one witness then one query; witness refusal →zero queries.
- lookup7 →42; lookup9 →expected-cell/null error; both have one witness then one query. Witness refusal →zero queries.

Capture uses the established `QueryContext.prototype.query` observer pattern to retain TS operation arrays,query gas/errors and interleaved witness/query events. Reset observation after initialization and snapshot event arrays per row so later constructor calls cannot mutate already stored rows. During scratch development a shared event-array reference initially contaminated prior rows; the final script copies each row's array, and the retained12case output was rerun after correction. This is a capture-harness correction, not a contract finding.

Implementation should turn the probe into a maintainable capture script with ordinary generated-contract runtime resolution, typed-byte/BigInt normalization and source/runtime/compiler hashes. Compare successful serialized public state/effects/private outputs/gas as applicable. Preserve source-semantic injected failure labels and any TS/native error-format differences rather than demanding identical text across runtimes.

### Planned owners / files

- `examples/rust_backend/witness_effect_order.compact` — new tiny source.
- `tests-rust-backend/witness-effect-order/Cargo.toml`, `lib.rs`, `tests/witness_effect_order.rs` — new registered consumer, unedited generated library, meaningful scenarios.
- `runtime-rs/tests/fixtures/capture-witness-effect-order.mjs` and `witness-effect-order.json` — independent TS observation producer/capture.
- Root `Cargo.toml` member and expected local `Cargo.lock` package entry; no dependency versions change.
- No fixture_inventory.py edit expected: its standard underscore→hyphen source mapping discovers the new pair. Maintained render cohort becomes198, with an explicitly recorded census amendment.
- Existing value/collection AST unit tests remain as retained syntax evidence. No production emitter/runtime/API/ABI/IR change planned.

If a source/generation/consumer failure is discovered, stop to isolate its exact primitive and obtain the corresponding focused implementation decision; do not silently broaden this regression slice.

### Alternatives and gates

Alternatives: extend existing witness fixture (less setup, but changes public witness trait and mixes query ownership); hand-authored private IR only (misses source-frontend reachability); use recorded mode/proofs (unsupported today and unnecessary for the native effect claim). Choose the source-generated fixture.

Gates: source-to-Rust and source-to-TS compilation; deterministic capture sanity/event checks; focused package tests on normal1.99 locked/offline target, strict Clippy and formatting; compare generated bytes and append source/export/assertion/receipt rows. Rust1.88 check/run only if new generated syntax/API requires MSRV qualification; this source uses already-supported ordinary carriers and witness/query APIs. Run the complete198freshness checker once at the joined gate. No optional broad runtime suite, cryptographic proof work or remote CI.

Root independent review and conventional signed/DCO delivery follow the scoped passing receipt. No stopped investigation is included or accepted by this plan.


Delivery issue: [#458](https://github.com/MediaNoxLabs/compact/issues/458), milestone `rust-backend-v0.3.0`. Implementation waits for the current audit remediation batch.


### Delivery — 2026-10-07

## Native effects and exact negative controls — 2026-10-07

### Delivered locally

- ADR0334 / [#458](https://github.com/MediaNoxLabs/compact/issues/458): signed DCO commit `827ac965219e8fc203ab0ed8cfb8416094d06644`. Four generated native integration tests execute18 separately captured TS scenarios: equal conditional arms retain callbacks; Set/Map membership and Map lookup use witnessed keys; missing cells, refusal and zero-gas precedence have exact errors. Results, serialized state, private outputs, effects and gas are asserted. Strict scoped Clippy, formatting, JS validation and one-source freshness pass. Maintained roots now198; full joined freshness awaits the separately tracked compiler fix.
- ADR0335 / [#459](https://github.com/MediaNoxLabs/compact/issues/459): signed DCO commit `6dd4e965973042ce67ee87458d39ab35f0181fba`. The chunked-cell negative control independently requires `AssertionFailed("active mismatch")` from native and recorded calls, preventing two successes from passing a Boolean equality. One comprehensive integration test, strict scoped Clippy and formatting pass.

Root reviewed assertions, source hashes and manifest/lock changes. The only dependency change is one local fixture package. No production runtime/emitter changes in these two commits. Native query exports retain unavailable recorded APIs; query program/event evidence belongs to TS, while Rust asserts callback count, gas and error precedence. No new proofs or network acceptance are claimed.

### Evidence

[ADR0334-0335 — Native effect and negative-control execution.zip](references-0.3.0.md#note-113): 92 hashed entries; SHA256 `083f72be3af05c0b5b7172d305e89fa0121c2d5d414bd0089b28d687a56c579c`. Contains TS capture outputs and failed calibration attempts, generated source, execution joins, exact receipts/logs and signed commit patches. Original paths `/tmp/rust030-adr334-delivery` and `/tmp/rust030-adr335` remain available.

