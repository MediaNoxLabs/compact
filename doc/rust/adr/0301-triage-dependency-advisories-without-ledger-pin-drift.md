---
id: RUST-ADR-0301
alias: ADR-0301
source_sha256: 105dd01d0e60a6cf903be68df7c6d8a1fce089c93da58ba09c5e88d91d7287f8
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0301 — Triage dependency advisories without ledger pin drift

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted for read-only triage and isolated compatible-update rehearsal. Parents R030-13/#357, R030-19/#363 and R030-20/#364. No product dependency changes approved by this first decision. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0301 — Triage dependency advisories without ledger pin drift

Status: accepted for read-only triage and isolated compatible-update rehearsal. Parents R030-13/#357, R030-19/#363 and R030-20/#364. No product dependency changes approved by this first decision.

### Problem and observed baseline

At signed ca97a69f, Cargo.lock SHA2563f4c30f69b917c1ee2344a795219c841753ca1aac1347a3b0323c1af71f8532a. All-workspace/all-feature/all-platform metadata resolves597 external registry packages. A normal/build-edge reachability inventory retains shortest dependency paths per owned root; these are potential package paths, not executed-code or target-enabled-feature proof.

The installed cargo-audit0.20.1 cannot parse current RustSec CVSS4 entries and produced no audit report. Do not interpret that failure as a clean scan. An isolated local install of cargo-audit0.22.2 successfully scanned the same lock against advisory-db commit ef6173cbc5c50ec8166f9a5b28f07834144373ee (1290 advisories). It reported8 vulnerability matches,4 unsound notices,3 unmaintained notices and1 yanked package. Preserve raw reports and database/tool identities; no ignore list or database editing.

Vulnerability matches: crossbeam-epoch0.9.18, h20.4.13, quick-xml0.37.5 and0.38.4 (two advisories each), quinn-proto0.11.14, rustls0.23.37. Unsound: anyhow1.0.102, lru0.16.4, memmap20.9.10, rand0.8.5. Unmaintained: bincode2.0.1, number_prefix0.4.0, paste1.0.15. Yanked: unicode-segmentation1.13.1. Distinguish database matches from confirmed reachable exploit conditions.

### Before / after decision model

Before:
```text
lockfile match -> broad upgrade or blanket ignore
```
Accepted workflow:
```text
exact advisory + locked identity
 -> direct parents + target/features + used API/preconditions
 -> compatible isolated patch trial, or explicit residual finding
 -> required local behavior/MSRV/graph gates
 -> reviewed minimal live patch
```

Compatible patch candidates from the observed advisory ranges include crossbeam-epoch>=0.9.20, h2>=0.4.16, quinn-proto>=0.11.15, rustls>=0.23.45, anyhow>=1.0.103, memmap2>=0.9.11 and rand0.8.6. Resolve actual available versions and declared dependency constraints before selecting exact pins. A patch classification does not bypass the declared Rust1.88 MSRV or required behavior gates.

quick-xml fixes require>=0.41 and lru>=0.18.2; their existing parents currently constrain older minor versions. Do not blindly widen upstream manifests, vendor/fork ledger crates, promote native ledger8.1, or substitute implementations just to obtain a clean scanner result. First establish actual component/target/call-site exposure and the smallest coherent option. Any incompatible dependency/API change needs a separately reviewed design and corresponding tests. Unmaintained notices require a recorded ownership/disposition; they are not automatically equivalent to exploitable vulnerabilities. No blanket ignore is accepted.

### Scope and ownership

First use an isolated manifest/lock rehearsal, preserving exact ledger8.0.3/zk graph identities and all unchanged package identities. Build no giant new ledger target. Keep compiler/runtime ABI50, private IR20, original DID/passport sources and all independent oracle values unchanged. Root owns production lock changes after reviewing the proposed package delta and gate scope.

Record package name/version/source/checksum, database ID/title/affected range/patch range, relevant paths, actual usage evidence, disposition, residual risk and required verification. All-target metadata is conservative; confirm cfg/feature boundaries before calling a package shipped or runtime reachable. Absence from normal/build paths is not absence from developer/test tools.

License inventory is related release evidence: retain declared expressions and source license-file hashes. Two packages use license_file instead of license metadata: midnight-circuits6.0.0 and midnight-zk-stdlib1.0.0 both contain Apache2.0 header text. Four MPL2.0 declarations and combined-license expressions need explicit distribution notice/source handling at final packaging. This is engineering provenance, not a legal conclusion from metadata alone.

### Acceptance and continuation

The first deliverable is a complete triage matrix and exact isolated lock delta proposal, with strict scan results (including unsound warnings), supported target/features and MSRV constraints. Do not hide remaining findings or close the release audit from a narrow scan. Then implement approved compatible remediation and run its scoped unit/consumer tests; expand to runtime/generated/proof qualification when affected semantics require it. Recheck final release lock and final artifact/native dependency closure at RC.

Planning and evidence stay in midnight until closeout. No CI, push or product dependency changes in this first step. The independent external coding-agent audit remains required and separate.

### Primary tool/database references

- https://github.com/rustsec/rustsec/tree/main/cargo-audit
- https://github.com/RustSec/advisory-db
- https://docs.rs/crate/cargo-audit/0.22.2

Issue: https://github.com/MediaNoxLabs/compact/issues/425

### Accepted local TLS qualification — 2026-10-07

The installer dependency graph changed after removing the unused nextest library. HTTP status mocks alone do not qualify its Rustls transport. Root approved a scratch-only local HTTPS fixture using the existing locked reqwest dependency and an explicit Rustls client. Before: archive/update behavior and proof gates passed, but no TLS trust or hostname control was executed. After: require default-trust refusal of a self-signed localhost certificate, explicit fixture-root acceptance of a known local body, and hostname-mismatch refusal with that same root.

Use a temporary OpenSSL SAN-localhost server bound only to loopback. No new dependencies, global trust edits, invalid-certificate bypass, or private-key archival. Retain the test source, public certificate fingerprint, server/tool identities and exact result logs. The scratch test is evidence only and is excluded from the eight-path candidate production patch. This is a bounded transport regression check, not a general TLS security audit.

### Root acceptance for live port — 2026-10-07

Root independently reviewed the eight-path patch, all normalized graph/feature deltas, scoped gates and residual dispositions; verified all 3236 artifact hashes, eight baseline/candidate identities and durable archive integrity. Receipt 0e1652fd9b1f198112e0211108a241224d752bff4d5b519bc4d02e83653ef374 is accepted for the exact live port at c8450ee3. The original seven update identities and 145 removals are bounded compatible remediation. No runtime source, emitter source, original oracle or Midnight package identity changes.

After port, repeat locked metadata and strict audit identity plus the later value-lowering unit slice against the updated graph. Existing exact-source proof/TLS/installer/runtime/consumer gates remain accepted without redundant reruns. Carry lru unsoundness and three maintenance residuals into #357/#364; independent review and final-RC rescan remain mandatory. Do not call this a clean strict audit or full milestone qualification.

### Signed delivery

ADR0301/#425 compatible remediation delivered at `e79c639f6f6ebad47c818900a225d87f76947ebf`, conventional GPG+DCO verified. Seven registry packages updated; 145 unused packages removed with the unused cargo-nextest library dependency. The external runner remains usable. Six owned rand pins move to 0.8.6; explicit Clap wrap_help preserves maintained output. All Midnight package identities stay fixed; all resolved dependency-edge/feature changes are retained in the reviewed graph report.

Qualified candidate gates: actual Rust 1.88 compilation for backend/runtime/testkit/Counter transaction feature; 27 installer tests through official pinned nextest; 13 backend diagnostic tests; 9 primitive tests; original DID digest behavior table; 5 passport tests; 3 local TLS trust/hostname cases; one seeded 2912-byte proof with default-strict apply, changed-binding/replay refusal and baseline-identical final state. Proof material was hash-reused, no redundant keygen.

Root independently verified 3236 candidate artifact hashes and eight baseline/candidate identities. The exact live port passed locked all-feature metadata, 12 later value-lowering unit methods, and an identical strict audit report. `cargo-audit --deny warnings` exits 1: zero vulnerability/yanked matches, one lru unsoundness notice and three maintenance notices. The initial root default-mode invocation exited 0 despite warnings and is not counted as a clean strict scan. No ignores were added.

Residual lru/bincode/number_prefix/paste dispositions remain owned by #357/#364 for independent review, upstream maintenance decisions and final-RC rescan. This closes the compatible-remediation child only, not milestone security acceptance or release qualification.

[ADR0301 — Isolated dependency remediation](references-0.3.0.md#note-086) retains the 3237-entry source/gate archive (SHA256 47f90d7929d2036f59d8eec5f7c238b7e334610cbf7367e0eba846978c321bf5). [ADR0301 — Live dependency port.zip](references-0.3.0.md#note-087) SHA256 `66aaca9f9bc9f9ec99a08bed1f11bdecf173c6c0b43d23b0507c40929ca4e9b9` binds root live commands and accepted candidate receipt `0e1652fd9b1f198112e0211108a241224d752bff4d5b519bc4d02e83653ef374`.

Accepted parents remain 6/20. Source/resource/relation joins and final package migration remain open. No CI or push; user ledger document is preserved.
