---
id: RUST-ADR-0055
alias: ADR-0055
title: "Make wallet JavaScript pass the repository license gate"
date: 2026-10-04
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["distribution", "provenance", "validation"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 29e43a535307e33a7b4ad92246a47b7fbeccd47071d7628c0efe88150a18fce5
---
# RUST-ADR-0055 — Make wallet JavaScript pass the repository license gate

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept applying the repository license template to the wallet JavaScript files rather than weakening header validation. Clean-source script and handoff checks establish this bounded repair; no compiler/runtime behavior changes. Historical pending CI language remains dated evidence, with final closure recorded separately.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#154 closure](https://github.com/MediaNoxLabs/compact/issues/154#issuecomment-6017491389). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`59dcb932`](https://github.com/MediaNoxLabs/compact/commit/59dcb93235cfd124cb7d8a47fd69f83e189eb1f9) · [`f155ae78`](https://github.com/MediaNoxLabs/compact/commit/f155ae7834bfcc9acec31648019f8e4eef45f7e2). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 55
status: accepted-partial
date: 2026-10-04
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/154
```

## Historical decision and amendments

### Problem and exact CI repro

At clean signed rc7/`f155ae78`, the `Compiler Build` workflow's first `license-check` job runs `python3 add_headers.py --validate` and exits 1. It reports four Rust-initiative JavaScript files: `check_wallet_handoff.mjs`, `wallet-live/provenance.mjs`, `wallet-live/provenance.test.mjs`, and `wallet-live/check.mjs`. Each starts with a three-line SPDX header, while `header_config.json` requires the complete Apache-2.0 notice. The CI test is authoritative and would block all downstream compiler jobs before proof checks.

### Before and after

Before, the files start with:

```javascript
// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
```

After, keep those lines and add the repository's exact full Apache-2.0 notice, including license URL, warranty disclaimer and permissions text, as already used by neighboring oracle `.mjs` files. Do not add a second competing header or change executable JavaScript. The generated Rust API, emitter, runtime, ABI 28, private IR schema 8 and wallet byte format are unchanged.

### Alternatives and ownership

Changing `header_config.json` to accept abbreviated notices would weaken a repository-wide CI policy for one folder. Excluding wallet scripts from the scan would hide shipping integration code from the license gate. The wallet source files own this correction; the checker and its template remain unchanged. No new ledger-8 or midnight-zk primitive is needed.

### Verification and limits

1. Run `python3 add_headers.py --validate` in a clean checkout at the resulting commit; expect zero missing files.
2. Run `node --check` on all four scripts and `node --test wallet-live/provenance.test.mjs`.
3. Recheck sealed rc7 deploy/call bytes through `check_wallet_handoff.mjs` with ledger-v8 8.0.3. Confirm the full Rust workspace test that was already running at rc7 exits successfully; distinguish its old-head evidence from the new header-only commit.
4. Record conventional signed/DCO commit, exact tests, and unchanged emitter/runtime ABI here, focused issue, ADR register and M2 map. Remote CI remains the revised milestone exit gate. Branch publication follows the user's answer to the local-only versus CI question.

### History

- 2026-10-04: proposed after exact clean rc7 `add_headers.py --validate` failed with four named files. The successful rc7 proof/package/wallet gate does not establish the separate license job.


- 2026-10-04: focused [MediaNoxLabs/compact#154](https://github.com/MediaNoxLabs/compact/issues/154) created in rust-backend-v2 before implementation.


### Local delivery — 2026-10-04

Conventional GPG-signed/DCO commit `59dcb93235cfd124cb7d8a47fd69f83e189eb1f9` adds the repository's full Apache-2.0 notice to exactly the four wallet `.mjs` files; 44 comment-only lines, no executable JavaScript, emitter, runtime, macro, generated crate, ABI or schema change. In the development checkout `python3 add_headers.py --validate` now checks 1,520 files with zero missing headers (previously four). `node --check` passes for all four scripts, `node --test wallet-live/provenance.test.mjs` passes four cases, and the modified `check_wallet_handoff.mjs` decodes the sealed rc7 deploy/call bytes with ledger-v8 8.0.3 (1,765/3,372 bytes). Scoped diff check passes. The separately running clean-worktree 142-member Cargo workspace test is at previous commit `f155ae78`; its outcome will be recorded separately and must not be described as a `59dcb932` exact-head run. Only the preexisting user-owned `doc/ledger-adt.mdx` edit remains unstaged in the development checkout.

The clean exact-head license gate and remote CI are still pending. The branch is local until the user answers the specific publication question.


### Exact-commit clean-source verification — 2026-10-04

Without touching the separate Cargo checkout, `git archive 59dcb93235cfd124cb7d8a47fd69f83e189eb1f9` was extracted into disposable `target/ci-license-head-59dcb932`. From that exact committed source, `python3 add_headers.py --validate` exits 0: 1,520 files checked, zero missing. All four `node --check` commands and the four provenance unit tests pass from the archive. The archived `check_wallet_handoff.mjs` decodes the prior rc7 sealed deploy/call bytes with pinned ledger-v8 8.0.3 (1,765/3,372 bytes). This proves the header-only commit's clean-source license and wallet-script behavior. It does not replace the still-running 142-member workspace test at `f155ae78` or the unpublished remote same-commit CI requirement.
