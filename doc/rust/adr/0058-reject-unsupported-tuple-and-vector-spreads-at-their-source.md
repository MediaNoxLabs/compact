---
id: RUST-ADR-0058
alias: ADR-0058
title: "Reject unsupported tuple and Vector spreads at their source"
date: 2026-10-04
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["compiler-cli", "diagnostics"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: c4d2dbd6fd20a11df0e6cc35f758e6bb4f711d208fee7909963d5c01b3595cc9
---
# RUST-ADR-0058 — Reject unsupported tuple and Vector spreads at their source

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept precise source-located refusal of unsupported tuple/Vector spreads before misleading arity errors. This is diagnostic support, not spread expansion. Preserve the frontend source-object column convention, no-spread controls and later pristine-head validation without broadening language support.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#157 closure](https://github.com/MediaNoxLabs/compact/issues/157#issuecomment-6017496471). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`2f0f3841`](https://github.com/MediaNoxLabs/compact/commit/2f0f3841e0c4bc8a8c1d2705c26692b3137d5177). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 0058
status: accepted-local
date: 2026-10-04
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/157
```

## Historical decision and amendments

### Problem

The Rust target rejects a valid Compact spread, but the diagnostic identifies the enclosing return or assignment as an unsupported circuit expression, or reports a tuple length mismatch before recognizing the spread. The frontend keeps a source object on each spread argument. A developer needs the unsupported token and precise line/column; an error at the enclosing statement hides the cause.

Small reproducer:
```compact
export pure circuit spread(v: Vector<2, Field>): Vector<2, Field> {
  return [...v];
}
```

### Before

```text
spread.compact line 2 char 3: Rust backend does not yet support this circuit expression
```

The no-spread control `return [1, 2];` compiles. A tuple spread currently reports a tuple literal length mismatch at the enclosing expression.

### Decision and after

Reject a `spread` tuple argument before arity validation, using its own Chez source object. Recognize the `Lnodisclose` Vector form sufficiently to reject a spread explicitly; this slice does not claim to implement spread expansion.

```text
spread.compact line 2 char 10: Rust backend does not yet support vector spreads
```

The exact one-based column is fixed by a compiler-backed test. Chez attaches the spread argument's source object at the opening bracket, one character before the first dot. Supported no-spread Vector/tuple literals must keep compiling and no rejected program may publish a partial output directory.

### Alternatives and rationale

Implementing spread expansion would broaden language semantics and requires type, disclosure and runtime parity work. A generic owner-level rejection retains a correct failure status but gives the wrong location and reason. This focused diagnostic uses source data that already exists in Chez, so the private IR schema, generated crate and runtime do not change.

### Emitter and runtime ownership

The Chez Rust IR pass owns the early unsupported-spread rejection and source selection. The typed IR and `syn` renderer remain unchanged. The generated Rust API, ledger-8/zk primitive mapping, runtime ABI 28, and private IR schema 8 remain unchanged. The public compactc target owns atomic output publication; the rejection gate must verify that behavior.

### Verification and risks

Add compiler-backed negative probes for pure Vector and tuple spreads at exact Compact positions and no output/staging debris. Keep a positive no-spread control. Run compiler tests, source rejection gate, fixture freshness, and generated consumer/proof gates from the changed head. This improves diagnostics only; spread support remains open under broader #104 acceptance. A source object may represent an expression immediately following the ellipsis rather than the punctuation itself; the test must record the actual frontend position honestly.

### Tracking and delivery

- Issue: [#157](https://github.com/MediaNoxLabs/compact/issues/157), assigned to rust-backend-v2 before implementation.
- Milestone: https://github.com/MediaNoxLabs/compact/milestone/2
- Local commits: `2f0f3841e0c4bc8a8c1d2705c26692b3137d5177`, conventional, GPG-signed and DCO.
- Delivery state: local; branch publication and same-commit remote CI remain open.

### Amendments

Append implementation evidence and any correction to the expected column here. Preserve the initial hypothesis above.


### Delivery amendment — 2026-10-04

**State and issue.** Local conventional GPG-signed/DCO commit `2f0f3841e0c4bc8a8c1d2705c26692b3137d5177` implements the decision under [#157](https://github.com/MediaNoxLabs/compact/issues/157) in rust-backend-v2. The branch is 111 commits ahead of the remote M1 cut and has not been pushed; no remote CI claim is made.

**Before/after.** For `return [...v];` in a pure Vector circuit, the pre-change packaged compiler reported `line 2 char 3: Rust backend does not yet support this circuit expression`. The changed Nix-built compiler reports `line 2 char 10: Rust backend does not yet support vector spreads`. The parallel tuple case changed from `Rust tuple literal length does not match its type` at the enclosing expression to `line 2 char 10: Rust backend does not yet support tuple spreads`. Chez marks the spread argument source at the opening bracket, before the dots; the message is at the nearest available nested source object and does not claim dot-level precision.

**Emitter and runtime.** `reject-spreads` inspects `Lnodisclose Tuple-Argument` nodes before arity checks, using each spread's source object. `expression-ir` also recognizes the lowered `vector` form so it cannot fall through to a generic circuit-expression error. Typed pure/stateful Vector and tuple lowering and stateful tuple expressions call the same rejection helper. The generated crate, Rust renderer, ledger adapter, zk/proving mapping, ABI 28 and private IR schema 8 are unchanged. Spread semantics remain unsupported.

**Evidence.** A Nix-built compiler from the changed source (`${HISTORICAL_NIX_STORE}/70r6lfw592xdln27griz41bnm6xafylh-compactc`) reported the two exact positions; both rejected outputs were absent while no-spread pure Vector and tuple controls built. `check_rejections.py` now checks four explicit source rejections, both controls, no staging debris and byte preservation of an existing generated output on rejected rebuild; it passed 0 failures. `check_fixture_outputs.py` passed 137/137 fresh fixtures. The packaged `check_compactc_target.py --consumer --proof` gate exited 0, including generated Cargo consumers, proof verification and ledger application. The full `compiler/go` Scheme/JavaScript suite exited 0 with PATH containing the repository's installed Vitest binary; an initial compiler-shell attempt lacked `vitest` on PATH and failed only its two JavaScript runner invocations. `git verify-commit` reports a good GPG signature and the DCO trailer is present. These gates ran from a rehearsal checkout carrying the same two source changes before commit; exact committed-head clean replay and remote CI remain pending.

**Limits.** This is a source-located rejection for two unsupported syntaxes, not full nested source spans, spread implementation, or complete #104 language acceptance. Imported path disambiguation remains open. No measured generated-source size or compile-time change is expected because valid output is unchanged; no optimization claim is made.


#### Pristine committed-head replay — 2026-10-04

A clean detached checkout at exact `2f0f3841e0c4bc8a8c1d2705c26692b3137d5177` built Nix `compactc` `${HISTORICAL_NIX_STORE}/nsmhgkw6lbysbxipvdfy11m17q3f6yk4-compactc`. That compiler passed the four-case rejection gate with both no-spread controls and existing-output preservation (0 failures), 137/137 fixture freshness, and the 37/37 pinned oracle inventory. `nix develop --command ./srcMaps/test.sh` passed six VLQ tests and 132 assertions. The exact-head Nix compiler passed generated consumer and proof/ledger application with exit 0; preserved `${LOCAL_EVIDENCE}/compact-adr58-proof-2f0f3841.log` contains 95 replayed/partitioned generated traces and 95 validated/applied proven calls, ending in `compactc target boundary and manifest: passed`. This supersedes the earlier limitation that only a changed-source rehearsal had run. The remote branch/CI gate remains open; there is no new runtime ABI or IR schema change.
