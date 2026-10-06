---
id: RUST-ADR-0230
alias: ADR-0230
title: "Prove historical self-update with pinned local releases"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-test-only"
topics: ["installer", "self-update", "provenance"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 0ec2d4e2adb0e1dfccbb068f8eb233d392d62a1001ab10b4b1c8b5ba29edefd3
---
# RUST-ADR-0230 — Prove historical self-update with pinned local releases

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-test-only. A disposable localhost fixture exercises genuine historical 0.5.0 install, check and 0.5.1 self-update with hash-verified assets and private home/receipt paths. Local ARM execution is demonstrated; other platform expectations remain separately configured and remote-tested, not inferred from ARM.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#333 closure](https://github.com/MediaNoxLabs/compact/issues/333#issuecomment-6017794672). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`126f8f8e`](https://github.com/MediaNoxLabs/compact/commit/126f8f8e0569f7063fa89bb3e300597913a75088) · [`157d7033`](https://github.com/MediaNoxLabs/compact/commit/157d703312d8b37a097379c9ae437f4e4694cf17) · [`c90188a9`](https://github.com/MediaNoxLabs/compact/commit/c90188a93e7608a9a7d515c2c8efd5f2bc53d845) · [`d5dd3f2e`](https://github.com/MediaNoxLabs/compact/commit/d5dd3f2eeeaad337ba40fcf2639d540a9738f161). These include historical prerequisites and probes, not only final delivery commits.

## Historical decision and amendments

Status: Accepted for bounded local research probe; no repository implementation yet.
Milestone: `rust-backend-v2`. Base: root `126f8f8e` (2026-10-06).

### Problem and before/after

The existing Compact installer/self-update scenarios depend on live release metadata and current user installation state. ADR225–229 stabilize binary selection, read-only checks and compiler archive fixtures, but do not yet prove a genuine historical launcher can replace itself without touching the host installation. A static source inspection found axoupdater 0.9.1's GitHub Enterprise override and the cargo-dist installer download override; the dynamic 0.5.0→0.5.1 path remains untested.

Before: historical self-update depends on mutable GitHub responses and may inherit HOME, receipt or install paths. After this probe: an unchanged, hash-checked 0.5.0 installer downloads a real pinned archive from a closed localhost route, produces its genuine private receipt, and its privately installed binary checks/updates to the real pinned 0.5.1 archive through a closed local metadata endpoint. The original owner/repository identity remains `midnightntwrk/compact` in receipts and metadata; only transport hosts are local.

### Compiler/runtime boundary

No compiler, Rust emitter/runtime, IR schema, generated crate or installer production source changes. The first run uses a temporary orchestration script only. A reusable checked-in fixture would require a subsequent implementation review and signed commit. This probe cannot claim cross-platform or public-network behavior.

### Exact bounded plan and acceptance



1. Use a new disposable directory containing `HOME`, `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_BIN_HOME`, install prefix, download/cache and logs. Never invoke a binary from the user's installation or write to the user's HOME. Capture the existing host `compact` executable path/hash and user installation paths read-only before/after to prove no replacement.
2. Load the acquisition receipt and validate **each** 0.5.0/0.5.1 installer script and ARM archive by exact path, byte count and SHA256 before exposing it to the server. Check published `.sha256`/`sha256.sum` and `dist-manifest.json` against the retained bytes; differentiate published hashes from local acquisition pins. Refuse missing/changed assets. Unpack archive copies into a separate private directory only to hash the exact old/new executables expected after install/update; retain the archive bytes unchanged.
3. Start a loopback HTTP server with a closed route table and request ledger. It serves only validated bytes: versioned 0.5.0 and 0.5.1 archive/checksum/manifest routes plus the exact 0.5.1 `compact-installer.sh`. It serves `/api/v3/repos/midnightntwrk/compact/releases/latest` as a pinned minimal GitHub release JSON with `tag_name=compact-v0.5.1`, `name`, `url`, `prerelease=false`, and a `compact-installer.sh` asset whose `browser_download_url` is the loopback route. Any other path/method fails and is recorded. No mutable GitHub metadata or fake compiler binary is introduced.
4. Set `COMPACT_INSTALLER_GHE_BASE_URL=http://127.0.0.1:<port>/` and leave `COMPACT_INSTALLER_GITHUB_BASE_URL` unset: axoupdater 0.9.1 joins `api/v3` and requests the exact latest-release route. Set all HTTP(S)/ALL proxy variables to a separate refusing loopback proxy and `NO_PROXY=127.0.0.1,localhost`; record any refusal connection as a test failure. Clear inherited auth tokens and updater working-directory overrides. Keep request timeouts and a fixed total deadline.

### Two-step genuine lifecycle

5. Run the **hash-checked unchanged 0.5.0 installer script** via a private shell with `COMPACT_DOWNLOAD_URL=http://127.0.0.1:<port>/compact-v0.5.0`, `COMPACT_INSTALL_DIR=<private install prefix>`, private XDG directories and `COMPACT_NO_MODIFY_PATH=1`. Require its actual archive download/unpack, genuine cargo-dist receipt at private `XDG_CONFIG_HOME/compact/compact-receipt.json`, installed executable hash equal to the expected archived 0.5.0 binary, `compact --version` = 0.5.0, receipt source owner/name and install-prefix identity. Do not preinstall/copy the binary to the final install path.
6. Launch **that private installed 0.5.0 executable by absolute path** for `self check` then `self update`. Before these child processes, change `COMPACT_DOWNLOAD_URL` to the **0.5.1** route so the fetched 0.5.1 script cannot accidentally download the old archive. Preserve `COMPACT_INSTALLER_GHE_BASE_URL` for the loopback metadata API and set `AXOUPDATER_CONFIG_PATH=<private XDG_CONFIG_HOME>/compact`; ensure no inherited `AXOUPDATER_CONFIG_WORKING_DIR` wins. Check the `self check` reports update available from 0.5.0 to 0.5.1, and `self update` fetches the pinned 0.5.1 script plus 0.5.1 archive through the allowlist.
7. After the old process exits, hash the same private installed executable path: it must equal the expected 0.5.1 archived executable and differ from old. Run that path with `--version` and require 0.5.1. Parse the updated receipt and require version 0.5.1, unchanged official source owner/name, unchanged private install prefix, and matching binary location. Optionally run a second `self check` for Up to date. Verify all server request routes and order, no refusal-proxy hits, no user-home/host-binary hash changes, no unexpected private paths outside the fixture, and no command left running.

### Negative boundaries and reporting

- A wrong/missing archive hash, unexpected route, stale `COMPACT_DOWNLOAD_URL` causing 0.5.0 bytes after update, malformed receipt, failed new executable hash, or any proxy hit aborts acceptance. Retain raw stdout/stderr, response route ledger, exact asset hashes, source binary hashes, before/after receipt hashes and private install-tree manifest in a receipt tied to the source revision.
- The local probe proves one ARM macOS 0.5.0→0.5.1 path against genuine historical artifacts with a simulated GitHub API; it does not prove public-network availability, other OS/architectures, a current 0.5.1→future update, or publisher signatures beyond the retained release metadata/digests.
- Implementation needs a separate ADR/issue and review before the first execution. Keep initial dynamic run isolated from root full gates and never replace the test runner or host executable.


### Local dynamic probe result — 2026-10-06

**Passed** on Apple Silicon macOS using pinned official 0.5.0 and 0.5.1 shell installers and ARM archives. Temporary orchestration script `${LOCAL_EVIDENCE}/compact_adr230_probe.py` SHA256 `4125d3a4d3b5722c906a0a42427adf249506b1700d49e8c9fe89184e37df48c8`; full receipt `${LOCAL_EVIDENCE}/compact-adr230-dynamic-receipt.json` SHA256 `bea8b1b925a11763efa712209044ce41d7d36809a0ae6a79746cdb1296a0a76b`; transcript `${LOCAL_EVIDENCE}/compact-adr230-dynamic-transcript.txt` SHA256 `64b722ad38b343aa341152111ec77309b2e87dfbc7374e3e9fd76ddf727a1fdd`. No repository files changed for this probe.

- All ten retained assets matched acquisition byte counts and SHA256. Published `.sha256`, `sha256.sum`, and `dist-manifest.json` agree with each archive hash; both tar member lists were inspected.
- The unchanged 0.5.0 installer downloaded the old ARM archive over localhost, installed the private binary with exact archive SHA256 `13a9cab10022ecd8c0dcf8bf9c9e3aad1919d5795e85b49d4c2cf131b7525678`, reported `compact 0.5.0`, and wrote a genuine cargo-dist receipt for `midnightntwrk/compact`.
- That private binary's `self check` reported `Update available -- 0.5.1`. Its `self update` fetched the 0.5.1 installer and archive, reported `Update installed -- 0.5.1`, and replaced the same private executable path with exact archived SHA256 `61014fa83633e518cc8e652a7706417425b72fb1db6c26504183e37c8fce407c`. The updated binary reported `compact 0.5.1`; second check reported `Up to date`.
- Receipt source owner/name/provider and private install prefix were preserved; only version changed. The private file manifest contains exactly `install/compact` and `config/compact/compact-receipt.json`. The localhost route ledger records old archive, metadata, new installer, new archive. Refusing proxy observed zero external attempts. Host compact binary and user receipt SHA256 were identical before and after.

This proves one genuine historical ARM self-update path with pinned local transport. GitHub release metadata was simulated by a closed localhost route; official scripts and binaries were unchanged. It does not establish public-network availability, publisher signatures, other platforms, or a future release pair.


### Accepted follow-up: reusable test-only fixture (before source edits)

The pinned ARM probe above passed, so ADR230 now authorizes a checked-in **test-only** regression fixture on the existing `test_self_scenarios` surface. It will replace live `curl | sh` in those scenarios with hash-pinned local release assets. It will not edit the Compact CLI, axoupdater, compiler, runtime, dependencies, release sources or production endpoints. Issue: https://github.com/MediaNoxLabs/compact/issues/333.

- A small asset manifest records exact official GitHub release tag, filename, byte count and SHA256 for both `compact-installer.sh` scripts and 0.5.0/0.5.1 archives for macOS arm64, macOS x86_64 and Linux x86_64 musl (the configured workflow matrix). The acquisition helper downloads only these versioned official URLs into a private cache, refuses size/hash mismatches, and never selects a moving latest release. Tests refuse absent/unverified assets. ARM is the only dynamically proven platform so far; the other two platform pins are source/metadata evidence until their runners execute.
- A new `tests/common/self_update_fixture.rs` runs the unchanged old installer under private HOME/XDG/config/install/temp, serves validated scripts and archives from a closed loopback route and exact `midnightntwrk/compact` latest metadata route, and refuses/records external HTTP(S) proxy attempts. It strips inherited credentials, installer overrides, updater working-directory selection and host install paths. `COMPACT_DOWNLOAD_URL` is switched from the 0.5.0 to the 0.5.1 loopback route before update. Each child is time bounded. No fake binary, receipt or output is synthesized.
- `test_self_scenarios.rs` retains the existing check and both platform-specific update expectations. It additionally asserts old and new exact binary hashes, real receipt source/provider/version/prefix, request route order, unchanged private path, and final `--version`/Up to date. It retains explicit selection of the downloaded binary, rather than the current Cargo build. A dedicated workflow step must acquire the platform-pinned assets before `cargo nextest`; the workflow owner will compose that step separately.
- Validation is focused ARM run, missing/corrupt asset refusal, strict Clippy, and all existing self scenarios. Preserve a concise receipt for the test fixture and distinguish the already passed ARM run from not-yet-executed x86_64/Linux matrix rows. No remote CI or push.


#### Platform checksum-tool observation

The existing Intel macOS update snapshot explicitly expects cargo-dist to report that `sha256sum` is unavailable. The reusable fixture sets that child PATH to real `/usr/bin:/bin`, excluding newer macOS `/sbin/sha256sum`, so the historical missing-tool branch stays deterministic without a fake executable or changed installer. Other platforms receive real system tool directories; every archive is independently SHA256-verified by the acquisition helper and Rust fixture before invocation. If a platform places a real `sha256sum` in the selected PATH, the exact original output assertion will expose that drift.


#### Implementation amendment to the original probe plan

The separate-ADR/issue condition above applied before the first dynamic probe. After the probe passed, the parent approved this reusable fixture as a continuation within ADR-0230 and issue #333, with the accepted follow-up recorded before repository edits. The checked-in server exposes only the exact routes actually used by the installer/updater (old archive, latest metadata, new installer, new archive); `.sha256`, `sha256.sum`, and `dist-manifest.json` were independently validated during pin preparation and need no HTTP route. The fixture uses one closed server as both release endpoint and refusing proxy: any external `CONNECT` or unknown path is recorded and refused, then fails the route assertion. This preserves the original isolation policy with fewer unused routes.


### Signed implementation receipt — 2026-10-06

Signed/DCO conventional commit `30b334d5f40b4841d55883ff33d2d2aa7b6578f7` on `codex/adr230-historical-self-update` (base `126f8f8e`); no push. Exact-head ARM focused test: 3 passed, 0 failed (original check, original update, new missing/same-size-corrupt guard); strict focused Clippy passed. The acquisition helper also refused an oversized response and dangling-symlink cache entry and cleaned its unique partial file; four-asset fresh acquisition and cache revalidation passed. Cross-platform static pin receipt `${LOCAL_EVIDENCE}/compact-adr230-platform-pins.json` validates all eight manifest entries against GitHub release size/digest and cargo-dist archive digests; four non-ARM archives were downloaded, hashed and inspected for exact executable member paths but not executed. Independent agent review found no actionable issue. Full handoff `${LOCAL_EVIDENCE}/compact-adr230-delivery/receipt.json` SHA256 `f0f8ada220517e3179b48143519bad6f759650f8f9a6cc002c3cbaa54d64f9df`. Root owns the mandatory CI workflow acquisition hook and combined package gate; this commit alone does not claim either.

### Combined local installer acceptance passed — 157d7033 (2026-10-06)

Frozen root **157d703312d8b37a097379c9ae437f4e4694cf17** passes the complete combined installer selection: **165/165 tests, zero skipped, 45.089 seconds**, followed by strict all-target/all-feature package Clippy (1.59 seconds) and **17 Python guards**. Immutable run receipt: `${LOCAL_EVIDENCE}/ci-0u4_twgg/receipt.json`; locator: `${LOCAL_EVIDENCE}/compact-current-installer-gate.json`. Root's audit covers **256 conventional, GPG-signed, DCO commits**; the user-owned ledger document remains unchanged.

ADR230 / [#333](https://github.com/MediaNoxLabs/compact/issues/333) source `30b334d5f40b4841d55883ff33d2d2aa7b6578f7` is integrated as **c90188a9**, with its mandatory workflow acquisition hook in **157d7033**. This combines private no-install baselines, genuine pinned compiler archives, existing update/format/compile/clean scenarios with real key generation, and genuine historical 0.5.0 → 0.5.1 self-update through closed local transport. Original assertions and platform guards remain. The full nextest selection stays enabled; setup verifies cache hits and retains provenance.

The **first attempt is retained**, not relabeled as success: `${LOCAL_EVIDENCE}/compact-installer-157d7033-ipt5yb3i/receipt.json` records 155/165 passing, with ten help snapshot mismatches caused by the long outer isolated HOME path changing Clap's default-path wrapping. All installation, key-generation and self-update cases passed that attempt. Root changed only the driver's private HOME to the short `${LOCAL_EVIDENCE}/ci-0u4_twgg/home` layout; repository source and assertions were unchanged. The successful rerun validates the recorded short HOME layout, not universal help formatting for every arbitrary path length.

Dynamic installer evidence is **ARM macOS**. Intel macOS/Linux compiler archives and parameter setup, plus the historical self-update assets, have static validation; their scenario execution remains a separate platform gate. Compiler 0.22 preserves archive-local `ZKIR_PP` and bundled `kzg`/`kzg.vp`; 0.24 uses Filecoin k=10, while 0.30/0.31 use Midnight k=5. No managed Filecoin k=9 substitute is used. The actual release scripts/binaries are unchanged, but public-network availability and future release pairs are not claimed.

These slices change tests/workflow only: no compiler/runtime, schema 20, ABI 49, production installer or dependency changes. The completed **d5dd3f2e product/full/Nix gates** and separately identified **actual live lifecycle heads** remain their own evidence; this installer pass is not a repeat or relabeling of those runs. Root has finalized the local evidence bundle with **136 hash-covered files**, manifest `local_closeout_head=157d7033`, the 165-test combined gate and final-receipt-bound scope audit. No established local gate remains open in this review. Root has requested explicit approval for branch push and remote CI; both remain unperformed pending that reply. Remote branch publication, exact-revision remote CI/remote Nix consumption and additional-platform execution remain deferred. Crates.io/public-registry publication remains outside approved v2 scope. No push or remote CI occurred. Earlier pending-integration/gate wording below is historical and superseded by this result.
