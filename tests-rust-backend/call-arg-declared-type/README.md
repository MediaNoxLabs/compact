# Declared call-argument types

Generates the current Rust backend during the build from the complete retained
`call_arg_declared_type.compact` schema-20 IR. Fresh source compilation is
byte-identical to the retained pre-owned-values source census IR. All 16 existing
stateful exports keep their recorded/observed capability; the build asserts that
exact set.

The owned-value foundation changes `pureFromImpure` recording ownership from the
legacy pair-hash route to the existing unit-actions algebra route. The existing
nine stateful oracle tests and separate direct-pure oracle test remain unchanged:
they check TypeScript values/state/VM traces/gas, witness advancement and negative
branch behavior. The field-pair-hash test includes `pureFromImpure`.

```sh
cargo test -p compact-rust-call-arg-declared-type-fixture --all-features
```

This fixture adds current-generation coverage to an existing runtime oracle. It
introduces no new source interface, recording capability or proof qualification.
