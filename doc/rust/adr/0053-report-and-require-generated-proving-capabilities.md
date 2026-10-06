---
id: RUST-ADR-0053
alias: ADR-0053
title: "Report and require generated proving capabilities"
date: 2026-10-04
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["compiler-cli", "diagnostics"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 3542621aedfba25e005bb0c9f9dc4d1db53803e63cbefdedb016c230ad31bd87
---
# RUST-ADR-0053 — Report and require generated proving capabilities

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept machine-readable recorded/observed-call capabilities from the actual typed lowering and an opt-in strict requirement before publishing output. Later ADR0054 invalidates the initial multiplication-based negative example; historical refusal examples are versioned evidence, not a current unsupported-language list. Capability availability is not proof execution.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#152 closure](https://github.com/MediaNoxLabs/compact/issues/152#issuecomment-6017487946). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`1b4f9128`](https://github.com/MediaNoxLabs/compact/commit/1b4f9128ba40f86e372833355ec347a0dcc9803c) · [`f155ae78`](https://github.com/MediaNoxLabs/compact/commit/f155ae7834bfcc9acec31648019f8e4eef45f7e2). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 53
status: accepted-partial
date: 2026-10-04
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/152
```

## Historical decision and amendments

### Problem and real compiler repro

A Rust target may succeed while an exported stateful Compact circuit has only a native method and no replayable recorded or observed call. The exact-head ABI-28 packaged `compactc` accepts this source with exit 0:

```compact
import CompactStandardLibrary;
export ledger value: Field;
export circuit write(left: Field, right: Field): [] {
  value = disclose(left * right);
}
```

The generated `Contract::write` exists, but `pub mod recorded` and `write_call` do not. A consumer discovering this at Rust compile time could have already accepted a purported proof-ready artifact. The renderer deliberately withholds incomplete traces; that safety choice needs an explicit machine-readable and CLI acceptance boundary. Pure circuits have no ledger call to prove and are outside this per-export report.

### Before and proposed after

Before, `render(&ir)` returns only Rust text. Its recorded loop silently skips `None` from `render_recorded_circuit`:

```rust
if let Some(item) = render_recorded_circuit(circuit, /* typed IR context */)? {
    recorded_items.push(item);
    // Generate typed recorded and observed call methods.
}
```

After, return a typed `RenderOutput` from the same pass, with each exported stateful circuit's source position and exact emitted capability. The ordinary `render` API remains a compatibility wrapper. `compactc --target rust` writes `contract/rust-capabilities.json` into the staged output, alongside the generated crate; the refreshed compiler manifest hashes it. `--rust-require-recording` rejects any exported stateful circuit without both a complete recorded method and its typed observed call. The rejection names the circuit and Compact source location and publishes no partial output. This option is for callers who need a provable contract; ordinary native-only generation remains explicit in the report.

Proposed report shape (schema version is independent of private IR schema 8 and runtime ABI 28):

```json
{
  "schema_version": 1,
  "circuits": [
    { "name": "write", "source": { "file": "...", "line": 3, "column": 1 },
      "recorded": false, "observed_call": false }
  ]
}
```

A supported Counter export reports both booleans true. A future name collision can report `recorded: true, observed_call: false` rather than overclaiming a developer-facing preparation path.

### Alternatives and ownership

Inferring support by parsing emitted `lib.rs` would undo the typed AST ownership and make formatting part of the contract. Treating every native-only circuit as a hard error would break legitimate Rust-only execution and many oracle fixtures before recording coverage is complete. A README table cannot describe a particular generated crate. The emitter already decides recordability in one pass, so it should expose that decision as data. `compactc` owns the report file, strict flag, source-located rejection and staging transaction. The runtime, macro, derives, ledger-8/zk primitives, generated public Rust API, ABI 28 and private IR schema 8 do not change. The report is compiler metadata, versioned separately; no runtime compatibility promise attaches to its first schema.

This first slice reports capability, not a precise reason for every unsupported expression. A follow-up reason taxonomy should come from typed renderer decisions, not text matching. The strict option's invariant is exact emitted API availability; it does not prove semantic parity or network admission.

### Acceptance and risks

1. Real packaged `compactc` native-only Field-product source succeeds normally with a report marking `write` false/false; strict mode fails at its Compact source position with no final output or leaked stage, and preserves an earlier output byte-for-byte on a failed strict rebuild.
2. Counter and another complete recorded circuit report true/true and compile with strict mode. A recorded method whose observed name collides reports true/false and fails strict mode. Internal circuits are omitted; source order is stable.
3. The report is included in the refreshed output manifest and checked against the emitted methods in renderer/CLI regressions. The external consumer and full packaged proof/ledger gate remain green. No false claim that the report itself verifies proofs.
4. Record exact signed/DCO commit, before/after source, tests, report schema/compatibility, and limits here, in the focused issue and [Milestone 2 — ADR delivery map](references.md#private-note-08). Branch remains local unless separately authorized.

### Tracking and history

- 2026-10-04: proposed after the real packaged ABI-28 repro at local signed rc5. Focused issue to be created in `rust-backend-v2` before implementation; parent #103 and #104.


- 2026-10-04: focused [MediaNoxLabs/compact#152](https://github.com/MediaNoxLabs/compact/issues/152) created and assigned to `rust-backend-v2` before implementation.


### Local delivery amendment — 2026-10-04, ABI 28

Conventional GPG-signed/DCO commit `1b4f9128ba40f86e372833355ec347a0dcc9803c` delivers the decision. The implementation adds `render_with_capabilities(&Contract) -> RenderedContract`, where the exact typed recorded-method pass collects each exported stateful circuit's source position and `recorded` / `observed_call` booleans. Existing `render` remains a source-only wrapper, and all 137 generated fixture libraries stay byte-for-byte current. `compactc` writes version-1 `contract/rust-capabilities.json` before refreshing the hashed output manifest. `--rust-require-recording` checks that report before publication, rejects missing recorded or observed APIs at the original Compact location, and preserves the previous output via the ADR-0052 staging boundary. Internal circuits and pure circuits are omitted. The backend gains the additive `render_with_capabilities` API; the generated Rust API, runtime and derive crates, ledger-8/zk mapping, ABI 28 and private IR schema 8 are unchanged.

Actual default output for the real Field-product repro:

```json
{"schema_version":1,"circuits":[{"name":"write","source":{"file":"product.compact","line":3,"column":1},"recorded":false,"observed_call":false}]}
```

Strict mode exits 1 with `product.compact line 3 char 1: exported circuit "write" has no complete recorded API`; no fresh final output or stage remains, and a failed strict rebuild leaves the prior tree hashes unchanged. A real `bump` / `bump_call` name collision reports `bump` as recorded true / observed false and rejects it at line 3. Counter and Boolean Cell strictly compile with both APIs true. Renderer tests verify internal omission, source order, report schema and method/report agreement. The report describes emitted APIs only; it is not a semantic proof or wallet admission statement.

Evidence: eight CLI tests and 57 renderer tests passed; packaged source rejection/output-publication/capability checks passed with zero failures; generated Counter/Boolean Cell and witnessed external consumers passed; 137 fixture outputs were current; bounded all-features backend check, formatting, Python syntax and scoped diff checks passed. A local wrapper over the pinned Scheme frontend passed the full `--consumer --proof` gate. More strongly, the clean detached checkout at this exact commit and GPG-signed annotated `rust-backend-v2-abi28-rc6` produced `target/rust-runtime-release-abi28-clean-rc6.json` (`dirty: false`), verified it on a second package run, built Nix `compactc` `${HISTORICAL_NIX_STORE}/1v3jw3q6lafyh2v7w3m0zyd2s64bsbxs-compactc`, passed packaged rejection checks and the untouched two-contract archive-only consumer with one shared runtime, then passed the exact-head `check_compactc_target.py --consumer --proof` gate. That gate generated ZKIR/keys, replayed traces, proved, independently verified, ledger-validated and applied its current offline cases, and wrote sealed Counter deploy/call handoffs. Macro/runtime archive SHA-256: `d6301b1741a4bd04e733cda549d1302b36fbf497e2925fc4a1dee0dc1d9b2413` / `66c896b511e7ba0fd35b0aa0417685c0f0d82190f306120f477f763f11028b40` (9/247 entries).

The first metadata schema has booleans but no precise typed reason taxonomy for every unsupported expression. Strict mode guarantees API availability, not TypeScript parity, proof validity for an arbitrary new circuit, authenticated observation, concurrency safety, registry distribution or remote CI. The branch and rc6 tag remain local. [#152](https://github.com/MediaNoxLabs/compact/issues/152) stays open for same-head remote/release gates.


### Coverage supersession — 2026-10-04, ADR-0054

The Field-product source used above was a valid native-only repro at rc6/`1b4f9128`. Signed/DCO `f155ae7834bfcc9acec31648019f8e4eef45f7e2` implements [ADR-0054 — Record Field subtraction and multiplication in ordered frames](0054-record-field-subtraction-and-multiplication-in-ordered-frames.md) / [#153](https://github.com/MediaNoxLabs/compact/issues/153), so the same product expression now emits recorded and observed-call methods and reports true/true. This does not change ADR-0053's capability-report and strict-mode contract. Its negative acceptance probe now uses a real unsupported pure-call/write source: `export pure circuit square(x: Field): Field { return x * x; }` followed by `export circuit write(input: Field): [] { value = disclose(square(input)); }`. The rc7 local wrapper and packaged compiler both report false/false, reject strict mode at the source, preserve previous output, and leave no fresh stage. The original product evidence remains here as dated history, not a current unsupported-language claim.
