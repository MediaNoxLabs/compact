# Candidate consumer, coverage and ADR0362 receipt

Date: 2026-10-08
Measured source: `2c798d2dc5f94ad13e89610068035ed70d94f469` (ADR0362 rejection harness measured as a recorded working-tree delta, then signed as `679be285d91baf87ae55ae3161140e371d259b62`). This is partial final qualification, not milestone closure.

## Accepted bounded outcomes

- Fresh Scheme source build: version0.31.133, 829 filtered source files matched to the Git archive; derivation and closure retained. Current Rust CLI rebuilt offline. Separate same-source Scheme tool output supplies formatter/fixup for packaging.
- Clean external W2 ContractLab: one witnessed commit/exhaustion-checkpoint test passes on Rust1.99 and1.88. Candidate registry identities and a single isolated runtime verified.
- API migration: migrated current caller runs; complete known-variant error match without fallback fails with expected E0004. Historical old ABI controls remain historical.
- Coverage: fresh decoder and full-feature testkit tests, exact-file equality joins for other runtime files. Runtime changed lines367/375 (97.87%), all mapped runtime6059/6883 (88.03%). Testkit411/421 (97.62%); module-only lib.rs unmapped. Floors met; no branch/formal guarantee claim.
- Portable compiler archive: manually assembled documented layout, authenticated debug Rust CLI, freshly built Scheme tools, pinned installed ZK binaries and current runtime source. Maintained archive builder and relocated path-with-spaces/installer-symlink smoke pass with overrides unset. TS and strict Rust generation, byte-identical runtime bundle, offline locked default/transaction Cargo checks pass. Not a release-optimized binary or successful full Nix distribution derivation claim. Initial archive attempt correctly refused a non-executable launcher; assembly mode corrected to documented0755 and successful attempt retained separately.
- Dependency scans use freshly cloned RustSec DB b8a1a33e246a0a9a3b5f377248c41a503defec74: strict exits1, workspace4notices/standalone3, zero vulnerability matches. All452/302external package identities match the reviewed baseline. Existing #471–474 dispositions remain; no clean audit assertion.
- ADR0362/#499: failed full gate retained; focused rejection gate passes after stale preflight/recovery assumption corrected. Complete exact frontend diagnostic, byte preservation and cleanup checks retained. Production source unchanged.

## Artifact

[[Candidate consumers coverage and ADR0362 — 2026-10-08.zip]] contains 88 files plus its byte-hash manifest. SHA256 `aa2c37bc5e177fe8ac7ad5b00e4f13a30a6ae6d2b5b93ca4dc882a973e935b37`. Size 1281037bytes. Instrumented binaries/profile raw files, extracted full SDK copies and portable executable ZIP remain in local scratch; hashes are retained, but this evidence archive does not claim to contain those binaries. No private product research included.

## Successor qualification

Signed/DCO successor679be285 verifies Good GPG and is pushed. Exact Git source join confirms the only change is check_rejections.py; compiler/runtime/dependencies are byte-identical to measured2c798d2d. Successor full gate is running separately; all original receipts retain original labels. Parent #364 remains open.
