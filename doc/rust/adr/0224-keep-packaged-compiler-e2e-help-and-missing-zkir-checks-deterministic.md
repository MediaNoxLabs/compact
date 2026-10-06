---
id: RUST-ADR-0224
alias: ADR-0224
title: "Keep packaged compiler E2E help and missing-ZKIR checks deterministic"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-test-only"
topics: ["E2E", "compiler", "packaging"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: e63a09428af2987766f30663377f3fadd90eea8b740c0ec6c7b967e4073a1306
---
# RUST-ADR-0224 — Keep packaged compiler E2E help and missing-ZKIR checks deterministic

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-test-only. The packaged E2E suite uses an isolated real compiler and controlled tool path to test missing ZKIR warning, exit success and artifact/no-key behavior. No backend semantics change or skipped gate is part of this decision.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#328 closure](https://github.com/MediaNoxLabs/compact/issues/328#issuecomment-6017786253). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`d5dd3f2e`](https://github.com/MediaNoxLabs/compact/commit/d5dd3f2eeeaad337ba40fcf2639d540a9738f161). These include historical prerequisites and probes, not only final delivery commits.

## Historical decision and amendments

Status: Accepted for local implementation; milestone rust-backend-v2.

### Problem and observed baseline

At frozen `d5dd3f2eeeaad337ba40fcf2639d540a9738f161`, the unchanged default `tests-e2e` suite against packaged compiler `${HISTORICAL_NIX_STORE}/88b4i0iv4l4f0wvfv42lbvy4mh0qd620-compactc-binary-dist` has 488 passes, 6 skips and 3 failures (50 test files). Two help checks compare an outdated manual-page fixture: it says `stateful circuits` where the compiler now correctly says `proof-required exported circuits`. The third test tries to simulate absent ZKIR by clearing inherited environment variables. The portable `compactc` launcher always prepends its bundled `../lib` to PATH, so the bundled `zkir` is found; with HOME cleared it errors while finding a cache directory. The test never reaches the intended missing-tool warning.

Baseline receipt: `${LOCAL_EVIDENCE}/compact-final-e2e-d5dd-receipt.json`, log `${LOCAL_EVIDENCE}/compact-final-e2e-d5dd.log` (SHA256 `0874700587e60173c3a48c5ff448639409c6cc0fb0d9c789e957cb9326b967a7`). These are local macOS results, not remote CI.

### Decision and before/after

Refresh only the one changed manual-page phrase in `tests-e2e/src/resources/compiler_man_page.txt`.

Before the missing-tool test:

```ts
await execa(getCompactcBinary(), [source, output], {
  env: { COMPACT_HOME: 'non_existing_path' },
  extendEnv: false,
  reject: false,
});
// The launcher restores bundled zkir to PATH; the test sees a missing HOME error.
```

After:

```ts
const compiler = getCompactcBinary();
const portableNative = path.join(path.dirname(compiler), 'compactc.bin');
const nativeCompiler = fs.existsSync(portableNative) ? portableNative : compiler;
const isolatedTools = createTempFolder();
fs.symlinkSync(resolvedSha256sum, path.join(isolatedTools, 'sha256sum'));
await execa(nativeCompiler, [source, output], {
  env: { COMPACT_HOME: 'non_existing_path', PATH: isolatedTools },
  extendEnv: false,
  reject: false,
});
// The actual compiler runs without zkir; contract TS/ZKIR is emitted, no keys.
```

Resolve `sha256sum` from the invoking PATH and symlink only that program into the private tool directory: compiler manifest emission requires it. For the portable distribution, use its adjacent `compactc.bin` to bypass the launcher. The normal Nix compiler package exposes native `compactc` with no adjacent `.bin`, so retain that entrypoint under the same isolated PATH. Require `sha256sum` explicitly and fail clearly if it is unavailable. If a future portable launcher lacks the sibling native binary, the warning assertion will fail instead of silently skipping. Do not edit installed binaries, package resources, process HOME, or the user's global PATH. Keep the existing exact stdout, warning and success assertions; also assert that the generated contract and ZKIR exist while keys do not. This tests the real missing-ZKIR success path instead of skipping or changing the expected result.

### Emitter/runtime boundary

No compiler passes, Rust emitter, runtime, schema, ABI, proof code, or generated fixture semantics change. This is an end-to-end fixture/test setup repair. The manual-page fixture follows the current compiler help contract. A packaged launcher still correctly exposes its bundled tools in normal runs; only the test's controlled native entrypoint removes them.

### Validation and limits

- Focused `compiler.smoke.e2e.test.ts` with the packaged compiler and bounded workers, then one full default `yarn test` run with the same packaged compiler.
- Prove exact warning plus exit 0, generated `contract/index.js` and `.zkir`, and absence of `keys/` in the isolated missing-tool test.
- Record test counts, source head, package identity, command, complete log path and clean worktree; sign a conventional DCO+GPG commit.
- Do not modify the dependency lock, remote CI, services, or root checkout. Other e2e cases still execute the normal launcher.

The test must run with both current package layouts: portable launcher plus sibling `compactc.bin`, and normal Nix native `compactc` with no sibling `.bin`. A changed portable layout that reinjects ZKIR should fail the warning assertion, not skip. The source-level output remains the authoritative check. This local suite does not establish complete original-contract proof or wallet/node acceptance.


### Signed local delivery — 2026-10-06

Issue: https://github.com/MediaNoxLabs/compact/issues/328, milestone `rust-backend-v2`. Signed/DCO conventional commit `a7f9deb7ba9e6a874b05b5aca79cf0b5e49d9a5f`; GPG key `68177A52E5E37FDCF31DAEF04BEFC05538080D6E` verified good. The tracked checkout is clean, and no push or remote CI was used.

The exact help fixture now matches the current `proof-required exported circuits` wording. The one missing-tool test resolves the installed compiler path; portable packages invoke adjacent native `compactc.bin`, whereas a normal Nix compiler package invokes its native `compactc`. It resolves executable `sha256sum` from the inherited PATH, symlinks only that utility into a private temporary PATH, and removes the directory in `finally` without following its read-only symlink target. The test still requires exit 0, the exact warning and stdout, emitted TypeScript/ZKIR, and no keys. All other tests use the normal packaged launcher.

Checks: portable focused 20/20; native Nix layout focused 20/20 with a pinned ZKIR available in the parent PATH; portable full default suite 50/50 files, 491 passed and 6 skipped; portable focused exact signed HEAD 20/20. Complete immutable log hashes, compiler hashes and commands: `${LOCAL_EVIDENCE}/compact-adr224-delivery-receipt.json`. The initial three failures remain in `${LOCAL_EVIDENCE}/compact-final-e2e-d5dd.log`. An optional `yarn build` outside the E2E workflow reports existing TS6 `tsconfig` deprecation/rootDir errors; it is separate from this passing E2E gate and was not changed.
