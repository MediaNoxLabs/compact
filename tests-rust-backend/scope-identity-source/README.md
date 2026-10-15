# Source binding identity regressions

Generated at build time from exact `scopes.compact` IR. The source lives under
`tools/compact-rust-backend/tests/scope-identity/`; compile it with the current
Scheme frontend and Rust backend before refreshing `scope-identity-schema20-ir.json`.

The suite distinguishes an inner action-local binding from a same-spelled outer
parameter in a separate return. Before #528, native Rust leaked the inner value
while the historical kernel recorder and TypeScript preserved the outer value.
Other controls retain genuine top-level local returns, nested and sibling scopes,
suffix-like source names, `$`/`_` spelling collisions, Counter constructor/action
scopes, constructor loop items, pure folds, and repeated aliases of one callable.
Ordinary public parameter names remain stable; `$` is normalized for Rust and
collisions receive deterministic suffixes.

```sh
cargo test -p compact-rust-scope-identity-source-fixture --all-features
python3 tools/compact-rust-backend/tests/scope-identity/check-ir.py FIRST_IR REPEAT_IR
node tools/compact-rust-backend/tests/scope-identity/check-typescript.mjs GENERATED_TS_INDEX
```

The TypeScript check needs the matching installed Compact runtime available to
its generated module. Its expected values and ledger effects are independent of
the Rust code. Rust tests compare native/recorded execution with manual official
operations. These are finite runtime regressions, not proofs. Historical IR that
already collapsed two lexical identities cannot be repaired by guessing its
intended source scopes; regenerate it from source.
