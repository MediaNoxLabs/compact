# rust-backend-v2 remote acceptance and closure

**Milestone closed.** All ten required workflows passed at the revision below. GitHub confirms 243 closed issues and zero open issues in [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2), closed at 2026-10-06T13:55:53Z.

Published revision: `03c03a39a2d39c43c91143b9526ca1c2ffae706c`, branch `codex/rust-backend-ast` in MediaNoxLabs/compact. The initiative contains 266 conventional, GPG-signed, DCO commits after milestone 1. Compiler 0.31.133; backend-private IR 20; Rust runtime ABI 49. IR 20 is an internal protocol, not a public Compact schema.

## Delivery

- `compactc --target rust` emits a generated Cargo crate and matched runtime/macros; TypeScript remains the default target. Legacy Rust aliases are retained.
- Scheme analysis lowers to a closed typed IR; Rust rendering uses `syn`, `quote`, `proc-macro2`, and `prettyplease`. Typed slots, derives, witness bridges, recorded circuits, observed calls and ledger transaction adapters divide ownership between compiler, generated code and runtime.
- Constructors, pure/stateful circuits, witnesses, scalar/composite values, Cells, collections, Merkle operations, control flow, arithmetic, crypto and shielded intents have the documented source/behavior/proof coverage. Unsupported Rust shapes fail with source diagnostics. Recording capability is reported separately; requiring an unavailable recorded path fails closed.
- Pinned midnight-ledger 8.0.3 and matching midnight-zk crates supply ledger, VM, representation, crypto and proving primitives. Rust 1.99.0 is the validated toolchain; no independent MSRV certification is implied.

## Required exact-revision workflows

All ten required workflows and their required validation steps completed successfully at `03c03a39` on 2026-10-06. The exact-revision external Nix consumer also passed. Historical results remain attached to their actual revisions.

| Workflow | Exact-revision run | Result |
|---|---|---|
| Compiler Build | [37460145204](https://github.com/MediaNoxLabs/compact/actions/runs/37460145204) | Success — Linux compiler, Scheme/source-map/smoke, native Rust proofs, packages, archive consumers and E2E passed; Apple consumer gate passed 13 aggregate test cases. |
| Compiler extracted | [37460155679](https://github.com/MediaNoxLabs/compact/actions/runs/37460155679) | Success — 2,918 snippet tests passed with --skip-zk; not 2,918 successful contract compilations or proofs. |
| Compact Tool Test | [37460173997](https://github.com/MediaNoxLabs/compact/actions/runs/37460173997) | Success — 170/170 Linux; 163/163 Intel macOS; 168/168 ARM macOS. Platform selections differ. |
| Testing runtime | [37460185479](https://github.com/MediaNoxLabs/compact/actions/runs/37460185479) | Success — 78 tests across four files passed. |
| VSC Plugin Build | [37460197011](https://github.com/MediaNoxLabs/compact/actions/runs/37460197011) | Success — Build passed. |
| VSC Plugin Installation Tests | [37460209683](https://github.com/MediaNoxLabs/compact/actions/runs/37460209683) | Success — Installation tests passed. |
| VSC Plugin Integration Tests | [37460227064](https://github.com/MediaNoxLabs/compact/actions/runs/37460227064) | Success — Integration tests passed. |
| CodeQL | [37460238160](https://github.com/MediaNoxLabs/compact/actions/runs/37460238160) | Success — JavaScript/TypeScript analysis passed. |
| Scan | [37460249674](https://github.com/MediaNoxLabs/compact/actions/runs/37460249674) | Success — Configured scan passed. |
| Debugging project build | [37460262158](https://github.com/MediaNoxLabs/compact/actions/runs/37460262158) | Success — Two test executions passed; two existing skips across source/dist copies. |

The main Linux job completed at 13:23:08 UTC. Native Rust validation, package rehearsal, archive consumers and E2E all passed. The Apple consumer count is an aggregate of its test cases and does not imply an Apple proof run. Extracted tests check their declared compiler/error/generated-JavaScript behavior with `--skip-zk`; debugging counts include source and dist copies. Neither is a new proof-coverage denominator.

At `eb0e4019`, nine of ten workflows passed, and the Apple compiler job passed. The main Linux native step completed numerous proof checks before failing during compilation of the runtime `recording_map` integration test with `No space left on device` (OS error 28). The step lasted about 69 minutes. This is an observed storage failure, not a contract assertion failure; later package, archive-consumer and E2E steps were skipped. It remains a failed historical acceptance candidate.

The prior `5671d65d` candidate completed 9/10 workflows successfully and failed the clean-runner native proof prerequisite. It remains historical evidence, not an accepted candidate.

## Evidence scopes

| Evidence | Result | Revision / limitation |
|---|---|---|
| Whole-source inventory | 217 sources; 196 roots; 755 exports; 386/386 proof-required APIs; 369 nonproof exports | Frozen d5dd3f2e inventory. API availability is not exhaustive semantic/proof coverage. |
| Full local product gate | 374 commands; 176 fresh fixtures; 371/371 proof-required APIs in that fixture scope | d5dd3f2e. Distinct denominator from the whole-source inventory. |
| Reviewed TS behavior | 37 sources / 203 rows | Per-row evidence varies across result, state, four-dimensional gas, VM transcript, private outputs and replay. No universal all-input/all-path claim. |
| Original Coracle / microDAO | 4 + 7 selected proof selectors | Selected seeded states. Three selectors—guess, vote_reveal and dao_voting_token—use balancing-disabled smoke application; the other eight use default-strict application with the documented funding setup. No continuous original-app lifecycle claim. |
| Clean proof-material cache | 12 built-ins; SRS parameters for k = 13, 14 and 15; expected missing/corrupt-cache refusals; strict funded proof/application/replay | Preparation/proof runner at eb0e4019; fresh keys from the frozen d5dd3f2e compiler and Scheme with relevant implementations verified identical. Closed endpoint/proxies, not an OS sandbox. Contract keygen separately added SRS for k = 7. |
| Conditional/sequential proof supplement | Six conditional calls plus a sequential two-write Cell case | ADR232 at ad4fc7c1; both boolean branches; unbalanced smoke application. |
| Previous remote-source Nix consumer | Passed at exact eb0e4019 | GitHub-pinned source on aarch64-darwin; original Coracle and microDAO generated; all 37 original files per crate unchanged before and after offline all-features Cargo checks. Only new Cargo.lock files allowed; no proof or live rerun. |
| Current remote-source Nix consumer | Passed at exact 03c03a39 | GitHub-pinned source on aarch64-darwin; Coracle and microDAO passed offline all-features checks. All 37 original files per crate unchanged; only Cargo.lock added. Compilation evidence, not new proof or live evidence. |
| Current runtime workflow | Passed at exact 03c03a39 | Four test files, 78 tests; run 37460185479. The complete current workflow set is listed above. |
| Historical native-frame measurement | 42 timed samples (21 paired before/after comparisons) and external signature/type checks | Historical before/after 4868f001. No reliable speedup/regression demonstrated. |
| Live wallet | Counter round 1 → 2; separate shielded release and recovery of 42 | Older exact heads e671db3b and 628d1c03; node 0.22.3 / indexer 4.0.1 / ledger 8.0.2 with codec 8.0.3. Trusts connected finalized observations; no consensus-proof verification. Exact replay RPC 1013 is not a fresh double-spend proof. |

Current `03c03a39` external consumer receipt SHA256: `9a8e8be920d5c643dd76e6dd3c4e6a63f9947ba1dba0ff1e58d7558fd12f2232`. GitHub source NAR: `sha256-J1lgvVHfsQ6yr6dX3HEgv3B4W426KwtGcemigK/Arfs=`. On aarch64-darwin, unchanged generated Coracle and microDAO crates passed offline all-features Cargo checks in 0.949 s and 0.378 s respectively. These are individual check timings, not clean-build or performance claims. All 37 original files per crate retained their hashes; only new Cargo.lock files were added. No proof or live rerun is implied.

Cold-cache receipt SHA256: `47e3b779e94de6097940c1174609cdd02702576e644c06a8350bed728bf43281`. Its preparation/proof binary is from `eb0e4019`, with SHA256 `ebd8ee7206c5e1b93978f11d1bcb097b465d0dbdb608d09a78560a9f5f344eb0`. Fresh contract keys used the frozen `d5dd3f2e` compiler and Scheme. The retained implementation-equivalence evidence and ADR240 source bridge connect that evidence to `03c03a39`; they do not relabel the historical executions or compiler artifacts.

Historical `eb0e4019` external consumer receipt SHA256: `df8e4b2945123b26ddb2a79490991e511bb0e33e5654b07481f28ccf5fb9f136`; GitHub source NAR: `sha256-v7uQUP5kN8nNGzkVw8HmKFIkRWBKZF6tqP3Lpp3ElHE=`. This remains exact-revision compilation/capability evidence for that earlier head, separate from the current consumer and proof receipts.

The broad local product gate is preserved at d5dd3f2e. Later commits include real installer and TypeScript runtime changes as well as test/workflow repairs; they have focused validation evidence and historical remote results. All ten workflows have now passed at `03c03a39`. These remote results are not represented as a rerun of the frozen 374-command local receipt.

## Remote repairs

- ADR231 / #335: added manual dispatch for extracted compiler and CodeQL acceptance.
- ADR232 / #336: completed the missing conditional and sequential write proof cases.
- ADR233 / #337: canonicalized duplicate formatter inputs before concurrent writers; retained per-input reports and concurrency for distinct files.
- ADR234 / #339: explicitly rejects noncanonical Schnorr signing keys before entropy, independent of upstream curve-provider behavior. The current `03c03a39` runtime gate passed 78 tests, while earlier provider receipts retain their original versions and revisions.
- ADR235 / #338: fetched locked dependencies into the clean macOS host Cargo cache before offline consumers.
- ADR236 / #340: explicitly selected Intel macOS and verified actual X64 provenance.
- ADR237 / #341: aligned the extracted compiler timeout with the 180-minute main job budget after the `9a94248c` run exceeded 45 minutes before E2E. All build/test/artifact commands remain intact.
- ADR238 / #342: replaced the Linux CLI's musl-native Rust/LLVM bootstrap with static musl output using native build tools. Both Linux graphs exclude the exact failed toolchain; both Darwin CLI derivations are unchanged. Seven ELF/error guard probes passed. The `eb0e4019` extracted compiler workflow completed Linux packaging and its E2E tests. The current `03c03a39` Linux compiler and extracted workflows also passed. Locks, IR 20, ABI 49, product semantics and workflow commands are unchanged.
- ADR239 / #343: explicit acquisition of 12 built-in Zswap/Dust assets and verified-IR-derived SRS parameters for k = 13, 14 and 15 through the upstream hash-verifying provider. Shared runner cache, verify-only mode and retained JSON receipt; existing synchronous resolvers and all strict proof assertions stay intact. The hash-matched built-in prover-key headers independently confirm k = 13, 14 and 15. Focused cold-cache acquisition, both closed-endpoint preparation modes, expected missing/corrupt-cache refusals, and the unchanged default-strict funded coin proof all passed at eb0e4019. Contract key generation additionally required SRS for k = 7; that is separate from built-in k = 13, 14 and 15. The preparation/proof runner is eb0e4019; fresh keys used the immutable d5dd3f2e compiler and Scheme with compiler, emitter and Rust runtime implementations verified identical to eb0e4019.

- ADR240 / #344: bound Linux CI artifact storage after the observed `eb0e4019` disk exhaustion. Preparation, generated consumers, workspace tests, Clippy and package rehearsal share the absolute workspace `target/` directory; `target/package` remains unchanged. Only the Linux job sets `CARGO_PROFILE_DEV_DEBUG=0` and `CARGO_PROFILE_TEST_DEBUG=0`; assertions, overflow checks, optimization level and every validation command remain intact. Storage telemetry records free space and target size. A focused aarch64-darwin test of these overrides passed both `recording_map` cases; temporary development/test probes confirmed `debuginfo=0`, `opt_level=0`, debug assertions and overflow checks. This local configuration evidence is now supplemented by the passed `03c03a39` Linux job, including native proofs, packages, archive consumers and E2E. Neither result measures exact runner space savings. No emitter, runtime, IR 20, ABI 49 or dependency pin changed. Local validation receipt SHA256: `84e7c89c87f4e5789b2e30b596a86249fa0d2b7f6d55cd8288388cbcac0e121b`.

At `5671d65d`, nine workflows passed, including 2,918 extracted tests, 78 runtime tests and all installer platforms. The main Linux compiler build and earlier tests passed before native proof execution failed on missing zswap/9/output.prover. The Apple compiler target passed. ADR239 addresses this newly exposed clean-runner prerequisite. This 9/10 result remains a failed acceptance candidate.

At `373b6aeb`, seven workflows passed, while both extracted and the main compiler reached explicitly annotated 180-minute timeouts, with subsequent tests skipped. The main compiler raw workflow/job conclusion is cancelled; the check-run annotation confirms the three-hour maximum. Both showed the exact musl-native Rust toolchain. Old debug was intentionally superseded while compiling; it is not classified as a timeout/test failure. These are preserved historical outcomes, not final `03c03a39` acceptance.

The `9a94248c` installer, runtime, VSC and security results remain historical. Its extracted run timed out before tests; its still-compiling compiler/debug jobs were intentionally superseded for `373b6aeb`. Product sources and dependency pins are identical between `9a94248c` and `373b6aeb`.

Initial remote failures remain part of the engineering record. The repaired revision's successful runs are linked above.

The package rehearsal reports repository-wide `source.dirty=true`; its status includes generated smoke outputs and is not a clean tagged-release attestation. Independent archive comparison matched all 6 macro-package and 427 runtime-package source files to `03c03a39`, with no mismatches or unmapped files. Cargo-generated normalized manifests, package lockfiles and VCS metadata are excluded from that source comparison. No registry publication occurred.

## Compatibility limits and distribution decision

Raw EC calls differ for scalar q between providers: Rust and registry runtime 3.0.0 reject it, while pinned Nix runtime 3.1.0-rc.1 reduces it. Use canonical scalars `[0,q)` or the explicit `jubjubScalarFromNative` reduction route for portable behavior. Compact's Field signature alone does not enforce this domain. Passing Schnorr boundary tests does not establish all-Field provider parity.

Strict offer binding checks consistency against the supplied state. CompleteLedger admission applies the offer against supplied full ledger state, including available history; TrustedObservation admission leaves global nullifier/root history to node admission. Neither mode authenticates consensus. Unknown/new Compact shapes require explicit IR support. Some constant errors intentionally occur at compilation in Rust versus runtime in TS. Long outer HOME paths can alter installer help snapshots; the historical failure and short-HOME validation are retained.

Per the milestone's recorded scope amendment, distribution is the exact published branch and matching Nix build. A new version, tag, GitHub release, crates.io publication, independent release-candidate gate and public-registry consumer are outside this milestone. Windows and arbitrary provider/platform combinations are not certified.

All 243 scoped issues and rust-backend-v2 are closed, with individual closure comments linking delivery evidence and the final workflows. Unrelated issues are outside this closure. These results do not certify every valid Compact program. The final Obsidian handoff preserves the chronological ADRs, delivery map, architecture assessment, sanitized evidence bundle and engineering deck.
