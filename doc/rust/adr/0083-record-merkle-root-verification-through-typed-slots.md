---
id: RUST-ADR-0083
alias: ADR-0083
title: "Record Merkle root verification through typed slots"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "runtime", "merkle", "recording", "compatibility"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 09c39b71c5687b8c20c374e3d34aeedbc5e71e94b453c25d70ed227283637ef1
---
# RUST-ADR-0083 — Record Merkle root verification through typed slots

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted the bounded witnessed Merkle-root expression path through typed recording slots. A compatibility amendment corrects the initially stated ABI35 to ABI36; the integrated checkpoint records both Boolean outcome proofs/application after refresh. Prepared append/replace state is not a chained proof of those mutations, and broader Merkle domains are separate.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#183 closure](https://github.com/MediaNoxLabs/compact/issues/183#issuecomment-6017540604). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`2895c986`](https://github.com/MediaNoxLabs/compact/commit/2895c98603b04d3a9fcb9a2a94ad0fba23a30ece) · [`34fc44b2`](https://github.com/MediaNoxLabs/compact/commit/34fc44b239a6cf1a211da6c01b817b3063e388ee) · [`50333d6b`](https://github.com/MediaNoxLabs/compact/commit/50333d6b98dad542d9a77498fbce711a728750f0) · [`706d251e`](https://github.com/MediaNoxLabs/compact/commit/706d251e7281ffc78a598be32b3287002ca1e753) · [`744badf3`](https://github.com/MediaNoxLabs/compact/commit/744badf38335fb56dbf57e7a131fb1d4d873da8a). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 83
status: accepted-partial
date: 2026-10-05
milestone: rust-backend-v2
```

## Historical decision and amendments

### Problem and testable scope

`examples/rust_backend/merkle_path_verify.compact::verify` calls witness `leaf_path()`, computes `merkleTreePathRoot<3, Uint<8>>`, then `t.checkRoot(...)`. The compiler's `contract-info.json` marks `verify` `proof:true`; TypeScript places it in `ProvableCircuits` and emits `verify.zkir`. Generated Rust has a native `verify` method and typed `MerkleSlot::check_root`, but no `recorded::verify` or `verify_call`. It is the sole proof-required circuit among the 47 action-free `StateReturn::Expression` first blockers in the measured 137-fixture corpus. A blanket ReturnExpression recorder would manufacture methods for 46 proof-false neighbors.

Before consumer code can execute the native result but cannot obtain a typed provable call:

```rust
let native = contract.verify(context)?;
// contract.recording.verify_call(&observed_state, private_state) does not exist.
```

After the proposed vertical slice:

```rust
let call = contract.recording.verify_call(&observed_state, private_state)?;
let proven = prover.prove(call)?;
ledger.apply(proven)?;
```

Signatures above are conceptual and must be confirmed in generated consumer tests. The generated recorder should call typed `ledger_slots::t.record_check_root(frame, root)` after the existing witness and pure-root path, not emit raw `Dup`, `Idx`, `Root`, `Push`, `Eq`, `Popeq` instructions. TypeScript currently emits that ordered query sequence; it is an independent parity oracle. The native `MerkleSlot::check_root` already exists. Ledger-8's current `check_root_program` result shape and recording-frame observation need inspection before the typed recorded API is finalized; whether to add a ledger-backed verify/read variant or adapt a proven primitive must be decided from upstream semantics. Avoid a local VM reimplementation.

### Proposed ownership

The emitter owns eligibility and AST generation for this exact `MerkleCheckRoot`-return shape and witness/pure-root expression graph. The runtime owns `MerkleSlot::record_check_root` and `RecordingFrame` delegation if upstream ledger primitives can return the observed Boolean and faithful VM transcript. Pinned midnight-ledger and midnight-zk own VM semantics, FAB and proof verification. Bump runtime ABI only if a public runtime method is introduced; private IR schema 8 can remain if it already carries the typed action and return. ADR-0082 owns the proof applicability/report/gate semantics and must land or be accounted for independently.

### Acceptance and uncertainty

First subset is exactly `merkle_path_verify.verify`, with source/TS contract-info proof flag and ZKIR pinned. Compare native/recorded/replayed result and state, private FAB, four-dimensional gas, ordered public operations, source-to-proof verification and ledger application for both a valid path and a deliberately invalid root/path. Add generated Cargo consumer evidence and strict recording success for this proof-required circuit. Re-rank other Merkle/HistoricMerkle variants only after this case; no blanket coverage is claimed. Inspect upstream `check_root_program` return semantics, transaction observation and whether invalid-root failure or `false` is the contract behavior before implementation. No code, runtime ABI or proof gain is claimed by this proposal.


### Tracking

Focused rust-backend-v2 issue: https://github.com/MediaNoxLabs/compact/issues/183. This ADR is proposed and has no code delivery yet.
### Typed IR audit (2026-10-05)

A frozen 50333d6b compiler snapshot emitted schema-8 `verify` as `StateReturn::Expression`, not direct `StateReturn::MerkleCheckRoot`. Its `Expr::Let` binds `tmp_9: MerkleTreeDigest` to `merkleTreePathRoot(coerce(leaf_path()))`; the body is `Expr::MerkleCheckRoot { field: t, index: 0, root: Parameter(tmp_9) }`. It has no actions. The recorded lowering must therefore preserve witness evaluation, the typed pure root calculation, and the ledger read in that order. A direct return arm alone would leave this circuit unsupported. The checked Rust IR already carries these nodes, so schema 8 need not change. The runtime ledger helper currently emits gather-mode `Dup, Idx, Idx, Root, Push, Eq, Popeq` for plain trees; the new record path must use the same ledger primitive with a verify-mode observed result and retain its gas and effects. This note refines the proposed scope; acceptance remains limited to the exported proof:true `verify` circuit.
### Local implementation evidence (2026-10-05)

A narrow schema-8 lowering now recognizes the one-binding `merkleTreePathRoot(coerce(leaf_path()))` followed by plain `MerkleCheckRoot`. It emits `frame.try_witness_metered`, the existing generated pure circuit, then typed `ledger_slots::t.record_check_root`; other expression shapes retain explicit unsupported reasons. The runtime reuses midnight-ledger VM operations through the same generic gather/verify `check_root_program` and pins the observed Boolean in `Popeq`. The generated `recorded::verify`, borrowed recording handle and `verify_call` are now present. This adds only backward-compatible runtime methods; runtime ABI 35 and IR schema 8 remain unchanged.

The pinned TS 0.16.101 capture shows `afterAppend=true` and `afterReplace=false`, each with one ordered `Dup, Idx, Idx, Root, Push, Eq, Popeq` query, gas readTime 340000000, computeTime 1425433517, zero bytes written/deleted, one private witness path output and 715-byte serialized state. Rust native, recorded and replay match both results, state, four gas dimensions, public program and private FAB value/alignment; an invalid slot path rejects. The focused package has 2 passing tests; focused fmt and Clippy pass. Frozen compiler SHA-256 cb00b568ce9b9c995aab20d8ad41cfdbfcf6db7f53a75dbfe2c32e339e6d9958 and Scheme SHA-256 7762515422c08a71e1626ce2172bebbaefb58a32930a2979997948b9b3bad6d2 compiled source HEAD 50333d6b. The receipt is `${LOCAL_EVIDENCE}/compact-local-parity-2ozmdvfy/receipt.json`: 2/3 exported APIs recorded, all 3 proof-eligible, with `replace` the remaining proof-required gap.

Strict compactc emitted verify `.zkir`, `.bzkir`, prover and verifier keys using the pinned ledger-8 ZKIR v2 CLI. A separate `--merkle-verify` proof smoke prepared the generated observed call, proved, verified and applied both Boolean outcomes through ledger-8. Its deployment states are prepared with the generated native append/replace operations; this checks the verify proof and ledger application, not a chained proof of append or replace. Verify prover key is 2,811,435 bytes, verifier key 2,119 bytes. The complete all-fixture/full proof gate has not run in this isolated worktree; root integration will run combined gates after cherry-pick.

### Compatibility correction
The generated verify method calls new public MerkleSlot::record_check_root and RecordingFrame::merkle_check_root methods. Runtime ABI is therefore 36, and the compiler emits an ABI 36 assertion. The schema-8 IR and capability-report schema are unchanged. Focused ABI36 renderer tests (68/68), Merkle fixture tests (2/2), and an immutable compiler snapshot receipt (1 source; 2/3 recorded; compiler SHA-256 97fc8dc3d0a57647d7ec271c781bf2fdb0cea597b495da575dbf979ddb55cf48) passed. The full 137-fixture ABI assertion refresh is deferred to the integration commit after cherry-pick to avoid conflicts with ADR-0081. The proof smoke previously passed at ABI35, but its ABI36 rerun must follow that global fixture refresh because its other fixture dependencies currently assert ABI35. No claim of complete ABI36 proof acceptance is made before that rerun.

### Integrated ABI36 proof checkpoint — 2026-10-05

Cherry-picked as signed/DCO `744badf3`, refreshed all 137 generated fixture ABI assertions in `34fc44b2`, and corrected the strict rejection expectation in `2895c986`. The exact full local gate at `706d251e7281ffc78a598be32b3287002ca1e753` passed generated consumers and pinned source-to-proof-to-ledger proof smoke. `merkle_path_verify.verify` generated an observed call, proved and applied both true and false Boolean outcomes on ledger-8. The log is `${LOCAL_EVIDENCE}/compact-local-full-abi36-706d251e/logs/280-consumer-proof-ledger.log`; receipt `${LOCAL_EVIDENCE}/compact-local-full-abi36-706d251e/receipt.json` binds frozen compiler SHA-256 `f5c8e80e3b50a9b6aee8425b928e55fb98bdc55dba6479e5f53e7270da5b1db8`. This closes the ABI36 rerun requirement stated above. `merkle_path_verify.replace` and other Merkle/HistoricMerkle variants remain separate proof-required gaps.
