---
id: RUST-ADR-0008
alias: ADR-0008
title: "Preserve Compact source locations through the Rust IR"
date: 2026-10-02
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["compiler-cli", "diagnostics", "typed-ir"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: c9f06a017f4a7ce07777069413518820eb1fb30b3f2b72d86e10a16c84f42595
---
# RUST-ADR-0008 — Preserve Compact source locations through the Rust IR

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept typed source-location metadata on supported IR owners and source-located renderer diagnostics. Later declaration-owner delivery extends the initial ledger-field slice; owner starts are not full nested spans or unambiguous imported relative paths. Historical schema7/8 transitions and remaining source-provenance limits must remain dated.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#104 closure](https://github.com/MediaNoxLabs/compact/issues/104#issuecomment-6017403902). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`1e0216c6`](https://github.com/MediaNoxLabs/compact/commit/1e0216c67835cbf7e8b160099a4856c3f8c7fbfd) · [`6925beaf`](https://github.com/MediaNoxLabs/compact/commit/6925beaf7bc9e97c67dd8f86e211a8e3f32e1097). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
status: accepted-partial
created: 2026-10-02
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/104
```

## Historical decision and amendments

### Problem

Chez has source objects and produces source-located lowering errors, but private Rust IR schema 6 drops the location. A validation error raised by `compact-rust-backend` can only say `invalid ledger field path [..]` or `expression has type ...`, with no Compact file or line. Engineers cannot reliably connect a renderer error back to a declaration. This is a production diagnostics gap tracked by [#104](https://github.com/MediaNoxLabs/compact/issues/104) in [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).

### Decision and boundaries

Carry typed, optional `SourceLocation { file, line, column }` metadata on semantic IR owners. Schema 7 is a deliberate breaking change to the private Chez-to-Rust interchange. The first delivery attaches locations to ledger declarations and decorates their validation errors. The compiler emits the basename of the Compact source and one-based line/column from Chez, keeping generated artifacts independent of a machine's absolute checkout path. Errors from hand-built IR without locations retain their prior display and equality behavior. The runtime has no role in compile-time diagnostics and will not carry source locations in on-ledger values.

Subsequent work must carry locations into exported types, witnesses, circuits, constructor steps, state actions and nested expressions, and use the narrowest available location for each renderer failure. Imported files with the same basename may require a normalized relative source path; this first slice documents that limit rather than inventing a misleading path.

### Before and after

Before:
```text
compactc: invalid ledger field path [3, 0]
```

After (renderer rejects a malformed field in `vault.compact`):
```text
compactc: vault.compact line 2 char 8: invalid ledger field path [3, 0]
```

IR before:
```json
{"id":"balance","index":3,"path":[3,0],"declaration":{"kind":"counter"}}
```

IR after:
```json
{"id":"balance","index":3,"path":[3,0],"source":{"file":"vault.compact","line":2,"column":8},"declaration":{"kind":"counter"}}
```

### Ownership and alternatives

The Chez emitter owns source extraction and private schema versioning. The Rust IR owns the typed location value; the renderer owns `RenderError` context and display; `compactc` prints the resulting diagnostic. The generated crate and runtime do not acquire compile-time source metadata. A plain location string would be simpler, but it prevents structured tests and tooling. A top-level sidecar keyed by field name cannot identify repeated or nested syntax precisely. Embedding an optional typed value at its semantic owner permits incremental coverage and explicit provenance.

### Acceptance evidence and risks

Delivery must include a renderer test for an invalid ledger declaration with location, a compiler-emitted IR location test or fixture, a public `compactc` rejection probe with file/line/column, complete fixture and schema updates, and the existing positive proof/application gate. Never claim issue #104 complete until nested expression, constructor, witness, action and type errors are located. No runtime ABI change is expected because generated runtime behavior does not change. Record commit and test evidence in a dated amendment, the issue, and [Milestone 2 — ADR delivery map](references.md#private-note-08).


### Delivery amendment — 2026-10-02

**Status: accepted, partial.** Signed and DCO conventional commit `6925beaf7bc9e97c67dd8f86e211a8e3f32e1097` delivers the first declaration slice locally on `codex/rust-backend-ast`. The branch remains local. Issue [#104](https://github.com/MediaNoxLabs/compact/issues/104) remains open in `rust-backend-v2`.

#### Engineer-facing code change

Before, the backend CLI's `main() -> Result` displayed Rust's debug representation of any renderer error:
```rust
fn main() -> Result<(), Box<dyn Error>> {
    let contract: Contract = serde_json::from_str(&input)?;
    print!("{}", render(&contract)?);
    Ok(())
}
// Error: InvalidLedgerPath([3, 0])
```

After, it prints the `Display` diagnostic, while `RenderError::source()` retains the underlying typed cause:
```rust
fn main() {
    if let Err(error) = run() {
        eprintln!("compact-rust-backend: {error}");
        std::process::exit(1);
    }
}
// compact-rust-backend: counter.compact line 18 char 1: invalid ledger field path [3, 0]
```

The IR now carries `source: Option<SourceLocation>` on `LedgerField`, and the renderer attaches it to named-type collection, path/index, Merkle depth, and duplicate-field validation failures. The Chez emitter extracts this value from the field's source object. `compact-rustc` also prints Display errors. The compiler version is 0.31.132, Rust IR schema is 7, and generated/runtime ABI stays 4. There is no generated Rust API or runtime behavior delta for valid contracts.

#### Evidence

- `cargo check --workspace` passed. `cargo test -p compact-rust-backend` passed: four CLI unit tests and 50 renderer tests, including typed error and process-level diagnostic checks.
- Nix-built `compactc` 0.31.132 emitted `counter.compact` ledger `round` at line 18, column 1. Mutating that real compiler IR to an invalid path and feeding it to the renderer CLI produced exactly the diagnostic shown above with exit code 1.
- `check_compactc_target.py` passed with an exact emitted source-position assertion; `check_rejections.py` passed both source-located rejection probes; `check_fixture_outputs.py` found 129 current generated Rust fixtures and no stale output.
- The packaged `--consumer --proof` gate passed in the Nix compiler shell, including external consumers, proof verification, replay/partition, and ledger validation/application. Rust formatting and `git diff --check` passed.
- Commit signature verified as good and DCO trailer present. No push or remote CI is claimed.

#### Remaining scope

This is a declaration-start location, not a full source span. Types, witnesses, circuits, constructor steps, state actions, nested expressions, and precise imported-module paths still need locations. The production diagnostics exit gate in #104 therefore remains open. The next schema change should model source ranges and stable relative paths before treating those errors as complete.


Delivery record: [#104 comment](https://github.com/MediaNoxLabs/compact/issues/104#issuecomment-5945965010); [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2) description updated for ADR-0008.


### Delivery amendment — 2026-10-02, declaration owners

**Problem.** The first slice located ledger declaration failures, but `PureCircuit`, `StatefulCircuit`, `WitnessDeclaration`, `Constructor`, and `TypeAlias` still lost their Chez source objects. A renderer error in one of these owners could only name the invalid type or operation. This left ordinary circuit and witness diagnostics detached from the Compact file.

**Decision.** The same typed `SourceLocation { file, line, column }` now appears optionally on all five top-level owners. Private IR schema advances from 7 to 8; compiler version is 0.31.133. The renderer attaches the owner location to type collection and output construction errors. `RenderError::at` keeps an already located cause, so a later nested expression location can remain more precise than the declaration fallback. `witness::build` attaches witness locations and ledger-view construction uses its field location. Generated Rust APIs and runtime ABI 4 remain unchanged; the runtime owns no compile-time diagnostic state.

Before (a malformed private IR circuit, shown as the relevant fields):
```json
{"name":"field_add","result":{"kind":"field"},"body":{"kind":"boolean","value":true}}
```
```text
compact-rust-backend: expression has type Boolean, expected Field
```

After, the Chez emitter carries the circuit declaration position and the renderer uses it:
```json
{"name":"field_add","source":{"file":"field_add.compact","line":18,"column":1},"result":{"kind":"field"},"body":{"kind":"boolean","value":true}}
```
```text
compact-rust-backend: field_add.compact line 18 char 1: expression has type Boolean, expected Field
```

The renderer's owner boundary is explicit Rust code:
```rust
let item = located(circuit.source.as_ref(), || {
    let (body, actual) = expression_with_calls(
        &circuit.body, &parameters, &callable_circuits
    )?;
    // Construct the checked Rust item.
    Ok(item)
})?;
```

**Emitter and review evidence.** Chez `with-source` emits positions for exported aliases, pure and stateful circuits (including internal names), witnesses, and explicit constructors. The compiler target gate checks the real positions of `field_add.compact:18:1`, `counter.compact:20:1`, `witness_cell_write.compact:20:1`, `constructor_map_actions.compact:21:1`, and `aliases_oracle.compact:27:1`, plus the existing ledger field at `counter.compact:18:1`. Local signed/DCO conventional commit `1e0216c67835cbf7e8b160099a4856c3f8c7fbfd` contains the change.

**Validation.** `cargo check --workspace`; `cargo test -p compact-rust-backend` (51 renderer tests and four CLI unit tests); Nix-built 0.31.133 compiler target gate; two source-located rejection probes; 129 generated fixture comparisons with zero stale output; and the packaged external consumer/proof/replay/partition/ledger-application gate all passed. Mutating a real compiler-emitted `field_add` IR body to Boolean produced the exact source-located renderer diagnostic above. `cargo fmt --check`, `git diff --check`, GPG signature and DCO passed.

**Remaining.** These are owner-start locations. Constructor steps, state actions, nested expressions, parameter/type syntax, full start/end spans, and stable relative paths for imports with duplicate basenames still need precise provenance. Many malformed Compact sources are rejected earlier by Chez; this amendment proves renderer context for malformed private IR and preserves the broader source-acceptance work in #104. The branch is local; remote CI has not run.


Schema-8 delivery record: [#104 comment](https://github.com/MediaNoxLabs/compact/issues/104#issuecomment-5946124664); [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2) description updated.
