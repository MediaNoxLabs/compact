# Original microDAO, current generated Rust

`build.rs` generates the complete original `test-center/test-contracts/micro-dao.compact`
from `micro-dao-source-schema20-ir.json` with the current backend. All seven public
recording surfaces must remain available. The older `token-query-schema20-ir.json`
is an unchanged dependency projection (2 pure and 1 stateful circuit versus the
complete source's 22 pure and 17 stateful circuits); it cannot substitute for the
complete source in the token proof adapter.

The `token_query` test compares retained TypeScript observations, native execution,
current recording, official token derivation, public transcript and independent
replay gas across the retained addresses. Other existing tests keep their original
scope. To run the token qualification:

```sh
cargo test -p compact-rust-test-center-micro-dao-fixture --all-features --test token_query
```

Compile the exact original source using `compactc --target rust`, generate keys for
`dao_voting_token`, and run `compact-rust-proof-smoke --micro-dao-token OUTPUT`.
The proof adapter compares the complete retained IR before opening any key file;
its negative test rejects a changed declaration and the historical projection.
This is a real proof, verification and ledger application from constructor state
under the shared **unbalanced smoke policy**. It is not a funded DAO lifecycle test
or a proof of every export.
