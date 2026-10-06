---
id: RUST-ADR-0077
alias: ADR-0077
title: "Serialize Rust output publication and recover interrupted replacement"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["compiler-cli", "diagnostics", "output-publication"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: bfa6c61a1260e9e7c7229aef11f24aa8f97d842838dab65384f37e9b11fd0699
---
# RUST-ADR-0077 — Serialize Rust output publication and recover interrupted replacement

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept cross-process locking and next-invocation recovery around the staged output publication introduced by ADR0052. Participating launcher/filesystem assumptions remain explicit; recovery is not instantaneous or power-loss atomic. Preserve user-visible backups and orphan stages rather than claiming automatic safe deletion.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#176 closure](https://github.com/MediaNoxLabs/compact/issues/176#issuecomment-6017528816). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`e4984cb1`](https://github.com/MediaNoxLabs/compact/commit/e4984cb1c34e6af0c9481aea291d9c771fbd632f). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 77
status: accepted-local
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/176
```

## Historical decision and amendments

### Problem

ADR-0052 stages a complete Rust/joint output and replaces an existing directory with two renames. It protects handled errors, but two `compactc --target rust` processes can race on the same requested path. A process crash after moving the old output to its unique `-previous` sibling, and before moving the complete stage into place, leaves the requested path absent until a person finds that backup. The compiler is intended as a production build tool; these are concrete artifact reliability gaps under #151. A sibling stage is still valuable and remains the base design.

### Before and proposed after

```rust
// Before: both processes may enter this sequence.
fs::rename(output, &backup)?;
fs::rename(&stage, output)?;
// A crash between the two calls leaves output missing.

// Proposed: a sibling advisory lock is held from preflight through publication.
let lock = OutputLock::acquire(output)?;
recover_previous_output_if_missing(output)?;
let stage = StagedOutput::new_under_lock(output, lock)?;
// compile, render, package, refresh manifest
stage.publish(output)?;
```

The lock file is a persistent `.<output-name>.compactc.lock` sibling. A process holding it prevents a second participating compiler from entering the same output transaction. The operating system releases the lock on process death; the sidecar file itself must remain so waiters continue locking one inode. On the next invocation, if the requested output is absent and exactly one tool-named `-previous` directory exists, restore it before starting a new stage. Multiple candidates, a symlink, or a non-directory candidate fail with a clear diagnostic rather than guessing or deleting data. If a complete output exists alongside an old backup, keep the backup and report its path; do not silently delete it. Normal successful replacement still removes its own backup. Orphan stages may be reported but are not automatically deleted without an ownership proof.

### Ownership, alternatives and limits

Only the `compactc` Rust-target output transaction changes. Scheme, typed IR, AST emitter, generated crate and API, runtime, ledger-8/zk VM operations, ABI 34 and private schema 8 stay unchanged. Use a small cross-platform file-lock crate compatible with the package's Rust 1.88 minimum; `std::fs::File::lock` is unstable in 1.88. An in-process mutex would not protect separate compiler processes. PID files risk permanent stale locks after crashes. An atomic symlink pointer would change the public output-root contract, which currently rejects symlink roots. Advisory locking coordinates participating modern launchers only; it cannot stop an unrelated process or an older binary. Restoring after restart is not a guarantee of instantaneous crash atomicity or power-loss durability on every filesystem.

### Verification and delivery

Create a focused MediaNoxLabs issue in `rust-backend-v2` before source edits. Add unit/integration probes for lock ownership, same-output serialization, recovery after the first rename, ambiguous/unsafe backups, ordinary failed rebuild and successful replacement. Use the quickest direct `compactc` tests first, then packaged target/rejection and consumer gates if the transaction behavior changed; run the exact Rust 1.99 Clippy gate after code settles. Confirm the generated crate and runtime bytes remain unchanged. Record conventional GPG/DCO commit, exact local evidence and residual limits here, in the issue, and in the milestone delivery map. Keep the branch local; remote CI is deferred until the local backlog is complete.


### Local acceptance — 2026-10-05

ADR-0077/[#176](https://github.com/MediaNoxLabs/compact/issues/176) is accepted locally at conventional GPG/DCO `e4984cb1c34e6af0c9481aea291d9c771fbd632f` (signature verified). The Rust/joint `compactc` output transaction now holds a persistent sibling file lock from preflight through Scheme, AST rendering, runtime packaging, manifest refresh and publication. The lock is released by the OS when the process dies. The next invocation restores exactly one tool-named previous directory if the requested output is absent; ambiguous, file and symlink backups fail without deletion. A complete output with a stale old backup is kept and warned about. `fs2` 0.4.3 is pinned in both workspace and Nix CLI locks to preserve the declared Rust 1.88 minimum.

Verification: 12/12 `compactc` unit tests, including same-name lock exclusion, independent output names, unique-backup recovery, unsafe/ambiguous rejection and replacement; direct and Nix-packaged `check_rejections.py` (four source rejections plus failed/successful publication, crash-window recovery, cross-process serialization and capability reports); exact Rust 1.99 all-target/all-feature workspace Clippy with `-D warnings`; formatting and diff checks; fresh aarch64-darwin `nix build .#compactc` at `${HISTORICAL_NIX_STORE}/gf61s0jdnzyibzvjvamj4s21n4syl449-compactc`; packaged `check_compactc_target.py --consumer`; and packaged 137/137 fixture freshness. Generated code, runtime, VM operations, ABI 34 and private schema 8 are unchanged, so the proof/ledger gate was not repeated for this slice.

Limits: advisory locking coordinates participating modern launchers on filesystems that support it, not unrelated tools or older binaries. Recovery is on the next invocation, not instantaneous crash atomicity; power-loss durability is filesystem-dependent. A backup left after new output became visible is reported and preserved, and orphan stage directories are not silently deleted. The Nix source tree was dirty because of the user's unrelated documentation edit; no clean release or remote same-commit CI is claimed. The branch remains local/unpushed.
