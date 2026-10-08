---
id: RUST-ADR-0276
alias: ADR-0276
source_sha256: 8b16bcd5736a91f26ab225b196d0d8c4f29715a2e1d8194b0400baf94b5410a4
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0276 — Prototype generated named argument facades

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted for an isolated generated-code experiment before production promotion. Parents R030-02/#346, R030-03/#347 and R030-08/#352. The earlier handwritten adapters are evidence for choosing this experiment, not evidence that the generated API already exists. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0276 — Prototype generated named argument facades

Status: accepted for an isolated generated-code experiment before production promotion. Parents R030-02/#346, R030-03/#347 and R030-08/#352. The earlier handwritten adapters are evidence for choosing this experiment, not evidence that the generated API already exists.

### Decision and promotion boundary

Implement the proposed ordinary named Args records and forwarding methods in an immutable scratch compiler source copy, producing unedited generated crates for the actual witnessed Cell, original DID and pure passport. Keep production compiler sources and committed fixtures unchanged during this first experiment. Use an explicit source manifest/patch so the prototype is reviewable and reproducible; no public abstraction-level switch, annotation or version bump.

Choose structural eligibility (noninternal exports with at least two parameters), per-circuit modules, existing parameter spellings and fully qualified types. Do not infer semantic role types from names or duplicate FAB/proof packaging. The accepted design and matrix below define the required collision, witness, feature, lifetime, MSRV, exact behavior and successful prepared-DID equivalence checks. Retain old positional methods and zero/one-parameter controls. Same-type fields remain semantically interchangeable: show that limitation.

After those checks, root will review generated before/after examples, total API/source/expansion/artifact cost and diagnostics against the application-owned alternative. Production adoption requires an explicit recorded disposition and full generated-cohort change attribution; a passing scratch experiment alone does not close this issue or its parent. If the usability benefit does not justify the added public surface, record that outcome and keep the ordinary application wrapper recipe. No new runtime/macros/ABI/IR dependency is expected.

### Design basis

## Named arguments: deterministic additive facade proposal

Read-only design, 2026-10-07. No emitter edits, generated refresh, runtime/API change or new switch. `evidence.json` identifies inspected working source and the earlier successful handwritten consumer experiment. All proposed generated code below is illustrative, not a compiled implementation. One new tiny standalone Rust program validates namespace coexistence only.

### Recommendation

Use ordinary owned `Args` structs in per-circuit modules, not CamelCase name inference or new methods on existing handles:

- Stateful: `crate::ledger_contract::arguments::<emitted_circuit_ident>::Args`.
- Pure: `crate::pure_circuits::arguments::<emitted_circuit_ident>::Args`.
- Emit only for noninternal exports with at least two public circuit parameters. Constructors, witnesses, internal helpers, zero- and one-parameter exports retain only their existing API in this first slice.
- Fields use exactly `public_parameter_idents` in declaration order and exactly the corresponding owned `rust_type`. No source camelCase→snake_case renaming. No derives, Default, codec implementation, builders, context, witnesses, private state or observation in the record.
- Methods live on `Args`, consume it and delegate to existing positional facades: `native`, `recorded`, `call`; pure records have `evaluate`. Only expose methods for capabilities already emitted. In particular, do not synthesize observed calls when existing `_call` is suppressed by a source-name collision.

The rule applies structurally across eligible exports, not only the three example names. First experimental acceptance fixtures are witnessed Cell, original DID and pure passport; inventory all other changed facade outputs before adopting. No representation or planner rewrite is needed.

### Why these names and locations

Current identifiers replace `$` with `_`, then escape Rust keywords (`naming.rs:24–44`). The function namespace validator rejects normalized circuit-name duplicates (`:46–66`). Reuse those exact circuit identifiers as module names; avoid CamelCase conversion, its extra collisions, hashes, order-dependent suffixes and a second source-to-Rust naming map.

Current public parameter identifiers preserve Compact spellings, with deterministic positional fallback for wrapper-reserved/invalid/duplicate names (`lib.rs:274–317`). Use exactly that list for fields, including `controllerSignature`, `expectedVersion`, keyword raw identifiers and `__compact_param_N` fallbacks. Attach field doc comments recording the original Compact parameter when a fallback differs. Do not independently reserve `native`, `recorded`, `call`, `evaluate` or `Args` as field names: fields and methods coexist.

`arguments` is nested in generated execution modules, not crate root or `types`. Root aliases and Compact declared structs therefore retain their namespaces. Execution modules currently introduce only fixed support types/imports plus circuit functions (`lib.rs:3810–3864`); none occupies their type namespace as `arguments`. A source circuit named `arguments` occupies the value namespace and can coexist with the module. A circuit named `Args`, `runtime`, `call` or `arguments` just produces a nested module of that name. No blanket new source reservation is necessary. The tiny executed `namespace.rs` confirms a function/module pair named `arguments`, nested `arguments::arguments`, raw `r#type`, and a field/method pair `call` compile and run with warnings denied. It does not validate the full emitter.

Inside every new module use fully qualified support paths, including `::core::result::Result`, primitive paths and `::midnight_compact_runtime`; retain `crate::types::<SourceType>` for source types. Reuse/extract the recursive support-path conversion from `type_declarations.rs:29–53`, rather than introduce imports named `runtime`, `Result`, `Option`, `Vec` or `From` that source circuit modules could shadow. Nested vectors/tuples must receive the same treatment. Do not widen alias validation or rewrite old declarations in this slice.

Every collision comparison must use semantic identifiers (strip an optional raw prefix after normalization). Existing parameter-helper tests do not exercise a raw non-keyword IR spelling followed by its unescaped spelling. Add that adversarial IR control before adoption; if the old facade is already invalid for it, report/fix that existing helper defect explicitly rather than quietly claim the new module guarantees arbitrary malformed IR compatibility. Preserve currently valid source parameter spellings and old diagnostics; do not change the naming policy incidentally.

### Exact existing BEFORE and proposed AFTER

Current witnessed Cell public method (`witness-cell-write/lib.rs:571`) accepts context, seed, offset positionally. DID's current observed facade (`did-adoption/lib.rs:4165`) accepts observation, private state, value, mutation, controllerSignature, expectedVersion, and constructs FAB in that declaration order. Passport (`vc-passport-adoption/lib.rs:3113`) accepts seven positional values.

```rust
// Existing APIs remain byte-for-byte intact in their old AST items.
contract.write_offset(context, seed, offset)?;
contract.recording().setAlsoKnownAs_call(
    observed, private_state, value, mutation, controllerSignature, expectedVersion,
)?;
pure_circuits::assertValidDigitalPassportAgePredicate(
    credential, presentation, currentDay, dateOfBirthDays,
    dateOfBirthOpening, currentDate, dateOfBirthDate,
)?;
```

```rust
// PROPOSED generated facade usage; these paths are not emitted today.
use contract::ledger_contract::arguments::write_offset::Args as WriteOffsetArgs;
WriteOffsetArgs { seed, offset }.native(&contract, context)?;

use did::ledger_contract::arguments::setAlsoKnownAs::Args as AliasArgs;
AliasArgs { value, mutation, controllerSignature, expectedVersion }
    .call(&contract.recording(), observed, private_state)?;

use passport::pure_circuits::arguments::assertValidDigitalPassportAgePredicate::Args;
Args { credential, presentation, currentDay, dateOfBirthDays,
       dateOfBirthOpening, currentDate, dateOfBirthDate }.evaluate()?;
```

Representative proposed DID method (qualified types shortened only in this illustration):

```rust
pub struct Args {
    pub value: OpaqueString,
    pub mutation: crate::types::SetMutation,
    pub controllerSignature: crate::types::SchnorrSignature,
    pub expectedVersion: BoundedUint<18446744073709551615>,
}
impl Args {
    #[cfg(feature = "ledger-transaction")]
    pub fn call<'observed, Private, W>(
        self,
        handle: &crate::ledger_contract::recorded::BorrowedContract<'_, W>,
        observed: &'observed ObservedContractState,
        private_state: Private,
    ) -> Result<RecordedCall<'observed, Private, ()>, CompactError>
    where W: crate::ledger_contract::TryWitnesses<Private> {
        let Self {
            value: __compact_arg_0,
            mutation: __compact_arg_1,
            controllerSignature: __compact_arg_2,
            expectedVersion: __compact_arg_3,
        } = self;
        handle.setAlsoKnownAs_call(observed, private_state,
            __compact_arg_0, __compact_arg_1, __compact_arg_2, __compact_arg_3)
    }
}
```

Destructure into emitter-owned positional locals, so a field named `handle`, `args`, `native` or `__compact_arg_0` cannot shadow the forwarding context. Do not clone or repack FAB in this wrapper. Existing `render_observed_call_method` owns retention/encoding/entry-point binding (`recorded/facade.rs:109–200`). Source order is preserved when forwarding already materialized values. Rust evaluates record field expressions in their written order; labels do not promise declaration-order evaluation of caller-side effects. Examples should materialize inputs first.

### Handle, lifetime and feature rules

- Native `Args::native<Private, W>(self, contract: &ledger_contract::Contract<W>, context: CircuitContext<Private>)` uses the existing native method. Add `W: TryWitnesses<Private>` only when that existing method requires it. A nonwitness call in a witnessed contract must work with arbitrary `W` as it does today.
- Recorded/call use the existing recording handle, not private witness fields. For a contract with any witnessed recording, take `&recorded::BorrowedContract<'_, W>` and the per-circuit witness bound only if needed. Otherwise take `&recorded::Contract` without `W`. Select from the same already-computed `witnessed_recorded` fact that emits the handle (`lib.rs:3599–3633`). Do not rediscover witness usage or require the witness trait merely because another export uses it.
- `Args` itself has no generics or lifetimes: source monomorphized/owned parameter types suffice for this scope. `Private` and `W` occur only on forwarding methods. No new Clone, Default, Send, Sync, static lifetime or trait-object requirement.
- The result lifetime of `call` is exactly the observation's lifetime, independent of `self` or the borrowed witness-handle lifetime. Forwarding an existing `_call` enforces this. A call can outlive the temporary handle after execution; it cannot outlive its observation. Do not tie these lifetimes together accidentally.
- Records, native methods and recorded methods do not require `ledger-transaction`. Only `call` has that cfg, and only when the existing capability `observed_call` is true. Passport's pure-only crate currently has no such feature at all; emit no transaction cfg or type there.
- Preserve the old policy suppressing `_call` when a source export already owns its name (`lib.rs:3357–3379`). A new record must not bypass that policy. Internal pure/stateful helpers get no new public record even if old internal helpers happen to reside in a public module.

### Acceptance matrix before production promotion

| Area | Required positive/control | Required refusal or invariant |
|---|---|---|
| Namespace | Source circuit `arguments`, `Args`, `runtime`, `call`; root aliases/types named arguments/Args/Result/Option; `$` and keyword circuit names | Existing normalized function collisions retain exact diagnostic; no new global name reservation |
| Fields | Existing camelCase and keywords; context/input/observed fallbacks; numbered-local spellings; duplicate normalized IR spellings | Missing field E0063, wrong field type E0308; no default construction; raw semantic duplicates assessed explicitly |
| Arity | Two-argument Cell, four-argument DID alias, seven-argument passport | Zero/one-parameter exports and constructors have unchanged output; no empty/singleton facade churn |
| Types | Owned string, enum, signature struct, bounded integer, vector/nested tuple and empty tuple as a parameter type | Exactly old bounds/types; no representation derives, phantom witness/private data or accidental type generic |
| Witnesses | Non-Clone/non-Default witness implementor; two private-state types; mixed witnessed/nonwitnessed exports | Wrong witness/private pairing fails with existing bound; no gratuitous W bound on nonwitness calls |
| Features | No-default-feature Cell/DID native+recorded; transaction-enabled observed call; pure-only passport | Transaction method unavailable when disabled; unsupported recording export has native only; suppressed observed call stays absent |
| Lifetime | Return call after Args/temporary handle is dropped, while observation lives | Escaping observation E0515; no fabricated static observation; named facade return tied only to same observation |
| Behavior | Cell state/effects/private outputs/witness order/query gas/error order; all 25 passport rows; actual DID alias call from valid existing fixture | Same-type fields intentionally swapped still compile: document semantic-role limitation; reject false security claim |
| Binding | Successful named versus positional DID call/preparation using two equivalent observed fixtures | Compare declared input FAB, entry point, sealed program/execution, prepared bytes where available; wrong observation preparation refuses; no new proof claim from compile-only control |
| Corpus | Existing item AST/bytes, runtime/compiler metadata and all capability reports unchanged except added module items | Enumerate every changed generated crate; no body/planner/ABI assertion changes; compare removing only new module from AST |

Use existing valid DID prestate helpers and alias oracle, not constructor authorization investigation. A new cryptographic proof is unnecessary for a forwarding-only experiment if successful prepared bytes are equivalent and existing strict gate remains intact; do not label that as a new proof result. Previously executed handwritten DID probe only typechecked and checked lifetime: actual successful FAB/observation equivalence is still required for the new DID facade.

### Implementation ownership and versioning decision

Proposed new backend `argument_facade.rs` owns struct/module/forwarding syntax. `lib.rs` supplies public declarations plus already-computed recording/observed/handle facts and appends generated modules; do not add another effect analyzer. `recorded/facade.rs` remains the sole owner of observed input packaging. A tiny shared support-type qualification helper may be extracted from `type_declarations.rs` unchanged. Existing `stateful.rs`, planners, runtime and macros need no semantic edits. A single immutable fixture regeneration follows implementation freeze.

ABI50, IR20, capability schema2 and compatibility metadata can remain unchanged **if** generated code only invokes existing APIs, as proposed. The source compatibility matrix describes runtime/macros/ledger requirements, not every additive generated convenience (`compatibility.rs:33–58`). Requiring a new runtime helper or adding fields to that deny-unknown-fields record would require separate review; neither is needed here. Generated source/artifact hashes change, so receipt bindings must be refreshed honestly. Compiler build/version provenance identifies availability of the new generated surface; no source name annotation or public switch. Preserve Rust1.88 compile and Rust1.99 execution gates; do not infer MSRV success from the existing scratch run alone.

### Alternative and decision criterion

An application-owned `WriteOffsetArgs` wrapper works today, chooses familiar local names, stays stable even if the generator evolves and can add meaningful role types/validation owned by that application. Costs are duplicate forwarding/maintenance for large parameter lists. Generated Args improve discovery and consistent completeness checks across many exports; they do not prevent binding the wrong Field to a correctly named field or validate relationships between fields.

The prior controlled handwritten experiment added 14 display lines / 483 source bytes and 2,704 debug rlib bytes; three warm paired rebuild ranges overlapped (0.221–0.241 s positional; 0.222–0.230 s named). Those are historical scratch results, not predicted production costs. For adoption measure current generated display/expanded LOC, total API count, artifact size and paired forced consumer rebuilds under the same locks/toolchain; record diagnostics and namespace/capability preservation. Proceed only if discoverability and reduced consumer duplication justify the additional modules; do not optimize for macro count or claim speedup.

Issue: https://github.com/MediaNoxLabs/compact/issues/400


### Research disposition — 2026-10-07

Research delivered, promotion deferred pending a scoped API decision. The scratch compiler emitted actual Args modules; unedited generated Cell, original DID and passport consumers passed their bounded behavior gates, including four successful DID prepared-call equivalence cases and Rust1.88 checks. Existing positional output/capabilities remained unchanged outside the added modules.

Root retains Args as a viable additive candidate, with **no default generation promotion now**. Twenty-four DID and thirty-nine passport records and roughly twenty percent displayed source growth require targeted selection/usability justification. Application-owned wrappers remain a useful baseline. The genuine raw-name collision is tracked separately by ADR0281; issue #400 stays open until its green refresh is recorded. No claim of performance improvement, new proof acceptance or full-cohort production adoption is made.

Full results: [ADR0276 — Generated named argument facade experiment — 2026-10-07](references-0.3.0.md#note-047). Durable inputs/receipt/patch/logs: [Initiatives/02 Compact Rust emission/Artifacts/rust030-adr0276-generated-arguments-fca7577b/README](references-0.3.0.md#note-154). Preserve the original before baseline and failed raw cases when appending the future guard refresh.



### 2026-10-07 — ADR0281 raw-name refresh verified

Research delivered; default Args promotion remains deferred pending a scoped API decision.

The frozen fca7577b + Args compiler received only the corresponding shared naming/public wrapper/pure binding fix from signed ADR0281 commit `f0775cc61f54dd836c362e1dc8712cba784bc676`. It did not receive the newer root component extractions or other moving source. Before/after files and the applied patch are retained here; executables remain local and hash-bound only.

All seven generated consumer cases compiled and executed under Rust 1.99.0: original pure/stateful failures, both raw-name orders, keyword aliases, dollar normalization and fallback collisions. Twenty-six pure positional/named assertions establish distinct binding correctness. Four witnessed Cell calls establish equivalent native/recorded positional/named results: witness input 3, private state 8, Cell value 11, and equal state, effects, private outputs and gas. These are direct renderer IR cases, not claims that the Compact frontend accepts raw identifiers.

Cell, DID, passport and Counter libraries rendered from their unchanged original IR are byte-identical to the earlier Args prototype. The original red artifacts, receipt, MSRV controls, prepared DID comparisons and paired measurements are preserved. No unrelated proof or benchmark was rerun. ABI 50, IR 20 and capability schema 3 remain unchanged.

Generated Args remain a viable additive candidate. Automatic records for 24 DID and 39 passport exports add roughly 20% displayed source; targeted selection and usability justification remain open design decisions. Application-owned wrappers remain the simpler available alternative. Closing the research issue does not promote generated Args or accept the parent milestone.

Signed fix: [f0775cc61f54dd836c362e1dc8712cba784bc676](https://github.com/MediaNoxLabs/compact/commit/f0775cc61f54dd836c362e1dc8712cba784bc676).

Amendment: `Artifacts/rust030-adr0276-generated-arguments-fca7577b/adr281-refresh/receipt.json`, SHA256 `9c61646cadd373ab2eed5e0508b72dd4fb6e2d386f1460b5cfb3f644876f9478`. Full immutable inputs, source patch and meaningful logs accompany it. Original receipt SHA256 remains `24017360c50838542c60c85bcc7a7597cafc731c716eb48063774f268ec42da2`. Issue #400 is now eligible for research closure under the recorded promotion-deferred disposition; actual closure is verified separately.



#### Research closure verified

[Issue #400](https://github.com/MediaNoxLabs/compact/issues/400) was verified CLOSED / COMPLETED at 2026-10-06T23:48:27Z. [The disposition comment](https://github.com/MediaNoxLabs/compact/issues/400#issuecomment-6027618643) records successful scratch research and deferred default Args promotion. This does not close or accept the parent milestone.
