---
id: RUST-ADR-0229
alias: ADR-0229
title: "Bind existing installer scenarios to genuine archives"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-test-only"
topics: ["installer", "archive", "scenario"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: e7f62b3d3b2265bc32fa691b56da1248bc181465b9b22fc67c22d9deb6269add
---
# RUST-ADR-0229 — Bind existing installer scenarios to genuine archives

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-test-only. Existing installer scenario tests use the genuine pinned archives and mandatory CI acquisition while retaining their original assertions and platform variants. It is not a new compiler/runtime implementation or an unbounded latest-release test.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#334 closure](https://github.com/MediaNoxLabs/compact/issues/334#issuecomment-6017796479). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`0edbfe3f`](https://github.com/MediaNoxLabs/compact/commit/0edbfe3f26f657410031ff3339ab58ff0d9672b4) · [`45c950a5`](https://github.com/MediaNoxLabs/compact/commit/45c950a5addb0f6167f4fe416c195282c2292a45) · [`d5dd3f2e`](https://github.com/MediaNoxLabs/compact/commit/d5dd3f2eeeaad337ba40fcf2639d540a9738f161). These include historical prerequisites and probes, not only final delivery commits.

## Historical decision and amendments

Status: accepted for implementation under parent assignment, 2026-10-06. Base root126f8f8e plus signed ADR228 dependency8078eb5f (source76c78e33).

### Problem and before/after

Existing update/scenario tests use mutable public release metadata, inherited HOME and ad hoc downloads. Before: `run_command(args, None, ...)` depends on current releases. After: `run_archive_command(&mut fixture, args, env, ...)` uses a fresh private cache and exact verified historical archive bytes while retaining actual installation, formatting and key-generation assertions. No emitter/runtime implementation, ABI or schema effect.

Skipping scenarios, changing expected latest versions, injecting fake installed state, or bypassing key generation would weaken existing acceptance and are rejected alternatives.

Base: wait for root integration of ADR228 signed 76c78e33; preserve completed ADR227 branch. Test-only common/mod.rs, test_update.rs, four test_scenarios*.rs, and compact-test.yml. No production, dependencies, self_scenarios, or output snapshot rewrites.

### Exact existing cohort

| Binary | Declared tests |
|---|---:|
| test_update | 11 |
| test_scenarios | 11 |
| test_scenarios_two | 10 |
| test_scenarios_three | 12 |
| test_scenarios_four | 9 |
| Total | 53 |

Existing cfgs yield 49 on ARM macOS, 44 on Intel macOS, 51 on Linux. Preserve cfgs verbatim. Pure update help/version/parser cases can retain existing read-only presentation, matching ADR227's help boundary. Every test that reads catalogue or installs archives receives one writable fixture for its complete sequence.

### Adapter design

Expose ADR228 ArchiveFixture from common. Add narrowly named common wrapper(s) taking &mut ArchiveFixture, plus the original run_command/run_command_sorted arguments. Each call refreshes fixture.environment(), merges explicit caller env afterward, invokes the existing exact assertion helper, then checks fixture.assert_no_external_requests(). Keep explicit COMPACT_DIRECTORY/custom --directory precedence and test-owned temp directories; preserve current file-content, directory inventory, keygen and formatter assertions. Do not use ArchiveFixture::run(), since its env_clear would drop required proof configuration.

Per-test fixture lifetime preserves catalogue/server/home across all sequential update/list/compile/clean calls. Exact Cargo binary remains the default, explicit downloaded overrides remain untouched. Unknown network routes fail via the archive fixture; no fallback to mutable latest metadata.

### Key generation prerequisite

Four existing scenarios compile counter without --skip-zk: sc9 (0.31.0, explicit prover/verifier output assertions), sc10 (0.30.0), sc10a (0.24.0), sc11 (0.22.0, excluded ARM macOS). These must remain actual compiler/keygen calls. Preserve inherited explicit MIDNIGHT_PP. Before any proxy-isolated run, inspect each pinned historical zk tool's parameter convention and establish the needed public parameter files from an existing verified local cache or explicit acquisition step. Do not infer compatibility from the current ledger tool. Record exact files/digests and fail actionable preflight if absent; never permit arbitrary external requests during test execution or replace keys with --skip-zk. Workflow needs the same prerequisite; if historical tools require another layout/version, report that concrete boundary before expanding.

### Mandatory workflow setup

Keep all existing nextest selection and strict Clippy. Require Python3; run ADR228's ten Python tests. Cache genuine archive bytes with runner OS/architecture + manifest digest, then always run acquire_compilers.py --platform auto --cache ... --receipt ... before nextest. Export COMPACT_TEST_ARCHIVE_CACHE using fixed runner-owned paths and retain the acquisition receipt in workflow provenance. No cache-hit bypass of verification. Add parameter setup only after confirming pinned historical command behavior; reuse existing pinned actions when possible, avoid introducing arbitrary action versions.

### Validation and evidence

Use own warm target/adr157, CARGO_INCREMENTAL=0, Rust1.99; run all five selected binaries sequentially plus archive harness and affected no-install controls as warranted by the shared wrapper. Keep raw first failures (including historical formatting/parameter differences), make no blanket fixture changes. Strict package Clippy, fmt, Python tests, actionlint, and independent review. Sign GPG+DCO; report ARM macOS execution separately from other platform metadata/workflow wiring. Real release self-update remains a separate residual.

ADR229 parameter boundary confirmed before final acceptance: historical compiler0.24 zkir1.3.0 uses Filecoin k10/29rows;0.30/0.31 midnight-zkir2.1.0 use Midnight k5/24rows. Existing0.22 output asserts Filecoin k9/49rows (not executable on local ARM). Explicit setup now verifies pinned archive/tool bytes and embedded expected SHA values, then acquires only applicable public params using authoritative source SHA256; runtime tests keep network-denying proxy. Upstream source commits a81e393e81e2fa6e7e6ff89d0ed470c7be5e74c4 (Filecoin) and0b1e7be93a0709d433212f28f16f9a96e0285e2b (Midnight). Local probe offsets for SHA bytes:0.24 zk tool8852048;0.30 tool7969040. Receipts bind full tool hashes. No implication that a source commit is exact binary source provenance. Workflow cold setup and cache verification are mandatory before nextest.
#### Corrected historical 0.22 parameter boundary

A static cross-platform check caught and corrected an initial inference before delivery. Compiler0.22 zkir1.2.0 predates MIDNIGHT_PP managed parameters. Its exact pinned archive wrapper exports ZKIR_PP="$thisdir" and ships kzg plus kzg.vp. Those files are already covered by the archive SHA and are now separately hashed in parameter provenance. No Filecoin k9 is downloaded or substituted. Existing0.22 counter expected output k9/49rows stays unchanged. Only0.24 Filecoin k10 and0.30/31 Midnight k5 require public parameter acquisition. ARM selected original scenarios pass with exact keygen assertions.

The first static0.22 expected-digest check failed (correctly exposing this distinction), and its diagnostic is retained by the independent reviewer at ${LOCAL_EVIDENCE}/compact-adr229-old-helper-x86-macos-refusal.log. The corrected helper validates original wrapper binding and both bundled files. A permanent unit test accepts this binding and rejects an altered one. Help/version of the historical x86 tool was inspected locally under translation; no original non-ARM scenario execution success is claimed.
### ADR229 signed delivery

Commit `b85a7cfb0cd9bbe133182344b81a5b7eecdfdd33` is GPG verified with DCO. Parent `8078eb5f560add853bf4a3641fedc51fc8ee4cc3` is the signed local cherry-pick of ADR228 source76c78e33; parent root should integrate original228 plus this229 commit, not duplicate the dependency. Root base126f8f8e; own branch `codex/adr229-installer-scenario-fixtures` is clean.

The existing 53 declared update/scenario tests retain exact arguments, expected outputs, key/formatter assertions and cfgs. On ARM macOS all49 original selected cases plus5 shared transport checks pass at this exact signed head. Real sc9/sc10/sc10a key generation passes using a two-file cold-prepared parameter directory; no skip-zk was introduced. 17 Python guards, package strict all-target/all-feature Clippy, actionlint, package fmt and committed diff checks pass.

Historical0.22 differs: its original wrapper selects bundled kzg/kzg.vp through ZKIR_PP. Setup records those hashes from the verified archive; it does not acquire Filecoin k9.0.24 selects Filecoin k10;0.30/31 select Midnight k5. Expected managed hashes come from pinned upstream source and are embedded in the verified tools. Initial incorrect0.22 managed-parameter assumption was caught by static validation and corrected before signing; refusal log is retained. Independent corrected x86mac/Linux static setup passes, without claiming original non-ARM scenario execution.

Workflow setup is mandatory before unchanged nextest: pinned Python action,17 acquisition guards, hash-keyed archive/parameter cache, verification even on hits, explicit cold acquisition, absolute cache env paths and uploaded public provenance receipts. Existing Clippy and test selection stay intact. No production, dependency or expected-output-file changes. ADR230 self-update setup remains a separate integration with its source files.

Receipt `${LOCAL_EVIDENCE}/compact-adr229-delivery-receipt.json`; focused log `${LOCAL_EVIDENCE}/compact-adr229-focused-signed.log`; static review receipt `${LOCAL_EVIDENCE}/compact-adr229-cross-platform-static-receipt.json`; parameter provenance `${LOCAL_EVIDENCE}/compact-adr229-parameters-signed-receipt.json`; cold-download receipt `${LOCAL_EVIDENCE}/compact-adr229-parameters-cold-receipt.json`; binary/k/row bindings `${LOCAL_EVIDENCE}/compact-adr229-probe-bindings.json`.

Exact tested CLI SHA256 `33e3da1a4699c65fe9a8cc08d816123252db807bead92bdfca5d1bc2d57062e1` at `${COMPACT_SOURCE}/target/adr157/debug/compact`. Own warm target/adr157, Rust1.99, CARGO_INCREMENTAL=0. No remote CI, publication, broad proof suite or foreign-platform execution claim. Independent review reports no remaining actionable findings.

### Genuine installer scenarios integrated — 45c950a5 (2026-10-06)

ADR228 / [#331](https://github.com/MediaNoxLabs/compact/issues/331) and ADR229 / [#334](https://github.com/MediaNoxLabs/compact/issues/334) are integrated as **0edbfe3f** and **45c950a5addb0f6167f4fe416c195282c2292a45**. The complete root Git tree equals the tested signed source `b85a7cfb0cd9bbe133182344b81a5b7eecdfdd33`. Root reports **254 conventional, GPG-signed, DCO commits**, 17 Python guards passing, and the user-owned ledger document unchanged.

All **49 original ARM macOS update/scenario cases plus five shared transport checks** pass at that source, including the three actual counter key-generation scenarios. Original command arguments, output fixtures, directory overrides, formatter/key assertions and platform guards remain intact. Genuine historical archives are hash-pinned; private per-scenario state and a refusing HTTP proxy replace mutable release discovery. The workflow now prepares Python, verifies archives even on cache hits, acquires the exact required public parameters before tests, exports absolute paths and retains provenance receipts.

The parameter boundary is version-specific: compiler 0.22 retains its original archive-local `ZKIR_PP` with bundled `kzg`/`kzg.vp`; compiler 0.24 uses Filecoin k=10, while 0.30/0.31 use Midnight k=5. Only those last two small managed parameter files are acquired. Independent **Intel macOS and Linux static archive/parameter setup** passed; this is not execution of their original scenario suites. The initial incorrect inference that 0.22 used managed Filecoin parameters was caught and corrected before delivery.

Evidence: `${LOCAL_EVIDENCE}/compact-adr229-delivery-receipt.json`, `${LOCAL_EVIDENCE}/compact-adr229-focused-signed.log`, `${LOCAL_EVIDENCE}/compact-adr229-parameters-signed-receipt.json`, and `${LOCAL_EVIDENCE}/compact-adr229-cross-platform-static-receipt.json`. Strict package Clippy, actionlint, formatting and diff checks pass. No compiler/runtime, schema 20, ABI 49, production installer or dependency changes occurred; the earlier **d5dd3f2e product/full-gate reference and separately recorded live lifecycle head** remain unchanged.

Remaining local installer work: integrate ADR230's reusable historical self-update fixture together with its mandatory workflow acquisition hook, then run the combined full installer selection. Its disposable probe and agent tests do not yet replace that combined checkpoint. Remote CI, other-platform execution and publication remain separate. No push or remote CI was triggered. Earlier dated entries below are historical and are superseded by this status where they describe ADR228/229 as pending.
