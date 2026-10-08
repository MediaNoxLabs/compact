# Compact Rust backend and runtime

[Milestone 0.3.0 closure report](0.3.0-closure.md) · [Acceptance crosswalk](evidence/0.3.0/closure/crosswalk.md)

The Rust target uses the ledger-8 Compact frontend, a typed private IR, a Rust AST emitter (`syn`, `quote`, `proc-macro2`, `prettyplease`) and a native runtime built on Midnight ledger/zk primitives.

## Guides

- [Start with a task: compile, implement witnesses, test and integrate](guides/index.md)
- [ContractLab testkit](../../testkit-rs/README.md)

- [Compiler backend and generated crate guide](../../tools/compact-rust-backend/README.md)
- [Runtime APIs, ledger ownership and ABI](../../runtime-rs/README.md)
- [Mechanical representation derives](../../runtime-rs-macros/README.md)
- [Rust architecture decision register](adr/README.md): 360 published records, topic index, original aliases and dated engineering history
- [ADR publication provenance](adr/publication-manifest.json) and [evidence conventions](adr/references.md)

## Accepted baseline and governance

Current 0.3.0 engineering acceptance uses implementation `f9a49666`, private IR20 and runtime ABI50; see the [closure report](0.3.0-closure.md) for its composite qualification, application scope and known limits. Package versions are unchanged.

Milestone 2 acceptance belongs to implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c): compiler 0.31.133, private IR schema 20 and runtime ABI 49. See the [public closeout](https://github.com/MediaNoxLabs/compact/issues/103#issuecomment-6017383781) for source/behavior/proof/live evidence scopes and distribution limits. Publishing these documents does not rerun implementation validation.

The Rust ADR series records branch implementation decisions. [CoIPs](../../coips/README.md) remain the separate upstream proposal and adoption process, with independent numbering and review. No new milestone is opened by this publication.
