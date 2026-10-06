---
id: RUST-ADR-0159
alias: ADR-0159
title: "Record composite witnesses and typed commitment values"
date: not-recorded
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "composite-values", "witness", "typed-hash"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 1866002dcf69cba4b74248a963941cd1989860414f999d8946ab966fe1a0a8bc
---
# RUST-ADR-0159 — Record composite witnesses and typed commitment values

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted typed composite witnesses and a completely audited commitment-value helper for original Zerocash mint, preserving three callback/private-output steps before historic insertion. The review amendment restricts new call boundaries to identity struct coercions and rejects widening rather than erasing types; general composite-call coercions and spend remain separate.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#263 closure](https://github.com/MediaNoxLabs/compact/issues/263#issuecomment-6017675310). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

### Original source metadata

```yaml
status: accepted
issue: https://github.com/MediaNoxLabs/compact/issues/263
```

## Historical decision and amendments

### Problem
Original Zerocash mint is accepted and executes natively but recording stops at its first composite witness binding. It must preserve the source sequence: new coin witness, public key witness, add-coin Unit witness, typed pure commitment hash, historic Merkle insertion. The commitment, coin, and public key are structs backed by existing checked codecs.

### Before / after
Compact stays unchanged:
```compact
const coin = context$new_coin_info();
const pk = private$zk_public_key();
private$add_coin(coin);
const cm = commitment_from_coin_info(coin, pk);
commitments.insert(cm);
```
Before: generated native calls work but recorded/observed mint is unavailable.
After, conceptually:
```rust
let (frame, coin) = frame.try_witness_metered(new_coin)?;
let (frame, pk) = frame.try_witness_metered(public_key)?;
let (frame, ()) = frame.try_witness_metered(add_coin)?;
let commitment = pure_circuits::commitment_from_coin_info(coin, pk)?;
let frame = ledger_slots::commitments.record_insert(frame, commitment)?;
```
Actual generated signatures remain typed; values follow the normal clone/ownership rules.

### Proposed emitter change
Reuse recordable structural value checks for composite witness output and a Unit witness input. Admit typed struct-returning pure helpers only after recursively auditing their complete value expression (parameters, projections, struct/tuple construction, byte constants/coercions and persistent hashing); reject effects or unsupported calls. Use the existing native pure function body and typed recorded local instead of duplicating its algorithm. No names or fixed hashes decide admission.

### Runtime and boundaries
Existing checked derives, metered witness frame, ledger8 historic tree and midnight-zk hashing own behavior. No schema or ABI change expected. Spend remains an independent capability; any changed first-gap diagnostic must be documented in the original-source cohort.

### Acceptance
Original and oracle source capability joins; fresh TS/private/gas/public-query/state capture; native/recorded/replay parity with exact witness order; tampered helper/type/effect guards; pinned proof verify and ledger apply; all fixtures fresh; targeted Clippy and immutable focused receipt. No delivery yet.


### Review correction — typed call boundaries
Independent review found that the existing recorded local map stores expressions without their source types. Reusing cell_source for a newly admitted struct helper or struct-result witness can erase a Uint8-to-Uint16 coercion and emit an ill-typed call. This slice admits identity struct coercions and rejects other explicit argument coercions at both new boundaries. Native validation guarantees these struct coercions preserve the exact type. Two negative renderer cases retain the widening source IR, one as a pure helper and one as a witness. Fully typed recorded local provenance is a follow-up; this slice does not claim general composite-call coercion coverage.

### Local evidence before commit
Fresh unchanged Zerocash TS mint matched native and recorded execution: ordered new_coin/public_key/add_coin callbacks; exact private outputs (2 byte atoms, 1 byte atom, Unit); one public query and exact Verify program; serialized pre/post state. All four gas fields agree: read 3740000000, compute 6626175185, written 1939, deleted 504. Verify replay has the same state/effects/gas. Pinned proof at ${LOCAL_EVIDENCE}/compact-adr159/proof passed generated trace partitioning, proof verification and ledger deployment/call validation+application. No runtime, schema or ABI change. Spend remains unsupported at its nested assertion.


### Signed delivery
Commit c36a135b5df5c516ed5cdf4dc039faff656d9db5 (GPG good, DCO), pending main integration after its frozen broad checkpoint. 129 renderer tests, 3 Zerocash fixture tests, strict backend/fixture/proof Clippy and 152 fresh fixtures (zero stale/failed). Original election+Zerocash source cohort passes. Widening reproducer compiles as a native consumer offline, reports recording unavailable, and both helper/witness variants are regression guarded. Pure mint generated output is unchanged by this correction; its proof evidence remains applicable. Focused receipt: ${LOCAL_EVIDENCE}/compact-focused-c36a135b/receipt.json.
