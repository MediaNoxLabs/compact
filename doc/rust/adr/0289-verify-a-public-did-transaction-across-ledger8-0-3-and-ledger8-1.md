---
id: RUST-ADR-0289
alias: ADR-0289
source_sha256: ff848ec49e70f6ab23d28660af4366fa66daf91b73837c52616f666e77b01fad
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0289 — Verify a public DID transaction across ledger8.0.3 and ledger8.1

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted-isolated-transaction-bridge. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

```yaml
status: accepted-isolated-transaction-bridge
date: 2026-10-07
milestone: "0.3.0"
parents: [R030-09, R030-16]
issue: https://github.com/MediaNoxLabs/compact/issues/412
```

## ADR-0289 — Verify a public DID transaction across ledger8.0.3 and ledger8.1

### Problem
ADR0287 establishes original-release oracle equality, but DID's application lock selects ledger-v8 8.1.0 while the Rust runtime constructs transactions with8.0.3. Previous coherent8.1 compile/golden probes do not establish actual transaction/proof acceptance. A version label change is insufficient.

### Decision
Perform approved isolated Slice B: transport a public, fee-funded Rust8.0.3 transaction for one controller-authorized mutation of unchanged original DID, its public pre/post ledger and exact context/material identities into a minimal executable using the coherent upstream8.1 graph. Independently validate strict proof/well-formedness/apply/replay and compare public wire roundtrips through JS ledger-v8 8.1.0. No production pin promotion, compatibility metadata weakening, runtime/compiler/source-contract changes, authorization redesign, safety audit, CI or push.

| Before | After |
|---|---|
| Rust8.0.3 strict original-DID proof receipts and isolated8.1 representation probe | Same actual8.0.3 transaction bytes are independently decoded/verified/applied by8.1 with exact public context and compared through JS8.1 |

```rust
// Producer: public accepted artifact, never ProofPreimage/private witness/seed.
tagged_serialize(&balanced_transaction, &mut public_transaction_bytes)?;
// Receiver: matched upstream8.1 graph, exact public bytes, default strictness.
let verified = tx.well_formed(&prestate, WellFormedStrictness::default(), time)?;
let (after, result) = prestate.apply(&verified, &context);
```

### Boundaries and ownership
Only narrow test-only proof-smoke public interchange emission after coordinating source freezes; no emitted contract/runtime changes. Receiver and JS adapter are isolated under `/tmp/rust030-adr289`. Export only sealed/proven public transaction, public ledger/contract state, time/network/block context, verifier/proof public inputs and artifact hash manifest. Never serialize proving preimages, private witness outputs, RNG state, seeds or private wallet state. Existing source fixtures use synthetic test data; transaction funding provenance remains explicit. Root reviews any repository changes before commit.

### Identities
Original DID commit `4e7f6b0f69bf4e2c8506a9693f8d0c3dfe68e550`, unchanged source/import hashes in existing source manifest. Producer ledger8.0.3 revision `615be91b079ed8df4026c1fd75352ea6d49de1a4`, exact current lock. Receiver ledger/Zswap8.1.0 revision `d89e0b6334f83bc9477152fb5edf7eca71660237`, published ledger SHA256 `2182054f3a43ccabac514448fff2487437be293f517a01afc23905df701e8548`; Zswap SHA256 `4c83f946d8ac03abea38ca470599801fa08e91e2ddf3cb0963027a7701180c7c`. Candidate graph onchain-runtime/VM3.1.0, state3.0.0, transient2.1.0, serialize1.1.0, storage-core1.2.0/storage2.0.1, circuits6.1.0/proofs0.7.1/zk-stdlib1.1.0, ZKIR2.1.0. Exact lock and artifact checksums retained; zero incompatible duplicate Midnight carriers. Upstream-declared ledger-static9.0.0 is not consensus ledger9. JS8.1 archive SHA256 `16502aadeab3088a99775c26bce60383017bf1370da12a316e0034ace1e1ae1a` and original DID lock SHA512. Preserve old audited proof-static resolver; no bypass of its8.0.3 guard.

### Success and refusal
- First one real controller mutation after actual strict constructor-data deployment, no handpatched public contract state. Producer independently verifies proof and changed-input refusal.
- Full-consumption8.1 decode and public reserialization, strict proof verification/well-formedness, fee-funded application and exact intended public state/accounting; same-time replay refuses with unchanged ledger.
- JS8.1 public transaction/state/operation roundtrips compare actual bytes and exposed values. Unsupported JS private preparation/proof APIs are explicitly outside claim.
- Wrong graph/material, corrupted/trailing bytes, altered public binding, unknown profile, strict rejection or unexplained state difference refuses and yields a minimal regression. Never regenerate a different8.1 transaction or relax checks to call the transport successful.
- Candidate evidence is finite wire compatibility, not a supported native8.1 runtime profile, all-export qualification, full wallet/API migration or network acceptance. DID#353 and compatibility#360 remain open.

### Plan and resource controls
Inspect actual receiver APIs before a large build. Reuse isolated ADR0252 warm target where possible, four jobs, no whole-workspace8.1 rebuild, no other cache deletion. Synchronize source freeze with DID/compiler agents before proof-smoke edits. Root acceptance required before widening this slice or promoting the graph.

### Local delivery
One actual Rust8.0.3 controller-rotation transaction is accepted by the coherent Rust8.1 receiver: independent real contract-proof verification, changed-public-input refusal, default-strict complete transaction verification/application, whole post-ledger byte equality and exact same-time replay refusal with unchanged ledger. Six JS8.1 public carriers roundtrip byte-for-byte and trailing/truncated bytes refuse. All19Midnight dependency archives and596extractedsourcefiles match pinned identities; ledgerproof-verifying enabled, mockverify absent. No productiongraph promotion. Three focused exporter-context guards pass; producer andreceiver strictClippy pass. Finalproducer original deployment/rotate/recover/deactivate proof scenario passes, but onlyrotate exported/qualifiedcrossprofile. Rootreview added refusal for a whitelist or reference state differing from exported preledger. Exact publicBlockContext is serialized; expectedpoststate is direct apply before TestState automaticpostblockupdate. Publicledger parameters are transported unchanged, not substituted with candidate defaults. [ADR0289 — Public DID ledger8.1 transaction bridge](references-0.3.0.md#note-070). Parents remain open; root reviews producer patch before commit.
