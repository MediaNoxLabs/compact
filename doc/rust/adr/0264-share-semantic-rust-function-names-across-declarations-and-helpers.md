---
id: RUST-ADR-0264
alias: ADR-0264
source_sha256: 3ba24b5750e7f1381ee86c2b0d8250a8c2274ab1a867499a75d7cfe5edf398fc
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0264 — Share semantic Rust function names across declarations and helpers

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** not separately declared. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0264 — Share semantic Rust function names across declarations and helpers

Read-only review on Compact `c43e8fd3a5931b7ffed5ee6c7f73943ab0621cdf` (current root worktree also has independent ADR0263 `recorded/pure_calls.rs` WIP, excluded here). No production edits, issue or ADR created.

### Problem and call-site evidence

`tools/compact-rust-backend/src/lib.rs:269–305` has the accepted ADR0246 declaration rule: `ident(name)` translates Compact `$` to `_` and raw-escapes Rust keywords, then `validate_circuit_function_namespace` strips `r#` for the semantic collision key. Thus a *single* stateful declaration named `r#type` is valid and emitted as `fn r#type(...)` (and `foo` plus `r#foo` in one function namespace is correctly rejected, with later-source location).

`tools/compact-rust-backend/src/recorded/helpers.rs:19–64` separately constructs a shared recorded helper by formatting raw source text: `ident(format!("__compact_recorded_body_{name}"))`. For `name = "r#type"`, this forms `__compact_recorded_body_r#type`, which cannot parse as one Rust identifier. Its fallback `r#__compact_recorded_body_r#type` cannot parse either. A root `StateAction::CircuitCall` to this otherwise valid internal Unit helper routes through `helpers::plan_recorded_helpers` (`helpers.rs:112+`, called by `lib.rs:3372–3379`), so the generated recorded surface can fail with `InvalidIdentifier("__compact_recorded_body_r#type")` after the native function was accepted. The helper collision loop also compares `syn::Ident` values instead of the explicit ADR0246 semantic key, giving two policies for the same Rust function namespace. This is a naming/domain inconsistency; it is not a claimed exploit or a verified production Compact source failure.

A nearby existing renderer test constructs exactly the required shared-helper shape: `tests/render.rs:5930–6040` has internal `inner` (typed Field parameter, witness, Cell write) and exported `outer` calling it, then checks one `__compact_recorded_body_inner` and the fallback on a declared name collision. `tests/declaration_validation.rs:280–320` covers raw-name collision precedence. The proposal adds variants to those tests rather than creating a broader framework.

### Bounded model and before/after

Introduce a private `RustFunctionName` (or similarly named small type) beside `ident` in `lib.rs`, with `source_ident: syn::Ident` and `semantic_key: String` computed once from normalized/escaped `ident(name)`. Use it in `validate_circuit_function_namespace` and `recorded/helpers.rs::helper_ident`. Helper names concatenate the *semantic key*, then run through existing deterministic collision/fallback policy. The normal public `ident` return can remain for non-function identifiers. No IR/schema/ABI change, no mangling of public contract functions, and no expansion of recording admission.

Before (valid private IR source declaration):
```text
internal circuit r#type(seed: Field): Unit { Cell.write(seed); }
circuit outer(seed: Field): Unit { r#type(seed); }
```
Native `fn r#type` passes, but recorded helper construction attempts `__compact_recorded_body_r#type` and fails.

After:
```rust
fn r#type(...) { /* native body unchanged */ }
fn __compact_recorded_body_type(...) { /* same audited recorded body */ }
// outer's recorded call targets __compact_recorded_body_type.
```
A declared user function named `__compact_recorded_body_type` still triggers the existing stable index+raw-byte fallback, checked with the same semantic collision key. Ordinary `inner` keeps exactly its existing helper spelling. Source names `foo` and `r#foo` still collide in one emitted function namespace and report the later declaration's source location before helper planning.

### Acceptance and preserved error order

1. Extend existing shared-helper render test with `inner.name = "r#type"` and the root call's `name = "r#type"`; assert `render_with_capabilities` succeeds, recorded root and helper are present, generated source parses with `syn`, exactly one semantic helper name exists, and a focused generated Cargo fixture type-checks if test harness already supports it. A Rust keyword source name `type` should follow the same semantic helper base.
2. Add a declared `__compact_recorded_body_type` stateful sibling and assert deterministic fallback, no helper/user collision, unchanged recorded root availability. Retain the established `inner` output spelling and fallback test byte-for-byte.
3. Preserve ADR0246 declaration errors: raw duplicate `foo`/`foo` wins before normalized collision; `foo`/`r#foo` and `a$b`/`r#a_b` still yield `ConflictingCircuitIdentifier` with same later location/text; an otherwise invalid source name still yields `InvalidIdentifier`, not recording Unsupported or a renderer panic.
4. Preserve non-name validation order: effectful ReturnPlan with duplicate actions remains `MalformedReturnPlan` before helper planning; invalid helper body remains its preexisting recording gap/error, not silently admitted. Reuse current render tests and capability output to compare before/after ordinary source fixtures.
5. Strict renderer test suite plus fixture freshness for only affected helper fixtures; no broad compiler/ledger proof gate required unless a generated artifact changes beyond the intended helper spelling.

### Ownership and scope

Likely files: `lib.rs` small private name type; `recorded/helpers.rs` helper naming/collision calls; `tests/declaration_validation.rs` and existing shared-helper test in `tests/render.rs`. Exclude ADR0263 pure-call extraction WIP, `stateful.rs` renderer semantics, typed Plan domains, ledger/runtime code and generated ABI. This fixes one duplicated invariant and prevents a valid declaration from being rejected solely by internal recorded-helper spelling; splitting large files without that behavior would not address the issue.

### Reproduced scope and accepted decision

Public render_with_capabilities succeeds for a raw-named native-only internal helper, but adding its recorded call returns InvalidIdentifier for __compact_recorded_body_r#type. No panic. Ordinary helper/control and deterministic collision fallback succeed. Actual Compact0.31.133 rejects both r#type and keyword type as circuit names; this is an imported/private typed-IR consumer inconsistency, not a reproduced Compact source bug. Reproducer receipt /tmp/rust030-adr264-repro/receipt.json retained below.

Accept small private RustFunctionName ownership (prefer dedicated naming module) and reuse semantic key in declaration and helper collision checks. Preserve normal spelling/fallback/error order. Add real renderer regression and ordinary external generated Cargo compile. No schema/ABI/semantic domain widening.

```json
{
  "head": "c43e8fd3a5931b7ffed5ee6c7f73943ab0621cdf",
  "scope": "public compact_rust_backend::render_with_capabilities typed-IR boundary; source parser control",
  "renderer_command": "CARGO_TARGET_DIR=/Users/ysh/.codex/worktrees/unassessed-source-bridge/compact/target/adr169 CARGO_INCREMENTAL=0 cargo +1.99.0 run --offline --quiet --manifest-path /tmp/rust030-adr264-repro/Cargo.toml",
  "renderer_results": [
    "control: OK recorded=[(\"outer\", true)] functions=[\"fn __compact_recorded_body_inner<Private, W: super::TryWitnesses<Private>>(\"]",
    "native_raw: OK recorded=[] functions=[\"pub(crate) fn r#type<Private, W: TryWitnesses<Private>>(\"]",
    "recorded_raw: RENDER_ERROR debug=InvalidIdentifier(\"__compact_recorded_body_r#type\") display=invalid Rust identifier \"__compact_recorded_body_r#type\"",
    "collision_control: OK recorded=[(\"outer\", true), (\"__compact_recorded_body_inner\", true)] functions=[\"pub fn __compact_recorded_body_inner<Private, W: TryWitnesses<Private>>(\", \"fn __compact_recorded_body_1_x696e6e6572<\", \"pub fn __compact_recorded_body_inner<Private, W: super::TryWitnesses<Private>>(\", \"pub fn __compact_recorded_body_inner<Private>(\", \"pub fn __compact_recorded_body_inner_call<'observed, Private>(\", \"pub fn __compact_recorded_body_inner<Private>(\"]",
    "collision_raw: RENDER_ERROR debug=InvalidIdentifier(\"__compact_recorded_body_r#type\") display=invalid Rust identifier \"__compact_recorded_body_r#type\""
  ],
  "source_compiler": "/tmp/rust030-hygiene/bin/compactc --version = 0.31.133",
  "source_commands": [
    "compactc --skip-zk --emit-rust-ir examples/rust_backend/counter_parameter.compact /tmp/rust030-adr264-repro/source/control-output",
    "compactc --skip-zk --emit-rust-ir /tmp/rust030-adr264-repro/source/raw.compact /tmp/rust030-adr264-repro/source/output",
    "compactc --skip-zk --emit-rust-ir /tmp/rust030-adr264-repro/source/keyword.compact /tmp/rust030-adr264-repro/source/keyword-output"
  ],
  "source_results": {
    "control_exit": 0,
    "raw_exit": 255,
    "raw_diagnostic": "Exception: raw.compact line 20 char 17:\n  parse error: found \"#\" looking for a generic parameter list or a pattern parameter list",
    "keyword_exit": 255,
    "keyword_diagnostic": "Exception: keyword.compact line 20 char 16:\n  parse error: found keyword \"type\" looking for an identifier"
  },
  "generated_native_raw_excerpt": "pub(crate) fn r#type<Private, W: TryWitnesses<Private>>(",
  "generated_collision_fallback_excerpt": "fn __compact_recorded_body_1_x696e6e6572<",
  "files_sha256": {
    "/tmp/rust030-adr264-repro/src/main.rs": "0dcedb498f7d1e52f4254e50100904a9555a2f24c3c5f89703f7a5c2af6f499f",
    "/tmp/rust030-adr264-repro/output.txt": "c4160fc41f801285d3fe415ff4c563865a45b161b8df0bc604f5697b791dc8b9",
    "/tmp/rust030-adr264-repro/native_raw.rs": "456e548cab13a92f35fbb6c73779c7597d418906591c6eb8e47c43203f44822d",
    "/tmp/rust030-adr264-repro/control.rs": "00c34dcc0442ac35fa044b520e14685475015a99c6dae0180af9e3a2daf7e379",
    "/tmp/rust030-adr264-repro/collision_control.rs": "3b4a98683c493feb180eab9acd5c84ab3b275bac8c593df592afe52e7cb7e76f",
    "/tmp/rust030-adr264-repro/source/raw.compact": "b4e52a6c7a2a88c1c0b0153b4c6c557ce4991cd2012f1d562274536d66daf573",
    "/tmp/rust030-adr264-repro/source/raw.log": "54c3a8b33a9d7279fbc191c1ec9b915f61ad0ea49e2a49469527bc4251dd73aa",
    "/tmp/rust030-adr264-repro/source/keyword.compact": "788307251bb2b153baf6b044f5bbafa77263e6836693a418d4236abffae889c4",
    "/tmp/rust030-adr264-repro/source/keyword.log": "0d17f68de23efd346077385472c4323140efa38bd2fac669bbc5c30228f0b2cd",
    "/tmp/rust030-adr264-repro/source/control.log": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  },
  "backend_files_sha256": {
    "/Users/ysh/.codex/worktrees/07ae/compact/tools/compact-rust-backend/src/lib.rs": "5a589aa74ecbc764222a75012bb1cdb5a76a12a778fb267fd5647aae0c46c33f",
    "/Users/ysh/.codex/worktrees/07ae/compact/tools/compact-rust-backend/src/recorded/helpers.rs": "1da61f0ac2ae4cb39b32ef3311a62fc5f71fed7e8de713a2b4d302b11454959a"
  },
  "limitations": [
    "Current Compact parser rejects Rust raw spelling r#type and keyword type in this source control; this is typed/private IR consumer boundary only on reproduced input.",
    "No production fix or generated-source behavior change was made.",
    "Raw helper-name collision fallback remains unreachable before a fix; the ordinary collision control exercises existing fallback."
  ]
}

```

### Local delivery

`f103fb345d5425c99e8e553fb21360adfd62a055`. 6newtests,10declarationtests,153renderertests,strictClippy and four unedited generated consumers pass. Exact before Err reproduced; raw-spelled preferred/fallback occupancy controls also failed before correction. Shared semantic function-name key now owns namespace and helper occupancy, implemented as private functions in naming.rs rather than a wrapper type. Ordinary/native control outputs unchanged. Scope is imported/private typed IR: Compact source grammar rejects the raw spelling. No runtime/schema/ABI change. [ADR0264 — Semantic Rust function names local receipt](references-0.3.0.md#note-030).
