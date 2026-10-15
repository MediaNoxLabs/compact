# Current-generated receive consumer

Builds the complete retained source IR with the current backend, preserving the
original Compact source, TypeScript oracle and existing runtime tests.

The receive cutover adds an internal recorded `receiveShielded` helper. Public
recorded bodies and export capabilities remain unchanged; this refresh qualifies
the newly generated crate and retains the original runtime oracle coverage. It
is not another selector retirement or a new proof qualification.

```sh
cargo test -p compact-rust-transient-receive-send-oracle-fixture --all-features
```
