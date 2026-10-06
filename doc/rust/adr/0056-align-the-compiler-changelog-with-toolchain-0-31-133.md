---
id: RUST-ADR-0056
alias: ADR-0056
title: "Align the compiler changelog with toolchain 0.31.133"
date: 2026-10-04
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["distribution", "provenance", "validation"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 8125ea42b30cbbf24fa7deb0ef6da01385fe8b8f608bed03a7f7331b3eb632ed
---
# RUST-ADR-0056 — Align the compiler changelog with toolchain 0.31.133

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept changelog entries aligned to the actual compiler/language/runtime version files while preserving each version transition. Preserve the separate successful compiler/source-map runs and the failed archive shell setup explanation. This is provenance/documentation maintenance, not a language or runtime change.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#155 closure](https://github.com/MediaNoxLabs/compact/issues/155#issuecomment-6017493215). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`09c52a7d`](https://github.com/MediaNoxLabs/compact/commit/09c52a7dfd0f122313b57e1134aefe852d38a2e8) · [`1e0216c6`](https://github.com/MediaNoxLabs/compact/commit/1e0216c67835cbf7e8b160099a4856c3f8c7fbfd) · [`59dcb932`](https://github.com/MediaNoxLabs/compact/commit/59dcb93235cfd124cb7d8a47fd69f83e189eb1f9) · [`6925beaf`](https://github.com/MediaNoxLabs/compact/commit/6925beaf7bc9e97c67dd8f86e211a8e3f32e1097) · [`6b3b9685`](https://github.com/MediaNoxLabs/compact/commit/6b3b9685be3759c8f799b928c35e7d75e3c6e4ff) · [`f155ae78`](https://github.com/MediaNoxLabs/compact/commit/f155ae7834bfcc9acec31648019f8e4eef45f7e2). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 56
status: accepted-partial
date: 2026-10-04
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/155
```

## Historical decision and amendments

### Problem and exact CI repro

At committed `59dcb932`, `flake.nix`, `compiler/compiler-version.ss`, and generated `doc/ledger-adt.mdx` identify compiler/toolchain 0.31.133, while the top `CHANGELOG.md` entry is 0.31.130. `.github/workflows/build-compiler.yml` compares the top changelog toolchain/language/runtime triple with the source version files before compiler build and exits 1 on this mismatch. The clean Git archive source reproduces it. The three version bumps were real commits, but their changelog entries were omitted.

### Before and after

Before:

```text
## [Toolchain 0.31.130, language 0.23.105, runtime 0.16.101]
```

After, the top three entries should be 0.31.133, 0.31.132, and 0.31.131 with accurate changes from their original signed commits, followed by the existing 0.31.130 entry. At 0.31.131, checked Compact enum codec derivation and malformed-tag rejection; at 0.31.132, ledger declaration source locations in private Rust IR/diagnostics; at 0.31.133, circuit/witness/constructor/alias declaration locations and source-located renderer diagnostics. Language 0.23.105 and runtime 0.16.101 remain unchanged. Do not alter version numbers to make the gate pass; repair the missing history.

### Ownership and alternatives

The changelog owns human-readable release history and the workflow owns its exact version gate. Emitter, generated crate, Rust runtime, macro, ledger-8/zk pins, ABI 28 and private IR schema 8 do not change in this correction. Changing the workflow to ignore stale release notes would lose a useful provenance check; collapsing three version bumps into one 0.31.133 entry would erase their original sequence. The 0.31.131–133 entries retain those boundaries.

### Verification and limits

1. Check the exact `flake.nix`/compiler/language/runtime/doc/changelog versions in a clean committed source snapshot; the workflow's top-entry extraction must match 0.31.133/0.23.105/0.16.101.
2. Verify 0.31.131–133 descriptions against commits `6b3b9685`, `6925beaf`, `1e0216c6`, and leave prior entries unmodified.
3. Run the scoped documentation/header/diff checks. The in-flight clean Cargo workspace run is at an earlier commit; do not attribute it to this documentation-only commit. Remote full Compiler Build CI remains the revised milestone exit.
4. Record conventional GPG/DCO commit, exact evidence and limits in this ADR, a focused MediaNoxLabs issue, the ADR register and M2 delivery map. The branch remains local pending the user's publication answer.

### History

- 2026-10-04: proposed after exact committed source showed a deterministic pre-build CI mismatch: 0.31.130 changelog versus 0.31.133 compiler/flake/doc. The three version-bump commits were inspected before editing.


- 2026-10-04: focused [MediaNoxLabs/compact#155](https://github.com/MediaNoxLabs/compact/issues/155) created in rust-backend-v2 before implementation.


### Local delivery — 2026-10-04

Conventional GPG-signed/DCO commit `09c52a7dfd0f122313b57e1134aefe852d38a2e8` adds 21 lines to `CHANGELOG.md` only: distinct 0.31.131, 0.31.132 and 0.31.133 entries sourced from signed version-bump commits `6b3b9685`, `6925beaf` and `1e0216c6`. The preexisting 0.31.130 and earlier entries are unchanged. No executable source, emitter, runtime, macro, generated crate, ABI 28, schema 8, dependency pin or version file changes. The development checkout's scoped diff and 1,520-file header checks pass.

An exact `git archive 09c52a7dfd0f122313b57e1134aefe852d38a2e8` source snapshot independently passes the workflow-equivalent version comparison: compiler/flake/doc 0.31.133, language/doc 0.23.105, runtime 0.16.101, top changelog matching all three, and ordered 0.31.133/132/131/130 entries. The same clean committed tree passes `add_headers.py --validate` with zero missing. The separate full Cargo workspace test is still running at earlier `f155ae78`; it is not an exact-head `09c52a7d` claim. The complete remote Compiler Build suite remains the final milestone gate, and branch publication awaits the user's answer to the original local-only instruction.



### Exact-source compiler gate follow-up — 2026-10-04

The disposable `git archive` of commit `09c52a7d` passed the `Compiler Build` workflow's exact `nix develop --command ./compiler/go` command with exit 0. Both feature-zkir-v3 modes passed 635/635 `save-manifest` cases and 1/1 `run-javascript` invocation; each Vitest invocation reported 6 files, 1,482 passing tests and no type errors. Chez coverage was 79% (61,488/77,983). The resulting `doc/compact-reference.mdx` and `compiler/compact-reference-proto.mdx` are byte-identical to the pristine committed archive, and `coverage/` exists. This validates the compiler test/document-consistency portion locally at the exact changelog-fix commit; it is not a complete remote workflow run. The same archive separately passed source maps (six tests, 132 assertions). The clean Rust workspace suite at older `f155ae78` remains running.



#### Default-shell source-map reproduction — 2026-10-04

A second fresh `git archive` of `09c52a7d` passed the workflow's exact `nix develop --command ./srcMaps/test.sh` invocation with exit 0: six VLQ tests, 132 assertions, zero failures. An attempted repeat in the first non-Git archive failed during Nix environment construction before tests, because `compiler/go` had caused the shell hook to populate `test-center/node_modules` and Nix then included that untracked directory as source when reevaluating the archive path. The fresh archive reproduced a clean CI checkout and passed. No source-map or compiler code change follows from that local harness artifact. Remote workflow evidence remains pending.
