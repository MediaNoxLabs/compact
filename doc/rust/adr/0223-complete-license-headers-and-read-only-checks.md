---
id: RUST-ADR-0223
alias: ADR-0223
title: "Complete license headers and read-only checks"
date: not-recorded
publication_date: 2026-10-07
decision_status: "accepted-test-only"
topics: ["license", "headers", "tooling"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 68f27cf49a976cff218f519fe47397c521e8bf0f967c8314c15b80757c3e1caf
---
# RUST-ADR-0223 — Complete license headers and read-only checks

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-test-only. The audited header repair completes licenses in exactly 133 tracked files and refreshes 15 affected manifest digests; a read-only help/flag regression protects source bytes. This is compliance/tooling maintenance, not compiler behavior.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#327 closure](https://github.com/MediaNoxLabs/compact/issues/327#issuecomment-6017784383). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`cc333811`](https://github.com/MediaNoxLabs/compact/commit/cc333811fa8dc3566c59eeccd00b5a7cb71193b5). These include historical prerequisites and probes, not only final delivery commits.

## Historical decision and amendments

Status: Accepted for implementation; local-only delivery.

### Problem

At frozen `cc333811fa8dc3566c59eeccd00b5a7cb71193b5`, `python3 add_headers.py --validate` checks 1,912 files and refuses 133 tracked files whose abbreviated headers lack the complete Apache notice. The misses span 12 compiler modules, 3 runtime modules, 26 hand-maintained fixture Cargo manifests, 83 tests/captures/proof modules and 9 other files. Four reviewed behavior manifests pin 15 hashes of affected files.

The command currently recognizes only a first argument of `--validate`; `--help` or an unknown option falls through to the modifying add operation. Asking for help must not modify a checkout.

### Decision and before/after

Repair precisely the audited 133 files by completing their existing license comment block. Preserve shebangs and every executable/non-header byte. Update only the 15 affected digest values in the four reviewed manifests; keep independent TS captures and behavioral assertions unchanged.

Before:

```text
python3 add_headers.py --help       # accidentally adds headers
python3 add_headers.py --typo       # accidentally adds headers
python3 add_headers.py --validate   # exits 1 with 133 misses
```

After:

```text
python3 add_headers.py --help       # prints usage, exits 0, no tree changes
python3 add_headers.py --typo       # rejects option, exits 2, no tree changes
python3 add_headers.py --validate   # checks only, exits 0 after repair
python3 add_headers.py              # retains explicit existing default add behavior
```

Use argparse before constructing/scanning the header manager. Preserve default add and `--validate`; do not introduce new mutation modes or change file coverage.

### Emitter/runtime and fixture boundaries

Compiler and runtime changes are comments only: no lowering, behavior, schema20 or ABI49 change. The 26 Cargo.toml files are fixture package wrappers, not generated contract output. Review the freshness/update path to establish it writes generated lib.rs only and does not replace wrapper manifests. Do not edit generated libraries or regenerate proof keys.

The three affected example Compact sources and wallet acceptance source retain their non-comment bytes. New source/build identities must be recorded honestly; existing live-run and historical receipts remain attached to their original source heads.

### Evidence and acceptance

- Exact input audit: `${LOCAL_EVIDENCE}/compact-header-audit.json` and `.md`.
- Full header validation after repair.
- Existing oracle identity/matrix checker after the 15 digest replacements.
- Temporary-directory subprocess regression proving `--help` and unknown flags preserve byte-identical files and do not require a valid configuration. Verify default add, idempotence and read-only validation.
- Scoped diff and comment-only verification for all 133 paths; no captured JSON changes except the reviewed digest manifests.
- No full Cargo/proof suite; no root checkout or user-document changes. Conventional GPG+DCO delivery.

### Alternatives and limits

Skipping the header gate or excluding new files would conceal the accumulated misses. A blanket add run can duplicate abbreviated headers and modify unrelated files, so use the exact audit list and preserve the remainder of each file. This is compliance tooling and metadata maintenance, not new behavioral or proof coverage.


### Signed local delivery

Commit `ae0e4d0abffbc7f9a1a47177cc176fbd33d0da44` is GPG verified, DCO signed and clean, based on frozen `cc333811fa8dc3566c59eeccd00b5a7cb71193b5`. The change completes exactly 133 tracked headers and refreshes only 15 affected digest values in four reviewed manifests. Every byte outside the replaced/inserted license span is identical for those 133 files. Captured TS JSON, assertions, generated libraries and the user document are unchanged.

The CLI now handles help before loading configuration or scanning. Unknown options, abbreviated options and unexpected positionals exit 2. Default add and read-only validation remain intact. Three temporary-directory tests cover these behaviors and idempotence.

Exact signed-head checks: header gate checks 1,913 files with zero misses; 37-source identity/matrix checker passes; all three CLI tests pass. Scoped review confirms 139 changed paths. `check_fixture_outputs.py --update` writes only generated `lib.rs`, so the 26 repaired wrapper Cargo manifests are stable without changing generator behavior. No Cargo/proof suite was run.

Receipt: `${LOCAL_EVIDENCE}/compact-adr223-delivery-receipt.json`. Detailed exact header-only and digest review: `${LOCAL_EVIDENCE}/compact-adr223-header-only-proof.json` and `${LOCAL_EVIDENCE}/compact-adr223-semantic-review.json`. Historical and live-run receipts remain tied to their original source heads; this header repair does not relabel them.
