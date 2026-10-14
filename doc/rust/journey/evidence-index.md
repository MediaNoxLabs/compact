# Evidence index and chapter source map

## Repository sources

| Topic | Repository source |
| --- | --- |
| Rust target overview | [`doc/rust-backend-ast.md`](../../rust-backend-ast.md) |
| Current Rust documentation | [`doc/rust/README.md`](../README.md) |
| 0.3 closure | [`doc/rust/0.3.0-closure.md`](../0.3.0-closure.md) |
| Final metrics | [`doc/rust/0.3.0-final-metrics.md`](../0.3.0-final-metrics.md) |
| Backend guide | [`tools/compact-rust-backend/README.md`](../../../tools/compact-rust-backend/README.md) |
| Runtime guide | [`runtime-rs/README.md`](../../../runtime-rs/README.md) |
| Published ADR register | [`doc/rust/adr/README.md`](../adr/README.md) |
| Problem statements | [`doc/cops/README.md`](../../cops/README.md) |

## Milestone anchors

| Era | Revision | Meaning |
| --- | --- | --- |
| ledger-8 | [`eb72a5ab`](https://github.com/LFDT-Minokawa/compact/commit/eb72a5ab6085bf9b8564cc80c98c973e04825c2d) | Shared starting base |
| Original Rust comparison | [`589c92ef`](https://github.com/MediaNoxLabs/compact/commit/589c92ef961ce89ac730140e81b76d4c08222a70) | Initial oracle used by Milestone 1 research |
| Original Rust retained branch | [`b9fdb80b`](https://github.com/MediaNoxLabs/compact/commit/b9fdb80bfd474c24ce0553e66b7219e32d0add90) | Branch head inspected for current chronology |
| Rust-AST Milestone 1 | [`ea4cc006`](https://github.com/MediaNoxLabs/compact/commit/ea4cc006f973632c4c7b1fb4569af2b4c93ff38a) / published [`a5ee30f1`](https://github.com/MediaNoxLabs/compact/commit/a5ee30f171322f07c06e3846117f20c9e075ef95) | Local native baseline / published branch |
| Rust-AST Milestone 2 | [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c) | Remote-accepted scoped close |
| Rust-AST 0.3 | [`f9a49666`](https://github.com/MediaNoxLabs/compact/commit/f9a496660a4a69f4dd7c7927111767ad5c0c12b2) / [`0d7610b2`](https://github.com/MediaNoxLabs/compact/commit/0d7610b2792d60fee5d3713d4eb0ede3c98cc82a) | Qualified implementation / closure |
| Rust-AST published docs | [`46fdf089`](https://github.com/MediaNoxLabs/compact/commit/46fdf0895a012a50250d9b8f3c0b49e49de48b00) | Final 0.3 metrics/deck branch head |
| Effect prototype cutoff | `f3cef799` | Saved local documentation cutoff |

## Selected published ADRs

- [ADR-0001 — replayable ledger trace](../adr/0001-record-circuit-execution-as-a-replayable-ledger-trace.md)
- [ADR-0005 — typed runtime DSL](../adr/0005-use-a-typed-runtime-dsl-instead-of-a-body-wide-macro.md)
- [ADR-0008 — source locations](../adr/0008-preserve-compact-source-locations-through-the-rust-ir.md)
- [ADR-0051 — matching runtime](../adr/0051-select-the-matching-versioned-runtime-at-code-generation.md)
- [ADR-0053 — proving capabilities](../adr/0053-report-and-require-generated-proving-capabilities.md)
- [ADR-0079 — recording gaps](../adr/0079-explain-missing-rust-recording-capabilities-from-typed-lowering.md)
- [ADR-0188 — Zswap authority](../adr/0188-record-native-zswap-intents-against-authoritative-offers.md)
- [ADR-0217 — live wallet acceptance](../adr/0217-current-revision-live-wallet-shielded-acceptance.md)
- [ADR-0327 — midpoint architecture review](../adr/0327-independent-compiler-architecture-review-at-the-midpoint.md)
- [ADR-0347 — conformance scope](../adr/0347-track-versioned-compact-specification-conformance-separately-from-fixtures.md)
- [ADR-0351 — checked VM paths](../adr/0351-check-merkle-vm-path-operands-before-program-construction.md)
- [ADR-0352 — truthful wide-operation errors](../adr/0352-distinguish-unsupported-wide-operations-from-malformed-uint-bounds.md)
- [ADR-0353 — decoder byte limits](../adr/0353-admit-serialized-ledger-inputs-with-explicit-byte-limits.md)

## Vault-only sources used during authorship

These notes are not public links and are not copied wholesale into the site:

- `Milestone 1 — Native Rust execution baseline.md`
- `Milestone 2 — Remote acceptance and closure — 2026-10-06.md`
- `Milestone 0.3.0/0.3.0 closure report.md`
- `Milestone 0.4.0/README.md`
- `Milestone 0.4.0/Recording effect algebra — Full migration plan.md`
- `Milestone 0.4.0/Delivery dashboard.md`
- `Milestone 0.4.0/Final maintainability measurements.md`
- `Milestone 0.4.0/EA13 — Final qualification and closure plan.md`
- `Milestone 0.4.0/EA13 — Test-run optimization.md`
- `TypeScript parity — what the reviews found.md`
- ADRs 0368–0494 and their linked per-slice evidence records.

Vault-only decisions are described as such. Published Git ADRs remain the
stable public link target.

## Chapter outline and evidence routing

| Chapter | Primary evidence |
| --- | --- |
| Journey | Milestone 1, Milestone 2 close, 0.3 closure, 0.4 dashboard |
| Architecture | Backend/runtime guides, ADR-0001/0005/0008/0079, current source |
| Comparison | 0.3 final metrics, parity findings, prototype census and EA13 plan |
| Corrections | CoPS register, audit ADRs, retained failed attempts |
| Contributors | Current guides, extension plan and closure criteria |
