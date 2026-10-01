# Midnight Compact runtime derives

This crate provides procedural derives for ledger representation types emitted
by the Compact Rust backend: `CompactCellValue`, `CompactMerklePath`,
`CompactMerklePathEntry`, and `CompactMerkleTreeDigest`. The main
`midnight-compact-runtime` crate reexports these derives and is the intended
dependency for generated contracts.

The derives implement mechanical conversions to Midnight ledger values. The
compiler owns source typing and the runtime owns the corresponding ledger
semantics. See the [Rust backend guide](https://github.com/MediaNoxLabs/compact/blob/codex/rust-backend-ast/tools/compact-rust-backend/README.md)
for the generated crate and ABI contract.
