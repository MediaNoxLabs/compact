---
id: RUST-ADR-0256
alias: ADR-0256
source_sha256: 841d2bd41ccfde9ea50f75c4b2da013c8e95cafbaf238f8e2c24f0dde97ba82c
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0256 — Measure a bounded generated Rust WebAssembly consumer

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted experiment scope, 2026-10-07. Parent R030-15/#359. No production target promise or runtime feature change. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR-0256 — Measure a bounded generated Rust WebAssembly consumer

Status: accepted experiment scope, 2026-10-07. Parent R030-15/#359. No production target promise or runtime feature change.

### Problem and initial evidence

The native Rust runtime includes crypto/proof dependencies even for small contracts. Dependency names alone cannot establish browser compatibility. A fresh unedited Counter crate successfully checked for wasm32-unknown-unknown on Rust1.99.0 after selecting the installed LLVM Clang and llvm-ar for the blst C dependency. Apple Clang lacked the target; this was a host compiler limitation, not a Rust source incompatibility. No production source changes were required for Cargo check.

### Decision

Build a scratch consumer of freshly generated, unedited Field arithmetic/hash and Counter/witnessed Cell crates. Use the existing runtime and ContractLab with one shared runtime root. Add a thin wasm-bindgen boundary with lossless byte/hex transport. Run the same deterministic scenarios natively, in Node and in an actual headless browser. Preserve build/toolchain/source/lock provenance and diagnose exact blockers. No fake entropy, substitute VM or native subprocess masquerading as WebAssembly execution.

### Before/after

```text
Before: native generated crate checked on host.
After: same generated crate -> thin JS binding -> Wasm -> Node and browser.
```

Only the scratch transport wrapper changes. The emitter continues to own Rust syntax, generated code owns source semantics, upstream runtime owns primitives/VM, and JS owns transport/host entropy where the actual dependency graph requests it.

### Acceptance and measurements

- Actual Cargo build, not only metadata/check, then JS invocation.
- Exact native/Wasm outputs for pure arithmetic/hash, stateful Counter and witnessed Cell; real replay, failed-call rollback and malformed/noncanonical byte handling where exposed.
- Record imports, target features, toolchain, source/lock hashes, raw/compressed Wasm size, sampled startup and current/peak-observed memory with honest measurement limits.
- Node and browser tested separately. A supported bounded subset or a minimal blocker report is acceptable; browser proving, threads, file IO and network acceptance remain outside this prototype.
- Preserve all experiment code and commands in midnight Obsidian; no repo documentation until closeout. A production feature/API change needs a follow-up decision.

### Primary references

Rust documents that this target supports core/alloc but filesystem and thread operations are limited; target feature support depends on the host engine. wasm-bindgen supplies Rust/JavaScript bindings. These documents do not qualify our dependency graph; the experiment does.

- https://doc.rust-lang.org/rustc/platform-support/wasm32-unknown-unknown.html
- https://wasm-bindgen.github.io/wasm-bindgen/print.html

### Local disposition

Validated bounded subset in native Rust, Node24.14.0 and Chromium151.0.7922.34: pure arithmetic/hash, Counter and witnessed Cell through the real VM. Large-value byte transport, exact replay and failure behavior pass. Graph/imports, size, startup/call samples and linear memory retained in [ADR0256 — Node and browser WASM receipt](references-0.3.0.md#note-023). This closes the bounded experiment and parent R030-15 locally. It does not promote production WASM, proving or universal engine support. No production edits.
