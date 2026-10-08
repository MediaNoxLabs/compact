---
id: RUST-ADR-0285
alias: ADR-0285
source_sha256: a2bc1e2fa3da61067a50232afc83c48fffe315bb372b8ae5282b5307c72de710
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0285 — Qualify typed consumer safety and pinned upstream trust boundaries

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted, bounded test/evidence delivery; parent acceptance remains root-owned. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.

**Resumed and delivered, bounded scope:** the 2026-10-08 amendment records owner-authorized resumption and accepted consumer qualification at `4c8aebc3` (two positive controls and intended E0382/E0423 negatives; retained receipt archive SHA256 `6cd197c5a63a5938f2ab9534433300bce54b62094c645c7bd75c816f17a0c4d4`). #409 has no pending scope question. The original stop and partial logs below remain historical and unaccepted; neither this receipt nor publication establishes whole-milestone or dependency security acceptance.

## Original decision and amendments

## ADR0285 — Qualify typed consumer safety and pinned upstream trust boundaries

- Status: accepted, bounded test/evidence delivery; parent acceptance remains root-owned.
- Date: 2026-10-07
- Parent: R030-03 / #347
- Milestone: rust-backend-v0.3.0

### Problem

Existing typed slots, canonical scalars, bounded arithmetic, recorded replay and sealed offer policies have concrete negative and positive tests, but parent347 lacks one fair Rust-versus-TS misuse inventory and an explicit pinned dependency unsafe/FFI disposition. Ordinary typing is shared with TS; Rust's owned execution handoff and private bounded-value construction deserve direct consumer controls rather than broad superiority claims. Owned runtime forbid(unsafe_code) does not imply a dependency-free unsafe trust base.

### Decision and before/after consumer examples

Add two focused negative/positive pairs using the existing external generated Counter consumer harness. No compiler, runtime, public API, ABI50, IR20, package or dependency change.

Before, accidental reuse is merely an undocumented type-system expectation:

```rust
let first = counter::ledger_contract::increment(context)?;
let second = counter::ledger_contract::increment(context)?; // moved input
```

After, this is an executable external compile-fail control requiring Rust E0382, paired with a successful real generated call sequence:

```rust
let first = counter::ledger_contract::increment(context)?;
let second = counter::ledger_contract::increment(first.context)?;
assert_eq!(counter::ledger_contract::read_round(second.context)?.result.value(), 2);
```

Unchecked `BoundedUint::<255>(256)` must refuse through constructor privacy (E0423), paired with checked new(0)/new(255) success and new(256) returning the exact UnsignedOutOfRange variant and payload. Compile the valid consumer before trusting any failure; parse structured compiler diagnostic codes so an arbitrary Cargo/environment failure cannot count as a negative result.

### Ownership and integration

New focused consumer_safety.py and consumers/safety/*.rs own these controls. A small check_archive_consumer.py hook executes them in the already established generated Counter/archive graph. The helper also accepts an existing external consumer manifest for focused local execution without re-running archive extraction or broad proof suites. Generated source stays unedited. Caller environment/Cargo toolchain/target are explicit. Temporary test files must not overwrite existing consumer files and must be removed after the run.

Root approved paths and leases the warm target/compact-rust-parity-gate; Rust1.99.0, incremental0, locked/offline, j4. No parallel competing Cargo. Historical package/archive qualification is not silently relabeled as this run.

### Pinned dependency trust disposition

Use actual Cargo.lock source/checksums and host normal/build feature trees. The inspected host graph contains227 default and236 ledger-transaction package identities; those are dependency counts, not executed or unsafe function counts. Direct inherited boundaries include base-crypto repr-transparent FAB cast, midnight-curves/blst C+assembly, proof MSM TypeId casts, storage loader type_name cast, persistent shared pointers, provider TLS/ring and OS entropy/threading. Distinguish optional absent SQLite from default in-memory storage. This is a scoped boundary inventory, not a full upstream memory-safety, constant-time, proof-system or human security audit.

Consolidate the fair misuse matrix, concrete limits and historical receipts in midnight. Caller-provided key identity, supplied observation provenance, authenticated ownership and finalized chain evidence remain distinct. Trusted callback final-boundary checks do not attest transient arbitrary Rust actions. Existing scalar canonical/reduction and Keccak no-added-tags semantics remain unchanged.

### Alternatives and limits

- No new wrappers/role traits just to manufacture an implementation milestone.
- Do not repeat existing wrong-slot tests or weaken compile/runtime assertions.
- Do not assert Rust uniquely typechecks; TS declarations and runtime/ledger checks are real boundaries too.
- The broader R030-17 fuzz/resource obligations remain separate. ADR282's measured supported-renderer fix does not establish arbitrary-depth safety.
- No stopped cross-instance authorization research, remote CI, publication or parent issue closure.

### Acceptance

Valid external consumer compiles; positive pairs execute expected state/boundary behavior; negatives fail with intended compiler classes; hook and safe temporary-file cleanup have focused checks. Retain exact source/generated/runtime/lock/toolchain identities and commands. Root reviews before committing tests and accepting parent347.

### Execution stopped — 2026-10-07

Automatic review stopped the delegated consumer-safety/dependency-boundary review for possible cybersecurity risk without a more specific reason. Partial files/logs preserved, uncommitted and unaccepted. [ADR0285 — Automatic review stop](references-0.3.0.md#note-067). Parent and child remain open; no retry.

### Owner authorizes resumption — 2026-10-08

The owner explicitly keeps #409 required and authorizes proceeding. Resume the documented local positive/negative ownership and private-construction controls, plus read-only pinned trust-boundary inventory. Preserve the earlier stop and partial logs as history; they remain unaccepted. New current-source execution and review must establish acceptance. If automatic review rejects a specific action again, record and report that action/reason rather than bypassing it. No scope deferral or claim that this authorization itself qualifies safety.

The staged ADR publication package predates this amendment and must be refreshed before publication; do not silently publish the obsolete stopped-only disposition.

### ADR0285/#409 resumed and delivered — 2026-10-08

Owner retained required scope and authorized resumption. Signed/GPG/DCO `4c8aebc3` pushed; new external generatedCounter qualification passes two positive runtime controls and intendedE0382/E0423 negatives.147Python tests pass. Read-only74-file owned-unsafe/FFI inventory and fairTScomparison retain inherited dependency/aliasing/provenance limits; independent harness review accepted. [ADR0285 resumed consumer qualification — 2026-10-08](references-0.3.0.md#note-066) holds exact evidence. ArchiveSHA256 `6cd197c5a63a5938f2ab9534433300bce54b62094c645c7bd75c816f17a0c4d4`. Prior partial stopped logs remain history, not acceptance. Goalactive; no #409 scope question pending. Parent347/351/357/361 and final gates still need acceptance;12/19 unchanged. ADR publication package requires refresh before closeout.
