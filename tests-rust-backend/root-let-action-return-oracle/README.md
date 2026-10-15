# Root Let action return oracle

`build.rs` generates the fixture from current source-bound schema-20 IR using
the current backend. The source is
`examples/rust_backend/root_let_action_return_oracle.compact`. The historical
schema-17 backend fixture remains unchanged; the new retained IR differs only
in `schema_version` and permits exact pairing with fresh compiler output.

The existing tests retain the two-call TypeScript state/result/gas oracle and
native/recorded/replay comparison. `--root-let-action-return` checks the exact
retained IR before keys, then uses its existing unbalanced smoke proof policy.
The separate terminal lexical adapter covers default-strict, Dust-funded ledger
application; these policies are intentionally reported separately.
