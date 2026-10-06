---
id: RUST-ADR-0239
alias: ADR-0239
title: "Explicit preparation of built-in proof material"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-ci"
topics: ["CI", "proof-material", "SRS"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: d770b1e83803a6e65e953f297e47adbcfe5beb9b00ddbe6b5b328593369451a3
---
# RUST-ADR-0239 — Explicit preparation of built-in proof material

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-ci. An explicit hash-verified preparation step acquires the locked built-in proof keys and SRS before strict proof smoke, with offline verify-only refusal controls. The cold-cache test and final remote gate are bounded evidence; key bytes are not publication material.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#343 closure](https://github.com/MediaNoxLabs/compact/issues/343#issuecomment-6017813198). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c) · [`eb0e4019`](https://github.com/MediaNoxLabs/compact/commit/eb0e4019994629338e655179ddc6fc440e2c48d0). These include historical prerequisites and probes, not only final delivery commits.

## Historical decision and amendments

<!-- compact-adr-final-acceptance:03c03a39a2d39c43c91143b9526ca1c2ffae706c:ADR-0239 -->
### Final remote acceptance — 2026-10-06

At published commit `03c03a39a2d39c43c91143b9526ca1c2ffae706c`, all 10 required remote workflows passed on the same revision; the root-reviewed acceptance record and closed rust-backend-v2 milestone cover this bounded decision. [Issue #343 closure record](https://github.com/MediaNoxLabs/compact/issues/343#issuecomment-6017813198) and the [milestone closeout](https://github.com/MediaNoxLabs/compact/issues/103#issuecomment-6017383781) document the verified scope.

[Final evidence bundle](references.md#private-note-06) · [Architecture successor](references.md#private-note-07)

Earlier status statements, observations, tests and limits below remain historical evidence. Final acceptance supersedes their pending wording and does not expand implementation scope.

### Historical decision and delivery record


Date: 2026-10-06
Status: accepted; implementation and validation pending
Milestone: rust-backend-v2

### Problem

At5671d65d the Linux compiler built successfully, but native proof execution failed because `zswap/9/output.prover` was absent from the clean runner. Nine other workflows passed, including2918 extracted tests. A warm host cache had masked the missing prerequisite. `MIDNIGHT_LEDGER_TEST_STATIC_DIR` locates ledger test fixtures; it does not prepare the built-in key/SRS cache. Synchronous MidnightDataProvider resolution only reads verified local files.

### Decision and before/after

Before: run the strict qualified-coin proof immediately, implicitly depending on an existing user cache.

After:
```sh
export MIDNIGHT_PP="$RUNNER_TEMP/compact-proof-parameters"
cargo run --locked --offline -p compact-rust-proof-smoke -- --prepare-proof-material
# Existing synchronous proof gates follow, with the same cache.
```

Use the upstream union of ZSWAP_EXPECTED_FILES and DUST_EXPECTED_FILES and MidnightDataProvider.fetch to acquire and hash-verify all12 assets. Deserialize the verified built-in IR and derive the distinct required SRS sizes through its public k method, then fetch_k those sizes. Do not copy hash tables, invent an arbitrary SRS range, or require private proof artifacts. A cache hit remains hash-verified. Fetch errors fail preparation visibly. Use a Tokio runtime for upstream HTTP acquisition.

### Emitter/runtime impact

This changes the validation harness and CI prerequisites. It does not change emitted code, runtime ABI49, IR20, dependency pins, synchronous proof resolvers or strict transaction assertions. Contract-specific key generation retains its existing behavior. No universal offline acquisition claim is made: the explicit preparation step needs network access on an empty cache.

### Alternatives and costs

Changing resolvers to OnDemand hides acquisition inside proof execution and can encounter executor assumptions. Copying published crate static directories misses deliberately omitted prover keys. Reusing the host cache fails clean-runner reproducibility. Explicit preparation adds bounded downloads and cache storage; it reuses pinned upstream authenticity checks.

### Validation

Acquire into an empty isolated MIDNIGHT_PP. Repeat with transport unavailable to verify cache reuse. Corrupt or remove a file in an isolated copy and require failure. Run the unchanged qualified-coin-cell strict funded proof using prepared material; this exercises wallet output/spend, Dust fees, contract proving, application and replay. Do not claim a sign transaction from this selector. Run formatter and targeted strict Clippy. Publish a conventional GPG/DCO commit and require all ten workflows at that final revision before closure.

Evidence: https://github.com/MediaNoxLabs/compact/actions/runs/37443664998 . Preserve the failed candidate receipt alongside final acceptance.


### Publication and focused review

Published conventional GPG/DCO commit `eb0e4019994629338e655179ddc6fc440e2c48d0` delivers issue343. The branch audit now covers265 initiative commits and the milestone242 issues. The implementation reuses actual locked expected-file tables, verified tagged IR and upstream provider fetching. Twelve assets require distinct SRS k13, k14 and k15. A verify-only option performs local checked reads without repair. CI shares MIDNIGHT_PP across preparation and all following commands; a successful JSON receipt is retained with package artifacts.

Build, strict targeted Clippy, formatter, two invalid-argument checks, warm verification with a closed transport endpoint and independent source/workflow reviews passed. A new empty cache is being filled through upstream downloads; its funded proof and negative checks remain pending. Because one upstream key download is slow locally, exact-head remote validation runs in parallel. This does not waive either acceptance result.

No original proof assertion, synchronous resolver, dependency pin, runtime ABI49 or private IR20 changed. User-owned `doc/ledger-adt.mdx` remains untouched with SHA2566e126928f8fbd8b48bf16984b419e421b63a84b3db3013cc2b6c598a9ee3a8ce.

Remote compiler: https://github.com/MediaNoxLabs/compact/actions/runs/37448331896 . Closure requires all ten workflows on this exact revision plus completion of the focused cold-cache check.


### ADR239 cold-cache acceptance passed

At published eb0e4019994629338e655179ddc6fc440e2c48d0, the new preparation binary fetched all12 upstream built-ins and SRS k13/14/15 into a genuinely empty cache. Acquisition took1458.453 seconds because one Dust key response was slow; no restart or host-cache seeding occurred. This is download latency, not proving time.

Both closed-endpoint verification (7.961 seconds) and ordinary preparation (9.232 seconds) passed. Empty cache, missing output key and missing k15 SRS failed with the exact expected synchronous-provider diagnostics; single-byte corrupted output keys failed with Hash mismatch in both modes, without repair.

Fresh qualified-cell contract key generation added its separate SRS k7. The unchanged selector then passed its actual default-strict funded Zswap/Dust proof, ledger application, native/recorded state and allocation checks, and exact spent-nullifier replay rejection in26.665 seconds. Earlier balancing-disabled smoke and unfunded rejection inside the selector are explicitly distinct from funded strict success. The original verified cache retained its hashes.

Network controls used a closed provider endpoint and upper/lower HTTP(S)/ALL proxies in fresh processes; this is not an OS network sandbox. The first harness metadata file omitted dependencies and was correctly refused by the static resolver; resumption used authenticated complete metadata/current lock. This harness correction is retained and required no product source change. Raw proof logs are excluded from public evidence.

Public receipt: `${LOCAL_EVIDENCE}/compact-adr239-cold-validation/public-receipt.json`, SHA256 `47e3b779e94de6097940c1174609cdd02702576e644c06a8350bed728bf43281`. Root verified both source hashes and the executed binary against this receipt. Local issue343 acceptance is complete; final remote workflow acceptance and issue/milestone closure remain pending.
