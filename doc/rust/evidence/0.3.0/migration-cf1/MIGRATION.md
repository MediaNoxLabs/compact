# Rust release compatibility and consumer migration — ADR0293 candidate

This isolated candidate implements the accepted release mapping. It does not publish a crate, create a tag, or establish registry availability.

| Boundary | Candidate |
|---|---|
| Fork artifact label | rust-backend-v0.3.0 |
| Backend package | 0.2.0 |
| Runtime and macros | 0.2.0, exact paired dependency |
| ContractLab testkit | 0.1.0, unpublished; exact runtime 0.2.0 dependency |
| Generated application default | 0.1.0, unpublished |
| Runtime ABI / IR / capability / compatibility schemas | 50 / 20 / 3 / 1 |
| Compiler / language / TS runtime | 0.31.133 / 0.23.105 / 0.16.101 |
| Native MSRV | 1.88, bounded tested graph and feature scope |

## Backend callers

The resource limit variant and `#[non_exhaustive]` belong to the new backend 0.2 compatibility line. Runtime ABI 50 does not make an exhaustive Rust enum match source-compatible. Preserve explicit cases where useful, then add a fallback:

```rust
match error {
    RenderError::ResourceLimit { resource, limit, observed } => {
        format!("{resource}: limit {limit}, observed {observed}")
    }
    _ => error.to_string(),
}
```

Resource labels and Display text are human diagnostics, not a machine protocol. The old exhaustive consumer is retained unchanged as a migration failure control. A second control explicitly lists the new resource variant but has no wildcard; this isolates the new non-exhaustive contract from merely forgetting one variant.

## Generated contracts

Keep a historical ABI49 generated contract with its exact matching source bundle, or regenerate using the approved ABI50 compiler/runtime/macro pair. Editing its ABI assertion is not migration. Existing ABI50 generated code is the compatibility control; package labels alone do not authenticate source. The selected-source metadata and fingerprints remain necessary mismatch checks.

Application versions belong to the application owner. This change does not rewrite generated positional signatures or introduce named argument records. Bundled and explicit shared-source modes remain supported. Registry mode emits the exact approved runtime requirement but cannot make an unpublished package available.

## Scope and pending gates

Source snapshot: signed e79c639f plus the separately hash-bound accepted ADR0291 overlay. ADR0295 is not yet joined. Runtime rand remains exactly 0.8.6; the installer has no nextest library dependency. Ledger semantics remain the native 8.0.3 graph. Original compiler ledger8.0.2 and isolated DID wire/receiver8.1 profiles are separate evidence scopes.

Focused candidate evidence now includes 16 CLI tests, 21 compatibility tests, strict backend Clippy, actual Rust 1.88 all-feature checks, old/new ABI50 generated execution and shared-source ContractLab recorded/replay execution. Four external RenderError controls preserve the actual old source, isolate the new non-exhaustive requirement, and run the migrated resource handler. The original ABI49 fixture fails specifically at its unchanged ABI assertion. Both runtime/macro package archives were verified with an explicit local macro patch; no registry claim follows.

The tracked standalone backend lock has a real separate MSRV gap: copied without the workspace, it selects konst 0.4.3 and konst_proc_macros 0.4.1 (Rust 1.89 required). Nix uses this lock directly. The approved workspace-lock graph passes Rust 1.88. The failure and exact graph comparison are retained. Accepted ADR0304 synchronizes the standalone closure: 303 packages, zero new identities/checksum differences/added edges, 24 feature-inactive edges explicitly pruned. Actual locked Rust 1.88 standalone check and 18 resource tests pass; ten lock regression tests pass. Both package archives were refreshed after the two approved README corrections. ADR0295 final source joining remains pending.
