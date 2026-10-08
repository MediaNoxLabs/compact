---
id: RUST-ADR-0269
alias: ADR-0269
source_sha256: 2ebf70e9e3815d95ca7ed5b4bcc1374907034b0fa1d8d286e0036e48db007fb5
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0269 — Record opaque-string product Maps through shared typed composition

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** locally delivered at signed commit `a536360ab002149afc7f6b33465c6d3071eb1d12`, 2026-10-07. Historical proposal and sequencing below are preserved. Parents R030-09/#353 and R030-11/#355. Baseline is signed repository HEAD `9949b01340f5801c9ef2f65e805d4427930c2778` plus the frozen post-ADR0265 compiler `/tmp/compact-adr265/bin/compactc-delivery` and pinned Scheme `/tmp/compact-adr251/source-gate-final2/bin/compactc-scheme`. The compiler emits IR schema 20 / runtime ABI 50. Original source is the unchanged midnight-did v0.7.0 `did.compact` at `examples/rust_backend/did_adoption/packages/contract/src/did.compact`, SHA256 `632f34af543924edb185fe9c8eda54aa4ca0503b0b9e8ee3ff06ec470f6d7456`. Original upstream package depends on ledger 8.1; this compiler/runtime profile is pinned to ledger 8.0.3, and no cross-version interoperability is asserted. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0269 — Record opaque-string product Maps through shared typed composition

Status: locally delivered at signed commit `a536360ab002149afc7f6b33465c6d3071eb1d12`, 2026-10-07. Historical proposal and sequencing below are preserved. Parents R030-09/#353 and R030-11/#355. Baseline is signed repository HEAD `9949b01340f5801c9ef2f65e805d4427930c2778` plus the frozen post-ADR0265 compiler `/tmp/compact-adr265/bin/compactc-delivery` and pinned Scheme `/tmp/compact-adr251/source-gate-final2/bin/compactc-scheme`. The compiler emits IR schema 20 / runtime ABI 50. Original source is the unchanged midnight-did v0.7.0 `did.compact` at `examples/rust_backend/did_adoption/packages/contract/src/did.compact`, SHA256 `632f34af543924edb185fe9c8eda54aa4ca0503b0b9e8ee3ff06ec470f6d7456`. Original upstream package depends on ledger 8.1; this compiler/runtime profile is pinned to ledger 8.0.3, and no cross-version interoperability is asserted.

### Recommendation and scope

Record **both `setService` and `removeService`**, one closed Service Map vertical. `removeService` cannot succeed on the original constructor state without a preceding insertion, so splitting its delivery into a standalone proof would require fabricated storage. Successful set Insert → set Update → remove is the smallest constructor-derived CRUD sequence. This would move original DID stateful recording availability from 4/12 to **6/12**, if implemented and verified; native acceptance remains 12/12. Other verification-method Maps, relations, lookup helpers and generic Map APIs remain out of scope.

Before: the unchanged original compiles, generates native Rust, and the direct native lifecycle test covers Service cases, but neither Service export has a recorded/observed-call API. At current post-265 baseline, `setService` refuses `StateAction::Let` at `actions[0]`; `removeService` refuses `StateAction::CircuitCall` at `actions[0].action.actions[0]`. These coarse first diagnostics mask the actual missing Map leaves. The minimal reducers below isolate them: direct `MapInsert` and `MapRemove` each refuse as unsupported action, and direct `MapMember` refuses as unsupported return. `service_mutation` combines original branch shape/pure Unit guard and still refuses root Let. All reducers emit native Rust that `cargo check` accepts. `service_digest.compact` passes an entire three-string Service struct through a pure transient-hash helper into a stateful Unit helper and is already recorded, isolating Map operations rather than nested Service hashing as the missing family. Two- and four-field products prove native compilation does not depend on Service's field count; an empty product compiles but is explicitly outside this proposed recording family. The original `setAlsoKnownAs` remains recorded at the same baseline.

After (target): generated recorded methods use the same typed `MapSlot<OpaqueString, Service>::record_member`, `record_insert`, `record_remove` as the runtime's existing upstream-backed Map VM. They retain exact source order and lexical values. For `setService`, evaluate disclosed Service and mutation once, form/authenticate the existing digest, run `assertMapMutationDefined`, perform Update member→remove or Insert !member, then insert the same typed Service, then run `recordUpdate`. For `removeService`, authenticate first, check member, remove, then update counters/timestamp. Native Rust bytes and all unrelated recording capabilities stay unchanged.

### Source and transitive graph

- `did.compact:42`: `services: Map<Opaque<"string">, Service>`, physical path `[1,14]`, IR index 1. `Service` at line 102 is exactly three opaque-string fields `id`, `typ`, `serviceEndpoint`. The source name is diagnostic context, not an emitter dispatch key.
- `did.compact:791–814`: `setService(Service, MapMutation, Signature, Uint64) -> Unit`. `MapMutation` is Undefined/Insert/Update. The Update branch checks membership and removes before the common insertion; Insert checks nonmembership; an Undefined mutation fails in the pure guard before either Map query. Authorization precedes the mutation guard.
- `did.compact:816–831`: `removeService(OpaqueString, Signature, Uint64) -> Unit`; authorization precedes membership assertion and removal.
- The transitive call graph in `/tmp/rust030-adr265/inventory.json` includes the pure authorization digest and `controllerOperationHash`, the pure Unit `assertMapMutationDefined` for set, the stateful `assertControllerCanUpdate → assertController → imported Schnorr verifier`, and `recordUpdate` for both. Existing ADR0259/0265 already audit/record these local and pure helpers for Point and alias mutations; this slice must retain complete nested body audits, exact parameter/result types, cycle guards, lazy branches, and public/private effect ordering. Do not replace controller authorization with a witness shortcut.
- Existing runtime `runtime-rs/src/slots.rs:818,904,924` provides typed Map member/insert/remove. The generated native path already proves `Service` is a valid `CellValue` for its Map slot. No new runtime primitive, schema or generated-facing ABI is expected.

### Smallest emitter domain

Use `recorded/typed_plan/composition.rs`'s existing declaration-driven `Audit` and shared `Plan`; add no parallel evaluator or contract-name/source-count gate. Admission is a closed `Map<OpaqueString, V>` where `V` is a **nonempty flat named struct whose every field is OpaqueString**, with arbitrary legal field and struct names/count. This is the structural string-product family used by Service, not a rule about its three fields. The existing IR declaration validator owns field-name uniqueness and type identity; the shared Plan verifies exact key/value types and scope. Every action/branch/callee is audited structurally. Reuse `composition::value_type` for parameters and pure digest; preserve `observed_value_type` for Cell/witness reads. Keep the previously accepted String Set predicates separate. Nested structs, mixed Enum/Point/number products, arbitrary Maps, lookup, size and reset remain outside this profile. Ledger slot lookup must verify declaration, index and full physical path through existing validation. A zero-field struct is legal in current Compact and its native Map crate checks, but is deliberately **not** part of this recording family: it carries no string value and has no original Service use or independent recorded-program/proof evidence. Its recording can be considered separately rather than admitted incidentally.

1. `Audit::value` accepts `Expr::MapMember` in a stateful context only for that Map shape, recursively audits the key as `OpaqueString`, and counts the public query. Pure helper bodies must still reject it, including unused Let values and unselected If arms.
2. `Audit::action` accepts `StateAction::MapInsert` and `MapRemove` only for the same Map shape; it recursively audits key and value and counts the public mutation. It does not accept MapLookup, reset, size, default insert, unrelated Maps or broad Map return shapes.
3. The existing typed `Plan` gets matching gated `Expr::MapMember` and `StateAction::{MapInsert,MapRemove}` leaves only when a complete `composition_calls` audit exists. It evaluates each typed key/value once in source order, checks their exact declared types and lexical scopes, and emits the existing slot methods. The old direct `StateReturn::MapMember` reducer may remain unavailable because this is a Unit composition profile; it should not be counted as a delivery failure for the two original Unit exports.
4. Keep `composition::Audit`'s `public > 0`, declared recorded-Unit helper, root Unit result and pure/recorded helper closure checks. Use structural admission rather than a post-hoc query-count whitelist. Preserve native/type/action error precedence and earlier profile ordering; a new profile must not absorb unrelated String Maps or alter the 4/12 prior capability output.

Owner expectation: compiler `recorded/typed_plan/composition.rs` and narrowly gated leaves in `recorded/typed_plan.rs`; runtime only for a concrete demonstrated missing primitive (none found). No frontend, IR schema, ABI, TS runtime or original-source edit is proposed.

### Independent reducers and baseline

All files are immutable research fixtures under `/tmp/rust030-adr269/reducers`; outputs under `/tmp/rust030-adr269/compiled`. Command: `COMPACTC_SCHEME=/tmp/compact-adr251/source-gate-final2/bin/compactc-scheme /tmp/compact-adr265/bin/compactc-delivery --target rust --skip-zk --rust-runtime-root /Users/ysh/.codex/worktrees/07ae/compact <source> <output>`. Generated crates passed `cargo +1.99.0 check --offline` with `CARGO_TARGET_DIR=/Users/ysh/.codex/worktrees/unassessed-source-bridge/compact/target/adr169` and `CARGO_INCREMENTAL=0`.

| Reducer | Observed exact first refusal |
|---|---|
| `service_map_leaf.compact` `put` | `unsupported_action / StateAction::MapInsert / actions[0]` |
| same `drop` | `unsupported_action / StateAction::MapRemove / actions[0]` |
| same `has` | `unsupported_return / StateReturn::MapMember / return_value`; this standalone Boolean API is outside the proposed Unit profile |
| `service_map.compact` `insert_control` / `update_control` | `unsupported_action / StateAction::Let / actions[0]` |
| same `remove_control` / `member_control` | MapRemove / MapMember as above |
| `service_mutation.compact` `set_service` / `remove_service` | `unsupported_action / StateAction::Let / actions[0]` |
| `service_digest.compact` `run` | **recorded=true**; an entire three-string Service product through pure transient hash and stateful Unit call is already admitted |
| `flat2_map.compact` `put` / `drop` | Two-field `Pair{label,target}` Map; native generated crate checks; current recording first refuses MapInsert / Assert |
| `flat4_map.compact` `put` / `drop` | Four-field `Quad{label,target,nonce,note}` Map; native generated crate checks; same first refusals |
| `empty_map.compact` `put` | Empty product is legal and native generated crate checks; excluded by proposed nonempty recording predicate |
| prior ADR0265 `opaque_digest.compact` `run` | **recorded=true** on current compiler |

The direct leaf reducer intentionally exposes precise missing IR operations; the composed reducer exercises pure Unit guard, Enum branch, String product, Counter helper and Map effects. The two- and four-field controls show no Service arity or name dependency in native generation; both must become recorded under the proposed closed composition profile (their `put` and `drop` include a declared Counter helper). A standalone `has` need not turn recorded under this proposal; broadening it is a separate read-only Map return decision. The empty-product control is an explicit negative for recording even though native compilation succeeds.

### Required behavioral and proof gates

Reuse existing independent TS oracle and native lifecycle `tests-rust-backend/did-adoption/oracle/{capture-lifecycle.mjs,lifecycle.json,reviewed-matrix.json}`. Seven exact source cases already captured: missing Update (5 successful TS queries, reduction witness only), Insert (9, reduction→timestamp), duplicate Insert (5), Undefined mutation (4), Update (10, reduction→timestamp), missing Remove (5), Remove (9, reduction→timestamp). Retain source error strings, query-summed gas, full TS public program for successes, per-query prefix/gas for failures, native full committed-state/private rollback, and witness order. Existing pure API tests directly cover `setServiceAuthorizationDigest` and `removeServiceAuthorizationDigest`, including empty and Unicode fields; this recording slice must not change those vectors.

Add an independent original-source **service-only** constructor-derived TS sequence (no storage seeding): Insert Unicode/nonempty Service → Update same id with empty endpoint → Remove, with separate snapshots for missing/duplicate/Undefined and late timestamp failure. Rust generated native and recorded calls must agree with TS on exact typed result, state/effects, every ordered public VM operation, each query gas and summed gas, private transcript/witness order. Replay the complete recorded program from the same prestate and compare effects and resulting state; preserve partial TS query prefix and committed rollback on errors. A wrong-map-shape, path/index, changed struct value, wrong key type, hidden Map query/write in pure helper, hidden unused/unselected Map effect, recursive helper, argument count/order, and escaping local mutants must refuse recording or fail typed render as appropriate. Positive reducers must include both renamed two- and four-field flat products, in addition to original three-field Service; nested/mixed/empty products remain explicit refusal controls. Compare all capabilities after implementation and attribute any incidental newly recorded APIs instead of suppressing them by source name.

For strict proof/application, extend the existing original DID constructor-derived deployment harness (`--did-alias-lifecycle`) with three sequential **actual applied-state** calls using original `setService` Insert, Update, then `removeService`. Generate/select original circuit keys; use default strictness, separately funded Dust, nonempty proof verification, changed input/state binding rejection and same-time replay refusal. Check each applied Map content, version/operationCount, updated timestamp, retained private outputs and next context from the prior accepted ledger state. Retain the constructor limit: deployment accepts constructor-produced state but does not prove constructor execution, and stored zero id remains distinct from deployed address. Reject missing/duplicate/Undefined before mutation, and test timestamp failure rollback without fabricating successful ledger history. Do not claim all DID functions or ledger-8.1 compatibility from these three calls.

Full local acceptance: focused typed-plan/unit/renderer negatives, immutable before/after corpus diff and capability attribution (exact two newly recorded original exports; prior four unchanged), generated fixture freshness, independent TS/native/recorded/replay comparisons, strict original-source proof/apply/rollback, and Clippy. If original proof graph needs a different k/key provisioning or ledger policy, report it as a separate blocker rather than weakening strictness or source.

### Root review and sequencing

Accepted the refined structural family: opaque-string keys and a nonempty, flat named product of opaque-string fields. Field count and spelling are not dispatch conditions. The original three-field Service and independent two/four-field controls establish the proposed domain; an empty product is separately declined pending evidence, and nested/non-string products remain out of scope. This avoids encoding a particular contract's three-field layout as compiler policy.

ADR0265 is integrated at `23d5e8cada2adaf11039bb3d731010a4073167fb`. Preserve native output and all earlier supported exports. Additional capability changes must be explained and tested rather than suppressed using contract names.

Coordinate shared compiler, DID fixture and proof harness edits with ADR0268. The new proof-gate run requires frozen source identities; prepare independent reductions/captures in scratch while it runs, then acquire production ownership. Extend the scenario inventory after actual Service proof acceptance, so a later full run includes delivered Service scenarios. No remote CI or push.

Issue: https://github.com/MediaNoxLabs/compact/issues/393

### Local delivery — 2026-10-07

The accepted nonempty flat named product domain records original `setService` and `removeService`, bringing original DID stateful recorded availability to **6/12**. The new 14-case original-constructor TS capture and seven historical Service cases are checked against native/recorded/replay ordered state, program, query gas, effects and private witness behavior. The dedicated gate verified eight original-source proof calls across Point, Alias and Service scenarios; Service Insert → Update → Remove applied sequentially under default strictness, and changed binding plus same-time replay were refused. Constructor-produced state was deployed; constructor execution itself was not proved. Ledger 8.1 source versus Rust ledger 8.0.3 remains a separate compatibility gate.

The frozen post-ADR0265 versus final renderer comparison covered 183 sources. Only this original DID source changed, and only the two intended Service exports gained recorded and observed-call capability. All native prefixes and the other 182 Rust/capability outputs are byte-identical. The ADR0274 extraction fixes a default worker stack regression without changing emitted DID bytes.

Evidence: [ADR0269 — Service Map delivery receipt](references-0.3.0.md#note-039), [ADR0269 — Strict DID Service proof receipt](references-0.3.0.md#note-040), [ADR0269 — 183-source immutable renderer differential](references-0.3.0.md#note-038), [ADR0274 — Default worker stack regression and correction](references-0.3.0.md#note-045). The source commit is conventional, GPG signed, and DCO signed off. Remaining six original DID exports are still unrecorded; this delivery does not claim complete DID parity or a proven constructor.


### Issue closeout

[MediaNoxLabs/compact#393](https://github.com/MediaNoxLabs/compact/issues/393) closed as completed after the local delivery evidence was saved; delivery comment: https://github.com/MediaNoxLabs/compact/issues/393#issuecomment-6027055022. Parent DID adoption remains open.
