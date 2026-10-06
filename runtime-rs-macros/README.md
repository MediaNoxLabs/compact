# Midnight Compact runtime derives

Architecture history: [Rust backend and runtime ADRs](https://github.com/MediaNoxLabs/compact/tree/codex/rust-backend-ast/doc/rust/adr), with decision status, code examples and delivery evidence.

This crate provides procedural derives for ledger representation types emitted
by the Compact Rust backend: `CompactCellValue`, `CompactEnum`, `CompactMerklePath`,
`CompactMerklePathEntry`, and `CompactMerkleTreeDigest`. The main
`midnight-compact-runtime` crate reexports these derives and is the intended
dependency for generated contracts.

`CompactEnum` derives the default first variant and the checked field and
binary representations for a concrete unit enum. Variant order determines
ordinals; the binary width grows from one to two bytes at 257 variants. It
rejects data-carrying variants and explicit Rust discriminants.

The derives implement mechanical conversions to Midnight ledger values. The
compiler owns source typing and the runtime owns the corresponding ledger
semantics. See the [Rust backend guide](https://github.com/MediaNoxLabs/compact/blob/codex/rust-backend-ast/tools/compact-rust-backend/README.md)
for the generated crate and ABI contract.
