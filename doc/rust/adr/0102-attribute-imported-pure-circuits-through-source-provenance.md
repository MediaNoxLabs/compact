---
id: RUST-ADR-0102
alias: ADR-0102
title: "Attribute imported pure circuits through source provenance"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["inventory", "source-provenance", "pure-circuits"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 1eb923559ffb2ac2a589672b25f198fb6e16acf6702b2e6e5b6df3f3cab1dc50
---
# RUST-ADR-0102 — Attribute imported pure circuits through source provenance

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted source-inventory attribution through a bounded, root-contained include closure and unique compiler-classified pure names. This establishes metadata provenance for imported declarations, not Rust API availability or behavioral parity; ambiguous names, escapes and mismatches remain rejected.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#205 closure](https://github.com/MediaNoxLabs/compact/issues/205#issuecomment-6017577762). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`f3142ad5`](https://github.com/MediaNoxLabs/compact/commit/f3142ad5f6a3b5d624f55e6081c431c5d6576f88). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 102
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/205
```

## Historical decision and amendments

### Problem

At integrated commit f3142ad5, the 191-source inventory reports 120 exported circuits as unassessed. The compiled digital-passport package root `src/digital-passport-credential.compact` emits 77 `contract-info.json` entries that are all `pure=true`, `proof=false`, but these declarations live in included files. The inventory joins compiler rows only to lexical rows with the package-root source path, so all 77 remain unassessed even though the compiler evaluated them. This is metadata attribution, not a codegen or semantic parity gap.

### Before and after report

Before, a representative imported row remains unknown:

```json
{"source":".../credentials/types.compact","name":"assertValidSchemaRef","proof_required":null,"compiler_pure":null,"compiler_metadata_source":null}
```

After a provenance-checked package-root join, the same row retains its source identity and receives only compiler applicability metadata:

```json
{"source":".../credentials/types.compact","name":"assertValidSchemaRef","proof_required":false,"compiler_pure":true,"compiler_metadata_source":".../src/digital-passport-credential.compact","rust_recording_status":"not_applicable"}
```

No generated Rust API is implied; `rust_recorded` and `rust_observed_call` stay null for pure declarations absent from the capability report. The explicit `compiler_metadata_source` points to the compiled package root that supplied the attribution, while the declaration `source` continues to point to its original included file.

### Decision and safety constraints

For an explicitly compiled package root, follow only quoted local `include` paths transitively, resolving each against its including file and requiring a real `.compact` file inside the inventory root. Preserve lexical declaration source, module path, signature, visibility and baseline identity. `contract-info.json` does not carry declaration file paths, so attribution is an inference from the compiled root, local include closure, unique name and pure/proof flags. A compiler metadata entry may attribute an included exported pure circuit only when the declaration is reachable from that package root, its name has exactly one candidate across the root and included closure, and the compiler entry is uniquely named with Boolean `pure=true`, `proof=false`. Reject ambiguous names, duplicate metadata, local path escapes, missing included files, or pure/proof mismatches rather than guessing. Continue the existing direct root join and capability consistency checks. Do not relabel module-only exports or add rows; this is a narrow package-root metadata join.

The expected digital-passport include closure has 18 source files and 77 unique exported pure declarations, matching all 77 compiler metadata names. The compiler Rust capability list is empty for this package. Expected full summary: 191 sources and 296 proof-required circuits unchanged; unassessed exported circuits 120→43, nonproof 265→342. No emitter, runtime, IR/schema, proof or ledger behavior changes.

### Validation

Create small temporary-source tests for a reachable imported pure row, an unreachable same-name row, duplicate/ambiguous names, malformed or escaping includes, and pure/proof mismatches. Verify the exact full 191-source inventory delta using the f3142ad5 compiler and pinned schema-11 Scheme, plus the checked-in identity baseline. Run the inventory unit suite and focused Python syntax checks. A compiler metadata `proof=false` classification must never be described as TypeScript/Rust behavior parity. Root runs the combined exact-head gate after integration; no push or remote CI.

### Tracking

- MediaNoxLabs issue: https://github.com/MediaNoxLabs/compact/issues/205 (rust-backend-v2).
- Local branch: `codex/adr101-module-provenance` from f3142ad5.
- Signed GPG/DCO commit: `022bdc0a13f8826e332b26d06374fe98fcd0a064`.


### Local delivery, 2026-10-05
Signed GPG/DCO conventional commit 022bdc0a13f8826e332b26d06374fe98fcd0a064 is based on f3142ad5. The pinned schema-11 compiler package-root contract-info contains 77 unique pure/proof-false entries. The local include graph reaches 18 source files and 77 matching exported pure declarations. All 77 rows now retain original declaration source and carry compiler_metadata_source pointing to the package root; no Rust capability API is inferred. The exact 191-source receipt at ${LOCAL_EVIDENCE}/adr102-full-inventory-final.json has zero identity additions/removals, 296 proof-required and 218 proof-available unchanged, nonproof 265 to 342, unassessed exported circuits 120 to 43, and zero unmatched or missing compiler rows. Focused package-root inventory reports 18 sources and 77 nonproof rows. All 17 inventory unit tests, Python syntax and diff checks pass. This is proof applicability metadata attribution only; TypeScript/Rust behavior parity is not established by it. Root exact-head combined gate remains after integration. No push or remote CI.
