---
id: RUST-ADR-0009
alias: ADR-0009
title: "Pin oracle acceptance provenance separately from behavior"
date: 2026-10-02
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["distribution", "provenance", "validation"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 71bfd7a326e4f422b2ab7bf9185a5b51117a19fb7fbe5b7b6778f698b08020af
---
# RUST-ADR-0009 — Pin oracle acceptance provenance separately from behavior

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept a pinned source/provenance manifest and fast file/link checks separate from behavioral acceptance. The37-source inventory establishes identities and evidence associations, not all-input TypeScript parity or proof coverage. Preserve the historical matrix and link present repository evidence without relabeling later coverage as an original result.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#104 closure](https://github.com/MediaNoxLabs/compact/issues/104#issuecomment-6017403902). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`2d66965b`](https://github.com/MediaNoxLabs/compact/commit/2d66965bae870ecb3fa699a438f3019fa69a3117). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 9
status: accepted-partial
date: 2026-10-02
milestone: rust-backend-v2
issue: "https://github.com/MediaNoxLabs/compact/issues/104"
```

## Historical decision and amendments

### Problem

The backend README says all 37 top-level `codegen-rust` oracle fixtures are ported, but a fixture regeneration pass only compares Rust output against checked-in Rust output. It does not prove that a local Compact source still matches the oracle, or that a TypeScript capture and executing Rust test are attached. Engineers could accidentally edit a local fixture and keep a green generator test while losing the intended comparison. The issue #104 exit gate also needs a precise distinction between source provenance, test presence, result/state parity, negative acceptance, gas, and VM transcripts.

### Before

```sh
# A green result said only that generated lib.rs matched its checked-in copy.
COMPACTC=compactc python3 tools/compact-rust-backend/check_fixture_outputs.py
```

The README's “37 exact oracle fixtures” claim was also too broad: 35 local files are byte identical to `codegen-rust` at commit `589c92ef961ce89ac730140e81b76d4c08222a70`; `chunked_ledger` and `sealed_ledger` have changed full-line comments but identical executable lines. This is an audit result, not a compiler behavior difference.

### Decision and after

Keep a pinned, reviewable manifest of the 37 upstream source names, local paths, full local SHA-256 hashes, non-comment-line SHA-256 hashes, Rust test files, and TypeScript fixture JSON files. A fast checker validates those files and links in CI without requiring the upstream Git remote. It intentionally makes only provenance and inventory claims.

```sh
python3 tools/compact-rust-backend/check_oracle_acceptance.py
# Checked 37 pinned oracle sources; 0 failed
```

```json
{
  "oracle_source": "examples/aliases_fixture.compact",
  "source": "examples/rust_backend/aliases_oracle.compact",
  "rust_tests": ["tests-rust-backend/aliases-oracle/tests/aliases_oracle.rs"],
  "typescript_fixtures": ["runtime-rs/tests/fixtures/aliases-oracle.json"]
}
```

See [Oracle acceptance matrix — 37 codegen-rust fixtures — 2026-10-02](references.md#private-note-12) for the per-fixture mapping and the dimensions still awaiting evidence. Source hashes live in the manifest rather than this example.

### Alternatives and rationale

Depending on `origin/codegen-rust` in CI would make a clean checkout depend on a mutable branch and remote availability. Merely counting `_oracle.compact` files would conflate additional local probes with the original 37. A pinned commit and hashes make source drift explicit while letting engineers update the baseline through review. Full-line comments are excluded from the executable-line hash because two fixtures deliberately replaced historical prose; the full local hash still catches comment edits.

### Emitter and runtime ownership

No typed IR, emitter, runtime, generated API, or runtime ABI changes. This is an acceptance-evidence boundary around existing generated-crate tests. The CI and README now invoke and explain the manifest checker. When a future emitter/runtime change affects a fixture, its own ADR amendment must record before/after generated Rust, ownership, parity/proof evidence, and limitations; the manifest alone cannot approve that change.

### Verification and risks

The current source audit found 35 byte-identical fixtures and two comment-only differences. All 37 have at least one Rust test file and a referenced TypeScript fixture JSON; the manifest has 38 Rust test files and 41 `#[test]` functions in total. `check_oracle_acceptance.py` passed 37/37; Python syntax compilation and `git diff --check` passed. The commit signature and Signed-off-by trailer were verified. The checker does not execute Rust tests or independently authenticate the JSON captures. It does not establish per-fixture result/state breadth, negative counterparts, gas, VM transcript, or proof parity. Issue #104 remains open for those dimensions and the `tiny`/`election`/`zerocash`/passport flows.

### Tracking and delivery

- Issue: [MediaNoxLabs/compact#104](https://github.com/MediaNoxLabs/compact/issues/104).
- Milestone: [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).
- Local commit: conventional GPG-signed and DCO-signed `2d66965bae870ecb3fa699a438f3019fa69a3117`; M2 branch remains local.
- Delivery state: local gate and audit; remote CI and behavioral acceptance remain open.

### Amendments

Append dated revisions or write a superseding ADR when the evidence policy changes. Preserve the original source audit and its limitations.


Issue delivery record: [#104 comment](https://github.com/MediaNoxLabs/compact/issues/104#issuecomment-5946224203). The issue remains open in `rust-backend-v2`.
