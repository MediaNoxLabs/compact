---
id: RUST-ADR-0228
alias: ADR-0228
title: "Real pinned compiler archives for installer acceptance"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-test-only"
topics: ["installer", "archive", "provenance"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 472d0dd954eed84d3a0783db09662646271457e718cb67bb6dc7fe710a47dabd
---
# RUST-ADR-0228 — Real pinned compiler archives for installer acceptance

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-test-only. A separate hash-pinned fixture acquires genuine historical compiler archives for the supported platform/version matrix, preserving unavailable combinations. It provides acquisition trust and metadata; ADR229 wires existing scenarios to those assets.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#331 closure](https://github.com/MediaNoxLabs/compact/issues/331#issuecomment-6017791022). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`0edbfe3f`](https://github.com/MediaNoxLabs/compact/commit/0edbfe3f26f657410031ff3339ab58ff0d9672b4) · [`45c950a5`](https://github.com/MediaNoxLabs/compact/commit/45c950a5addb0f6167f4fe416c195282c2292a45) · [`d0e499f3`](https://github.com/MediaNoxLabs/compact/commit/d0e499f33d65dd568670ed34ed64fff32a7ce54b) · [`d5dd3f2e`](https://github.com/MediaNoxLabs/compact/commit/d5dd3f2eeeaad337ba40fcf2639d540a9738f161). These include historical prerequisites and probes, not only final delivery commits.

## Historical decision and amendments

- Status: signed bounded delivery; integration waits for ADR229 mandatory setup and existing scenario adapters.
- Issue: https://github.com/MediaNoxLabs/compact/issues/331
- Date: 2026-10-06
- Milestone: rust-backend-v2
- Base: d0e499f3

### Problem

Existing installer update and compiler/formatter scenarios download mutable release metadata while asserting historical compiler versions and exact outputs. The user's cache/default installation can influence these tests. ADR226 supplies isolated read-only check/list metadata and ADR227 isolates no-install commands, but neither exercises genuine installation or compiler execution. Pre-installing version directories would bypass download/unzip and change the asserted installed/default messages.

### Decision and trust boundary

Add a distinct writable test fixture with real historical compiler archives. Keep production sources, dependencies, common/mod.rs and existing integration files untouched in this first slice. New manifest lists versions 0.22.0, 0.23.0, 0.24.0, 0.30.0 and 0.31.0 with faithful platform absence, exact official release/asset IDs, URL, size and SHA256. Cover every architecture needed by configured Ubuntu/macOS CI; only ARM macOS execution is claimed locally. Pin published Linux ARM assets when available without inventing missing legacy support.

A small acquisition helper validates bytes and publishes an acquisition receipt in a task-owned cache. Existing cached bytes may be reused only after hash/size checks. GitHub reported digests are provenance, not a signature. Old assets without a publisher digest use explicitly labeled hashes of exact official downloads; 0.23.0 ARM matches the independent existing cache. Do not substitute the current 0.31.133 binary for historical 0.31.0 or fake compiler scripts.

The new fixture serves verified ZIP bytes on localhost through existing Wiremock facilities, seeds the existing parsed-release cache with those local URLs and a fresh timestamp per child, and supplies a private HOME/config/cache/default install/receipt directory. Unexpected network goes to a refusal proxy. Each test retains its own writable installed state; immutable archive data may be shared. It remains separate from ReadOnlyBaseline's no-installed-artifacts invariant.

### Before / after

Before:

```text
update -> live GitHub latest -> repeated upstream ZIP downloads -> user-dependent cache
scenario expected version = 0.31.0
```

After:

```text
acquire helper -> checked manifest / immutable real ZIP cache / public receipt
writable fixture -> private parsed release cache -> localhost exact ZIP
update -> ordinary production download/unzip/default selection
compile/format -> genuine historical compiler bytes and existing assertions
```

No production installer behavior, compiler emitter, runtime, schema 20 or ABI 49 changes. Existing original test assertions are preserved when root wires the shared adapter later. Historical installer/self-update replacement is a separate follow-up, not claimed here.

### Owned files

Only new files: pinned manifest, acquisition helper/tests, tools/compact/tests/common/archive_fixture.rs and a dedicated new integration harness. Do not edit common/mod.rs, existing tests or workflow jobs concurrently with ADR226/227. Root serializes later fixture adapters and all integration.

### Local gate

- Helper tests: missing/wrong/duplicate manifest identities, absent platforms, corrupt/size-mismatched bytes, partial download/output refusal, cache reuse and no silent latest fallback.
- Acquisition validates actual historical ARM archives and retains exact public provenance.
- Focused fixture harness uses real 0.31.0 installation, version/output behavior, compile-generated artifacts and update/default/clean transitions, while asserting no unexpected external request and isolation from sentinel host paths.
- Explicit missing fixture/corrupt fixture refusal; no silent network acquisition during test execution.
- Strict focused Clippy/format, header validation, signed conventional GPG+DCO delivery. No push or remote CI. Non-ARM platform artifact coverage is provenance/acquisition only, not executed acceptance.

### References

- ADR225/#329, ADR226/#330, ADR227 pending no-install isolation
- [Milestone 2 — Installer stabilization plan and artifact provenance — 2026-10-06](references.md#private-note-11)
- ${LOCAL_EVIDENCE}/compact-installer-fixtures-research-receipt.json
- ${LOCAL_EVIDENCE}/compact-installer-remaining-acceptance-plan.md


### Accepted scope refinement

Root approved the bounded implementation after issue #331 was created. Preserve the complete nine-version official catalogue (0.22, 0.23, 0.24, 0.25, 0.26, 0.28, 0.29, 0.30, 0.31), since existing list assertions require it; only the five versions actually executed by existing scenarios have downloadable archive pins. Fifteen platform assets are pinned across published macOS/Linux variants. Missing platforms remain absent.

Use a small standard-library TCP listener as both archive endpoint and refusal proxy: only exact own-origin pinned asset routes are accepted. This avoids additional dependencies and supports counting denied CONNECT/external/unknown requests. The acquisition helper is Python standard library; --verify-only rehashes actual cache bytes immediately before serving, never relying solely on an old receipt. Mandatory setup commands and platform mapping are documented in tests/fixtures/README.md; root must wire acquisition before integration of the new all-tests harness. No test skip or implicit runtime download is allowed.

The focused smoke executes genuine install/default/repeat/clean and counter compilation with --skip-zk; original scenario key generation and formatting assertions remain separate and unchanged until adapters are integrated. Local execution scope is aarch64-darwin.


### Signed delivery — 2026-10-06

Commit **76c78e33cac7b140cffc5680dce8600edb1fb368**, conventional subject, GPG verified and DCO; checkout clean. Six new files only, no common helper or existing scenario modifications. Root reviewed the implementation with no blocker. ADR229 owns the original update/scenario adapters and mandatory Python/acquisition workflow setup; these must integrate together so no tests are skipped or left without required assets.

Passed: ten acquisition tests; four integration tests (genuine install/default/repeat/counter compilation/clean, missing and corrupt cache refusal, external/unknown/partial request refusal); focused strict Clippy; rustfmt; full new-source headers. The successful integration run took 2.91 seconds on ARM macOS. Actual counter compiler output is tested with --skip-zk; original scenario proving-key and formatter assertions remain separate. New-file copyright years were corrected afterward without semantic changes, then headers/rustfmt rechecked.

Receipt `${LOCAL_EVIDENCE}/compact-adr228-receipt.json`, SHA256 `2f869582d573fe7fe9d402c25b51b3a50c54e92d6dc10354fd99c5d5f2ba3469`. Actual prepared cache: `${COMPACT_SOURCE}/target/installer-fixtures-research/verified`. All nine catalogue versions and 15 archive pins preserve published platform absences. Four legacy asset hashes (0.22 Intel/Linux and 0.23 ARM/Linux) are explicitly hashes from exact official downloads because GitHub reports no publisher digest. Other platforms have metadata coverage, not local execution acceptance.

Initial fixture-only macOS download truncation came from accepted sockets inheriting nonblocking mode; explicit blocking mode with bounded timeouts fixed it. The first failure log remains `${LOCAL_EVIDENCE}/compact-adr228-real-fixture.log`. No compiler/runtime/ABI/dependency changes, no push and no remote CI. Historical self-update remains separately scoped.

### Genuine installer scenarios integrated — 45c950a5 (2026-10-06)

ADR228 / [#331](https://github.com/MediaNoxLabs/compact/issues/331) and ADR229 / [#334](https://github.com/MediaNoxLabs/compact/issues/334) are integrated as **0edbfe3f** and **45c950a5addb0f6167f4fe416c195282c2292a45**. The complete root Git tree equals the tested signed source `b85a7cfb0cd9bbe133182344b81a5b7eecdfdd33`. Root reports **254 conventional, GPG-signed, DCO commits**, 17 Python guards passing, and the user-owned ledger document unchanged.

All **49 original ARM macOS update/scenario cases plus five shared transport checks** pass at that source, including the three actual counter key-generation scenarios. Original command arguments, output fixtures, directory overrides, formatter/key assertions and platform guards remain intact. Genuine historical archives are hash-pinned; private per-scenario state and a refusing HTTP proxy replace mutable release discovery. The workflow now prepares Python, verifies archives even on cache hits, acquires the exact required public parameters before tests, exports absolute paths and retains provenance receipts.

The parameter boundary is version-specific: compiler 0.22 retains its original archive-local `ZKIR_PP` with bundled `kzg`/`kzg.vp`; compiler 0.24 uses Filecoin k=10, while 0.30/0.31 use Midnight k=5. Only those last two small managed parameter files are acquired. Independent **Intel macOS and Linux static archive/parameter setup** passed; this is not execution of their original scenario suites. The initial incorrect inference that 0.22 used managed Filecoin parameters was caught and corrected before delivery.

Evidence: `${LOCAL_EVIDENCE}/compact-adr229-delivery-receipt.json`, `${LOCAL_EVIDENCE}/compact-adr229-focused-signed.log`, `${LOCAL_EVIDENCE}/compact-adr229-parameters-signed-receipt.json`, and `${LOCAL_EVIDENCE}/compact-adr229-cross-platform-static-receipt.json`. Strict package Clippy, actionlint, formatting and diff checks pass. No compiler/runtime, schema 20, ABI 49, production installer or dependency changes occurred; the earlier **d5dd3f2e product/full-gate reference and separately recorded live lifecycle head** remain unchanged.

Remaining local installer work: integrate ADR230's reusable historical self-update fixture together with its mandatory workflow acquisition hook, then run the combined full installer selection. Its disposable probe and agent tests do not yet replace that combined checkpoint. Remote CI, other-platform execution and publication remain separate. No push or remote CI was triggered. Earlier dated entries below are historical and are superseded by this status where they describe ADR228/229 as pending.
